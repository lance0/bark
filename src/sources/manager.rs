//! Manages multiple log source streams, merging them into a single receiver.

use super::{LogSource, SourcedLogEvent};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Manages multiple log sources and merges their streams
pub struct SourceManager {
    tx: mpsc::Sender<SourcedLogEvent>,
    handles: Vec<JoinHandle<()>>,
}

impl SourceManager {
    /// Create a new source manager and return the merged receiver
    pub fn new(buffer_size: usize) -> (Self, mpsc::Receiver<SourcedLogEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let manager = Self {
            tx,
            handles: Vec::new(),
        };
        (manager, rx)
    }

    /// Add a source and start streaming from it
    pub async fn add_source(&mut self, source_id: usize, source: Box<dyn LogSource>) {
        let tx = self.tx.clone();
        let mut source_rx = source.stream().await;

        let handle = tokio::spawn(async move {
            while let Some(event) = source_rx.recv().await {
                let sourced = SourcedLogEvent { source_id, event };
                if tx.send(sourced).await.is_err() {
                    break; // Receiver dropped
                }
            }
        });

        self.handles.push(handle);
    }

    /// Get the number of active sources
    #[allow(dead_code)]
    pub fn source_count(&self) -> usize {
        self.handles.len()
    }

    /// Shutdown all source streams
    #[allow(dead_code)]
    pub fn shutdown(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}
