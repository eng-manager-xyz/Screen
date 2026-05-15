//! Staged-write batching — sample all animations into a reused
//! buffer (read phase), then apply writes in one pass with
//! deterministic last-wins semantics per `(NodeId, Property)`.
//!
//! Design inspired by [fastdom](https://github.com/wilsonpage/fastdom):
//! batch reads and writes into separate phases so the work is
//! linear, not interleaved. wisp doesn't have DOM-style forced
//! layout, but the *shape* — pure-sample → staged-write → render —
//! is the same and gives us:
//!
//! - **Determinism**: two animations writing the same property
//!   produce last-wins regardless of registration order.
//! - **Zero per-frame allocation**: the staging buffer is a
//!   `Vec` owned by [`BatchDriver`] and reused across ticks.
//! - **Budget headroom**: sample + apply work is `O(N)` linear
//!   in active animations; sized for RAIL's <10 ms animation
//!   frame budget.
//!
//! See [`crate::Driver`] for the underlying clock. `BatchDriver`
//! is a `Driver` plus the staging buffer.

use std::time::Duration;

use wisp::scene::Stage;

use crate::{Animation, Driver, NodeProperty, Property, Target};

/// One scalar-output animation bound to a [`NodeProperty`] target.
/// Composition is deferred to the driver — the animation is
/// sampled, the target is told to write the result.
pub struct BoundScalar {
    /// The underlying animation, type-erased to `f32` output.
    pub anim: Box<dyn Animation<Output = f32>>,
    /// Where to write the sampled value.
    pub target: NodeProperty,
}

impl BoundScalar {
    /// Bind a scalar animation to a node property.
    pub fn new<A: Animation<Output = f32> + 'static>(anim: A, target: NodeProperty) -> Self {
        Self {
            anim: Box::new(anim),
            target,
        }
    }
}

/// Driver + staging buffer. The buffer is reused across ticks so
/// `tick_scalars` allocates nothing once warmed up.
///
/// # Example
///
/// ```ignore
/// use std::time::Duration;
/// use wisp_animation::{BatchDriver, BoundScalar, NodeProperty, Tween};
///
/// let mut driver = BatchDriver::realtime();
/// driver.play();
///
/// let alpha_fade = Tween::new(0.0_f32, 1.0, Duration::from_millis(500));
/// let mut anims = vec![BoundScalar::new(alpha_fade, NodeProperty::alpha(my_node))];
///
/// // Each frame:
/// driver.tick_scalars(Duration::from_millis(16), &mut anims, &mut stage);
/// ```
pub struct BatchDriver {
    /// Underlying driver clock.
    pub driver: Driver,
    /// Reused staging buffer. `(NodeId, Property, sampled_value,
    /// registration_index)`. The trailing `u32` is the entry's
    /// position in the caller's `&mut [BoundScalar]` slice and
    /// serves as a tiebreaker so the dedup sort can stay
    /// `sort_unstable` (alloc-free).
    staged: Vec<(wisp::scene::NodeId, Property, f32, u32)>,
}

impl BatchDriver {
    /// Real-time driver, no-alloc tick.
    #[must_use]
    pub fn realtime() -> Self {
        Self {
            driver: Driver::realtime(),
            staged: Vec::with_capacity(64),
        }
    }

    /// Fixed-step driver, deterministic export-friendly.
    #[must_use]
    pub fn fixed(dt: Duration) -> Self {
        Self {
            driver: Driver::fixed(dt),
            staged: Vec::with_capacity(64),
        }
    }

    /// Begin advancing.
    pub const fn play(&mut self) {
        self.driver.play();
    }

    /// Halt advancement.
    pub const fn pause(&mut self) {
        self.driver.pause();
    }

