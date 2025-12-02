pub mod file;
pub mod docker;
pub mod k8s;
pub mod ssh;

use std::path::PathBuf;
use tokio::sync::mpsc;
use crate::app::LogLine;

/// Describes how a log source is configured
#[derive(Clone, Debug)]
pub enum LogSourceType {
    File { path: PathBuf },
    Docker { container: String },
    K8s { pod: String, namespace: Option<String>, container: Option<String> },
    Ssh { host: String, path: String },
}

impl LogSourceType {
    pub fn name(&self) -> String {
        match self {
            LogSourceType::File { path } => {
                path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            }
            LogSourceType::Docker { container } => format!("docker:{}", container),
            LogSourceType::K8s { pod, namespace, container } => {
                match (namespace, container) {
                    (Some(ns), Some(c)) => format!("k8s:{}/{}/{}", ns, pod, c),
                    (Some(ns), None) => format!("k8s:{}/{}", ns, pod),
                    (None, Some(c)) => format!("k8s:{}/{}", pod, c),
                    (None, None) => format!("k8s:{}", pod),
                }
            }
            LogSourceType::Ssh { host, path } => format!("ssh:{}:{}", host, path),
        }
    }
}

/// Events emitted by log sources
pub enum LogEvent {
    Line(LogLine),
    Error(String),
    EndOfStream,
}

/// Trait for log sources
#[async_trait::async_trait]
pub trait LogSource: Send + Sync {
    /// Start streaming log events
    async fn stream(&self) -> mpsc::Receiver<LogEvent>;

    /// Get the display name for this source
    fn name(&self) -> String;
}
