mod app;
mod config;
mod discovery;
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

use app::{AppState, LogLine, PickerMode};
use config::Config;
use discovery::{discover_docker_containers, discover_k8s_pods};
use input::{PickerAction, handle_picker_input};
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

    // Load config first (needed for SSH settings)
    let config = Config::from_env();

    // Parse all sources from command line (or empty if none specified)
    let (parsed_sources, open_picker_mode) = parse_sources(&args, &config)?;

    // Extract source types for AppState
    let source_types: Vec<LogSourceType> = parsed_sources
        .iter()
        .map(|p| p.source_type.clone())
        .collect();

    // Initialize state
    let mut state = AppState::new(&config, source_types);

    // Open picker on startup if requested
    if let Some(mode) = open_picker_mode {
        state.picker.open(mode);
    }

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
    let result = run_event_loop(
        &mut terminal,
        &mut state,
        &mut event_rx,
        &mut source_manager,
    )
    .await;

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
/// Returns (sources, optional picker mode to open on startup)
fn parse_sources(
    args: &[String],
    config: &Config,
) -> Result<(Vec<ParsedSource>, Option<PickerMode>)> {
    let mut sources: Vec<ParsedSource> = Vec::new();
    let mut i = 1;

    // No args - open picker
    if args.len() < 2 {
        return Ok((sources, Some(PickerMode::Docker)));
    }

    while i < args.len() {
        match args[i].as_str() {
            "--all" => {
                // Discover all Docker containers
                if let Ok(docker_sources) = discover_docker_containers() {
                    for ds in docker_sources {
                        sources.push(ParsedSource {
                            source_type: LogSourceType::Docker {
                                container: ds.name.clone(),
                            },
                            source: Box::new(sources::docker::DockerSource::new(ds.name)),
                        });
                    }
                }
                // Discover all K8s pods
                if let Ok(k8s_sources) = discover_k8s_pods(None) {
                    for ds in k8s_sources {
                        sources.push(ParsedSource {
                            source_type: LogSourceType::K8s {
                                pod: ds.name.clone(),
                                namespace: ds.namespace.clone(),
                                container: None,
                            },
                            source: Box::new(sources::k8s::K8sSource::new(
                                ds.name,
                                ds.namespace,
                                None,
                            )),
                        });
                    }
                }
                i += 1;
            }
            "--docker" => {
                // Check if next arg is a container name or another flag
                let has_container_name = i + 1 < args.len() && !args[i + 1].starts_with('-');

                if has_container_name {
                    let container = args[i + 1].clone();

                    // Validate container name to prevent option injection
                    if let Err(e) = sources::docker::validate_container_name(&container) {
                        anyhow::bail!("{}", e);
                    }

                    sources.push(ParsedSource {
                        source_type: LogSourceType::Docker {
                            container: container.clone(),
                        },
                        source: Box::new(sources::docker::DockerSource::new(container)),
                    });
                    i += 2;
                } else {
                    // --docker without name: discover all Docker containers
                    if let Ok(docker_sources) = discover_docker_containers() {
                        for ds in docker_sources {
                            sources.push(ParsedSource {
                                source_type: LogSourceType::Docker {
                                    container: ds.name.clone(),
                                },
                                source: Box::new(sources::docker::DockerSource::new(ds.name)),
                            });
                        }
                    }
                    i += 1;
                }
            }
            "--k8s" => {
                // Check if next arg is a pod name or another flag/namespace option
                let has_pod_name = i + 1 < args.len()
                    && !args[i + 1].starts_with('-')
                    && args[i + 1] != "-n"
                    && args[i + 1] != "-c";

                if has_pod_name {
                    let pod = args[i + 1].clone();

                    // Validate pod name to prevent option injection
                    if let Err(e) = sources::k8s::validate_pod_name(&pod) {
                        anyhow::bail!("{}", e);
                    }
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
                } else {
                    // --k8s without pod name: parse optional namespace, then discover all
                    let mut namespace: Option<String> = None;
                    i += 1;

                    // Check for -n flag
                    if i < args.len()
                        && (args[i] == "-n" || args[i] == "--namespace")
                        && i + 1 < args.len()
                    {
                        namespace = Some(args[i + 1].clone());
                        i += 2;
                    }

                    // Discover all K8s pods in namespace
                    if let Ok(k8s_sources) = discover_k8s_pods(namespace.as_deref()) {
                        for ds in k8s_sources {
                            sources.push(ParsedSource {
                                source_type: LogSourceType::K8s {
                                    pod: ds.name.clone(),
                                    namespace: ds.namespace.clone(),
                                    container: None,
                                },
                                source: Box::new(sources::k8s::K8sSource::new(
                                    ds.name,
                                    ds.namespace,
                                    None,
                                )),
                            });
                        }
                    }
                }
            }
            "--ssh" => {
                if i + 2 >= args.len() {
                    anyhow::bail!("--ssh requires <host> <remote_path>");
                }
                let host = args[i + 1].clone();
                let path = args[i + 2].clone();

                // Validate SSH host to prevent command injection
                if let Err(e) = sources::ssh::validate_ssh_host(&host) {
                    anyhow::bail!("{}", e);
                }

                // Validate remote path
                if let Err(e) = sources::ssh::validate_remote_path(&path) {
                    anyhow::bail!("{}", e);
                }

                sources.push(ParsedSource {
                    source_type: LogSourceType::Ssh {
                        host: host.clone(),
                        path: path.clone(),
                    },
                    source: Box::new(sources::ssh::SshSource::with_host_key_checking(
                        host,
                        path,
                        config.ssh_host_key_checking.clone(),
                    )),
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

    Ok((sources, None))
}

fn print_help() {
    println!(
        "bark {} - A keyboard-driven TUI for exploring logs",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("    bark                                      # Open source picker");
    println!("    bark --docker                             # All Docker containers");
    println!("    bark --docker <container>                 # Specific container");
    println!("    bark --k8s                                # All K8s pods");
    println!("    bark --k8s <pod> [-n namespace]           # Specific pod");
    println!("    bark --all                                # All Docker + K8s");
    println!("    bark <file_path>                          # Tail a file");
    println!("    bark --ssh <host> <remote_path>           # Remote file via SSH");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print help information");
    println!("    -V, --version    Print version information");
    println!("    --all            Discover all Docker containers and K8s pods");
    println!();
    println!("SOURCES:");
    println!("    <file_path>      Tail a local log file");
    println!("    --docker         Follow Docker container logs (all if no name given)");
    println!("    --k8s            Follow Kubernetes pod logs (all if no name given)");
    println!("    --ssh            Tail a remote file via SSH");
    println!();
    println!("EXAMPLES:");
    println!("    bark                                      # Interactive picker");
    println!("    bark --docker                             # All running containers");
    println!("    bark --docker nginx                       # Specific container");
    println!("    bark --docker nginx --docker redis        # Multiple containers");
    println!("    bark --k8s -n production                  # All pods in namespace");
    println!("    bark --k8s my-app -n production           # Specific pod");
    println!("    bark --all                                # Everything");
    println!("    bark /var/log/app.log --docker nginx      # Mixed sources");
    println!("    bark --ssh user@server /var/log/app.log   # Remote file");
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
    println!("    D                Open Docker container picker");
    println!("    K                Open Kubernetes pod picker");
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

/// Target frame rate for UI updates (~60fps)
const FRAME_DURATION: Duration = Duration::from_millis(16);

/// Maximum lines to batch before forcing a draw
const MAX_BATCH_SIZE: usize = 500;

async fn run_event_loop<'a>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState<'a>,
    event_rx: &mut tokio::sync::mpsc::Receiver<SourcedLogEvent>,
    source_manager: &mut SourceManager,
) -> Result<()> {
    // Track pending discovery task to avoid blocking UI
    let mut discovery_rx: Option<
        tokio::sync::oneshot::Receiver<anyhow::Result<Vec<discovery::DiscoveredSource>>>,
    > = None;

    // Track when we last drew for frame rate limiting
    let mut last_draw = std::time::Instant::now();

    loop {
        // Check filter debounce before drawing
        state.check_filter_debounce();

        // Clear pending discovery if picker was closed
        if !state.picker.visible && discovery_rx.is_some() {
            discovery_rx = None;
        }

        // Check if picker needs to trigger discovery (non-blocking)
        if state.picker.visible && state.picker.loading && discovery_rx.is_none() {
            let mode = state.picker.mode;
            let (tx, rx) = tokio::sync::oneshot::channel();
            discovery_rx = Some(rx);

            // Spawn blocking discovery in background
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || match mode {
                    PickerMode::Docker => discover_docker_containers(),
                    PickerMode::K8s => discover_k8s_pods(None),
                })
                .await
                .unwrap_or_else(|e| Err(anyhow::anyhow!("Discovery task panicked: {}", e)));
                let _ = tx.send(result);
            });
        }

        // Check for discovery result (non-blocking)
        if let Some(ref mut rx) = discovery_rx {
            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(sources) => state.picker.set_sources(sources, &state.sources),
                        Err(e) => state.picker.set_error(e.to_string()),
                    }
                    discovery_rx = None;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    // Still waiting, keep the receiver
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    // Sender dropped without sending - shouldn't happen
                    state.picker.set_error("Discovery task failed".to_string());
                    discovery_rx = None;
                }
            }
        }

        // Throttled drawing - only draw if enough time has passed
        let elapsed = last_draw.elapsed();
        if elapsed >= FRAME_DURATION {
            terminal.draw(|frame| {
                ui::draw(frame, state);
            })?;
            last_draw = std::time::Instant::now();
        }

        // Calculate page size for scrolling
        let page_size = terminal.size()?.height.saturating_sub(4) as usize;

        // Use tokio::select! to handle both terminal events and log events
        tokio::select! {
            // Check for terminal input events
            _ = tokio::time::sleep(Duration::from_millis(1)) => {
                // Poll for events with no blocking
                if event::poll(Duration::ZERO)? {
                    match event::read()? {
                        Event::Key(key) => {
                            // Only handle key press events (not release)
                            if key.kind == KeyEventKind::Press {
                                // Handle picker mode separately
                                if state.picker.visible {
                                    let action = handle_picker_input(state, key);
                                    if let PickerAction::ModifySources { add, remove, mode } = action {
                                        let mut added_count = 0;
                                        let mut removed_count = 0;

                                        // Hide sources that were deselected
                                        for to_remove in &remove {
                                            // Find and hide the source
                                            for (idx, source) in state.sources.iter().enumerate() {
                                                let matches = match (source, mode) {
                                                    (LogSourceType::Docker { container }, PickerMode::Docker) => {
                                                        container == &to_remove.name
                                                    }
                                                    (LogSourceType::K8s { pod, namespace, .. }, PickerMode::K8s) => {
                                                        pod == &to_remove.name && *namespace == to_remove.namespace
                                                    }
                                                    _ => false,
                                                };
                                                if matches {
                                                    // Hide in all panes
                                                    for pane in &mut state.panes {
                                                        if let Some(visible) = pane.visible_sources.get_mut(idx) {
                                                            *visible = false;
                                                        }
                                                    }
                                                    removed_count += 1;
                                                    break;
                                                }
                                            }
                                        }

                                        // Add new sources
                                        for selected in add {
                                            let source_id = state.sources.len();
                                            let (source_type, source): (LogSourceType, Box<dyn LogSource>) = match mode {
                                                PickerMode::Docker => {
                                                    (
                                                        LogSourceType::Docker { container: selected.name.clone() },
                                                        Box::new(sources::docker::DockerSource::new(selected.name)),
                                                    )
                                                }
                                                PickerMode::K8s => {
                                                    (
                                                        LogSourceType::K8s {
                                                            pod: selected.name.clone(),
                                                            namespace: selected.namespace.clone(),
                                                            container: None,
                                                        },
                                                        Box::new(sources::k8s::K8sSource::new(
                                                            selected.name,
                                                            selected.namespace,
                                                            None,
                                                        )),
                                                    )
                                                }
                                            };

                                            // Add to app state
                                            state.add_source(source_type);

                                            // Add to source manager
                                            source_manager.add_source(source_id, source).await;
                                            added_count += 1;
                                        }

                                        // Status message
                                        let msg = match (added_count, removed_count) {
                                            (0, 0) => "No changes".to_string(),
                                            (a, 0) => format!("Added {} source(s)", a),
                                            (0, r) => format!("Hidden {} source(s)", r),
                                            (a, r) => format!("Added {}, hidden {} source(s)", a, r),
                                        };
                                        state.status_message = Some(msg);
                                    }
                                } else {
                                    input::handle_key(state, key, page_size);
                                }
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
                // Batch processing: collect this event and any others available
                let mut batch: Vec<LogLine> = Vec::new();

                // Process the first event
                match sourced_event.event {
                    LogEvent::Line(line) => {
                        batch.push(line.with_source_id(sourced_event.source_id));
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

                // Drain any additional available events (non-blocking)
                while batch.len() < MAX_BATCH_SIZE {
                    match event_rx.try_recv() {
                        Ok(sourced_event) => {
                            match sourced_event.event {
                                LogEvent::Line(line) => {
                                    batch.push(line.with_source_id(sourced_event.source_id));
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
                        Err(_) => break, // No more events available
                    }
                }

                // Push all batched lines at once
                if !batch.is_empty() {
                    state.push_lines(batch);
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
