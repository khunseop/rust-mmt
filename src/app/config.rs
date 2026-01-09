use std::path::{Path, PathBuf};

/// 실행 파일의 디렉터리를 기준으로 config 파일 경로를 반환합니다.
/// 실행 파일과 같은 디렉터리 또는 현재 작업 디렉터리에서 찾습니다.
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
    let current_dir_path = Path::new("config").join(filename);
    if current_dir_path.exists() {
        return current_dir_path;
    }
    
    // 둘 다 없으면 기본값으로 현재 작업 디렉터리 반환 (에러는 나중에 발생)
    Path::new("config").join(filename)
}
