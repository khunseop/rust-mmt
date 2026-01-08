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
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 컨트롤 영역
            Constraint::Min(3),     // 데이터 테이블
        ])
        .split(area);

    // 컨트롤 영역
    let control_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // 그룹
            Constraint::Length(25), // 자동수집 버튼
            Constraint::Length(20), // 수집 주기
            Constraint::Length(20), // 상태
            Constraint::Min(0),     // 나머지
        ])
        .split(chunks[0]);
    
    use ratatui::widgets::Paragraph;
    
    // 그룹 선택
    let group_name = app.resource_usage.get_group_display_name();
    let group_text = format!("그룹: {}\nShift+←/→", group_name);
    frame.render_widget(
        Paragraph::new(group_text)
            .block(Block::default().borders(Borders::ALL).title("필터"))
            .style(Style::default().fg(Color::Cyan)),
        control_chunks[0],
    );
    
    // 자동수집 버튼
    let auto_status = if app.resource_usage.auto_collection_enabled {
        if let Some(next_time) = app.resource_usage.next_auto_collection_time {
            let remaining = (next_time - chrono::Local::now()).num_seconds();
            if remaining > 0 {
                format!("🔄 ON ({}초 후)\nSpace: 중지", remaining)
            } else {
                "🔄 ON\nSpace: 중지".to_string()
            }
        } else {
            "🔄 ON\nSpace: 중지".to_string()
        }
    } else {
        "▶ OFF\nSpace: 시작".to_string()
    };
    
    let auto_style = if app.resource_usage.auto_collection_enabled {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };
    
    frame.render_widget(
        Paragraph::new(auto_status)
            .block(Block::default().borders(Borders::ALL).title("자동수집"))
            .style(auto_style),
        control_chunks[1],
    );
    
    // 수집 주기
    let interval = app.resource_usage.get_interval_display();
    let interval_text = format!("주기: {}\n+/-: 변경", interval);
    frame.render_widget(
        Paragraph::new(interval_text)
            .block(Block::default().borders(Borders::ALL).title("수집주기"))
            .style(Style::default().fg(Color::White)),
        control_chunks[2],
    );
    
    // 상태
    let (status_text, status_color) = match app.resource_usage.collection_status {
        crate::app::CollectionStatus::Idle => ("대기중".to_string(), Color::Gray),
        crate::app::CollectionStatus::Starting => ("시작중".to_string(), Color::Yellow),
        crate::app::CollectionStatus::Collecting => {
            if let Some((completed, total)) = app.resource_usage.collection_progress {
                (format!("수집중 ({}/{})", completed, total), Color::Yellow)
            } else {
                ("수집중".to_string(), Color::Yellow)
            }
        }
        crate::app::CollectionStatus::Success => ("완료".to_string(), Color::Green),
        crate::app::CollectionStatus::Failed => ("실패".to_string(), Color::Red),
    };
    
    let status_display = format!("{}\nC: 즉시수집", status_text);
    frame.render_widget(
        Paragraph::new(status_display)
            .block(Block::default().borders(Borders::ALL).title("상태"))
            .style(Style::default().fg(status_color)),
        control_chunks[3],
    );

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
                    let error_msg = data.error_message.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("실패");
                    
                    let style = if app.resource_usage.table_state.selected() == Some(i) {
                        Style::default().bg(Color::Red).fg(Color::White)
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    Row::new(vec![
                        Cell::from(data.host.clone()).style(style),
                        Cell::from("실패").style(style),
                        Cell::from("실패").style(style),
                        Cell::from("실패").style(style),
                        Cell::from("실패").style(style),
                        Cell::from("실패").style(style),
                        Cell::from("실패").style(style),
                        Cell::from("실패").style(style),
                        Cell::from(error_msg).style(style),
                    ])
                } else {
                    // 성공한 경우
                    let format_value = |v: Option<f64>| -> String {
                        v.map(|val| format!("{:.1}", val))
                            .unwrap_or_else(|| "N/A".to_string())
                    };

                    let cpu_str = format_value(data.cpu);
                    let mem_str = format_value(data.mem);
                    let cc_str = format_value(data.cc);
                    let cs_str = format_value(data.cs);
                    let http_str = format_value(data.http);
                    let https_str = format_value(data.https);
                    let ftp_str = format_value(data.ftp);
                    
                    // 회선 정보 (인터페이스)
                    let interface_str = if data.interfaces.is_empty() {
                        "N/A".to_string()
                    } else {
                        data.interfaces.iter()
                            .map(|iface| format!("{}: {:.2}/{:.2}", iface.name, iface.in_mbps, iface.out_mbps))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    let style = if app.resource_usage.table_state.selected() == Some(i) {
                        Style::default().bg(Color::Blue)
                    } else {
                        Style::default()
                    };

                    Row::new(vec![
                        Cell::from(data.host.clone()).style(style),
                        Cell::from(cpu_str).style(style),
                        Cell::from(mem_str).style(style),
                        Cell::from(cc_str).style(style),
                        Cell::from(cs_str).style(style),
                        Cell::from(http_str).style(style),
                        Cell::from(https_str).style(style),
                        Cell::from(ftp_str).style(style),
                        Cell::from(interface_str).style(style),
                    ])
                }
            })
            .collect();

        // 컬럼 너비 설정 (프록시, CPU, MEM, CC, CS, HTTP, HTTPS, FTP, 회선)
        Table::new(rows, [
            Constraint::Length(15),  // 프록시
            Constraint::Length(8),   // CPU
            Constraint::Length(8),   // MEM
            Constraint::Length(8),   // CC
            Constraint::Length(8),   // CS
            Constraint::Length(10),  // HTTP
            Constraint::Length(10),  // HTTPS
            Constraint::Length(10),  // FTP
            Constraint::Min(0),      // 회선 정보 (나머지 공간)
        ])
        .header(Row::new(vec![
            Cell::from("프록시").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CPU").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("MEM").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CC").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CS").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("HTTP").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("HTTPS").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("FTP").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("회선정보").style(Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL).title("자원 사용률 모니터링"))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ")
    };

    frame.render_stateful_widget(table, chunks[1], &mut app.resource_usage.table_state);
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