    /// Current elapsed time.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.driver.elapsed()
    }

    /// Read phase: sample every animation into the staging buffer.
    /// Write phase: sort by `(NodeId, Property)` with the original
    /// registration index as a *descending* tiebreaker, then walk
    /// forward and apply only the first entry per `(NodeId,
    /// Property)` run — that entry has the highest registration
    /// index, which is "last-wins" by construction.
    ///
    /// The two-phase split mirrors fastdom's measure/mutate
    /// separation — we don't get the forced-layout payoff (wisp
    /// has no layout) but we *do* get deterministic last-wins +
    /// zero per-frame allocation.
    ///
    /// # Performance
    ///
    /// `O(N log N)` per frame from the sort; `O(N)` from the
    /// read phase and the dedup walk. The whole thing stays
    /// alloc-free after warm-up because [`slice::sort_unstable_by`]
    /// is in-place pdqsort. Stable sort (`sort_by`) would allocate
    /// `O(N)` scratch every frame and is deliberately avoided —
    /// the trailing-index tiebreaker recovers the stable-sort
    /// semantics without the allocation.
    pub fn tick_scalars(
        &mut self,
        dt: Duration,
        anims: &mut [BoundScalar],
        stage: &mut Stage,
    ) -> Duration {
        let elapsed = self.driver.tick(dt);

        // Read phase. Reuse the staging buffer; no allocation
        // after warm-up.
        self.staged.clear();
        self.staged.reserve(anims.len());
        for (i, anim) in anims.iter().enumerate() {
            let v = anim.anim.sample(elapsed);
            // Cap the tiebreaker index at u32::MAX — far more than
            // any realistic active-animation count, and 32 bits
            // saves stack bytes per staged entry.
            #[allow(
                clippy::cast_possible_truncation,
                reason = "active-animation count is well under u32::MAX in practice"
            )]
            let idx = i as u32;
            self.staged
                .push((anim.target.node, anim.target.prop, v, idx));
        }

        // Sort: primary key = (node, prop) ascending; tiebreaker =
        // registration index DESCENDING so the latest-registered
        // entry per key lands first in each run.
        self.staged
            .sort_unstable_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then(b.3.cmp(&a.3)));

        // Write phase: emit only the first entry of each
        // (node, prop) run — that's the last-wins winner.
        let mut last_key: Option<(wisp::scene::NodeId, Property)> = None;
        for &(node, prop, value, _) in &self.staged {
            if last_key == Some((node, prop)) {
                continue;
            }
            last_key = Some((node, prop));
            let target = NodeProperty { node, prop };
            <NodeProperty as Target<f32>>::write(&target, stage, value);
        }

        elapsed
    }

    /// Drain staged writes since last `tick_scalars`. Test helper —
    /// returns the staging buffer contents in sorted (key-major)
    /// order, with the trailing `u32` being the original
    /// registration index.
    #[must_use]
    pub fn last_staged(&self) -> &[(wisp::scene::NodeId, Property, f32, u32)] {
        &self.staged
    }
}

impl Default for BatchDriver {
    fn default() -> Self {
        Self::realtime()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ease, LinearRamp, Tween};
    use wisp::scene::Graphics;

    fn fresh_node(stage: &mut Stage) -> wisp::scene::NodeId {
        let root = stage.root();
        stage.add_child(root, Graphics::new()).expect("add child")
    }

    #[test]
    fn single_anim_writes_through() {
        let mut stage = Stage::new();
        let node = fresh_node(&mut stage);
        let mut driver = BatchDriver::fixed(Duration::from_millis(100));
        driver.play();
        let mut anims = vec![BoundScalar::new(
            Tween::new(0.0_f32, 1.0, Duration::from_millis(500)),
            NodeProperty::alpha(node),
        )];
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
        // After one 100ms tick = 100/500 of the tween = 0.2.
        let alpha = stage.get(node).unwrap().container().alpha;
        assert!((alpha - 0.2).abs() < 1e-3, "got alpha = {alpha}");
    }

    #[test]
    fn last_registered_wins_for_same_property() {
        let mut stage = Stage::new();
        let node = fresh_node(&mut stage);
        let mut driver = BatchDriver::fixed(Duration::from_millis(100));
        driver.play();
        // First tween writes 0.2; second tween writes 0.8. Last wins.
        let mut anims = vec![
            BoundScalar::new(
                Tween::new(0.0_f32, 1.0, Duration::from_millis(500)),
                NodeProperty::alpha(node),
            ),
            BoundScalar::new(
                Tween::new(0.0_f32, 0.8, Duration::from_millis(100)).ease(Ease::Linear),
                NodeProperty::alpha(node),
            ),
        ];
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
        let alpha = stage.get(node).unwrap().container().alpha;
        assert!(
            (alpha - 0.8).abs() < 1e-3,
            "expected last-wins → 0.8, got {alpha}"
        );
    }

    #[test]
    fn different_properties_on_same_node_both_land() {
        let mut stage = Stage::new();
        let node = fresh_node(&mut stage);
        let mut driver = BatchDriver::fixed(Duration::from_millis(100));
        driver.play();
        let mut anims = vec![
            BoundScalar::new(
                LinearRamp::new(0.0_f32, 0.6, Duration::from_millis(100)),
                NodeProperty::alpha(node),
            ),
            BoundScalar::new(
                LinearRamp::new(0.0_f32, 1.2, Duration::from_millis(100)),
                NodeProperty::rotation(node),
            ),
        ];
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
        let n = stage.get(node).unwrap();
        assert!((n.container().alpha - 0.6).abs() < 1e-3);
        assert!((n.container().transform.rotation - 1.2).abs() < 1e-3);
    }

    #[test]
    fn stale_node_writes_are_silent() {
        let mut stage = Stage::new();
        let node = fresh_node(&mut stage);
        let mut driver = BatchDriver::fixed(Duration::from_millis(50));
        driver.play();
        stage.destroy(node);
        let mut anims = vec![BoundScalar::new(
            Tween::new(0.0_f32, 1.0, Duration::from_millis(500)),
            NodeProperty::alpha(node),
        )];
        // Must not panic.
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
    }
}
