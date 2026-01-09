mod config;
mod proxy_management;
mod resource_usage;
mod session_browser;
mod traffic_logs;

use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Tabs},
    Frame,
};

use crate::app::{App, TabIndex};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = ratatui::layout::Layout::default()
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
        TabIndex::ProxyManagement => proxy_management::draw(frame, app, chunks[1]),
        TabIndex::ResourceUsage => resource_usage::draw(frame, app, chunks[1]),
        TabIndex::SessionBrowser => session_browser::draw(frame, app, chunks[1]),
        TabIndex::TrafficLogs => traffic_logs::draw(frame, app, chunks[1]),
    }
}
