use windows::Graphics::{Capture::GraphicsCaptureItem, DirectX::Direct3D11::IDirect3DDevice};

use crate::capture_providers::{
    CaptureError,
    windows::{
        capture::WindowsCaptureProvider,
        d3d11_utils::{create_d3d_device, native_to_winrt_d3d11device, user_pick_capture_item},
    },
};

type Result<T> = std::result::Result<T, BuilderError>;

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Missing device")]
    MissingDevice,
    #[error("Missing capture item")]
    MissingCaptureItem,
    #[error("Initialization error: {0}")]
    InitializationError(#[from] CaptureError),
    #[error("Windows error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

pub struct WindowsCaptureProviderBuilder {
    device: Option<IDirect3DDevice>,
    capture_item: Option<GraphicsCaptureItem>,
}

impl WindowsCaptureProviderBuilder {
    pub fn new() -> Self {
        WindowsCaptureProviderBuilder {
            device: None,
            capture_item: None,
        }
    }

    pub fn with_device(mut self, device: IDirect3DDevice) -> Self {
        self.device = Some(device);
        self
    }

    pub fn with_default_device(mut self) -> Result<Self> {
        let d3d_device = create_d3d_device()?;
        let winrt_device = native_to_winrt_d3d11device(&d3d_device)?;
        self.device = Some(winrt_device);
        Ok(self)
    }

    pub async fn with_user_picked_capture_item(mut self) -> Result<Self> {
        let item_result = user_pick_capture_item().await;
        self.capture_item = Some(item_result?);
        Ok(self)
    }

    /// Must be called from the main thread.
    pub fn build_from_main_thread(self) -> Result<WindowsCaptureProvider> {
        let device = self.device.ok_or(BuilderError::MissingDevice)?;
        let item = self.capture_item.ok_or(BuilderError::MissingCaptureItem)?;
        WindowsCaptureProvider::new_from_main_thread(device, item).map_err(Into::into)
    }
}
