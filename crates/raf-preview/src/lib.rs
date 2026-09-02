//! Camera-backed RAF preview orchestration.
//!
//! This crate is intentionally transport-agnostic. It models the recoverable
//! workflow (stage RAF → upload → apply temporary recipe → render → download
//! JPEG → cleanup), but does not emit a vendor PTP transaction itself. A model
//! adapter may implement `RafPreviewTransport` only after it has a separately
//! validated capability record and hardware recovery checklist.

use camera_core::UsbId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RafPreviewRequest {
    pub source_name: String,
    pub source_bytes: u64,
    pub recipe_id: String,
    pub target: Option<RafPreviewTarget>,
}

/// Exact camera identity required before a model adapter may communicate with a
/// camera. Offline staging purposefully leaves this unset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RafPreviewTarget {
    pub usb_id: UsbIdText,
    pub model: String,
    pub firmware: String,
}

/// Serializable counterpart to `UsbId`, kept free of hardware handles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UsbIdText(pub String);

impl From<UsbId> for UsbIdText {
    fn from(id: UsbId) -> Self {
        Self(id.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RafPreviewState {
    Staged,
    Uploading,
    ApplyingRecipe,
    Rendering,
    DownloadingJpeg,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RafPreviewProgress {
    pub state: RafPreviewState,
    pub recovery_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RafPreviewError {
    pub stage: RafPreviewState,
    pub detail: String,
    pub cleanup_attempted: bool,
}

impl std::fmt::Display for RafPreviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "RAF preview failed during {:?}: {}; cleanup attempted: {}",
            self.stage, self.detail, self.cleanup_attempted
        )
    }
}

impl std::error::Error for RafPreviewError {}

/// Implemented by a model-specific PTP adapter once its upload, conversion,
/// download, and abort messages are hardware-validated. The current application
/// deliberately ships no implementation of this trait.
pub trait RafPreviewTransport {
    type Error: std::fmt::Display;

    fn upload_raf(&mut self, request: &RafPreviewRequest) -> Result<(), Self::Error>;
    fn apply_temporary_recipe(&mut self, request: &RafPreviewRequest) -> Result<(), Self::Error>;
    fn start_conversion(&mut self, request: &RafPreviewRequest) -> Result<(), Self::Error>;
    fn download_preview_jpeg(&mut self, request: &RafPreviewRequest) -> Result<(), Self::Error>;
    fn cleanup_temporary_conversion(
        &mut self,
        request: &RafPreviewRequest,
    ) -> Result<(), Self::Error>;
}

/// One camera-side operation in the fixed preview transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewOperation {
    Upload,
    ApplyRecipe,
    Render,
    DownloadJpeg,
}

impl PreviewOperation {
    fn state(self) -> RafPreviewState {
        match self {
            Self::Upload => RafPreviewState::Uploading,
            Self::ApplyRecipe => RafPreviewState::ApplyingRecipe,
            Self::Render => RafPreviewState::Rendering,
            Self::DownloadJpeg => RafPreviewState::DownloadingJpeg,
        }
    }

