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

/// 세션 데이터
#[derive(Debug, Clone)]
pub struct SessionData {
    pub proxy_id: u32,
    pub host: String,
    pub transaction: Option<String>, // 트랜잭션 ID
    pub creation_time: Option<chrono::DateTime<chrono::Local>>, // 생성 시간
    pub protocol: Option<String>,
    pub cust_id: Option<String>,
    pub user_name: Option<String>,
    pub client_ip: String,
    pub client_side_mwg_ip: Option<String>,
    pub server_side_mwg_ip: Option<String>,
    pub server_ip: Option<String>,
    pub cl_bytes_received: Option<i64>, // 클라이언트 수신 바이트
    pub cl_bytes_sent: Option<i64>, // 클라이언트 송신 바이트
    pub srv_bytes_received: Option<i64>, // 서버 수신 바이트
    pub srv_bytes_sent: Option<i64>, // 서버 송신 바이트
    pub trxn_index: Option<i64>, // 트랜잭션 인덱스
    pub age_seconds: Option<i64>, // 세션 나이 (초)
    pub status: Option<String>, // 상태
    pub in_use: Option<i64>, // 사용 중 여부
    pub url: Option<String>,
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

/// 프록시 설정 파일 구조
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub proxies: Vec<Proxy>,
}
