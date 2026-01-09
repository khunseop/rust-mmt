use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use std::collections::HashMap;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 컨트롤 영역 (그룹선택)
            Constraint::Min(3),     // 데이터 테이블
            Constraint::Length(4),  // 키보드 단축키 도움말
        ])
        .split(area);

    // 컨트롤 영역
    let control_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(18), // 그룹선택
            Constraint::Length(18), // 상태
            Constraint::Length(20), // 마지막조회
            Constraint::Min(0),     // 나머지
        ])
        .split(chunks[0]);
    
    // 그룹선택
    let group_name = app.session_browser.get_group_display_name();
    frame.render_widget(
        Paragraph::new(group_name.as_str())
            .block(Block::default().borders(Borders::ALL).title("그룹선택"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[0],
    );

    // 상태 (스피너 포함)
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_char = spinner_chars[app.session_browser.spinner_frame % spinner_chars.len()];
    
    let (status_text, status_color, elapsed_sec) = match app.session_browser.query_status {
        crate::app::CollectionStatus::Idle => ("대기중".to_string(), Color::Gray, None),
        crate::app::CollectionStatus::Starting => {
            (format!("{} 시작중", spinner_char), Color::Yellow, None)
        }
        crate::app::CollectionStatus::Collecting => {
            let elapsed = app.session_browser.query_start_time
                .map(|start| (chrono::Local::now() - start).num_seconds());
            let progress_text = if let Some((completed, total)) = app.session_browser.query_progress {
                format!("{} 조회중 ({}/{})", spinner_char, completed, total)
            } else {
                format!("{} 조회중", spinner_char)
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

    // 마지막 조회 시간
    let last_query_text = if let Some(last_time) = app.session_browser.last_query_time {
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

    // 프록시 ID를 그룹으로 매핑하는 HashMap 생성
    let proxy_group_map: HashMap<u32, String> = app.proxies
        .iter()
        .map(|p| (p.id, p.group.clone()))
        .collect();

    // 선택된 그룹에 따라 세션 필터링
    let filtered_sessions: Vec<&crate::app::SessionData> = match &app.session_browser.selected_group {
        None => {
            // 전체보기
            app.session_browser.sessions.iter().collect()
        }
        Some(selected_group) => {
            // 선택된 그룹의 프록시들만 필터링
            app.session_browser.sessions
                .iter()
                .filter(|session| {
                    proxy_group_map.get(&session.proxy_id)
                        .map(|group| group == selected_group)
                        .unwrap_or(false)
                })
                .collect()
        }
    };

    // 테이블 영역
    let table = if filtered_sessions.is_empty() {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner_char = spinner_chars[app.session_browser.spinner_frame % spinner_chars.len()];
        
        let empty_message = if app.session_browser.query_status == crate::app::CollectionStatus::Collecting
            || app.session_browser.query_status == crate::app::CollectionStatus::Starting {
            format!("{} 조회 중...", spinner_char)
        } else if app.session_browser.query_status == crate::app::CollectionStatus::Failed {
            if let Some(ref error) = app.session_browser.last_error {
                format!("조회 실패: {}", error)
            } else {
                "조회 실패".to_string()
            }
        } else {
            "데이터가 없습니다. [S] 키를 눌러 조회하세요.".to_string()
        };
        Table::new(
            vec![Row::new(vec![
                Cell::from(empty_message),
            ])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL).title("세션 목록"))
    } else {
        let rows: Vec<Row> = filtered_sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let style = if app.session_browser.table_state.selected() == Some(i) {
                    Style::default().bg(Color::Blue)
                } else {
                    Style::default()
                };

                // 모든 필드 준비
                let transaction = session.transaction.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let creation_time = session.creation_time
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                let protocol = session.protocol.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let cust_id = session.cust_id.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let user_name = session.user_name.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let client_ip = session.client_ip.clone();
                let client_side_mwg_ip = session.client_side_mwg_ip.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let server_side_mwg_ip = session.server_side_mwg_ip.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let server_ip = session.server_ip.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let cl_bytes_received = session.cl_bytes_received.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string());
                let cl_bytes_sent = session.cl_bytes_sent.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string());
                let srv_bytes_received = session.srv_bytes_received.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string());
                let srv_bytes_sent = session.srv_bytes_sent.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string());
                let trxn_index = session.trxn_index.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string());
                let age_seconds = session.age_seconds.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string());
                let status = session.status.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let in_use = session.in_use.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string());
                let url_display = session.url.as_ref().map(|s| {
                    if s.len() > 50 {
                        format!("{}...", &s[..47])
                    } else {
                        s.clone()
                    }
                }).unwrap_or_else(|| "N/A".to_string());

                // 모든 컬럼 데이터 (19개 컬럼)
                let all_cells = vec![
                    Cell::from(session.host.clone()).style(style),
                    Cell::from(transaction).style(style),
                    Cell::from(creation_time.clone()).style(style),
                    Cell::from(protocol).style(style),
                    Cell::from(cust_id).style(style),
                    Cell::from(user_name).style(style),
                    Cell::from(client_ip.clone()).style(style),
                    Cell::from(client_side_mwg_ip).style(style),
                    Cell::from(server_side_mwg_ip).style(style),
                    Cell::from(server_ip).style(style),
                    Cell::from(cl_bytes_received.clone()).style(style),
                    Cell::from(cl_bytes_sent.clone()).style(style),
                    Cell::from(srv_bytes_received.clone()).style(style),
                    Cell::from(srv_bytes_sent.clone()).style(style),
                    Cell::from(trxn_index.clone()).style(style),
                    Cell::from(age_seconds.clone()).style(style),
                    Cell::from(status).style(style),
                    Cell::from(in_use.clone()).style(style),
                    Cell::from(url_display).style(style),
                ];

                // 컬럼 오프셋에 따라 표시할 컬럼 선택
                let max_visible = 10;
                let start_idx = app.session_browser.column_offset.min(all_cells.len());
                let end_idx = (start_idx + max_visible).min(all_cells.len());
                let visible_cells = if start_idx < all_cells.len() {
                    all_cells[start_idx..end_idx].to_vec()
                } else {
                    vec![Cell::from("").style(style)]
                };

                Row::new(visible_cells)
            })
            .collect();

        // 컬럼 정의 (모든 컬럼 - 19개)
        let all_columns = vec![
            ("호스트", Constraint::Length(15)),
            ("트랜잭션", Constraint::Length(12)),
            ("생성시간", Constraint::Length(19)),
            ("프로토콜", Constraint::Length(10)),
            ("CustID", Constraint::Length(10)),
            ("사용자", Constraint::Length(12)),
            ("클라이언트IP", Constraint::Length(15)),
            ("CL-MWG-IP", Constraint::Length(15)),
            ("SRV-MWG-IP", Constraint::Length(15)),
            ("서버IP", Constraint::Length(15)),
            ("CL수신", Constraint::Length(12)),
            ("CL송신", Constraint::Length(12)),
            ("SRV수신", Constraint::Length(12)),
            ("SRV송신", Constraint::Length(12)),
            ("TrxnIdx", Constraint::Length(10)),
            ("Age(초)", Constraint::Length(10)),
            ("상태", Constraint::Length(8)),
            ("InUse", Constraint::Length(8)),
            ("URL", Constraint::Min(30)),
        ];

        // 표시할 컬럼 선택 (최대 10개)
        let max_visible = 10;
        let start_idx = app.session_browser.column_offset.min(all_columns.len());
        let end_idx = (start_idx + max_visible).min(all_columns.len());
        let visible_columns = if start_idx < all_columns.len() {
            &all_columns[start_idx..end_idx]
        } else {
            &[]
        };

        let constraints: Vec<Constraint> = visible_columns.iter().map(|(_, c)| *c).collect();
        let header_cells: Vec<Cell> = visible_columns.iter().map(|(name, _)| {
            Cell::from(*name).style(Style::default().add_modifier(Modifier::BOLD))
        }).collect();

        Table::new(rows, constraints)
        .header(Row::new(header_cells))
        .block(Block::default().borders(Borders::ALL).title(format!(
            "세션 목록 (총 {}개)",
            filtered_sessions.len()
        )))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ")
    };

    frame.render_stateful_widget(table, chunks[1], &mut app.session_browser.table_state);

    // 키보드 단축키 도움말
    let total_columns = 19;
    let current_col = app.session_browser.column_offset + 1;
    let help_text = vec![
        format!("Tab: 탭전환 | q/Esc: 종료 | ↑↓: 행이동 | ←→: 컬럼스크롤({}/{}) | Shift+←→: 그룹선택 | S: 세션조회", 
            current_col, total_columns),
    ];
    frame.render_widget(
        Paragraph::new(help_text.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("단축키"))
            .style(Style::default().fg(Color::Gray)),
        chunks[2],
    );
}
