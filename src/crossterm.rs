use std::{
    error::Error,
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use tokio::sync::Mutex;

use crate::{app::App, ui};


/// 수집 작업을 시작하는 헬퍼 함수
fn spawn_collection_task(
    app: Arc<tokio::sync::Mutex<App>>,
    rt: &tokio::runtime::Runtime,
) -> tokio::task::JoinHandle<()> {
    let app_clone = app.clone();
    rt.spawn(async move {
        let mut app = app_clone.lock().await;
        if let Err(e) = app.start_collection().await {
            eprintln!("수집 실패: {}", e);
        }
    })
}

pub fn run(tick_rate: Duration) -> Result<(), Box<dyn Error>> {
    // 터미널 설정
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 앱 생성 및 프록시 로드
    let mut app = App::new("MWG Monitoring Tool".to_string());
    if let Err(e) = app.load_proxies() {
        eprintln!("프록시 설정 파일 로드 실패: {}", e);
        eprintln!("config/proxies.json 파일을 확인하세요.");
    }

    // 런타임 생성
    let rt = tokio::runtime::Runtime::new()?;
    let app_mutex = Arc::new(Mutex::new(app));
    
    // 스피너 애니메이션을 위한 주기적 업데이트 태스크
    let app_for_spinner = app_mutex.clone();
    rt.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100)); // 100ms마다 업데이트
        loop {
            interval.tick().await;
            let mut app_guard = app_for_spinner.lock().await;
            // 자원 사용률 탭 스피너
            if app_guard.resource_usage.collection_status == crate::app::CollectionStatus::Collecting
                || app_guard.resource_usage.collection_status == crate::app::CollectionStatus::Starting {
                app_guard.resource_usage.spinner_frame = (app_guard.resource_usage.spinner_frame + 1) % 10;
            }
            // 세션 브라우저 탭 스피너
            if app_guard.session_browser.query_status == crate::app::CollectionStatus::Collecting
                || app_guard.session_browser.query_status == crate::app::CollectionStatus::Starting {
                app_guard.session_browser.spinner_frame = (app_guard.session_browser.spinner_frame + 1) % 10;
            }
            // 트래픽 로그 탭 스피너
            if app_guard.traffic_logs.query_status == crate::app::CollectionStatus::Collecting
                || app_guard.traffic_logs.query_status == crate::app::CollectionStatus::Starting {
                app_guard.traffic_logs.spinner_frame = (app_guard.traffic_logs.spinner_frame + 1) % 10;
            }
        }
    });

    // 앱 실행
    let app_result = run_app(&mut terminal, app_mutex, tick_rate, rt);

    // 터미널 복원
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = app_result {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
    tick_rate: Duration,
    rt: tokio::runtime::Runtime,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut collection_task: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        // UI 렌더링
        {
            let mut app_guard = rt.block_on(app.lock());
            terminal.draw(|frame| ui::draw(frame, &mut *app_guard))?;
        }

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Ctrl+C 처리
                if key.code == KeyCode::Char('c') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    let mut app_guard = rt.block_on(app.lock());
                    app_guard.should_quit = true;
                    drop(app_guard);
                    return Ok(());
                }
                
                if key.kind == KeyEventKind::Press {
                    let mut app_guard = rt.block_on(app.lock());
                    match key.code {
                        KeyCode::Left | KeyCode::Char('h') => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser
                                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                                && app_guard.session_browser.selected_column.is_some() {
                                // Ctrl+←: 컬럼 순서 변경 (왼쪽으로 이동)
                                if let Some(col_idx) = app_guard.session_browser.selected_column {
                                    app_guard.session_browser.move_column_left(col_idx);
                                    // 선택된 컬럼도 함께 이동
                                    if col_idx > 0 {
                                        app_guard.session_browser.selected_column = Some(col_idx - 1);
                                    }
                                }
                            } else if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                app_guard.on_group_previous();
                            } else {
                                app_guard.on_left();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => app_guard.on_up(),
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser
                                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                                && app_guard.session_browser.selected_column.is_some() {
                                // Ctrl+→: 컬럼 순서 변경 (오른쪽으로 이동)
                                if let Some(col_idx) = app_guard.session_browser.selected_column {
                                    app_guard.session_browser.move_column_right(col_idx);
                                    // 선택된 컬럼도 함께 이동
                                    if col_idx < 18 {
                                        app_guard.session_browser.selected_column = Some(col_idx + 1);
                                    }
                                }
                            } else if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                app_guard.on_group_next();
                            } else {
                                app_guard.on_right();
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app_guard.on_down(),
                        KeyCode::Tab => {
                            // Tab 키는 항상 탭 전환만 (모든 탭에서 동일하게 동작)
                            app_guard.current_tab = app_guard.current_tab.next();
                        }
                        KeyCode::BackTab => {
                            // Shift+Tab도 항상 탭 전환
                            app_guard.current_tab = app_guard.current_tab.previous();
                        }
                        KeyCode::Char(' ') => {
                            // Space 키 처리
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                // 세션 브라우저 탭: 다음 페이지
                                app_guard.session_browser.next_page();
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                // 트래픽 로그 탭: 다음 페이지
                                app_guard.traffic_logs.next_page();
                            } else if app_guard.current_tab == crate::app::TabIndex::ResourceUsage
                                && app_guard.resource_usage.collection_status == crate::app::CollectionStatus::Idle
                            {
                                // 자원 사용률 탭: 자동 수집 토글
                                app_guard.resource_usage.toggle_auto_collection();
                                if app_guard.resource_usage.auto_collection_enabled && collection_task.is_none() {
                                    collection_task = Some(spawn_collection_task(app.clone(), &rt));
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                if app_guard.session_browser.search_mode {
                                    // 검색 모드에서 Enter: 검색 완료 (검색어 유지)
                                    app_guard.session_browser.finish_search_mode();
                                } else {
                                    // 일반 모드에서 Enter: 상세보기 모달 토글
                                    app_guard.session_browser.toggle_detail_modal();
                                }
                            }
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                if app_guard.session_browser.search_mode {
                                    // 검색 모드에서는 문자 입력
                                    app_guard.session_browser.add_search_char('q');
                                } else if app_guard.session_browser.show_detail_modal {
                                    // 모달이 열려있으면 모달만 닫기
                                    app_guard.session_browser.close_detail_modal();
                                }
                                // q 키로 종료하지 않음 (Ctrl+C 사용)
                            }
                            // q 키로 종료하지 않음 (Ctrl+C 사용)
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            if app_guard.current_tab == crate::app::TabIndex::ResourceUsage {
                                app_guard.resource_usage.increase_interval();
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                // 조회 라인 수 증가 (100씩, 최대 2000)
                                if app_guard.traffic_logs.log_limit < 2000 {
                                    app_guard.traffic_logs.log_limit += 100;
                                }
                            }
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            if app_guard.current_tab == crate::app::TabIndex::ResourceUsage {
                                app_guard.resource_usage.decrease_interval();
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                // 조회 라인 수 감소 (100씩, 최소 100)
                                if app_guard.traffic_logs.log_limit > 100 {
                                    app_guard.traffic_logs.log_limit -= 100;
                                }
                            }
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                // R: 세션 조회 시작 (Refresh)
                                if !app_guard.session_browser.search_mode {
                                    let should_query = app_guard.session_browser.query_status != crate::app::CollectionStatus::Collecting;
                                    
                                    if should_query {
                                        // 조회 시작 상태로 즉시 변경
                                        app_guard.session_browser.query_status = crate::app::CollectionStatus::Starting;
                                        app_guard.session_browser.query_start_time = Some(chrono::Local::now());
                                        
                                        let app_clone = app.clone();
                                        drop(app_guard);
                                        rt.spawn(async move {
                                            let mut app_guard = app_clone.lock().await;
                                            if let Err(e) = app_guard.start_session_query().await {
                                                eprintln!("세션 조회 실패: {}", e);
                                                app_guard.session_browser.query_status = crate::app::CollectionStatus::Failed;
                                                app_guard.session_browser.last_error = Some(format!("{}", e));
                                                app_guard.session_browser.query_start_time = None;
                                            }
                                        });
                                        // app_guard가 drop되었으므로 다시 lock 필요
                                        app_guard = rt.block_on(app.lock());
                                    }
                                } else {
                                    // 검색 모드에서는 문자 입력
                                    app_guard.session_browser.add_search_char('r');
                                }
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                // R: 트래픽 로그 조회 시작
                                let should_query = app_guard.traffic_logs.query_status != crate::app::CollectionStatus::Collecting;
                                
                                if should_query {
                                    // 프록시 선택 확인 (선택 안됨 시 첫 번째 프록시 자동 선택)
                                    let proxy_id = if let Some(id) = app_guard.traffic_logs.selected_proxy {
                                        id as u32
                                    } else if !app_guard.proxies.is_empty() {
                                        let id = app_guard.proxies[app_guard.traffic_logs.proxy_list_index].id;
                                        app_guard.traffic_logs.selected_proxy = Some(id as usize);
                                        id
                                    } else {
                                        return Ok(()); // 프록시가 없으면 무시
                                    };
                                    
                                    // 조회 시작 상태로 즉시 변경
                                    app_guard.traffic_logs.query_status = crate::app::CollectionStatus::Starting;
                                    app_guard.traffic_logs.query_start_time = Some(chrono::Local::now());
                                    
                                    let app_clone = app.clone();
                                    drop(app_guard);
                                    rt.spawn(async move {
                                        let mut app_guard = app_clone.lock().await;
                                        if let Err(e) = app_guard.start_traffic_log_query(proxy_id).await {
                                            eprintln!("트래픽 로그 조회 실패: {}", e);
                                            app_guard.traffic_logs.query_status = crate::app::CollectionStatus::Failed;
                                            app_guard.traffic_logs.last_error = Some(format!("{}", e));
                                            app_guard.traffic_logs.query_start_time = None;
                                        }
                                    });
                                    // app_guard가 drop되었으므로 다시 lock 필요
                                    app_guard = rt.block_on(app.lock());
                                }
                            }
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                if !app_guard.session_browser.search_mode {
                                    // S: 정렬 토글 (컬럼이 선택되어 있을 때)
                                    if app_guard.session_browser.selected_column.is_some() {
                                        app_guard.session_browser.toggle_sort();
                                        // 정렬 후 세션 목록 재정렬
                                        let sort_col = app_guard.session_browser.sort_column;
                                        let sort_asc = app_guard.session_browser.sort_ascending;
                                        crate::app::App::sort_sessions(
                                            &mut app_guard.session_browser.sessions,
                                            sort_col,
                                            sort_asc
                                        );
                                    }
                                } else {
                                    // 검색 모드에서는 문자 입력
                                    app_guard.session_browser.add_search_char('s');
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                if app_guard.session_browser.search_mode {
                                    // 검색 모드일 때 - 모든 문자를 검색어로 입력
                                    app_guard.session_browser.add_search_char(c);
                                } else {
                                    // 일반 모드일 때
                                    if c == '/' {
                                        // / 키로 검색 모드 시작
                                        app_guard.session_browser.start_search_mode();
                                    } else if c == 'b' || c == 'B' {
                                        app_guard.session_browser.previous_page();
                                    } else {
                                        app_guard.on_key(c);
                                    }
                                }
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                // 트래픽 로그 탭에서의 문자 키 처리
                                if c == 'b' || c == 'B' {
                                    app_guard.traffic_logs.previous_page();
                                } else {
                                    app_guard.on_key(c);
                                }
                            } else if c == 'a' || c == 'A' {
                                // A 키로 트래픽 로그 분석 시작
                                if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                    if let Some(proxy_id) = app_guard.traffic_logs.selected_proxy {
                                        let proxy_id_u32 = proxy_id as u32;
                                        app_guard.traffic_logs.analysis_status = crate::app::CollectionStatus::Starting;
                                        app_guard.traffic_logs.analysis_start_time = Some(chrono::Local::now());
                                        
                                        let app_clone = app.clone();
                                        drop(app_guard);
                                        rt.spawn(async move {
                                            let mut app_guard = app_clone.lock().await;
                                            if let Err(e) = app_guard.start_traffic_log_analysis(proxy_id_u32).await {
                                                eprintln!("트래픽 로그 분석 실패: {}", e);
                                                app_guard.traffic_logs.analysis_status = crate::app::CollectionStatus::Failed;
                                                app_guard.traffic_logs.last_error = Some(format!("{}", e));
                                                app_guard.traffic_logs.analysis_start_time = None;
                                            }
                                        });
                                        app_guard = rt.block_on(app.lock());
                                    }
                                } else {
                                    app_guard.on_key(c);
                                }
                            } else {
                                app_guard.on_key(c);
                            }
                        }
                        KeyCode::Backspace => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser
                                && app_guard.session_browser.search_mode {
                                app_guard.session_browser.backspace_search();
                            }
                        }
                        KeyCode::Esc => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                if app_guard.session_browser.search_mode {
                                    // 검색 취소 (검색어 초기화)
                                    app_guard.session_browser.cancel_search_mode();
                                } else if app_guard.session_browser.show_detail_modal {
                                    // 모달이 열려있으면 모달만 닫기
                                    app_guard.session_browser.close_detail_modal();
                                } else if app_guard.session_browser.selected_column.is_some() {
                                    // 컬럼이 선택되어 있으면 컬럼 선택 해제
                                    app_guard.session_browser.clear_column_selection();
                                }
                                // Esc로 종료하지 않음 (Ctrl+C 사용)
                            }
                            // Esc로 종료하지 않음 (Ctrl+C 사용)
                        }
                        KeyCode::PageDown => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                app_guard.session_browser.next_page();
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                app_guard.traffic_logs.next_page();
                            }
                        }
                        KeyCode::PageUp => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                app_guard.session_browser.previous_page();
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                app_guard.traffic_logs.previous_page();
                            }
                        }
                        KeyCode::Home => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                app_guard.session_browser.first_page();
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                app_guard.traffic_logs.first_page();
                            }
                        }
                        KeyCode::End => {
                            if app_guard.current_tab == crate::app::TabIndex::SessionBrowser {
                                app_guard.session_browser.last_page();
                            } else if app_guard.current_tab == crate::app::TabIndex::TrafficLogs {
                                app_guard.traffic_logs.last_page();
                            }
                        }
                        _ => {}
                    }
                    
                    let should_quit = app_guard.should_quit;
                    drop(app_guard);
                    
                    if should_quit {
                        return Ok(());
                    }
                }
            }
        }

        // 수집 작업 완료 확인
        if let Some(ref task) = collection_task {
            if task.is_finished() {
                // 작업 완료 처리
                collection_task = None;
                // 수집 완료 후 2초 후에 상태를 Idle로 변경
                let app_clone = app.clone();
                rt.spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    let mut app_guard = app_clone.lock().await;
                    if app_guard.resource_usage.collection_status == crate::app::CollectionStatus::Success
                        || app_guard.resource_usage.collection_status == crate::app::CollectionStatus::Failed {
                        app_guard.resource_usage.collection_status = crate::app::CollectionStatus::Idle;
                        app_guard.resource_usage.collection_progress = None;
                        app_guard.resource_usage.collection_start_time = None;
                    }
                });
            }
        }

        // 자동 수집 확인 및 실행
        {
            let app_guard = rt.block_on(app.lock());
            if app_guard.resource_usage.should_trigger_auto_collection() && collection_task.is_none() {
                drop(app_guard);
                collection_task = Some(spawn_collection_task(app.clone(), &rt));
            }
        }

        if last_tick.elapsed() >= tick_rate {
            let mut app_guard = rt.block_on(app.lock());
            app_guard.on_tick();
            drop(app_guard);
            last_tick = Instant::now();
        }

        {
            let app_guard = rt.block_on(app.lock());
            if app_guard.should_quit {
                return Ok(());
            }
        }
    }
}

