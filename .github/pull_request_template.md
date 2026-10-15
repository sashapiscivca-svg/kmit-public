## What changed · Що змінено
<!-- Briefly: what and why. For content: unit (subject × grade) and topics. · Коротко: що й навіщо. Для вмісту: одиниця (предмет × клас), теми. -->

## Checks · Перевірка
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [ ] `scripts/check_ui.sh`; if the UI changed · якщо змінювався інтерфейс: `scripts/e2e.sh` (light + dark), `scripts/e2e-reviewer.sh`
- [ ] Content · Вміст: `scripts/pack_lint.py` passes for the changed pack · проходить для зміненого пакета; facts checked against reliable sources · факти звірено з надійними джерелами
- [ ] No children's data, personal data, secrets or copied third-party texts · Немає даних дітей, персональних даних, секретів чи скопійованих чужих текстів
- [ ] AI tools used? Say so here and confirm you checked the result · Чи використовували ШІ? Вкажіть і підтвердьте, що перевірили результат

## Licensing · Ліцензування (required · обов'язково)
- [ ] Every commit is signed off (`git commit -s`) under the [DCO](https://github.com/sashapiscivca-svg/kmit/blob/main/DCO) · Кожен коміт підписано (`git commit -s`) згідно з DCO
- [ ] I agree to the [licensing of contributions](https://github.com/sashapiscivca-svg/kmit/blob/main/CONTRIBUTING.md#licensing-of-contributions), including the additional licence to the copyright holder (point 3) · Я погоджуюся з умовами ліцензування внесків, зокрема з додатковою ліцензією правовласникові (пункт 3)
