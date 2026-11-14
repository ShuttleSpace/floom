#!/usr/bin/env bash
# Locally self-sign the Floom OHOS demo HAP so it installs on the HarmonyOS/OpenHarmony
# emulator. Adapted from the verified hiddify-app local-debug recipe; kept fully
# self-contained (own keystores, plaintext dev password, no AGC).
#
# shape forced by hap-sign-tool:
#   -appCertFile  = app leaf + root CA (a 2-cert chain)
#   -profileFile  = a signed Provision Profile (p7b) whose bundle-name matches
#                   the HAP, type=debug, and device-ids=[] (any emulator)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIG="$HERE/signing"
TOOL=/Applications/DevEco-Studio.app/Contents/sdk/default/openharmony/toolchains/lib/hap-sign-tool.jar
JAVA=/Users/humphrey/.local/share/mise/installs/java/21.0.0/bin/java
UNSIGNED="$HERE/entry/build/default/outputs/default/entry-default-unsigned.hap"
SIGNED="$HERE/floom-signed.hap"
BUNDLE="com.honorbox.reminder"
PWD_APP="floom-dev-2026"
PWD_PROF="floom-prof-ca-2026"
DAYS=3650
ST() { "$JAVA" -jar "$TOOL" "$@"; }

APP_KS="$SIG/floom-app.p12";   APP_ROOT="$SIG/root-ca.cer";   APP_LEAF="$SIG/app-leaf.cer";   APP_CHAIN="$SIG/app-chain.pem"
PROF_KS="$SIG/profile-ks.p12"; PR_ROOT="$SIG/prof-root.cer";  PR_SUB="$SIG/prof-sub.cer";    PROF_CERT="$SIG/profile-cert.cer"
PROF_JSON="$SIG/profile.json"; PROF_P7B="$SIG/signed-profile.p7b"
A_APP="floom-app"; A_RCA="prof_root_ca"; A_SUB="prof_sub_ca"; A_PROF="prof_leaf"
S_ROOT="C=CN,O=Floom,OU=OHOS,CN=Floom OHOS Root CA"
S_APP="C=CN,O=Floom,OU=OHOS,CN=floom-app"
S_PR_ROOT="C=CN,O=Floom,OU=OHOS,CN=Floom Profile Root CA"
S_PR_SUB="C=CN,O=Floom,OU=OHOS,CN=Floom Profile Signing CA"
S_PR_LEAF="C=CN,O=Floom,OU=OHOS,CN=Floom Profile"

rm -rf "$SIG"; mkdir -p "$SIG"
echo "== 1/6 app keystore + root CA + leaf + chain =="
ST generate-keypair -keyAlias "$A_APP" -keyPwd "$PWD_APP" -keyAlg ECC -keySize NIST-P-256 -keystoreFile "$APP_KS" -keystorePwd "$PWD_APP" >/dev/null
ST generate-ca -keyAlias "$A_APP" -keyPwd "$PWD_APP" -keyAlg ECC -keySize NIST-P-256 -subject "$S_ROOT" -validity "$DAYS" -signAlg SHA256withECDSA -keystoreFile "$APP_KS" -keystorePwd "$PWD_APP" -basicConstraintsPathLen 3 -outFile "$APP_ROOT" >/dev/null
ST generate-cert -keyAlias "$A_APP" -keyPwd "$PWD_APP" -issuer "$S_ROOT" -issuerKeyAlias "$A_APP" -issuerKeyPwd "$PWD_APP" -subject "$S_APP" -validity "$DAYS" -keyUsage digitalSignature -keyUsageCritical false -extKeyUsage codeSignature -signAlg SHA256withECDSA -keystoreFile "$APP_KS" -keystorePwd "$PWD_APP" -outFile "$APP_LEAF" >/dev/null
cat "$APP_LEAF" "$APP_ROOT" > "$APP_CHAIN"

