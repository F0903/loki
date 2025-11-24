use crate::{
    capture_providers::{
        shared::Frame,
        windows::{WindowsCaptureProvider, WindowsCaptureStream, user_pick_capture_item},
    },
    utils::image_utils::ensure_image_rgba,
};
use iced::{
    Element, Length, Program, Subscription, Task, executor,
    widget::{self, button, column, container, row},
    window,
};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::sync::{Mutex, OwnedMutexGuard};
use windows::Graphics::Capture::GraphicsCaptureItem;

#[derive(Debug, Clone)]
struct FrameReceiverSubData {
    capture: Arc<Mutex<WindowsCaptureProvider>>,
    stream_name: &'static str,
}

impl Hash for FrameReceiverSubData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.stream_name.hash(state);
    }
}

pub enum Message {
    StartCapture,
    CaptureStarted,
    StopCapture,
    CaptureStopped,

    WindowsUserPickedCaptureItem(windows::core::Result<GraphicsCaptureItem>),
    FrameReceived(Frame),

    CaptureProviderAction(
        OwnedMutexGuard<WindowsCaptureProvider>,
        Box<dyn FnOnce(OwnedMutexGuard<WindowsCaptureProvider>) + Send>,
    ),

    WindowOpened(window::Id),
    WindowIdFetched(u64),

    Error(String),
}

#[derive(Debug)]
pub(crate) struct MutableState {
    active_window_handle: Option<u64>,
    capturing: bool,
    latest_frame: std::sync::Mutex<Option<Frame>>,
}

#[derive(Debug)]
pub(crate) struct App {
    capture: Arc<Mutex<WindowsCaptureProvider>>,
}

impl App {
    const APP_TITLE: &'static str = "loki";

    pub fn new(
        capture: Arc<Mutex<WindowsCaptureProvider>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { capture })
    }

    fn create_frame_receiver_subscription(data: &FrameReceiverSubData) -> WindowsCaptureStream {
        data.capture
            .blocking_lock()
            .create_stream()
            .expect("Failed to create stream!")
    }

    pub fn run(self) -> crate::Result<()> {
        // #[cfg(debug_assertions)]
        // iced_debug::init(iced_debug::Metadata {
        //     name: Self::APP_TITLE,
        //     theme: None,
        //     can_time_travel: false,
        // });

        // #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        // let program = iced_devtools::attach(self);

        iced_winit::run(self)?;
        Ok(())
    }
}

impl Program for App {
    type State = MutableState;
    type Message = Message;
    type Theme = iced::Theme;
    type Renderer = iced::Renderer;
    type Executor = executor::Default;

