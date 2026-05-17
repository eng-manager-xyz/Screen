# macOS permissions — step-by-step

This doc is for the **first time you run the recorder after the macOS permissions PR lands**. It walks through exactly what to do, why, and what to do if something goes wrong. Every macOS-specific term is defined in the **[Glossary](#glossary)** at the end — if a word makes you go "huh?", scroll down.

You'll go through this once. After today, macOS remembers your choices and the prompts stop appearing.

---

## TL;DR — the whole thing in 30 seconds

1. Pull `main`.
2. Run `just test-recorder`.
3. macOS pops up and says *"screen-app would like to access the camera."* Click **Allow**.
4. Same for microphone (when you record audio) and screen recording (when you record your display) — one prompt per kind, on first use.
5. Open **System Settings → Privacy & Security → Camera**. You should see `screen-app` listed with a toggle. Same under Microphone and Screen Recording.
6. Done. macOS remembers. Re-launches are silent.

If a prompt doesn't appear when you expect one, jump to **[Troubleshooting](#troubleshooting)**.

---

## Step-by-step

### Step 1 — Pull the latest `main`

```bash
git checkout main
git pull
```

**Why:** the permissions PR baked an **Info.plist** (a small text file declaring "this app wants to use the camera/mic/screen") into the binary. You need the new binary.

> **Don't know what Info.plist means?** See the [Glossary](#glossary).

### Step 2 — Build and launch

```bash
just test-recorder
```

**Why:** this command rebuilds `target/debug/screen-app` (so the new Info.plist gets embedded), then launches the app. The first time the app touches the camera, macOS reads the embedded declaration and shows the permission prompt.

> **What does "embedded" mean?** Normally Apple apps you download from the App Store have an Info.plist as a separate file inside the app's folder. Our dev binary is a single file — no folder — so we use a trick: write the Info.plist contents into a special region of the binary itself called a **Mach-O section**. macOS knows to look there for command-line-tool-style binaries. See [Glossary → Mach-O](#glossary).

### Step 3 — Open the Recorder surface

1. Click the **circle icon** in the menubar (top-right of your Mac screen).
2. The AppShell window opens.
3. The Recorder surface should already be the active section. If not, click the camera icon in the left navigation rail.

**Why:** the Recorder surface auto-tries to enumerate cameras when it mounts. That's the trigger for the macOS permission prompt.

### Step 4 — Click "Allow" on the macOS prompt

A system prompt will appear:

> **"screen-app" would like to access the camera.**
> *Screen needs camera access to record your webcam alongside your screen.*
>
> [Don't Allow]    [Allow]

Click **Allow**.

**Why:** "Allow" tells macOS to remember that `screen-app` (specifically, the app with bundle identifier `com.screen.app`) is permitted to read the camera. macOS stores this in a system database called **TCC**.

> **Why does the prompt say "screen-app" specifically?** Because we declared `CFBundleName = "screen-app"` in the Info.plist. macOS reads that string and shows it in the prompt. See [Glossary → CFBundleName](#glossary).

### Step 5 — Verify in System Settings

1. Open **System Settings** (Apple menu → System Settings, or `⌘+Space` and type "System Settings").
2. Click **Privacy & Security** in the left sidebar.
3. Click **Camera** in the right pane.
4. You should now see `screen-app` in the list with a toggle (probably on, since you clicked Allow).

**Why:** if `screen-app` appears here, the embedded Info.plist worked and macOS has registered the app. If it doesn't appear, jump to [Troubleshooting](#troubleshooting).

### Step 6 — Verify the camera now works

Back in the recorder, click **"Select camera"** in the Recorder surface.

The dropdown should now show your camera (e.g. *FaceTime HD Camera*, *Insta360 Link*, *Logitech BRIO*). Before the permissions PR, it said "No cameras detected" — that was macOS silently refusing to enumerate cameras because the app hadn't asked for permission.

If you're on PR #46 (the diagnostics PR), the "Camera pipeline" overlay near the bottom of the Recorder surface should also start showing:
- `Source:` `480×480 @ 30 fps`
- `Frames:` (a number ticking up at ~30/sec)
- `First-frame PNG:` (a file path you can open in Finder to see your first captured frame)

### Step 7 — Repeat for microphone and screen recording (when you use those)

**Update (M-AUDIO ships):** the M-MIC chain (AUT-277/-278/-279) and the M-AUDIO-SYS chain (AUT-280/-281/-282) have all landed. You'll see the mic + screen-recording prompts the **first time you use those features in the recorder** — both follow the identical Allow → System Settings → working flow as the camera.

| Capture path | Triggers prompt when | TCC category | Permission string |
| --- | --- | --- | --- |
| Microphone (M-MIC) | First time you flip on a mic from the recorder | **Microphone** | `NSMicrophoneUsageDescription` |
| System audio (M-AUDIO-SYS) | First time you flip on "System audio" in the recorder | **Screen Recording** | `NSScreenCaptureUsageDescription` |
| Per-app audio (M-AUDIO-SYS) | First time you expand the system-audio picker | **Screen Recording** (same TCC entry as above — one grant covers both) | `NSScreenCaptureUsageDescription` |
| Screen video (M-SCK — future) | Future ticket | **Screen Recording** (same entry) | `NSScreenCaptureUsageDescription` |

> **Note about screen recording specifically:** after you grant screen-recording permission, macOS will ask you to **relaunch the app** before it takes effect. This is a well-known macOS quirk, not a bug. The recorder will show a "Quit and reopen" button. This applies to **both** the system-audio and the screen-video paths — they share the TCC entry.

```admonish tip title="One grant unlocks both SCK paths"
ScreenCaptureKit uses the **Screen Recording** TCC entry for both video and audio capture. If you grant Screen Recording the first time you flip on System Audio, you won't see a separate prompt later when the screen-capture video path lands — same grant covers both. Verified end-to-end in M-AUDIO.PERMS (AUT-283).
```

---

## Troubleshooting

### "No prompt appeared when I clicked Select camera"

Three possible causes, in order of likelihood:

**A. macOS cached an old "no permission" state for the binary.** Reset:

```bash
tccutil reset Camera com.screen.app
tccutil reset Microphone com.screen.app
tccutil reset ScreenCapture com.screen.app
```

> **What's `tccutil`?** A macOS command-line tool for managing TCC entries — see [Glossary → tccutil](#glossary).

Then re-launch the app (`just test-recorder` again). The prompt should appear.

**B. The Info.plist didn't actually embed into the binary.** Verify:

```bash
otool -s __TEXT __info_plist target/debug/screen-app | head -5
```

You should see hex bytes printed. If you see nothing or an error like "section not found," the build didn't embed the plist. Tell Claude — likely a build-script issue.

> **What's `otool`?** A macOS command for inspecting compiled binaries — see [Glossary → otool](#glossary).

**C. You're running a stale binary.** `just test-recorder` should rebuild, but if for some reason cargo cached the old object file, force a clean:

```bash
cargo clean -p screen-app
just test-recorder
```

### "`screen-app` doesn't appear in System Settings → Privacy & Security → Camera"

Means the prompt was never shown OR you clicked "Don't Allow" (which still creates an entry, just with the toggle off — if it's not there at all, the prompt was skipped).

Run the `tccutil reset` commands above, re-launch, watch for the prompt.

### "The prompt appeared but I clicked Don't Allow by mistake"

1. Open **System Settings → Privacy & Security → Camera**.
2. Find `screen-app` in the list.
3. Flip the toggle to **on**.
4. Re-launch the app.

If `screen-app` isn't there, run `tccutil reset Camera com.screen.app` first, re-launch, and you'll see the prompt again.

### "It works for camera, but I'm worried about mic and screen recording"

**Verified working as of M-AUDIO.PERMS (AUT-283).** The same Info.plist already declares mic + screen-recording strings. When the recorder uses the microphone (M-MIC) or system audio (M-AUDIO-SYS), macOS prompts for those separately, with the same Allow → System Settings → working flow. **You don't have to do anything to prepare for those.**

### "I granted Screen Recording but per-process audio still returns silence"

**Quit and relaunch the app.** This is the well-known macOS quirk: after granting Screen Recording for the first time, the running app's process still operates under the old (denied) TCC state until restart. The Recorder UI will show a "Quit and reopen" button when it detects this case (future M-AUDIO-SYS polish ticket).

Verification: `cargo run -p media --example system_audio_smoke` against a freshly-reset TCC + freshly-granted Screen Recording should print non-zero RMS when audio plays in another window. If RMS is still zero post-grant, you almost certainly haven't relaunched yet.

### "I'm on macOS 12.x and the recorder won't launch"

Intentional. `LSMinimumSystemVersion` was bumped from 12.3 → 13.0 in M-AUDIO-SYS.0 (AUT-280) because `SCStreamConfiguration.capturesAudio` (the API we use for system audio) is macOS 13.0+. macOS 12.3–12.7 users see *"This app requires macOS 13.0 or later"* at launch. Upgrade to 13.0+ or wait for a future runtime-feature-detect ticket that would disable system audio on 12.x rather than blocking the whole app.

### "Will I have to do all this again every time the app is rebuilt?"

No. macOS pairs the permission grant against the **CFBundleIdentifier** string `com.screen.app` (declared in the Info.plist). Rebuilds keep the same identifier, so macOS keeps the grant. You only re-prompt if:

- You explicitly run `tccutil reset` (above).
- The bundle identifier in `tauri.conf.json` ever changes (don't do this).
- macOS major version upgrade in some rare cases.

---

## What about people who download the app?

Future you ships a `.dmg` to users. Same Info.plist is used by the bundled `.app` (Tauri's bundler picks up the file automatically). End-users:

1. Open the `.dmg`, drag `screen-app.app` to Applications.
2. Launch.
3. First camera use → macOS prompts them with the same friendly string.
4. They click Allow → app appears in their System Settings → done.

**You're not on the hook to do anything per-user.** macOS handles the whole lifecycle.

The one extra thing for public distribution (separate concern, not this PR): you'll need to **code-sign** the bundle with an **Apple Developer ID** (~$99/year from Apple) so Gatekeeper doesn't show *"this app is from an unidentified developer"* to your users. Permissions still work without code-signing, but unsigned apps have a worse first-launch experience. See [Glossary → Gatekeeper, Code signing](#glossary).

---

## What's *not* in this PR (and why that's fine)

| Concern | Status | Why / when |
|---|---|---|
| Camera, microphone, screen recording permissions | ✅ In this PR | Load-bearing for the recorder to function |
| Documents / Downloads / Desktop / Removable-volume access | ✅ In this PR | Lets the recorder save+read recordings to/from those folders without a picker |
| File pickers (Open / Save dialogs) | ✅ No permission needed | macOS treats the user's explicit pick as implicit grant — Tauri's file-dialog plugin uses these. Drag/drop also routes through pickers. |
| Hardened Runtime entitlements | ❌ Deferred | Only matter for code-signed builds. We don't ship signed yet. |
| Code signing | ❌ Deferred | Needs an Apple Developer account ($99/year). Defer until public release. |
| Notarization | ❌ Deferred | Requires code signing first. Same defer. |
| Accessibility permission | ❌ Not needed yet | Only needed if we add global hotkeys (e.g. "press F12 to start recording from anywhere"). |
| AppleScript / automation permission | ❌ Not needed | We don't automate other apps. |
| Full Disk Access | ❌ Not needed (intentionally) | Way more invasive — reads any file on the system. The four user-folder permissions above are the right granularity. |

You're not forgetting anything for getting **camera + mic + screen capture + file save/load** to work.

---

## Glossary

These are arranged roughly by "what you'll trip over first."

### Info.plist

A small **XML text file** that describes a macOS app to the operating system: what it's called, its version, what kinds of OS features it wants to use (camera, microphone, etc.). Apple apps you download from the App Store have this as a separate file inside the app's folder (`MyApp.app/Contents/Info.plist`). Our dev binary doesn't have a folder — see the [Mach-O](#mach-o) entry for how we still get one.

### TCC

Short for **Transparency, Consent, and Control**. The macOS subsystem that:
- Reads Info.plist to find out what permissions an app wants.
- Shows the permission prompt when an app first tries to use a sensitive feature.
- Remembers the user's Allow/Deny choice in a database.
- Surfaces the entries in System Settings → Privacy & Security so the user can change their mind later.

When you see "no prompt appeared" or "the app isn't in Privacy settings," it's TCC behind the scenes deciding what to do.

### Mach-O

The **executable file format** Apple uses (the equivalent of Windows's `.exe` or Linux's ELF). Our `target/debug/screen-app` is one Mach-O file.

Mach-O files are split into **sections**. Most sections hold the actual program code, but Apple defined a special section called `__TEXT,__info_plist` where command-line-style binaries can embed their Info.plist. macOS knows to look there if the binary isn't inside a folder.

### `__TEXT,__info_plist` section

The Mach-O section name where the embedded Info.plist lives in our dev binary. `__TEXT` is the segment (a group of sections); `__info_plist` is the specific section. The `otool -s __TEXT __info_plist <binary>` command prints its contents.

### `embed_plist` (historical)

The **Rust crate** that PR #47 originally used to manually embed `Info.plist` into the Mach-O via a macro. The macro emitted a static byte array tagged with `#[link_section = "__TEXT,__info_plist"]`.

**Removed in M-MIC.1 (AUT-278).** Tauri 2.6.1+'s `tauri::generate_context!()` macro auto-embeds `Info.plist` for every debug macOS build (see `tauri-codegen-2.6.1/src/context.rs::context_codegen`). The manual `embed_plist!` call was redundant *and* it emitted the **same** `_EMBED_INFO_PLIST` symbol as the auto-embed, breaking every integration test in `screen-app` at link time. The auto-embed is now the only path; the `embed_plist` dep is removed from `crates/app/Cargo.toml`.

If you need to verify the section is still being embedded post-removal, the same `otool` command works:

```bash
otool -s __TEXT __info_plist target/debug/screen-app | head -5
```

You'll still see hex bytes — Tauri's auto-embed is producing them now, not the manual macro.

### CFBundleIdentifier

The **unique ID for the app**. Our value is `com.screen.app`. macOS's TCC keys every permission grant against this string. Think of it as a primary key in a database — if it changes, the row changes, and the permission grant is effectively lost.

Common convention: reverse-domain form (`com.yourdomain.appname`). Once a user has installed your app, **never change this string**, because every existing user will be re-prompted for every permission and your old grants are orphaned.

### CFBundleName

The **display name** macOS shows in the permission prompt. Our value is `screen-app`. So the dialog says *"screen-app would like to access the camera"*. Distinct from `CFBundleIdentifier`.

### CFBundleShortVersionString / CFBundleVersion

The **version string** of the app. Both values should match — we use `0.1.0`. Stored in TCC alongside the identifier on some macOS versions, so they should remain consistent with the version in `tauri.conf.json`.

### LSMinimumSystemVersion

The **minimum macOS version** the app needs. Currently `13.0`. Bumped from 12.3 in M-AUDIO-SYS.0 (AUT-280) because ScreenCaptureKit's *audio* API (`SCStreamConfiguration.capturesAudio`) is macOS 13.0+. macOS refuses to launch the app on older systems instead of letting it run + fail later.

```admonish note title="History"
The original PR #47 declared `12.3` — ScreenCaptureKit's *video* API was introduced then. M-AUDIO-SYS.0 needed the 13.0-only audio API and bumped accordingly.
```

### NSCameraUsageDescription / NSMicrophoneUsageDescription / NSScreenCaptureUsageDescription

The **three Info.plist keys** that declare which permissions the app wants. The string value of each key is the **text that appears in the macOS permission prompt** — that's why they say things like *"Screen needs camera access to record your webcam alongside your screen."* — that's user-facing copy, not a technical comment.

Without these strings, macOS won't even show the prompt — it'll silently return empty results (zero cameras, zero capture sessions) and the app appears broken.

### `NSScreenCaptureUsageDescription` (special note)

This **one string covers a lot** in modern macOS:
- Full-display capture (recording your entire screen).
- Single-window capture.
- **System audio capture** (recording whatever's playing through your speakers — game audio, music, etc.).
- Per-application audio capture (recording just one specific app's audio output).

All of these go through ScreenCaptureKit, which uses the screen-recording TCC entry rather than the microphone one — so this single permission unlocks everything except your microphone.

### `tccutil`

A **command-line tool** built into macOS for managing TCC entries. Use it to reset a permission so the prompt re-appears:

```bash
tccutil reset Camera com.screen.app
```

Categories you can reset: `Camera`, `Microphone`, `ScreenCapture`, `Accessibility`, `AppleEvents`, `All`. The second argument is the bundle identifier; omit it to reset the category for every app.

### `otool`

A **command-line tool** built into macOS for inspecting Mach-O binaries (the macOS equivalent of `objdump` on Linux). We use it to verify the Info.plist embedded correctly:

```bash
otool -s __TEXT __info_plist target/debug/screen-app
```

The output is hex bytes — if they look like text (look for `<?xml`, `<plist>`, etc. spelled out in the right column), the embed worked.

### `plutil`

A **command-line tool** built into macOS for working with `.plist` files (validating, pretty-printing, converting between XML and binary formats):

```bash
plutil -lint crates/app/Info.plist          # validate
plutil -p crates/app/Info.plist             # pretty-print
```

### Bundled `.app`

A **macOS application bundle** — a folder named `something.app` that macOS treats as a single application. Inside it has a standard layout:

```
screen-app.app/
├── Contents/
│   ├── Info.plist          ← TCC reads this
│   ├── MacOS/screen-app    ← the actual binary
│   └── Resources/          ← icons, fonts, etc.
```

When you eventually ship a `.dmg` to users, this is what they get. Tauri's bundler (`cargo tauri build`) creates it automatically.

### ScreenCaptureKit (SCK)

Apple's **modern screen-capture API**, introduced in macOS 12.3. Provides display capture, window capture, and system-audio capture in one framework. The older `CGScreen*` API still exists but is deprecated and has performance limits we'd hit.

### AVFoundation

Apple's **media-capture framework** — used for camera and microphone. The `AVCaptureDevice` class is how the recorder enumerates cameras under the hood (via GStreamer's `avfvideosrc`).

### Hardened Runtime

A **macOS security feature** that restricts what a signed app can do at runtime — prevents loading unsigned libraries, prevents debugger attachment, etc. Only relevant for code-signed apps. **You don't have to worry about this until you start signing for distribution.**

### Code signing

Adding a **cryptographic signature** to the app bundle that proves it was built by you. Required to avoid Gatekeeper warnings on user machines. Needs an Apple Developer ID (~$99/year). **Defer until you ship to actual users.**

### Notarization

A **post-signing step** where Apple scans your signed app for malware and attaches a notarization ticket. Required for `.dmg`s distributed outside the App Store on modern macOS. **Defer until you ship.**

### Gatekeeper

The **macOS security feature** that decides whether to let a downloaded app run. Signed + notarized apps launch silently. Unsigned apps trigger *"this app cannot be opened because it is from an unidentified developer"* (the user can right-click → Open to override, but it's a worse experience).

### Apple Developer ID

A **$99/year membership** that gets you a certificate for code signing. Required for shipping macOS apps outside the App Store without the Gatekeeper warning. Sign up at [developer.apple.com](https://developer.apple.com).

---

## Related docs

- [`_docs/book/src/app-ui/chunks/macos-permissions.md`](./book/src/app-ui/chunks/macos-permissions.md) — the technical deep-dive on how the embedding mechanism works, for contributors editing the Info.plist or build scripts.
- [`_docs/book/src/app-ui/chunks/camera-pipeline.md`](./book/src/app-ui/chunks/camera-pipeline.md) — the M-CAM.3 worker that consumes camera frames once permission is granted.
- Linear ticket: [AUT-272](https://linear.app/harwood/issue/AUT-272) — screen-recording permission deep-link to System Settings, the M-SCK companion to AUT-261's camera deep-link.
