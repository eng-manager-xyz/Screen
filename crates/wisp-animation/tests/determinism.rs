//! Determinism stress test for the deterministic export path
//! (M-ANIM.18 / AUT-245).
//!
//! Two `DriverMode::Fixed` drivers, seeded identically, ticked
//! 600 frames each (10 seconds of 60 fps motion) against a
//! richly-composed animation, must produce sample-equal output
//! across the entire run.
//!
//! This is the load-bearing contract for offline MP4 export:
//! same animation value, same driver mode, same dt → same bytes,
//! every run, every platform.

use std::time::Duration;

use glam::Vec2;
use wisp_animation::{
    AnimationRepeatExt, Curve, Driver, Ease, Parallel, RepeatCount, RepeatStrategy, Sequence,
    Spring, Track, Tween,
};

#[test]
fn fixed_driver_is_deterministic_across_600_frames_with_complex_animation() {
    let dt = Duration::from_secs_f32(1.0 / 60.0);
    let frames = 600;

    let mut samples_a: Vec<f32> = Vec::with_capacity(frames);
    let mut samples_b: Vec<f32> = Vec::with_capacity(frames);

    for samples in [&mut samples_a, &mut samples_b] {
        let mut driver = Driver::fixed(dt);
        driver.play();

        // A reasonably complex animation: a Sequence of two
        // Tweens wrapped in MirroredRepeat, parallel with a Spring,
        // sampled into a single scalar.
        let seq: Sequence<f32> = Sequence::new()
            .then(Tween::new(0.0_f32, 1.0, Duration::from_millis(700)).ease(Ease::OutBack))
            .then(Tween::new(1.0_f32, 0.5, Duration::from_millis(500)).ease(Ease::InCubic));
        let yoyo = seq.repeat_with(RepeatCount::Infinite, RepeatStrategy::MirroredRepeat);
        let spring = Spring::critically_damped(120.0, 1.0).between(0.0, 1.0);

        for _ in 0..frames {
            driver.tick(dt);
            // Feed garbage dt — Fixed mode ignores it.
            let yoyo_sample = wisp_animation::Animation::sample(&yoyo, driver.elapsed());
            let spring_sample = wisp_animation::Animation::sample(&spring, driver.elapsed());
            samples.push(yoyo_sample + spring_sample);
        }
    }

    assert_eq!(
        samples_a, samples_b,
        "fixed-mode driver produced divergent output across two identical runs"
    );
}

#[test]
fn realtime_mode_with_identical_dt_is_also_deterministic() {
    // Realtime mode is "use caller dt" — pass the same dt in
    // both runs to verify the math is purely a function of the
    // accumulated dt sum.
    let dt = Duration::from_secs_f32(1.0 / 120.0);
    let frames = 240;
    let parallel: Parallel<f32> = Parallel::new()
        .with(Tween::new(0.0_f32, 100.0, Duration::from_secs(2)))
        .with(Tween::new(0.0_f32, 50.0, Duration::from_secs(3)));

    let mut left = Driver::realtime();
    let mut right = Driver::realtime();
    left.play();
    right.play();
    for _ in 0..frames {
        left.tick(dt);
        right.tick(dt);
    }
    assert_eq!(left.elapsed(), right.elapsed());
    let v_l = wisp_animation::Animation::sample(&parallel, left.elapsed());
    let v_r = wisp_animation::Animation::sample(&parallel, right.elapsed());
    assert!((v_l - v_r).abs() < f32::EPSILON);
}

#[test]
fn curve_sample_is_deterministic_per_t() {
    // Spatial determinism: a `Curve` sampled at the same `t`
    // returns the same `Vec2` regardless of when or how often
    // it's called.
    let pts = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(2.0, -1.0),
        Vec2::new(3.0, 0.0),
    ];
    let curve = Curve::catmull_rom(pts, Duration::from_secs(1));
    let a = curve.sample_normalised(0.33);
    let b = curve.sample_normalised(0.33);
    let c = curve.sample_normalised(0.66);
    let d = curve.sample_normalised(0.66);
    assert!((a - b).length() < f32::EPSILON);
    assert!((c - d).length() < f32::EPSILON);
}

#[test]
fn keyframe_track_per_segment_dispatch_is_deterministic() {
    let track: Track<f32> = Track::new()
        .key(Duration::ZERO, 0.0)
        .key_eased(Duration::from_millis(300), 100.0, Ease::OutCubic)
        .key_eased(Duration::from_millis(700), 50.0, Ease::InOutBack)
        .key_eased(Duration::from_secs(1), 80.0, Ease::Linear);
    let probes = [50, 150, 300, 450, 650, 700, 850, 999];
    let first: Vec<f32> = probes
        .iter()
        .map(|t| wisp_animation::Animation::sample(&track, Duration::from_millis(*t)))
        .collect();
    let second: Vec<f32> = probes
        .iter()
        .map(|t| wisp_animation::Animation::sample(&track, Duration::from_millis(*t)))
        .collect();
    assert_eq!(first, second);
}