    fn name() -> &'static str {
        Self::APP_TITLE
    }

    fn settings(&self) -> iced::Settings {
        iced::Settings::default()
    }

    fn window(&self) -> Option<window::Settings> {
        Some(window::Settings::default())
    }

    fn boot(&self) -> (Self::State, Task<Self::Message>) {
        (
            MutableState {
                latest_frame: std::sync::Mutex::new(None),
                capturing: false,
                active_window_handle: None,
            },
            Task::none(),
        )
    }

    fn subscription(&self, state: &Self::State) -> Subscription<Message> {
        let mut subscriptions = vec![];

        if state.capturing {
            subscriptions.push(
                Subscription::<Frame>::run_with(
                    FrameReceiverSubData {
                        capture: self.capture.clone(),
                        stream_name: "frame-receiver",
                    },
                    Self::create_frame_receiver_subscription,
                )
                .map(Message::FrameReceived),
            );
        }
        subscriptions.push(iced::window::open_events().map(Message::WindowOpened));

        Subscription::batch(subscriptions)
    }

    fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::WindowOpened(id) => {
                let fetch_id_task =
                    iced::window::raw_id::<Message>(id).map(Message::WindowIdFetched);
                fetch_id_task
            }
            Message::WindowIdFetched(id) => {
                state.active_window_handle = Some(id);
                Task::none()
            }
            Message::StartCapture => {
                let window_handle = match state.active_window_handle {
                    Some(handle) => handle,
                    None => {
                        return Task::done(Message::Error(format!("No active window handle")));
                    }
                };

                // Returned future completes when the user picks a capture item
                let user_pick_future = match user_pick_capture_item(window_handle) {
                    Ok(item) => item,
                    Err(err) => {
                        return Task::done(Message::Error(format!(
                            "Failed to pick capture item: {}",
                            err
                        )));
                    }
                };

                Task::future(user_pick_future).map(Message::WindowsUserPickedCaptureItem)
            }
            Message::CaptureStarted => {
                state.capturing = true;
                Task::none()
            }
            Message::StopCapture => {
                let capture_arc = self.capture.clone();
                Task::perform(
                    async move { capture_arc.lock_owned().await },
                    |mut capture| {
                        if let Err(err) = capture.stop_capture() {
                            eprintln!("Failed to stop capture: {}", err);
                        }
                        if let Err(err) = capture.poll_stream_closer() {
                            eprintln!("Failed to poll stream closer: {}", err);
                        }

                        Message::CaptureStopped
                    },
                )
            }
            Message::CaptureStopped => {
                state.capturing = false;
                Task::none()
            }
            Message::WindowsUserPickedCaptureItem(capture_item_result) => {
                let capture_item = match capture_item_result {
                    Ok(item) => item,
                    Err(err) => {
                        return Task::done(Message::Error(format!(
                            "Failed to pick capture item: {}",
                            err
                        )));
                    }
                };

                //TODO: Implement async waiting for capture lock, but then still call methods on it on the main thread.
                let capture_arc = self.capture.clone();
                let mut capture = capture_arc.blocking_lock_owned();

                if let Err(err) = capture.set_capture_item(capture_item) {
                    return Task::done(Message::Error(format!(
                        "Failed to set capture item: {}",
                        err
                    )));
                }
                if let Err(err) = capture.start_capture() {
                    return Task::done(Message::Error(format!("Failed to start capture: {}", err)));
                }

                Task::done(Message::CaptureStarted)
            }
            Message::FrameReceived(frame) => {
                match state.latest_frame.get_mut() {
                    Ok(lock) => *lock = Some(frame),
                    Err(err) => eprintln!("Failed to get mutable lock for latest frame: {}", err),
                };
                Task::none()
            }
            Message::Error(err) => {
                eprintln!("Error: {}", err);
                Task::none()
            }
        }
    }

    fn view<'a>(
        &self,
        state: &'a Self::State,
        _window: window::Id,
    ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
        let control_buttons: Element<'a, Self::Message, Self::Theme, Self::Renderer> = container(
            row([
                button("Start Capture")
                    .on_press(Message::StartCapture)
                    .into(),
                button("Stop Capture").on_press(Message::StopCapture).into(),
            ])
            .spacing(10),
        )
        .padding(10)
        .center_x(Length::Fill)
        .into();

        let screen_share_preview: Element<'a, Self::Message, Self::Theme, Self::Renderer> =
            (|| {
                let mut frame_lock = match state.latest_frame.lock() {
                    Ok(frame_lock) => frame_lock,
                    Err(err) => {
                        let err_msg = format!("Failed to get frame lock: {}", err);
                        eprintln!("{}", err_msg);
                        return widget::text(err_msg).into();
                    }
                };

                match &mut *frame_lock {
                    Some(frame) => {
                        if let Err(err) = ensure_image_rgba(&mut frame.data, &mut frame.format) {
                            let err_msg = format!("Frame format conversion failed: {}", err);
                            eprintln!(" {}", err_msg);
                            return widget::text(err_msg).into();
                        }

                        widget::image(widget::image::Handle::from_rgba(
                            frame.size.x as u32,
                            frame.size.y as u32,
                            frame.data.clone(),
                        ))
                        .into()
                    }
                    None => widget::text("No preview available.").into(),
                }
            })();

        column([control_buttons, screen_share_preview]).into()
    }
}
