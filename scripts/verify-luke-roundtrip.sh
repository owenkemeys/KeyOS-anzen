#!/usr/bin/env bash
set -euo pipefail

readonly UPSTREAM_URL=https://github.com/lukechilds/anzen.git
readonly UPSTREAM_COMMIT=01794bb14d34d01413e3b539c7e5496ecbce0c87
readonly DEVELOPMENT_SEED=4242424242424242424242424242424242424242424242424242424242424242
readonly HWW_MNEMONIC='drastic bamboo mountain loyal category cancel animal embark drastic bamboo mountain loyal category cancel animal embark drastic bamboo mountain loyal category cancel animal embark'
readonly HWW_PUBLIC_KEY=906979e124fa1d21104bbcfce84d825cb1269fa1dc95a0a27e9bfe120d88e97c

repo_root=$(cd "$(dirname "$0")/.." && pwd)
work_root=$(mktemp -d)
upstream=${ANZEN_UPSTREAM_DIR:-$work_root/upstream-anzen}
export COMPOSE_PROJECT_NAME="keyos-anzen-parity-$$"

cleanup() {
    if [[ -f $upstream/compose.yaml ]]; then
        docker compose --project-directory "$upstream" down --volumes --remove-orphans \
            >/dev/null 2>&1 || true
    fi
    rm -rf -- "$work_root"
}
trap cleanup EXIT

if [[ ! -d $upstream/.git ]]; then
    git clone --filter=blob:none --no-checkout "$UPSTREAM_URL" "$upstream"
fi
git -C "$upstream" fetch --depth 1 origin "$UPSTREAM_COMMIT"
git -C "$upstream" checkout --detach "$UPSTREAM_COMMIT"
test "$(git -C "$upstream" rev-parse HEAD)" = "$UPSTREAM_COMMIT"

cargo build --locked -p anzen-prime-adapter
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" build cli
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" up --detach --wait bitcoind

anzen() {
    COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
        --volume "$work_root:/work" cli --data-dir /data "$@"
}

anzen phone init >/dev/null
anzen hww init >/dev/null
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --entrypoint sh cli -c \
    "mkdir -p /data/hww && printf '%s\n' '{\"kind\":\"hww\",\"network\":\"regtest\",\"mnemonic\":\"$HWW_MNEMONIC\",\"vault_key_index\":0}' > /data/hww/device.json && printf '%s\n' '{\"version\":1,\"kind\":\"hww-public-key\",\"network\":\"regtest\",\"vault_pubkey\":\"$HWW_PUBLIC_KEY\"}' > /data/hww/public.json"

init_output=$(anzen init)
vault_address=$(printf '%s\n' "$init_output" | sed -n 's/^Vault address: //p')
test -n "$vault_address"
hot_output=$(anzen phone receive-address)
mining_address=$(printf '%s\n' "$hot_output" | sed -n 's/^Hot receive address: //p')
test -n "$mining_address"

anzen node mine 101 "$mining_address" >/dev/null
anzen phone send "$vault_address" 200000000 >/dev/null
anzen node mine 1 "$mining_address" >/dev/null
anzen phone set-policy \
    --monthly-limit 10000000 \
    --emergency-access-limit 50000000 \
    --output /work/policy.json >/dev/null
test -s "$work_root/policy.json"

"$repo_root/target/debug/anzen-prime-adapter" approve \
    "$work_root/policy.json" \
    "$work_root/approved-policy.json" \
    "$DEVELOPMENT_SEED"
test -s "$work_root/approved-policy.json"

activation=$(anzen phone activate-policy /work/approved-policy.json)
printf '%s\n' "$activation"
printf '%s\n' "$activation" | grep -q '^Rollover broadcast: '
printf '%s\n' "$activation" | grep -q '^Active monthly limit: 10000000 sats$'
printf '%s\n' "$activation" | grep -q '^Encrypted allowance transaction pairs: 12$'
printf '%s\n' "$activation" | grep -q '^Active emergency access: 50000000 sats$'

printf 'Luke upstream: %s\n' "$UPSTREAM_COMMIT"
printf 'Proposal SHA-256: '
sha256sum "$work_root/policy.json" | cut -d' ' -f1
printf 'Approved SHA-256: '
sha256sum "$work_root/approved-policy.json" | cut -d' ' -f1
printf 'Real regtest policy-package round trip passed.\n'
