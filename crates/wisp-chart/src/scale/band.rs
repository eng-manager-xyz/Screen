//! Band scale — discrete categories → fixed-width bands with
//! configurable padding.

#![allow(
    clippy::cast_precision_loss,
    reason = "category count is small (typically ≤ 50) — well below f32 precision."
)]

/// Maps a list of categories to evenly-spaced bands across a
/// continuous range. Each band has a `start` and `end` pixel
/// coordinate that the bar / box-plot / candlestick renderer can
/// use directly.
///
/// `padding` is the fraction of band width left empty on each
/// side — `0.0` packs bars flush together, `0.1` leaves 10% gap
/// on each end (matching d3's default), `0.5` gives bands the
/// same width as the gap between them.
///
/// # Example
///
/// ```
/// use wisp_chart::scale::BandScale;
/// let scale = BandScale::new(
///     ["Q1", "Q2", "Q3", "Q4"],
///     (180.0, 960.0),
/// ).padding(0.1);
/// let (start, end) = scale.range_for(&"Q1").unwrap();
/// // Q1's band starts after the outer padding.
/// assert!(start > 180.0);
/// assert!(end < 960.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct BandScale<C: Eq + Clone> {
    categories: Vec<C>,
    range: (f32, f32),
    padding: f32,
}

impl<C: Eq + Clone> BandScale<C> {
    /// Construct from a category iterator and `(range_min,
    /// range_max)` pixel bounds.
    pub fn new<I: IntoIterator<Item = C>>(categories: I, range: (f32, f32)) -> Self {
        Self {
            categories: categories.into_iter().collect(),
            range,
            padding: 0.1,
        }
    }

    /// Builder: set padding as a fraction of band width
    /// (`0.0..=0.5`). Defaults to 0.1.
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding.clamp(0.0, 0.5);
        self
    }

    /// Number of categories.
    #[must_use]
    pub fn category_count(&self) -> usize {
        self.categories.len()
    }

    /// Width of a single band's inner (drawable) region in pixels.
    #[must_use]
    pub fn band_width(&self) -> f32 {
        let n = self.categories.len();
        if n == 0 {
            return 0.0;
        }
        let total = (self.range.1 - self.range.0).abs();
        let step = total / n as f32;
        let inner = step * (1.0 - self.padding * 2.0);
        inner.max(0.0)
    }

    /// Pixel `(start, end)` for the band containing `category`,
    /// or `None` if the category isn't present.
    #[must_use]
    pub fn range_for(&self, category: &C) -> Option<(f32, f32)> {
        let i = self.categories.iter().position(|c| c == category)?;
        Some(self.band_range_at(i))
    }

    /// Pixel `(start, end)` for the band at index `i`, or `None`
    /// if `i` is out of bounds.
    #[must_use]
    pub fn band_at(&self, i: usize) -> Option<(f32, f32)> {
        if i >= self.categories.len() {
            return None;
        }
        Some(self.band_range_at(i))
    }

    /// Pixel centre for the band at index `i`, or `None` if out of
    /// bounds. Useful for placing point marks / tick labels at
    /// band midpoints.
    #[must_use]
    pub fn band_centre(&self, i: usize) -> Option<f32> {
        let (s, e) = self.band_at(i)?;
        Some((s + e) * 0.5)
    }

    fn band_range_at(&self, i: usize) -> (f32, f32) {
        let n = self.categories.len() as f32;
        let total = self.range.1 - self.range.0;
        let step = total / n;
        let pad_px = step.abs() * self.padding;
        let raw_start = self.range.0 + (i as f32) * step;
        let raw_end = raw_start + step;
        // Apply padding inward from each side. step can be
        // negative (right-to-left ranges); take care to add /
        // subtract by sign.
        let pad_signed = pad_px.copysign(step);
        (raw_start + pad_signed, raw_end - pad_signed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_widths_sum_with_padding_to_total() {
        let s = BandScale::new(["a", "b", "c", "d"], (0.0, 400.0)).padding(0.0);
        // No padding: 4 bands × 100 px each = 400.
        let widths: f32 = (0..4)
            .map(|i| {
                let (s0, s1) = s.band_at(i).unwrap();
                s1 - s0
            })
            .sum();
        assert!((widths - 400.0).abs() < 1e-4);
    }

    #[test]
    fn padding_0p1_leaves_10pct_gap_on_each_side() {
        let s = BandScale::new(["a", "b"], (0.0, 200.0)).padding(0.1);
        // Each band step = 100 px; padding 10% = 10 px each side.
        let (s0, s1) = s.band_at(0).unwrap();
        assert!((s0 - 10.0).abs() < 1e-4);
        assert!((s1 - 90.0).abs() < 1e-4);
    }

    #[test]
    fn padding_clamps_to_max_half() {
        let s = BandScale::new(["a"], (0.0, 100.0)).padding(0.8);
        // Clamped to 0.5 → band width = 0.
        assert!(s.band_width() < 1e-6);
    }

    #[test]
    fn range_for_missing_category_returns_none() {
        let s = BandScale::new(["a", "b"], (0.0, 200.0));
        assert!(s.range_for(&"ghost").is_none());
    }

    #[test]
    fn band_centre_lies_between_start_and_end() {
        let s = BandScale::new(["a", "b", "c"], (0.0, 300.0)).padding(0.2);
        let centre = s.band_centre(1).unwrap();
        let (s0, s1) = s.band_at(1).unwrap();
        assert!((centre - (s0 + s1) * 0.5).abs() < 1e-4);
        assert!(s0 < centre && centre < s1);
    }

    #[test]
    fn band_at_out_of_bounds_returns_none() {
        let s = BandScale::new(["a", "b"], (0.0, 200.0));
        assert!(s.band_at(2).is_none());
    }

    #[test]
    fn reversed_range_keeps_first_band_at_range_start() {
        // For horizontal bar charts where the y-pixel range is
        // top-down but categories iterate top-to-bottom.
        let s = BandScale::new(["a", "b", "c"], (400.0, 100.0)).padding(0.0);
        let (a0, a1) = s.band_at(0).unwrap();
        assert!(a0 <= 400.0 && a1 < a0);
    }
}
