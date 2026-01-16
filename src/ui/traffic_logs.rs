use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    // 상세보기 모달이 열려있으면 모달만 표시
    if app.traffic_logs.show_detail_modal {
        draw_detail_modal(frame, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 컨트롤 영역
            Constraint::Min(3),     // 데이터 테이블
            Constraint::Length(if app.traffic_logs.search_mode { 5 } else { 4 }),  // 키보드 단축키 도움말 + 검색 UI
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
            .block(Block::default().borders(Borders::ALL).title("프록시(Shift+↑↓)"))
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

    // 필터링된 로그 수 계산
    let filtered_count = get_filtered_records(app).len();
    
    // 페이지 정보
    let page_info = if filtered_count > 0 {
        format!("페이지 {}/{} ({}개)", 
            app.traffic_logs.current_page + 1,
            ((filtered_count + app.traffic_logs.page_size - 1) / app.traffic_logs.page_size).max(1),
            filtered_count)
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
                    "데이터가 없습니다. [R] 키를 눌러 조회하세요.".to_string()
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
    
    let total_columns = 15;
    let current_col = app.traffic_logs.column_offset + 1;
    let help_text = if app.traffic_logs.view_mode == crate::app::states::TrafficLogViewMode::LogList {
        vec![
            format!("Tab: 탭전환 | ↑↓: 행이동 | ←→: 컬럼스크롤({}/{}) | Shift+↑↓: 프록시선택 | R: 조회 | Enter: 상세보기", 
                current_col, total_columns),
            format!("PageDown/Space: 다음페이지 | PageUp/b: 이전페이지 | /: 검색 | +/-: 조회라인수 ({})", app.traffic_logs.log_limit),
        ]
    } else {
        vec![
            format!("Tab: 탭전환 | ←→: 뷰모드변경({}) | Shift+↑↓: 프록시선택 | R: 로그조회", view_mode_text),
            format!("+/-: 조회라인수 조정 (현재: {})", app.traffic_logs.log_limit),
        ]
    };

    if app.traffic_logs.search_mode {
        // 검색 모드일 때 검색 UI 표시
        let search_chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 도움말
                Constraint::Length(3),  // 검색 입력
            ])
            .split(chunks[2]);
        
        frame.render_widget(
            Paragraph::new(help_text.join("\n"))
                .block(Block::default().borders(Borders::ALL).title("단축키"))
                .style(Style::default().fg(Color::Gray)),
            search_chunks[0],
        );
        
        // 검색 입력 UI
        let cursor = "█";
        let search_display = format!("검색어: {}{}", app.traffic_logs.search_query, cursor);
        let result_info = if app.traffic_logs.search_query.is_empty() {
            format!("전체 {}건 | Enter: 완료 | Esc: 취소", filtered_count)
        } else {
            format!("검색결과 {}건 | Enter: 완료(유지) | Esc: 취소(초기화)", filtered_count)
        };
        frame.render_widget(
            Paragraph::new(format!("{}\n{}", search_display, result_info))
                .block(Block::default().borders(Borders::ALL).title("검색 모드"))
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

/// 검색 필터가 적용된 레코드 반환
fn get_filtered_records(app: &App) -> Vec<&crate::traffic_log_parser::TrafficLogRecord> {
    if app.traffic_logs.search_query.is_empty() {
        app.traffic_logs.log_records.iter().collect()
    } else {
        let query = app.traffic_logs.search_query.to_lowercase();
        app.traffic_logs.log_records.iter()
            .filter(|record| {
                record.client_ip.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                record.username.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                record.url_host.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                record.url_path.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                record.action_names.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                record.response_statuscode.map(|c| c.to_string().contains(&query)).unwrap_or(false) ||
                record.url_categories.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false) ||
                record.url_destination_ip.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
            })
            .collect()
    }
}

/// 로그 목록 뷰 렌더링
fn draw_log_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_char = spinner_chars[app.traffic_logs.spinner_frame % spinner_chars.len()];

    // 검색 필터 적용
    let filtered_records = get_filtered_records(app);
    
    if filtered_records.is_empty() {
        let empty_message = if app.traffic_logs.query_status == crate::app::CollectionStatus::Collecting
            || app.traffic_logs.query_status == crate::app::CollectionStatus::Starting {
            format!("{} 조회 중...", spinner_char)
        } else if app.traffic_logs.query_status == crate::app::CollectionStatus::Failed {
            if let Some(ref error) = app.traffic_logs.last_error {
                format!("조회 실패: {}", error)
            } else {
                "조회 실패".to_string()
            }
        } else if !app.traffic_logs.search_query.is_empty() {
            format!("검색 결과가 없습니다: \"{}\"", app.traffic_logs.search_query)
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
    let total_filtered = filtered_records.len();
    let page_start = app.traffic_logs.current_page * app.traffic_logs.page_size;
    let page_end = (page_start + app.traffic_logs.page_size).min(total_filtered);
    
    let paginated_records: Vec<&crate::traffic_log_parser::TrafficLogRecord> = 
        if page_start < total_filtered {
            filtered_records[page_start..page_end].to_vec()
        } else {
            Vec::new()
        };

    // 모든 컬럼 정의 (15개)
    let all_columns = vec![
        "시간", "클라이언트IP", "사용자", "호스트", "경로", "상태", 
        "수신", "송신", "프로토콜", "카테고리", "액션", "목적지IP",
        "UserAgent", "Referer", "트랜잭션시간",
    ];

    // 표시할 컬럼 선택 (최대 8개)
    let max_visible = 8;
    let start_idx = app.traffic_logs.column_offset.min(all_columns.len().saturating_sub(1));
    let end_idx = (start_idx + max_visible).min(all_columns.len());
    let visible_columns = &all_columns[start_idx..end_idx];

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

            // 모든 컬럼 데이터 준비
            let all_cells = vec![
                record.datetime
                    .map(|dt| dt.format("%H:%M:%S").to_string())
                    .unwrap_or_else(|| "-".to_string()),
                record.client_ip.clone().unwrap_or_else(|| "-".to_string()),
                record.username.clone().unwrap_or_else(|| "-".to_string()),
                record.url_host.clone().unwrap_or_else(|| "-".to_string()),
                record.url_path.as_ref()
                    .map(|s| if s.len() > 30 { format!("{}...", &s[..27]) } else { s.clone() })
                    .unwrap_or_else(|| "-".to_string()),
                record.response_statuscode
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                record.recv_byte
                    .map(|b| format_bytes(b))
                    .unwrap_or_else(|| "-".to_string()),
                record.sent_byte
                    .map(|b| format_bytes(b))
                    .unwrap_or_else(|| "-".to_string()),
                record.url_protocol.clone().unwrap_or_else(|| "-".to_string()),
                record.url_categories.as_ref()
                    .map(|s| if s.len() > 15 { format!("{}...", &s[..12]) } else { s.clone() })
                    .unwrap_or_else(|| "-".to_string()),
                record.action_names.as_ref()
                    .map(|s| if s.len() > 15 { format!("{}...", &s[..12]) } else { s.clone() })
                    .unwrap_or_else(|| "-".to_string()),
                record.url_destination_ip.clone().unwrap_or_else(|| "-".to_string()),
                record.user_agent.as_ref()
                    .map(|s| if s.len() > 20 { format!("{}...", &s[..17]) } else { s.clone() })
                    .unwrap_or_else(|| "-".to_string()),
                record.referer.as_ref()
                    .map(|s| if s.len() > 20 { format!("{}...", &s[..17]) } else { s.clone() })
                    .unwrap_or_else(|| "-".to_string()),
                record.timeintransaction
                    .map(|t| format!("{:.2}s", t))
                    .unwrap_or_else(|| "-".to_string()),
            ];

            // 현재 보이는 컬럼만 선택
            let visible_cells: Vec<Cell> = all_cells[start_idx..end_idx.min(all_cells.len())]
                .iter()
                .map(|s| Cell::from(s.clone()).style(style))
                .collect();

            Row::new(visible_cells)
        })
        .collect();

    let title = if app.traffic_logs.search_query.is_empty() {
        format!(
            "로그 목록 (총 {}개, 페이지 {}/{})",
            total_filtered,
            app.traffic_logs.current_page + 1,
            ((total_filtered + app.traffic_logs.page_size - 1) / app.traffic_logs.page_size).max(1)
        )
    } else {
        format!(
            "로그 목록 [검색: \"{}\"] ({}개, 페이지 {}/{})",
            app.traffic_logs.search_query,
            total_filtered,
            app.traffic_logs.current_page + 1,
            ((total_filtered + app.traffic_logs.page_size - 1) / app.traffic_logs.page_size).max(1)
        )
    };

    // 컬럼 너비 설정
    let column_widths = vec![
        Constraint::Length(10),  // 시간
        Constraint::Length(16),  // 클라이언트IP
        Constraint::Length(12),  // 사용자
        Constraint::Min(15),     // 호스트
        Constraint::Min(20),     // 경로
        Constraint::Length(6),   // 상태
        Constraint::Length(10),  // 수신
        Constraint::Length(10),  // 송신
        Constraint::Length(8),   // 프로토콜
        Constraint::Length(15),  // 카테고리
        Constraint::Length(15),  // 액션
        Constraint::Length(16),  // 목적지IP
        Constraint::Length(20),  // UserAgent
        Constraint::Length(20),  // Referer
        Constraint::Length(10),  // 트랜잭션시간
    ];
    let visible_widths: Vec<Constraint> = column_widths[start_idx..end_idx.min(column_widths.len())].to_vec();

    // 헤더 생성
    let header_cells: Vec<Cell> = visible_columns.iter()
        .map(|name| Cell::from(*name).style(Style::default().add_modifier(Modifier::BOLD)))
        .collect();

    let table = Table::new(rows, visible_widths)
        .header(Row::new(header_cells))
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ");

    frame.render_stateful_widget(table, area, &mut app.traffic_logs.table_state);
}

/// 상세보기 모달 렌더링
fn draw_detail_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(80, 70, area);
    
    let selected_idx = app.traffic_logs.table_state.selected();
    let filtered_records = get_filtered_records(app);
    
    // 페이지네이션 적용
    let page_start = app.traffic_logs.current_page * app.traffic_logs.page_size;
    let page_end = (page_start + app.traffic_logs.page_size).min(filtered_records.len());
    let paginated_records: Vec<&crate::traffic_log_parser::TrafficLogRecord> = 
        if page_start < filtered_records.len() {
            filtered_records[page_start..page_end].to_vec()
        } else {
            Vec::new()
        };

    if let Some(idx) = selected_idx {
        if let Some(record) = paginated_records.get(idx) {
            let items: Vec<ListItem> = vec![
                ListItem::new(format!("시간: {}", record.datetime
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("사용자: {}", record.username.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("클라이언트IP: {}", record.client_ip.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("목적지IP: {}", record.url_destination_ip.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("트랜잭션시간: {}", record.timeintransaction.map(|t| format!("{:.3}초", t)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("응답코드: {}", record.response_statuscode.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("캐시상태: {}", record.cache_status.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("프로토콜: {}", record.url_protocol.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("호스트: {}", record.url_host.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("경로: {}", record.url_path.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("포트: {}", record.url_port.map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("카테고리: {}", record.url_categories.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("평판: {}", record.url_reputationstring.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("미디어타입: {}", record.mediatype_header.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("수신바이트: {}", record.recv_byte.map(|b| format_bytes(b)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("송신바이트: {}", record.sent_byte.map(|b| format_bytes(b)).unwrap_or_else(|| "N/A".to_string()))),
                ListItem::new(format!("UserAgent: {}", record.user_agent.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("Referer: {}", record.referer.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("지역: {}", record.url_geolocation.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("애플리케이션: {}", record.application_name.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("룰셋: {}", record.currentruleset.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("룰: {}", record.currentrule.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("액션: {}", record.action_names.as_ref().unwrap_or(&"N/A".to_string()))),
                ListItem::new(format!("블록ID: {}", record.block_id.as_ref().unwrap_or(&"N/A".to_string()))),
            ];

            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("트래픽 로그 상세 정보 [Esc: 닫기]"))
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
