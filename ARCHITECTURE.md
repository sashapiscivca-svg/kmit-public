# Architecture

This document explains how Kmit is put together: which crate does what, how the three builds differ, how data moves (it never leaves the device), how storage is protected, and which invariants the code enforces. For build instructions see [BUILDING.md](BUILDING.md).

## Overview

Kmit is a thin UI over a Rust core. The UI (`app/ui`, plain ES modules) never computes anything that matters: it sends a command name, JSON arguments and the current time to the core and renders the JSON it gets back. All learning logic, storage and PDF generation live in Rust.

```
On the device (nothing crosses the network):

  app/ui (JS)  --call(name, args, ctx)-->  shell  -->  app-core  -->  storage (SQLCipher)
       ^                                                  |      -->  report (PDF)
       +-------------------- JSON view <------------------+

  shell = Tauri 2 on Android: invoke('call')
        | dev-server on 127.0.0.1 (development only): POST /api/<name>
```

The clock is read only by the UI (`makeCtx()` in `app/ui/api.js`) and passed in as `Ctx {wall, day, minute}`; the core has no clock of its own, which keeps it deterministic and testable.

## Crates

| Crate | Responsibility | Depends on (workspace) |
|---|---|---|
| `tutor-core` | Pure engine with no I/O, UI or network: task model, hint ladder (R1 → R2 → R3 → analogous example → "ask an adult"), leak guard, BKT mastery model (only `+ - * /` on `f64`, no `libm`, so results match across platforms), integer spaced-review intervals, session limits and anti-"gaming" delays, exact fractions, deterministic RNG, lessons and figures, sticker album, pixel hero, the teacher-sheet model, learner-state codec | none |
| `course-pack` | `SkillGraph`: skills and prerequisites, validated on construction (duplicates, unknown prerequisites, cycles) | none |
| `tutor-engine` | Selection of the next skill and task: readiness, continuation, review and microtests, interleaving, de-duplication of statements; teacher sheet assembly from learner state. Deterministic: the same state gives the same choice | `tutor-core`, `course-pack` |
| `generators-math` | Grade 5 mathematics (35 skills): generators with an exact oracle, protected intermediate values, scripted hints, typical-error ("bug") messages, lessons, figures, worksheet content | `tutor-core` |
| `subjects` | All non-maths subjects: embedded JSON packs (`include_str!` of `packs/*.json`), pack parsing and validation, multiple-choice and typed-answer banks, lessons, Ed25519 verification of installable packs | `tutor-core` |
| `content-tools` | Library and CLI for content work: validate packs, produce CSV/HTML review reports, apply reviewer CSVs, set review status, generate keys, sign and verify packs. The app links it with default features off (no signing code) | `subjects` |
| `storage` | Encrypted per-profile vaults, key file format, Argon2id KDF, PIN lockout, recovery codes, profile slots, encrypted export and import, state schema and migrations | `tutor-core` |
| `report` | Self-contained PDF writer with embedded TrueType fonts (Cyrillic): child sheet, teacher worksheets, keys and answer sheets, lesson plans and handouts, certificates | `tutor-core` |
| `app-core` | The application: one entry point `App::call(name, args, ctx)`; profiles and roles, lesson and task flow, review and recall, weekly ruler, badges, grade ceremony, teacher cabinet, installed packs, reviewer commands (feature `reviewer`); builds every JSON response from view types | all of the above |
| `dev-server` | Desktop HTTP server (Rust standard library only) on 127.0.0.1 for development and e2e tests | `app-core`, `storage` |
| `kmit-app` (`app/src-tauri`) | Tauri 2 Android shell: forwards `call`, saves files via the system "Save as" dialog, text-to-speech, screenshot blocking, optional Keystore hook | `app-core`, `storage` |

Dependency graph (`X -> Y` means X depends on Y):

