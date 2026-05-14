//! Encoding spec — names which `DataFrame` column feeds which
//! visual channel (X, Y, Color).
//!
//! Each encoding is built from a column name + a scale kind. The
//! Plot facade derives the concrete `Scale` from the column's
//! values automatically (`Linear` reads `numeric_extent`; `Band`
//! / `Ordinal` read `distinct_categories`). Callers can override
//! the auto-derived domain via the builder methods on
//! [`Encoding`].

/// How a `Channel::Size` encoding maps numeric input to marker
/// size on `Mark::Point`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeMapping {
    /// Map value directly to marker radius. Visually misleading
    /// for magnitudes — a 4× value looks 16× larger.
    Radius,
    /// Map value to marker *area*, then take sqrt for radius —
    /// the bubble-chart default. A 4× value looks 4× larger.
    #[default]
    Area,
}

/// Which scale family the encoding's column maps through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleKind {
    /// Continuous linear mapping.
    Linear,
    /// Discrete categories → fixed-width pixel bands. Used for
    /// bar X axes.
    Band,
    /// Discrete categories → dense indices. Used for `Color`
    /// encoding lookups.
    Ordinal,
    /// `jiff::civil::Date` → continuous pixel range. Plot v1 only
    /// uses Linear / Band; Time scale lands when the time-axis
    /// charts ship.
    Time,
    /// Log-distributed continuous mapping.
    Log,
}

/// Which visual channel an encoding feeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Horizontal position.
    X,
    /// Vertical position.
    Y,
    /// Mark fill / stroke colour.
    Color,
    /// Sub-band offset within the X band (grouped bar charts).
    /// Re-bands the outer X scale into one inner band per
    /// distinct value of this encoding's column.
    XOffset,
    /// Numeric magnitude → marker size mapping (scatter +
    /// bubble charts). Source column must be numeric.
    Size,
}

/// One encoding — a channel + column + scale kind, optionally
/// with an explicit numeric domain override.
#[derive(Clone, Debug, PartialEq)]
pub struct Encoding {
    pub(crate) channel: Channel,
    pub(crate) field: String,
    pub(crate) scale_kind: ScaleKind,
    pub(crate) domain_override: Option<(f32, f32)>,
    pub(crate) size_mapping: SizeMapping,
}

impl Encoding {
    /// Construct from a channel + column name. Default
    /// `scale_kind` is `Linear`; override with
    /// [`Encoding::scale`].
    #[must_use]
    pub fn new(channel: Channel, field: impl Into<String>) -> Self {
        Self {
            channel,
            field: field.into(),
            scale_kind: ScaleKind::Linear,
            domain_override: None,
            size_mapping: SizeMapping::default(),
        }
    }

    /// Set the scale kind. Choose `Band` for X on bar charts,
    /// `Linear` for numeric Y, `Ordinal` for colour categories.
    #[must_use]
    pub fn scale(mut self, kind: ScaleKind) -> Self {
        self.scale_kind = kind;
        self
    }

    /// Override the auto-derived numeric `(min, max)` domain.
    /// Only consulted for `ScaleKind::Linear` / `Log`.
    #[must_use]
    pub fn domain(mut self, domain: (f32, f32)) -> Self {
        self.domain_override = Some(domain);
        self
    }

    /// For [`Channel::Size`] encodings, choose between linear
    /// radius mapping ([`SizeMapping::Radius`]) and the
    /// perceptually-correct area mapping ([`SizeMapping::Area`],
    /// the default).
    #[must_use]
    pub const fn size_mapping(mut self, mapping: SizeMapping) -> Self {
        self.size_mapping = mapping;
        self
    }
}

/// Convenience: `Encoding::new(Channel::X, field).scale(Band)`.
#[must_use]
pub fn x(field: impl Into<String>, kind: ScaleKind) -> Encoding {
    Encoding::new(Channel::X, field).scale(kind)
}

/// Convenience: `Encoding::new(Channel::Y, field).scale(Linear)`.
#[must_use]
pub fn y(field: impl Into<String>, kind: ScaleKind) -> Encoding {
    Encoding::new(Channel::Y, field).scale(kind)
}

/// Convenience: `Encoding::new(Channel::Color, field).scale(Ordinal)`.
#[must_use]
pub fn color(field: impl Into<String>) -> Encoding {
    Encoding::new(Channel::Color, field).scale(ScaleKind::Ordinal)
}

/// Convenience: `Encoding::new(Channel::XOffset, field).scale(Band)`.
/// Use to enable grouped-bar layout — each distinct value of `field`
/// gets its own sub-band within the outer X band.
#[must_use]
pub fn x_offset(field: impl Into<String>) -> Encoding {
    Encoding::new(Channel::XOffset, field).scale(ScaleKind::Band)
}

/// Convenience: `Encoding::new(Channel::Size, field).scale(Linear)`.
/// Maps a numeric column to marker radius / area on Point marks.
#[must_use]
pub fn size(field: impl Into<String>) -> Encoding {
    Encoding::new(Channel::Size, field).scale(ScaleKind::Linear)
}
