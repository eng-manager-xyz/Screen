//! Finance-style charts — candlestick, OHLC bar, waterfall.
//!
//! All three are self-contained value types (like
//! [`crate::indicator`]) rather than `Plot` marks, because they
//! consume domain-specific encoding fields (open / high / low /
//! close, or cumulative deltas) that don't fit the
//! grammar-of-graphics `(X, Y, Color)` channel model.

pub mod candlestick;
pub mod ohlc;
pub mod waterfall;

pub use candlestick::{Candlestick, Period};
pub use ohlc::Ohlc;
pub use waterfall::{Waterfall, WaterfallRow};
