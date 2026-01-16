use crate::app::ResourceData;
use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 실행 파일의 디렉터리를 기준으로 config 파일 경로를 반환합니다.
fn get_config_path(filename: &str) -> std::path::PathBuf {
    // 먼저 실행 파일의 디렉터리에서 찾기 시도
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let config_path = exe_dir.join("config").join(filename);
            if config_path.exists() {
                return config_path;
            }
            // 실행 파일과 같은 디렉터리에 직접 있는 경우도 확인
            let direct_path = exe_dir.join(filename);
            if direct_path.exists() {
                return direct_path;
            }
        }
    }
    
    // 실행 파일 위치에서 찾지 못하면 현재 작업 디렉터리에서 찾기
    let current_dir_path = std::path::Path::new("config").join(filename);
    if current_dir_path.exists() {
        return current_dir_path;
    }
    
    // 둘 다 없으면 기본값으로 현재 작업 디렉터리 반환 (에러는 나중에 발생)
    std::path::Path::new("config").join(filename)
}

/// 설정 파일에서 회선 목록을 읽어옵니다.
fn get_interface_names() -> Vec<String> {
    let config_path = get_config_path("resource_config.json");
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(interface_oids) = config.get("interface_oids").and_then(|v| v.as_object()) {
                let mut names: Vec<String> = interface_oids.keys().cloned().collect();
                names.sort(); // 정렬하여 일관된 순서 유지
                return names;
            }
        }
    }
    Vec::new()
}

/// CSV 파일 작성기
pub struct CsvWriter;

impl CsvWriter {
    /// 자원 사용률 데이터를 CSV 파일로 저장합니다.
    /// 파일이 없으면 생성하고, 있으면 append 모드로 추가합니다.
    pub fn save_resource_usage(data: &[ResourceData]) -> Result<PathBuf> {
        // logs 디렉토리 생성
        let logs_dir = PathBuf::from("logs");
        fs::create_dir_all(&logs_dir).context("Failed to create logs directory")?;

        // 파일명 생성 (날짜별로 하나의 파일)
        let date_str = Local::now().format("%Y%m%d");
        let filename = format!("resource_usage_{}.csv", date_str);
        let filepath = logs_dir.join(&filename);

        // 파일이 존재하는지 확인
        let file_exists = filepath.exists();
        
        // 회선 목록 가져오기
        let interface_names = get_interface_names();
        
        // CSV 작성 (append 모드)
        let mut wtr = if file_exists {
            // 파일이 있으면 append 모드로 열기
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&filepath)
                .context("Failed to open CSV file for appending")?;
            csv::Writer::from_writer(file)
        } else {
            // 파일이 없으면 새로 생성
            csv::Writer::from_path(&filepath)
                .context("Failed to create CSV file")?
        };

        // 헤더는 파일이 없을 때만 작성
        if !file_exists {
            let mut header = vec![
                "timestamp", "proxy_id", "host", "cpu", "mem", "cc", "cs", "http", "https", "ftp"
            ];
            
            // 각 회선에 대해 컬럼 추가
            for if_name in &interface_names {
                header.push(if_name);
            }
            
            header.push("status");
            
            wtr.write_record(&header)
                .context("Failed to write CSV header")?;
        }

        // 데이터 작성
        for record in data {
            let format_value = |v: Option<f64>| -> String {
                v.map(|val| format!("{:.2}", val)).unwrap_or_else(|| "".to_string())
            };

            let cpu_str = format_value(record.cpu);
            let mem_str = format_value(record.mem);
            let cc_str = format_value(record.cc);
            let cs_str = format_value(record.cs);
            let http_str = format_value(record.http);
            let https_str = format_value(record.https);
            let ftp_str = format_value(record.ftp);
            
            // 회선 정보를 HashMap으로 변환 (빠른 조회를 위해)
            let interface_map: HashMap<String, (f64, f64)> = record.interfaces
                .iter()
                .map(|iface| (iface.name.clone(), (iface.in_mbps, iface.out_mbps)))
                .collect();

            let status = if record.collection_failed {
                record.error_message.as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("실패")
            } else {
                "성공"
            };

            // 기본 필드들
            let mut record_fields = vec![
                record.collected_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                record.proxy_id.to_string(),
                record.host.clone(),
                cpu_str,
                mem_str,
                cc_str,
                cs_str,
                http_str,
                https_str,
                ftp_str,
            ];
            
            // 각 회선에 대해 값 추가 (In/Out 형식)
            for if_name in &interface_names {
                if let Some((in_mbps, out_mbps)) = interface_map.get(if_name) {
                    record_fields.push(format!("{:.2}/{:.2}", in_mbps, out_mbps));
                } else {
                    record_fields.push("".to_string());
                }
            }
            
            record_fields.push(status.to_string());

            wtr.write_record(&record_fields)
            .context("Failed to write CSV record")?;
        }

        wtr.flush().context("Failed to flush CSV file")?;

