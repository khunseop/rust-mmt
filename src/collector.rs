use crate::app::{Proxy, ResourceData};
use crate::snmp::snmp_get_async;
use crate::ssh::SshClient;
use anyhow::Result;
use chrono::Local;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;

/// 자원 수집기
pub struct ResourceCollector {
    oids: HashMap<String, String>, // key -> OID 매핑
    community: String,
}

impl ResourceCollector {
    pub fn new(oids: HashMap<String, String>, community: String) -> Self {
        Self { oids, community }
    }

    /// 프록시의 자원 사용률을 수집합니다.
    pub async fn collect_for_proxy(&self, proxy: &Proxy) -> Result<ResourceData> {
        let mut cpu: Option<f64> = None;
        let mut mem: Option<f64> = None;

        // CPU 수집 (SNMP)
        if let Some(cpu_oid) = self.oids.get("cpu") {
            if !cpu_oid.trim().is_empty() && !cpu_oid.eq_ignore_ascii_case("ssh") {
                match snmp_get_async(&proxy.host, &self.community, cpu_oid).await {
                    Ok(value) => cpu = Some(value),
                    Err(e) => {
                        log_error(&format!("SNMP CPU collection failed for {}: {}", proxy.host, e));
                        // 실패해도 계속 진행 (CPU는 None으로 유지)
                    }
                }
            }
        }

        // 메모리 수집 (SNMP 또는 SSH)
        if let Some(mem_oid) = self.oids.get("mem") {
            if mem_oid.eq_ignore_ascii_case("ssh") {
                // SSH를 통한 메모리 수집
                let ssh_client = SshClient::new(
                    proxy.host.clone(),
                    proxy.port,
                    proxy.username.clone(),
                    proxy.password.clone(),
                );
                match ssh_client.get_memory_percent().await {
                    Ok(value) => mem = Some(value),
                    Err(e) => {
                        log_error(&format!("SSH memory collection failed for {}: {}", proxy.host, e));
                        // 실패해도 계속 진행 (메모리는 None으로 유지)
                    }
                }
            } else if !mem_oid.trim().is_empty() {
                // SNMP를 통한 메모리 수집
                match snmp_get_async(&proxy.host, &self.community, mem_oid).await {
                    Ok(value) => mem = Some(value),
                    Err(e) => {
                        log_error(&format!("SNMP memory collection failed for {}: {}", proxy.host, e));
                        // 실패해도 계속 진행 (메모리는 None으로 유지)
                    }
                }
            }
        }

        Ok(ResourceData {
            proxy_id: proxy.id,
            host: proxy.host.clone(),
            cpu,
            mem,
            collected_at: Local::now(),
        })
    }

    /// 여러 프록시의 자원 사용률을 병렬로 수집합니다.
    /// 실패한 프록시는 건너뛰고 성공한 것만 수집합니다.
    pub async fn collect_multiple(&self, proxies: &[Proxy]) -> Result<Vec<ResourceData>> {
        let mut tasks = Vec::new();
        
        for proxy in proxies {
            let oids = self.oids.clone();
            let community = self.community.clone();
            let proxy_clone = proxy.clone();
            
            tasks.push(tokio::spawn(async move {
                let collector = ResourceCollector::new(oids, community);
                collector.collect_for_proxy(&proxy_clone).await
            }));
        }

        // 모든 작업을 병렬로 실행하고 타임아웃 적용 (각 프록시당 최대 5초)
        let mut results = Vec::new();
        let mut failed_count = 0;
        
        for task in tasks {
            // 각 작업에 개별 타임아웃 적용
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                task
            ).await {
                Ok(Ok(Ok(data))) => {
                    // 성공
                    results.push(data);
                }
                Ok(Ok(Err(e))) => {
                    // 수집 실패 - 로그만 기록하고 계속 진행
                    log_error(&format!("프록시 수집 실패: {}", e));
                    failed_count += 1;
                }
                Ok(Err(e)) => {
                    // 태스크 실행 실패 - 로그만 기록하고 계속 진행
                    log_error(&format!("태스크 실행 실패: {}", e));
                    failed_count += 1;
                }
                Err(_) => {
                    // 타임아웃 - 로그만 기록하고 계속 진행
                    log_error("프록시 수집 타임아웃");
                    failed_count += 1;
                }
            }
        }

        // 하나도 성공하지 못했으면 에러 반환
        if results.is_empty() && failed_count > 0 {
            return Err(anyhow::anyhow!("모든 프록시 수집 실패 ({}개 실패)", failed_count));
        }

        // 일부라도 성공했으면 성공으로 처리
        Ok(results)
    }
}

/// 에러를 로그 파일에 기록
fn log_error(message: &str) {
    let log_dir = "logs";
    let _ = std::fs::create_dir_all(log_dir);
    
    let log_file = format!("{}/error.log", log_dir);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, message);
    }
}

