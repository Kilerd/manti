use crate::db::DatabaseService;
use crate::models::usage::CreateUsage;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, warn};

/// Async usage recorder that buffers writes for non-blocking operation
pub struct UsageRecorder {
    sender: mpsc::UnboundedSender<CreateUsage>,
}

impl UsageRecorder {
    /// Create a new usage recorder with a background write task
    pub fn new(db: Arc<DatabaseService>) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<CreateUsage>();

        // Spawn background task for writing usage records
        tokio::spawn(async move {
            while let Some(usage) = receiver.recv().await {
                match db.record_usage(usage).await {
                    Ok(_) => {
                        // Successfully recorded
                    }
                    Err(e) => {
                        error!("Failed to record usage: {}", e);
                        // In production, might want to:
                        // 1. Retry with backoff
                        // 2. Write to a dead letter queue
                        // 3. Alert monitoring system
                    }
                }
            }
            warn!("Usage recorder channel closed");
        });

        Self { sender }
    }

    /// Record usage asynchronously (non-blocking)
    /// Returns immediately without waiting for DB write
    pub fn record(&self, usage: CreateUsage) {
        if let Err(e) = self.sender.send(usage) {
            error!("Failed to queue usage record: {}", e);
        }
    }

    /// Check if the recorder is still accepting records
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl Clone for UsageRecorder {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}
