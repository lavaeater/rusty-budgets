# Mobile (Android)

The mobile crate is a thin shell over the shared `ui` crate — same routing
(`/budget/:period/:tab`) and the same `BudgetHero` component tree as `web`
and `desktop`, styled for touch with `mobile/assets/main.css`.

## Prerequisites

- Android SDK + NDK (set `ANDROID_HOME` / `ANDROID_NDK_HOME`), `adb` on `PATH`.
- Dioxus CLI ≥ 0.7: `cargo install dioxus-cli` (or `curl -sSL https://dioxus.dev/install.sh | sh`).
- A running emulator (`emulator -avd <name>`) or a device connected over USB
  with debugging enabled.

## Networking: reaching the dev server from the device

The fullstack server (`dx serve`) listens on the *host's* `localhost:8080`.
Inside the Android WebView, `localhost` means the device's own loopback, not
the host's — so the app can't reach the server until you forward the port:

```sh
adb reverse tcp:8080 tcp:8080
```

This works for both the emulator and a USB-connected physical device, and
only needs to be re-run when the device reconnects. `mobile/src/main.rs`
points the client at `http://localhost:8080` accordingly, and
`AndroidManifest.xml` allows cleartext (plain HTTP) traffic for this — fine
for local dev, but revisit before shipping a build that talks to a real
deployed server over HTTPS.

## Serve (hot-reload dev loop)

```sh
adb reverse tcp:8080 tcp:8080
dx serve --package mobile --platform android
```

`dx serve` builds and installs a debug APK on the selected device/emulator
and streams logs. Rust logs are routed to logcat under the `rusty_budgets`
tag:

```sh
adb logcat -s rusty_budgets
```

## Build a release APK

```sh
dx bundle --package mobile --platform android --package-types apk --release
adb install -r ./target/dx/mobile/release/android/app/app/build/outputs/apk/release/app-release.apk
```

## Android build config

- `Dioxus.toml` (workspace root) — `[android]` (min/target/compile SDK 24/35/35),
  permissions (`INTERNET`, `ACCESS_NETWORK_STATE`), and the bundle icon.
- `mobile/assets/android/AndroidManifest.xml` — the manifest `dx` merges in;
  edit here for any further native Android config.
- `mobile/assets/icons/icon.svg` / `icon.png` — launcher icon source; `dx bundle`
  generates the mipmap densities from `icon.png`.
