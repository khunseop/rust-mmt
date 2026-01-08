use crate::app::{Proxy, ResourceData, InterfaceTraffic};
use crate::snmp::snmp_get_async;
use crate::ssh::SshClient;
use anyhow::Result;
use chrono::Local;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// 인터페이스 카운터 캐시: (proxy_id, interface_name) -> (in_counter, out_counter, timestamp)
fn get_interface_cache() -> &'static Mutex<HashMap<(u32, String), (u64, u64, f64)>> {
    static CACHE: OnceLock<Mutex<HashMap<(u32, String), (u64, u64, f64)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 자원 수집기
pub struct ResourceCollector {
    oids: HashMap<String, String>, // key -> OID 매핑
    community: String,
    interface_oids: HashMap<String, (String, String)>, // interface_name -> (in_oid, out_oid)
}

impl ResourceCollector {
    pub fn new(
        oids: HashMap<String, String>,
        community: String,
        interface_oids: HashMap<String, (String, String)>,
    ) -> Self {
        Self {
            oids,
            community,
            interface_oids,
        }
    }

    /// 프록시의 자원 사용률을 수집합니다.
    pub async fn collect_for_proxy(&self, proxy: &Proxy) -> Result<ResourceData> {
        let mut cpu: Option<f64> = None;
        let mut mem: Option<f64> = None;
        let mut cc: Option<f64> = None;
        let mut cs: Option<f64> = None;
        let mut http: Option<f64> = None;
        let mut https: Option<f64> = None;
        let mut ftp: Option<f64> = None;
        let mut interfaces: Vec<InterfaceTraffic> = Vec::new();
        let collection_failed = false;
        let error_message: Option<String> = None;

        // 모든 지표를 병렬로 수집
        let mut tasks: Vec<(String, tokio::task::JoinHandle<Result<f64>>)> = Vec::new();

        // CPU 수집
        if let Some(cpu_oid) = self.oids.get("cpu") {
            if !cpu_oid.trim().is_empty() && !cpu_oid.eq_ignore_ascii_case("ssh") {
                let host = proxy.host.clone();
                let community = self.community.clone();
                let oid = cpu_oid.clone();
                tasks.push(("cpu".to_string(), tokio::spawn(async move {
                    snmp_get_async(&host, &community, &oid).await
                })));
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
                tasks.push(("mem".to_string(), tokio::spawn(async move {
                    ssh_client.get_memory_percent().await
                })));
            } else if !mem_oid.trim().is_empty() {
                let host = proxy.host.clone();
                let community = self.community.clone();
                let oid = mem_oid.clone();
                tasks.push(("mem".to_string(), tokio::spawn(async move {
                    snmp_get_async(&host, &community, &oid).await
                })));
            }
        }

        // CC 수집
        if let Some(cc_oid) = self.oids.get("cc") {
            if !cc_oid.trim().is_empty() {
                let host = proxy.host.clone();
                let community = self.community.clone();
                let oid = cc_oid.clone();
                tasks.push(("cc".to_string(), tokio::spawn(async move {
                    snmp_get_async(&host, &community, &oid).await
                })));
            }
        }

        // CS 수집
        if let Some(cs_oid) = self.oids.get("cs") {
            if !cs_oid.trim().is_empty() {
                let host = proxy.host.clone();
                let community = self.community.clone();
                let oid = cs_oid.clone();
                tasks.push(("cs".to_string(), tokio::spawn(async move {
                    snmp_get_async(&host, &community, &oid).await
                })));
            }
        }

        // HTTP 수집
        if let Some(http_oid) = self.oids.get("http") {
            if !http_oid.trim().is_empty() {
                let host = proxy.host.clone();
                let community = self.community.clone();
                let oid = http_oid.clone();
                tasks.push(("http".to_string(), tokio::spawn(async move {
                    snmp_get_async(&host, &community, &oid).await
                })));
            }
        }

        // HTTPS 수집
        if let Some(https_oid) = self.oids.get("https") {
            if !https_oid.trim().is_empty() {
                let host = proxy.host.clone();
                let community = self.community.clone();
                let oid = https_oid.clone();
                tasks.push(("https".to_string(), tokio::spawn(async move {
                    snmp_get_async(&host, &community, &oid).await
                })));
            }
        }

        // FTP 수집
        if let Some(ftp_oid) = self.oids.get("ftp") {
            if !ftp_oid.trim().is_empty() {
                let host = proxy.host.clone();
                let community = self.community.clone();
                let oid = ftp_oid.clone();
                tasks.push(("ftp".to_string(), tokio::spawn(async move {
                    snmp_get_async(&host, &community, &oid).await
                })));
            }
        }

        // 모든 작업 실행 (각 작업에 개별 타임아웃 적용)
        for (key, handle) in tasks {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                handle
            ).await;

            match result {
                Ok(Ok(Ok(value))) => {
                    // 성공
                    match key.as_str() {
                        "cpu" => cpu = Some(value),
                        "mem" => mem = Some(value),
                        "cc" => cc = Some(value),
                        "cs" => cs = Some(value),
                        "http" => http = Some(value),
                        "https" => https = Some(value),
                        "ftp" => ftp = Some(value),
                        _ => {}
                    }
                }
                Ok(Ok(Err(e))) => {
                    log_error(&format!("{} 수집 실패 for {}: {}", key, proxy.host, e));
                }
                Ok(Err(e)) => {
                    log_error(&format!("{} 태스크 실패 for {}: {}", key, proxy.host, e));
                }
                Err(_) => {
                    log_error(&format!("{} 수집 타임아웃 for {}", key, proxy.host));
                }
            }
        }

