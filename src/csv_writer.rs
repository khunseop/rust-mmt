use crate::app::ResourceData;
use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 설정 파일에서 회선 목록을 읽어옵니다.
fn get_interface_names() -> Vec<String> {
    let config_path = "config/resource_config.json";
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
}

