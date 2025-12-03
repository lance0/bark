use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::{LogEvent, LogSource};
use crate::app::LogLine;
use crate::config::{DEFAULT_CHANNEL_BUFFER, DEFAULT_TAIL_LINES};

/// Validate Docker container name to prevent option injection.
pub fn validate_container_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Container name cannot be empty".to_string());
    }

    // Reject names starting with '-' to prevent option injection
    if name.starts_with('-') {
        return Err("Invalid container name: cannot start with '-'".to_string());
    }

    Ok(())
}

/// A log source that reads from a Docker container using docker logs -f
pub struct DockerSource {
    container: String,
}

impl DockerSource {
    pub fn new(container: String) -> Self {
        Self { container }
    }
}

#[async_trait::async_trait]
impl LogSource for DockerSource {
    async fn stream(&self) -> mpsc::Receiver<LogEvent> {
        let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER);
        let container = self.container.clone();

        tokio::spawn(async move {
            let result = Command::new("docker")
                .arg("logs")
                .arg("-f")
                .arg("--tail")
                .arg(DEFAULT_TAIL_LINES)
                .arg("--") // Prevent option injection from container name
                .arg(&container)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match result {
                Ok(mut child) => {
                    let tx_stderr = tx.clone();

                    // Spawn task to read stderr
                    let stderr_handle = child.stderr.take().map(|stderr| {
                        tokio::spawn(async move {
                            let reader = BufReader::new(stderr);
                            let mut lines = reader.lines();
                            loop {
                                match lines.next_line().await {
                                    Ok(Some(line)) => {
                                        if tx_stderr
                                            .send(LogEvent::Line(LogLine::new(line)))
                                            .await
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                    Ok(None) => break,
                                    Err(e) => {
                                        let _ = tx_stderr
                                            .send(LogEvent::Error(format!(
                                                "stderr read error: {}",
                                                e
                                            )))
                                            .await;
                                        // Continue reading
                                    }
                                }
                            }
                        })
                    });

                    // Read stdout in main task
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();

                        loop {
                            match lines.next_line().await {
                                Ok(Some(line)) => {
                                    if tx.send(LogEvent::Line(LogLine::new(line))).await.is_err() {
                                        break;
                                    }
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    let _ = tx
                                        .send(LogEvent::Error(format!("stdout read error: {}", e)))
                                        .await;
                                    // Continue reading - don't abort on single bad line
                                }
                            }
                        }
                    }

                    // Wait for stderr task
                    if let Some(handle) = stderr_handle {
                        let _ = handle.await;
                    }

                    match child.wait().await {
                        Ok(status) if !status.success() => {
                            let _ = tx
                                .send(LogEvent::Error(format!(
                                    "docker logs exited with status: {}",
                                    status
                                )))
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(LogEvent::Error(format!(
                                    "Error waiting for docker logs: {}",
                                    e
                                )))
                                .await;
                        }
                        _ => {}
                    }

                    let _ = tx.send(LogEvent::EndOfStream).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(LogEvent::Error(format!(
                            "Failed to start docker logs for '{}': {}. Is Docker installed and running?",
                            container, e
                        )))
                        .await;
                    let _ = tx.send(LogEvent::EndOfStream).await;
                }
            }
        });

        rx
    }

    fn name(&self) -> String {
        self.container.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_container_name_valid() {
        assert!(validate_container_name("nginx").is_ok());
        assert!(validate_container_name("my-container").is_ok());
        assert!(validate_container_name("my_container").is_ok());
        assert!(validate_container_name("container123").is_ok());
        assert!(validate_container_name("my-app-v1.2.3").is_ok());
    }

    #[test]
    fn test_validate_container_name_rejects_dash_prefix() {
        // Prevent option injection to docker logs
        assert!(validate_container_name("-f").is_err());
        assert!(validate_container_name("--help").is_err());
        assert!(validate_container_name("-v").is_err());
    }

    #[test]
    fn test_validate_container_name_rejects_empty() {
        assert!(validate_container_name("").is_err());
    }
}
