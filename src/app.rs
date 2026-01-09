use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 탭 인덱스
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabIndex {
    ProxyManagement = 0,
    ResourceUsage = 1,
    SessionBrowser = 2,
    TrafficLogs = 3,
}

impl TabIndex {
    pub fn next(&self) -> Self {
        match self {
            TabIndex::ProxyManagement => TabIndex::ResourceUsage,
            TabIndex::ResourceUsage => TabIndex::SessionBrowser,
            TabIndex::SessionBrowser => TabIndex::TrafficLogs,
            TabIndex::TrafficLogs => TabIndex::ProxyManagement,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            TabIndex::ProxyManagement => TabIndex::TrafficLogs,
            TabIndex::ResourceUsage => TabIndex::ProxyManagement,
            TabIndex::SessionBrowser => TabIndex::ResourceUsage,
            TabIndex::TrafficLogs => TabIndex::SessionBrowser,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => TabIndex::ProxyManagement,
            1 => TabIndex::ResourceUsage,
            2 => TabIndex::SessionBrowser,
            3 => TabIndex::TrafficLogs,
            _ => TabIndex::ProxyManagement,
        }
    }
}

/// 프록시 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proxy {
    pub id: u32,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub group: String,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub traffic_log_path: Option<String>,
}

/// 인터페이스 트래픽 정보 (bps 단위)
#[derive(Debug, Clone)]
pub struct InterfaceTraffic {
    pub name: String,
    pub in_mbps: f64,  // 실제로는 bps 값이지만 필드명은 유지
    pub out_mbps: f64, // 실제로는 bps 값이지만 필드명은 유지
}

/// 자원 사용률 데이터
#[derive(Debug, Clone)]
pub struct ResourceData {
    pub proxy_id: u32,
    pub host: String,
    pub proxy_name: Option<String>, // 프록시 이름 (설정에서 가져올 수 있으면)
    pub cpu: Option<f64>,
    pub mem: Option<f64>,
    pub cc: Option<f64>,
    pub cs: Option<f64>,
    pub http: Option<f64>,
    pub https: Option<f64>,
    pub ftp: Option<f64>,
    pub interfaces: Vec<InterfaceTraffic>, // 회선 정보
    pub collected_at: chrono::DateTime<chrono::Local>,
    pub collection_failed: bool, // 수집 실패 여부
    pub error_message: Option<String>, // 실패 시 에러 메시지
}

// Clone 구현을 위해 Proxy도 Clone 가능해야 함

/// 세션 데이터
#[derive(Debug, Clone)]
pub struct SessionData {
    pub proxy_id: u32,
    pub host: String,
    pub client_ip: String,
    pub server_ip: Option<String>,
    pub url: Option<String>,
    pub protocol: Option<String>,
    pub transaction: Option<String>, // 트랜잭션 ID
    pub creation_time: Option<chrono::DateTime<chrono::Local>>, // 생성 시간
}

/// 수집 상태
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CollectionStatus {
    #[default]
    Idle,           // 대기 중
    Starting,       // 시작 중
    Collecting,    // 수집 중
    Success,       // 성공
    Failed,        // 실패
}

/// 자원 사용률 탭 상태
#[derive(Default)]
pub struct ResourceUsageState {
    pub table_state: TableState,
    pub data: Vec<ResourceData>,
    pub selected_proxy: Option<usize>,
    pub selected_group: Option<String>, // None = 전체보기
    pub available_groups: Vec<String>,
    pub collection_interval_sec: u64, // 수집 주기 (초)
    pub last_collection_time: Option<chrono::DateTime<chrono::Local>>,
    pub last_error: Option<String>, // 마지막 에러 메시지
    pub collection_status: CollectionStatus, // 수집 상태
    pub collection_progress: Option<(usize, usize)>, // (완료된 수, 전체 수)
    pub spinner_frame: usize, // 스피너 애니메이션 프레임
    pub selected_control: Option<usize>, // 선택된 컨트롤 (None: 테이블, 0-4: 컨트롤 그리드)
    // 컨트롤 그리드: 0:그룹선택, 1:자동수집, 2:수집주기, 3:상태, 4:마지막수집
    pub auto_collection_enabled: bool, // 자동 수집 활성화 여부
    pub next_auto_collection_time: Option<chrono::DateTime<chrono::Local>>, // 다음 자동 수집 예정 시간
    pub collection_start_time: Option<chrono::DateTime<chrono::Local>>, // 수집 시작 시간
}

