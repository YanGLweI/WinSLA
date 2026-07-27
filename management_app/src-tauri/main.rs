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

/// Show a Windows error message box (visible to user even without console)
fn show_error(title: &str, message: &str) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("cmd.exe")
        .args(["/C", &format!("echo {} & pause", message)])
        .creation_flags(0x0000_0010) // CREATE_NEW_CONSOLE
        .spawn();
    // Also try MessageBoxW via windows API
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let title_wide: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
        let msg_wide: Vec<u16> = OsStr::new(message).encode_wide().chain(Some(0)).collect();
        MessageBoxW(std::ptr::null_mut(), msg_wide.as_ptr(), title_wide.as_ptr(), 0x10); // MB_ICONERROR
    }
}

#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, utype: u32) -> i32;
}

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
    let db = match database::Database::open(db_path.to_str().unwrap_or("winsla.db")) {
        Ok(db) => db,
        Err(e) => {
            let msg = format!("Failed to open database at {}\n\nError: {}\n\nPlease check file permissions and disk space.", db_path.display(), e);
            show_error("WinSLA Management - Database Error", &msg);
            std::process::exit(1);
        }
    };
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
