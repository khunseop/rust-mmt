use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};

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
    pub traffic_log_path: Option<String>,
    #[serde(default = "default_snmp_community")]
    pub snmp_community: String,
}

fn default_snmp_community() -> String {
    "public".to_string()
}

/// 자원 사용률 데이터
#[derive(Debug, Clone)]
pub struct ResourceData {
    pub proxy_id: u32,
    pub host: String,
    pub cpu: Option<f64>,
    pub mem: Option<f64>,
    pub collected_at: chrono::DateTime<chrono::Local>,
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
        }
    }

    pub fn increase_interval(&mut self) {
        // 10초, 30초, 60초, 120초, 300초 순서로 증가
        self.collection_interval_sec = match self.collection_interval_sec {
            0..=9 => 10,
            10..=29 => 30,
            30..=59 => 60,
            60..=119 => 120,
            120..=299 => 300,
            _ => 600, // 600초 이상이면 600초로
        };
    }

    pub fn decrease_interval(&mut self) {
        // 10초, 30초, 60초, 120초, 300초 순서로 감소
        self.collection_interval_sec = match self.collection_interval_sec {
            0..=10 => 10,
            11..=30 => 10,
            31..=60 => 30,
            61..=120 => 60,
            121..=300 => 120,
            _ => 300,
        };
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
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.data.len().saturating_sub(1) {
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
                    self.data.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
}

/// 세션 브라우저 탭 상태
#[derive(Default)]
pub struct SessionBrowserState {
    pub table_state: TableState,
    pub sessions: Vec<SessionData>,
    pub filter: String,
}

impl SessionBrowserState {
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
            sessions: Vec::new(),
            filter: String::new(),
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
        let config_path = "config/proxies.json";
        let content = std::fs::read_to_string(config_path)?;
        let config: ProxyConfig = serde_json::from_str(&content)?;
        self.proxies = config.proxies;
        // 그룹 목록 업데이트
        self.resource_usage.update_groups(&self.proxies);
        Ok(())
    }

    pub fn on_tick(&mut self) {
        // 스피너 애니메이션 업데이트
        if self.resource_usage.collection_status == CollectionStatus::Collecting
            || self.resource_usage.collection_status == CollectionStatus::Starting {
            self.resource_usage.spinner_frame = (self.resource_usage.spinner_frame + 1) % 4;
        }
    }

    pub fn on_left(&mut self) {
        self.current_tab = self.current_tab.previous();
    }

    pub fn on_right(&mut self) {
        self.current_tab = self.current_tab.next();
    }

    pub fn on_up(&mut self) {
        match self.current_tab {
            TabIndex::ProxyManagement => {}
            TabIndex::ResourceUsage => self.resource_usage.previous(),
            TabIndex::SessionBrowser => self.session_browser.previous(),
            TabIndex::TrafficLogs => {}
        }
    }

    pub fn on_down(&mut self) {
        match self.current_tab {
            TabIndex::ProxyManagement => {}
            TabIndex::ResourceUsage => self.resource_usage.next(),
            TabIndex::SessionBrowser => self.session_browser.next(),
            TabIndex::TrafficLogs => {}
        }
    }

    pub fn on_group_next(&mut self) {
        match self.current_tab {
            TabIndex::ResourceUsage => self.resource_usage.next_group(),
            _ => {}
        }
    }

    pub fn on_group_previous(&mut self) {
        match self.current_tab {
            TabIndex::ResourceUsage => self.resource_usage.previous_group(),
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
            '+' | '=' => {
                if self.current_tab == TabIndex::ResourceUsage {
                    self.resource_usage.increase_interval();
                }
            }
            '-' | '_' => {
                if self.current_tab == TabIndex::ResourceUsage {
                    self.resource_usage.decrease_interval();
                }
            }
            'c' | 'C' => {
                if self.current_tab == TabIndex::ResourceUsage && !self.is_collecting {
                    // 수집 시작은 이벤트 루프에서 처리해야 하므로 플래그만 설정
                    // 실제 수집은 crossterm.rs의 이벤트 루프에서 처리
                }
            }
            _ => {}
        }
    }

    /// 자원 사용률 수집 시작 (비동기)
    pub async fn start_collection(&mut self) -> anyhow::Result<()> {
        if self.is_collecting {
            return Ok(()); // 이미 수집 중이면 무시
        }

        // 설정 파일 읽기
        let config_path = "config/resource_config.json";
        let content = std::fs::read_to_string(config_path)?;
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
        self.resource_usage.collection_status = CollectionStatus::Starting;
        self.resource_usage.collection_progress = Some((0, proxies_to_collect.len()));

        // 수집 실행
        let collector = crate::collector::ResourceCollector::new(oids, community);
        self.resource_usage.collection_status = CollectionStatus::Collecting;
        
        match collector.collect_multiple(&proxies_to_collect).await {
            Ok(results) => {
                // 결과 저장
                let results_count = results.len();
                self.resource_usage.data = results;
                self.resource_usage.last_collection_time = Some(chrono::Local::now());
                self.resource_usage.collection_status = CollectionStatus::Success;
                self.resource_usage.collection_progress = Some((results_count, proxies_to_collect.len()));

                // CSV 저장
                if let Err(e) = crate::csv_writer::CsvWriter::save_resource_usage(&self.resource_usage.data) {
                    self.resource_usage.last_error = Some(format!("CSV 저장 실패: {}", e));
                }
            }
            Err(e) => {
                // 수집 실패 - 에러 메시지 저장
                self.resource_usage.last_error = Some(format!("수집 실패: {}", e));
                self.resource_usage.collection_status = CollectionStatus::Failed;
                self.resource_usage.data = Vec::new();
                self.resource_usage.collection_progress = None;
            }
        }

        self.is_collecting = false;
        Ok(())
    }
}

/// 프록시 설정 파일 구조
#[derive(Debug, Serialize, Deserialize)]
struct ProxyConfig {
    proxies: Vec<Proxy>,
}