impl ResourceUsageState {
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
            data: Vec::new(),
            selected_proxy: None,
            selected_group: None, // None = 전체보기
            available_groups: Vec::new(),
            collection_interval_sec: 60, // 기본 60초
            last_collection_time: None,
            last_error: None,
            collection_status: CollectionStatus::Idle,
            collection_progress: None,
            spinner_frame: 0,
            selected_control: None, // 기본값: 테이블 모드
            auto_collection_enabled: false,
            next_auto_collection_time: None,
            collection_start_time: None,
        }
    }

    /// 컨트롤 그리드에서 오른쪽으로 이동 (0->1->2, 3->4->5)
    pub fn move_control_right(&mut self) {
        self.selected_control = match self.selected_control {
            None => Some(0),
            Some(0) => Some(1),
            Some(1) => Some(2),
            Some(2) => Some(2), // 오른쪽 끝
            Some(3) => Some(4),
            Some(4) => Some(5),
            Some(5) => Some(5), // 오른쪽 끝
            _ => Some(0),
        };
    }

    /// 컨트롤 그리드에서 왼쪽으로 이동 (2->1->0, 5->4->3)
    pub fn move_control_left(&mut self) {
        self.selected_control = match self.selected_control {
            None => Some(0),
            Some(0) => Some(0), // 왼쪽 끝
            Some(1) => Some(0),
            Some(2) => Some(1),
            Some(3) => Some(3), // 왼쪽 끝
            Some(4) => Some(3),
            Some(5) => Some(4),
            _ => Some(0),
        };
    }

    /// 컨트롤 그리드에서 아래로 이동 (0->3, 1->4, 2->5)
    pub fn move_control_down(&mut self) {
        self.selected_control = match self.selected_control {
            None => Some(0), // 테이블에서 첫 번째 컨트롤로
            Some(0) => Some(3),
            Some(1) => Some(4),
            Some(2) => Some(5),
            Some(3) => None, // 아래 끝에서 테이블로
            Some(4) => None, // 아래 끝에서 테이블로
            Some(5) => None, // 아래 끝에서 테이블로
            _ => Some(0),
        };
    }

    /// 컨트롤 그리드에서 위로 이동 (3->0, 4->1, 5->2)
    pub fn move_control_up(&mut self) {
        self.selected_control = match self.selected_control {
            None => Some(3), // 테이블에서 아래 줄로
            Some(0) => None, // 위 끝에서 테이블로
            Some(1) => None, // 위 끝에서 테이블로
            Some(2) => None, // 위 끝에서 테이블로
            Some(3) => Some(0),
            Some(4) => Some(1),
            Some(5) => Some(2),
            _ => Some(0),
        };
    }

    pub fn toggle_auto_collection(&mut self) {
        self.auto_collection_enabled = !self.auto_collection_enabled;
        if self.auto_collection_enabled {
            // 자동 수집 시작 시 다음 수집 시간 설정
            self.next_auto_collection_time = Some(chrono::Local::now() + chrono::Duration::seconds(self.collection_interval_sec as i64));
        } else {
            // 자동 수집 중지
            self.next_auto_collection_time = None;
        }
    }

    pub fn activate_control(&mut self) {
        match self.selected_control {
            Some(1) => {
                // 자동수집 - 토글
                self.toggle_auto_collection();
            }
            // 다른 컨트롤들은 정보 표시용이거나 다른 키로 처리됨
            _ => {}
        }
    }
    
    /// 자동 수집이 활성화되어 있고 다음 수집 시간이 되었는지 확인
    pub fn should_trigger_auto_collection(&self) -> bool {
        if !self.auto_collection_enabled {
            return false;
        }
        
        if let Some(next_time) = self.next_auto_collection_time {
            chrono::Local::now() >= next_time
                && self.collection_status == CollectionStatus::Idle
        } else {
            false
        }
    }
    
    /// 다음 자동 수집 시간 업데이트
    pub fn update_next_auto_collection_time(&mut self) {
        if self.auto_collection_enabled {
            self.next_auto_collection_time = Some(chrono::Local::now() + chrono::Duration::seconds(self.collection_interval_sec as i64));
        }
    }

    /// 수집 주기 증가 (10초 단위)
    pub fn increase_interval(&mut self) {
        const INTERVALS: &[u64] = &[10, 30, 60, 120, 300, 600];
        self.collection_interval_sec = INTERVALS
            .iter()
            .find(|&&interval| interval > self.collection_interval_sec)
            .copied()
            .unwrap_or(600);
        self.update_interval_if_auto_enabled();
    }

    /// 수집 주기 감소 (10초 단위)
    pub fn decrease_interval(&mut self) {
        const INTERVALS: &[u64] = &[10, 30, 60, 120, 300, 600];
        self.collection_interval_sec = INTERVALS
            .iter()
            .rev()
            .find(|&&interval| interval < self.collection_interval_sec)
            .copied()
            .unwrap_or(10);
        self.update_interval_if_auto_enabled();
    }

    /// 자동 수집이 활성화되어 있으면 다음 수집 시간 업데이트
    fn update_interval_if_auto_enabled(&mut self) {
        if self.auto_collection_enabled {
            self.update_next_auto_collection_time();
        }
    }

    pub fn get_interval_display(&self) -> String {
        if self.collection_interval_sec < 60 {
            format!("{}초", self.collection_interval_sec)
        } else {
            format!("{}분", self.collection_interval_sec / 60)
        }
    }

    pub fn update_groups(&mut self, proxies: &[Proxy]) {
        use std::collections::HashSet;
        let mut groups: HashSet<String> = HashSet::new();
        for proxy in proxies {
            groups.insert(proxy.group.clone());
        }
        self.available_groups = groups.into_iter().collect();
        self.available_groups.sort();
    }

    pub fn next_group(&mut self) {
        if self.available_groups.is_empty() {
            return;
        }
        match &self.selected_group {
            None => {
                // 전체보기 -> 첫 번째 그룹
                self.selected_group = Some(self.available_groups[0].clone());
            }
            Some(current) => {
                if let Some(index) = self.available_groups.iter().position(|g| g == current) {
                    if index + 1 < self.available_groups.len() {
                        self.selected_group = Some(self.available_groups[index + 1].clone());
                    } else {
                        // 마지막 그룹 -> 전체보기
                        self.selected_group = None;
                    }
                }
            }
        }
    }

    pub fn previous_group(&mut self) {
        if self.available_groups.is_empty() {
            return;
        }
        match &self.selected_group {
            None => {
                // 전체보기 -> 마지막 그룹
                self.selected_group = Some(self.available_groups.last().unwrap().clone());
            }
            Some(current) => {
                if let Some(index) = self.available_groups.iter().position(|g| g == current) {
                    if index > 0 {
                        self.selected_group = Some(self.available_groups[index - 1].clone());
                    } else {
                        // 첫 번째 그룹 -> 전체보기
                        self.selected_group = None;
                    }
                }
            }
        }
    }

    pub fn get_group_display_name(&self) -> String {
        match &self.selected_group {
            None => "전체".to_string(),
            Some(group) => group.clone(),
        }
    }

    pub fn next(&mut self) {
        let next_idx = match self.table_state.selected() {
            Some(i) => {
                if i >= self.data.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(next_idx));
    }

    pub fn previous(&mut self) {
        let prev_idx = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.data.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(prev_idx));
    }
}

