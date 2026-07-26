//! WinSLA Management Application
//!
//! Launches a local HTTP server + WebView2 native window.
#![windows_subsystem = "windows"]

use std::sync::{Arc, Mutex};

mod commands;
mod database;
mod frontend;
mod gui;
mod server;
mod wincred;

const PORT: u16 = 19830;

/// Get the app data directory (writable location for WebView2 data)
fn app_data_dir() -> std::path::PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let dir = base.join("WinSLA");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Get the shared database directory (accessible by both service and management app)
fn shared_db_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(r"C:\ProgramData\WinSLA");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn main() {
    env_logger::init();

    let data_dir = app_data_dir();

    // Open database in shared location (accessible by win_service running as SYSTEM)
    let db_dir = shared_db_dir();
    let db_path = db_dir.join("winsla.db");
    let db = database::Database::open(db_path.to_str().unwrap_or("winsla.db"))
        .expect("Failed to open database");
    let state = Arc::new(Mutex::new(db));

    // Start axum server in a background tokio runtime
    let server_state = state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            if let Err(e) = server::start_server(server_state, PORT).await {
                log::error!("Server error: {}", e);
            }
        });
    });

    // Wait for server to be ready
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", PORT)).is_ok() {
            break;
        }
    }

    // Launch WebView2 window (blocks until window closed)
    gui::launch_gui(PORT, &data_dir);
}