    fn execute<T: RafPreviewTransport>(
        self,
        transport: &mut T,
        request: &RafPreviewRequest,
    ) -> Result<(), T::Error> {
        match self {
            Self::Upload => transport.upload_raf(request),
            Self::ApplyRecipe => transport.apply_temporary_recipe(request),
            Self::Render => transport.start_conversion(request),
            Self::DownloadJpeg => transport.download_preview_jpeg(request),
        }
    }
}

/// Validate the part of a RAF preview workflow that is safe to perform offline.
pub fn stage_request(request: RafPreviewRequest) -> Result<RafPreviewProgress, RafPreviewError> {
    if !request.source_name.to_ascii_lowercase().ends_with(".raf") {
        return Err(RafPreviewError {
            stage: RafPreviewState::Staged,
            detail: "source must be a .RAF file".to_string(),
            cleanup_attempted: false,
        });
    }
    if request.source_bytes == 0 {
        return Err(RafPreviewError {
            stage: RafPreviewState::Staged,
            detail: "RAF source is empty".to_string(),
            cleanup_attempted: false,
        });
    }
    if request.recipe_id.trim().is_empty() {
        return Err(RafPreviewError {
            stage: RafPreviewState::Staged,
            detail: "a local Recipe must be selected".to_string(),
            cleanup_attempted: false,
        });
    }
    Ok(RafPreviewProgress {
        state: RafPreviewState::Staged,
        recovery_action: Some(
            "No camera connection was opened. A validated adapter is required before transfer."
                .to_string(),
        ),
    })
}

/// Execute a model adapter through a fixed cleanup path. A failed upload,
/// render, or JPEG download cannot be reported as complete; cleanup is always
/// attempted after the first operation starts.
pub fn run_preview<T: RafPreviewTransport>(
    transport: &mut T,
    request: &RafPreviewRequest,
) -> Result<RafPreviewProgress, RafPreviewError> {
    stage_request(request.clone())?;
    if request.target.is_none() {
        return Err(RafPreviewError {
            stage: RafPreviewState::Staged,
            detail: "an exact USB ID, model, and firmware capability record is required before camera transfer".to_string(),
            cleanup_attempted: false,
        });
    }
    let operations = [
        PreviewOperation::Upload,
        PreviewOperation::ApplyRecipe,
        PreviewOperation::Render,
        PreviewOperation::DownloadJpeg,
    ];
    for operation in operations {
        if let Err(error) = operation.execute(transport, request) {
            let cleanup_attempted = transport.cleanup_temporary_conversion(request).is_ok();
            return Err(RafPreviewError {
                stage: operation.state(),
                detail: error.to_string(),
                cleanup_attempted,
            });
        }
    }
    if let Err(error) = transport.cleanup_temporary_conversion(request) {
        return Err(RafPreviewError {
            stage: RafPreviewState::Completed,
            detail: format!("preview JPEG completed but cleanup failed: {error}"),
            cleanup_attempted: true,
        });
    }
    Ok(RafPreviewProgress {
        state: RafPreviewState::Completed,
        recovery_action: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RafPreviewRequest {
        RafPreviewRequest {
            source_name: "sample.RAF".to_string(),
            source_bytes: 42,
            recipe_id: "recipe-1".to_string(),
            target: Some(RafPreviewTarget {
                usb_id: UsbIdText("04CB:030C".to_string()),
                model: "X-M5".to_string(),
                firmware: "1.30".to_string(),
            }),
        }
    }

    #[derive(Default)]
    struct MockTransport {
        fail_render: bool,
        cleanup_calls: u8,
    }

    impl RafPreviewTransport for MockTransport {
        type Error = &'static str;

        fn upload_raf(&mut self, _: &RafPreviewRequest) -> Result<(), Self::Error> {
            Ok(())
        }
        fn apply_temporary_recipe(&mut self, _: &RafPreviewRequest) -> Result<(), Self::Error> {
            Ok(())
        }
        fn start_conversion(&mut self, _: &RafPreviewRequest) -> Result<(), Self::Error> {
            if self.fail_render {
                Err("render rejected")
            } else {
                Ok(())
            }
        }
        fn download_preview_jpeg(&mut self, _: &RafPreviewRequest) -> Result<(), Self::Error> {
            Ok(())
        }
        fn cleanup_temporary_conversion(
            &mut self,
            _: &RafPreviewRequest,
        ) -> Result<(), Self::Error> {
            self.cleanup_calls += 1;
            Ok(())
        }
    }

    #[test]
    fn staging_never_opens_a_camera_or_accepts_non_raf_input() {
        let mut invalid = request();
        invalid.source_name = "sample.jpg".to_string();
        let error = stage_request(invalid).unwrap_err();
        assert_eq!(error.stage, RafPreviewState::Staged);
        assert!(!error.cleanup_attempted);
    }

    #[test]
    fn failed_render_runs_cleanup_before_returning_an_error() {
        let mut transport = MockTransport {
            fail_render: true,
            ..Default::default()
        };
        let error = run_preview(&mut transport, &request()).unwrap_err();
        assert_eq!(error.stage, RafPreviewState::Rendering);
        assert!(error.cleanup_attempted);
        assert_eq!(transport.cleanup_calls, 1);
    }

    #[test]
    fn camera_transfer_requires_an_exact_capability_target() {
        let mut transport = MockTransport::default();
        let mut offline_request = request();
        offline_request.target = None;
        let error = run_preview(&mut transport, &offline_request).unwrap_err();
        assert_eq!(error.stage, RafPreviewState::Staged);
        assert!(!error.cleanup_attempted);
        assert_eq!(transport.cleanup_calls, 0);
    }

    #[test]
    fn successful_preview_also_cleans_up_the_temporary_conversion() {
        let mut transport = MockTransport::default();
        let result = run_preview(&mut transport, &request()).unwrap();
        assert_eq!(result.state, RafPreviewState::Completed);
        assert_eq!(transport.cleanup_calls, 1);
    }
}