/// 세션 브라우저 탭 상태
#[derive(Default)]
pub struct SessionBrowserState {
    pub table_state: TableState,
    pub sessions: Vec<SessionData>,
    pub filter: String,
    pub selected_group: Option<String>, // None = 전체보기
    pub available_groups: Vec<String>,
    pub query_status: CollectionStatus, // 조회 상태
    pub query_progress: Option<(usize, usize)>, // (완료된 수, 전체 수)
    pub last_query_time: Option<chrono::DateTime<chrono::Local>>,
    pub last_error: Option<String>, // 마지막 에러 메시지
    pub query_start_time: Option<chrono::DateTime<chrono::Local>>,
    pub spinner_frame: usize, // 스피너 애니메이션 프레임
    pub column_offset: usize, // 가로 스크롤 오프셋 (표시할 첫 번째 컬럼 인덱스)
}

impl SessionBrowserState {
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
            sessions: Vec::new(),
            filter: String::new(),
            selected_group: None, // None = 전체보기
            available_groups: Vec::new(),
            query_status: CollectionStatus::Idle,
            query_progress: None,
            last_query_time: None,
            last_error: None,
            query_start_time: None,
            spinner_frame: 0,
            column_offset: 0,
        }
    }

    /// 컬럼 오프셋 증가 (오른쪽으로 스크롤)
    pub fn scroll_right(&mut self) {
        // 최대 컬럼 수는 동적으로 계산되므로 여기서는 제한 없이 증가
        self.column_offset = self.column_offset.saturating_add(1);
    }

    /// 컬럼 오프셋 감소 (왼쪽으로 스크롤)
    pub fn scroll_left(&mut self) {
        self.column_offset = self.column_offset.saturating_sub(1);
    }

    pub fn update_groups(&mut self, proxies: &[Proxy]) {
        use std::collections::HashSet;
        let mut groups: HashSet<String> = HashSet::new();
        for proxy in proxies {
            groups.insert(proxy.group.clone());
        }
        self.available_groups = groups.into_iter().collect();
        self.available_groups.sort();
    }

    pub fn next_group(&mut self) {
        if self.available_groups.is_empty() {
            return;
        }
        match &self.selected_group {
            None => {
                // 전체보기 -> 첫 번째 그룹
                self.selected_group = Some(self.available_groups[0].clone());
            }
            Some(current) => {
                if let Some(index) = self.available_groups.iter().position(|g| g == current) {
                    if index + 1 < self.available_groups.len() {
                        self.selected_group = Some(self.available_groups[index + 1].clone());
                    } else {
                        // 마지막 그룹 -> 전체보기
                        self.selected_group = None;
                    }
                }
            }
        }
    }

    pub fn previous_group(&mut self) {
        if self.available_groups.is_empty() {
            return;
        }
        match &self.selected_group {
            None => {
                // 전체보기 -> 마지막 그룹
                self.selected_group = Some(self.available_groups.last().unwrap().clone());
            }
            Some(current) => {
                if let Some(index) = self.available_groups.iter().position(|g| g == current) {
                    if index > 0 {
                        self.selected_group = Some(self.available_groups[index - 1].clone());
                    } else {
                        // 첫 번째 그룹 -> 전체보기
                        self.selected_group = None;
                    }
                }
            }
        }
    }

    pub fn get_group_display_name(&self) -> String {
        match &self.selected_group {
            None => "전체".to_string(),
            Some(group) => group.clone(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.sessions.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.sessions.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
}

/// 트래픽 로그 분석 탭 상태
#[derive(Default)]
pub struct TrafficLogsState {
    pub selected_proxy: Option<usize>,
    pub analysis_result: Option<String>,
}

impl TrafficLogsState {
    pub fn new() -> Self {
        Self {
            selected_proxy: None,
            analysis_result: None,
        }
    }
}

/// 앱 상태
pub struct App {
    pub title: String,
    pub current_tab: TabIndex,
    pub should_quit: bool,
    pub proxies: Vec<Proxy>,
    pub resource_usage: ResourceUsageState,
    pub session_browser: SessionBrowserState,
    pub traffic_logs: TrafficLogsState,
    pub is_collecting: bool, // 수집 중 플래그
}

/// 실행 파일의 디렉터리를 기준으로 config 파일 경로를 반환합니다.
/// 실행 파일과 같은 디렉터리 또는 현재 작업 디렉터리에서 찾습니다.
fn get_config_path(filename: &str) -> PathBuf {
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

impl App {
    pub fn new(title: String) -> Self {
        Self {
            title,
            current_tab: TabIndex::ProxyManagement,
            should_quit: false,
            proxies: Vec::new(),
            resource_usage: ResourceUsageState::new(),
            session_browser: SessionBrowserState::new(),
            traffic_logs: TrafficLogsState::new(),
            is_collecting: false,
        }
    }

    pub fn load_proxies(&mut self) -> anyhow::Result<()> {
        let config_path = get_config_path("proxies.json");
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("설정 파일을 찾을 수 없습니다: {} (에러: {})", config_path.display(), e))?;
        let config: ProxyConfig = serde_json::from_str(&content)?;
        self.proxies = config.proxies;
        // 그룹 목록 업데이트
        self.resource_usage.update_groups(&self.proxies);
        self.session_browser.update_groups(&self.proxies);
        Ok(())
    }

    pub fn on_tick(&mut self) {
        // 주기적 작업이 필요하면 여기에 추가
        // 스피너 애니메이션과 자동 수집은 백그라운드 태스크에서 처리됨
    }


    pub fn on_up(&mut self) {
        match self.current_tab {
            TabIndex::ProxyManagement => {}
            TabIndex::ResourceUsage => {
                // 컨트롤 선택 기능 제거, 항상 테이블 모드
                self.resource_usage.selected_control = None;
                self.resource_usage.previous();
            }
            TabIndex::SessionBrowser => self.session_browser.previous(),
            TabIndex::TrafficLogs => {}
        }
    }

    pub fn on_down(&mut self) {
        match self.current_tab {
            TabIndex::ProxyManagement => {}
            TabIndex::ResourceUsage => {
                // 컨트롤 선택 기능 제거, 항상 테이블 모드
                self.resource_usage.selected_control = None;
                self.resource_usage.next();
            }
            TabIndex::SessionBrowser => self.session_browser.next(),
            TabIndex::TrafficLogs => {}
        }
    }

    pub fn on_left(&mut self) {
        // 모든 탭에서 탭 전환
        self.current_tab = self.current_tab.previous();
    }

    pub fn on_right(&mut self) {
        // 모든 탭에서 탭 전환
        self.current_tab = self.current_tab.next();
    }

    pub fn on_group_next(&mut self) {
        match self.current_tab {
            TabIndex::ResourceUsage => self.resource_usage.next_group(),
            TabIndex::SessionBrowser => self.session_browser.next_group(),
            _ => {}
        }
    }

    pub fn on_group_previous(&mut self) {
        match self.current_tab {
            TabIndex::ResourceUsage => self.resource_usage.previous_group(),
            TabIndex::SessionBrowser => self.session_browser.previous_group(),
            _ => {}
        }
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => self.should_quit = true,
            '1' => self.current_tab = TabIndex::ProxyManagement,
            '2' => self.current_tab = TabIndex::ResourceUsage,
            '3' => self.current_tab = TabIndex::SessionBrowser,
            '4' => self.current_tab = TabIndex::TrafficLogs,
            // +/- 키는 crossterm.rs에서 직접 처리
            _ => {}
        }
    }

    /// 자원 사용률 수집 시작 (비동기)
    pub async fn start_collection(&mut self) -> anyhow::Result<()> {
        if self.is_collecting {
            return Ok(()); // 이미 수집 중이면 무시
        }

        // 설정 파일 읽기
        let config_path = get_config_path("resource_config.json");
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("설정 파일을 찾을 수 없습니다: {} (에러: {})", config_path.display(), e))?;
        let config: serde_json::Value = serde_json::from_str(&content)?;
        
        let community = config["community"]
            .as_str()
            .unwrap_or("public")
            .to_string();
        
        let oids_json = config.get("oids").and_then(|v| v.as_object());
        let mut oids = std::collections::HashMap::new();
        if let Some(oids_obj) = oids_json {
            for (key, value) in oids_obj {
                if let Some(oid_str) = value.as_str() {
                    oids.insert(key.clone(), oid_str.to_string());
                }
            }
        }

        // 인터페이스 OID 설정 읽기
        let interface_oids_json = config.get("interface_oids").and_then(|v| v.as_object());
        let mut interface_oids = std::collections::HashMap::new();
        if let Some(if_oids_obj) = interface_oids_json {
            for (if_name, if_config) in if_oids_obj {
                if let Some(if_config_obj) = if_config.as_object() {
                    let in_oid = if_config_obj.get("in_oid")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let out_oid = if_config_obj.get("out_oid")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !in_oid.is_empty() || !out_oid.is_empty() {
                        interface_oids.insert(if_name.clone(), (in_oid, out_oid));
                    }
                }
            }
        }

        // 필터링된 프록시 목록 가져오기
        let proxies_to_collect: Vec<Proxy> = match &self.resource_usage.selected_group {
            None => self.proxies.clone(), // 전체
            Some(group) => self
                .proxies
                .iter()
                .filter(|p| &p.group == group)
                .cloned()
                .collect(),
        };

        if proxies_to_collect.is_empty() {
            return Ok(()); // 수집할 프록시가 없음
        }

        self.is_collecting = true;
        self.resource_usage.last_error = None;
        self.resource_usage.collection_status = CollectionStatus::Collecting;
        self.resource_usage.collection_progress = Some((0, proxies_to_collect.len()));
        self.resource_usage.collection_start_time = Some(chrono::Local::now());
        
        // 자동 수집이 활성화되어 있으면 다음 수집 시간 업데이트
        if self.resource_usage.auto_collection_enabled {
            self.resource_usage.update_next_auto_collection_time();
        }

        // 수집 실행
        let collector = crate::collector::ResourceCollector::new(oids, community, interface_oids);
        
        match collector.collect_multiple(&proxies_to_collect).await {
            Ok(results) => {
                // 결과 저장
                let success_count = results.iter().filter(|r| !r.collection_failed).count();
                let failed_count = results.iter().filter(|r| r.collection_failed).count();
                let total_count = proxies_to_collect.len();
                
                self.resource_usage.data = results;
                let now = chrono::Local::now();
                self.resource_usage.last_collection_time = Some(now);
                
                // 부분 성공도 성공으로 처리
                if success_count > 0 {
                    self.resource_usage.collection_status = CollectionStatus::Success;
                    self.resource_usage.collection_progress = Some((success_count, total_count));
                    
                    // 일부만 성공했으면 경고 메시지
                    if failed_count > 0 {
                        self.resource_usage.last_error = Some(format!(
                            "일부 프록시 수집 실패 ({}개 성공, {}개 실패)",
                            success_count,
                            failed_count
                        ));
                    }
                } else {
                    // 하나도 성공하지 못함
                    self.resource_usage.collection_status = CollectionStatus::Failed;
                    self.resource_usage.last_error = Some("모든 프록시 수집 실패".to_string());
                    self.resource_usage.collection_progress = None;
                }
                
                // 수집 완료 후 상태 초기화
                self.is_collecting = false;
                self.resource_usage.collection_start_time = None;

                // CSV 저장 (실패한 것도 포함)
                if !self.resource_usage.data.is_empty() {
                    if let Err(e) = crate::csv_writer::CsvWriter::save_resource_usage(&self.resource_usage.data) {
                        let existing_error = self.resource_usage.last_error.clone();
                        self.resource_usage.last_error = Some(format!(
                            "{}{}",
                            existing_error.map(|e| format!("{} / ", e)).unwrap_or_default(),
                            format!("CSV 저장 실패: {}", e)
                        ));
                    }
                }
            }
            Err(e) => {
                // 수집 실패 - 에러 메시지 저장
                self.resource_usage.last_error = Some(format!("수집 실패: {}", e));
                self.resource_usage.collection_status = CollectionStatus::Failed;
                self.resource_usage.data = Vec::new();
                self.resource_usage.collection_progress = None;
                self.is_collecting = false;
                self.resource_usage.collection_start_time = None;
            }
        }

        self.is_collecting = false;
        Ok(())
    }

    /// 세션 조회 시작 (비동기)
    pub async fn start_session_query(&mut self) -> anyhow::Result<()> {
        // 이미 조회 중이면 무시
        if self.session_browser.query_status == CollectionStatus::Collecting {
            return Ok(());
        }

        // 필터링된 프록시 목록 가져오기
        let proxies_to_query: Vec<Proxy> = match &self.session_browser.selected_group {
            None => self.proxies.clone(), // 전체
            Some(group) => self
                .proxies
                .iter()
                .filter(|p| &p.group == group)
                .cloned()
                .collect(),
        };

        if proxies_to_query.is_empty() {
            return Ok(()); // 조회할 프록시가 없음
        }

        self.session_browser.last_error = None;
        self.session_browser.query_status = CollectionStatus::Collecting;
        self.session_browser.query_progress = Some((0, proxies_to_query.len()));
        self.session_browser.query_start_time = Some(chrono::Local::now());

        // 세션 브라우저 설정 로드 (기본값 사용)
        let config = crate::session_collector::SessionBrowserConfig::default();
        let collector = crate::session_collector::SessionCollector::new(config);

        // 세션 조회 실행
        match collector.query_multiple(&proxies_to_query).await {
            Ok(sessions) => {
                let success_count = proxies_to_query.len();
                let total_count = proxies_to_query.len();
                
                self.session_browser.sessions = sessions;
                let now = chrono::Local::now();
                self.session_browser.last_query_time = Some(now);
                
                self.session_browser.query_status = CollectionStatus::Success;
                self.session_browser.query_progress = Some((success_count, total_count));
                
                // CSV 저장
                if !self.session_browser.sessions.is_empty() {
                    if let Err(e) = crate::csv_writer::CsvWriter::save_sessions(&self.session_browser.sessions) {
                        let existing_error = self.session_browser.last_error.clone();
                        self.session_browser.last_error = Some(format!(
                            "{}{}",
                            existing_error.map(|e| format!("{} / ", e)).unwrap_or_default(),
                            format!("CSV 저장 실패: {}", e)
                        ));
                    }
                }
            }
            Err(e) => {
                // 조회 실패 - 에러 메시지 저장
                self.session_browser.last_error = Some(format!("세션 조회 실패: {}", e));
                self.session_browser.query_status = CollectionStatus::Failed;
                self.session_browser.sessions = Vec::new();
                self.session_browser.query_progress = None;
            }
        }

        self.session_browser.query_start_time = None;
        Ok(())
    }
}

/// 프록시 설정 파일 구조
#[derive(Debug, Serialize, Deserialize)]
struct ProxyConfig {
    proxies: Vec<Proxy>,
}

