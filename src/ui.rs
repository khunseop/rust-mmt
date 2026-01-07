use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, Tabs},
    Frame,
};

use crate::app::{App, TabIndex};

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

fn draw_resource_usage(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(3),  // 헤더
            Constraint::Length(5),  // 설정 및 정보
            Constraint::Min(0),     // 데이터 테이블
        ])
        .split(area);

    // 헤더 영역
    let header = Block::default()
        .borders(Borders::ALL)
        .title("자원 사용률 모니터링");
    frame.render_widget(header, chunks[0]);

    // 설정 및 정보 영역
    let info_chunks = Layout::default()
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // 그룹 선택
    let group_text = if app.proxies.is_empty() {
        "프록시가 설정되지 않았습니다.".to_string()
    } else {
        format!("그룹: [{}]\n(Shift+←/→: 변경)", app.resource_usage.get_group_display_name())
    };
    
    let group_block = Block::default()
        .borders(Borders::ALL)
        .title("필터");
    
    use ratatui::widgets::Paragraph;
    frame.render_widget(
        Paragraph::new(group_text)
            .block(group_block)
            .style(Style::default().fg(Color::Cyan)),
        info_chunks[0],
    );

    // 수집 주기 및 마지막 수집 시간
    let last_collection_str = match app.resource_usage.last_collection_time {
        Some(time) => format!("{}", time.format("%Y-%m-%d %H:%M:%S")),
        None => "수집 이력 없음".to_string(),
    };
    
    // 스피너 문자 (회전 애니메이션)
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[app.resource_usage.spinner_frame % spinner_chars.len()];
    
    // 수집 상태 메시지
    let (status_text, status_style) = match app.resource_usage.collection_status {
        crate::app::CollectionStatus::Idle => {
            ("대기 중".to_string(), Style::default().fg(Color::Gray))
        }
        crate::app::CollectionStatus::Starting => {
            (format!("{} 수집 시작 중...", spinner), Style::default().fg(Color::Yellow))
        }
        crate::app::CollectionStatus::Collecting => {
            let progress_text = if let Some((completed, total)) = app.resource_usage.collection_progress {
                format!("{} 수집 중... ({}/{})", spinner, completed, total)
            } else {
                format!("{} 수집 중...", spinner)
            };
            (progress_text, Style::default().fg(Color::Yellow))
        }
        crate::app::CollectionStatus::Success => {
            let success_text = if let Some((completed, _total)) = app.resource_usage.collection_progress {
                format!("✅ 수집 완료! ({}개 성공)", completed)
            } else {
                "✅ 수집 완료!".to_string()
            };
            (success_text, Style::default().fg(Color::Green))
        }
        crate::app::CollectionStatus::Failed => {
            let error_text = if let Some(ref error) = app.resource_usage.last_error {
                format!("❌ 수집 실패: {}", error)
            } else {
                "❌ 수집 실패".to_string()
            };
            (error_text, Style::default().fg(Color::Red))
        }
    };
    
    let interval_text = format!(
        "수집 주기: [{}]\n마지막 수집: {}\n상태: {}\n(+/-: 주기 변경, C: 수집)",
        app.resource_usage.get_interval_display(),
        last_collection_str,
        status_text
    );
    
    let interval_block = Block::default()
        .borders(Borders::ALL)
        .title("설정");
    
    let interval_style = status_style;
    
    frame.render_widget(
        Paragraph::new(interval_text)
            .block(interval_block)
            .style(interval_style),
        info_chunks[1],
    );

    // 테이블 영역
    let table = if app.resource_usage.data.is_empty() {
        // 데이터가 없을 때 빈 테이블
        Table::new(
            vec![Row::new(vec![
                Cell::from("데이터가 없습니다. [C] 키를 눌러 수집하세요."),
            ])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL))
    } else {
        // 데이터가 있을 때 실제 테이블
        let rows: Vec<Row> = app
            .resource_usage
            .data
            .iter()
            .enumerate()
            .map(|(i, data)| {
                let cpu_str = data
                    .cpu
                    .map(|v| format!("{:.1}%", v))
                    .unwrap_or_else(|| "N/A".to_string());
                let mem_str = data
                    .mem
                    .map(|v| format!("{:.1}%", v))
                    .unwrap_or_else(|| "N/A".to_string());
                let time_str = data.collected_at.format("%H:%M:%S").to_string();

                let style = if app.resource_usage.table_state.selected() == Some(i) {
                    Style::default().bg(Color::Blue)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(data.host.clone()).style(style),
                    Cell::from(cpu_str).style(style),
                    Cell::from(mem_str).style(style),
                    Cell::from(time_str).style(style),
                ])
            })
            .collect();

        Table::new(rows, [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ])
        .header(Row::new(vec![
            Cell::from("호스트").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CPU").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("MEM").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("시간").style(Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL).title("자원 사용률 데이터"))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ")
    };

    frame.render_stateful_widget(table, chunks[2], &mut app.resource_usage.table_state);
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

