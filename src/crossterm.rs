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
                if key.kind == KeyEventKind::Press {
                    let mut app_guard = rt.block_on(app.lock());
                    match key.code {
                        KeyCode::Left | KeyCode::Char('h') => {
                            if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                app_guard.on_group_previous();
                            } else {
                                app_guard.on_left();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => app_guard.on_up(),
                        KeyCode::Right | KeyCode::Char('l') => {
                            if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                app_guard.on_group_next();
                            } else {
                                app_guard.on_right();
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app_guard.on_down(),
                        KeyCode::Tab => app_guard.on_right(),
                        KeyCode::BackTab => app_guard.on_left(),
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            if app_guard.current_tab == crate::app::TabIndex::ResourceUsage
                                && !app_guard.is_collecting
                            {
                                // 수집 시작
                                let app_clone = app.clone();
                                collection_task = Some(rt.spawn(async move {
                                    let mut app = app_clone.lock().await;
                                    if let Err(e) = app.start_collection().await {
                                        eprintln!("수집 실패: {}", e);
                                    }
                                }));
                            }
                        }
                        KeyCode::Char(c) => app_guard.on_key(c),
                        KeyCode::Esc => app_guard.should_quit = true,
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
        if let Some(task) = &mut collection_task {
            if task.is_finished() {
                collection_task = None;
                // 수집 완료 후 3초 후에 상태를 Idle로 변경
                let app_clone = app.clone();
                rt.spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    let mut app_guard = app_clone.lock().await;
                    if app_guard.resource_usage.collection_status == crate::app::CollectionStatus::Success
                        || app_guard.resource_usage.collection_status == crate::app::CollectionStatus::Failed {
                        app_guard.resource_usage.collection_status = crate::app::CollectionStatus::Idle;
                        app_guard.resource_usage.collection_progress = None;
                    }
                });
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

