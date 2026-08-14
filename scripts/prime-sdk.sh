#!/usr/bin/env bash
set -euo pipefail

action="${1:-}"
case "$action" in
  doctor | build | sim) ;;
  *)
    echo "usage: scripts/prime-sdk.sh <doctor|build|sim>" >&2
    exit 2
    ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_dir="$repo_root/app"
sdk_root="${FOUNDATION_SDK_ROOT:-$HOME/.foundation/sdk/current}"

if [[ ! -d "$sdk_root" ]]; then
  echo "Foundation SDK not found at: $sdk_root" >&2
  exit 1
fi

if [[ ! -f "$app_dir/app-config.toml" ]]; then
  echo "Prime app config not found at: $app_dir/app-config.toml" >&2
  exit 1
fi

source /etc/profile.d/nix.sh

echo "repository: $(git -C "$repo_root" rev-parse HEAD 2>/dev/null || echo uncommitted-copy)"
echo "sdk: $(git -C "$sdk_root" rev-parse HEAD 2>/dev/null || echo unknown)"
echo "action: $action"

cd "$sdk_root"
if [[ "$action" == "doctor" ]]; then
  exec nix develop --command foundation doctor
fi

if [[ "$action" == "build" ]]; then
  exec nix develop --command env \
    CC_armv7a_unknown_xous_elf=arm-none-eabi-gcc \
    AR_armv7a_unknown_xous_elf=arm-none-eabi-ar \
    bash -c 'cd "$1" && foundation "$2"' bash "$app_dir" "$action"
fi

exec nix develop --command bash -c 'cd "$1" && foundation "$2"' bash "$app_dir" "$action"
