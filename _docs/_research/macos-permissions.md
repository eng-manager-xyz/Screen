# macOS permissions tracker

Every macOS-only capture surface eventually needs an Info.plist
`NS*UsageDescription` string. Without the right string the OS-level
permission prompt **does not appear**, and the API call fails with
a misleading error (typically "device busy" or "permission denied"
without context).

This file is the single source of truth for which strings the app
needs at each milestone. Update it before adding any capture-API
call that would trigger an OS permission prompt.

## Strings the app uses (or will use)

| Permission key | Triggered by | Milestone | Status |
|---|---|---|---|
| `NSCameraUsageDescription` | `avfvideosrc` (M-CAM.0 `from_default_camera`) | M-CAM.0 (AUT-254) | **pending** — bundling is disabled (`tauri.conf.json: bundle.active = false`), dev-launched binary inherits the user's existing permission. Add to Info.plist before re-enabling bundling. |
| `NSMicrophoneUsageDescription` | Future audio capture (M-MIC) | M-MIC | not yet |
| `NSScreenCaptureUsageDescription` + screen-recording entitlement | Future ScreenCaptureKit (M-SCK) | M-SCK | not yet |
| `NSAccessibilityUsageDescription` | Future cursor / global-shortcut work | M-TRAY.6 / future | not yet |

## Suggested copy

When bundling re-enables, add these strings to the Info.plist via
Tauri's plist-merge feature (set `bundle.macOS.exceptionDomain` /
`bundle.macOS.entitlements` paths in `tauri.conf.json` to point at
the merged plist):

```text
NSCameraUsageDescription = "Screen needs camera access to record your webcam alongside your screen."
NSMicrophoneUsageDescription = "Screen needs microphone access to record audio alongside your screen."
NSScreenCaptureUsageDescription = "Screen needs screen recording access to capture your display."
```

## How to verify the string is wired

1. Delete the app from `/Applications/` (this resets the OS's
   per-app permission cache).
2. `tccutil reset Camera com.screen.app` (or the relevant
   permission's `tccutil` keyword) — belt and braces.
3. Rebuild + launch.
4. macOS should show the permission prompt with the configured
   string as the rationale. If no prompt appears, the string is
   missing or malformed.

## CI considerations

The CI runners on `macos-latest` don't bundle the app, so they
inherit whatever permission state the runner has (typically none,
so capture calls fail). M-CAM.0's integration smoke runtime-skips
via `default_camera_available()` to avoid CI red.
