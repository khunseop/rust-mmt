use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use std::collections::HashMap;

/// 컬럼 너비 자동 계산 (URL 제외)
fn calculate_column_widths(
    sessions: &[&crate::app::SessionData],
    available_width: usize,
    url_min_width: usize,
) -> Vec<Constraint> {
    if sessions.is_empty() {
        // 기본 너비 반환
        return vec![
            Constraint::Min(12), Constraint::Min(10), Constraint::Min(19),
            Constraint::Min(8), Constraint::Min(8), Constraint::Min(10),
            Constraint::Min(15), Constraint::Min(12), Constraint::Min(12),
            Constraint::Min(15), Constraint::Min(10), Constraint::Min(10),
            Constraint::Min(10), Constraint::Min(10), Constraint::Min(8),
            Constraint::Min(8), Constraint::Min(6), Constraint::Min(6),
            Constraint::Min(url_min_width.min(u16::MAX as usize) as u16),
        ];
    }

    let headers = vec![
        "호스트", "트랜잭션", "생성시간", "프로토콜", "CustID", "사용자",
        "클라이언트IP", "CL-MWG-IP", "SRV-MWG-IP", "서버IP",
        "CL수신", "CL송신", "SRV수신", "SRV송신", "TrxnIdx", "Age(초)",
        "상태", "InUse", "URL",
    ];

    let mut max_widths = vec![0; 19];

    // 헤더 길이 고려
    for (i, header) in headers.iter().enumerate() {
        max_widths[i] = header.len();
    }

    // 각 컬럼의 최대 내용 길이 계산
    for session in sessions {
        max_widths[0] = max_widths[0].max(session.host.len());
        max_widths[1] = max_widths[1].max(session.transaction.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[2] = max_widths[2].max(19); // 생성시간은 고정
        max_widths[3] = max_widths[3].max(session.protocol.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[4] = max_widths[4].max(session.cust_id.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[5] = max_widths[5].max(session.user_name.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[6] = max_widths[6].max(session.client_ip.len());
        max_widths[7] = max_widths[7].max(session.client_side_mwg_ip.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[8] = max_widths[8].max(session.server_side_mwg_ip.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[9] = max_widths[9].max(session.server_ip.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[10] = max_widths[10].max(session.cl_bytes_received.map(|v| format!("{}", v).len()).unwrap_or(0));
        max_widths[11] = max_widths[11].max(session.cl_bytes_sent.map(|v| format!("{}", v).len()).unwrap_or(0));
        max_widths[12] = max_widths[12].max(session.srv_bytes_received.map(|v| format!("{}", v).len()).unwrap_or(0));
        max_widths[13] = max_widths[13].max(session.srv_bytes_sent.map(|v| format!("{}", v).len()).unwrap_or(0));
        max_widths[14] = max_widths[14].max(session.trxn_index.map(|v| format!("{}", v).len()).unwrap_or(0));
        max_widths[15] = max_widths[15].max(session.age_seconds.map(|v| format!("{}", v).len()).unwrap_or(0));
        max_widths[16] = max_widths[16].max(session.status.as_ref().map(|s| s.len()).unwrap_or(0));
        max_widths[17] = max_widths[17].max(session.in_use.map(|v| format!("{}", v).len()).unwrap_or(0));
        // URL은 제외 (고정 너비 사용)
    }

    // URL을 제외한 총 너비 계산
    let url_index = 18;
    let _total_non_url_width: usize = max_widths.iter()
        .enumerate()
        .filter(|(i, _)| *i != url_index)
        .map(|(_, &w)| w.max(8) + 2) // 최소 8자, 패딩 포함
        .sum();

    let url_width = url_min_width;
    let _remaining_width = available_width.saturating_sub(url_width + 2); // 테두리 고려

    // Constraint 생성
    let mut constraints = Vec::new();
    for (i, &max_w) in max_widths.iter().enumerate() {
        if i == url_index {
            constraints.push(Constraint::Min(url_width.min(u16::MAX as usize) as u16));
        } else {
            let min_width = max_w.max(8);
            // 비율 계산 (간단하게 Min 사용, 필요시 Percentage 추가)
            constraints.push(Constraint::Min(min_width.min(u16::MAX as usize) as u16));
        }
    }

    constraints
}

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    // 상세보기 모달이 열려있으면 모달만 표시
    if app.session_browser.show_detail_modal {
        draw_detail_modal(frame, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 컨트롤 영역
            Constraint::Min(3),     // 데이터 테이블
            Constraint::Length(if app.session_browser.search_mode { 5 } else { 4 }),  // 키보드 단축키 도움말 + 검색 UI
        ])
        .split(area);

    // 컨트롤 영역
    let control_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(18), // 그룹선택
            Constraint::Length(18), // 상태
            Constraint::Length(20), // 마지막조회
            Constraint::Min(0),     // 페이지 정보
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

    // 페이지 정보
    let total_sessions = app.session_browser.sessions.len();
    let page_info = if total_sessions > 0 {
        format!("페이지 {}/{} (총 {}개)", 
            app.session_browser.current_page + 1,
            app.session_browser.total_pages.max(1),
            total_sessions)
    } else {
        "페이지 0/0 (0개)".to_string()
    };
    frame.render_widget(
        Paragraph::new(page_info.as_str())
            .block(Block::default().borders(Borders::ALL).title("페이지"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[3],
    );

    // 프록시 ID를 그룹으로 매핑하는 HashMap 생성
    let proxy_group_map: HashMap<u32, String> = app.proxies
        .iter()
        .map(|p| (p.id, p.group.clone()))
        .collect();

    // 필요한 값들을 먼저 가져오기
    let selected_group = app.session_browser.selected_group.clone();
    let search_query = app.session_browser.search_query.clone();
    let current_page = app.session_browser.current_page;
    let page_size = app.session_browser.page_size;

    // 필터링 및 페이지네이션된 세션 데이터를 클론하여 소유 (borrow 문제 해결)
    let (paginated_sessions, total_filtered): (Vec<crate::app::SessionData>, usize) = {
        let sessions_ref = &app.session_browser.sessions;

        // 선택된 그룹에 따라 세션 필터링
        let group_filtered: Vec<&crate::app::SessionData> = match &selected_group {
            None => {
                sessions_ref.iter().collect()
            }
            Some(selected_group) => {
                sessions_ref
                    .iter()
                    .filter(|session| {
                        proxy_group_map.get(&session.proxy_id)
                            .map(|group| group == selected_group)
                            .unwrap_or(false)
                    })
                    .collect()
            }
        };

        // 검색어로 필터링
        let filtered_sessions: Vec<&crate::app::SessionData> = if search_query.is_empty() {
            group_filtered
        } else {
            let query = search_query.to_lowercase();
            group_filtered.into_iter()
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
                .collect()
        };

        let total = filtered_sessions.len();
        let page_start = current_page * page_size;
        let page_end = (page_start + page_size).min(total);
        
        // 소유 데이터로 클론하여 borrow 종료
        let paginated: Vec<crate::app::SessionData> = if page_start < total {
            filtered_sessions[page_start..page_end].iter().map(|s| (*s).clone()).collect()
        } else {
            Vec::new()
        };
        
        (paginated, total)
    };
    
    // 페이지네이션 업데이트 (borrow 종료 후 가능)
    app.session_browser.update_total_pages(total_filtered);

    // 테이블 영역
    let table = if paginated_sessions.is_empty() {
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
        // 컬럼 너비 계산을 위해 참조 벡터 생성
        let paginated_refs: Vec<&crate::app::SessionData> = paginated_sessions.iter().collect();
        let table_width = chunks[1].width as usize;
        let constraints = calculate_column_widths(&paginated_refs, table_width, 30);

        let rows: Vec<Row> = paginated_sessions
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

                // 컬럼 순서 적용
                let ordered_cells: Vec<Cell> = app.session_browser.column_order.iter()
                    .filter_map(|&idx| all_cells.get(idx))
                    .cloned()
                    .collect();

                // 컬럼 오프셋에 따라 표시할 컬럼 선택
                let max_visible = 10;
                let start_idx = app.session_browser.column_offset.min(ordered_cells.len());
                let end_idx = (start_idx + max_visible).min(ordered_cells.len());
                let visible_cells = if start_idx < ordered_cells.len() {
                    ordered_cells[start_idx..end_idx].to_vec()
                } else {
                    vec![Cell::from("").style(style)]
                };

                Row::new(visible_cells)
            })
            .collect();

        // 컬럼 정의 (모든 컬럼 - 19개)
        let all_columns = vec![
            "호스트", "트랜잭션", "생성시간", "프로토콜", "CustID", "사용자",
            "클라이언트IP", "CL-MWG-IP", "SRV-MWG-IP", "서버IP",
            "CL수신", "CL송신", "SRV수신", "SRV송신", "TrxnIdx", "Age(초)",
            "상태", "InUse", "URL",
        ];

        // 컬럼 순서 적용
        let ordered_columns: Vec<&str> = app.session_browser.column_order.iter()
            .filter_map(|&idx| all_columns.get(idx))
            .copied()
            .collect();

        // 표시할 컬럼 선택 (최대 10개)
        let max_visible = 10;
        let start_idx = app.session_browser.column_offset.min(ordered_columns.len());
        let end_idx = (start_idx + max_visible).min(ordered_columns.len());
        let visible_columns = if start_idx < ordered_columns.len() {
            &ordered_columns[start_idx..end_idx]
        } else {
            &[]
        };

        // 컬럼 순서에 맞는 constraints 가져오기
        let ordered_constraints: Vec<Constraint> = app.session_browser.column_order.iter()
            .filter_map(|&idx| constraints.get(idx))
            .cloned()
            .collect();

        let visible_constraints: Vec<Constraint> = if start_idx < ordered_constraints.len() {
            ordered_constraints[start_idx..end_idx.min(ordered_constraints.len())].to_vec()
        } else {
            vec![Constraint::Percentage(100)]
        };

        // 헤더 생성 (정렬 표시 포함)
        let header_cells: Vec<Cell> = visible_columns.iter().enumerate().map(|(idx, name)| {
            // 실제 컬럼 인덱스 찾기
            let display_idx = start_idx + idx;
            let actual_col_idx = if display_idx < app.session_browser.column_order.len() {
                app.session_browser.column_order[display_idx]
            } else {
                display_idx
            };
            let mut header_text = (*name).to_string();
            
            // 정렬 표시 추가 (실제 컬럼 인덱스 사용)
            if app.session_browser.sort_column == Some(actual_col_idx) {
                if app.session_browser.sort_ascending {
                    header_text.push_str(" ↑");
                } else {
                    header_text.push_str(" ↓");
                }
            }
            
            // 컬럼 선택 모드일 때 하이라이트 (실제 컬럼 인덱스 사용)
            let style = if app.session_browser.selected_column == Some(actual_col_idx) {
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            
            Cell::from(header_text).style(style)
        }).collect();

        Table::new(rows, visible_constraints)
        .header(Row::new(header_cells))
        .block(Block::default().borders(Borders::ALL).title(format!(
            "세션 목록 (총 {}개, 페이지 {}/{})",
            total_filtered,
            app.session_browser.current_page + 1,
            app.session_browser.total_pages.max(1)
        )))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ")
    };

    frame.render_stateful_widget(table, chunks[1], &mut app.session_browser.table_state);

    // 키보드 단축키 도움말
    let total_columns = 19;
    let current_col = app.session_browser.column_offset + 1;
    let help_text = vec![
        format!("Tab: 탭전환 | q: 종료 | ↑↓: 행이동 | ←→: 컬럼선택/스크롤({}/{}) | Shift+←→: 그룹선택 | S: 세션조회 | Enter: 상세보기 | Shift+S: 정렬 | Esc: 컬럼선택해제", 
            current_col, total_columns),
        format!("PageDown/Space: 다음페이지 | PageUp/b: 이전페이지 | Home: 첫페이지 | End: 마지막페이지 | /: 검색 | Ctrl+←→: 컬럼순서변경"),
    ];
    
    if app.session_browser.search_mode {
        // 검색 모드일 때 검색 UI 표시
        let search_chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 도움말
                Constraint::Length(2),  // 검색 입력
            ])
            .split(chunks[2]);
        
        frame.render_widget(
            Paragraph::new(help_text.join("\n"))
                .block(Block::default().borders(Borders::ALL).title("단축키"))
                .style(Style::default().fg(Color::Gray)),
            search_chunks[0],
        );
        
        // 검색 입력 UI
        let search_display = format!("검색: {}", app.session_browser.search_query);
        frame.render_widget(
            Paragraph::new(search_display.as_str())
                .block(Block::default().borders(Borders::ALL).title("검색 [Esc: 종료]"))
                .style(Style::default().fg(Color::Yellow)),
            search_chunks[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(help_text.join("\n"))
                .block(Block::default().borders(Borders::ALL).title("단축키"))
                .style(Style::default().fg(Color::Gray)),
            chunks[2],
        );
    }
}

/// 상세보기 모달 렌더링
fn draw_detail_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(80, 60, area);
    
    let selected_idx = app.session_browser.table_state.selected();
    let filtered_sessions: Vec<&crate::app::SessionData> = match &app.session_browser.selected_group {
        None => app.session_browser.sessions.iter().collect(),
        Some(selected_group) => {
            let proxy_group_map: HashMap<u32, String> = app.proxies
                .iter()
                .map(|p| (p.id, p.group.clone()))
                .collect();
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

    // 페이지네이션 적용
    let page_start = app.session_browser.current_page * app.session_browser.page_size;
    let page_end = (page_start + app.session_browser.page_size).min(filtered_sessions.len());
    let paginated_sessions: Vec<&crate::app::SessionData> = if page_start < filtered_sessions.len() {
        filtered_sessions[page_start..page_end].iter().copied().collect()
    } else {
        Vec::new()
    };

    if let Some(idx) = selected_idx {
        if let Some(session) = paginated_sessions.get(idx) {
            let items: Vec<ListItem> = vec![
                ListItem::new(format!("호스트: {}", session.host)),
                ListItem::new(format!("트랜잭션: {}", session.transaction.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("생성시간: {}", session.creation_time
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("프로토콜: {}", session.protocol.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("CustID: {}", session.cust_id.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("사용자: {}", session.user_name.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("클라이언트IP: {}", session.client_ip)),
                ListItem::new(format!("CL-MWG-IP: {}", session.client_side_mwg_ip.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("SRV-MWG-IP: {}", session.server_side_mwg_ip.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("서버IP: {}", session.server_ip.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("CL수신: {}", session.cl_bytes_received.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("CL송신: {}", session.cl_bytes_sent.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("SRV수신: {}", session.srv_bytes_received.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("SRV송신: {}", session.srv_bytes_sent.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("TrxnIdx: {}", session.trxn_index.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("Age(초): {}", session.age_seconds.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("상태: {}", session.status.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("InUse: {}", session.in_use.map(|v| format!("{}", v)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("URL: {}", session.url.as_ref().unwrap_or(&"N/A".to_string()))),
            ];

            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("세션 상세 정보 [Esc: 닫기]"))
                .style(Style::default().fg(Color::White));

            frame.render_widget(list, popup_area);
        }
    }
}

/// 중앙에 위치한 사각형 계산
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
