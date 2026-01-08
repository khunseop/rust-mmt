use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, Tabs},
    Frame,
};

use crate::app::{App, TabIndex};
use std::collections::HashMap;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(frame.size());

    // 탭 헤더
    let tabs = Tabs::new(vec!["프록시관리", "자원사용률", "세션브라우저", "트래픽로그"])
        .block(Block::default().borders(Borders::ALL).title(app.title.clone()))
        .select(match app.current_tab {
            TabIndex::ProxyManagement => 0,
            TabIndex::ResourceUsage => 1,
            TabIndex::SessionBrowser => 2,
            TabIndex::TrafficLogs => 3,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, chunks[0]);

    // 각 탭의 콘텐츠
    match app.current_tab {
        TabIndex::ProxyManagement => draw_proxy_management(frame, app, chunks[1]),
        TabIndex::ResourceUsage => draw_resource_usage(frame, app, chunks[1]),
        TabIndex::SessionBrowser => draw_session_browser(frame, app, chunks[1]),
        TabIndex::TrafficLogs => draw_traffic_logs(frame, app, chunks[1]),
    }
}

fn draw_proxy_management(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // 헤더 영역
    let header = Block::default()
        .borders(Borders::ALL)
        .title("프록시 관리");
    frame.render_widget(header, chunks[0]);

    // 프록시 목록 테이블
    let proxy_table = if app.proxies.is_empty() {
        Table::new(
            vec![Row::new(vec![Cell::from("프록시가 설정되지 않았습니다. config/proxies.json을 확인하세요.")])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL).title("프록시 목록"))
    } else {
        // 그룹별로 프록시 그룹화
        use std::collections::HashMap;
        let mut groups: HashMap<String, Vec<&crate::app::Proxy>> = HashMap::new();
        for proxy in &app.proxies {
            groups.entry(proxy.group.clone()).or_insert_with(Vec::new).push(proxy);
        }

        let mut rows = Vec::new();
        for (group, proxies) in &groups {
            // 그룹 헤더
            rows.push(Row::new(vec![
                Cell::from(format!("📁 {}", group))
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ]));

            // 그룹 내 프록시들
            for proxy in proxies {
                rows.push(Row::new(vec![
                    Cell::from(format!("  └─ {}", proxy.host)),
                    Cell::from(format!(":{}", proxy.port)),
                    Cell::from(proxy.username.clone()),
                    Cell::from(proxy.group.clone()),
                ]));
            }
        }

        Table::new(rows, [
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .header(Row::new(vec![
            Cell::from("호스트").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("포트").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("사용자").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("그룹").style(Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL).title(format!("프록시 목록 ({}개)", app.proxies.len())))
    };
    frame.render_widget(proxy_table, chunks[1]);
}

/// 실행 파일의 디렉터리를 기준으로 config 파일 경로를 반환합니다.
fn get_config_path(filename: &str) -> std::path::PathBuf {
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
    let current_dir_path = std::path::Path::new("config").join(filename);
    if current_dir_path.exists() {
        return current_dir_path;
    }
    
    // 둘 다 없으면 기본값으로 현재 작업 디렉터리 반환 (에러는 나중에 발생)
    std::path::Path::new("config").join(filename)
}

/// 설정 파일에서 회선 목록을 읽어옵니다.
fn get_interface_names() -> Vec<String> {
    let config_path = get_config_path("resource_config.json");
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(interface_oids) = config.get("interface_oids").and_then(|v| v.as_object()) {
                let mut names: Vec<String> = interface_oids.keys().cloned().collect();
                names.sort(); // 정렬하여 일관된 순서 유지
                return names;
            }
        }
    }
    Vec::new()
}

fn draw_resource_usage(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 컨트롤 영역 (한 줄)
            Constraint::Min(3),     // 데이터 테이블
            Constraint::Length(4),  // 키보드 단축키 도움말 (컴팩트)
        ])
        .split(area);

    // 컨트롤 영역을 한 줄로 구성 (선택 불가능)
    let control_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(18), // 그룹선택
            Constraint::Length(18), // 자동수집
            Constraint::Length(18), // 수집주기
            Constraint::Length(18), // 상태
            Constraint::Length(20), // 마지막수집
            Constraint::Min(0),     // 나머지
        ])
        .split(chunks[0]);
    
    use ratatui::widgets::Paragraph;
    
    // 컨트롤 렌더링 헬퍼 함수 (선택 불가능, 정보 표시만)
    fn render_info_box(frame: &mut Frame, title: &str, content: &str, style: Style, area: Rect) {
        frame.render_widget(
            Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title(title))
                .style(style),
            area,
        );
    }
    
    // 그룹선택
    let group_name = app.resource_usage.get_group_display_name();
    render_info_box(frame, "그룹선택", &group_name, Style::default().fg(Color::Cyan), control_chunks[0]);
    
    // 자동수집
    let auto_status = if app.resource_usage.auto_collection_enabled {
        if let Some(next_time) = app.resource_usage.next_auto_collection_time {
            let remaining = (next_time - chrono::Local::now()).num_seconds();
            if remaining > 0 {
                format!("🔄 ON ({}초 후)", remaining)
            } else {
                "🔄 ON".to_string()
            }
        } else {
            "🔄 ON".to_string()
        }
    } else {
        "▶ OFF".to_string()
    };
    let auto_style = if app.resource_usage.auto_collection_enabled {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };
    render_info_box(frame, "자동수집", &auto_status, auto_style, control_chunks[1]);
    
    // 수집주기
    let interval = app.resource_usage.get_interval_display();
    render_info_box(frame, "수집주기", &interval, Style::default().fg(Color::White), control_chunks[2]);
    
    // 상태
    let (status_text, status_color, elapsed_sec) = match app.resource_usage.collection_status {
        crate::app::CollectionStatus::Idle => ("대기중".to_string(), Color::Gray, None),
        crate::app::CollectionStatus::Starting => ("시작중".to_string(), Color::Yellow, None),
        crate::app::CollectionStatus::Collecting => {
            let elapsed = app.resource_usage.collection_start_time
                .map(|start| (chrono::Local::now() - start).num_seconds());
            if let Some((completed, total)) = app.resource_usage.collection_progress {
                (format!("수집중 ({}/{})", completed, total), Color::Yellow, elapsed)
            } else {
                ("수집중".to_string(), Color::Yellow, elapsed)
            }
        }
        crate::app::CollectionStatus::Success => ("완료".to_string(), Color::Green, None),
        crate::app::CollectionStatus::Failed => ("실패".to_string(), Color::Red, None),
    };
    let status_display = if let Some(elapsed) = elapsed_sec {
        format!("{}\n{}초", status_text, elapsed)
    } else {
        status_text
    };
    render_info_box(frame, "상태", &status_display, Style::default().fg(status_color), control_chunks[3]);

    // 마지막 수집 시간
    let last_collection_text = if let Some(last_time) = app.resource_usage.last_collection_time {
        format!("{}\n{}", 
            last_time.format("%H:%M:%S"),
            last_time.format("%m/%d"))
    } else {
        "없음".to_string()
    };
    render_info_box(frame, "마지막수집", &last_collection_text, Style::default().fg(Color::Cyan), control_chunks[4]);

    // 회선 목록 가져오기
    let interface_names = get_interface_names();
    
    // 테이블 영역 - Python 앱과 동일한 구조
    let table = if app.resource_usage.data.is_empty() {
        // 데이터가 없을 때 빈 테이블
        Table::new(
            vec![Row::new(vec![
                Cell::from("데이터가 없습니다. 시작 버튼을 눌러 수집하세요."),
            ])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL))
    } else {
        // 데이터가 있을 때 실제 테이블 - 프록시별 행
        let rows: Vec<Row> = app
            .resource_usage
            .data
            .iter()
            .enumerate()
            .map(|(i, data)| {
                // 실패한 경우
                if data.collection_failed {
                    let _error_msg = data.error_message.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("실패");
                    
                    let style = if app.resource_usage.table_state.selected() == Some(i) {
                        Style::default().bg(Color::Red).fg(Color::White)
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    // 기본 컬럼 + 회선 컬럼들 (컴팩트하게)
                    let proxy_display_name = data.proxy_name.as_ref().unwrap_or(&data.host);
                    let mut cells = vec![
                        Cell::from(proxy_display_name.clone()).style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                    ];
                    
                    // 각 회선에 대해 빈 셀 추가
                    for _ in &interface_names {
                        cells.push(Cell::from("-").style(style));
                    }
                    
                    cells.push(Cell::from("✗").style(style));
                    Row::new(cells)
                } else {
                    // 성공한 경우
                    let format_percent = |v: Option<f64>| -> String {
                        v.map(|val| format!("{:.0}%", val))
                            .unwrap_or_else(|| "-".to_string())
                    };

                    // 개수 표시 (CC, CS)
                    let format_count = |v: Option<f64>| -> String {
                        v.map(|val| {
                            let count = val as u64;
                            if count >= 1_000_000 {
                                format!("{:.1}M", count as f64 / 1_000_000.0)
                            } else if count >= 1_000 {
                                format!("{:.1}K", count as f64 / 1_000.0)
                            } else {
                                format!("{}", count)
                            }
                        })
                        .unwrap_or_else(|| "-".to_string())
                    };

                    // bps를 컴팩트한 형식으로 표시 (K/M/G 단위)
                    let format_bps = |v: Option<f64>| -> String {
                        v.map(|bps| {
                            if bps >= 1_000_000_000.0 {
                                format!("{:.1}G", bps / 1_000_000_000.0)
                            } else if bps >= 1_000_000.0 {
                                format!("{:.1}M", bps / 1_000_000.0)
                            } else if bps >= 1_000.0 {
                                format!("{:.1}K", bps / 1_000.0)
                            } else {
                                format!("{:.0}", bps)
                            }
                        })
                        .unwrap_or_else(|| "-".to_string())
                    };

                    let cpu_str = format_percent(data.cpu);
                    let mem_str = format_percent(data.mem);
                    let cc_str = format_count(data.cc);
                    let cs_str = format_count(data.cs);
                    let http_str = format_bps(data.http);
                    let https_str = format_bps(data.https);
                    let ftp_str = format_bps(data.ftp);
                    
                    // 회선 정보를 HashMap으로 변환 (빠른 조회를 위해)
                    let interface_map: HashMap<String, (f64, f64)> = data.interfaces
                        .iter()
                        .map(|iface| (iface.name.clone(), (iface.in_mbps, iface.out_mbps)))
                        .collect();

                    let style = if app.resource_usage.table_state.selected() == Some(i) {
                        Style::default().bg(Color::Blue)
                    } else {
                        Style::default()
                    };

                    // 기본 컬럼들
                    let proxy_display_name = data.proxy_name.as_ref().unwrap_or(&data.host);
                    let mut cells = vec![
                        Cell::from(proxy_display_name.clone()).style(style),
                        Cell::from(cpu_str).style(style),
                        Cell::from(mem_str).style(style),
                        Cell::from(cc_str).style(style),
                        Cell::from(cs_str).style(style),
                        Cell::from(http_str).style(style),
                        Cell::from(https_str).style(style),
                        Cell::from(ftp_str).style(style),
                    ];
                    
                    // 각 회선에 대해 별도 컬럼 추가 (bps를 컴팩트하게 표시)
                    for if_name in &interface_names {
                        if let Some((in_bps, out_bps)) = interface_map.get(if_name) {
                            let in_str = if *in_bps >= 1_000_000_000.0 {
                                format!("{:.1}G", in_bps / 1_000_000_000.0)
                            } else if *in_bps >= 1_000_000.0 {
                                format!("{:.1}M", in_bps / 1_000_000.0)
                            } else if *in_bps >= 1_000.0 {
                                format!("{:.1}K", in_bps / 1_000.0)
                            } else {
                                format!("{:.0}", in_bps)
                            };
                            let out_str = if *out_bps >= 1_000_000_000.0 {
                                format!("{:.1}G", out_bps / 1_000_000_000.0)
                            } else if *out_bps >= 1_000_000.0 {
                                format!("{:.1}M", out_bps / 1_000_000.0)
                            } else if *out_bps >= 1_000.0 {
                                format!("{:.1}K", out_bps / 1_000.0)
                            } else {
                                format!("{:.0}", out_bps)
                            };
                            cells.push(Cell::from(format!("{}/{}", in_str, out_str)).style(style));
                        } else {
                            cells.push(Cell::from("-").style(style));
                        }
                    }
                    
                    // 상태 컬럼
                    cells.push(Cell::from("✓").style(style));
                    
                    Row::new(cells)
                }
            })
            .collect();

        // 컬럼 너비 설정 (컴팩트하게)
        let mut constraints = vec![
            Constraint::Length(12),  // 프록시
            Constraint::Length(5),   // CPU
            Constraint::Length(5),   // MEM
            Constraint::Length(5),   // CC
            Constraint::Length(5),   // CS
            Constraint::Length(6),   // HTTP (bps)
            Constraint::Length(6),   // HTTPS (bps)
            Constraint::Length(6),   // FTP (bps)
        ];
        
        // 각 회선에 대해 컬럼 추가 (너비 증가로 잘림 방지)
        for _ in &interface_names {
            constraints.push(Constraint::Length(12)); // 각 회선 컬럼 (in/out bps) - 9에서 12로 증가
        }
        
        constraints.push(Constraint::Length(3)); // 상태 컬럼
        
        // 헤더 생성
        let mut header_cells = vec![
            Cell::from("프록시").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CPU%").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("MEM%").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CC").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CS").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("HTTP").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("HTTPS").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("FTP").style(Style::default().add_modifier(Modifier::BOLD)),
        ];
        
        // 각 회선에 대해 헤더 추가 (컴팩트하게)
        for if_name in &interface_names {
            // 인터페이스 이름을 컴팩트하게 표시
            // bond0, bond1 같은 경우를 고려하여 숫자 포함
            let short_name = if if_name.len() > 5 {
                // 5자 초과면 앞부분 + 마지막 숫자 (예: "eth10" -> "eth1")
                if let Some(last_char) = if_name.chars().last() {
                    if last_char.is_ascii_digit() {
                        // 마지막 문자가 숫자면 앞부분 + 숫자
                        let prefix_len = (if_name.len() - 1).min(4);
                        format!("{}{}", &if_name[..prefix_len], last_char)
                    } else {
                        format!("{}", &if_name[..5])
                    }
                } else {
                    format!("{}", &if_name[..5])
                }
            } else {
                // 5자 이하면 그대로 표시 (bond0, bond1 등)
                if_name.clone()
            };
            header_cells.push(Cell::from(short_name).style(Style::default().add_modifier(Modifier::BOLD)));
        }
        
        header_cells.push(Cell::from("✓").style(Style::default().add_modifier(Modifier::BOLD)));
        
        Table::new(rows, constraints)
        .header(Row::new(header_cells))
        .block(Block::default().borders(Borders::ALL).title("자원 사용률 모니터링"))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ")
    };

    frame.render_stateful_widget(table, chunks[1], &mut app.resource_usage.table_state);

    // 키보드 단축키 도움말 (컴팩트)
    let help_text = vec![
        "Tab: 탭전환 | q/Esc: 종료 | ↑↓: 테이블이동 | Space: 자동수집토글 | +/-: 주기 | Shift+←→: 그룹",
    ];
    frame.render_widget(
        Paragraph::new(help_text.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("단축키"))
            .style(Style::default().fg(Color::Gray)),
        chunks[2],
    );
}

