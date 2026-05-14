//! Encoding spec — names which `DataFrame` column feeds which
//! visual channel (X, Y, Color).
//!
//! Each encoding is built from a column name + a scale kind. The
//! Plot facade derives the concrete `Scale` from the column's
//! values automatically (`Linear` reads `numeric_extent`; `Band`
//! / `Ordinal` read `distinct_categories`). Callers can override
//! the auto-derived domain via the builder methods on
//! [`Encoding`].

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
}

/// One encoding — a channel + column + scale kind, optionally
/// with an explicit numeric domain override.
#[derive(Clone, Debug, PartialEq)]
pub struct Encoding {
    pub(crate) channel: Channel,
    pub(crate) field: String,
    pub(crate) scale_kind: ScaleKind,
    pub(crate) domain_override: Option<(f32, f32)>,
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
