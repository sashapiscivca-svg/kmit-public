#!/usr/bin/env bash
# Збирає й підписує ТЕСТОВИЙ APK застосунку методиста (arm64): окремий застосунок поза Google Play, розсилається поштою.
# Інший applicationId (ua.kmit.reviewer) і назва «Kmit Методист», feature `reviewer` (лише вичитка, без дитячих профілів).
# Середовище збірки Android і підпис: див. BUILDING.md (змінна ENV_SH вказує на ваш файл з exports).
set -euo pipefail
source "${ENV_SH:-$HOME/.cache/kmit-s0/env.sh}"
cd "$(dirname "$0")/.."
scripts/build-reviewer-ui.sh
cd app/src-tauri
export ORG_GRADLE_PROJECT_kmitAppId=ua.kmit.reviewer
RES=gen/android/app/src/main/res
# Що б не сталося, робоче дерево повертається до дитячого вигляду (назва, значки).
trap 'git checkout -- "$RES" 2>/dev/null || true' EXIT
sed -i 's/"Kmit"/"Kmit Методист"/' $RES/values/strings.xml
cp -r icons-reviewer/res/. $RES/   # значок методиста (scripts/make_icons.py reviewer)
nice -n 10 ionice -c3 cargo tauri android build --apk --target aarch64 --features reviewer --config '{"productName":"Kmit Методист","build":{"frontendDist":"../reviewer-dist"}}'
(cd gen/android && ./gradlew --stop >/dev/null 2>&1 || true)
git checkout -- $RES 2>/dev/null || true
BT="$(ls -d "$ANDROID_HOME"/build-tools/* | tail -1)"
OUT=gen/android/app/build/outputs/apk/universal/release
D="${RELEASE_DIR:-$HOME/kmit-release}"
mkdir -p "${D:?}"
# Постійний ключ видавця: KMIT_KEYSTORE=/шлях.jks KMIT_KEY_ALIAS=… KMIT_STORE_PASS=… KMIT_KEY_PASS=… (без нього створюється тестовий).
[ -z "${KMIT_KEYSTORE:-}" ] || [ -f "$KMIT_KEYSTORE" ] || { echo "KMIT_KEYSTORE вказує на неіснуючий файл: $KMIT_KEYSTORE" >&2; exit 1; }
KS="${KMIT_KEYSTORE:-$D/test-key.jks}"; KA="${KMIT_KEY_ALIAS:-test}"; SP="${KMIT_STORE_PASS:-testtest}"; KP="${KMIT_KEY_PASS:-testtest}"
[ -f "$KS" ] || keytool -genkeypair -keystore "$D/test-key.jks" -storepass testtest -keypass testtest \
  -alias test -keyalg RSA -keysize 2048 -validity 3650 -dname "CN=kmit test key"
"$BT/zipalign" -f -p 4 "$OUT/app-universal-release-unsigned.apk" "$D/aligned-reviewer.apk"
"$BT/apksigner" sign --ks "$KS" --ks-key-alias "$KA" --ks-pass "pass:$SP" --key-pass "pass:$KP" \
  --out "$D/kmit-reviewer-arm64-test.apk" "$D/aligned-reviewer.apk"
rm -f "${D:?}/aligned-reviewer.apk" "${D:?}/kmit-reviewer-arm64-test.apk.idsig"
"$BT/apksigner" verify "$D/kmit-reviewer-arm64-test.apk" && echo "OK: $D/kmit-reviewer-arm64-test.apk"
echo "Пакет і дозволи (INTERNET не має бути):"; "$BT/aapt2" dump badging "$D/kmit-reviewer-arm64-test.apk" | grep -E "^package|application-label:"; "$BT/aapt2" dump permissions "$D/kmit-reviewer-arm64-test.apk"
