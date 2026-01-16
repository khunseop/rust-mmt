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
            Constraint::Length(20), // 마지막분석
            Constraint::Min(0),     // 나머지
        ])
        .split(chunks[0]);

    // 프록시 선택
    let proxy_text = if let Some(proxy_id) = app.traffic_logs.selected_proxy {
        if let Some(proxy) = app.proxies.iter().find(|p| p.id == proxy_id as u32) {
            format!("{}\n{}", proxy.host, proxy.group)
        } else {
            "선택 안됨".to_string()
        }
    } else {
        "선택 안됨".to_string()
    };
    frame.render_widget(
        Paragraph::new(proxy_text.as_str())
            .block(Block::default().borders(Borders::ALL).title("프록시"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[0],
    );

    // 상태 (스피너 포함)
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_char = spinner_chars[app.traffic_logs.spinner_frame % spinner_chars.len()];
    
    let (status_text, status_color, elapsed_sec) = match app.traffic_logs.analysis_status {
        crate::app::CollectionStatus::Idle => ("대기중".to_string(), Color::Gray, None),
        crate::app::CollectionStatus::Starting => {
            (format!("{} 시작중", spinner_char), Color::Yellow, None)
        }
        crate::app::CollectionStatus::Collecting => {
            let elapsed = app.traffic_logs.analysis_start_time
                .map(|start| (chrono::Local::now() - start).num_seconds());
            let progress_text = if let Some((completed, total)) = app.traffic_logs.analysis_progress {
                format!("{} 분석중 ({}/{})", spinner_char, completed, total)
            } else {
                format!("{} 분석중", spinner_char)
            };
            (progress_text, Color::Yellow, elapsed)
        }
        crate::app::CollectionStatus::Success => ("✓ 완료".to_string(), Color::Green, None),
        crate::app::CollectionStatus::Failed => ("✗ 실패".to_string(), Color::Red, None),
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

    // 마지막 분석 시간
    let last_analysis_text = if let Some(last_time) = app.traffic_logs.last_analysis_time {
        format!("{}\n{}", 
            last_time.format("%H:%M:%S"),
            last_time.format("%m/%d"))
    } else {
        "없음".to_string()
    };
    frame.render_widget(
        Paragraph::new(last_analysis_text.as_str())
            .block(Block::default().borders(Borders::ALL).title("마지막분석"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[2],
    );

    // 테이블 영역
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
        }
    } else {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner_char = spinner_chars[app.traffic_logs.spinner_frame % spinner_chars.len()];
        
        let empty_message = if app.traffic_logs.analysis_status == crate::app::CollectionStatus::Collecting
            || app.traffic_logs.analysis_status == crate::app::CollectionStatus::Starting {
            format!("{} 분석 중...", spinner_char)
        } else if app.traffic_logs.analysis_status == crate::app::CollectionStatus::Failed {
            if let Some(ref error) = app.traffic_logs.last_error {
                format!("분석 실패: {}", error)
            } else {
                "분석 실패".to_string()
            }
        } else {
            "데이터가 없습니다. 프록시를 선택하고 [A] 키를 눌러 분석하세요.".to_string()
        };
        Table::new(
            vec![Row::new(vec![
                Cell::from(empty_message),
            ])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL).title("트래픽 로그 분석"))
    };

    frame.render_widget(table, chunks[1]);

    // 키보드 단축키 도움말
    let view_mode_text = match app.traffic_logs.view_mode {
        crate::app::states::TrafficLogViewMode::Summary => "요약",
        crate::app::states::TrafficLogViewMode::TopClients => "TOP클라이언트",
        crate::app::states::TrafficLogViewMode::TopHosts => "TOP호스트",
        crate::app::states::TrafficLogViewMode::TopUrls => "TOPURL",
    };
    let help_text = vec![
        format!("Tab: 탭전환 | q/Esc: 종료 | ↑↓: 행이동 | ←→: 뷰모드변경({}) | A: 분석시작", view_mode_text),
        format!("프록시 선택: 숫자키(1-9) 또는 ↑↓로 선택 후 Enter"),
    ];
    frame.render_widget(
        Paragraph::new(help_text.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("단축키"))
            .style(Style::default().fg(Color::Gray)),
        chunks[2],
    );
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
