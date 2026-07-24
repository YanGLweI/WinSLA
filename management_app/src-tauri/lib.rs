//! # WinSLA Management Library
//!
//! Core logic for the WinSLA management application.
//! Provides dual-account pairing configuration, service control,
//! and audit log querying.

pub mod commands;
pub mod database;

/// Application configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub service_name: String,
    pub pipe_path: String,
    pub db_path: String,
    pub max_retry_count: u32,
    pub auth_timeout_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            service_name: "WinSLA Service".to_string(),
            pipe_path: r"\\.\pipe\winsla-auth-pipe".to_string(),
            db_path: "winsla.db".to_string(),
            max_retry_count: 3,
            auth_timeout_secs: 30,
        }
    }
}

/// Run the management application (interactive console)
pub fn run_app() {
    use std::io::{self, Write};

    let config = AppConfig::default();
    let db = match database::Database::open(&config.db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[ERROR] Failed to open database: {}", e);
            eprintln!("Press Enter to exit...");
            let _ = io::stdin().read_line(&mut String::new());
            return;
        }
    };

    loop {
        println!();
        println!("========================================");
        println!("  WinSLA Management Console v{}", env!("CARGO_PKG_VERSION"));
        println!("========================================");
        println!();
        println!("  1. Service status");
        println!("  2. List dual-account pairs");
        println!("  3. Add dual-account pair");
        println!("  4. Remove dual-account pair");
        println!("  5. List emergency accounts");
        println!("  6. Add emergency account");
        println!("  7. View audit log (last 20)");
        println!("  8. View/edit policy config");
        println!("  0. Exit");
        println!();
        print!("  Select [0-8]: ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        match input.trim() {
            "1" => cmd_service_status(),
            "2" => cmd_list_pairs(&db),
            "3" => cmd_add_pair(&db),
            "4" => cmd_remove_pair(&db),
            "5" => cmd_list_emergency(&db),
            "6" => cmd_add_emergency(&db),
            "7" => cmd_audit_log(&db),
            "8" => cmd_policy_config(&db),
            "0" | "q" | "Q" => {
                println!("  Bye.");
                break;
            }
            _ => println!("  Invalid option."),
        }
    }
}

fn cmd_service_status() {
    match commands::get_service_status() {
        Ok(status) => {
            println!();
            println!("  Service Status:");
            println!("    Running:    {}", if status.running { "YES" } else { "NO" });
            println!("    Version:    {}", status.version);
            println!("    Connections: {}", status.connections_accepted);
            println!("    Auth OK:    {}", status.successful_auths);
            println!("    Auth Fail:  {}", status.failed_auths);
        }
        Err(e) => println!("  [ERROR] {}", e),
    }
}

fn cmd_list_pairs(db: &database::Database) {
    match db.get_all_dual_pairs() {
        Ok(pairs) => {
            println!();
            if pairs.is_empty() {
                println!("  No dual-account pairs configured.");
            } else {
                println!("  {:<36} {:<20} {:<20} {}", "ID", "User A", "User B", "Enabled");
                println!("  {}", "-".repeat(90));
                for p in &pairs {
                    println!("  {:<36} {:<20} {:<20} {}", p.id, p.user_a_name, p.user_b_name, p.enabled);
                }
            }
        }
        Err(e) => println!("  [ERROR] {}", e),
    }
}

fn cmd_add_pair(db: &database::Database) {
    use std::io::{self, Write};
    print!("  User A name: ");
    io::stdout().flush().ok();
    let mut a = String::new();
    io::stdin().read_line(&mut a).ok();

    print!("  User B name: ");
    io::stdout().flush().ok();
    let mut b = String::new();
    io::stdin().read_line(&mut b).ok();

    let name_a = a.trim().to_string();
    let name_b = b.trim().to_string();

    match db.add_dual_pair("", "", &name_a, &name_b) {
        Ok(_) => println!("  Pair added: {} <-> {}", name_a, name_b),
        Err(e) => println!("  [ERROR] {}", e),
    }
}

fn cmd_remove_pair(db: &database::Database) {
    use std::io::{self, Write};
    print!("  Pair ID to remove: ");
    io::stdout().flush().ok();
    let mut id = String::new();
    io::stdin().read_line(&mut id).ok();

    match db.remove_dual_pair(id.trim()) {
        Ok(_) => println!("  Pair removed."),
        Err(e) => println!("  [ERROR] {}", e),
    }
}

fn cmd_list_emergency(db: &database::Database) {
    match db.get_emergency_accounts() {
        Ok(accounts) => {
            println!();
            if accounts.is_empty() {
                println!("  No emergency accounts configured.");
            } else {
                println!("  {:<36} {:<20} {:<20} {}", "SID", "Username", "Reason", "Approved By");
                println!("  {}", "-".repeat(90));
                for a in &accounts {
                    println!("  {:<36} {:<20} {:<20} {}", a.sid, a.username, a.reason, a.approved_by);
                }
            }
        }
        Err(e) => println!("  [ERROR] {}", e),
    }
}

fn cmd_add_emergency(db: &database::Database) {
    use std::io::{self, Write};
    print!("  Username: ");
    io::stdout().flush().ok();
    let mut name = String::new();
    io::stdin().read_line(&mut name).ok();

    print!("  Account SID: ");
    io::stdout().flush().ok();
    let mut sid = String::new();
    io::stdin().read_line(&mut sid).ok();

    print!("  Reason: ");
    io::stdout().flush().ok();
    let mut reason = String::new();
    io::stdin().read_line(&mut reason).ok();

    match db.add_emergency_account(sid.trim(), name.trim(), reason.trim(), "admin") {
        Ok(_) => println!("  Emergency account added: {}", name.trim()),
        Err(e) => println!("  [ERROR] {}", e),
    }
}

fn cmd_audit_log(db: &database::Database) {
    match db.get_audit_log(20) {
        Ok(entries) => {
            println!();
            if entries.is_empty() {
                println!("  No audit log entries.");
            } else {
                println!("  {:<22} {:<20} {:<20} {:<10} {}", "Timestamp", "User A SID", "User B SID", "Result", "Error");
                println!("  {}", "-".repeat(95));
                for e in &entries {
                    println!(
                        "  {:<22} {:<20} {:<20} {:<10} {}",
                        e.timestamp, e.user_a_sid, e.user_b_sid, e.result,
                        e.error_message.as_deref().unwrap_or("")
                    );
                }
            }
        }
        Err(e) => println!("  [ERROR] {}", e),
    }
}

fn cmd_policy_config(db: &database::Database) {
    match db.get_policy() {
        Ok(config) => {
            println!();
            println!("  Policy Configuration:");
            println!("    Max retry count:        {}", config.max_retry_count);
            println!("    Auth timeout (secs):    {}", config.auth_timeout_secs);
            println!("    Emergency override:     {}", config.allow_emergency_override);
            println!("    Emergency req. reason:  {}", config.emergency_requires_reason);
            println!("    Offline cache:          {}", config.offline_cache_enabled);
        }
        Err(e) => println!("  [ERROR] {}", e),
    }
}
