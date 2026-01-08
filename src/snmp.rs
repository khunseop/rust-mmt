use anyhow::{Context, Result};
use rasn_snmp::v2c::{GetRequestPdu, Message, Pdu, VarBind, VarBindList};
use rasn::types::ObjectIdentifier;
use std::net::{UdpSocket, ToSocketAddrs};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

// 전역 요청 ID 카운터 (스레드 안전)
static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// SNMP 클라이언트 (rasn-snmp 사용)
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

    /// SNMP GET 요청을 보내고 값을 반환합니다.
    pub fn get(&self, host: &str, oid: &str) -> Result<f64> {
        let request_id = Self::next_request_id();
        
        // OID 파싱
        let oid_parts: Vec<u32> = oid
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        
        if oid_parts.is_empty() {
            anyhow::bail!("Invalid OID format");
        }
        
        // OID를 ObjectIdentifier로 변환
        let oid_obj = ObjectIdentifier::from(oid_parts.as_slice());
        
        // VarBind 생성 (GET 요청에서는 value가 Unspecified)
        let varbind = VarBind {
            name: oid_obj.clone(),
            value: rasn_snmp::v2c::VarBindValue::Unspecified,
        };
        
        // VarBindList 생성
        let varbind_list = VarBindList(vec![varbind]);
        
        // PDU 생성
        let pdu = Pdu::GetRequest(GetRequestPdu {
            request_id: request_id as i32,
            error_status: 0,
            error_index: 0,
            variable_bindings: varbind_list,
        });
        
        // SNMP 메시지 생성
        let message = Message {
            version: rasn_snmp::v2c::Version::V2c,
            community: self.community.as_bytes().to_vec(),
            data: pdu,
        };
        
        // BER 인코딩
        let request = rasn::ber::encode(&message)
            .context("Failed to encode SNMP message")?;
        
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
        
        // BER 디코딩
        let response: Message = rasn::ber::decode(&buffer[..size])
            .context("Failed to decode SNMP response")?;
        
        // 응답 처리
        match response.data {
            Pdu::GetResponse(pdu) => {
                // 에러 상태 확인
                if pdu.error_status != 0 {
                    let error_msg = match pdu.error_status {
                        1 => "tooBig",
                        2 => "noSuchName",
                        3 => "badValue",
                        4 => "readOnly",
                        5 => "genErr",
                        _ => "unknown",
                    };
                    anyhow::bail!("SNMP error: {} (error-status: {}, error-index: {})",
                        error_msg, pdu.error_status, pdu.error_index);
                }
                
                // 첫 번째 VarBind에서 값 추출
                if let Some(varbind) = pdu.variable_bindings.0.first() {
                    match &varbind.value {
                        rasn_snmp::v2c::VarBindValue::Value(value) => {
                            match value {
                                rasn_snmp::v2c::Value::Integer(i) => Ok(*i as f64),
                                rasn_snmp::v2c::Value::Unsigned32(u) => Ok(*u as f64),
                                rasn_snmp::v2c::Value::Counter32(c) => Ok(*c as f64),
                                rasn_snmp::v2c::Value::Gauge32(g) => Ok(*g as f64),
                                rasn_snmp::v2c::Value::Counter64(c) => Ok(*c as f64),
                                rasn_snmp::v2c::Value::TimeTicks(t) => Ok(*t as f64),
                                rasn_snmp::v2c::Value::IpAddress(_) => anyhow::bail!("IP address value not supported"),
                                rasn_snmp::v2c::Value::OctetString(_) => anyhow::bail!("Octet string value not supported"),
                                rasn_snmp::v2c::Value::ObjectIdentifier(_) => anyhow::bail!("OID value not supported"),
                                _ => anyhow::bail!("Unsupported SNMP value type"),
                            }
                        }
                        rasn_snmp::v2c::VarBindValue::Unspecified => anyhow::bail!("Unspecified value in SNMP response"),
                        rasn_snmp::v2c::VarBindValue::NoSuchObject => anyhow::bail!("NoSuchObject in SNMP response"),
                        rasn_snmp::v2c::VarBindValue::NoSuchInstance => anyhow::bail!("NoSuchInstance in SNMP response"),
                        rasn_snmp::v2c::VarBindValue::EndOfMibView => anyhow::bail!("EndOfMibView in SNMP response"),
                    }
                } else {
                    anyhow::bail!("Empty VarBindList in SNMP response")
                }
            }
            _ => anyhow::bail!("Unexpected PDU type in SNMP response"),
        }
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
    let community_debug = community.to_string();
    
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
                    host_str, oid_str, community_debug, timeout
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
