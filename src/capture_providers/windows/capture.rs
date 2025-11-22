use std::{mem::MaybeUninit, sync::Arc};

use tokio::{
    io::Empty,
    sync::{RwLock, mpsc::error::TryRecvError},
};
use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::*,
        DirectX::{Direct3D11::*, DirectXPixelFormat},
    },
    Win32::Graphics::Direct3D11::*,
    core::*,
};

use crate::capture_providers::{
    CaptureError, CaptureResult,
    shared::{Frame, Vector2},
    windows::{WindowsCaptureStream, d3d11_utils::read_texture},
};

type Result<T> = CaptureResult<T>;

pub struct WindowsCaptureProvider {
    device: IDirect3DDevice,
    frame_pool: Direct3D11CaptureFramePool,
    capture_item: GraphicsCaptureItem,
    session: Option<GraphicsCaptureSession>,
    staging_texture: Arc<RwLock<Option<ID3D11Texture2D>>>,

    stream_close_tx: tokio::sync::mpsc::Sender<i64>,
    stream_close_rx: tokio::sync::mpsc::Receiver<i64>,

    capturing: bool,
}

impl WindowsCaptureProvider {
    const FRAME_COUNT: i32 = 2;
    const PIXEL_FORMAT: DirectXPixelFormat = DirectXPixelFormat::B8G8R8A8UIntNormalized;
    const BYTES_PER_PIXEL: usize = 4; // Match the pixel format, 4 bytes per pixel for BGRA8

    /// Must be called from the main thread
    pub fn new_from_main_thread(
        device: IDirect3DDevice,
        item: GraphicsCaptureItem,
    ) -> CaptureResult<Self> {
        let size = item.Size()?;
        let frame_pool = Direct3D11CaptureFramePool::Create(
            &device,
            Self::PIXEL_FORMAT,
            Self::FRAME_COUNT,
            size,
        )?;

        // Disabled for testing reasons, uncomment later
        //session.SetIsCursorCaptureEnabled(true)?;
        //session.SetIsBorderRequired(false)?;

        let (stream_close_tx, stream_close_rx) = tokio::sync::mpsc::channel(32);
        Ok(Self {
            device: device,
            frame_pool,
            capture_item: item,
            session: None,
            staging_texture: Arc::new(RwLock::new(None)),
            stream_close_tx,
            stream_close_rx,
            capturing: false,
        })
    }

    /// Creates a new stream for receiving frames.
    pub fn create_stream(&self) -> Result<WindowsCaptureStream> {
        let (tx, rx) = tokio::sync::mpsc::channel(2);

        // We can't send self raw to the closure, so we need to just copy the staging texture which is inside an Arc Mutex.
        let staging_tex_ptr = self.staging_texture.clone();

        let frame_arrived_token =
            self.frame_pool
                .FrameArrived(&TypedEventHandler::new(move |sender, _args| {
                    let sender = match &*sender {
                        Some(sender) => sender,
                        None => {
                            eprintln!("No sender provided with FrameArrived!");
                            return Ok(());
                        }
                    };
                    let sender: &Direct3D11CaptureFramePool = sender;

                    let frame = match sender.TryGetNextFrame() {
                        Ok(frame) => frame,
                        Err(err) => {
                            eprintln!("Failed to get next frame: {}", err);
                            return Ok(());
                        }
                    };

                    let surface = frame.Surface()?;
                    let texture: ID3D11Texture2D = surface.cast()?;
                    let size = frame.ContentSize()?;
                    println!(
                        "Frame: {} x {}, ptr={:?}",
                        size.Width,
                        size.Height,
                        texture.as_raw()
                    );

                    let desc = unsafe {
                        let mut d = std::mem::zeroed::<D3D11_TEXTURE2D_DESC>();
                        texture.GetDesc(&mut d);
                        d.BindFlags = 0;
                        d.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                        d.Usage = D3D11_USAGE_STAGING;
                        d
                    };

                    let device = unsafe { texture.GetDevice()? };

                    let staging_tex = { staging_tex_ptr.blocking_read().clone() };
                    let staging_tex = match staging_tex {
                        Some(staging_tex) => staging_tex,
                        None => unsafe {
                            let mut tex = MaybeUninit::<Option<ID3D11Texture2D>>::uninit();
                            device.CreateTexture2D(&desc, None, Some(tex.as_mut_ptr()))?;

                            let new_staging_tex = tex
                                .assume_init()
                                .expect("Failed to create staging texture!");
                            *staging_tex_ptr.blocking_write() = Some(new_staging_tex);
                            staging_tex.clone().unwrap()
                        },
                    };

                    let context = unsafe { device.GetImmediateContext()? };

                    let data = read_texture::<{ Self::BYTES_PER_PIXEL }>(
                        &context,
                        texture,
                        staging_tex,
                        &desc,
                    )
                    .expect("Unable to read texture into byte array!");

                    let send_result = tx.blocking_send(Frame {
                        data,
                        size: Vector2 {
                            x: size.Width,
                            y: size.Height,
                        },
                        timestamp: frame.SystemRelativeTime()?.Duration,
                        dirty_rects: frame.DirtyRegions()?.into_iter().map(Into::into).collect(),
                    });
                    if let Err(err) = send_result {
                        eprintln!("Could not send frame! {}", err);
                    }

                    Ok(())
                }))?;

        let stream =
            WindowsCaptureStream::new(self.stream_close_tx.clone(), rx, frame_arrived_token);

        Ok(stream)
    }

    pub fn start_capture(&mut self) -> Result<()> {
        if self.capturing {
            return Err(CaptureError::AlreadyCapturing);
        }

        let session = match &self.session {
            Some(session) => session,
            None => {
                let new_session = self.frame_pool.CreateCaptureSession(&self.capture_item)?;
                self.session = Some(new_session);
                self.session.as_ref().unwrap()
            }
        };

        session.StartCapture()?;
        self.capturing = true;

        Ok(())
    }

    pub fn stop_capture(&mut self) -> Result<()> {
        if !self.capturing {
            return Err(CaptureError::NotCapturing);
        }

        self.session.take(); // Drop the old session
        self.capturing = false;

        Ok(())
    }

    pub fn poll_stream_closer(&mut self) -> Result<()> {
        loop {
            let next = self.stream_close_rx.try_recv();
            match next {
                Ok(token) => {
                    self.unregister_frame_arrived(token)?;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        Ok(())
    }

    pub(super) fn unregister_frame_arrived(&self, token: i64) -> Result<()> {
        self.frame_pool.RemoveFrameArrived(token)?;
        Ok(())
    }
}

impl Drop for WindowsCaptureProvider {
    fn drop(&mut self) {
        self.stop_capture().ok();
    }
}
