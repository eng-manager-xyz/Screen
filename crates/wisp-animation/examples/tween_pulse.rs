//! `tween_pulse` — minimal proof-of-life for `Driver` + `Animation`.
//!
//! Drives a 1-second `LinearRamp` from 0 → 1 in fixed-step mode at
//! 60 fps, prints the elapsed-time + sampled value every 6 frames,
//! and asserts the ramp lands exactly at the endpoint.
//!
//! Run with `cargo run -p wisp-animation --example tween_pulse`.

use std::time::Duration;

use wisp_animation::{Driver, LinearRamp};

fn main() {
    let ramp = LinearRamp::new(0.0, 1.0, Duration::from_secs(1));
    let mut driver = Driver::fixed(Duration::from_secs_f32(1.0 / 60.0));
    driver.play();

    println!(
        "{:>5}  {:>12}  {:>8}  {:>8}",
        "frame", "elapsed_ms", "sample", "progress"
    );
    for frame in 0..=60 {
        if frame % 6 == 0 {
            let elapsed_ms = driver.elapsed().as_secs_f64() * 1000.0;
            println!(
                "{frame:>5}  {elapsed_ms:>12.3}  {:>8.4}  {:>8.4}",
                driver.sample(&ramp),
                driver.progress(&ramp),
            );
        }
        // Pass `Duration::ZERO` to prove Fixed mode ignores caller dt.
        driver.tick(Duration::ZERO);
    }

    let final_sample = driver.sample(&ramp);
    assert!(
        (final_sample - ramp.to).abs() < 1e-6,
        "expected ramp to land at to={}, got {final_sample}",
        ramp.to,
    );
    println!("\nramp converged to {final_sample}");
}
