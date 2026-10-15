# Contributing to Kmit

**English** · [Українська](CONTRIBUTING.uk.md)

Thank you for helping. Kmit is a free, offline learning companion for children in Ukraine, so careful contributions of every size matter: a corrected task, a clearer explanation, a bug report or a code change.

Kmit is **source-available**, not open source in the OSI sense. You may use, study, change and share it for **noncommercial** purposes:

| Part | Licence |
|---|---|
| Software | PolyForm Noncommercial 1.0.0, see [`LICENSE`](LICENSE) |
| Learning content and documentation | CC BY-NC-SA 4.0, see [`LICENSE-CONTENT.md`](LICENSE-CONTENT.md) |

Please read the [licensing terms for contributions](#licensing-of-contributions) below before you submit anything.

## What helps most

1. **Corrections to learning content.** Kmit's lessons and tasks were drafted with AI tools and have not yet been reviewed by human methodologists. Teachers and methodologists are the most valuable reviewers. Open an issue with the "Content error" template, or submit a fix to `crates/subjects/packs/*.json` (format: [`docs/content-packs.md`](docs/content-packs.md); review process: [`docs/review-workflow.md`](docs/review-workflow.md)).
2. **Testing on real Android devices**, especially older and low-cost phones, with a description of the device and what happened.
3. **Bug reports** with steps to reproduce. Use the "Помилка" (Bug) template.
4. **Code changes.** For anything larger than a small fix, open an issue first so that we can agree on the approach.

## Rules

- **Never add children's data or personal data** to the repository, issues or screenshots. This covers names, PINs, recovery codes, progress exports and photos of handwriting.
- **Do not copy protected texts.** This includes textbooks, articles and other apps. Write explanations yourself. Short quotations from literary works must be clearly attributed.
- **Do not add network access, advertising, analytics or tracking.** `crates/app-core/tests/compliance_tests.rs` enforces this, and pull requests that weaken it will not be accepted.
- **Keep the child's view hint-based.** The app never shows a child the numeric answer instead of a hint (invariant I1).
- **Mark AI assistance.** If you use AI tools, you remain responsible for what you submit. Check every fact and say in the pull request that AI was used.

## Before you open a pull request

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check_ui.sh                      # UI syntax and compatibility with older WebViews
python3 scripts/pack_lint.py crates/subjects/packs/<pack>.json   # if you changed a content pack
```

If you changed the user interface, also run `scripts/e2e.sh` (with `THEME=light` and `THEME=dark`) and, for the reviewer app, `scripts/e2e-reviewer.sh`. Both need Chrome or Chromium. Build instructions are in [`BUILDING.md`](BUILDING.md).

## Licensing of contributions

Kmit is offered free of charge for noncommercial use. Commercial use is possible under a separate agreement with the copyright holder. That agreement covers, for example, publishing in an app store or a contract with a school network. For this to remain possible after outside contributions, every contributor agrees to the following. These terms apply to code, learning content, documentation, translations, reviews and any other material you submit.

1. **Certificate of origin.** You certify the [Developer Certificate of Origin 1.1](DCO) for your contribution. You do this by adding a `Signed-off-by: Your Name <email>` line to every commit (`git commit -s`). Use your real name or a name you are publicly known by.
2. **Same licence as the project.** Your contribution is licensed to everyone under the licence that applies to the files it changes. That is PolyForm Noncommercial 1.0.0 for software and CC BY-NC-SA 4.0 for learning content and documentation ([`REUSE.toml`](REUSE.toml) maps paths to licences).
3. **Additional licence to the copyright holder.** You also grant Oleksandr Shynkarenko (Олександр Шинкаренко), and his successors and assigns, a perpetual, worldwide, non-exclusive, royalty-free and irrevocable licence to:
   - use, reproduce, modify, publish and distribute your contribution, alone or as part of Kmit;
   - sublicense and relicense it under any terms, including commercial terms.

   For contributions of code, this includes a licence under any patent claims you can license that are necessarily infringed by your contribution, alone or combined with Kmit.
4. **You keep your copyright.** You remain the author and may use your contribution in any other way.
5. **You have the right to contribute.** If your employer or school has rights in your work, you have their permission. You also confirm that your contribution does not include material you are not allowed to share.
6. **No warranty.** Your contribution is provided "as is", without warranty of any kind.

If you cannot agree to point 3, please do not submit a contribution. Instead, open an issue that describes the problem and let us write the fix. If a contributor's terms change, we will ask before using their contribution in a new way.

## Questions and contact

- **Questions about contributing:** open an issue.
- **Commercial licensing and permissions:** <sasha.piscivca@gmail.com>.
- **Security or privacy problems:** follow [`SECURITY.md`](SECURITY.md) and do not use public issues.
- **Code of conduct:** everyone taking part follows [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
