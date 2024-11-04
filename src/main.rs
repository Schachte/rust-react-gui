use cocoa::{
    appkit::{NSApplication, NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular},
    base::nil,
};
use std::{
    sync::mpsc::{channel as std_channel, TryRecvError},
    time::Instant,
};

use notify::{Event as NotifyEvent, RecursiveMode, Watcher};

use objc::{class, msg_send, sel, sel_impl};
use serde::{Deserialize, Serialize};
use std::{
    env,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    time::Duration,
};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};
use wry::{Result as WryResult, WebViewBuilder};

mod gui;

const WINDOW_WIDTH: f64 = 600.0;
const WINDOW_HEIGHT: f64 = 300.0;
const APP_HTML: &[u8] = include_bytes!("../frontend/dist/index.html");
const APP_BUNDLE: &[u8] = include_bytes!("../frontend/dist/assets/index.js");
const APP_CSS: &[u8] = include_bytes!("../frontend/dist/assets/style.css");

struct AssetManager {
    base_path: PathBuf,
}

impl AssetManager {
    fn new() -> Result<Self, ()> {
        // Get the executable's directory
        let exe_dir = env::current_exe()
            .map_err(|_| ())?
            .parent()
            .ok_or(())?
            .to_path_buf();

        // In development, use the frontend/dist directory
        let dev_path = Path::new("frontend/dist");

        // Check if we're running in development or production
        let base_path = if dev_path.exists() {
            dev_path.to_path_buf()
        } else {
            // In production, look for assets in a directory next to the executable
            exe_dir.join("assets")
        };

        Ok(Self { base_path })
    }

    fn load_asset(&self, relative_path: &str) -> Result<Vec<u8>, std::io::Error> {
        let path = self.base_path.join(relative_path);
        fs::read(&path)
    }

    fn get_html(&self) -> Result<Vec<u8>, std::io::Error> {
        self.load_asset("index.html")
    }

    fn get_js(&self) -> Result<Vec<u8>, std::io::Error> {
        self.load_asset("assets/index.js")
    }

    fn get_css(&self) -> Result<Vec<u8>, std::io::Error> {
        self.load_asset("assets/style.css")
    }
}

// Structured message types
#[derive(Debug, Deserialize)]
struct IpcRequest {
    function: String,
    args: Vec<String>,
}

#[derive(Debug, Serialize)]
struct IpcResponse {
    success: bool,
    data: Option<String>,
    error: Option<String>,
}

// Error handling
#[derive(Debug)]
enum AppError {
    InvalidArgCount {
        function: String,
        expected: usize,
        got: usize,
    },
    ParseError {
        message: String,
    },
    UnknownFunction(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgCount {
                function,
                expected,
                got,
            } => write!(
                f,
                "Invalid argument count for {}: expected {}, got {}",
                function, expected, got
            ),
            Self::ParseError { message } => write!(f, "Parse error: {}", message),
            Self::UnknownFunction(name) => write!(f, "Unknown function: {}", name),
        }
    }
}

impl Error for AppError {}

// Protocol handlers
trait ProtocolHandler: Send + Sync {
    fn handle(&self, function: &str, args: &[String]) -> Result<String, AppError>;
}

struct TuffiProtocolHandler;

impl ProtocolHandler for TuffiProtocolHandler {
    fn handle(&self, function: &str, args: &[String]) -> Result<String, AppError> {
        match function {
            "hello" => Ok(format!("Hello from Rust! Args: {:?}", args)),
            "add" => self.handle_add(args),
            _ => Err(AppError::UnknownFunction(function.to_string())),
        }
    }
}

impl TuffiProtocolHandler {
    fn handle_add(&self, args: &[String]) -> Result<String, AppError> {
        if args.len() != 2 {
            return Err(AppError::InvalidArgCount {
                function: "add".to_string(),
                expected: 2,
                got: args.len(),
            });
        }

        let parse_number = |s: &str| -> Result<i32, AppError> {
            s.parse().map_err(|_| AppError::ParseError {
                message: format!("Failed to parse '{}' as number", s),
            })
        };

        let a = parse_number(&args[0])?;
        let b = parse_number(&args[1])?;

        Ok(format!("Sum: {}", a + b))
    }
}

