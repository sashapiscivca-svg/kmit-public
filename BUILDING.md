# Building Kmit

**English** · Українською: див. підсумок нижче.

> **Коротко українською.** Для розробки досить Rust і браузера: `scripts/dev.sh`, потім http://127.0.0.1:8765/. Для Android потрібні Rust із ціллю `aarch64-linux-android`, `tauri-cli` 2.12.1, JDK 17, Android SDK (platform 37, build-tools 37.0.0), NDK r29, а також `perl` і `make`. Створіть власний файл середовища `env.sh` (приклад нижче, розділ 3) і вкажіть його в змінній `ENV_SH`. Дитячий і вчительський застосунок збирає `scripts/android-apk.sh`, застосунок методиста `scripts/android-reviewer-apk.sh`. Без змінних `KMIT_KEYSTORE`, `KMIT_KEY_ALIAS`, `KMIT_STORE_PASS`, `KMIT_KEY_PASS` APK підписується **тестовим** ключем, який створюється локально й не годиться для публікації. Наприкінці скрипти друкують дозволи APK: дозволу `INTERNET` там бути не має. Застосунок ще не запускали на справжньому телефоні.

This document covers building and testing Kmit from source: the desktop development build, the Android APKs, signing, and checking that the APK has no network permission. For what the pieces are, see [ARCHITECTURE.md](ARCHITECTURE.md).

> **Status.** The Android build has been produced on a Linux x86_64 host and verified with automated checks only. **No APK has been run on a real device or emulator yet**, so on-device behaviour (WebView compatibility on old phones, SQLCipher at runtime, Argon2 speed, TalkBack, file saving, text-to-speech) is unverified.

## 1. Desktop build (no Android tools needed)

The Rust workspace (`crates/*`) builds on Linux, macOS and Windows (CI runs all three).

| Tool | Version | Used for |
|---|---|---|
| Rust (rustup) | stable; MSRV 1.80 (`rust-version` in `Cargo.toml`) | everything |
| C compiler, `perl`, `make` | any recent | `rusqlite` with `bundled-sqlcipher-vendored-openssl` compiles SQLCipher and OpenSSL from source |
| Node.js | ≥ 22 | `scripts/check_ui.sh`, end-to-end tests |
| Chromium or Chrome | recent | end-to-end tests (headless) |
| Python 3 | ≥ 3.8 | content scripts in `scripts/*.py`, `site/tools/build_site.py` |

```bash
cargo fetch                         # once: the helper scripts run Cargo with --offline
scripts/dev.sh                      # browser build: http://127.0.0.1:8765/
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check_ui.sh
CHROME=/path/to/chrome THEME=light scripts/e2e.sh
CHROME=/path/to/chrome THEME=dark  scripts/e2e.sh
CHROME=/path/to/chrome scripts/e2e-reviewer.sh
```

`scripts/dev.sh` builds `dev-server` in release mode and serves `app/ui` on 127.0.0.1 only. Data is stored in `~/.local/share/kmit-dev` (override with `KMIT_DATA`; port with `PORT`). Stop it with Ctrl+C. The dev server uses the production Argon2 parameters, so creating or opening a profile takes a moment; the e2e scripts build it with the `fast-kdf` feature instead.

Feature flags used in development (never enable them in a build for people):

| Crate | Feature | Effect |
|---|---|---|
| `storage` | `test-params` | allows very weak Argon2id parameters (tests only) |
| `tutor-core`, `app-core` | `test-support` | lets tests peek at answers to drive a child's path |
| `dev-server` | `fast-kdf` | weak Argon2id for UI tests |
| `dev-server` | `e2e` | `fast-kdf` + `test-support`; exposes the current answer at `/__peek` for automated tests |
| `app-core`, `dev-server`, `kmit-app` | `reviewer` | the methodologist build (see section 5) |
| `kmit-app` | `keystore` | wraps the profile key with Android Keystore; **off by default, untested on devices** |

## 2. Android toolchain

The Android app is a Tauri 2 project in `app/src-tauri`, a separate Cargo workspace with its own `Cargo.lock`. The generated Android project in `app/src-tauri/gen/android` is committed, including the manifest change that removes `INTERNET`. Do not regenerate it with `cargo tauri android init` without re-applying that change.

Versions the build was produced with:

