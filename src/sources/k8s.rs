use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::{LogEvent, LogSource};
use crate::app::LogLine;
use async_trait::async_trait;

/// Kubernetes pod log source using kubectl
pub struct K8sSource {
    /// Pod name (or name pattern)
    pod: String,
    /// Namespace (optional, defaults to current context)
    namespace: Option<String>,
    /// Container name (optional, required for multi-container pods)
    container: Option<String>,
}

impl K8sSource {
    pub fn new(pod: String, namespace: Option<String>, container: Option<String>) -> Self {
        Self { pod, namespace, container }
    }
}

#[async_trait]
impl LogSource for K8sSource {
    async fn stream(&self) -> mpsc::Receiver<LogEvent> {
        let (tx, rx) = mpsc::channel(1000);

        let pod = self.pod.clone();
        let namespace = self.namespace.clone();
        let container = self.container.clone();

        tokio::spawn(async move {
            let mut cmd = Command::new("kubectl");
            cmd.arg("logs")
                .arg("-f")
                .arg("--tail=1000");

            if let Some(ns) = namespace {
                cmd.arg("-n").arg(ns);
            }

            cmd.arg(&pod);

            if let Some(c) = container {
                cmd.arg("-c").arg(c);
            }

            cmd.stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(e) => {
                    let _ = tx.send(LogEvent::Error(format!("Failed to run kubectl: {}", e))).await;
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
        match (&self.namespace, &self.container) {
            (Some(ns), Some(c)) => format!("k8s:{}/{}/{}", ns, self.pod, c),
            (Some(ns), None) => format!("k8s:{}/{}", ns, self.pod),
            (None, Some(c)) => format!("k8s:{}/{}", self.pod, c),
            (None, None) => format!("k8s:{}", self.pod),
        }
    }
}
