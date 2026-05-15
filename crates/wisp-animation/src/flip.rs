//! `Flip` — capture-mutate-from layout transitions.
//!
//! Pattern:
//!
//! 1. `let state = Flip::capture(&stage)` — snapshot every reachable
//!    node's container transform.
//! 2. Mutate the stage (data update; re-emit chart; reorder).
//! 3. `let anim = Flip::from(state, dur, ease)` — produces a
//!    `Vec<NodeFlipTween>` of per-node animations from each node's
//!    captured transform back to its current transform via the
//!    *delta*.
//!
//! Each `NodeFlipTween` implements `Animation<Output = Transform>`
//! and writes to the appropriate `NodeProperty::transform` target
//! when sampled. Callers apply the samples to the stage directly.

use std::collections::HashMap;
use std::time::Duration;

use wisp::scene::{NodeId, Stage, Transform};

use crate::{Animation, Ease};

/// Snapshot of every node's container transform.
#[derive(Clone, Debug, Default)]
pub struct FlipState {
    /// Map of `NodeId` → captured local transform.
    pub captures: HashMap<NodeId, Transform>,
}

/// Per-node FLIP tween — animates the transform from the captured
/// value back toward the mutated (current) value.
#[derive(Clone, Copy, Debug)]
pub struct NodeFlipTween {
    /// Target node.
    pub node: NodeId,
    /// Captured pre-mutation transform.
    pub from: Transform,
    /// Current post-mutation transform.
    pub to: Transform,
    /// Total tween duration.
    pub duration: Duration,
    /// Ease.
    pub ease: Ease,
}

impl Animation for NodeFlipTween {
    type Output = Transform;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> Transform {
        if self.duration.is_zero() {
            return self.to;
        }
        #[allow(clippy::cast_possible_truncation, reason = "progress bounded [0, 1]")]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        let p = self.ease.eval(raw.clamp(0.0, 1.0));
        Transform {
            position: self.from.position + (self.to.position - self.from.position) * p,
            scale: self.from.scale + (self.to.scale - self.from.scale) * p,
            rotation: self.from.rotation + (self.to.rotation - self.from.rotation) * p,
            pivot: self.to.pivot,
            skew: self.from.skew + (self.to.skew - self.from.skew) * p,
        }
    }
}

/// Static API for capturing + producing per-node tweens.
pub struct Flip;

impl Flip {
    /// Snapshot every reachable node's container transform.
    #[must_use]
    pub fn capture(stage: &Stage) -> FlipState {
        let mut captures = HashMap::new();
        stage.traverse_pre_order(stage.root(), |id, node| {
            captures.insert(id, node.container().transform);
        });
        FlipState { captures }
    }

    /// Build per-node tweens from a captured [`FlipState`] to the
    /// stage's current transforms. Nodes whose transforms didn't
    /// change return zero tweens.
    #[must_use]
    pub fn from(
        prev: &FlipState,
        stage: &Stage,
        duration: Duration,
        ease: Ease,
    ) -> Vec<NodeFlipTween> {
        let mut tweens = Vec::with_capacity(prev.captures.len());
        for (id, from) in &prev.captures {
            let Some(node) = stage.get(*id) else { continue };
            let to = node.container().transform;
            if from.position == to.position
                && (from.rotation - to.rotation).abs() < f32::EPSILON
                && from.scale == to.scale
            {
                continue;
            }
            tweens.push(NodeFlipTween {
                node: *id,
                from: *from,
                to,
                duration,
                ease,
            });
        }
        tweens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use wisp::scene::Graphics;

    #[test]
    fn no_mutation_emits_no_tweens() {
        let mut stage = Stage::new();
        let root = stage.root();
        let g = Graphics::new();
        let _ = stage.add_child(root, g);
        let captured = Flip::capture(&stage);
        let tweens = Flip::from(&captured, &stage, Duration::from_millis(300), Ease::Linear);
        assert!(tweens.is_empty());
    }

    #[test]
    fn translation_change_emits_one_tween() {
        let mut stage = Stage::new();
        let root = stage.root();
        let g = Graphics::new();
        let node = stage.add_child(root, g).expect("add child");
        let captured = Flip::capture(&stage);
        if let Some(n) = stage.get_mut(node) {
            n.container_mut().transform.position = Vec2::new(0.5, 0.0);
        }
        let tweens = Flip::from(&captured, &stage, Duration::from_millis(300), Ease::Linear);
        assert_eq!(tweens.len(), 1);
        assert_eq!(tweens[0].node, node);
        assert_eq!(tweens[0].from.position, Vec2::ZERO);
        assert_eq!(tweens[0].to.position, Vec2::new(0.5, 0.0));
    }

    #[test]
    fn flip_tween_midpoint_lerps_transform() {
        let from = Transform {
            position: Vec2::ZERO,
            ..Transform::IDENTITY
        };
        let to = Transform {
            position: Vec2::new(10.0, 10.0),
            ..Transform::IDENTITY
        };
        let t = NodeFlipTween {
            node: Stage::new().root(),
            from,
            to,
            duration: Duration::from_secs(1),
            ease: Ease::Linear,
        };
        let mid = t.sample(Duration::from_millis(500));
        assert!((mid.position - Vec2::new(5.0, 5.0)).length() < 1e-3);
    }
}
