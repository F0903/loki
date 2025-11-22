use std::sync::Arc;

use iced::{
    Subscription, Task,
    widget::{button, column},
};
use tokio::sync::Mutex;

use crate::capture_providers::{shared::Frame, windows::WindowsCaptureProvider};

#[derive(Debug, Clone)]
enum Message {
    StartCapture,
    StopCapture,
    FrameReceived(Frame),
}

struct State {
    capture: Arc<Mutex<WindowsCaptureProvider>>,
    latest_frame: Option<Frame>,
}

impl State {
    fn new(capture_provider: Arc<Mutex<WindowsCaptureProvider>>) -> Self {
        State {
            capture: capture_provider,
            latest_frame: None,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let stream = self
            .capture
            .blocking_lock()
            .create_stream()
            .expect("Failed to create stream!");
        Subscription::run_with_id("frame-receiver", stream).map(Message::FrameReceived)
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::StartCapture => {
                let mut capture = self.capture.blocking_lock();
                if let Err(err) = capture.start_capture() {
                    eprintln!("Failed to start capture: {}", err);
                }
            }
            Message::StopCapture => {
                let mut capture = self.capture.blocking_lock();
                if let Err(err) = capture.stop_capture() {
                    eprintln!("Failed to stop capture: {}", err);
                }
                if let Err(err) = capture.poll_stream_closer() {
                    eprintln!("Failed to poll stream closer: {}", err);
                }
            }
            Message::FrameReceived(frame) => {
                self.latest_frame = Some(frame);
            }
        }
    }

    pub fn view(&'_ self) -> iced::Element<'_, Message> {
        column([
            button("Start Capture")
                .on_press(Message::StartCapture)
                .into(),
            button("Stop Capture").on_press(Message::StopCapture).into(),
        ])
        .into()
    }
}

pub(crate) fn run(
    title: &'static str,
    capture_provider: Arc<Mutex<WindowsCaptureProvider>>,
) -> iced::Result {
    iced::application(title, State::update, State::view)
        .subscription(State::subscription)
        .run_with(|| (State::new(capture_provider), Task::none()))
}
