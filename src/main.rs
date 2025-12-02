mod app;
mod config;
mod filter;
mod input;
mod sources;
mod theme;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::AppState;
use config::Config;
use sources::{file::FileSource, LogEvent, LogSource, LogSourceType};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Handle --help and --version
    if args.len() >= 2 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("bark {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {}
        }
    }

    if args.len() < 2 {
        eprintln!("Usage: bark <file_path>");
        eprintln!("       bark --docker <container_name>");
        eprintln!("       bark --k8s <pod_name> [-n namespace] [-c container]");
        eprintln!("       bark --ssh <host> <remote_path>");
        eprintln!("\nRun 'bark --help' for more information.");
        std::process::exit(1);
    }

    let (source_type, source): (LogSourceType, Box<dyn LogSource>) = if args[1] == "--docker" {
        if args.len() < 3 {
            eprintln!("Usage: bark --docker <container_name>");
            std::process::exit(1);
        }
        let container = args[2].clone();
        (
            LogSourceType::Docker { container: container.clone() },
            Box::new(sources::docker::DockerSource::new(container)),
        )
    } else if args[1] == "--k8s" {
        if args.len() < 3 {
            eprintln!("Usage: bark --k8s <pod_name> [-n namespace] [-c container]");
            std::process::exit(1);
        }
        let pod = args[2].clone();
        let mut namespace: Option<String> = None;
        let mut container: Option<String> = None;

        // Parse optional arguments
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "-n" | "--namespace" => {
                    if i + 1 < args.len() {
                        namespace = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("Missing namespace after -n");
                        std::process::exit(1);
                    }
                }
                "-c" | "--container" => {
                    if i + 1 < args.len() {
                        container = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("Missing container after -c");
                        std::process::exit(1);
                    }
                }
                _ => {
                    eprintln!("Unknown argument: {}", args[i]);
                    std::process::exit(1);
                }
            }
        }

        (
            LogSourceType::K8s { pod: pod.clone(), namespace: namespace.clone(), container: container.clone() },
            Box::new(sources::k8s::K8sSource::new(pod, namespace, container)),
        )
    } else if args[1] == "--ssh" {
        if args.len() < 4 {
            eprintln!("Usage: bark --ssh <host> <remote_path>");
            std::process::exit(1);
        }
        let host = args[2].clone();
        let path = args[3].clone();
        (
            LogSourceType::Ssh { host: host.clone(), path: path.clone() },
            Box::new(sources::ssh::SshSource::new(host, path)),
        )
    } else {
        let path = PathBuf::from(&args[1]);
        (
            LogSourceType::File { path: path.clone() },
            Box::new(FileSource::new(path)),
        )
    };

    // Load config
    let config = Config::from_env();

    // Initialize state
    let mut state = AppState::new(&config, source_type);

    // Start the log source stream
    let mut log_rx = source.stream().await;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        original_hook(panic);
    }));

    // Main event loop
    let result = run_event_loop(&mut terminal, &mut state, &mut log_rx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;

    result
}

fn print_help() {
    println!("bark {} - A keyboard-driven TUI for exploring logs", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    bark <file_path>");
    println!("    bark --docker <container_name>");
    println!("    bark --k8s <pod_name> [-n namespace] [-c container]");
    println!("    bark --ssh <host> <remote_path>");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print help information");
    println!("    -V, --version    Print version information");
    println!();
    println!("SOURCES:");
    println!("    <file_path>      Tail a local log file");
    println!("    --docker         Follow Docker container logs");
    println!("    --k8s            Follow Kubernetes pod logs");
    println!("    --ssh            Tail a remote file via SSH");
    println!();
    println!("KEYBOARD SHORTCUTS:");
    println!("    j/k              Scroll down/up");
    println!("    g/G              Go to top/bottom");
    println!("    /                Start filter input");
    println!("    n/N              Next/previous match");
    println!("    m                Toggle bookmark");
    println!("    [/]              Previous/next bookmark");
    println!("    t                Toggle relative time");
    println!("    J                Toggle JSON pretty-print");
    println!("    w                Toggle line wrap");
    println!("    e                Export filtered lines");
    println!("    ?                Show full help");
    println!("    q                Quit");
    println!();
    println!("CONFIG:");
    println!("    ~/.config/bark/config.toml");
    println!();
    println!("For more information, see: https://github.com/lance0/bark");
}

async fn run_event_loop<'a>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState<'a>,
    log_rx: &mut tokio::sync::mpsc::Receiver<LogEvent>,
) -> Result<()> {
    loop {
        // Check filter debounce before drawing
        state.check_filter_debounce();

        // Draw UI
        terminal.draw(|frame| {
            ui::draw(frame, state);
        })?;

        // Calculate page size for scrolling
        let page_size = terminal.size()?.height.saturating_sub(4) as usize;

        // Use tokio::select! to handle both terminal events and log events
        tokio::select! {
            // Check for terminal input events
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                // Poll for events with no blocking
                if event::poll(Duration::ZERO)? {
                    match event::read()? {
                        Event::Key(key) => {
                            // Only handle key press events (not release)
                            if key.kind == KeyEventKind::Press {
                                input::handle_key(state, key, page_size);
                            }
                        }
                        Event::Mouse(mouse) => {
                            input::handle_mouse(state, mouse, page_size);
                        }
                        _ => {}
                    }
                }
            }

            // Check for new log lines
            Some(event) = log_rx.recv() => {
                match event {
                    LogEvent::Line(line) => {
                        state.push_line(line);
                    }
                    LogEvent::Error(msg) => {
                        state.status_message = Some(format!("Error: {}", msg));
                    }
                    LogEvent::EndOfStream => {
                        state.status_message = Some("Stream ended".to_string());
                    }
                }
            }
        }

        // Check if we should quit
        if state.should_quit {
            break;
        }
    }

    Ok(())
}
