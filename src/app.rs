use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};

/// 탭 인덱스
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabIndex {
    ResourceUsage = 0,
    SessionBrowser = 1,
    TrafficLogs = 2,
}

impl TabIndex {
    pub fn next(&self) -> Self {
        match self {
            TabIndex::ResourceUsage => TabIndex::SessionBrowser,
            TabIndex::SessionBrowser => TabIndex::TrafficLogs,
            TabIndex::TrafficLogs => TabIndex::ResourceUsage,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            TabIndex::ResourceUsage => TabIndex::TrafficLogs,
            TabIndex::SessionBrowser => TabIndex::ResourceUsage,
            TabIndex::TrafficLogs => TabIndex::SessionBrowser,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index % 3 {
            0 => TabIndex::ResourceUsage,
            1 => TabIndex::SessionBrowser,
            2 => TabIndex::TrafficLogs,
            _ => TabIndex::ResourceUsage,
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

/// 자원 사용률 탭 상태
#[derive(Default)]
pub struct ResourceUsageState {
    pub table_state: TableState,
    pub data: Vec<ResourceData>,
    pub selected_proxy: Option<usize>,
}

impl ResourceUsageState {
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
            data: Vec::new(),
            selected_proxy: None,
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
}

impl App {
    pub fn new(title: String) -> Self {
        Self {
            title,
            current_tab: TabIndex::ResourceUsage,
            should_quit: false,
            proxies: Vec::new(),
            resource_usage: ResourceUsageState::new(),
            session_browser: SessionBrowserState::new(),
            traffic_logs: TrafficLogsState::new(),
        }
    }

    pub fn load_proxies(&mut self) -> anyhow::Result<()> {
        let config_path = "config/proxies.json";
        let content = std::fs::read_to_string(config_path)?;
        let config: ProxyConfig = serde_json::from_str(&content)?;
        self.proxies = config.proxies;
        Ok(())
    }

    pub fn on_tick(&mut self) {
        // 주기적으로 실행할 작업 (필요시)
    }

    pub fn on_left(&mut self) {
        self.current_tab = self.current_tab.previous();
    }

    pub fn on_right(&mut self) {
        self.current_tab = self.current_tab.next();
    }

    pub fn on_up(&mut self) {
        match self.current_tab {
            TabIndex::ResourceUsage => self.resource_usage.previous(),
            TabIndex::SessionBrowser => self.session_browser.previous(),
            TabIndex::TrafficLogs => {}
        }
    }

    pub fn on_down(&mut self) {
        match self.current_tab {
            TabIndex::ResourceUsage => self.resource_usage.next(),
            TabIndex::SessionBrowser => self.session_browser.next(),
            TabIndex::TrafficLogs => {}
        }
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => self.should_quit = true,
            '1' => self.current_tab = TabIndex::ResourceUsage,
            '2' => self.current_tab = TabIndex::SessionBrowser,
            '3' => self.current_tab = TabIndex::TrafficLogs,
            _ => {}
        }
    }
}

/// 프록시 설정 파일 구조
#[derive(Debug, Serialize, Deserialize)]
struct ProxyConfig {
    proxies: Vec<Proxy>,
}

