use anyhow::{Context, Result};
use std::net::{UdpSocket, ToSocketAddrs};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

// 전역 요청 ID 카운터 (스레드 안전)
static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// SNMP 클라이언트
pub struct SnmpClient {
    community: String,
    timeout: Duration,
}

impl SnmpClient {
    pub fn new(community: String) -> Self {
        Self {
            community,
            timeout: Duration::from_secs(5), // 기본값을 5초로 증가
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// 고유한 요청 ID 생성
    fn next_request_id() -> u32 {
        REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// SNMP GET 요청을 보내고 값을 반환합니다.
    /// OID는 점으로 구분된 문자열 형식 (예: "1.3.6.1.4.1.2021.11.11.0")
    pub fn get(&self, host: &str, oid: &str) -> Result<f64> {
        // 고유한 요청 ID 생성
        let request_id = Self::next_request_id();
        
        // IPv4 주소로 명시적으로 바인딩
        let socket = UdpSocket::bind("0.0.0.0:0")
            .or_else(|_| UdpSocket::bind("127.0.0.1:0"))
            .context("Failed to bind UDP socket")?;
        
        // 타임아웃 설정
        socket
            .set_read_timeout(Some(self.timeout))
            .context("Failed to set read timeout")?;
        
        // 송신 타임아웃도 설정
        socket
            .set_write_timeout(Some(self.timeout))
            .context("Failed to set write timeout")?;

        // SNMPv2c GET 요청 생성
        let request = self.build_get_request(oid, request_id)?;
        
        // 패킷 크기 확인 (SNMPv2c는 최대 1472 바이트 권장)
        if request.len() > 1472 {
            anyhow::bail!("SNMP request packet too large: {} bytes", request.len());
        }
        
        // 서버 주소 파싱 (IPv4/IPv6 지원)
        let server_addr: std::net::SocketAddr = format!("{}:161", host).parse()
            .or_else(|_| {
                // 호스트명이면 DNS 조회 시도
                (host, 161u16).to_socket_addrs()
                    .and_then(|mut addrs| addrs.next().ok_or(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Could not resolve host"
                    )))
            })
            .context(format!("Failed to resolve server address: {}", host))?;
        
        // 요청 전송
        let sent_bytes = socket
            .send_to(&request, &server_addr)
            .context(format!("Failed to send SNMP request to {} ({} bytes)", host, request.len()))?;
        
        if sent_bytes != request.len() {
            anyhow::bail!("Partial send: sent {}/{} bytes to {}", sent_bytes, request.len(), host);
        }

        // 응답 수신 (타임아웃 적용됨)
        let mut buffer = [0u8; 1500]; // 더 큰 버퍼 사용
        let (size, _) = match socket.recv_from(&mut buffer) {
            Ok(result) => result,
            Err(e) => {
                // 타임아웃 또는 네트워크 오류
                if e.kind() == std::io::ErrorKind::TimedOut {
                    anyhow::bail!(
                        "SNMP timeout: no response from {} for OID {} (timeout: {:?}, request_id: {})",
                        host, oid, self.timeout, request_id
                    );
                } else {
                    anyhow::bail!(
                        "SNMP network error for {}:{}: {} (request_id: {})",
                        host, oid, e, request_id
                    );
                }
            }
        };

        if size == 0 {
            anyhow::bail!("Empty SNMP response from {}", host);
        }

        self.parse_get_response(&buffer[..size], request_id)
    }

    /// SNMP GET 요청 패킷 생성 (BER 인코딩)
    fn build_get_request(&self, oid: &str, request_id: u32) -> Result<Vec<u8>> {
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
        pdu.extend(encode_integer(request_id as i32)); // request-id
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
    fn parse_get_response(&self, response: &[u8], expected_request_id: u32) -> Result<f64> {
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
        
        // PDU 내용은 SEQUENCE로 시작합니다
        if pos >= response.len() || response[pos] != 0x30 {
            anyhow::bail!("Invalid SNMP response: expected SEQUENCE in PDU (found: 0x{:02x})", 
                if pos < response.len() { response[pos] } else { 0 });
        }
        pos += 1;
        let (_pdu_seq_length, pdu_seq_length_bytes) = decode_length(&response[pos..])?;
        pos += pdu_seq_length_bytes;
        
        // request-id 확인 및 검증
        if pos >= response.len() || response[pos] != 0x02 {
            anyhow::bail!("Invalid SNMP response: expected INTEGER for request-id (found: 0x{:02x} at pos {})", 
                if pos < response.len() { response[pos] } else { 0 }, pos);
        }
        pos += 1;
        let (req_id_length, req_id_length_bytes) = decode_length(&response[pos..])?;
        pos += req_id_length_bytes;
        if pos + req_id_length > response.len() {
            anyhow::bail!("Invalid SNMP response: request-id length exceeds buffer");
        }
        let req_id_bytes = &response[pos..pos + req_id_length];
        let received_request_id = decode_integer(req_id_bytes)?;
        pos += req_id_length;
        
        // 요청 ID 일치 확인 (선택적, 디버깅용)
        if received_request_id != expected_request_id as i32 {
            // 경고만 하고 계속 진행 (일부 SNMP 구현에서는 다를 수 있음)
        }
        
        // error-status 확인 (중요!)
        if pos >= response.len() || response[pos] != 0x02 {
            anyhow::bail!("Invalid SNMP response: expected INTEGER for error-status");
        }
        pos += 1;
        let (error_status_length, error_status_length_bytes) = decode_length(&response[pos..])?;
        pos += error_status_length_bytes;
        if pos + error_status_length > response.len() {
            anyhow::bail!("Invalid SNMP response: error-status length exceeds buffer");
        }
        let error_status_bytes = &response[pos..pos + error_status_length];
        let error_status = decode_integer(error_status_bytes)?;
        pos += error_status_length;
        
        // error-index 확인
        if pos >= response.len() || response[pos] != 0x02 {
            anyhow::bail!("Invalid SNMP response: expected INTEGER for error-index");
        }
        pos += 1;
        let (error_index_length, error_index_length_bytes) = decode_length(&response[pos..])?;
        pos += error_index_length_bytes;
        if pos + error_index_length > response.len() {
            anyhow::bail!("Invalid SNMP response: error-index length exceeds buffer");
        }
        let error_index_bytes = &response[pos..pos + error_index_length];
        let error_index = decode_integer(error_index_bytes)?;
        pos += error_index_length;
        
        // SNMP 에러 상태 확인
        if error_status != 0 {
            let error_msg = match error_status {
                1 => "tooBig - 응답이 너무 큼",
                2 => "noSuchName - 요청한 OID가 존재하지 않음",
                3 => "badValue - 잘못된 값",
                4 => "readOnly - 읽기 전용 OID",
                5 => "genErr - 일반 오류",
                _ => "알 수 없는 SNMP 오류",
            };
            anyhow::bail!("SNMP error: {} (error-status: {}, error-index: {})", 
                error_msg, error_status, error_index);
        }
        
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
    // SNMP 타임아웃 설정 (UDP 소켓 레벨)
    let snmp_timeout = Duration::from_secs(10); // 10초로 증가
    let host_str = host.to_string();
    let oid_str = oid.to_string();
    let community_str = community.to_string();
    let community_debug = community.to_string();
    
    // 타임아웃이 명시적으로 설정된 클라이언트 생성
    let client = SnmpClient::new(community_str)
        .with_timeout(snmp_timeout);
    
    // 타임아웃 메시지를 위한 복사본
    let host_for_error = host_str.clone();
    let oid_for_error = oid_str.clone();
    
    // 토키오 타임아웃은 SNMP 타임아웃보다 충분히 길게 설정
    // (SNMP 타임아웃 + 스레드 풀 오버헤드 + 여유 시간)
    let tokio_timeout = snmp_timeout + Duration::from_secs(3);
    
    // 블로킹 작업을 스레드 풀에서 실행하고 타임아웃 적용
    match tokio::time::timeout(
        tokio_timeout,
        tokio::task::spawn_blocking(move || {
            client.get(&host_str, &oid_str)
                .with_context(|| format!(
                    "SNMP GET failed: host={}, oid={}, community={}, timeout={:?}",
                    host_str, oid_str, community_debug, snmp_timeout
                ))
        })
    )
    .await {
        Ok(Ok(Ok(value))) => Ok(value),
        Ok(Ok(Err(e))) => Err(e), // SNMP 에러
        Ok(Err(e)) => Err(anyhow::anyhow!("SNMP task execution failed: {}", e)),
        Err(_) => Err(anyhow::anyhow!(
            "SNMP request timeout: no response from {} for OID {} after {:?} (UDP timeout: {:?})",
            host_for_error, oid_for_error, tokio_timeout, snmp_timeout
        )),
    }
}
