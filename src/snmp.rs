use anyhow::{Context, Result};
use std::net::UdpSocket;
use std::time::Duration;

/// SNMP 클라이언트
pub struct SnmpClient {
    community: String,
    timeout: Duration,
    request_id: u32,
}

impl SnmpClient {
    pub fn new(community: String) -> Self {
        Self {
            community,
            timeout: Duration::from_secs(2),
            request_id: 1,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// SNMP GET 요청을 보내고 값을 반환합니다.
    /// OID는 점으로 구분된 문자열 형식 (예: "1.3.6.1.4.1.2021.11.11.0")
    pub fn get(&self, host: &str, oid: &str) -> Result<f64> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .context("Failed to bind UDP socket")?;
        
        // 타임아웃 설정
        socket
            .set_read_timeout(Some(self.timeout))
            .context("Failed to set read timeout")?;
        
        // 송신 타임아웃도 설정 (선택적)
        socket
            .set_write_timeout(Some(self.timeout))
            .context("Failed to set write timeout")?;

        // SNMPv2c GET 요청 생성
        let request = self.build_get_request(oid)?;
        let server_addr = format!("{}:161", host);
        
        // 요청 전송
        socket
            .send_to(&request, &server_addr)
            .context("Failed to send SNMP request")?;

        // 응답 수신 (타임아웃 적용됨)
        let mut buffer = [0u8; 1024];
        let (size, _) = match socket.recv_from(&mut buffer) {
            Ok(result) => result,
            Err(e) => {
                // 타임아웃 또는 네트워크 오류
                if e.kind() == std::io::ErrorKind::TimedOut {
                    anyhow::bail!("SNMP request timeout for {}:{}", host, oid);
                } else {
                    anyhow::bail!("SNMP request failed for {}:{}: {}", host, oid, e);
                }
            }
        };

        self.parse_get_response(&buffer[..size])
    }

    /// SNMP GET 요청 패킷 생성 (BER 인코딩)
    fn build_get_request(&self, oid: &str) -> Result<Vec<u8>> {
        // OID를 숫자 배열로 변환
        let oid_parts: Vec<u32> = oid
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        if oid_parts.is_empty() {
            anyhow::bail!("Invalid OID format");
        }

        let mut packet = Vec::new();

        // OID 인코딩
        let mut oid_bytes = Vec::new();
        // 첫 두 숫자는 특별 처리: (first * 40) + second
        if oid_parts.len() >= 2 {
            let first_two = oid_parts[0] * 40 + oid_parts[1];
            oid_bytes.extend_from_slice(&encode_unsigned_int(first_two));
        }
        // 나머지 숫자들
        for &part in oid_parts.iter().skip(2) {
            oid_bytes.extend_from_slice(&encode_unsigned_int(part));
        }

        // VarBind: SEQUENCE { OID, NULL }
        let mut varbind = Vec::new();
        varbind.extend(encode_oid(&oid_bytes)); // OID
        varbind.extend(encode_null()); // NULL value
        let varbind_encoded = encode_sequence(&varbind);

        // VarBindList: SEQUENCE OF VarBind
        let varbind_list = encode_sequence(&varbind_encoded);

        // PDU: SEQUENCE { request-id, error-status, error-index, varbind-list }
        let mut pdu = Vec::new();
        pdu.extend(encode_integer(self.request_id as i32)); // request-id
        pdu.extend(encode_integer(0)); // error-status
        pdu.extend(encode_integer(0)); // error-index
        pdu.extend(varbind_list); // varbind-list
        let pdu_encoded = encode_sequence(&pdu);
        // PDU 타입: GET-REQUEST (0xa0)
        let mut pdu_with_type = vec![0xa0];
        pdu_with_type.extend(encode_length(pdu_encoded.len()));
        pdu_with_type.extend(pdu_encoded);

        // Community string
        let community_bytes = self.community.as_bytes();
        let community_encoded = encode_octet_string(community_bytes);

        // SNMP Message: SEQUENCE { version, community, PDU }
        packet.extend(encode_integer(1)); // SNMPv2c (version 1)
        packet.extend(community_encoded);
        packet.extend(pdu_with_type);

        // 전체 메시지: SEQUENCE
        let mut message = Vec::new();
        message.extend(encode_sequence(&packet));

        Ok(message)
    }

    /// SNMP 응답 파싱 (BER 디코딩)
    fn parse_get_response(&self, response: &[u8]) -> Result<f64> {
        let mut pos = 0;
        
        // SEQUENCE 헤더 확인
        if pos >= response.len() || response[pos] != 0x30 {
            anyhow::bail!("Invalid SNMP response: expected SEQUENCE");
        }
        pos += 1;
        let (_length, length_bytes) = decode_length(&response[pos..])?;
        pos += length_bytes;
        
        // Version (INTEGER 1)
        pos = skip_ber_value(response, pos)?;
        
        // Community (OCTET STRING)
        pos = skip_ber_value(response, pos)?;
        
        // PDU (GET-RESPONSE, 0xa2)
        if pos >= response.len() || response[pos] != 0xa2 {
            anyhow::bail!("Invalid SNMP response: expected GET-RESPONSE PDU");
        }
        pos += 1;
        let (_pdu_length, pdu_length_bytes) = decode_length(&response[pos..])?;
        pos += pdu_length_bytes;
        
        // request-id, error-status, error-index 건너뛰기
        pos = skip_ber_value(response, pos)?; // request-id
        pos = skip_ber_value(response, pos)?; // error-status
        pos = skip_ber_value(response, pos)?; // error-index
        
        // VarBindList
        if pos >= response.len() || response[pos] != 0x30 {
            anyhow::bail!("Invalid SNMP response: expected VarBindList");
        }
        pos += 1;
        let (_vbl_length, vbl_length_bytes) = decode_length(&response[pos..])?;
        pos += vbl_length_bytes;
        
        // VarBind
        if pos >= response.len() || response[pos] != 0x30 {
            anyhow::bail!("Invalid SNMP response: expected VarBind");
        }
        pos += 1;
        let (_vb_length, vb_length_bytes) = decode_length(&response[pos..])?;
        pos += vb_length_bytes;
        
        // OID 건너뛰기
        pos = skip_ber_value(response, pos)?;
        
        // Value 추출
        if pos >= response.len() {
            anyhow::bail!("Invalid SNMP response: missing value");
        }
        
        let value_type = response[pos];
        pos += 1;
        let (value_length, value_length_bytes) = decode_length(&response[pos..])?;
        pos += value_length_bytes;
        
        if pos + value_length > response.len() {
            anyhow::bail!("Invalid SNMP response: value length exceeds buffer");
        }
        
        let value_bytes = &response[pos..pos + value_length];
        
        // 값 타입에 따라 파싱
        match value_type {
            0x02 => { // INTEGER
                let value = decode_integer(value_bytes)?;
                Ok(value as f64)
            }
            0x42 => { // Counter32
                let value = decode_unsigned_int(value_bytes)?;
                Ok(value as f64)
            }
            0x43 => { // Gauge32
                let value = decode_unsigned_int(value_bytes)?;
                Ok(value as f64)
            }
            0x46 => { // Counter64
                let value = decode_unsigned_int_64(value_bytes)?;
                Ok(value as f64)
            }
            _ => {
                anyhow::bail!("Unsupported SNMP value type: 0x{:02x}", value_type)
            }
        }
    }
}

// BER 인코딩 헬퍼 함수들

fn encode_sequence(data: &[u8]) -> Vec<u8> {
    let mut result = vec![0x30]; // SEQUENCE tag
    result.extend(encode_length(data.len()));
    result.extend(data);
    result
}

fn encode_integer(value: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    
    if value == 0 {
        bytes.push(0);
    } else {
        // 음수 처리
        let is_negative = value < 0;
        let mut abs_val = value.abs() as u32;
        
        // 최소 바이트 수로 인코딩
        while abs_val > 0 || bytes.is_empty() {
            bytes.push((abs_val & 0xff) as u8);
            abs_val >>= 8;
        }
        
        // 음수인 경우 2의 보수
        if is_negative {
            for byte in &mut bytes {
                *byte = !*byte;
            }
            let mut carry = 1;
            for byte in &mut bytes {
                let sum = (*byte as u16) + carry;
                *byte = (sum & 0xff) as u8;
                carry = sum >> 8;
            }
        }
        
        bytes.reverse();
    }
    
    let mut result = vec![0x02]; // INTEGER tag
    result.extend(encode_length(bytes.len()));
    result.extend(bytes);
    result
}

fn encode_unsigned_int(value: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut val = value;
    
    if value == 0 {
        bytes.push(0);
    } else {
        while val > 0 {
            bytes.push((val & 0xff) as u8);
            val >>= 8;
        }
        bytes.reverse();
    }
    
    bytes
}

fn encode_unsigned_int_64(value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut val = value;
    
    if value == 0 {
        bytes.push(0);
    } else {
        while val > 0 {
            bytes.push((val & 0xff) as u8);
            val >>= 8;
        }
        bytes.reverse();
    }
    
    bytes
}

fn encode_octet_string(data: &[u8]) -> Vec<u8> {
    let mut result = vec![0x04]; // OCTET STRING tag
    result.extend(encode_length(data.len()));
    result.extend(data);
    result
}

fn encode_oid(oid_bytes: &[u8]) -> Vec<u8> {
    let mut result = vec![0x06]; // OBJECT IDENTIFIER tag
    result.extend(encode_length(oid_bytes.len()));
    result.extend(oid_bytes);
    result
}

fn encode_null() -> Vec<u8> {
    vec![0x05, 0x00] // NULL tag, length 0
}

fn encode_length(length: usize) -> Vec<u8> {
    if length < 128 {
        vec![length as u8]
    } else {
        let mut bytes = Vec::new();
        let mut len = length;
        while len > 0 {
            bytes.push((len & 0xff) as u8);
            len >>= 8;
        }
        bytes.reverse();
        let mut result = vec![0x80 | bytes.len() as u8];
        result.extend(bytes);
        result
    }
}

fn decode_length(data: &[u8]) -> Result<(usize, usize)> {
    if data.is_empty() {
        anyhow::bail!("Invalid length encoding: empty");
    }
    
    if (data[0] & 0x80) == 0 {
        // 단일 바이트 길이
        Ok((data[0] as usize, 1))
    } else {
        // 다중 바이트 길이
        let length_of_length = (data[0] & 0x7f) as usize;
        if length_of_length == 0 || length_of_length > 4 {
            anyhow::bail!("Invalid length encoding: length_of_length = {}", length_of_length);
        }
        if data.len() < 1 + length_of_length {
            anyhow::bail!("Invalid length encoding: insufficient data");
        }
        
        let mut length = 0usize;
        for i in 1..=length_of_length {
            length = (length << 8) | data[i] as usize;
        }
        Ok((length, 1 + length_of_length))
    }
}

fn skip_ber_value(data: &[u8], pos: usize) -> Result<usize> {
    if pos >= data.len() {
        anyhow::bail!("Invalid position: beyond data length");
    }
    
    let _tag = data[pos];
    let mut new_pos = pos + 1;
    
    let (length, length_bytes) = decode_length(&data[new_pos..])?;
    new_pos += length_bytes;
    
    if new_pos + length > data.len() {
        anyhow::bail!("Invalid BER value: length exceeds buffer");
    }
    
    Ok(new_pos + length)
}

fn decode_integer(data: &[u8]) -> Result<i32> {
    if data.is_empty() {
        anyhow::bail!("Invalid integer: empty");
    }
    
    let is_negative = (data[0] & 0x80) != 0;
    let mut value = 0i32;
    
    for &byte in data {
        value = (value << 8) | (byte as i32);
    }
    
    // 음수 처리 (2의 보수)
    if is_negative {
        let mask = !((1u32 << (data.len() * 8)) - 1) as i32;
        value |= mask;
    }
    
    Ok(value)
}

fn decode_unsigned_int(data: &[u8]) -> Result<u32> {
    if data.is_empty() {
        anyhow::bail!("Invalid unsigned integer: empty");
    }
    
    let mut value = 0u32;
    for &byte in data {
        value = (value << 8) | (byte as u32);
    }
    Ok(value)
}

fn decode_unsigned_int_64(data: &[u8]) -> Result<u64> {
    if data.is_empty() {
        anyhow::bail!("Invalid unsigned integer: empty");
    }
    
    let mut value = 0u64;
    for &byte in data {
        value = (value << 8) | (byte as u64);
    }
    Ok(value)
}

/// 비동기 SNMP GET (토키오 런타임 사용)
pub async fn snmp_get_async(
    host: &str,
    community: &str,
    oid: &str,
) -> Result<f64> {
    let client = SnmpClient::new(community.to_string());
    let host = host.to_string();
    let oid = oid.to_string();
    let timeout = Duration::from_secs(3); // 기본 타임아웃 3초
    
    // 블로킹 작업을 스레드 풀에서 실행하고 타임아웃 적용
    tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || client.get(&host, &oid))
    )
    .await
    .context("SNMP request timeout")?
    .context("SNMP task failed")?
}
