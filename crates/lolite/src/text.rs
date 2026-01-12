use crate::{
    layout::Size,
    style::{Length, Style},
};
use parking_lot::RwLock;
use skia_safe::{Font, FontMgr, FontStyle};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontSpec {
    pub family: String,
    pub size_px: u32,
}

impl FontSpec {
    pub fn from_style(style: &Style) -> Self {
        let family = style
            .font_family
            .clone()
            .unwrap_or_else(|| "Arial".to_string());

        let size_px = match style.font_size {
            Some(Length::Px(px)) if px > 0.0 => px.round().clamp(1.0, 512.0) as u32,
            _ => 12,
        };

        Self { family, size_px }
    }
}

pub trait TextMeasurer: Send + Sync {
    /// Called at the start of a layout pass.
    ///
    /// Implementations with caches can use this to begin "mark" tracking.
    fn begin_layout_pass(&self) {}

    /// Called at the end of a layout pass.
    ///
    /// Implementations with caches can use this to "sweep" anything not used
    /// during the pass.
    fn end_layout_pass_and_sweep(&self) {}

    fn measure_unwrapped(&self, text: &str, font: &FontSpec) -> Size;
    fn measure_wrapped(&self, text: &str, font: &FontSpec, max_width_px: f64) -> Size;
}

#[derive(Clone, Default)]
pub struct SkiaTextMeasurer {
    cache: Arc<RwLock<CacheState>>,
}

#[derive(Default)]
struct CacheState {
    epoch: u64,
    map: HashMap<CacheKey, CacheEntry>,
}

#[derive(Clone, Copy)]
struct CacheEntry {
    size: Size,
    last_used_epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    text: String,
    family: String,
    size_px: u32,
    max_width_px_rounded: u32,
}

impl SkiaTextMeasurer {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(CacheState::default())),
        }
    }

    pub(crate) fn make_font(font: &FontSpec) -> Font {
        let typeface = FontMgr::default()
            .match_family(&font.family)
            .match_style(FontStyle::normal())
            .unwrap_or_else(|| {
                // Fallback typeface.
                FontMgr::default()
                    .legacy_make_typeface(None, FontStyle::normal())
                    .expect("Failed to load any typeface")
            });

        Font::new(typeface, font.size_px as f32)
    }

    fn measure_unwrapped_uncached(&self, text: &str, font: &FontSpec) -> Size {
        let font = Self::make_font(font);

        // `measure_str` gives us an advance width; height comes from font metrics.
        let (advance_width, _bounds) = font.measure_str(text, None);

        let (_scale, metrics) = font.metrics();
        let height = (metrics.descent - metrics.ascent + metrics.leading) as f64;

        Size {
            width: advance_width as f64,
            height: height.max(0.0),
        }
    }

    fn measure_wrapped_uncached(&self, text: &str, font: &FontSpec, max_width_px: f64) -> Size {
        // NOTE: Skia has a proper paragraph layout API, but Lolite doesn’t depend on it yet.
        // This approximation is good enough to drive basic layout decisions.
        let max_width_px = max_width_px.max(0.0);
        if max_width_px == 0.0 {
            return Size::default();
        }

        let unwrapped = self.measure_unwrapped_uncached(text, font);
        if unwrapped.width <= max_width_px {
            return unwrapped;
        }

        // Naive wrapping: estimate number of lines by width ratio.
        let lines = (unwrapped.width / max_width_px).ceil().max(1.0);
        Size {
            width: max_width_px,
            height: unwrapped.height * lines,
        }
    }
}

impl TextMeasurer for SkiaTextMeasurer {
    fn begin_layout_pass(&self) {
        let mut state = self.cache.write();
        state.epoch = state.epoch.wrapping_add(1);
    }

    fn end_layout_pass_and_sweep(&self) {
        let mut state = self.cache.write();
        let epoch = state.epoch;
        state.map.retain(|_, entry| entry.last_used_epoch == epoch);
    }

    fn measure_unwrapped(&self, text: &str, font: &FontSpec) -> Size {
        let key = CacheKey {
            text: text.to_string(),
            family: font.family.clone(),
            size_px: font.size_px,
            max_width_px_rounded: 0,
        };

        let mut state = self.cache.write();
        let epoch = state.epoch;

        if let Some(entry) = state.map.get_mut(&key) {
            entry.last_used_epoch = epoch;
            return entry.size;
        }

        // Cache miss.
        let size = self.measure_unwrapped_uncached(text, font);
        state.map.insert(
            key,
            CacheEntry {
                size,
                last_used_epoch: epoch,
            },
        );
        size
    }

    fn measure_wrapped(&self, text: &str, font: &FontSpec, max_width_px: f64) -> Size {
        let key = CacheKey {
            text: text.to_string(),
            family: font.family.clone(),
            size_px: font.size_px,
            max_width_px_rounded: max_width_px.round().clamp(0.0, 1_000_000.0) as u32,
        };

        let mut state = self.cache.write();
        let epoch = state.epoch;

        if let Some(entry) = state.map.get_mut(&key) {
            entry.last_used_epoch = epoch;
            return entry.size;
        }

        // Cache miss.
        let size = self.measure_wrapped_uncached(text, font, max_width_px);
        state.map.insert(
            key,
            CacheEntry {
                size,
                last_used_epoch: epoch,
            },
        );
        size
    }
}

#[cfg(test)]
#[derive(Clone, Default)]
#[allow(unused)]
pub struct TestTextMeasurer;

#[cfg(test)]
impl TextMeasurer for TestTextMeasurer {
    fn measure_unwrapped(&self, text: &str, font: &FontSpec) -> Size {
        // Deterministic sizing for unit tests.
        let size = font.size_px as f64;
        let char_w = (size * 0.6).max(1.0);
        let line_h = (size * 1.2).max(1.0);

        Size {
            width: (text.chars().count() as f64) * char_w,
            height: line_h,
        }
    }

    fn measure_wrapped(&self, text: &str, font: &FontSpec, max_width_px: f64) -> Size {
        let unwrapped = self.measure_unwrapped(text, font);
        let max_width_px = max_width_px.max(0.0);
        if max_width_px == 0.0 {
            return Size::default();
        }
        if unwrapped.width <= max_width_px {
            return unwrapped;
        }

        let lines = (unwrapped.width / max_width_px).ceil().max(1.0);
        Size {
            width: max_width_px,
            height: unwrapped.height * lines,
        }
    }
}

pub fn default_text_measurer() -> Arc<dyn TextMeasurer> {
    #[cfg(test)]
    {
        Arc::new(TestTextMeasurer::default())
    }

    #[cfg(not(test))]
    {
        Arc::new(SkiaTextMeasurer::new())
    }
}
