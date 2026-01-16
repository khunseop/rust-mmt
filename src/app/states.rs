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
    
    // 페이지네이션
    pub page_size: usize,
    pub current_page: usize,
    pub total_pages: usize,
    
    // 열 선택 및 정렬
    pub selected_column: Option<usize>, // 선택된 컬럼 인덱스 (None = 행 선택 모드)
    pub sort_column: Option<usize>, // 정렬 기준 컬럼
    pub sort_ascending: bool, // 정렬 방향
    
    // 상세보기 모달
    pub show_detail_modal: bool, // 상세보기 모달 표시 여부
    
    // 검색 기능
    pub search_mode: bool, // 검색 모드 활성화 여부
    pub search_query: String, // 검색어
    
    // 컬럼 순서 변경
    pub column_order: Vec<usize>, // 컬럼 순서 배열 (인덱스)
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
            page_size: 50, // 기본 페이지 크기
            current_page: 0,
            total_pages: 0,
            selected_column: None, // None = 행 선택 모드
            sort_column: None,
            sort_ascending: true,
            show_detail_modal: false,
            search_mode: false,
            search_query: String::new(),
            column_order: (0..19).collect(), // 기본 순서: 0~18
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

    pub fn next(&mut self, current_page_items: usize) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= current_page_items.saturating_sub(1) {
                    // 현재 페이지의 마지막 행이면 다음 페이지로 이동
                    if self.current_page < self.total_pages.saturating_sub(1) {
                        self.current_page += 1;
                        0 // 다음 페이지의 첫 번째 행
                    } else {
                        // 마지막 페이지의 마지막 행이면 그대로 유지 (wrap 하지 않음)
                        i
                    }
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self, _current_page_items: usize) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    // 현재 페이지의 첫 번째 행이면 이전 페이지로 이동
                    if self.current_page > 0 {
                        self.current_page -= 1;
                        // 이전 페이지의 마지막 행 (이전 페이지는 항상 full page)
                        self.page_size.saturating_sub(1)
                    } else {
                        // 첫 페이지의 첫 번째 행이면 그대로 유지 (wrap 하지 않음)
                        0
                    }
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    /// 페이지네이션된 세션 목록 가져오기
    pub fn get_paginated_sessions<'a>(&self, sessions: &'a [crate::app::types::SessionData]) -> Vec<&'a crate::app::types::SessionData> {
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(sessions.len());
        if start >= sessions.len() {
            return Vec::new();
        }
        sessions[start..end].iter().collect()
    }

    /// 다음 페이지로 이동
    pub fn next_page(&mut self) {
        if self.current_page < self.total_pages.saturating_sub(1) {
            self.current_page += 1;
            self.table_state.select(Some(0));
        }
    }

    /// 이전 페이지로 이동
    pub fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.table_state.select(Some(0));
        }
    }

    /// 첫 페이지로 이동
    pub fn first_page(&mut self) {
        self.current_page = 0;
        self.table_state.select(Some(0));
    }

    /// 마지막 페이지로 이동
    pub fn last_page(&mut self) {
        if self.total_pages > 0 {
            self.current_page = self.total_pages - 1;
            self.table_state.select(Some(0));
        }
    }

    /// 총 페이지 수 업데이트
    pub fn update_total_pages(&mut self, total_items: usize) {
        self.total_pages = if total_items == 0 {
            0
        } else {
            (total_items + self.page_size - 1) / self.page_size
        };
        if self.current_page >= self.total_pages && self.total_pages > 0 {
            self.current_page = self.total_pages - 1;
        }
    }

    /// 컬럼 선택 이동 (왼쪽) - 가로 스크롤도 함께 처리
    pub fn select_column_left(&mut self) {
        if let Some(col) = self.selected_column {
            if col > 0 {
                self.selected_column = Some(col - 1);
                // 선택된 컬럼이 보이도록 가로 스크롤 조정
                const MAX_VISIBLE: usize = 10;
                if col - 1 < self.column_offset {
                    // 선택된 컬럼이 현재 보이는 범위 밖이면 스크롤 조정
                    self.column_offset = col - 1;
                }
            }
        } else {
            // 컬럼이 선택되지 않았으면 첫 번째 컬럼 선택
            self.selected_column = Some(0);
        }
    }

    /// 컬럼 선택 이동 (오른쪽) - 가로 스크롤도 함께 처리
    pub fn select_column_right(&mut self) {
        if let Some(col) = self.selected_column {
            if col < 18 {
                self.selected_column = Some(col + 1);
                // 선택된 컬럼이 보이도록 가로 스크롤 조정
                const MAX_VISIBLE: usize = 10;
                if col + 1 >= self.column_offset + MAX_VISIBLE {
                    // 선택된 컬럼이 현재 보이는 범위 밖이면 스크롤 조정
                    self.column_offset = (col + 1).saturating_sub(MAX_VISIBLE - 1);
                }
            }
        } else {
            // 컬럼이 선택되지 않았으면 첫 번째 컬럼 선택
            self.selected_column = Some(0);
        }
    }

    /// 선택된 컬럼 기준 정렬 토글
    pub fn toggle_sort(&mut self) {
        if let Some(col) = self.selected_column {
            if self.sort_column == Some(col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(col);
                self.sort_ascending = true;
            }
        }
    }

    /// 정렬 해제
    pub fn clear_sort(&mut self) {
        self.sort_column = None;
        self.sort_ascending = true;
    }

    /// 상세보기 모달 토글
    pub fn toggle_detail_modal(&mut self) {
        self.show_detail_modal = !self.show_detail_modal;
    }

    /// 상세보기 모달 닫기
    pub fn close_detail_modal(&mut self) {
        self.show_detail_modal = false;
    }

    /// 컬럼 선택 해제
    pub fn clear_column_selection(&mut self) {
        self.selected_column = None;
    }

    /// 검색 모드 시작
    pub fn start_search_mode(&mut self) {
        self.search_mode = true;
    }

    /// 검색 모드 종료 (검색어 유지)
    pub fn finish_search_mode(&mut self) {
        self.search_mode = false;
        // 검색어는 유지
    }

    /// 검색 취소 (검색어 초기화)
    pub fn cancel_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
    }

    /// 검색 모드 토글 (하위 호환성)
    pub fn toggle_search_mode(&mut self) {
        if self.search_mode {
            self.cancel_search_mode();
        } else {
            self.start_search_mode();
        }
    }

    /// 검색어 추가 (검색 모드일 때)
    pub fn add_search_char(&mut self, c: char) {
        if self.search_mode {
            self.search_query.push(c);
        }
    }

    /// 검색어 백스페이스 (검색 모드일 때)
    pub fn backspace_search(&mut self) {
        if self.search_mode {
            self.search_query.pop();
        }
    }

    /// 검색어로 세션 필터링
    pub fn filter_sessions<'a>(&self, sessions: &'a [&'a crate::app::types::SessionData]) -> Vec<&'a crate::app::types::SessionData> {
        if self.search_query.is_empty() {
            return sessions.iter().copied().collect();
        }

        let query = self.search_query.to_lowercase();
        sessions.iter()
            .copied()
            .filter(|session| {
                // 모든 필드에서 검색
                session.host.to_lowercase().contains(&query) ||
                session.transaction.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.protocol.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.cust_id.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.user_name.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.client_ip.to_lowercase().contains(&query) ||
                session.client_side_mwg_ip.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.server_side_mwg_ip.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.server_ip.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.status.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.url.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                session.cl_bytes_received.map(|v| format!("{}", v).contains(&query)).unwrap_or(false) ||
                session.cl_bytes_sent.map(|v| format!("{}", v).contains(&query)).unwrap_or(false) ||
                session.srv_bytes_received.map(|v| format!("{}", v).contains(&query)).unwrap_or(false) ||
                session.srv_bytes_sent.map(|v| format!("{}", v).contains(&query)).unwrap_or(false) ||
                session.trxn_index.map(|v| format!("{}", v).contains(&query)).unwrap_or(false) ||
                session.age_seconds.map(|v| format!("{}", v).contains(&query)).unwrap_or(false) ||
                session.in_use.map(|v| format!("{}", v).contains(&query)).unwrap_or(false) ||
                session.creation_time.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string().to_lowercase().contains(&query)).unwrap_or(false)
            })
            .collect::<Vec<_>>()
    }

    /// 컬럼 순서 변경 (왼쪽으로 이동)
    pub fn move_column_left(&mut self, col_idx: usize) {
        if col_idx > 0 && col_idx < self.column_order.len() {
            self.column_order.swap(col_idx, col_idx - 1);
        }
    }

    /// 컬럼 순서 변경 (오른쪽으로 이동)
    pub fn move_column_right(&mut self, col_idx: usize) {
        if col_idx < self.column_order.len().saturating_sub(1) {
            self.column_order.swap(col_idx, col_idx + 1);
        }
    }
}

