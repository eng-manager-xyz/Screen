//! Smoke test for `wisp-3d-web`: native target builds + the crate
//! exposes its name marker. Trunk-based wasm32 build is exercised
//! manually (per CLAUDE.md; we don't want CI to install Trunk on
//! every gate run).

#[test]
fn native_stub_exposes_crate_name() {
    assert_eq!(wisp_3d_web::crate_name(), "wisp-3d-web");
}