// Modified WebView setup function
fn setup_webview(
    window: &tao::window::Window,
    protocol_handler: Arc<dyn ProtocolHandler>,
) -> WryResult<(wry::WebView, mpsc::Receiver<String>)> {
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(tx);

    // Create asset manager
    let asset_manager = match AssetManager::new() {
        Ok(am) => Arc::new(am),
        Err(e) => {
            eprintln!("Failed to initialize asset manager: {:?}", e);
            std::process::exit(1);
        }
    };

    let webview = WebViewBuilder::new()
        .with_initialization_script(&format!(
            "
            document.addEventListener('DOMContentLoaded', () => {{
                const style = document.createElement('style');
                style.textContent = `{}`;
                document.head.appendChild(style);
            }});
        ",
            r#"
                * {
                    cursor: default !important;
                    -webkit-user-select: none;
                    -moz-user-select: none;
                    -ms-user-select: none;
                    user-select: none;
                }
            "#
        ))
        .with_url("application://index.html")
        .with_ipc_handler(move |req| {
            let tx = tx.clone();
            let handler = protocol_handler.clone();
            handle_ipc_message(req.body(), tx, handler);
        })
        .with_initialization_script(
            r#"
            // Enable HMR support detection
            window.__HMR_ENABLED__ = true;
        "#,
        )
        .with_url("http://localhost:5173")
        .with_custom_protocol("application".into(), {
            let asset_manager = asset_manager.clone();
            move |_req, _resp| match asset_manager.get_html() {
                Ok(content) => wry::http::Response::builder()
                    .header("Content-Type", "text/html")
                    .body(std::borrow::Cow::Owned(content))
                    .unwrap(),
                Err(e) => {
                    eprintln!("Failed to load HTML: {}", e);
                    wry::http::Response::builder()
                        .status(500)
                        .body(std::borrow::Cow::Owned(Vec::new()))
                        .unwrap()
                }
            }
        })
        .with_custom_protocol("assets".into(), {
            let asset_manager = asset_manager.clone();
            move |_, req| {
                let path = req.uri().path();

                let (content_type, content) = if path.ends_with(".css") {
                    ("text/css", asset_manager.get_css())
                } else if path.ends_with(".js") {
                    ("application/javascript", asset_manager.get_js())
                } else if path.ends_with(".wasm") {
                    ("application/wasm", asset_manager.get_js())
                } else {
                    ("application/octet-stream", asset_manager.get_js())
                };

                match content {
                    Ok(data) => wry::http::Response::builder()
                        .header("Content-Type", content_type)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                        .header("Access-Control-Allow-Headers", "Content-Type")
                        .header("Cross-Origin-Opener-Policy", "same-origin")
                        .header("Cross-Origin-Embedder-Policy", "require-corp")
                        .body(std::borrow::Cow::Owned(data))
                        .unwrap(),
                    Err(e) => {
                        eprintln!("Failed to load asset {}: {}", path, e);
                        wry::http::Response::builder()
                            .status(404)
                            .body(std::borrow::Cow::Owned(Vec::new()))
                            .unwrap()
                    }
                }
            }
        })
        .build(window)?;

    Ok((webview, rx))
}

fn handle_ipc_message(
    body: &str,
    tx: Arc<mpsc::Sender<String>>,
    protocol_handler: Arc<dyn ProtocolHandler>,
) {
    let response = match serde_json::from_str::<IpcRequest>(body) {
        Ok(req) => match protocol_handler.handle(&req.function, &req.args) {
            Ok(result) => IpcResponse {
                success: true,
                data: Some(result),
                error: None,
            },
            Err(e) => IpcResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            },
        },
        Err(e) => IpcResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to parse message: {}", e)),
        },
    };

    let js = format!(
        "window.dispatchEvent(new CustomEvent('rust-response', {{ detail: {} }}));",
        serde_json::to_string(&response).unwrap_or_default()
    );

    if let Err(e) = tx.send(js) {
        eprintln!("Failed to send response: {}", e);
    }
}

fn main() -> WryResult<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(tao::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .build(&event_loop)
        .expect("Failed to build window");

    unsafe {
        let app = NSApplication::sharedApplication(nil);
        let _: () = msg_send![app, setActivationPolicy: NSApplicationActivationPolicyRegular];

        app.activateIgnoringOtherApps_(true);
        gui::make_borderless(&window);
        gui::disable_window_resize(&window);
        gui::show_titlebar_and_controls(&window);
        gui::create_menu_bar("React GUI In Rust");
    }

    let protocol_handler = Arc::new(TuffiProtocolHandler);
    let (webview, rx) = setup_webview(&window, protocol_handler)?;
    let webview = Arc::new(webview);

    // Initialize webview with HMR support script

    // Set up file watcher with crossbeam channel for better performance
    let (watcher_tx, watcher_rx) = crossbeam_channel::unbounded();
    let mut watcher =
        notify::recommended_watcher(move |res: Result<NotifyEvent, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    // Check if any of the modified paths have .js or .css extension
                    let should_reload = event.paths.iter().any(|path| {
                        if let Some(ext) = path.extension() {
                            matches!(ext.to_str(), Some("js") | Some("css") | Some("html"))
                        } else {
                            false
                        }
                    });

                    if should_reload {
                        println!("Detected change in JS/CSS file: {:?}", event.paths);
                        let _ = watcher_tx.try_send(());
                    }
                }
            }
        })
        .expect("Failed to create file watcher");

    // Watch the assets directory
    watcher
        .watch(Path::new("frontend/dist"), RecursiveMode::Recursive)
        .expect("Failed to watch assets directory");

    event_loop.run(move |event, _, control_flow| {
        // Use Poll mode for more responsive events
        *control_flow = ControlFlow::Poll;

        match event {
            Event::NewEvents(StartCause::Init) => (),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // Use try_recv in a loop to handle all pending events
                while let Ok(()) = watcher_rx.try_recv() {
                    println!(
                        "File change detected at {:?}, reloading",
                        std::time::SystemTime::now()
                    );
                    if let Err(e) = webview.evaluate_script("import.meta.hot.invalidate()") {
                        eprintln!("Failed to reload page: {}", e);
                    }
                    window.request_redraw();
                }

                // Handle other events
                while let Ok(js) = rx.try_recv() {
                    if let Err(e) = webview.evaluate_script(&js) {
                        eprintln!("Failed to evaluate script: {}", e);
                    }
                    window.request_redraw();
                }
            }
            _ => (),
        }
    });
}