        Ok(filepath)
    }

    /// 세션 데이터를 CSV 파일로 저장합니다.
    pub fn save_sessions(sessions: &[crate::app::SessionData]) -> Result<PathBuf> {
        // logs 디렉토리 생성
        let logs_dir = PathBuf::from("logs");
        fs::create_dir_all(&logs_dir).context("Failed to create logs directory")?;

        // 파일명 생성 (타임스탬프 포함)
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("sessions_{}.csv", timestamp);
        let filepath = logs_dir.join(&filename);

        // CSV 작성
        let mut wtr = csv::Writer::from_path(&filepath)
            .context("Failed to create CSV file")?;

        // 헤더 작성 (모든 필드 포함)
        wtr.write_record(&[
            "timestamp",
            "proxy_id",
            "host",
            "transaction",
            "creation_time",
            "protocol",
            "cust_id",
            "user_name",
            "client_ip",
            "client_side_mwg_ip",
            "server_side_mwg_ip",
            "server_ip",
            "cl_bytes_received",
            "cl_bytes_sent",
            "srv_bytes_received",
            "srv_bytes_sent",
            "trxn_index",
            "age_seconds",
            "status",
            "in_use",
            "url",
        ])
        .context("Failed to write CSV header")?;

        // 데이터 작성
        let timestamp_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        for session in sessions {
            let creation_time_str = session.creation_time
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default();
            
            wtr.write_record(&[
                timestamp_str.clone(),
                session.proxy_id.to_string(),
                session.host.clone(),
                session.transaction.as_ref().unwrap_or(&String::new()).clone(),
                creation_time_str,
                session.protocol.as_ref().unwrap_or(&String::new()).clone(),
                session.cust_id.as_ref().unwrap_or(&String::new()).clone(),
                session.user_name.as_ref().unwrap_or(&String::new()).clone(),
                session.client_ip.clone(),
                session.client_side_mwg_ip.as_ref().unwrap_or(&String::new()).clone(),
                session.server_side_mwg_ip.as_ref().unwrap_or(&String::new()).clone(),
                session.server_ip.as_ref().unwrap_or(&String::new()).clone(),
                session.cl_bytes_received.map(|v| v.to_string()).unwrap_or_default(),
                session.cl_bytes_sent.map(|v| v.to_string()).unwrap_or_default(),
                session.srv_bytes_received.map(|v| v.to_string()).unwrap_or_default(),
                session.srv_bytes_sent.map(|v| v.to_string()).unwrap_or_default(),
                session.trxn_index.map(|v| v.to_string()).unwrap_or_default(),
                session.age_seconds.map(|v| v.to_string()).unwrap_or_default(),
                session.status.as_ref().unwrap_or(&String::new()).clone(),
                session.in_use.map(|v| v.to_string()).unwrap_or_default(),
                session.url.as_ref().unwrap_or(&String::new()).clone(),
            ])
            .context("Failed to write CSV record")?;
        }

        wtr.flush().context("Failed to flush CSV file")?;

        Ok(filepath)
    }

    /// 트래픽 로그 분석 결과를 CSV 파일로 저장합니다.
    pub fn save_traffic_analysis(analysis: &crate::traffic_log_parser::TopNAnalysis) -> Result<PathBuf> {
        // logs 디렉토리 생성
        let logs_dir = PathBuf::from("logs");
        fs::create_dir_all(&logs_dir).context("Failed to create logs directory")?;

        // 파일명 생성 (타임스탬프 포함)
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("traffic_analysis_{}.csv", timestamp);
        let filepath = logs_dir.join(&filename);

        // CSV 작성
        let mut wtr = csv::Writer::from_path(&filepath)
            .context("Failed to create CSV file")?;

        // 요약 정보 저장
        wtr.write_record(&[
            "type", "category", "rank", "value", "request_count", "recv_bytes", "sent_bytes"
        ])
        .context("Failed to write CSV header")?;

        // TOP 클라이언트
        for (i, client) in analysis.top_clients.iter().enumerate() {
            wtr.write_record(&[
                "top_clients".to_string(),
                "client_ip".to_string(),
                (i + 1).to_string(),
                client.client_ip.clone(),
                client.request_count.to_string(),
                client.recv_bytes.to_string(),
                client.sent_bytes.to_string(),
            ])
            .context("Failed to write CSV record")?;
        }

        // TOP 호스트
        for (i, host) in analysis.top_hosts.iter().enumerate() {
            wtr.write_record(&[
                "top_hosts".to_string(),
                "host".to_string(),
                (i + 1).to_string(),
                host.host.clone(),
                host.request_count.to_string(),
                host.recv_bytes.to_string(),
                host.sent_bytes.to_string(),
            ])
            .context("Failed to write CSV record")?;
        }

        // TOP URL
        for (i, url) in analysis.top_urls.iter().enumerate() {
            wtr.write_record(&[
                "top_urls".to_string(),
                "url".to_string(),
                (i + 1).to_string(),
                url.url.clone(),
                url.request_count.to_string(),
                String::new(), // recv_bytes 없음
                String::new(), // sent_bytes 없음
            ])
            .context("Failed to write CSV record")?;
        }

        // 요약 통계
        wtr.write_record(&[
            "summary".to_string(),
            "total_records".to_string(),
            String::new(),
            analysis.total_records.to_string(),
            String::new(),
            String::new(),
            String::new(),
        ])
        .context("Failed to write CSV record")?;

        wtr.write_record(&[
            "summary".to_string(),
            "parsed_records".to_string(),
            String::new(),
            analysis.parsed_records.to_string(),
            String::new(),
            String::new(),
            String::new(),
        ])
        .context("Failed to write CSV record")?;

        wtr.write_record(&[
            "summary".to_string(),
            "total_recv_bytes".to_string(),
            String::new(),
            analysis.total_recv_bytes.to_string(),
            String::new(),
            String::new(),
            String::new(),
        ])
        .context("Failed to write CSV record")?;

        wtr.write_record(&[
            "summary".to_string(),
            "total_sent_bytes".to_string(),
            String::new(),
            analysis.total_sent_bytes.to_string(),
            String::new(),
            String::new(),
            String::new(),
        ])
        .context("Failed to write CSV record")?;

        wtr.write_record(&[
            "summary".to_string(),
            "blocked_count".to_string(),
            String::new(),
            analysis.blocked_count.to_string(),
            String::new(),
            String::new(),
            String::new(),
        ])
        .context("Failed to write CSV record")?;

        wtr.flush().context("Failed to flush CSV file")?;

        Ok(filepath)
    }
}

