use std::sync::{Arc, Mutex, MutexGuard};

mod application;
mod bencode;
mod destination;
mod engine;
mod media;
mod meta;
mod persistence;

pub use application::{
    MachineCapabilities, MachineCommandReceipt, MachineDownload, MachineDownloadCommand,
    MachineDownloadState, MachineFileOrganization, MachineFolder, MachineFolderRequest,
    MachineFormat, MachineGrouping, MachineHoldCause, MachineLayout, MachineMediaItem,
    MachineMediaPolicy, MachineMediaRequest, MachinePlatformEffect, MachineSessionState,
    MachineSnapshot, MachineSort, MachineSource, MachineTorrentFile, MachineTorrentRequest,
    MachineWorkspace, MachineWorkspaceIcon, MachineWorkspaceIconKind, MachineWorkspaceRequest,
    OpulMachineConfig, OpulStateMachine, StateMachineError,
};

pub use destination::DownloadContentLayout;
pub use engine::EngineConfig;

pub use media::{
    GuidedCandidate, GuidedDownloadSelection, GuidedExtractionSession, GuidedHttpHeader,
    GuidedMediaKind, GuidedObservation, GuidedObservationSource, MediaDownloadOptions, MediaEngine,
    MediaEngineConfig, MediaFormat, MediaProbe, MediaProbeItem, MediaToolStatus,
    MediaTransferSnapshot, MediaTransferState, classify_guided_url,
};

uniffi::setup_scaffolding!();

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct Summary {
    pub count: u64,
    pub total: i64,
    pub mean: f64,
}

#[derive(Debug, thiserror::Error, uniffi::Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("the running total overflowed")]
    TotalOverflow,
    #[error("{reason}")]
    InvalidTorrent { reason: String },
    #[error("{reason}")]
    InvalidMagnet { reason: String },
    #[error("{reason}")]
    Engine { reason: String },
    #[error("{reason}")]
    InvalidUrl { reason: String },
    #[error("{reason}")]
    InvalidConfiguration { reason: String },
    #[error("{reason}")]
    DependencyUnavailable { reason: String },
    #[error("{reason}")]
    Extraction { reason: String },
}

pub fn summarize(samples: &[i64]) -> Result<Summary, CoreError> {
    let mut total: i64 = 0;
    for sample in samples {
        total = total.checked_add(*sample).ok_or(CoreError::TotalOverflow)?;
    }
    let count = samples.len() as u64;
    let mean = if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    };
    Ok(Summary { count, total, mean })
}

#[derive(uniffi::Object)]
pub struct Session {
    samples: Mutex<Vec<i64>>,
}

#[uniffi::export]
impl Session {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            samples: Mutex::new(Vec::new()),
        })
    }

    pub fn record(&self, value: i64) {
        self.samples().push(value);
    }

    pub fn summarize(&self) -> Result<Summary, CoreError> {
        summarize(&self.samples())
    }
}

impl Session {
    fn samples(&self) -> MutexGuard<'_, Vec<i64>> {
        self.samples
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreError, Session, summarize};

    #[test]
    fn empty_input_summarizes_to_zero() {
        let summary = summarize(&[]).unwrap();
        assert_eq!(summary.count, 0);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.mean, 0.0);
    }

    #[test]
    fn summarize_sums_and_averages() {
        let summary = summarize(&[2, 4, 6]).unwrap();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.total, 12);
        assert_eq!(summary.mean, 4.0);
    }

    #[test]
    fn summarize_reports_overflow() {
        let error = summarize(&[i64::MAX, 1]).unwrap_err();
        assert_eq!(error, CoreError::TotalOverflow);
    }

    #[test]
    fn session_records_then_summarizes() {
        let session = Session::new();
        session.record(10);
        session.record(30);
        let summary = session.summarize().unwrap();
        assert_eq!(summary.count, 2);
        assert_eq!(summary.total, 40);
        assert_eq!(summary.mean, 20.0);
    }
}
