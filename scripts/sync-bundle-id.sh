#!/usr/bin/env bash
# Apply ANDROID_APPLICATION_ID from .env (or the environment) to Dioxus.toml.
# Empty / unset → com.example.demo (Maestro / CI default).
# Maestro flows use appId: ${ANDROID_APPLICATION_ID}; pass -e when running maestro.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

default_id="com.example.demo"
app_id="${ANDROID_APPLICATION_ID:-}"
if [ -z "$app_id" ] && [ -f .env ]; then
  app_id="$(
    grep -E '^[[:space:]]*ANDROID_APPLICATION_ID=' .env \
      | tail -1 \
      | sed 's/^[[:space:]]*ANDROID_APPLICATION_ID=//' \
      | sed 's/^["'\'']//;s/["'\'']$//' \
      | tr -d '\r'
  )"
fi
app_id="$(echo -n "$app_id" | xargs)"
if [ -z "$app_id" ]; then
  app_id="$default_id"
fi

python3 - "$app_id" <<'PY'
import pathlib, re, sys
app_id = sys.argv[1]
dioxus = pathlib.Path("Dioxus.toml")
text = dioxus.read_text()
new, n = re.subn(
    r'(?m)^(identifier\s*=\s*)".*"',
    rf'\1"{app_id}"',
    text,
    count=1,
)
if n != 1:
    raise SystemExit("Dioxus.toml: could not update [bundle] identifier")
dioxus.write_text(new)
print(f"Synced Dioxus.toml identifier → {app_id}")
PY