| Component | Version |
|---|---|
| Host OS | Linux x86_64 |
| Rust | stable 1.99.0 (rustup) |
| Rust targets | `aarch64-linux-android` (required); `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android` (optional) |
| `tauri-cli` | 2.12.1 (`cargo install tauri-cli --version 2.12.1 --locked`) |
| `tauri` crate | 2 (locked in `app/src-tauri/Cargo.lock`) |
| JDK | Temurin 17 (17.0.20.1); a newer system JDK was not used |
| Android SDK platform | `platforms;android-37.0` (compileSdk 37, targetSdk 37) |
| Android build-tools | 37.0.0 (`zipalign`, `apksigner`, `aapt2`) |
| Android NDK | r29, `ndk;29.0.14206865` |
| Android cmdline-tools | latest (`sdkmanager`) |
| Gradle / Android Gradle Plugin / Kotlin | 9.6.1 (wrapper) / 9.3.1 / 2.2.10, from the generated project |
| Minimum Android | 7.0, API 24 (`minSdkVersion` in `tauri.conf.json`) |
| Also needed | `perl`, `make` (OpenSSL build), `keytool` (from the JDK) |

Install the Rust parts:

```bash
rustup toolchain install stable
rustup target add aarch64-linux-android
# optional, for other ABIs:
rustup target add armv7-linux-androideabi i686-linux-android x86_64-linux-android
cargo install tauri-cli --version 2.12.1 --locked
```

Install the Android parts with the official command-line tools (download `cmdline-tools` from developer.android.com/studio and check its SHA-256):

```bash
sdkmanager --sdk_root="$ANDROID_HOME" "platform-tools" "platforms;android-37.0" "build-tools;37.0.0" "ndk;29.0.14206865"
```

The first Gradle build downloads the Gradle distribution and Android libraries, so the build machine needs network access then. The app itself never does.

## 3. The environment file (`env.sh`)

Both APK scripts start with `source "${ENV_SH:-$HOME/.cache/kmit-s0/env.sh}"`. Create your own file anywhere and point `ENV_SH` at it. It must export the following variables (adjust paths to your installation; nothing here is specific to Kmit except that the scripts read `ANDROID_HOME`):

```bash
# env.sh: example; replace the paths with your own
export JAVA_HOME="$HOME/opt/jdk-17"                 # JDK 17
export ANDROID_HOME="$HOME/opt/android-sdk"         # Android SDK root
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"   # NDK r29 (Tauri reads NDK_HOME)
export ANDROID_NDK_ROOT="$NDK_HOME"                 # openssl-src reads ANDROID_NDK_ROOT
# Optional: keep toolchains and caches outside your home defaults
# export RUSTUP_HOME=...  CARGO_HOME=...  GRADLE_USER_HOME=...
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$PATH"
# NDK r29 has no aarch64-linux-android-ranlib; the vendored OpenSSL build needs the LLVM tools:
export PATH="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
export RANLIB=llvm-ranlib AR=llvm-ar
```

Without the last two lines the OpenSSL build fails with `aarch64-linux-android-ranlib: not found`. If you set `RUSTUP_HOME`/`CARGO_HOME`, also put `$CARGO_HOME/bin` on `PATH`.

## 4. Child and teacher app

```bash
ENV_SH=/path/to/env.sh scripts/android-apk.sh
```

The script:

1. runs `cargo tauri android build --apk --target aarch64` in `app/src-tauri` (frontend: `app/ui` as is, no bundler);
2. aligns the unsigned universal release APK with `zipalign`;
3. signs it with `apksigner` (see section 6) and verifies the signature;
4. prints the APK's permissions with `aapt2 dump permissions`.

Output: `$RELEASE_DIR/kmit-tutor-arm64-test.apk` (default `RELEASE_DIR` is `~/kmit-release`). Application id `ua.kmit.tutor`, label "Kmit". Child and teacher profiles are in the same app; there is no separate teacher APK.

The release Cargo profile uses `opt-level = "z"`, LTO, one codegen unit, `panic = "abort"` and stripping, so the first build (which also compiles OpenSSL and SQLCipher) takes several minutes.

## 5. Reviewer (methodologist) app

```bash
ENV_SH=/path/to/env.sh scripts/android-reviewer-apk.sh
```

This builds a separate app for content reviewers, distributed outside Google Play:

- `scripts/build-reviewer-ui.sh` assembles `app/reviewer-dist` (reviewer UI, shared `api.js`, the Manrope font);
- the Gradle property `kmitAppId` sets the application id `ua.kmit.reviewer`; the label becomes "Kmit Методист" and reviewer icons are copied in;
- Cargo feature `reviewer` is enabled: only review commands work; child profiles, teacher commands and profile import are refused.