```
tutor-engine     -> tutor-core, course-pack
generators-math  -> tutor-core
subjects         -> tutor-core
storage          -> tutor-core
report           -> tutor-core
content-tools    -> subjects
app-core         -> tutor-core, course-pack, tutor-engine, generators-math,
                    subjects, content-tools, storage, report
dev-server       -> app-core, storage
kmit-app         -> app-core, storage      (app/src-tauri, separate workspace)
```

The workspace sets `unsafe_code = "forbid"` for every crate. CI also checks that `tutor-core` has no networking dependency (`reqwest`, `hyper`, `tokio`, `ureq`, `curl`, `rustls`, `openssl`).

## Three builds

| Build | How | What it contains |
|---|---|---|
| **Child and teacher app** (`ua.kmit.tutor`, "Kmit") | `scripts/android-apk.sh` | Child profiles and teacher profiles in one app; the role is chosen when a profile is created. Child commands are refused for a teacher profile and vice versa |
| **Reviewer app** (`ua.kmit.reviewer`, "Kmit Методист") | `scripts/android-reviewer-apk.sh`, Cargo feature `reviewer` | Only review commands (`rv_*`) and recovery; child profiles, teacher commands and profile import are refused; one profile; own UI from `app/reviewer`. Distributed outside Google Play |
| **Development server** | `scripts/dev.sh`, `scripts/e2e.sh`, `scripts/e2e-reviewer.sh` | The same `app-core` behind a local HTTP server. Test-only features (`fast-kdf`, `e2e`) weaken the KDF or expose answers for automated tests and are never used in builds for people |

## Data flow: no network

- The Android manifest removes `android.permission.INTERNET` (`tools:node="remove"`), sets `usesCleartextTraffic="false"` for release builds and disables backup (`allowBackup="false"`, `fullBackupContent="false"`).
- The Tauri CSP allows only `'self'`, `data:` images and the IPC channel. The UI has no external URLs; `compliance_tests` check both.
- Content is embedded in the binary. Newer packs can be installed only from a local file pair (`<id>.json` + `<id>.sig`), and only if the Ed25519 signature matches the embedded publisher key and the version is higher. Installed packs are re-verified at every start.
- Files leave the app only when the user saves them through the system "Save as" dialog: PDFs (sheets, worksheets, certificates), the passphrase-encrypted profile backup, the reviewer's CSV and the child's optional "problem with this task" report. The app never sends anything itself.
- **Child data never reaches a teacher digitally.** The child's sheet for the teacher is a printable page without free text, timestamps or diagnoses (`tutor-core::sheet`); teacher profiles hold only their own worksheets, notes and bookmarks; there is no import of children's work.

## Storage, encryption, PIN and recovery

Each profile lives in its own slot directory `p<slot>` (up to 4 slots). Without a PIN only the slot number and avatar number in `slot.meta` are visible; the nickname is encrypted.

- **Database:** SQLCipher (`rusqlite` with `bundled-sqlcipher-vendored-openssl`) keyed with a random 256-bit data key (DEK) as a raw key; WAL, `synchronous=FULL`, `secure_delete=ON`, `cipher_memory_security=ON`, integrity check on open.
- **Key file `vault.key`:** Argon2id parameters plus the DEK wrapped twice with XChaCha20-Poly1305: once by a key derived from the PIN, once by a key derived from the recovery code. A keyed BLAKE2b MAC over the recovery section lets the app detect a damaged recovery section after a normal PIN login. Production KDF parameters: 64 MiB, 3 passes, 1 lane (still to be calibrated on real devices); parameters read from files are bounds-checked.
- **PIN lockout:** 5 free attempts, then a delay starting at 30 s and doubling up to 15 minutes, persisted on disk.
- **Recovery code:** 20 characters (100 bits, Crockford base32 without I, L, O, U), shown once, can be regenerated by an adult. Recovery opens the DEK through the second wrap and sets a new PIN.
- **Order of operations:** a new key is always checked against the database before the key file is rewritten atomically (temp file, fsync, rename, directory fsync), so a failed attempt changes nothing.
- **Erase:** deleting a profile destroys both wrapped keys and any device-bound key, which makes the data unreadable (crypto-erase).
- **Device binding (optional):** a `KeyWrapHook` can additionally wrap the PIN path with Android Keystore. It is implemented behind the `keystore` feature of `kmit-app`, **disabled by default and untested on devices**.
- **Export:** a passphrase-encrypted file for moving a profile to another device; import creates a new slot with a new PIN and recovery code.
- Secrets are held in `Zeroizing` buffers. Error messages never contain personal data or secrets.