/// 트래픽 로그 분석 탭 상태
#[derive(Default)]
pub struct TrafficLogsState {
    pub selected_proxy: Option<usize>,
    pub analysis_result: Option<String>,
    pub top_n_analysis: Option<crate::traffic_log_parser::TopNAnalysis>,
    pub analysis_status: CollectionStatus,
    pub analysis_progress: Option<(usize, usize)>,
    pub last_analysis_time: Option<chrono::DateTime<chrono::Local>>,
    pub last_error: Option<String>,
    pub analysis_start_time: Option<chrono::DateTime<chrono::Local>>,
    pub spinner_frame: usize,
    pub top_n: usize, // TOP N 개수 (기본값: 20)
    pub view_mode: TrafficLogViewMode, // 표시 모드
    // 로그 조회 관련 필드
    pub log_records: Vec<crate::traffic_log_parser::TrafficLogRecord>,
    pub log_limit: usize, // 조회할 로그 수 (기본값: 500)
    pub query_status: CollectionStatus,
    pub query_progress: Option<(usize, usize)>,
    pub last_query_time: Option<chrono::DateTime<chrono::Local>>,
    pub query_start_time: Option<chrono::DateTime<chrono::Local>>,
    // 페이지네이션
    pub current_page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub table_state: ratatui::widgets::TableState,
    // 프록시 선택 (숫자 인덱스)
    pub proxy_list_index: usize,
    // 컬럼 스크롤
    pub column_offset: usize,
    // 상세보기 모달
    pub show_detail_modal: bool,
    // 검색 기능
    pub search_mode: bool,
    pub search_query: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TrafficLogViewMode {
    #[default]
    Summary,    // 요약 정보
    TopClients, // TOP 클라이언트
    TopHosts,   // TOP 호스트
    TopUrls,    // TOP URL
    LogList,    // 로그 목록
}

impl TrafficLogsState {
    pub fn new() -> Self {
        Self {
            selected_proxy: None,
            analysis_result: None,
            top_n_analysis: None,
            analysis_status: CollectionStatus::Idle,
            analysis_progress: None,
            last_analysis_time: None,
            last_error: None,
            analysis_start_time: None,
            spinner_frame: 0,
            top_n: 20,
            view_mode: TrafficLogViewMode::Summary,
            // 로그 조회 관련
            log_records: Vec::new(),
            log_limit: 500,
            query_status: CollectionStatus::Idle,
            query_progress: None,
            last_query_time: None,
            query_start_time: None,
            // 페이지네이션
            current_page: 0,
            page_size: 50,
            total_pages: 0,
            table_state: ratatui::widgets::TableState::default(),
            proxy_list_index: 0,
            // 컬럼 스크롤
            column_offset: 0,
            // 상세보기 모달
            show_detail_modal: false,
            // 검색 기능
            search_mode: false,
            search_query: String::new(),
        }
    }

