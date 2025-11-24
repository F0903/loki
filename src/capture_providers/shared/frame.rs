use crate::capture_providers::shared::{Rect, Vector2};

#[derive(Debug, Clone)]
pub enum PixelFormat {
    RGBA8,
    BGRA8,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub format: PixelFormat,
    pub size: Vector2<i32>,
    pub timestamp: i64,
    pub dirty_rects: Vec<Rect<i32>>,
}
