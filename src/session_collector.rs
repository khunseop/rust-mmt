use crate::app::{Proxy, SessionData};
use crate::ssh::SshClient;
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use regex::Regex;
use std::time::Duration;

/// 세션 브라우저 설정
#[derive(Debug, Clone)]
pub struct SessionBrowserConfig {
    pub command_path: String,
    pub command_args: String,
    pub ssh_port: u16,
    pub timeout_sec: u64,
    pub max_workers: usize,
}

impl Default for SessionBrowserConfig {
    fn default() -> Self {
        Self {
            command_path: "/opt/mwg/bin/mwg-core".to_string(),
            command_args: "-S connections".to_string(),
            ssh_port: 22,
            timeout_sec: 10,
            max_workers: 4,
        }
    }
}

/// 세션 조회기
pub struct SessionCollector {
    config: SessionBrowserConfig,
}

impl SessionCollector {
    pub fn new(config: SessionBrowserConfig) -> Self {
        Self { config }
    }

    /// 프록시에서 세션 목록을 조회합니다.
    pub async fn query_sessions(&self, proxy: &Proxy) -> Result<Vec<SessionData>> {
        let ssh_client = SshClient::new(
            proxy.host.clone(),
            self.config.ssh_port,
            proxy.username.clone(),
            proxy.password.clone(),
        )
        .with_timeout(Duration::from_secs(self.config.timeout_sec));

        // MWG 명령어 실행: command_path + command_args
        let command = format!("{} {}", self.config.command_path, self.config.command_args).trim().to_string();
        
        let output = ssh_client.execute(&command).await
            .context(format!("SSH 명령어 실행 실패: {}", command))?;

        // 출력 파싱
        Self::parse_sessions(&output, proxy)
    }

    /// 세션 출력을 파싱하여 SessionData 벡터로 변환합니다.
    /// 기존 Python 앱의 _parse_sessions 함수를 Rust로 포팅
    fn parse_sessions(output: &str, proxy: &Proxy) -> Result<Vec<SessionData>> {
        let lines: Vec<&str> = output
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            return Ok(Vec::new());
        }

        // 첫 줄이 요약 줄인지 확인 ("There are currently"로 시작)
        let mut start_idx = 0;
        if lines[0].to_lowercase().starts_with("there are currently") {
            start_idx = 1;
        }

        // 두 번째 줄이 헤더 줄인지 확인 (Transaction과 URL 포함)
        if start_idx < lines.len()
            && lines[start_idx].contains("Transaction")
            && lines[start_idx].contains("URL")
        {
            start_idx += 1;
        }

        let mut sessions = Vec::new();
        let dt_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}$").unwrap();

        for line in lines.iter().skip(start_idx) {
            // 파이프로 구분된 필드 파싱
            let parts: Vec<&str> = line.split('|').map(|p| p.trim()).collect();

            if parts.is_empty() {
                continue;
            }

            // Transaction은 항상 첫 번째 필드
            let transaction = parts.get(0).and_then(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });

            // Creation Time 찾기 (다음 몇 개 필드 중에서 날짜 형식 찾기)
            let mut creation_time_idx = None;
            for i in 1..parts.len().min(6) {
                if let Some(part) = parts.get(i) {
                    if dt_regex.is_match(part) {
                        creation_time_idx = Some(i);
                        break;
                    }
                }
            }

            // Creation Time 파싱
            let creation_time = creation_time_idx.and_then(|idx| {
                parts.get(idx).and_then(|ct_str| {
                    DateTime::parse_from_str(ct_str, "%Y-%m-%d %H:%M:%S")
                        .ok()
                        .map(|dt| dt.with_timezone(&Local))
                })
            });

            // Creation Time 이후 필드들의 인덱스 조정
            let shift_after = creation_time_idx.map(|idx| idx - 1).unwrap_or(0);

            // 필드 추출 헬퍼 함수
            let get_after = |expected_index: usize| -> Option<String> {
                let idx = expected_index + shift_after;
                parts.get(idx).and_then(|s| {
                    let s = s.trim();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
            };

            // 정수 파싱 헬퍼 함수
            let to_int = |value: Option<String>| -> Option<i64> {
                value?.parse().ok()
            };

            // 필드 추출 (기존 Python 코드의 순서대로)
            let protocol = get_after(2);
            let _cust_id = get_after(3);
            let _user_name = get_after(4);
            let client_ip_raw = get_after(5);
            
            // Client IP에서 포트 제거 (예: "1.2.3.4:56789" -> "1.2.3.4")
            let client_ip = client_ip_raw.and_then(|ip| {
                let ip_regex = Regex::new(r"^(\d+\.\d+\.\d+\.\d+):\d+$").unwrap();
                if let Some(caps) = ip_regex.captures(&ip) {
                    caps.get(1).map(|m| m.as_str().to_string())
                } else {
                    Some(ip)
                }
            });

            let _client_side_mwg_ip = get_after(6);
            let _server_side_mwg_ip = get_after(7);
            let server_ip = get_after(8);
            let _cl_bytes_received = to_int(get_after(9));
            let _cl_bytes_sent = to_int(get_after(10));
            let _srv_bytes_received = to_int(get_after(11));
            let _srv_bytes_sent = to_int(get_after(12));
            let _trxn_index = to_int(get_after(13));
            let _age_seconds = to_int(get_after(14));
            let _status = get_after(15);
            let _in_use = to_int(get_after(16));
            let mut url = get_after(17);

            // URL이 없으면 마지막 필드에서 찾기
            if url.is_none() && !parts.is_empty() {
                if let Some(last) = parts.last() {
                    let last = last.trim();
                    if last.starts_with("http://") || last.starts_with("https://") {
                        url = Some(last.to_string());
                    }
                }
            }

            // Client IP가 있어야 세션으로 인정
            if let Some(client_ip) = client_ip {
                sessions.push(SessionData {
                    proxy_id: proxy.id,
                    host: proxy.host.clone(),
                    client_ip,
                    server_ip,
                    url,
                    protocol,
                    transaction,
                    creation_time,
                });
            }
        }

        Ok(sessions)
    }

    /// 여러 프록시에서 세션을 병렬로 조회합니다.
    pub async fn query_multiple(&self, proxies: &[Proxy]) -> Result<Vec<SessionData>> {
        let mut tasks = Vec::new();

        for proxy in proxies {
            let proxy_clone = proxy.clone();
            let collector = self.clone();
            tasks.push(tokio::spawn(async move {
                collector.query_sessions(&proxy_clone).await
            }));
        }

        let mut all_sessions = Vec::new();

        for task in tasks {
            match task.await {
                Ok(Ok(sessions)) => {
                    all_sessions.extend(sessions);
                }
                Ok(Err(e)) => {
                    // 개별 프록시 조회 실패는 무시하고 계속 진행
                    eprintln!("세션 조회 실패: {}", e);
                }
                Err(e) => {
                    eprintln!("태스크 실행 실패: {}", e);
                }
            }
        }

        Ok(all_sessions)
    }
}

impl Clone for SessionCollector {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
        }
    }
}
