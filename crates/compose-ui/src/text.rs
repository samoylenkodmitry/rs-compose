use std::sync::{OnceLock, RwLock};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    /// Height of a single line of text
    pub line_height: f32,
    /// Number of lines in the text
    pub line_count: usize,
}

pub trait TextMeasurer: Send + Sync + 'static {
    fn measure(&self, text: &str) -> TextMetrics;
}

#[derive(Default)]
struct MonospacedTextMeasurer;

impl TextMeasurer for MonospacedTextMeasurer {
    fn measure(&self, text: &str) -> TextMetrics {
        const CHAR_WIDTH: f32 = 8.0;
        const LINE_HEIGHT: f32 = 20.0;
        
        // Split by newlines to handle multiline
        let lines: Vec<&str> = text.split('\n').collect();
        let line_count = lines.len().max(1);
        
        // Width is the max width of any line
        let width = lines.iter()
            .map(|line| line.chars().count() as f32 * CHAR_WIDTH)
            .fold(0.0_f32, f32::max);
        
        TextMetrics {
            width,
            height: line_count as f32 * LINE_HEIGHT,
            line_height: LINE_HEIGHT,
            line_count,
        }
    }
}

fn global_text_measurer() -> &'static RwLock<Box<dyn TextMeasurer>> {
    static TEXT_MEASURER: OnceLock<RwLock<Box<dyn TextMeasurer>>> = OnceLock::new();
    TEXT_MEASURER.get_or_init(|| RwLock::new(Box::new(MonospacedTextMeasurer)))
}

pub fn set_text_measurer<M: TextMeasurer>(measurer: M) {
    let mut guard = global_text_measurer()
        .write()
        .expect("text measurer lock poisoned");
    *guard = Box::new(measurer);
}

pub fn measure_text(text: &str) -> TextMetrics {
    global_text_measurer()
        .read()
        .expect("text measurer lock poisoned")
        .measure(text)
}
