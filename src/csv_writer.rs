use crate::app::ResourceData;
use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::PathBuf;

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
            wtr.write_record(&[
                "timestamp", "proxy_id", "host", "cpu", "mem", "cc", "cs", "http", "https", "ftp", "interfaces", "status"
            ])
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
            
            // 회선 정보
            let interface_str = if record.interfaces.is_empty() {
                "".to_string()
            } else {
                record.interfaces.iter()
                    .map(|iface| format!("{}: {:.2}/{:.2}", iface.name, iface.in_mbps, iface.out_mbps))
                    .collect::<Vec<_>>()
                    .join("; ")
            };

            let status = if record.collection_failed {
                record.error_message.as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("실패")
            } else {
                "성공"
            };

            wtr.write_record(&[
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
                interface_str,
                status.to_string(),
            ])
            .context("Failed to write CSV record")?;
        }

        wtr.flush().context("Failed to flush CSV file")?;

        Ok(filepath)
    }
}