        // 인터페이스 트래픽 수집
        if !self.interface_oids.is_empty() {
            let mut interface_tasks = Vec::new();
            for (if_name, (in_oid, out_oid)) in &self.interface_oids {
                if !in_oid.is_empty() {
                    let host = proxy.host.clone();
                    let community = self.community.clone();
                    let oid = in_oid.clone();
                    let if_name_clone = if_name.clone();
                    interface_tasks.push((
                        if_name_clone.clone(),
                        "in".to_string(),
                        tokio::spawn(async move {
                            snmp_get_async(&host, &community, &oid).await
                        }),
                    ));
                }
                if !out_oid.is_empty() {
                    let host = proxy.host.clone();
                    let community = self.community.clone();
                    let oid = out_oid.clone();
                    let if_name_clone = if_name.clone();
                    interface_tasks.push((
                        if_name_clone.clone(),
                        "out".to_string(),
                        tokio::spawn(async move {
                            snmp_get_async(&host, &community, &oid).await
                        }),
                    ));
                }
            }

            // 인터페이스 카운터 수집
            let mut interface_counters: HashMap<String, (Option<u64>, Option<u64>)> = HashMap::new();
            for (if_name, direction, handle) in interface_tasks {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    handle,
                )
                .await;

                if let Ok(Ok(Ok(value))) = result {
                    let counter = value as u64;
                    let entry = interface_counters.entry(if_name).or_insert((None, None));
                    match direction.as_str() {
                        "in" => entry.0 = Some(counter),
                        "out" => entry.1 = Some(counter),
                        _ => {}
                    }
                }
            }

