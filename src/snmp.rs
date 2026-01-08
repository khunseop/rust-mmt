use anyhow::{Context, Result};
use std::net::{UdpSocket, ToSocketAddrs};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

// 전역 요청 ID 카운터 (스레드 안전)
static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// SNMP 클라이언트 (SNMP v2c)
pub struct SnmpClient {
    community: String,
    timeout: Duration,
}

impl SnmpClient {
    pub fn new(community: String) -> Self {
        Self {
            community,
            timeout: Duration::from_secs(10),
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

    /// OID 문자열을 바이트 벡터로 변환 (BER 인코딩)
    fn oid_to_bytes(oid: &str) -> Result<Vec<u8>> {
        let parts: Vec<u32> = oid
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        
        if parts.is_empty() {
            anyhow::bail!("Invalid OID format: {}", oid);
        }

        let mut result = Vec::new();
        
        // 첫 두 부분을 하나의 바이트로 인코딩: (first * 40) + second
        if parts.len() >= 2 {
            let first_byte = (parts[0] * 40 + parts[1]) as u8;
            result.push(first_byte);
        } else if parts.len() == 1 {
            result.push((parts[0] * 40) as u8);
        }
        
        // 나머지 부분들을 베이스 128 인코딩
        for &part in parts.iter().skip(2) {
            let mut value = part;
            let mut bytes = Vec::new();
            while value >= 128 {
                bytes.push((value % 128) as u8 | 0x80);
                value /= 128;
            }
            bytes.push(value as u8);
            result.extend(bytes.iter().rev());
        }
        
        Ok(result)
    }

    /// BER 인코딩: 길이 인코딩
    fn encode_length(len: usize) -> Vec<u8> {
        if len < 128 {
            vec![len as u8]
        } else {
            let mut bytes = Vec::new();
            let mut value = len;
            while value > 0 {
                bytes.push((value & 0xFF) as u8);
                value >>= 8;
            }
            bytes.reverse();
            vec![0x80 | bytes.len() as u8]
                .into_iter()
                .chain(bytes.into_iter())
                .collect()
        }
    }

    /// BER 인코딩: 정수 인코딩
    fn encode_integer(value: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut val = value as u32;
        
        // 최소 1바이트는 필요
        loop {
            bytes.push((val & 0xFF) as u8);
            val >>= 8;
            if val == 0 && (bytes.len() == 1 || (bytes[bytes.len() - 1] & 0x80) == 0) {
                break;
            }
        }
        bytes.reverse();
        
        // 음수 처리 (2의 보수)
        if value < 0 {
            for byte in &mut bytes {
                *byte = !*byte;
            }
            if bytes[0] & 0x80 == 0 {
                bytes.insert(0, 0xFF);
            }
        }
        
        vec![0x02] // INTEGER 태그
            .into_iter()
            .chain(Self::encode_length(bytes.len()).into_iter())
            .chain(bytes.into_iter())
            .collect()
    }

    /// BER 인코딩: 옥텟 스트링 인코딩
    fn encode_octet_string(data: &[u8]) -> Vec<u8> {
        vec![0x04] // OCTET STRING 태그
            .into_iter()
            .chain(Self::encode_length(data.len()).into_iter())
            .chain(data.iter().cloned())
            .collect()
    }

    /// BER 인코딩: 시퀀스 인코딩
    fn encode_sequence(items: &[Vec<u8>]) -> Vec<u8> {
        let content: Vec<u8> = items.iter().flat_map(|item| item.clone()).collect();
        vec![0x30] // SEQUENCE 태그
            .into_iter()
            .chain(Self::encode_length(content.len()).into_iter())
            .chain(content.into_iter())
            .collect()
    }

    /// BER 인코딩: NULL 인코딩
    fn encode_null() -> Vec<u8> {
        vec![0x05, 0x00] // NULL 태그, 길이 0
    }

    /// SNMP v2c GET 요청 생성
    fn build_get_request(&self, request_id: u32, oid: &str) -> Result<Vec<u8>> {
        let oid_bytes = Self::oid_to_bytes(oid)?;
        
        // VarBind: OID + NULL (GET 요청)
        let varbind = Self::encode_sequence(&[
            Self::encode_octet_string(&oid_bytes), // OID
            Self::encode_null(), // NULL value
        ]);
        
        // VarBindList
        let varbind_list = Self::encode_sequence(&[varbind]);
        
        // GetRequest-PDU
        let pdu = Self::encode_sequence(&[
            Self::encode_integer(request_id as i32), // request-id
            Self::encode_integer(0), // error-status
            Self::encode_integer(0), // error-index
            varbind_list, // variable-bindings
        ]);
        
        // SNMP-PDU (GetRequest)
        let snmp_pdu = Self::encode_sequence(&[
            Self::encode_integer(1), // PDU type: GetRequest (1)
            pdu,
        ]);
        
        // SNMP Message
        let message = Self::encode_sequence(&[
            Self::encode_integer(1), // version: v2c (1)
            Self::encode_octet_string(self.community.as_bytes()), // community
            snmp_pdu, // data
        ]);
        
        Ok(message)
    }

    /// BER 디코딩: 길이 디코딩
    fn decode_length(data: &[u8], offset: &mut usize) -> Result<usize> {
        if *offset >= data.len() {
            anyhow::bail!("Unexpected end of data while decoding length");
        }
        
        let first_byte = data[*offset];
        *offset += 1;
        
        if (first_byte & 0x80) == 0 {
            // 단일 바이트 길이
            Ok(first_byte as usize)
        } else {
            // 다중 바이트 길이
            let length_of_length = (first_byte & 0x7F) as usize;
            if length_of_length == 0 || length_of_length > 4 {
                anyhow::bail!("Invalid length encoding");
            }
            
            let mut length = 0usize;
            for _ in 0..length_of_length {
                if *offset >= data.len() {
                    anyhow::bail!("Unexpected end of data while decoding length");
                }
                length = (length << 8) | data[*offset] as usize;
                *offset += 1;
            }
            Ok(length)
        }
    }

    /// BER 디코딩: 태그 확인 및 건너뛰기
    fn skip_tag(data: &[u8], offset: &mut usize, expected_tag: u8) -> Result<()> {
        if *offset >= data.len() {
            anyhow::bail!("Unexpected end of data");
        }
        if data[*offset] != expected_tag {
            anyhow::bail!("Unexpected tag: expected 0x{:02x}, got 0x{:02x}", expected_tag, data[*offset]);
        }
        *offset += 1;
        Ok(())
    }

    /// BER 디코딩: 정수 디코딩
    fn decode_integer(data: &[u8], offset: &mut usize) -> Result<i64> {
        Self::skip_tag(data, offset, 0x02)?; // INTEGER 태그
        let length = Self::decode_length(data, offset)?;
        
        if *offset + length > data.len() {
            anyhow::bail!("Unexpected end of data while decoding integer");
        }
        
        let mut value: i64 = 0;
        let is_negative = length > 0 && (data[*offset] & 0x80) != 0;
        
        for i in 0..length {
            value = (value << 8) | data[*offset + i] as i64;
        }
        
        // 음수 처리 (2의 보수)
        if is_negative {
            let mask = (1u64 << (length * 8)) - 1;
            value = value - (mask as i64 + 1);
        }
        
        *offset += length;
        Ok(value)
    }

    /// BER 디코딩: 옥텟 스트링 디코딩
    fn decode_octet_string(data: &[u8], offset: &mut usize) -> Result<Vec<u8>> {
        Self::skip_tag(data, offset, 0x04)?; // OCTET STRING 태그
        let length = Self::decode_length(data, offset)?;
        
        if *offset + length > data.len() {
            anyhow::bail!("Unexpected end of data while decoding octet string");
        }
        
        let result = data[*offset..*offset + length].to_vec();
        *offset += length;
        Ok(result)
    }

    /// SNMP 응답 파싱
    #[allow(unused_assignments)]
    fn parse_response(&self, data: &[u8], expected_request_id: u32) -> Result<f64> {
        let mut offset = 0;
        
        // SEQUENCE (SNMP Message)
        Self::skip_tag(data, &mut offset, 0x30)?;
        let _msg_length = Self::decode_length(data, &mut offset)?;
        
        // version (INTEGER)
        let version = Self::decode_integer(data, &mut offset)?;
        if version != 1 {
            anyhow::bail!("Unsupported SNMP version: {}", version);
        }
        
        // community (OCTET STRING)
        let _community = Self::decode_octet_string(data, &mut offset)?;
        
        // SEQUENCE (SNMP-PDU)
        Self::skip_tag(data, &mut offset, 0x30)?;
        let _pdu_length = Self::decode_length(data, &mut offset)?;
        
        // PDU type (INTEGER) - GetResponse는 2
        let pdu_type = Self::decode_integer(data, &mut offset)?;
        if pdu_type != 2 {
            anyhow::bail!("Unexpected PDU type: expected GetResponse (2), got {}", pdu_type);
        }
        
        // SEQUENCE (GetResponse-PDU)
        Self::skip_tag(data, &mut offset, 0x30)?;
        let _pdu_content_length = Self::decode_length(data, &mut offset)?;
        
        // request-id (INTEGER)
        let request_id = Self::decode_integer(data, &mut offset)?;
        if request_id != expected_request_id as i64 {
            anyhow::bail!("Request ID mismatch: expected {}, got {}", expected_request_id, request_id);
        }
        
        // error-status (INTEGER)
        let error_status = Self::decode_integer(data, &mut offset)?;
        if error_status != 0 {
            let error_msg = match error_status {
                1 => "tooBig",
                2 => "noSuchName",
                3 => "badValue",
                4 => "readOnly",
                5 => "genErr",
                _ => "unknown",
            };
            anyhow::bail!("SNMP error: {} (error-status: {})", error_msg, error_status);
        }
        
        // error-index (INTEGER)
        let _error_index = Self::decode_integer(data, &mut offset)?;
        
        // SEQUENCE (VarBindList)
        Self::skip_tag(data, &mut offset, 0x30)?;
        let _varbind_list_length = Self::decode_length(data, &mut offset)?;
        
        // SEQUENCE (VarBind)
        Self::skip_tag(data, &mut offset, 0x30)?;
        let _varbind_length = Self::decode_length(data, &mut offset)?;
        
        // OID (OCTET STRING) - 건너뛰기
        let _oid = Self::decode_octet_string(data, &mut offset)?;
        
        // 값 추출
        if offset >= data.len() {
            anyhow::bail!("Unexpected end of data while reading value");
        }
        
        let value_tag = data[offset];
        match value_tag {
            0x02 => {
                // INTEGER
                let value = Self::decode_integer(data, &mut offset)?;
                Ok(value as f64)
            }
            0x41 => {
                // Counter32 (0x41 = APPLICATION 1)
                offset += 1; // 태그 건너뛰기
                let length = Self::decode_length(data, &mut offset)?;
                if offset + length > data.len() {
                    anyhow::bail!("Unexpected end of data");
                }
                let mut value: u64 = 0;
                for i in 0..length {
                    value = (value << 8) | data[offset + i] as u64;
                }
                offset += length;
                Ok(value as f64)
            }
            0x42 => {
                // Gauge32 (0x42 = APPLICATION 2)
                offset += 1;
                let length = Self::decode_length(data, &mut offset)?;
                if offset + length > data.len() {
                    anyhow::bail!("Unexpected end of data");
                }
                let mut value: u64 = 0;
                for i in 0..length {
                    value = (value << 8) | data[offset + i] as u64;
                }
                offset += length;
                Ok(value as f64)
            }
            0x43 => {
                // TimeTicks (0x43 = APPLICATION 3)
                offset += 1;
                let length = Self::decode_length(data, &mut offset)?;
                if offset + length > data.len() {
                    anyhow::bail!("Unexpected end of data");
                }
                let mut value: u64 = 0;
                for i in 0..length {
                    value = (value << 8) | data[offset + i] as u64;
                }
                offset += length;
                Ok(value as f64)
            }
            0x46 => {
                // Counter64 (0x46 = APPLICATION 6)
                offset += 1;
                let length = Self::decode_length(data, &mut offset)?;
                if offset + length > data.len() {
                    anyhow::bail!("Unexpected end of data");
                }
                let mut value: u64 = 0;
                for i in 0..length {
                    value = (value << 8) | data[offset + i] as u64;
                }
                offset += length;
                Ok(value as f64)
            }
            0x05 => {
                // NULL
                offset += 2; // NULL 태그 + 길이 0
                anyhow::bail!("NULL value in SNMP response")
            }
            0x80 => {
                // NoSuchObject
                anyhow::bail!("NoSuchObject in SNMP response")
            }
            0x81 => {
                // NoSuchInstance
                anyhow::bail!("NoSuchInstance in SNMP response")
            }
            0x82 => {
                // EndOfMibView
                anyhow::bail!("EndOfMibView in SNMP response")
            }
            _ => {
                anyhow::bail!("Unsupported SNMP value type: 0x{:02x}", value_tag)
            }
        }
    }

    /// SNMP GET 요청을 보내고 값을 반환합니다.
    pub fn get(&self, host: &str, oid: &str) -> Result<f64> {
        let request_id = Self::next_request_id();
        
        // SNMP 요청 생성
        let request = self.build_get_request(request_id, oid)
            .context("Failed to build SNMP request")?;
        
        // UDP 소켓 생성
        let socket = UdpSocket::bind("0.0.0.0:0")
            .or_else(|_| UdpSocket::bind("127.0.0.1:0"))
            .context("Failed to bind UDP socket")?;
        
        socket
            .set_read_timeout(Some(self.timeout))
            .context("Failed to set read timeout")?;
        
        socket
            .set_write_timeout(Some(self.timeout))
            .context("Failed to set write timeout")?;
        
        // 서버 주소 파싱
        let server_addr: std::net::SocketAddr = format!("{}:161", host).parse()
            .or_else(|_| {
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
            .context(format!("Failed to send SNMP request to {}", host))?;
        
        if sent_bytes != request.len() {
            anyhow::bail!("Partial send: sent {}/{} bytes", sent_bytes, request.len());
        }
        
        // 응답 수신
        let mut buffer = [0u8; 1500];
        let (size, _) = match socket.recv_from(&mut buffer) {
            Ok(result) => result,
            Err(e) => {
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
        
        // 응답 파싱
        self.parse_response(&buffer[..size], request_id)
            .with_context(|| format!("Failed to parse SNMP response from {} for OID {}", host, oid))
    }
}

/// 비동기 SNMP GET (토키오 런타임 사용)
pub async fn snmp_get_async(
    host: &str,
    community: &str,
    oid: &str,
) -> Result<f64> {
    let timeout = Duration::from_secs(10);
    let host_str = host.to_string();
    let oid_str = oid.to_string();
    let community_str = community.to_string();
    let community_for_error = community_str.clone();
    
    let client = SnmpClient::new(community_str)
        .with_timeout(timeout);
    
    let host_for_error = host_str.clone();
    let oid_for_error = oid_str.clone();
    
    let tokio_timeout = timeout + Duration::from_secs(2);
    
    match tokio::time::timeout(
        tokio_timeout,
        tokio::task::spawn_blocking(move || {
            client.get(&host_str, &oid_str)
                .with_context(|| format!(
                    "SNMP GET failed: host={}, oid={}, community={}, timeout={:?}",
                    host_str, oid_str, community_for_error, timeout
                ))
        })
    )
    .await {
        Ok(Ok(Ok(value))) => Ok(value),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(e)) => Err(anyhow::anyhow!("SNMP task execution failed: {}", e)),
        Err(_) => Err(anyhow::anyhow!(
            "SNMP request timeout: no response from {} for OID {} after {:?} (UDP timeout: {:?})",
            host_for_error, oid_for_error, tokio_timeout, timeout
        )),
    }
}
