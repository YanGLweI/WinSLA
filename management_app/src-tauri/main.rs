//! WinSLA Management Application
//!
//! Launches a local HTTP server + WebView2 native window.

use std::sync::{Arc, Mutex};

mod commands;
mod database;
mod frontend;
mod gui;
mod server;

const PORT: u16 = 19830;

fn main() {
    env_logger::init();

    // Open database
    let db_path = "winsla.db";
    let db = database::Database::open(db_path).expect("Failed to open database");
    let state = Arc::new(Mutex::new(db));

    // Start axum server in a background tokio runtime
    let server_state = state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            if let Err(e) = server::start_server(server_state, PORT).await {
                log::error!("Server error: {}", e);
                eprintln!("Server error: {}", e);
            }
        });
    });

    // Give server a moment to start
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Launch WebView2 window (blocks until window closed)
    println!("WinSLA Management starting on http://127.0.0.1:{}", PORT);
    gui::launch_gui(PORT);
}