            // Mbps 계산 (이전 값과 비교)
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            for (if_name, (in_counter, out_counter)) in interface_counters {
                let cache_key = (proxy.id, if_name.clone());
                let mut cache = get_interface_cache().lock().unwrap();

                if let Some((prev_in, prev_out, prev_time)) = cache.get(&cache_key) {
                    let time_diff = current_time - prev_time;
                    if time_diff >= 1.0 && time_diff <= 300.0 {
                        // 1초 이상 5분 이하 차이만 유효
                        let in_mbps = if let Some(current_in) = in_counter {
                            calculate_mbps(*prev_in, current_in, time_diff)
                        } else {
                            0.0
                        };
                        let out_mbps = if let Some(current_out) = out_counter {
                            calculate_mbps(*prev_out, current_out, time_diff)
                        } else {
                            0.0
                        };

                        if in_mbps > 0.0 || out_mbps > 0.0 {
                            interfaces.push(InterfaceTraffic {
                                name: if_name.clone(),
                                in_mbps,
                                out_mbps,
                            });
                        }
                    }
                }

                // 캐시 업데이트
                cache.insert(
                    cache_key,
                    (
                        in_counter.unwrap_or(0),
                        out_counter.unwrap_or(0),
                        current_time,
                    ),
                );
            }
        }

        Ok(ResourceData {
            proxy_id: proxy.id,
            host: proxy.host.clone(),
            proxy_name: None,
            cpu,
            mem,
            cc,
            cs,
            http,
            https,
            ftp,
            interfaces,
            collected_at: Local::now(),
            collection_failed,
            error_message,
        })
    }

    /// 여러 프록시의 자원 사용률을 병렬로 수집합니다.
    /// 실패한 프록시도 실패 데이터로 포함합니다.
    pub async fn collect_multiple(&self, proxies: &[Proxy]) -> Result<Vec<ResourceData>> {
        let mut tasks = Vec::new();
        let mut proxy_map: HashMap<u32, Proxy> = HashMap::new();
        
        for proxy in proxies {
            proxy_map.insert(proxy.id, proxy.clone());
            let oids = self.oids.clone();
            let community = self.community.clone();
            let interface_oids = self.interface_oids.clone();
            let proxy_clone = proxy.clone();
            
            tasks.push((proxy.id, tokio::spawn(async move {
                let collector = ResourceCollector::new(oids, community, interface_oids);
                collector.collect_for_proxy(&proxy_clone).await
            })));
        }

        // 모든 작업을 병렬로 실행하고 타임아웃 적용 (각 프록시당 최대 5초)
        let mut results = Vec::new();
        
        for (proxy_id, task) in tasks {
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
                    // 수집 실패 - 실패 데이터 생성
                    if let Some(proxy) = proxy_map.get(&proxy_id) {
                        let failed_data = ResourceData {
                            proxy_id: proxy.id,
                            host: proxy.host.clone(),
                            proxy_name: None,
                            cpu: None,
                            mem: None,
                            cc: None,
                            cs: None,
                            http: None,
                            https: None,
                            ftp: None,
                            interfaces: Vec::new(),
                            collected_at: Local::now(),
                            collection_failed: true,
                            error_message: Some(format!("수집 실패: {}", e)),
                        };
                        results.push(failed_data);
                    }
                    log_error(&format!("프록시 {} 수집 실패: {}", proxy_id, e));
                }
                Ok(Err(e)) => {
                    // 태스크 실행 실패 - 실패 데이터 생성
                    if let Some(proxy) = proxy_map.get(&proxy_id) {
                        let failed_data = ResourceData {
                            proxy_id: proxy.id,
                            host: proxy.host.clone(),
                            proxy_name: None,
                            cpu: None,
                            mem: None,
                            cc: None,
                            cs: None,
                            http: None,
                            https: None,
                            ftp: None,
                            interfaces: Vec::new(),
                            collected_at: Local::now(),
                            collection_failed: true,
                            error_message: Some(format!("태스크 실행 실패: {}", e)),
                        };
                        results.push(failed_data);
                    }
                    log_error(&format!("프록시 {} 태스크 실행 실패: {}", proxy_id, e));
                }
                Err(_) => {
                    // 타임아웃 - 실패 데이터 생성
                    if let Some(proxy) = proxy_map.get(&proxy_id) {
                        let failed_data = ResourceData {
                            proxy_id: proxy.id,
                            host: proxy.host.clone(),
                            proxy_name: None,
                            cpu: None,
                            mem: None,
                            cc: None,
                            cs: None,
                            http: None,
                            https: None,
                            ftp: None,
                            interfaces: Vec::new(),
                            collected_at: Local::now(),
                            collection_failed: true,
                            error_message: Some("수집 타임아웃".to_string()),
                        };
                        results.push(failed_data);
                    }
                    log_error(&format!("프록시 {} 수집 타임아웃", proxy_id));
                }
            }
        }

        // 프록시 ID 순서대로 정렬
        results.sort_by_key(|d| d.proxy_id);

        Ok(results)
    }
}

/// Mbps 계산 함수 (32비트 카운터 오버플로우 처리)
fn calculate_mbps(prev: u64, current: u64, time_diff_sec: f64) -> f64 {
    let diff = if current >= prev {
        current - prev
    } else {
        // 카운터 오버플로우 (32비트 최대값: 4294967295)
        (u64::MAX - prev) + current + 1
    };
    
    // 바이트를 Mbps로 변환: (bytes * 8) / (time_diff * 1_000_000)
    (diff as f64 * 8.0) / (time_diff_sec * 1_000_000.0)
}

// 로그 파일 쓰기를 위한 뮤텍스 (동시성 보장)
static LOG_MUTEX: Mutex<()> = Mutex::new(());

/// 에러를 로그 파일에 기록 (스레드 안전)
fn log_error(message: &str) {
    let _guard = LOG_MUTEX.lock().unwrap();
    
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
        let _ = file.flush();
    }
}

