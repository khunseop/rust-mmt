use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::ui::config::{get_interface_names, load_thresholds, ThresholdConfig};
use std::collections::HashMap;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 컨트롤 영역 (한 줄)
            Constraint::Min(3),     // 데이터 테이블
            Constraint::Length(4),  // 키보드 단축키 도움말 (컴팩트)
        ])
        .split(area);

    // 컨트롤 영역을 한 줄로 구성 (선택 불가능)
    let control_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(18), // 그룹선택
            Constraint::Length(18), // 자동수집
            Constraint::Length(18), // 수집주기
            Constraint::Length(18), // 상태
            Constraint::Length(20), // 마지막수집
            Constraint::Min(0),     // 나머지
        ])
        .split(chunks[0]);
    
    // 컨트롤 렌더링 헬퍼 함수 (선택 불가능, 정보 표시만)
    fn render_info_box(frame: &mut Frame, title: &str, content: &str, style: Style, area: Rect) {
        frame.render_widget(
            Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title(title))
                .style(style),
            area,
        );
    }
    
    // 그룹선택
    let group_name = app.resource_usage.get_group_display_name();
    render_info_box(frame, "그룹선택", &group_name, Style::default().fg(Color::Cyan), control_chunks[0]);
    
    // 자동수집
    let auto_status = if app.resource_usage.auto_collection_enabled {
        if let Some(next_time) = app.resource_usage.next_auto_collection_time {
            let remaining = (next_time - chrono::Local::now()).num_seconds();
            if remaining > 0 {
                format!("🔄 ON ({}초 후)", remaining)
            } else {
                "🔄 ON".to_string()
            }
        } else {
            "🔄 ON".to_string()
        }
    } else {
        "▶ OFF".to_string()
    };
    let auto_style = if app.resource_usage.auto_collection_enabled {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };
    render_info_box(frame, "자동수집", &auto_status, auto_style, control_chunks[1]);
    
    // 수집주기
    let interval = app.resource_usage.get_interval_display();
    render_info_box(frame, "수집주기", &interval, Style::default().fg(Color::White), control_chunks[2]);
    
    // 상태
    let (status_text, status_color, elapsed_sec) = match app.resource_usage.collection_status {
        crate::app::CollectionStatus::Idle => ("대기중".to_string(), Color::Gray, None),
        crate::app::CollectionStatus::Starting => ("시작중".to_string(), Color::Yellow, None),
        crate::app::CollectionStatus::Collecting => {
            let elapsed = app.resource_usage.collection_start_time
                .map(|start| (chrono::Local::now() - start).num_seconds());
            if let Some((completed, total)) = app.resource_usage.collection_progress {
                (format!("수집중 ({}/{})", completed, total), Color::Yellow, elapsed)
            } else {
                ("수집중".to_string(), Color::Yellow, elapsed)
            }
        }
        crate::app::CollectionStatus::Success => ("완료".to_string(), Color::Green, None),
        crate::app::CollectionStatus::Failed => ("실패".to_string(), Color::Red, None),
    };
    let status_display = if let Some(elapsed) = elapsed_sec {
        format!("{}\n{}초", status_text, elapsed)
    } else {
        status_text
    };
    render_info_box(frame, "상태", &status_display, Style::default().fg(status_color), control_chunks[3]);

    // 마지막 수집 시간
    let last_collection_text = if let Some(last_time) = app.resource_usage.last_collection_time {
        format!("{}\n{}", 
            last_time.format("%H:%M:%S"),
            last_time.format("%m/%d"))
    } else {
        "없음".to_string()
    };
    render_info_box(frame, "마지막수집", &last_collection_text, Style::default().fg(Color::Cyan), control_chunks[4]);

    // 회선 목록 가져오기
    let interface_names = get_interface_names();
    
    // 임계치 설정 로드
    let thresholds = load_thresholds();
    
    // 테이블 영역 - Python 앱과 동일한 구조
    let table = if app.resource_usage.data.is_empty() {
        // 데이터가 없을 때 빈 테이블
        Table::new(
            vec![Row::new(vec![
                Cell::from("데이터가 없습니다. Space를 눌러 수집하세요."),
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
                    let _error_msg = data.error_message.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("실패");
                    
                    let style = if app.resource_usage.table_state.selected() == Some(i) {
                        Style::default().bg(Color::Red).fg(Color::White)
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    // 기본 컬럼 + 회선 컬럼들 (컴팩트하게)
                    let proxy_display_name = data.proxy_name.as_ref().unwrap_or(&data.host);
                    let mut cells = vec![
                        Cell::from(proxy_display_name.clone()).style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                        Cell::from("-").style(style),
                    ];
                    
                    // 각 회선에 대해 빈 셀 추가
                    for _ in &interface_names {
                        cells.push(Cell::from("-").style(style));
                    }
                    
                    cells.push(Cell::from("✗").style(style));
                    Row::new(cells)
                } else {
                    // 성공한 경우
                    let format_percent = |v: Option<f64>| -> String {
                        v.map(|val| format!("{:.0}%", val))
                            .unwrap_or_else(|| "-".to_string())
                    };

                    // 개수 표시 (CC, CS)
                    let format_count = |v: Option<f64>| -> String {
                        v.map(|val| {
                            let count = val as u64;
                            if count >= 1_000_000 {
                                format!("{:.1}M", count as f64 / 1_000_000.0)
                            } else if count >= 1_000 {
                                format!("{:.1}K", count as f64 / 1_000.0)
                            } else {
                                format!("{}", count)
                            }
                        })
                        .unwrap_or_else(|| "-".to_string())
                    };

                    // bps를 컴팩트한 형식으로 표시 (K/M/G 단위)
                    let format_bps = |v: Option<f64>| -> String {
                        v.map(|bps| {
                            if bps >= 1_000_000_000.0 {
                                format!("{:.1}G", bps / 1_000_000_000.0)
                            } else if bps >= 1_000_000.0 {
                                format!("{:.1}M", bps / 1_000_000.0)
                            } else if bps >= 1_000.0 {
                                format!("{:.1}K", bps / 1_000.0)
                            } else {
                                format!("{:.0}", bps)
                            }
                        })
                        .unwrap_or_else(|| "-".to_string())
                    };

                    let cpu_str = format_percent(data.cpu);
                    let mem_str = format_percent(data.mem);
                    let cc_str = format_count(data.cc);
                    let cs_str = format_count(data.cs);
                    let http_str = format_bps(data.http);
                    let https_str = format_bps(data.https);
                    let ftp_str = format_bps(data.ftp);
                    
                    // 임계치에 따른 색상 결정 함수
                    fn get_threshold_color(value: Option<f64>, threshold: &ThresholdConfig) -> Color {
                        match value {
                            Some(v) if v >= threshold.critical => Color::Red,
                            Some(v) if v >= threshold.warning => Color::Yellow,
                            _ => Color::White,
                        }
                    }
                    
                    // 회선 정보를 HashMap으로 변환 (빠른 조회를 위해)
                    let interface_map: HashMap<String, (f64, f64)> = data.interfaces
                        .iter()
                        .map(|iface| (iface.name.clone(), (iface.in_mbps, iface.out_mbps)))
                        .collect();

                    let base_style = if app.resource_usage.table_state.selected() == Some(i) {
                        Style::default().bg(Color::Blue)
                    } else {
                        Style::default()
                    };

                    // 기본 컬럼들 - 각 셀에 임계치 색상 적용
                    let proxy_display_name = data.proxy_name.as_ref().unwrap_or(&data.host);
                    let cpu_color = thresholds.get("cpu").map(|t| get_threshold_color(data.cpu, t)).unwrap_or(Color::White);
                    let mem_color = thresholds.get("mem").map(|t| get_threshold_color(data.mem, t)).unwrap_or(Color::White);
                    let cc_color = thresholds.get("cc").map(|t| get_threshold_color(data.cc, t)).unwrap_or(Color::White);
                    let cs_color = thresholds.get("cs").map(|t| get_threshold_color(data.cs, t)).unwrap_or(Color::White);
                    let http_color = thresholds.get("http").map(|t| get_threshold_color(data.http, t)).unwrap_or(Color::White);
                    let https_color = thresholds.get("https").map(|t| get_threshold_color(data.https, t)).unwrap_or(Color::White);
                    let ftp_color = thresholds.get("ftp").map(|t| get_threshold_color(data.ftp, t)).unwrap_or(Color::White);
                    
                    let mut cells = vec![
                        Cell::from(proxy_display_name.clone()).style(base_style),
                        Cell::from(cpu_str).style(base_style.fg(cpu_color)),
                        Cell::from(mem_str).style(base_style.fg(mem_color)),
                        Cell::from(cc_str).style(base_style.fg(cc_color)),
                        Cell::from(cs_str).style(base_style.fg(cs_color)),
                        Cell::from(http_str).style(base_style.fg(http_color)),
                        Cell::from(https_str).style(base_style.fg(https_color)),
                        Cell::from(ftp_str).style(base_style.fg(ftp_color)),
                    ];
                    
                    // 각 회선에 대해 별도 컬럼 추가 (bps를 컴팩트하게 표시)
                    let interface_threshold = thresholds.get("interface_traffic");
                    for if_name in &interface_names {
                        if let Some((in_bps, out_bps)) = interface_map.get(if_name) {
                            let in_str = if *in_bps >= 1_000_000_000.0 {
                                format!("{:.1}G", in_bps / 1_000_000_000.0)
                            } else if *in_bps >= 1_000_000.0 {
                                format!("{:.1}M", in_bps / 1_000_000.0)
                            } else if *in_bps >= 1_000.0 {
                                format!("{:.1}K", in_bps / 1_000.0)
                            } else {
                                format!("{:.0}", in_bps)
                            };
                            let out_str = if *out_bps >= 1_000_000_000.0 {
                                format!("{:.1}G", out_bps / 1_000_000_000.0)
                            } else if *out_bps >= 1_000_000.0 {
                                format!("{:.1}M", out_bps / 1_000_000.0)
                            } else if *out_bps >= 1_000.0 {
                                format!("{:.1}K", out_bps / 1_000.0)
                            } else {
                                format!("{:.0}", out_bps)
                            };
                            
                            // 인터페이스 트래픽 색상 결정 (in/out 중 더 높은 값 기준)
                            let max_traffic = in_bps.max(*out_bps);
                            let traffic_color = interface_threshold
                                .map(|t| get_threshold_color(Some(max_traffic), t))
                                .unwrap_or(Color::White);
                            
                            cells.push(Cell::from(format!("{}/{}", in_str, out_str)).style(base_style.fg(traffic_color)));
                        } else {
                            cells.push(Cell::from("-").style(base_style));
                        }
                    }
                    
                    // 상태 컬럼
                    cells.push(Cell::from("✓").style(base_style));
                    
                    Row::new(cells)
                }
            })
            .collect();

        // 컬럼 너비 설정 (컴팩트하게)
        let mut constraints = vec![
            Constraint::Length(12),  // 프록시
            Constraint::Length(5),   // CPU
            Constraint::Length(5),   // MEM
            Constraint::Length(5),   // CC
            Constraint::Length(5),   // CS
            Constraint::Length(6),   // HTTP (bps)
            Constraint::Length(6),   // HTTPS (bps)
            Constraint::Length(6),   // FTP (bps)
        ];
        
        // 각 회선에 대해 컬럼 추가 (너비 증가로 잘림 방지)
        for _ in &interface_names {
            constraints.push(Constraint::Length(12)); // 각 회선 컬럼 (in/out bps)
        }
        
        constraints.push(Constraint::Length(3)); // 상태 컬럼
        
        // 헤더 생성
        let mut header_cells = vec![
            Cell::from("프록시").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CPU%").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("MEM%").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CC").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("CS").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("HTTP").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("HTTPS").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("FTP").style(Style::default().add_modifier(Modifier::BOLD)),
        ];
        
        // 각 회선에 대해 헤더 추가 (컴팩트하게)
        for if_name in &interface_names {
            // 인터페이스 이름을 컴팩트하게 표시
            let short_name = if if_name.len() > 5 {
                if let Some(last_char) = if_name.chars().last() {
                    if last_char.is_ascii_digit() {
                        let prefix_len = (if_name.len() - 1).min(4);
                        format!("{}{}", &if_name[..prefix_len], last_char)
                    } else {
                        format!("{}", &if_name[..5])
                    }
                } else {
                    format!("{}", &if_name[..5])
                }
            } else {
                if_name.clone()
            };
            header_cells.push(Cell::from(short_name).style(Style::default().add_modifier(Modifier::BOLD)));
        }
        
        header_cells.push(Cell::from("✓").style(Style::default().add_modifier(Modifier::BOLD)));
        
        Table::new(rows, constraints)
        .header(Row::new(header_cells))
        .block(Block::default().borders(Borders::ALL).title("자원 사용률 모니터링"))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ")
    };

    frame.render_stateful_widget(table, chunks[1], &mut app.resource_usage.table_state);

    // 키보드 단축키 도움말 (컴팩트)
    let help_text = vec![
        "Tab: 탭전환 | ↑↓: 테이블이동 | Space: 자동수집토글 | +/-: 주기 | Shift+←→: 그룹",
    ];
    frame.render_widget(
        Paragraph::new(help_text.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("단축키"))
            .style(Style::default().fg(Color::Gray)),
        chunks[2],
    );
}
