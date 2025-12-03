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
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::AppState;
use config::Config;
use sources::{
    LogEvent, LogSource, LogSourceType, SourcedLogEvent, file::FileSource, manager::SourceManager,
};

/// Parsed source with its type and implementation
struct ParsedSource {
    source_type: LogSourceType,
    source: Box<dyn LogSource>,
}

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
        eprintln!("Usage: bark <file_path> [additional sources...]");
        eprintln!("       bark --docker <container> [--docker <container2> ...]");
        eprintln!("       bark --k8s <pod> [-n namespace] [-c container] [--k8s <pod2> ...]");
        eprintln!("       bark --ssh <host> <remote_path>");
        eprintln!("\nMultiple sources can be combined:");
        eprintln!("       bark --docker nginx --docker redis");
        eprintln!("       bark /var/log/app.log --docker nginx");
        eprintln!("\nRun 'bark --help' for more information.");
        std::process::exit(1);
    }

    // Parse all sources from command line
    let parsed_sources = parse_sources(&args)?;

    if parsed_sources.is_empty() {
        eprintln!("No valid sources specified. Run 'bark --help' for usage.");
        std::process::exit(1);
    }

    // Load config
    let config = Config::from_env();

    // Extract source types for AppState
    let source_types: Vec<LogSourceType> = parsed_sources
        .iter()
        .map(|p| p.source_type.clone())
        .collect();

    // Initialize state
    let mut state = AppState::new(&config, source_types);

    // Create source manager and add all sources
    let (mut source_manager, mut event_rx) = SourceManager::new(1000);
    for (idx, parsed) in parsed_sources.into_iter().enumerate() {
        source_manager.add_source(idx, parsed.source).await;
    }

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
    let result = run_event_loop(&mut terminal, &mut state, &mut event_rx).await;

    // Clean up source manager
    drop(source_manager);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;

    result
}

/// Parse command line arguments into sources
fn parse_sources(args: &[String]) -> Result<Vec<ParsedSource>> {
    let mut sources: Vec<ParsedSource> = Vec::new();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--docker" => {
                if i + 1 >= args.len() {
                    anyhow::bail!("--docker requires a container name");
                }
                let container = args[i + 1].clone();
                sources.push(ParsedSource {
                    source_type: LogSourceType::Docker {
                        container: container.clone(),
                    },
                    source: Box::new(sources::docker::DockerSource::new(container)),
                });
                i += 2;
            }
            "--k8s" => {
                if i + 1 >= args.len() {
                    anyhow::bail!("--k8s requires a pod name");
                }
                let pod = args[i + 1].clone();
                let mut namespace: Option<String> = None;
                let mut container: Option<String> = None;
                i += 2;

                // Parse optional -n and -c following this --k8s
                while i < args.len() {
                    match args[i].as_str() {
                        "-n" | "--namespace" if i + 1 < args.len() => {
                            namespace = Some(args[i + 1].clone());
                            i += 2;
                        }
                        "-c" | "--container" if i + 1 < args.len() => {
                            container = Some(args[i + 1].clone());
                            i += 2;
                        }
                        // Stop at next source or unknown arg
                        _ => break,
                    }
                }

                sources.push(ParsedSource {
                    source_type: LogSourceType::K8s {
                        pod: pod.clone(),
                        namespace: namespace.clone(),
                        container: container.clone(),
                    },
                    source: Box::new(sources::k8s::K8sSource::new(pod, namespace, container)),
                });
            }
            "--ssh" => {
                if i + 2 >= args.len() {
                    anyhow::bail!("--ssh requires <host> <remote_path>");
                }
                let host = args[i + 1].clone();
                let path = args[i + 2].clone();
                sources.push(ParsedSource {
                    source_type: LogSourceType::Ssh {
                        host: host.clone(),
                        path: path.clone(),
                    },
                    source: Box::new(sources::ssh::SshSource::new(host, path)),
                });
                i += 3;
            }
            path if !path.starts_with('-') => {
                let path = PathBuf::from(path);
                sources.push(ParsedSource {
                    source_type: LogSourceType::File { path: path.clone() },
                    source: Box::new(FileSource::new(path)),
                });
                i += 1;
            }
            unknown => {
                anyhow::bail!("Unknown argument: {}", unknown);
            }
        }
    }

    Ok(sources)
}

fn print_help() {
    println!(
        "bark {} - A keyboard-driven TUI for exploring logs",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("    bark <file_path> [additional sources...]");
    println!("    bark --docker <container> [--docker <container2> ...]");
    println!("    bark --k8s <pod> [-n namespace] [-c container] [--k8s <pod2> ...]");
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
    println!("EXAMPLES:");
    println!("    bark /var/log/syslog");
    println!("    bark --docker nginx");
    println!("    bark --docker nginx --docker redis     # Multiple containers");
    println!("    bark --k8s my-app -n production");
    println!("    bark --k8s frontend --k8s backend      # Multiple pods");
    println!("    bark /var/log/app.log --docker nginx   # Mixed sources");
    println!("    bark --ssh user@server /var/log/app.log");
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
    println!("    b                Toggle side panel");
    println!("    Tab              Cycle panel focus");
    println!("    Space            Toggle source visibility (in Sources panel)");
    println!("    e                Export filtered lines");
    println!("    ?                Show full help");
    println!("    q                Quit");
    println!();
    println!("ENVIRONMENT:");
    println!("    BARK_MAX_LINES      Maximum lines in buffer (default: 10000)");
    println!("    BARK_THEME          Color theme (default, kawaii, cyber, dracula, monochrome)");
    println!("    BARK_LEVEL_COLORS   Enable level coloring (1/true or 0/false)");
    println!("    BARK_LINE_WRAP      Enable line wrapping (1/true or 0/false)");
    println!();
    println!("CONFIG:");
    println!("    ~/.config/bark/config.toml");
    println!();
    println!("For more information, see: https://github.com/lance0/bark");
}

async fn run_event_loop<'a>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState<'a>,
    event_rx: &mut tokio::sync::mpsc::Receiver<SourcedLogEvent>,
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

            // Check for new log lines from any source
            Some(sourced_event) = event_rx.recv() => {
                match sourced_event.event {
                    LogEvent::Line(line) => {
                        // Set source_id on the line before pushing
                        let line = line.with_source_id(sourced_event.source_id);
                        state.push_line(line);
                    }
                    LogEvent::Error(msg) => {
                        let source_name = state.sources
                            .get(sourced_event.source_id)
                            .map(|s| s.name())
                            .unwrap_or_else(|| "unknown".to_string());
                        state.status_message = Some(format!("[{}] Error: {}", source_name, msg));
                    }
                    LogEvent::EndOfStream => {
                        let source_name = state.sources
                            .get(sourced_event.source_id)
                            .map(|s| s.name())
                            .unwrap_or_else(|| "unknown".to_string());
                        state.status_message = Some(format!("[{}] Stream ended", source_name));
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