## Content packs

Content other than generated maths tasks is data: `crates/subjects/packs/*.json` in format `kmit-pack-1` (skills with prerequisites, multiple-choice banks, typed-answer banks, lessons made of text, worked examples, quizzes, self-checks, figures and "remember" blocks, plus a `review.status` of `draft`, `reviewed` or `approved`). The loader rejects unknown schemas, cycles, unknown skills and invalid figures. Format: [docs/content-packs.md](docs/content-packs.md). Review workflow: [docs/review-workflow.md](docs/review-workflow.md).

All topics of all subjects are merged into one skill graph (`app-core::catalog`); mathematics uses generators, other subjects use banks.

## Invariants

These rules are enforced by types and tests, not by convention.

- **I1: the answer never appears in a hint.** For a computational task, the answer and every protected intermediate value are extracted as exact fractions; every hint text is scanned for numeric tokens (`tutor-core::numtok`) and a hint containing a protected value is replaced by a safe fallback (`tutor-core::guard`).
- **I2: an analogous example is safe.** It must use different numbers and a different answer, must not show any protected value of the original task or its own final answer, and its answer must not coincide with numbers from the statement, known typical errors or answers the child already entered.
- **I3: no safe analogy means no analogy.** If no safe example is found, none is shown; a solution is never substituted.
- **View types only.** Everything sent to the child UI is built from view types (`ChildTaskView`, `ChildHint`, …) that have no field for the answer, intermediate results or typical errors (`app-core::views`, `tutor-core::view`). A test drives a whole course through the API and asserts that no answer leaks (`course_tests`).
- **Factual subjects** follow a different rule by design: after the child's own attempt the correct option is shown with an explanation, and the item returns later for review.
- **Child data never goes to the teacher digitally** (see above). Teacher and child profiles are separate profile kinds with disjoint commands.
- **Rewards are never taken away** and never depend on speed or time in the app; the grade ceremony never blocks moving between grades.
- **Determinism:** the RNG, BKT arithmetic, fractions and learner-state encoding give identical results on every platform; golden tests pin generator output.

## Compliance tests

`crates/app-core/tests/compliance_tests.rs` runs in the normal test suite (not in the reviewer feature build) and fails if:

- the Android manifest grants `INTERNET` (other than as a removal), `AD_ID`, location, contacts, phone state or numbers, external storage or media, microphone, camera, Bluetooth, accounts or SMS permissions;
- the workspace `Cargo.lock` contains known ad, analytics or crash-reporting crates (Firebase, Sentry, AdMob, Mixpanel and similar);
- the Android Gradle files include Play Services ads or analytics, Firebase, Crashlytics, AppsFlyer or Facebook;
- the CSP allows non-local sources;
- UI scripts or styles reference external URLs.

Together with the CI job that rejects networking crates in `tutor-core` and the permission dump printed by the APK scripts, this makes adding network access a visible, deliberate change.

## Testing

`cargo test --workspace` runs unit, property-based (`proptest`) and integration tests across all crates (494 `#[test]` functions at the time of writing; 480 run by default, the rest need the `reviewer` feature or are `#[ignore]`d performance checks), including full-course API walks, storage hardening and migration tests, PDF structure tests and content lint tests. UI behaviour is covered by headless Chromium scenarios in `app/e2e` (light and dark themes, teacher profile, grade ceremony, reviewer app). See [docs/how-to-test.md](docs/how-to-test.md).
