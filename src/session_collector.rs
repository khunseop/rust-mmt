use crate::app::{Proxy, SessionData};
use crate::ssh::SshClient;
use anyhow::{Context, Result};
use regex::Regex;

/// 세션 조회기
pub struct SessionCollector;

impl SessionCollector {
    /// 프록시에서 세션 목록을 조회합니다.
    pub async fn query_sessions(proxy: &Proxy) -> Result<Vec<SessionData>> {
        let ssh_client = SshClient::new(
            proxy.host.clone(),
            proxy.port,
            proxy.username.clone(),
            proxy.password.clone(),
        );

        // MWG 프록시 서버에서 세션 조회 명령어 실행
        // 일반적인 MWG 명령어: show sessions 또는 유사한 명령어
        // 여러 명령어를 시도하여 세션 정보를 가져옵니다
        let commands = vec![
            "show sessions",
            "sessions",
            "show active-sessions",
            "cat /proc/net/tcp | grep ESTABLISHED",
        ];

        let mut last_error = None;
        for command in commands {
            match ssh_client.execute(command).await {
                Ok(output) => {
                    if !output.trim().is_empty() {
                        return Self::parse_sessions(&output, proxy);
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // 모든 명령어가 실패한 경우
        if let Some(e) = last_error {
            Err(e).context("세션 조회 실패")
        } else {
            // 출력이 비어있는 경우 (세션이 없는 경우)
            Ok(Vec::new())
        }
    }

    /// 세션 출력을 파싱하여 SessionData 벡터로 변환합니다.
    fn parse_sessions(output: &str, proxy: &Proxy) -> Result<Vec<SessionData>> {
        let mut sessions = Vec::new();

        // 여러 형식의 세션 출력을 파싱 시도
        // 형식 1: MWG 표준 형식 (예: "T12345 1.2.3.4:12345 10.0.0.1:80 http://example.com HTTP")
        // 형식 2: netstat 형식
        // 형식 3: 커스텀 형식

        // 정규식 패턴들
        let patterns = vec![
            // MWG 형식: 트랜잭션ID 클라이언트IP:포트 서버IP:포트 URL 프로토콜
            (
                r"(?i)(?:T|Transaction|Txn)\s*(\d+)\s+(\d+\.\d+\.\d+\.\d+)(?::(\d+))?\s+(\d+\.\d+\.\d+\.\d+)(?::(\d+))?\s+(https?://[^\s]+|ftp://[^\s]+|[^\s]+)\s+(HTTP|HTTPS|FTP|http|https|ftp)",
                "mwg",
            ),
            // 간단한 형식: 클라이언트IP URL 프로토콜
            (
                r"(\d+\.\d+\.\d+\.\d+)\s+(https?://[^\s]+|ftp://[^\s]+|[^\s]+)\s+(HTTP|HTTPS|FTP|http|https|ftp)",
                "simple",
            ),
            // netstat 형식
            (
                r"(\d+\.\d+\.\d+\.\d+):(\d+)\s+(\d+\.\d+\.\d+\.\d+):(\d+)\s+ESTABLISHED",
                "netstat",
            ),
        ];

        for (pattern, format_type) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                for line in output.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    if let Some(captures) = re.captures(line) {
                        match format_type {
                            "mwg" => {
                                // MWG 형식 파싱
                                let client_ip = captures.get(2).map(|m| m.as_str().to_string());
                                let server_ip = captures.get(4).map(|m| m.as_str().to_string());
                                let url = captures.get(6).map(|m| m.as_str().to_string());
                                let protocol = captures.get(7).map(|m| m.as_str().to_uppercase());

                                if let Some(client_ip) = client_ip {
                                    sessions.push(SessionData {
                                        proxy_id: proxy.id,
                                        host: proxy.host.clone(),
                                        client_ip,
                                        server_ip,
                                        url,
                                        protocol,
                                    });
                                }
                            }
                            "simple" => {
                                // 간단한 형식 파싱
                                let client_ip = captures.get(1).map(|m| m.as_str().to_string());
                                let url = captures.get(2).map(|m| m.as_str().to_string());
                                let protocol = captures.get(3).map(|m| m.as_str().to_uppercase());

                                if let Some(client_ip) = client_ip {
                                    sessions.push(SessionData {
                                        proxy_id: proxy.id,
                                        host: proxy.host.clone(),
                                        client_ip,
                                        server_ip: None,
                                        url,
                                        protocol,
                                    });
                                }
                            }
                            "netstat" => {
                                // netstat 형식 파싱 (URL 정보 없음)
                                let client_ip = captures.get(1).map(|m| m.as_str().to_string());
                                let server_ip = captures.get(3).map(|m| m.as_str().to_string());

                                if let Some(client_ip) = client_ip {
                                    sessions.push(SessionData {
                                        proxy_id: proxy.id,
                                        host: proxy.host.clone(),
                                        client_ip,
                                        server_ip,
                                        url: None,
                                        protocol: None,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // 패턴이 매칭되면 파싱 완료
                if !sessions.is_empty() {
                    return Ok(sessions);
                }
            }
        }

        // 파싱 실패 시, 기본 파싱 시도 (공백으로 구분된 필드)
        Self::parse_sessions_fallback(output, proxy)
    }

    /// 기본 파싱 (공백으로 구분된 필드)
    fn parse_sessions_fallback(output: &str, proxy: &Proxy) -> Result<Vec<SessionData>> {
        let mut sessions = Vec::new();
        let ip_pattern = Regex::new(r"\d+\.\d+\.\d+\.\d+").unwrap();
        let url_pattern = Regex::new(r"https?://[^\s]+|ftp://[^\s]+").unwrap();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // IP 주소 찾기
            let client_ips: Vec<&str> = ip_pattern.find_iter(line).map(|m| m.as_str()).collect();
            if client_ips.is_empty() {
                continue;
            }

            let client_ip = client_ips[0].to_string();
            let server_ip = if client_ips.len() > 1 {
                Some(client_ips[1].to_string())
            } else {
                None
            };

            // URL 찾기
            let url = url_pattern.find(line).map(|m| m.as_str().to_string());

            // 프로토콜 찾기
            let protocol = if line.to_uppercase().contains("HTTPS") {
                Some("HTTPS".to_string())
            } else if line.to_uppercase().contains("HTTP") {
                Some("HTTP".to_string())
            } else if line.to_uppercase().contains("FTP") {
                Some("FTP".to_string())
            } else {
                None
            };

            sessions.push(SessionData {
                proxy_id: proxy.id,
                host: proxy.host.clone(),
                client_ip,
                server_ip,
                url,
                protocol,
            });
        }

        Ok(sessions)
    }

    /// 여러 프록시에서 세션을 병렬로 조회합니다.
    pub async fn query_multiple(proxies: &[Proxy]) -> Result<Vec<SessionData>> {
        let mut tasks = Vec::new();

        for proxy in proxies {
            let proxy_clone = proxy.clone();
            tasks.push(tokio::spawn(async move {
                Self::query_sessions(&proxy_clone).await
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
