# Changelog

All notable changes to this project are documented in this file. The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and the project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Until 1.0.0, any release may contain breaking changes.

## [Unreleased]

## [0.1.0] — unreleased, first public source release

This is the first public release of the source code. It is **pre-alpha**: the Android app has never been run on a real phone, and all learning content is AI-drafted and not yet reviewed by human methodologists.

### Added

- **Engine** (`tutor-core`, `tutor-engine`, `course-pack`): deterministic tutoring engine with a hint ladder, analogous worked examples and a leak guard that keeps answers and intermediate results out of hints (invariants I1–I3); BKT mastery model; integer spaced review; session limits and anti-guessing delays; skill graphs with integrity checks; next-task selection with interleaving and de-duplication.
- **Mathematics, grade 5** (`generators-math`): 35 skills with task generators, exact oracles, scripted hints, typical-error messages, lessons and figures.
- **Other subjects** (`subjects`): 23 embedded JSON content packs in format `kmit-pack-1` covering grades 5–7 (Ukrainian language and literature, English, history, nature studies, biology, geography, physics, chemistry, informatics): 446 topics with lessons, 10,886 multiple-choice and 444 typed-answer items. All packs have review status `draft`.
- **Signed content updates:** Ed25519-signed packs can be installed from a file; `content-tools` validates, reports, applies reviewer corrections, sets review status, and signs and verifies packs.
- **Encrypted storage** (`storage`): per-profile SQLCipher databases, Argon2id + XChaCha20-Poly1305 key wrapping, PIN lockout, one-time recovery code, crypto-erase, passphrase-encrypted export and import, up to 4 profiles per device.
- **Child experience** (`app-core`, `app/ui`): lessons with step-by-step worked examples and self-checks, personal notes, practice and review, "recall and explain", weekly effort ruler, badges, sticker album, pixel hero, creature worlds for grades 5–12, grade-transition celebration with a printable certificate, adult settings (quiet hours, limits, accessibility), printable sheet for the teacher without answers.
- **Teacher profile:** printable worksheets (independent work, test, homework, NMT-style practice) in up to 4 variants with key, answer sheet and cut-out cards; dictations and gap-fills; 45-minute lesson plans and student handouts; topic plans; searchable lesson library with bookmarks. No student data.
- **Reviewer app** (feature `reviewer`, `app/reviewer`): separate "Kmit Методист" build for methodologists with per-text OK/fix/remove marks and CSV export.
- **PDF generation** (`report`): self-contained PDF writer with embedded Cyrillic fonts.
- **Shells:** Tauri 2 Android shell (`app/src-tauri`) with no `INTERNET` permission and backup disabled; `dev-server` for running the UI in a desktop browser.
- **Quality gates:** 494 Rust `#[test]` functions, compliance tests for Android permissions, ad/analytics dependencies, CSP and external URLs; headless Chromium end-to-end scenarios; UI compatibility check for old WebViews; CI on Linux, macOS and Windows.
- **Documentation:** README (English and Ukrainian), BUILDING, ARCHITECTURE, content format and review workflow, privacy policy draft.
- **Landing page and presentation** in `site/`.

### Known limitations

- Not run on a real Android device or emulator; WebView compatibility, SQLCipher at runtime, Argon2 timing, TalkBack and text-to-speech are unverified.
- Content is AI-drafted and checked only automatically (lint and a blind answer-key pass); human methodologist review has not started.
- No mathematics for grades 6–7; no typed-answer items for grades 6–7; no content for grades 8–12.
- The embedded content-signing public key is a development key; the Android Keystore integration is behind a disabled feature flag.

[Unreleased]: https://github.com/sashapiscivca-svg/kmit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sashapiscivca-svg/kmit/releases/tag/v0.1.0
