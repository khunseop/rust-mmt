use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 컨트롤 영역
            Constraint::Min(3),     // 데이터 테이블
            Constraint::Length(4),  // 키보드 단축키 도움말
        ])
        .split(area);

    // 컨트롤 영역
    let control_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // 프록시 선택
            Constraint::Length(18), // 상태
            Constraint::Length(20), // 마지막조회
            Constraint::Length(15), // 조회라인수
            Constraint::Min(0),     // 페이지 정보
        ])
        .split(chunks[0]);

    // 프록시 선택
    let proxy_text = if let Some(proxy_id) = app.traffic_logs.selected_proxy {
        if let Some(proxy) = app.proxies.iter().find(|p| p.id == proxy_id as u32) {
            format!("{}\n{}", proxy.host, proxy.group)
        } else {
            "선택 안됨".to_string()
        }
    } else if !app.proxies.is_empty() {
        // 첫 번째 프록시 자동 선택
        let proxy = &app.proxies[app.traffic_logs.proxy_list_index % app.proxies.len()];
        format!("{}\n{}", proxy.host, proxy.group)
    } else {
        "프록시 없음".to_string()
    };
    frame.render_widget(
        Paragraph::new(proxy_text.as_str())
            .block(Block::default().borders(Borders::ALL).title("프록시(↑↓)"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[0],
    );

    // 상태 (스피너 포함) - 조회 상태 우선
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_char = spinner_chars[app.traffic_logs.spinner_frame % spinner_chars.len()];
    
    let (status_text, status_color, elapsed_sec) = if app.traffic_logs.query_status == crate::app::CollectionStatus::Collecting
        || app.traffic_logs.query_status == crate::app::CollectionStatus::Starting {
        let elapsed = app.traffic_logs.query_start_time
            .map(|start| (chrono::Local::now() - start).num_seconds());
        (format!("{} 조회중", spinner_char), Color::Yellow, elapsed)
    } else {
        match app.traffic_logs.query_status {
            crate::app::CollectionStatus::Idle => ("대기중".to_string(), Color::Gray, None),
            crate::app::CollectionStatus::Starting => {
                (format!("{} 시작중", spinner_char), Color::Yellow, None)
            }
            crate::app::CollectionStatus::Collecting => {
                let elapsed = app.traffic_logs.query_start_time
                    .map(|start| (chrono::Local::now() - start).num_seconds());
                (format!("{} 조회중", spinner_char), Color::Yellow, elapsed)
            }
            crate::app::CollectionStatus::Success => ("✓ 완료".to_string(), Color::Green, None),
            crate::app::CollectionStatus::Failed => ("✗ 실패".to_string(), Color::Red, None),
        }
    };
    let status_display = if let Some(elapsed) = elapsed_sec {
        format!("{}\n{}초", status_text, elapsed)
    } else {
        status_text
    };
    frame.render_widget(
        Paragraph::new(status_display.as_str())
            .block(Block::default().borders(Borders::ALL).title("상태"))
            .style(Style::default().fg(status_color)),
        control_chunks[1],
    );

    // 마지막 조회 시간
    let last_query_text = if let Some(last_time) = app.traffic_logs.last_query_time {
        format!("{}\n{}", 
            last_time.format("%H:%M:%S"),
            last_time.format("%m/%d"))
    } else {
        "없음".to_string()
    };
    frame.render_widget(
        Paragraph::new(last_query_text.as_str())
            .block(Block::default().borders(Borders::ALL).title("마지막조회"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[2],
    );

    // 조회 라인 수
    frame.render_widget(
        Paragraph::new(format!("{}", app.traffic_logs.log_limit))
            .block(Block::default().borders(Borders::ALL).title("조회라인"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[3],
    );

    // 페이지 정보 (로그 목록 뷰일 때만)
    let total_records = app.traffic_logs.log_records.len();
    let page_info = if total_records > 0 {
        format!("페이지 {}/{} (총 {}개)", 
            app.traffic_logs.current_page + 1,
            app.traffic_logs.total_pages.max(1),
            total_records)
    } else {
        "페이지 0/0 (0개)".to_string()
    };
    frame.render_widget(
        Paragraph::new(page_info.as_str())
            .block(Block::default().borders(Borders::ALL).title("페이지"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[4],
    );

    // 테이블 영역
    match app.traffic_logs.view_mode {
        crate::app::states::TrafficLogViewMode::LogList => {
            // 로그 목록 뷰
            draw_log_list(frame, app, chunks[1]);
        }
        _ => {
            // 분석 뷰
            let table = if let Some(ref analysis) = app.traffic_logs.top_n_analysis {
                match app.traffic_logs.view_mode {
                    crate::app::states::TrafficLogViewMode::Summary => {
                        draw_summary_table(frame, chunks[1], analysis, app)
                    }
                    crate::app::states::TrafficLogViewMode::TopClients => {
                        draw_top_clients_table(frame, chunks[1], analysis)
                    }
                    crate::app::states::TrafficLogViewMode::TopHosts => {
                        draw_top_hosts_table(frame, chunks[1], analysis)
                    }
                    crate::app::states::TrafficLogViewMode::TopUrls => {
                        draw_top_urls_table(frame, chunks[1], analysis)
                    }
                    crate::app::states::TrafficLogViewMode::LogList => {
                        unreachable!()
                    }
                }
            } else {
                let spinner_char = spinner_chars[app.traffic_logs.spinner_frame % spinner_chars.len()];
                
                let empty_message = if app.traffic_logs.query_status == crate::app::CollectionStatus::Collecting
                    || app.traffic_logs.query_status == crate::app::CollectionStatus::Starting {
                    format!("{} 조회 중...", spinner_char)
                } else if app.traffic_logs.query_status == crate::app::CollectionStatus::Failed {
                    if let Some(ref error) = app.traffic_logs.last_error {
                        format!("조회 실패: {}", error)
                    } else {
                        "조회 실패".to_string()
                    }
                } else {
                    "데이터가 없습니다. 프록시를 선택하고 [R] 키를 눌러 조회하세요.".to_string()
                };
                Table::new(
                    vec![Row::new(vec![
                        Cell::from(empty_message),
                    ])],
                    [Constraint::Percentage(100)],
                )
                .block(Block::default().borders(Borders::ALL).title("트래픽 로그"))
            };

            frame.render_widget(table, chunks[1]);
        }
    }

    // 키보드 단축키 도움말
    let view_mode_text = match app.traffic_logs.view_mode {
        crate::app::states::TrafficLogViewMode::Summary => "요약",
        crate::app::states::TrafficLogViewMode::TopClients => "TOP클라이언트",
        crate::app::states::TrafficLogViewMode::TopHosts => "TOP호스트",
        crate::app::states::TrafficLogViewMode::TopUrls => "TOPURL",
        crate::app::states::TrafficLogViewMode::LogList => "로그목록",
    };
    let help_text = vec![
        format!("Tab: 탭전환 | ↑↓: 프록시/행 선택 | ←→: 뷰모드변경({}) | R: 로그조회", view_mode_text),
        format!("PageDown/Space: 다음페이지 | PageUp/b: 이전페이지 | +/-: 조회라인수 조정 (현재: {})", app.traffic_logs.log_limit),
    ];
    frame.render_widget(
        Paragraph::new(help_text.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("단축키"))
            .style(Style::default().fg(Color::Gray)),
        chunks[2],
    );
}

/// 로그 목록 뷰 렌더링
fn draw_log_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_char = spinner_chars[app.traffic_logs.spinner_frame % spinner_chars.len()];

    if app.traffic_logs.log_records.is_empty() {
        let empty_message = if app.traffic_logs.query_status == crate::app::CollectionStatus::Collecting
            || app.traffic_logs.query_status == crate::app::CollectionStatus::Starting {
            format!("{} 조회 중...", spinner_char)
        } else if app.traffic_logs.query_status == crate::app::CollectionStatus::Failed {
            if let Some(ref error) = app.traffic_logs.last_error {
                format!("조회 실패: {}", error)
            } else {
                "조회 실패".to_string()
            }
        } else {
            "데이터가 없습니다. [R] 키를 눌러 조회하세요.".to_string()
        };
        let table = Table::new(
            vec![Row::new(vec![Cell::from(empty_message)])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL).title("로그 목록"));
        frame.render_widget(table, area);
        return;
    }

    // 페이지네이션 적용
    let page_start = app.traffic_logs.current_page * app.traffic_logs.page_size;
    let page_end = (page_start + app.traffic_logs.page_size).min(app.traffic_logs.log_records.len());
    
    let paginated_records: Vec<&crate::traffic_log_parser::TrafficLogRecord> = 
        app.traffic_logs.log_records[page_start..page_end].iter().collect();

    // 테이블 생성
    let rows: Vec<Row> = paginated_records
        .iter()
        .enumerate()
        .map(|(i, record)| {
            let style = if app.traffic_logs.table_state.selected() == Some(i) {
                Style::default().bg(Color::Blue)
            } else {
                Style::default()
            };

            let datetime = record.datetime
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let client_ip = record.client_ip.as_ref()
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let username = record.username.as_ref()
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let url_host = record.url_host.as_ref()
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let status_code = record.response_statuscode
                .map(|c| c.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let recv_bytes = record.recv_byte
                .map(|b| format_bytes(b))
                .unwrap_or_else(|| "N/A".to_string());
            let sent_bytes = record.sent_byte
                .map(|b| format_bytes(b))
                .unwrap_or_else(|| "N/A".to_string());
            let action = record.action_names.as_ref()
                .map(|s| if s.len() > 15 { format!("{}...", &s[..12]) } else { s.clone() })
                .unwrap_or_else(|| "N/A".to_string());

            Row::new(vec![
                Cell::from(datetime).style(style),
                Cell::from(client_ip).style(style),
                Cell::from(username).style(style),
                Cell::from(url_host).style(style),
                Cell::from(status_code).style(style),
                Cell::from(recv_bytes).style(style),
                Cell::from(sent_bytes).style(style),
                Cell::from(action).style(style),
            ])
        })
        .collect();

    let title = format!(
        "로그 목록 (총 {}개, 페이지 {}/{})",
        app.traffic_logs.log_records.len(),
        app.traffic_logs.current_page + 1,
        app.traffic_logs.total_pages.max(1)
    );

    let table = Table::new(rows, [
        Constraint::Length(10),  // 시간
        Constraint::Length(16),  // 클라이언트IP
        Constraint::Length(12),  // 사용자
        Constraint::Min(20),     // 호스트
        Constraint::Length(8),   // 상태코드
        Constraint::Length(10),  // 수신
        Constraint::Length(10),  // 송신
        Constraint::Length(15),  // 액션
    ])
    .header(Row::new(vec![
        Cell::from("시간").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("클라이언트IP").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("사용자").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("호스트").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("상태").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("수신").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("송신").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("액션").style(Style::default().add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).title(title))
    .highlight_style(Style::default().bg(Color::Blue))
    .highlight_symbol(">> ");

    frame.render_stateful_widget(table, area, &mut app.traffic_logs.table_state);
}

fn draw_summary_table(_frame: &mut Frame, _area: Rect, analysis: &crate::traffic_log_parser::TopNAnalysis, _app: &App) -> Table<'static> {
    let rows = vec![
        Row::new(vec![
            Cell::from("총 레코드 수"),
            Cell::from(analysis.total_records.to_string()),
        ]),
        Row::new(vec![
            Cell::from("파싱 성공"),
            Cell::from(analysis.parsed_records.to_string()),
        ]),
        Row::new(vec![
            Cell::from("파싱 실패"),
            Cell::from(analysis.unparsed_records.to_string()),
        ]),
        Row::new(vec![
            Cell::from("고유 클라이언트 수"),
            Cell::from(analysis.unique_clients.to_string()),
        ]),
        Row::new(vec![
            Cell::from("고유 호스트 수"),
            Cell::from(analysis.unique_hosts.to_string()),
        ]),
        Row::new(vec![
            Cell::from("총 수신 바이트"),
            Cell::from(format_bytes(analysis.total_recv_bytes)),
        ]),
        Row::new(vec![
            Cell::from("총 송신 바이트"),
            Cell::from(format_bytes(analysis.total_sent_bytes)),
        ]),
        Row::new(vec![
            Cell::from("차단된 요청 수"),
            Cell::from(analysis.blocked_count.to_string()),
        ]),
    ];

    Table::new(rows, [Constraint::Min(20), Constraint::Min(15)])
        .header(Row::new(vec![
            Cell::from("항목").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("값").style(Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL).title("요약 정보"))
}

fn draw_top_clients_table(_frame: &mut Frame, _area: Rect, analysis: &crate::traffic_log_parser::TopNAnalysis) -> Table<'static> {
    let rows: Vec<Row> = analysis.top_clients.iter().enumerate().map(|(i, client)| {
        Row::new(vec![
            Cell::from((i + 1).to_string()),
            Cell::from(client.client_ip.clone()),
            Cell::from(client.request_count.to_string()),
            Cell::from(format_bytes(client.recv_bytes)),
            Cell::from(format_bytes(client.sent_bytes)),
        ])
    }).collect();

    Table::new(rows, [
        Constraint::Length(5),
        Constraint::Min(15),
        Constraint::Length(12),
        Constraint::Length(15),
        Constraint::Length(15),
    ])
    .header(Row::new(vec![
        Cell::from("순위").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("클라이언트 IP").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("요청 수").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("수신 바이트").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("송신 바이트").style(Style::default().add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("TOP 클라이언트"))
}

fn draw_top_hosts_table(_frame: &mut Frame, _area: Rect, analysis: &crate::traffic_log_parser::TopNAnalysis) -> Table<'static> {
    let rows: Vec<Row> = analysis.top_hosts.iter().enumerate().map(|(i, host)| {
        Row::new(vec![
            Cell::from((i + 1).to_string()),
            Cell::from(host.host.clone()),
            Cell::from(host.request_count.to_string()),
            Cell::from(format_bytes(host.recv_bytes)),
            Cell::from(format_bytes(host.sent_bytes)),
        ])
    }).collect();

    Table::new(rows, [
        Constraint::Length(5),
        Constraint::Min(20),
        Constraint::Length(12),
        Constraint::Length(15),
        Constraint::Length(15),
    ])
    .header(Row::new(vec![
        Cell::from("순위").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("호스트").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("요청 수").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("수신 바이트").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("송신 바이트").style(Style::default().add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("TOP 호스트"))
}

fn draw_top_urls_table(_frame: &mut Frame, _area: Rect, analysis: &crate::traffic_log_parser::TopNAnalysis) -> Table<'static> {
    let rows: Vec<Row> = analysis.top_urls.iter().enumerate().map(|(i, url)| {
        let url_display = if url.url.len() > 60 {
            format!("{}...", &url.url[..57])
        } else {
            url.url.clone()
        };
        Row::new(vec![
            Cell::from((i + 1).to_string()),
            Cell::from(url_display),
            Cell::from(url.request_count.to_string()),
        ])
    }).collect();

    Table::new(rows, [
        Constraint::Length(5),
        Constraint::Min(40),
        Constraint::Length(12),
    ])
    .header(Row::new(vec![
        Cell::from("순위").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("URL").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("요청 수").style(Style::default().add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("TOP URL"))
}

fn format_bytes(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
