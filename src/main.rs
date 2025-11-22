use std::sync::Arc;

use tokio::sync::Mutex;

use crate::capture_providers::CaptureError;

mod capture_providers;
mod ui;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Capture error: {0}")]
    CaptureError(#[from] capture_providers::CaptureError),
    #[error("Windows capture builder error: {0}")]
    WindowsCaptureBuilderError(#[from] capture_providers::windows::BuilderError),
    #[error("Windows capture error: {0}")]
    WindowsError(#[from] windows_core::Error),
    #[error("UI error: {0}")]
    UiError(#[from] iced::Error),
}

fn main() -> Result<(), Error> {
    let main_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| CaptureError::FailedToInitialize)
        .unwrap();

    let windows_capture = main_rt.block_on(async {
        capture_providers::windows::WindowsCaptureProviderBuilder::new()
            .with_default_device()?
            .with_user_picked_capture_item()
            .await?
            .build_from_main_thread()
    })?;
    let windows_capture = Arc::new(Mutex::new(windows_capture));

    ui::run("loki", windows_capture)?;
    Ok(())
}
