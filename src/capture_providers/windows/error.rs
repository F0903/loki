pub type Result<T> = std::result::Result<T, WindowsCaptureError>;

#[derive(Debug, thiserror::Error)]
pub enum WindowsCaptureError {
    #[error("Already capturing")]
    AlreadyCapturing,
    #[error("Not capturing")]
    NotCapturing,
    #[error("No frame pool available")]
    NoFramePool,
    #[error("No capture item available")]
    NoCaptureItem,
    #[error("Failed to set min update interval: {0}")]
    SetMinUpdateIntervalFailed(windows_core::Error),
    #[error("Unknown Windows error: {0}")]
    UnknownWindowsError(#[from] windows_core::Error),
}
