pub mod shared;
pub mod windows;

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error(transparent)]
    WindowsCaptureError(#[from] windows::error::WindowsCaptureError),
}

type CaptureResult<T> = std::result::Result<T, CaptureError>;

//TODO: use this instead of specifying the windows one. This also allows for other platforms to be added in the future.
trait CaptureProvider {}
