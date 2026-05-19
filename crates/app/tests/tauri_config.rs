//! Static Tauri config contract tests.

#[test]
fn global_tauri_api_is_enabled_for_frontend_bridge() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).expect("valid tauri.conf.json");

    assert_eq!(
        config
            .pointer("/app/withGlobalTauri")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "app-ui/index.html calls window.__TAURI__ directly; Tauri 2 only injects it when app.withGlobalTauri is true"
    );
}

#[test]
fn default_capability_keeps_core_event_permissions() {
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/default.json"))
            .expect("valid default capability");

    let permissions = capability
        .get("permissions")
        .and_then(serde_json::Value::as_array)
        .expect("permissions array");

    assert!(
        permissions
            .iter()
            .any(|permission| permission.as_str() == Some("core:default")),
        "core:default includes core:event:default, required by window.__TAURI__.event.listen"
    );
}

#[test]
fn tauri_build_hooks_run_from_app_ui_crate() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).expect("valid tauri.conf.json");

    let hooks = [
        ("beforeDevCommand", "trunk serve --port 1420"),
        ("beforeBuildCommand", "trunk build --release"),
    ];

    for (key, trunk_command) in hooks {
        let hook = config
            .pointer(&format!("/build/{key}"))
            .and_then(serde_json::Value::as_object)
            .expect("build hook command object");

        assert_eq!(
            hook.get("cwd").and_then(serde_json::Value::as_str),
            Some("../app-ui"),
            "{key} must run from the Trunk app-ui crate"
        );
        assert!(
            hook.get("script")
                .and_then(serde_json::Value::as_str)
                .is_some_and(
                    |script| script.contains("env -u NO_COLOR") && script.contains(trunk_command)
                ),
            "{key} must clear NO_COLOR before invoking trunk"
        );
    }
}
