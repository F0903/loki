use std::{fmt::Display, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum CaptureFramerate {
    FPS5,
    FPS24,
    FPS30,
    FPS60,
    FPS120,
}

impl CaptureFramerate {
    pub const ALL: [CaptureFramerate; 5] = [
        CaptureFramerate::FPS5,
        CaptureFramerate::FPS24,
        CaptureFramerate::FPS30,
        CaptureFramerate::FPS60,
        CaptureFramerate::FPS120,
    ];

    pub fn to_frametime(&self) -> Duration {
        match self {
            Self::FPS5 => Duration::from_secs_f32(1 as f32 / 5 as f32),
            Self::FPS24 => Duration::from_secs_f32(1 as f32 / 24 as f32),
            Self::FPS30 => Duration::from_secs_f32(1 as f32 / 30 as f32),
            Self::FPS60 => Duration::from_secs_f32(1 as f32 / 60 as f32),
            Self::FPS120 => Duration::from_secs_f32(1 as f32 / 120 as f32),
        }
    }
}

impl Display for CaptureFramerate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FPS5 => f.write_str("5"),
            Self::FPS24 => f.write_str("24"),
            Self::FPS30 => f.write_str("30"),
            Self::FPS60 => f.write_str("60"),
            Self::FPS120 => f.write_str("120"),
        }
    }
}
