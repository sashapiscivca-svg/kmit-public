#!/usr/bin/env bash
# Збирає й підписує ТЕСТОВИЙ APK (arm64). Ключ створюється локально й не годиться для публікації (Q30).
# Потрібне середовище збірки Android: див. BUILDING.md (змінна ENV_SH вказує на ваш файл з exports).
set -euo pipefail
source "${ENV_SH:-$HOME/.cache/kmit-s0/env.sh}"
cd "$(dirname "$0")/../app/src-tauri"
nice -n 10 ionice -c3 cargo tauri android build --apk --target aarch64
(cd gen/android && ./gradlew --stop >/dev/null 2>&1 || true)
BT="$(ls -d "$ANDROID_HOME"/build-tools/* | tail -1)"
OUT=gen/android/app/build/outputs/apk/universal/release
D="${RELEASE_DIR:-$HOME/kmit-release}"
mkdir -p "${D:?}"
# Постійний ключ видавця: KMIT_KEYSTORE=/шлях.jks KMIT_KEY_ALIAS=… KMIT_STORE_PASS=… KMIT_KEY_PASS=… (без нього створюється тестовий).
[ -z "${KMIT_KEYSTORE:-}" ] || [ -f "$KMIT_KEYSTORE" ] || { echo "KMIT_KEYSTORE вказує на неіснуючий файл: $KMIT_KEYSTORE" >&2; exit 1; }
KS="${KMIT_KEYSTORE:-$D/test-key.jks}"; KA="${KMIT_KEY_ALIAS:-test}"; SP="${KMIT_STORE_PASS:-testtest}"; KP="${KMIT_KEY_PASS:-testtest}"
[ -f "$KS" ] || keytool -genkeypair -keystore "$D/test-key.jks" -storepass testtest -keypass testtest \
  -alias test -keyalg RSA -keysize 2048 -validity 3650 -dname "CN=kmit test key"
"$BT/zipalign" -f -p 4 "$OUT/app-universal-release-unsigned.apk" "$D/aligned.apk"
"$BT/apksigner" sign --ks "$KS" --ks-key-alias "$KA" --ks-pass "pass:$SP" --key-pass "pass:$KP" \
  --out "$D/kmit-tutor-arm64-test.apk" "$D/aligned.apk"
rm -f "${D:?}/aligned.apk" "${D:?}/kmit-tutor-arm64-test.apk.idsig"
"$BT/apksigner" verify "$D/kmit-tutor-arm64-test.apk" && echo "OK: $D/kmit-tutor-arm64-test.apk"
echo "Дозволи (INTERNET не має бути):"; "$BT/aapt2" dump permissions "$D/kmit-tutor-arm64-test.apk"
