use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    // 통계 정보 영역, 테이블 영역, 단축키 영역으로 분할
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 통계 정보 영역
            Constraint::Min(0),     // 프록시 목록 테이블
            Constraint::Length(3),  // 단축키 도움말
        ])
        .split(area);

    // 통계 정보 영역
    let stats_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // 전체 프록시 수
            Constraint::Length(20), // 그룹 수
            Constraint::Min(0),     // 나머지
        ])
        .split(chunks[0]);
    
    // 전체 프록시 수
    let total_count = app.proxies.len();
    frame.render_widget(
        Paragraph::new(format!("전체 프록시: {}개", total_count))
            .block(Block::default().borders(Borders::ALL).title("통계"))
            .style(Style::default().fg(Color::Cyan)),
        stats_chunks[0],
    );

    // 그룹 수
    use std::collections::HashSet;
    let group_count: HashSet<String> = app.proxies.iter().map(|p| p.group.clone()).collect();
    frame.render_widget(
        Paragraph::new(format!("그룹 수: {}개", group_count.len()))
            .block(Block::default().borders(Borders::ALL).title("그룹"))
            .style(Style::default().fg(Color::Green)),
        stats_chunks[1],
    );

    // 프록시 목록 테이블
    let proxy_table = if app.proxies.is_empty() {
        Table::new(
            vec![Row::new(vec![Cell::from("프록시가 설정되지 않았습니다. config/proxies.json을 확인하세요.")])],
            [Constraint::Percentage(100)],
        )
        .block(Block::default().borders(Borders::ALL).title("프록시 목록"))
    } else {
        // 그룹별로 프록시 그룹화 및 정렬
        use std::collections::HashMap;
        let mut groups: HashMap<String, Vec<&crate::app::Proxy>> = HashMap::new();
        for proxy in &app.proxies {
            groups.entry(proxy.group.clone()).or_insert_with(Vec::new).push(proxy);
        }

        // 그룹명으로 정렬
        let mut sorted_groups: Vec<_> = groups.iter().collect();
        sorted_groups.sort_by_key(|(group, _)| *group);

        let mut rows = Vec::new();
        for (group, proxies) in sorted_groups {
            // 그룹별로 프록시 ID로 정렬
            let mut sorted_proxies = proxies.clone();
            sorted_proxies.sort_by_key(|p| p.id);

            // 그룹 헤더 행
            rows.push(Row::new(vec![
                Cell::from(format!("📁 {}", group))
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Cell::from("")
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Cell::from("")
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Cell::from("")
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Cell::from(format!("({}개)", sorted_proxies.len()))
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));

            // 그룹 내 프록시들
            for proxy in sorted_proxies {
                let alias_display = proxy.alias.as_ref()
                    .map(|a| a.as_str())
                    .unwrap_or("-");
                
                let host_port = format!("{}:{}", proxy.host, proxy.port);
                
                let log_path_display = proxy.traffic_log_path.as_ref()
                    .map(|p| {
                        // 경로가 너무 길면 마지막 부분만 표시
                        if p.len() > 30 {
                            format!("...{}", &p[p.len().saturating_sub(27)..])
                        } else {
                            p.clone()
                        }
                    })
                    .unwrap_or_else(|| "-".to_string());

                rows.push(Row::new(vec![
                    Cell::from(format!("  ├─ ID: {}", proxy.id))
                        .style(Style::default().fg(Color::Gray)),
                    Cell::from(alias_display)
                        .style(Style::default().fg(Color::White)),
                    Cell::from(host_port)
                        .style(Style::default().fg(Color::Cyan)),
                    Cell::from(proxy.username.clone())
                        .style(Style::default().fg(Color::White)),
                    Cell::from(log_path_display)
                        .style(Style::default().fg(Color::Gray)),
                ]));
            }
        }

        Table::new(rows, [
            Constraint::Length(12),  // ID
            Constraint::Length(20),  // 별칭
            Constraint::Length(22),  // 호스트:포트
            Constraint::Length(15), // 사용자
            Constraint::Min(0),      // 로그 경로 (나머지 공간)
        ])
        .header(Row::new(vec![
            Cell::from("ID/그룹").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("별칭").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("호스트:포트").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("사용자").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("로그 경로").style(Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL).title(format!("프록시 목록 (총 {}개)", app.proxies.len())))
    };
    frame.render_widget(proxy_table, chunks[1]);

    // 키보드 단축키 도움말
    let help_text = "Tab: 탭전환 | 1~4: 탭선택";
    frame.render_widget(
        Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("단축키"))
            .style(Style::default().fg(Color::Gray)),
        chunks[2],
    );
}