fn draw_session_browser(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // 헤더 영역
    let header = Block::default()
        .borders(Borders::ALL)
        .title("세션 브라우저");
    frame.render_widget(header, chunks[0]);

    // 테이블 영역
    let table = if app.session_browser.sessions.is_empty() {
        Table::new(
            vec![Row::new(vec![
                Cell::from("데이터가 없습니다. [S] 키를 눌러 조회하세요."),
            ])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL))
    } else {
        let rows: Vec<Row> = app
            .session_browser
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let style = if app.session_browser.table_state.selected() == Some(i) {
                    Style::default().bg(Color::Blue)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(session.host.clone()).style(style),
                    Cell::from(session.client_ip.clone()).style(style),
                    Cell::from(
                        session
                            .url
                            .as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or("N/A"),
                    )
                    .style(style),
                ])
            })
            .collect();

        Table::new(rows, [
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ])
        .header(Row::new(vec![
            Cell::from("호스트").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("클라이언트IP").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("URL").style(Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ")
    };

    frame.render_stateful_widget(table, chunks[1], &mut app.session_browser.table_state);
}

fn draw_traffic_logs(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // 헤더 영역
    let header = Block::default()
        .borders(Borders::ALL)
        .title("트래픽 로그 분석");
    frame.render_widget(header, chunks[0]);

    // 콘텐츠 영역
    let content = if let Some(result) = &app.traffic_logs.analysis_result {
        result.clone()
    } else {
        "데이터가 없습니다. [A] 키를 눌러 분석하세요.".to_string()
    };

    let block = Block::default().borders(Borders::ALL);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(content).block(block),
        chunks[1],
    );
}