echo "== 2/6 profile CA 3-level chain =="
ST generate-keypair -keyAlias "$A_RCA" -keyPwd "$PWD_PROF" -keyAlg ECC -keySize NIST-P-256 -keystoreFile "$PROF_KS" -keystorePwd "$PWD_PROF" >/dev/null
ST generate-keypair -keyAlias "$A_SUB" -keyPwd "$PWD_PROF" -keyAlg ECC -keySize NIST-P-256 -keystoreFile "$PROF_KS" -keystorePwd "$PWD_PROF" >/dev/null
ST generate-keypair -keyAlias "$A_PROF" -keyPwd "$PWD_PROF" -keyAlg ECC -keySize NIST-P-256 -keystoreFile "$PROF_KS" -keystorePwd "$PWD_PROF" >/dev/null
ST generate-ca -keyAlias "$A_RCA" -keyPwd "$PWD_PROF" -keyAlg ECC -keySize NIST-P-256 -subject "$S_PR_ROOT" -validity "$DAYS" -signAlg SHA256withECDSA -keystoreFile "$PROF_KS" -keystorePwd "$PWD_PROF" -basicConstraintsPathLen 3 -outFile "$PR_ROOT" >/dev/null
ST generate-ca -keyAlias "$A_SUB" -keyPwd "$PWD_PROF" -keyAlg ECC -keySize NIST-P-256 -issuer "$S_PR_ROOT" -issuerKeyAlias "$A_RCA" -issuerKeyPwd "$PWD_PROF" -subject "$S_PR_SUB" -validity "$DAYS" -signAlg SHA256withECDSA -keystoreFile "$PROF_KS" -keystorePwd "$PWD_PROF" -basicConstraintsPathLen 0 -outFile "$PR_SUB" >/dev/null
ST generate-profile-cert -keyAlias "$A_PROF" -keyPwd "$PWD_PROF" -issuer "$S_PR_SUB" -issuerKeyAlias "$A_SUB" -issuerKeyPwd "$PWD_PROF" -subject "$S_PR_LEAF" -validity "$DAYS" -signAlg SHA256withECDSA -keystoreFile "$PROF_KS" -keystorePwd "$PWD_PROF" -outForm certChain -rootCaCertFile "$PR_ROOT" -subCaCertFile "$PR_SUB" -outFile "$PROF_CERT" >/dev/null

echo "== 3/6 provision profile json (bundle=$BUNDLE, any device) =="
python3 - "$APP_LEAF" "$BUNDLE" "$PROF_JSON" <<'PY'
import json, sys, time, uuid
cer, bundle, out = sys.argv[1:4]
leaf = open(cer).read().split("-----END CERTIFICATE-----")[0] + "-----END CERTIFICATE-----\n"
now = int(time.time())
p = {"version-name": "1.0.0", "version-code": 1, "uuid": str(uuid.uuid4()),
     "validity": {"not-before": now, "not-after": now + 3650*86400},
     "type": "debug",
     "bundle-info": {"developer-id": "Floom", "development-certificate": leaf,
                     "bundle-name": bundle, "app-feature": "hos_normal_app", "apl": "normal"},
     "permissions": {"restricted-permissions": []},
     "debug-info": {"device-ids": []},
     "issuer": "pki_internal"}
json.dump(p, open(out, "w"), indent=2)
PY

echo "== 4/6 sign profile -> p7b =="
ST sign-profile -mode localSign -keyAlias "$A_PROF" -keyPwd "$PWD_PROF" -profileCertFile "$PROF_CERT" -inFile "$PROF_JSON" -signAlg SHA256withECDSA -keystoreFile "$PROF_KS" -keystorePwd "$PWD_PROF" -outFile "$PROF_P7B" >/dev/null

echo "== 5/6 sign HAP =="
# -signCode 0 skips ELF code-signing: the bundled hap-sign-tool's
# commons-compress chokes on the zip64 CEN header of the native .so entries
# ("Invalid CEN header / invalid zip64 extra data field size"). Code signing is
# not required for a local debug install on the emulator.
ST sign-app -mode localSign -keyAlias "$A_APP" -keyPwd "$PWD_APP" -appCertFile "$APP_CHAIN" -profileFile "$PROF_P7B" -inFile "$UNSIGNED" -signAlg SHA256withECDSA -keystoreFile "$APP_KS" -keystorePwd "$PWD_APP" -outFile "$SIGNED" -signCode 0 >/dev/null
echo "== 6/6 done =="
ls -la "$SIGNED"
echo "SIGNED OK"
