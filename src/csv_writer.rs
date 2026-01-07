use crate::app::ResourceData;
use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::PathBuf;

/// CSV 파일 작성기
pub struct CsvWriter;

impl CsvWriter {
    /// 자원 사용률 데이터를 CSV 파일로 저장합니다.
    pub fn save_resource_usage(data: &[ResourceData]) -> Result<PathBuf> {
        // logs 디렉토리 생성
        let logs_dir = PathBuf::from("logs");
        fs::create_dir_all(&logs_dir).context("Failed to create logs directory")?;

        // 파일명 생성 (타임스탬프 포함)
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("resource_usage_{}.csv", timestamp);
        let filepath = logs_dir.join(filename);

        // CSV 작성
        let mut wtr = csv::Writer::from_path(&filepath)
            .context("Failed to create CSV file")?;

        // 헤더 작성
        wtr.write_record(&["timestamp", "proxy_id", "host", "cpu", "mem"])
            .context("Failed to write CSV header")?;

        // 데이터 작성
        for record in data {
            let cpu_str = record
                .cpu
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "".to_string());
            let mem_str = record
                .mem
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "".to_string());

            wtr.write_record(&[
                record.collected_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                record.proxy_id.to_string(),
                record.host.clone(),
                cpu_str,
                mem_str,
            ])
            .context("Failed to write CSV record")?;
        }

        wtr.flush().context("Failed to flush CSV file")?;

        Ok(filepath)
    }
}

