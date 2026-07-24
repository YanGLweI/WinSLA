//! # WinSLA Windows Service
//!
//! Core Windows Service that handles dual-account authentication via Named Pipe.

mod auth;
mod pipe_server;
mod audit;
mod com_types;

use std::sync::{Arc, Mutex};

/// Service name configuration
const SERVICE_NAME: &str = "WinSLA Service";

/// Service runtime state tracker
#[derive(Debug, Clone)]
pub struct ServiceState {
    pub running: bool,
    pub connections_accepted: u64,
    pub successful_authentications: u64,
    pub failed_authentications: u64,
}

impl ServiceState {
    pub fn new() -> Self {
        Self {
            running: true,
            connections_accepted: 0,
            successful_authentications: 0,
            failed_authentications: 0,
        }
    }

    pub fn update_activity(&mut self) {
        // Could track last activity timestamp here
    }

    pub fn record_success(&mut self) {
        self.successful_authentications += 1;
    }

    pub fn record_failure(&mut self) {
        self.failed_authentications += 1;
    }
}

impl Default for ServiceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Main entry point for the Windows Service
fn main() {
    env_logger::init();
    log::info!("Starting WinSLA Windows Service...");

    // When running as a Windows Service, use the service dispatcher.
    // When running from command line (for testing), run the pipe server directly.
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--service" {
        run_as_service();
    } else {
        log::info!("Running in standalone mode (not as Windows Service)");
        run_standalone();
    }
}

/// Run as a Windows Service using windows-service crate
fn run_as_service() {
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    define_windows_service!(ffi_service_main, service_main);

    fn service_main(_arguments: Vec<std::ffi::OsString>) {
        if let Err(e) = run_service_inner() {
            log::error!("Service error: {}", e);
        }
    }

    fn run_service_inner() -> Result<(), windows_service::Error> {
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop => {
                    log::info!("Service received stop command");
                    let _ = shutdown_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        let status_handle =
            service_control_handler::register(SERVICE_NAME, event_handler)?;

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: windows_service::service::ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })?;

        log::info!("Service started successfully");

        // Create shared state and run pipe server
        let state = Arc::new(Mutex::new(crate::ServiceState::new()));
        let state_clone = Arc::clone(&state);

        let server_handle = std::thread::spawn(move || {
            if let Err(e) = pipe_server::run_pipe_server(state_clone) {
                log::error!("Pipe server error: {}", e);
            }
        });

        // Wait for shutdown signal
        let _ = shutdown_rx.recv();
        log::info!("Service shutting down...");

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: windows_service::service::ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })?;

        let _ = server_handle.join();
        Ok(())
    }

    if let Err(e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        log::error!("Service dispatcher failed: {}", e);
    }
}

/// Run in standalone mode (for development/testing)
fn run_standalone() {
    let state = Arc::new(Mutex::new(ServiceState::new()));
    log::info!("Starting pipe server in standalone mode...");
    if let Err(e) = pipe_server::run_pipe_server(state) {
        log::error!("Pipe server error: {}", e);
    }
}
