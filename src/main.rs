mod app;
mod crossterm;
mod ui;
mod snmp;
mod ssh;
mod collector;
mod csv_writer;
mod session_collector;

use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tick_rate = Duration::from_millis(50); // 250ms -> 50ms로 변경하여 더 빠른 반응
    crossterm::run(tick_rate)?;
    Ok(())
}
