use std::collections::HashMap;
use std::path::PathBuf;

/// 실행 파일의 디렉터리를 기준으로 config 파일 경로를 반환합니다.
pub fn get_config_path(filename: &str) -> PathBuf {
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
pub fn get_interface_names() -> Vec<String> {
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

/// 임계치 설정 구조체
pub struct ThresholdConfig {
    pub warning: f64,
    pub critical: f64,
}

/// 설정 파일에서 임계치를 읽어옵니다.
pub fn load_thresholds() -> HashMap<String, ThresholdConfig> {
    let config_path = get_config_path("resource_config.json");
    let mut thresholds = HashMap::new();
    
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(thresholds_obj) = config.get("thresholds").and_then(|v| v.as_object()) {
                for (key, value) in thresholds_obj {
                    if let Some(threshold_obj) = value.as_object() {
                        let warning = threshold_obj.get("warning")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let critical = threshold_obj.get("critical")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        thresholds.insert(key.clone(), ThresholdConfig { warning, critical });
                    }
                }
            }
        }
    }
    
    // 기본값 설정 (설정 파일에 없을 경우)
    if !thresholds.contains_key("cpu") {
        thresholds.insert("cpu".to_string(), ThresholdConfig { warning: 70.0, critical: 90.0 });
    }
    if !thresholds.contains_key("mem") {
        thresholds.insert("mem".to_string(), ThresholdConfig { warning: 70.0, critical: 90.0 });
    }
    if !thresholds.contains_key("cc") {
        thresholds.insert("cc".to_string(), ThresholdConfig { warning: 10000.0, critical: 50000.0 });
    }
    if !thresholds.contains_key("cs") {
        thresholds.insert("cs".to_string(), ThresholdConfig { warning: 10000.0, critical: 50000.0 });
    }
    if !thresholds.contains_key("http") {
        thresholds.insert("http".to_string(), ThresholdConfig { warning: 1000000000.0, critical: 5000000000.0 });
    }
    if !thresholds.contains_key("https") {
        thresholds.insert("https".to_string(), ThresholdConfig { warning: 1000000000.0, critical: 5000000000.0 });
    }
    if !thresholds.contains_key("ftp") {
        thresholds.insert("ftp".to_string(), ThresholdConfig { warning: 1000000000.0, critical: 5000000000.0 });
    }
    if !thresholds.contains_key("interface_traffic") {
        thresholds.insert("interface_traffic".to_string(), ThresholdConfig { warning: 1000000000.0, critical: 5000000000.0 });
    }
    
    thresholds
}
