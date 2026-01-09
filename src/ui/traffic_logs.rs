use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
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
        Paragraph::new(content).block(block),
        chunks[1],
    );
}
