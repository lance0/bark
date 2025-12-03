//! Log source abstraction and implementations.
//!
//! Provides a unified `LogSource` trait with implementations for:
//! - Local files (via `tail -F`)
//! - Docker containers (via `docker logs -f`)
//! - Kubernetes pods (via `kubectl logs -f`)
//! - Remote files via SSH (via `ssh ... tail -F`)

pub mod docker;
pub mod file;
pub mod k8s;
pub mod manager;
pub mod ssh;

use crate::app::LogLine;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Describes how a log source is configured
#[derive(Clone, Debug)]
pub enum LogSourceType {
    File {
        path: PathBuf,
    },
    Docker {
        container: String,
    },
    K8s {
        pod: String,
        namespace: Option<String>,
        container: Option<String>,
    },
    Ssh {
        host: String,
        path: String,
    },
}

impl LogSourceType {
    pub fn name(&self) -> String {
        match self {
            LogSourceType::File { path } => path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string()),
            LogSourceType::Docker { container } => format!("docker:{}", container),
            LogSourceType::K8s {
                pod,
                namespace,
                container,
            } => match (namespace, container) {
                (Some(ns), Some(c)) => format!("k8s:{}/{}/{}", ns, pod, c),
                (Some(ns), None) => format!("k8s:{}/{}", ns, pod),
                (None, Some(c)) => format!("k8s:{}/{}", pod, c),
                (None, None) => format!("k8s:{}", pod),
            },
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

/// Event with source identification for multi-source merging
pub struct SourcedLogEvent {
    pub source_id: usize,
    pub event: LogEvent,
}

/// Trait for log sources
#[async_trait::async_trait]
pub trait LogSource: Send + Sync {
    /// Start streaming log events
    async fn stream(&self) -> mpsc::Receiver<LogEvent>;

    /// Get the display name for this source
    #[allow(dead_code)]
    fn name(&self) -> String;
}
