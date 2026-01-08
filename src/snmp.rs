use anyhow::{Context, Result};
use std::time::Duration;

/// SNMP 클라이언트 (SNMP v2c)
pub struct SnmpClient {
    community: Vec<u8>,
    timeout: Duration,
}

impl SnmpClient {
    pub fn new(community: String) -> Self {
        Self {
            community: community.into_bytes(),
            timeout: Duration::from_secs(10),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// OID 문자열을 u32 배열로 변환
    fn parse_oid(oid: &str) -> Result<Vec<u32>> {
        oid.split('.')
            .map(|s| {
                s.parse::<u32>()
                    .with_context(|| format!("Invalid OID part: {}", s))
            })
            .collect()
    }

    /// SNMP GET 요청을 보내고 값을 반환합니다.
    pub fn get(&self, host: &str, oid: &str) -> Result<f64> {
        use snmp::{SyncSession, Value};

        // OID 파싱
        let oid_vec = Self::parse_oid(oid)
            .with_context(|| format!("Failed to parse OID: {}", oid))?;

        // 호스트 주소에 포트 추가 (없으면 기본값 161)
        let agent_addr = if host.contains(':') {
            host.to_string()
        } else {
            format!("{}:161", host)
        };

        // SNMP 세션 생성
        let mut sess = SyncSession::new(
            &agent_addr,
            &self.community,
            Some(self.timeout),
            0, // retries
        )
        .map_err(|e| anyhow::anyhow!("Failed to create SNMP session for {}: {:?}", host, e))?;

        // SNMP GET 요청 수행
        let mut response = sess
            .get(&oid_vec)
            .map_err(|e| anyhow::anyhow!("SNMP GET failed for host: {}, OID: {}: {:?}", host, oid, e))?;

        // 응답에서 값 추출
        if let Some((_oid, value)) = response.varbinds.next() {
            match value {
                Value::Integer(i) => Ok(i as f64),
                Value::Counter32(c) => Ok(c as f64),
                Value::Unsigned32(u) => Ok(u as f64), // Gauge32는 Unsigned32로 처리
                Value::Timeticks(t) => Ok(t as f64),
                Value::Counter64(c) => Ok(c as f64),
                Value::OctetString(_) => {
                    anyhow::bail!("OID {} returned OctetString, expected numeric value", oid)
                }
                Value::Null => {
                    anyhow::bail!("OID {} returned NULL value", oid)
                }
                _ => {
                    anyhow::bail!("OID {} returned unsupported value type: {:?}", oid, value)
                }
            }
        } else {
            anyhow::bail!("Empty SNMP response from {} for OID {}", host, oid)
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
    
    let client = SnmpClient::new(community_str)
        .with_timeout(timeout);
    
    let host_for_error = host_str.clone();
    let oid_for_error = oid_str.clone();
    let community_for_error = community.to_string();
    
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
            "SNMP request timeout: no response from {} for OID {} after {:?}",
            host_for_error, oid_for_error, tokio_timeout
        )),
    }
}
