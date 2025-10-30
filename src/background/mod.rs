use chrono::Utc;
use futures::StreamExt;
use scheduler::{SchedulerEventStream, SchedulerQueueEvent};

use crate::database::{
    DbPool,
    secrets::{delete_excess_secret_versions, delete_scheduled_secrets},
};

mod scheduler;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BackgroundEvent {
    /// Task to purge scheduled secrets
    PurgeDeletedSecrets,

    /// Task to prune the secrets with versions in excess of 100 versions that are
    /// over 24h old
    PurgeExcessSecrets,
}

pub async fn perform_background_tasks(db: DbPool) {
    let events = vec![
        SchedulerQueueEvent {
            event: BackgroundEvent::PurgeDeletedSecrets,
            interval: 60 * 60,
        },
        SchedulerQueueEvent {
            event: BackgroundEvent::PurgeExcessSecrets,
            interval: 60 * 60,
        },
    ];

    let mut events = SchedulerEventStream::new(events);

    while let Some(event) = events.next().await {
        match event {
            BackgroundEvent::PurgeDeletedSecrets => {
                tracing::debug!("performing background purge for presigned tasks");
                let now = Utc::now();
                if let Err(error) = delete_scheduled_secrets(&db, now).await {
                    tracing::error!(?error, "failed to performed scheduled secrets deletion")
                }
            }

            BackgroundEvent::PurgeExcessSecrets => {
                tracing::debug!("performing background deletion for secret version limits");
                if let Err(error) = delete_excess_secret_versions(&db).await {
                    tracing::error!(
                        ?error,
                        "failed to performed background deletion for secret version limits"
                    )
                }
            }
        }
    }
}
