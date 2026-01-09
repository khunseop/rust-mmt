use crate::app::config::get_config_path;
use crate::app::states::{ResourceUsageState, SessionBrowserState, TrafficLogsState};
use crate::app::types::{CollectionStatus, Proxy, ProxyConfig, TabIndex};

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
        match self.current_tab {
            TabIndex::SessionBrowser => {
                // 세션 브라우저 탭에서는 테이블 가로 스크롤
                self.session_browser.scroll_left();
            }
            _ => {
                // 다른 탭에서는 탭 전환
                self.current_tab = self.current_tab.previous();
            }
        }
    }

    pub fn on_right(&mut self) {
        match self.current_tab {
            TabIndex::SessionBrowser => {
                // 세션 브라우저 탭에서는 테이블 가로 스크롤
                self.session_browser.scroll_right();
            }
            _ => {
                // 다른 탭에서는 탭 전환
                self.current_tab = self.current_tab.next();
            }
        }
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
                
                // 정렬 적용
                let mut sorted_sessions = sessions;
                Self::sort_sessions(&mut sorted_sessions, 
                    self.session_browser.sort_column, 
                    self.session_browser.sort_ascending);
                
                self.session_browser.sessions = sorted_sessions;
                let now = chrono::Local::now();
                self.session_browser.last_query_time = Some(now);
                
                self.session_browser.query_status = CollectionStatus::Success;
                self.session_browser.query_progress = Some((success_count, total_count));
                
                // 페이지네이션 업데이트
                self.session_browser.update_total_pages(self.session_browser.sessions.len());
                
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

    /// 세션 목록 정렬
    pub fn sort_sessions(sessions: &mut [crate::app::types::SessionData], column: Option<usize>, ascending: bool) {
        if let Some(col) = column {
            match col {
                0 => sessions.sort_by(|a, b| {
                    let ord = a.host.cmp(&b.host);
                    if ascending { ord } else { ord.reverse() }
                }),
                1 => sessions.sort_by(|a, b| {
                    let ord = a.transaction.as_ref().unwrap_or(&String::new())
                        .cmp(b.transaction.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                2 => sessions.sort_by(|a, b| {
                    let ord = a.creation_time.cmp(&b.creation_time);
                    if ascending { ord } else { ord.reverse() }
                }),
                3 => sessions.sort_by(|a, b| {
                    let ord = a.protocol.as_ref().unwrap_or(&String::new())
                        .cmp(b.protocol.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                4 => sessions.sort_by(|a, b| {
                    let ord = a.cust_id.as_ref().unwrap_or(&String::new())
                        .cmp(b.cust_id.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                5 => sessions.sort_by(|a, b| {
                    let ord = a.user_name.as_ref().unwrap_or(&String::new())
                        .cmp(b.user_name.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                6 => sessions.sort_by(|a, b| {
                    let ord = a.client_ip.cmp(&b.client_ip);
                    if ascending { ord } else { ord.reverse() }
                }),
                7 => sessions.sort_by(|a, b| {
                    let ord = a.client_side_mwg_ip.as_ref().unwrap_or(&String::new())
                        .cmp(b.client_side_mwg_ip.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                8 => sessions.sort_by(|a, b| {
                    let ord = a.server_side_mwg_ip.as_ref().unwrap_or(&String::new())
                        .cmp(b.server_side_mwg_ip.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                9 => sessions.sort_by(|a, b| {
                    let ord = a.server_ip.as_ref().unwrap_or(&String::new())
                        .cmp(b.server_ip.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                10 => sessions.sort_by(|a, b| {
                    let ord = a.cl_bytes_received.cmp(&b.cl_bytes_received);
                    if ascending { ord } else { ord.reverse() }
                }),
                11 => sessions.sort_by(|a, b| {
                    let ord = a.cl_bytes_sent.cmp(&b.cl_bytes_sent);
                    if ascending { ord } else { ord.reverse() }
                }),
                12 => sessions.sort_by(|a, b| {
                    let ord = a.srv_bytes_received.cmp(&b.srv_bytes_received);
                    if ascending { ord } else { ord.reverse() }
                }),
                13 => sessions.sort_by(|a, b| {
                    let ord = a.srv_bytes_sent.cmp(&b.srv_bytes_sent);
                    if ascending { ord } else { ord.reverse() }
                }),
                14 => sessions.sort_by(|a, b| {
                    let ord = a.trxn_index.cmp(&b.trxn_index);
                    if ascending { ord } else { ord.reverse() }
                }),
                15 => sessions.sort_by(|a, b| {
                    let ord = a.age_seconds.cmp(&b.age_seconds);
                    if ascending { ord } else { ord.reverse() }
                }),
                16 => sessions.sort_by(|a, b| {
                    let ord = a.status.as_ref().unwrap_or(&String::new())
                        .cmp(b.status.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                17 => sessions.sort_by(|a, b| {
                    let ord = a.in_use.cmp(&b.in_use);
                    if ascending { ord } else { ord.reverse() }
                }),
                18 => sessions.sort_by(|a, b| {
                    let ord = a.url.as_ref().unwrap_or(&String::new())
                        .cmp(b.url.as_ref().unwrap_or(&String::new()));
                    if ascending { ord } else { ord.reverse() }
                }),
                _ => {}
            }
        }
    }
}
