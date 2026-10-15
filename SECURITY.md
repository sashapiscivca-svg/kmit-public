# Security and privacy

**English** · [Українська](SECURITY.uk.md)

Kmit is used by children, so security and privacy come first. The app has no network permission, collects no data and keeps everything on the device in an encrypted database (SQLCipher) protected by a PIN. The draft privacy policy is in [`docs/legal/privacy-policy.en.md`](docs/legal/privacy-policy.en.md).

## How to report a problem

Please report privately. **Do not** open a public issue until the problem is fixed.

- Preferred: GitHub private vulnerability reporting on this repository (**Security → Report a vulnerability**).
- Or email Oleksandr Shynkarenko at <sasha.piscivca@gmail.com> with "Kmit security" in the subject.

Include what you found, how to reproduce it, the app version or commit, and the device and Android version if relevant. Never send real children's data, PINs, recovery codes or signing keys. We will acknowledge your report within 7 days and keep you informed until it is resolved. We are happy to credit you, if you wish.

## What we treat as serious

- Any transmission of data off the device, or any way to enable network access.
- Bypassing the PIN, the database encryption or the recovery mechanism.
- Showing a child the numeric answer instead of a hint (invariant I1).
- A child's data becoming visible in the teacher or methodologist profile.
- Content that is harmful or inappropriate for children.

## Supported versions

Kmit is pre-release software. Fixes are made on the `main` branch.
