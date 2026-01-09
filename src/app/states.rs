use crate::app::types::{CollectionStatus, Proxy};
use ratatui::widgets::TableState;

/// 자원 사용률 탭 상태
#[derive(Default)]
pub struct ResourceUsageState {
    pub table_state: TableState,
    pub data: Vec<crate::app::types::ResourceData>,
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
    pub sessions: Vec<crate::app::types::SessionData>,
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
        // 최대 컬럼 수는 19개, 한 번에 10개씩 표시하므로 최대 9까지
        let max_offset = 19usize.saturating_sub(10);
        self.column_offset = (self.column_offset + 1).min(max_offset);
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
