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
    let tabs = Tabs::new(vec!["자원사용률", "세션브라우저", "트래픽로그"])
        .block(Block::default().borders(Borders::ALL).title(app.title.clone()))
        .select(match app.current_tab {
            TabIndex::ResourceUsage => 0,
            TabIndex::SessionBrowser => 1,
            TabIndex::TrafficLogs => 2,
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
        TabIndex::ResourceUsage => draw_resource_usage(frame, app, chunks[1]),
        TabIndex::SessionBrowser => draw_session_browser(frame, app, chunks[1]),
        TabIndex::TrafficLogs => draw_traffic_logs(frame, app, chunks[1]),
    }
}

fn draw_resource_usage(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(3),  // 헤더
            Constraint::Length(8),  // 프록시 리스트
            Constraint::Min(0),     // 데이터 테이블
        ])
        .split(area);

    // 헤더 영역
    let header = Block::default()
        .borders(Borders::ALL)
        .title("자원 사용률 모니터링");
    frame.render_widget(header, chunks[0]);

    // 프록시 리스트 영역
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

