use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use glyphon::{Attrs, Buffer, FontSystem, Metrics, Shaping};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct TextCacheKey {
    text: String,
    scale_bits: u32,
}

impl TextCacheKey {
    pub fn new(text: &str, scale: f32) -> Self {
        Self {
            text: text.to_string(),
            scale_bits: scale.to_bits(),
        }
    }
}

#[derive(Debug)]
pub struct CachedTextBuffer {
    buffer: Buffer,
    text: String,
    scale: f32,
}

impl CachedTextBuffer {
    pub fn new(font_system: &mut FontSystem, text: &str, scale: f32, attrs: Attrs) -> Self {
        let metrics = Metrics::new(14.0 * scale, 20.0 * scale);
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_size(font_system, f32::MAX, f32::MAX);
        buffer.set_text(font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(font_system);

        Self {
            buffer,
            text: text.to_string(),
            scale,
        }
    }

    pub fn ensure(
        &mut self,
        font_system: &mut FontSystem,
        text: &str,
        scale: f32,
        attrs: Attrs,
    ) -> bool {
        if self.text == text && self.scale == scale {
            return false;
        }

        let metrics = Metrics::new(14.0 * scale, 20.0 * scale);
        self.buffer.set_metrics(font_system, metrics);
        self.buffer
            .set_text(font_system, text, attrs, Shaping::Advanced);
        self.buffer.shape_until_scroll(font_system);

        self.text.clear();
        self.text.push_str(text);
        self.scale = scale;

        true
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}

pub type SharedTextCache = Arc<Mutex<HashMap<TextCacheKey, CachedTextBuffer>>>;

pub fn new_shared_text_cache() -> SharedTextCache {
    Arc::new(Mutex::new(HashMap::new()))
}