    pub fn next_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            TrafficLogViewMode::Summary => TrafficLogViewMode::TopClients,
            TrafficLogViewMode::TopClients => TrafficLogViewMode::TopHosts,
            TrafficLogViewMode::TopHosts => TrafficLogViewMode::TopUrls,
            TrafficLogViewMode::TopUrls => TrafficLogViewMode::LogList,
            TrafficLogViewMode::LogList => TrafficLogViewMode::Summary,
        };
    }

    pub fn previous_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            TrafficLogViewMode::Summary => TrafficLogViewMode::LogList,
            TrafficLogViewMode::TopClients => TrafficLogViewMode::Summary,
            TrafficLogViewMode::TopHosts => TrafficLogViewMode::TopClients,
            TrafficLogViewMode::TopUrls => TrafficLogViewMode::TopHosts,
            TrafficLogViewMode::LogList => TrafficLogViewMode::TopUrls,
        };
    }

    /// 테이블에서 다음 행 선택
    pub fn next(&mut self, items_count: usize) {
        if items_count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= items_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    /// 테이블에서 이전 행 선택
    pub fn previous(&mut self, items_count: usize) {
        if items_count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    items_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    /// 다음 페이지로 이동
    pub fn next_page(&mut self) {
        if self.current_page < self.total_pages.saturating_sub(1) {
            self.current_page += 1;
            self.table_state.select(Some(0));
        }
    }

    /// 이전 페이지로 이동
    pub fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.table_state.select(Some(0));
        }
    }

    /// 첫 페이지로 이동
    pub fn first_page(&mut self) {
        self.current_page = 0;
        self.table_state.select(Some(0));
    }

    /// 마지막 페이지로 이동
    pub fn last_page(&mut self) {
        self.current_page = self.total_pages.saturating_sub(1);
        self.table_state.select(Some(0));
    }

    /// 총 페이지 수 업데이트
    pub fn update_total_pages(&mut self, total_items: usize) {
        self.total_pages = if total_items == 0 {
            0
        } else {
            (total_items + self.page_size - 1) / self.page_size
        };
        // 현재 페이지가 범위를 벗어나면 조정
        if self.current_page >= self.total_pages && self.total_pages > 0 {
            self.current_page = self.total_pages - 1;
        }
    }

    /// 다음 프록시 선택
    pub fn next_proxy(&mut self, proxy_count: usize) {
        if proxy_count == 0 {
            return;
        }
        self.proxy_list_index = (self.proxy_list_index + 1) % proxy_count;
    }

    /// 이전 프록시 선택
    pub fn previous_proxy(&mut self, proxy_count: usize) {
        if proxy_count == 0 {
            return;
        }
        if self.proxy_list_index == 0 {
            self.proxy_list_index = proxy_count - 1;
        } else {
            self.proxy_list_index -= 1;
        }
    }

    /// 컬럼 오른쪽으로 스크롤
    pub fn scroll_column_right(&mut self, max_columns: usize) {
        if self.column_offset < max_columns.saturating_sub(1) {
            self.column_offset += 1;
        }
    }

    /// 컬럼 왼쪽으로 스크롤
    pub fn scroll_column_left(&mut self) {
        if self.column_offset > 0 {
            self.column_offset -= 1;
        }
    }

    /// 상세보기 모달 토글
    pub fn toggle_detail_modal(&mut self) {
        self.show_detail_modal = !self.show_detail_modal;
    }

    /// 상세보기 모달 닫기
    pub fn close_detail_modal(&mut self) {
        self.show_detail_modal = false;
    }

    /// 검색 모드 시작
    pub fn start_search_mode(&mut self) {
        self.search_mode = true;
    }

    /// 검색 완료 (검색어 유지)
    pub fn finish_search_mode(&mut self) {
        self.search_mode = false;
    }

    /// 검색 취소 (검색어 초기화)
    pub fn cancel_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.current_page = 0;
    }

    /// 검색어에 문자 추가
    pub fn add_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.current_page = 0;
    }

    /// 검색어에서 문자 삭제
    pub fn backspace_search(&mut self) {
        self.search_query.pop();
        self.current_page = 0;
    }
}
