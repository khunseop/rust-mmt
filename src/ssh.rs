use anyhow::{Context, Result};
use ssh2::Session;
use std::io::prelude::*;
use std::net::TcpStream;
use std::time::Duration;
use tokio::time::timeout;

/// SSH 클라이언트 (ssh2 사용)
pub struct SshClient {
    host: String,
    port: u16,
    username: String,
    password: String,
    timeout: Duration,
}

impl SshClient {
    pub fn new(host: String, port: u16, username: String, password: String) -> Self {
        Self {
            host,
            port,
            username,
            password,
            timeout: Duration::from_secs(15),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// SSH를 통해 명령을 실행하고 결과를 반환합니다.
    pub async fn execute(&self, command: &str) -> Result<String> {
        let host = self.host.clone();
        let port = self.port;
        let username = self.username.clone();
        let password = self.password.clone();
        let command = command.to_string();
        let timeout_duration = self.timeout;

        // 블로킹 작업을 스레드 풀에서 실행
        let result = tokio::task::spawn_blocking(move || {
            // TCP 연결
            let addr = format!("{}:{}", host, port);
            let tcp = TcpStream::connect(&addr)
                .context("Failed to connect to SSH server")?;
            
            // SSH 세션 생성
            let mut sess = Session::new()
                .context("Failed to create SSH session")?;
            
            sess.set_tcp_stream(tcp);
            sess.handshake()
                .context("SSH handshake failed")?;
            
            // 인증
            sess.userauth_password(&username, &password)
                .context("SSH authentication failed")?;
            
            if !sess.authenticated() {
                anyhow::bail!("SSH authentication failed: invalid credentials");
            }
            
            // 명령 실행
            let mut channel = sess.channel_session()
                .context("Failed to open SSH channel")?;
            
            channel.exec(&command)
                .context("Failed to execute SSH command")?;
            
            // 출력 읽기
            let mut output = String::new();
            channel.read_to_string(&mut output)
                .context("Failed to read SSH command output")?;
            
            // 채널 종료 대기
            channel.wait_close()
                .context("Failed to close SSH channel")?;
            
            let exit_status = channel.exit_status()
                .context("Failed to get exit status")?;
            
            // stderr 읽기 (있는 경우)
            let mut stderr = String::new();
            channel.stderr().read_to_string(&mut stderr).ok();
            
            // 종료 상태가 0이 아니고 stdout이 비어있으면 에러
            if exit_status != 0 && output.is_empty() {
                let error_msg = if !stderr.is_empty() {
                    stderr
                } else {
                    format!("Command exited with status {}", exit_status)
                };
                anyhow::bail!("SSH command failed: {}", error_msg);
            }
            
            Ok(output)
        })
        .await
        .context("SSH task failed")?;

        // 타임아웃 적용 (블로킹 작업이므로 실제로는 연결 단계에서만 적용됨)
        timeout(timeout_duration, async { result })
            .await
            .context("SSH command execution timeout")?
    }

    /// 메모리 사용률을 가져옵니다 (SSH를 통해)
    pub async fn get_memory_percent(&self) -> Result<f64> {
        let command = "awk '/MemTotal/ {total=$2} /MemAvailable/ {available=$2} END {printf \"%.0f\", 100 - (available / total * 100)}' /proc/meminfo";
        let output = self.execute(command).await?;
        let value: f64 = output
            .trim()
            .parse()
            .context("Failed to parse memory percentage")?;
        
        // 값 범위 제한 (0-100)
        Ok(value.min(100.0).max(0.0))
    }
}