The script temporarily edits `gen/android/app/src/main/res` and restores it with `git checkout`, so run it from a **git working tree**; otherwise restore those files yourself afterwards.

Output: `$RELEASE_DIR/kmit-reviewer-arm64-test.apk`. The script also prints the package name and label (`aapt2 dump badging`) and the permissions.

## 6. Signing

Both scripts read four variables:

| Variable | Meaning |
|---|---|
| `KMIT_KEYSTORE` | path to your `.jks` keystore |
| `KMIT_KEY_ALIAS` | key alias |
| `KMIT_STORE_PASS` | keystore password |
| `KMIT_KEY_PASS` | key password |

If `KMIT_KEYSTORE` is set but the file does not exist, the script stops with an error. If it is **not** set, the script uses `$RELEASE_DIR/test-key.jks`, creating it on first run (RSA 2048, valid 10 years, alias `test`, passwords `testtest`, subject `CN=kmit test key`). **The test key is for local testing only.** Anyone can recreate it, so never distribute an APK signed with it. Android only installs an update over an existing app if both are signed with the same key, so use one persistent key for every build you hand out (including reviewer builds).

Create a persistent key once and keep it, and its passwords, outside the repository:

```bash
keytool -genkeypair -keystore ~/secure/kmit-release.jks -alias kmit -keyalg RSA -keysize 4096 -validity 10000
export KMIT_KEYSTORE=~/secure/kmit-release.jks KMIT_KEY_ALIAS=kmit
read -rs KMIT_STORE_PASS; export KMIT_STORE_PASS
read -rs KMIT_KEY_PASS;   export KMIT_KEY_PASS
```

Content packs are signed separately (Ed25519, `content-tools keygen` and `sign`); see [docs/review-workflow.md](docs/review-workflow.md). The app currently embeds a development public key (`PUBLISHER_PUBLIC_KEY` in `crates/subjects/src/pack.rs`), which must be replaced before a real release.

## 7. Verifying that the APK has no network permission

The app must not request `android.permission.INTERNET`. Three layers check this:

1. **Source:** `app/src-tauri/gen/android/app/src/main/AndroidManifest.xml` declares `<uses-permission android:name="android.permission.INTERNET" tools:node="remove" />`, which strips the permission even if Tauri's template or a library adds it.
2. **Tests:** `cargo test -p app-core --test compliance_tests` fails if the manifest grants `INTERNET`, `AD_ID`, location, camera, microphone, contacts, phone state, storage, Bluetooth, accounts or SMS permissions, if ad or analytics dependencies appear, or if the CSP or UI allow non-local sources.
3. **Built APK:** both scripts print the permissions at the end. To check any APK by hand:

```bash
BT="$ANDROID_HOME/build-tools/37.0.0"
"$BT/aapt2" dump permissions kmit-tutor-arm64-test.apk
"$BT/aapt2" dump permissions kmit-tutor-arm64-test.apk | grep -c 'android.permission.INTERNET'   # must print 0
"$BT/aapt2" dump xmltree --file AndroidManifest.xml kmit-tutor-arm64-test.apk | grep -i -E 'permission|usesCleartextTraffic'
```

The only expected entries are the package's own `DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION` added by AndroidX. On a connected device you can also run `adb shell dumpsys package ua.kmit.tutor | grep -i permission`.

Note that the Tauri **dev** mode (`cargo tauri android dev`) needs network access to a dev server and is not supported by this manifest; use `scripts/dev.sh` in a browser for UI work instead.

## 8. Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `aarch64-linux-android-ranlib: not found` while building OpenSSL | the NDK's LLVM tools are not on `PATH`, or `RANLIB`/`AR` are not set (section 3) |
| `no matching package named ...` with `--offline` | run `cargo fetch` once (for the Tauri app: `cd app/src-tauri && cargo fetch`) |
| `KMIT_KEYSTORE вказує на неіснуючий файл` | `KMIT_KEYSTORE` points to a missing file; fix the path or unset it to use the test key |
| `ls: cannot access '.../build-tools/*'` | `ANDROID_HOME` is wrong or build-tools are not installed |
| Reviewer icons or label left in the working tree | the reviewer script ran outside a git checkout; restore `app/src-tauri/gen/android/app/src/main/res` |
