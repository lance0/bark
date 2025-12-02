use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::{LogEvent, LogSource};
use crate::app::LogLine;
use async_trait::async_trait;

/// SSH remote file log source
pub struct SshSource {
    /// SSH host (user@host or just host)
    host: String,
    /// Remote file path
    path: String,
}

impl SshSource {
    pub fn new(host: String, path: String) -> Self {
        Self { host, path }
    }
}

#[async_trait]
impl LogSource for SshSource {
    async fn stream(&self) -> mpsc::Receiver<LogEvent> {
        let (tx, rx) = mpsc::channel(1000);

        let host = self.host.clone();
        let path = self.path.clone();

        tokio::spawn(async move {
            // Use ssh to run tail -F on the remote host
            let mut cmd = Command::new("ssh");
            cmd.arg("-o")
                .arg("BatchMode=yes")  // Disable password prompts
                .arg("-o")
                .arg("StrictHostKeyChecking=accept-new")  // Accept new host keys
                .arg(&host)
                .arg("tail")
                .arg("-F")
                .arg("-n")
                .arg("1000")
                .arg(&path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(e) => {
                    let _ = tx.send(LogEvent::Error(format!("Failed to run ssh: {}", e))).await;
                    return;
                }
            };

            let stdout = match child.stdout.take() {
                Some(stdout) => stdout,
                None => {
                    let _ = tx.send(LogEvent::Error("Failed to get stdout".to_string())).await;
                    return;
                }
            };

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if tx.send(LogEvent::Line(LogLine::new(line))).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = tx.send(LogEvent::EndOfStream).await;
                        break;
                    }
                    Err(e) => {
                        let _ = tx.send(LogEvent::Error(e.to_string())).await;
                        break;
                    }
                }
            }

            let _ = child.kill().await;
        });

        rx
    }

    fn name(&self) -> String {
        format!("ssh:{}:{}", self.host, self.path)
    }
}
