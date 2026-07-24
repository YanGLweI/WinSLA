//! WebView2 native window using wry + tao

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

/// Launch the WebView2 GUI window pointing to the local server
pub fn launch_gui(port: u16) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("WinSLA Management")
        .with_inner_size(tao::dpi::LogicalSize::new(1024, 720))
        .with_min_inner_size(tao::dpi::LogicalSize::new(860, 560))
        .build(&event_loop)
        .expect("Failed to create window");

    let _webview = WebViewBuilder::new()
        .with_url(format!("http://127.0.0.1:{}", port))
        .build(&window)
        .expect("Failed to create WebView2");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                log::info!("WinSLA Management window opened");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
