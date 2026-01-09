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
    let tick_rate = Duration::from_millis(250);
    crossterm::run(tick_rate)?;
    Ok(())
}
