use crate::app::Proxy;
use crate::ssh::SshClient;
use anyhow::{Context, Result};
use std::time::Duration;

/// 트래픽 로그 수집기 설정
#[derive(Debug, Clone)]
pub struct TrafficLogCollectorConfig {
    pub ssh_port: u16,
    pub timeout_sec: u64,
    pub limit: usize, // 조회할 라인 수
    pub direction: LogDirection, // head 또는 tail
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogDirection {
    Head, // 처음부터
    Tail, // 끝부터
}

impl Default for TrafficLogCollectorConfig {
    fn default() -> Self {
        Self {
            ssh_port: 22,
            timeout_sec: 30,
            limit: 500,
            direction: LogDirection::Tail,
        }
    }
}

/// 트래픽 로그 수집기
pub struct TrafficLogCollector {
    config: TrafficLogCollectorConfig,
}

impl TrafficLogCollector {
    pub fn new(config: TrafficLogCollectorConfig) -> Self {
        Self { config }
    }

    /// 프록시에서 트래픽 로그를 조회합니다.
    pub async fn fetch_logs(&self, proxy: &Proxy, log_path: &str) -> Result<Vec<String>> {
        let ssh_client = SshClient::new(
            proxy.host.clone(),
            self.config.ssh_port,
            proxy.username.clone(),
            proxy.password.clone(),
        )
        .with_timeout(Duration::from_secs(self.config.timeout_sec));

        // 로그 조회 명령어 생성
        let limit_str = self.config.limit.to_string();
        // 경로를 안전하게 이스케이프 (간단한 버전)
        let safe_path = format!("'{}'", log_path.replace('\'', r"'\''"));
        
        let command = match self.config.direction {
            LogDirection::Head => {
                format!("timeout 5s nice -n 10 ionice -c2 -n7 head -n {} {} | sed -e 's/[^[:print:]\\t]//g' | head -c 1048576 | cat", 
                    limit_str, safe_path)
            }
            LogDirection::Tail => {
                format!("timeout 5s nice -n 10 ionice -c2 -n7 tail -n {} {} | sed -e 's/[^[:print:]\\t]//g' | head -c 1048576 | cat", 
                    limit_str, safe_path)
            }
        };

        let output = ssh_client.execute(&command).await
            .context(format!("SSH 명령어 실행 실패: {}", command))?;

        // 라인으로 분리하고 빈 라인 제거
        let lines: Vec<String> = output
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(lines)
    }
}
