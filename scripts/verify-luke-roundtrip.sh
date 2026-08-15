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
        COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
            --volume "$work_root:/work" --entrypoint sh cli -c \
            'chmod -R a+rwX /work' >/dev/null 2>&1 || true
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

anzen_at() {
    local data_dir=$1
    shift
    COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
        --volume "$work_root:/work" cli --data-dir "/work/$data_dir" "$@"
}

bitcoin_cli() {
    COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" exec -T bitcoind \
        bitcoin-cli -regtest -rpcconnect=127.0.0.1 -rpcuser=anzen -rpcpassword=anzen "$@"
}

mine_delayed_recovery_blocks() {
    local destination=$1
    local remaining=65534
    local batch
    local mock_time
    mock_time=$(bitcoin_cli getblockheader "$(bitcoin_cli getbestblockhash)" | jq -r .time)
    while ((remaining > 0)); do
        batch=5000
        if ((remaining < batch)); then
            batch=$remaining
        fi
        mock_time=$((mock_time + batch + 1))
        bitcoin_cli setmocktime "$mock_time" >/dev/null
        bitcoin_cli generatetoaddress "$batch" "$destination" >/dev/null
        remaining=$((remaining - batch))
    done
    bitcoin_cli setmocktime 0 >/dev/null
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
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c 'chmod 0644 /work/policy.json'

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

anzen social generate-friend-key \
    --name 'Prime parity friend <friend@example.test>' \
    --public-key /work/friend-public.asc \
    --private-key /work/friend-private.asc >/dev/null
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c \
    'cp /data/anzen.json /work/friend-config.json && cp /data/cloud/phone-seed-backup.json /work/friend-current-backup.json && chmod 0644 /work/friend-public.asc /work/friend-config.json /work/friend-current-backup.json'
"$repo_root/target/debug/anzen-prime-adapter" add-recovery-friend \
    "$work_root/friend-current-backup.json" \
    "$work_root/friend-config.json" \
    "$work_root/friend-public.asc" \
    "$work_root/friend-added-backup.json" \
    "$DEVELOPMENT_SEED"
test "$(jq '.friends | length' "$work_root/friend-added-backup.json")" = 1
friend_fingerprint=$(jq -r '.friends[0].fingerprint' "$work_root/friend-added-backup.json")
old_friend_wrapper=$(jq -r '.friends[0].encrypted_symmetric_key' "$work_root/friend-added-backup.json")
test -n "$friend_fingerprint"
test -n "$old_friend_wrapper"
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c \
    'cp /work/friend-added-backup.json /data/cloud/phone-seed-backup.json'
anzen social decrypt-backup /data/cloud/phone-seed-backup.json \
    --private-key /work/friend-private.asc \
    --output /work/friend-recovery-before-rotation.json >/dev/null
test -s "$work_root/friend-recovery-before-rotation.json"

anzen node mine 1 "$mining_address" >/dev/null
anzen phone rotate-key --output /work/rotation.json >/dev/null
test -s "$work_root/rotation.json"
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c \
    'cp /data/anzen.json /work/current-config.json && cp /data/phone/pending-rotation.json /work/pending-rotation.json && cp /data/cloud/phone-seed-backup.json /work/current-backup.json && chmod 0644 /work/rotation.json /work/current-config.json /work/pending-rotation.json /work/current-backup.json'

"$repo_root/target/debug/anzen-prime-adapter" approve-rotation \
    "$work_root/rotation.json" \
    "$work_root/current-config.json" \
    "$work_root/pending-rotation.json" \
    "$work_root/current-backup.json" \
    "$work_root/approved-rotation.json" \
    "$DEVELOPMENT_SEED"
test -s "$work_root/approved-rotation.json"

rotation=$(anzen phone activate-rotation /work/approved-rotation.json)
printf '%s\n' "$rotation"
printf '%s\n' "$rotation" | grep -q '^Emergency phone-key rotation broadcast: '
printf '%s\n' "$rotation" | grep -q '^Monthly policy preserved: 10000000 sats$'
printf '%s\n' "$rotation" | grep -q '^Emergency access preserved: 50000000 sats$'

COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c \
    'cp /data/cloud/phone-seed-backup.json /work/rotated-phone-backup.json && chmod 0644 /work/rotated-phone-backup.json'
test "$(jq '.friends | length' "$work_root/rotated-phone-backup.json")" = 1
test "$(jq -r '.friends[0].fingerprint' "$work_root/rotated-phone-backup.json")" = "$friend_fingerprint"
new_friend_wrapper=$(jq -r '.friends[0].encrypted_symmetric_key' "$work_root/rotated-phone-backup.json")
test -n "$new_friend_wrapper"
test "$new_friend_wrapper" != "$old_friend_wrapper"
anzen social decrypt-backup /data/cloud/phone-seed-backup.json \
    --private-key /work/friend-private.asc \
    --output /work/friend-phone-recovery.json >/dev/null
test -s "$work_root/friend-phone-recovery.json"
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c \
    'chmod 0644 /work/friend-recovery-before-rotation.json /work/friend-phone-recovery.json'

anzen hww decrypt-phone-backup /data/cloud/phone-seed-backup.json \
    --output /work/luke-phone-recovery.json >/dev/null
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c \
    'cp /data/anzen.json /work/backup-config.json && cp /data/cloud/phone-seed-backup.json /work/phone-backup.json && chmod 0644 /work/luke-phone-recovery.json /work/backup-config.json /work/phone-backup.json'
"$repo_root/target/debug/anzen-prime-adapter" decrypt-phone-backup \
    "$work_root/phone-backup.json" \
    "$work_root/backup-config.json" \
    "$work_root/prime-phone-recovery.json" \
    "$DEVELOPMENT_SEED"
jq -S . "$work_root/luke-phone-recovery.json" >"$work_root/luke-phone-recovery.normalized.json"
jq -S . "$work_root/prime-phone-recovery.json" >"$work_root/prime-phone-recovery.normalized.json"
cmp "$work_root/luke-phone-recovery.normalized.json" "$work_root/prime-phone-recovery.normalized.json"
jq -S . "$work_root/friend-phone-recovery.json" >"$work_root/friend-phone-recovery.normalized.json"
cmp "$work_root/luke-phone-recovery.normalized.json" "$work_root/friend-phone-recovery.normalized.json"

anzen node mine 1 "$mining_address" >/dev/null
anzen phone create-sweep "$mining_address" --output /work/sweep.json >/dev/null
test -s "$work_root/sweep.json"
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c 'chmod 0644 /work/sweep.json'

"$repo_root/target/debug/anzen-prime-adapter" approve-sweep \
    "$work_root/sweep.json" \
    "$work_root/approved-sweep.json" \
    "$DEVELOPMENT_SEED"
test -s "$work_root/approved-sweep.json"

sweep=$(anzen phone broadcast-sweep /work/approved-sweep.json)
printf '%s\n' "$sweep"
printf '%s\n' "$sweep" | grep -q '^Cooperative vault sweep broadcast: '
printf '%s\n' "$sweep" | grep -q '^Inputs: '
printf '%s\n' "$sweep" | grep -q '^Fee: .* sats (1 sat/vB)$'
anzen node mine 1 "$mining_address" >/dev/null

for recovery_dir in luke-recovery prime-recovery; do
    anzen_at "$recovery_dir" phone init >/dev/null
    COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
        --volume "$work_root:/work" --entrypoint sh cli -c \
        "mkdir -p /work/$recovery_dir/hww && printf '%s\\n' '{\"kind\":\"hww\",\"network\":\"regtest\",\"mnemonic\":\"$HWW_MNEMONIC\",\"vault_key_index\":0}' > /work/$recovery_dir/hww/device.json && printf '%s\\n' '{\"version\":1,\"kind\":\"hww-public-key\",\"network\":\"regtest\",\"vault_pubkey\":\"$HWW_PUBLIC_KEY\"}' > /work/$recovery_dir/hww/public.json"
done

luke_recovery_init=$(anzen_at luke-recovery init)
luke_recovery_address=$(printf '%s\n' "$luke_recovery_init" | sed -n 's/^Vault address: //p')
prime_recovery_init=$(anzen_at prime-recovery init)
prime_recovery_address=$(printf '%s\n' "$prime_recovery_init" | sed -n 's/^Vault address: //p')
test -n "$luke_recovery_address"
test -n "$prime_recovery_address"
COMPOSE_PROGRESS=quiet docker compose --project-directory "$upstream" run --rm --no-deps \
    --volume "$work_root:/work" --entrypoint sh cli -c \
    'chmod -R a+rX /work/luke-recovery /work/prime-recovery'

recovery_funding_address=$(anzen phone receive-address | sed -n 's/^Hot receive address: //p')
test -n "$recovery_funding_address"
anzen node mine 101 "$recovery_funding_address" >/dev/null
anzen phone send "$luke_recovery_address" 1000000 >/dev/null
anzen phone send "$prime_recovery_address" 1000000 >/dev/null
anzen node mine 1 "$mining_address" >/dev/null
mine_delayed_recovery_blocks "$mining_address"

luke_recovery=$(anzen_at luke-recovery hww recover "$mining_address")
printf '%s\n' "$luke_recovery"
printf '%s\n' "$luke_recovery" | grep -q '^HWW recovery sweep broadcast: '

tip_height=$(bitcoin_cli getblockcount)
prime_scan=$(bitcoin_cli scantxoutset start "[\"addr($prime_recovery_address)\"]")
jq -n \
    --arg destination "$mining_address" \
    --argjson tip "$tip_height" \
    --slurpfile config "$work_root/prime-recovery/anzen.json" \
    --argjson scan "$prime_scan" \
    '{
      version: 1,
      kind: "hww-recovery-snapshot",
      network: $config[0].network,
      vault_descriptor: $config[0].vault_descriptor,
      destination: $destination,
      tip_height: $tip,
      utxos: [$scan.unspents[] | {
        txid: .txid,
        vout: .vout,
        value_sats: (.amount * 100000000 | round),
        script_pubkey: .scriptPubKey,
        confirmation_height: .height
      }]
    }' >"$work_root/hww-recovery-snapshot.json"

jq '.tip_height -= 1' "$work_root/hww-recovery-snapshot.json" \
    >"$work_root/early-hww-recovery-snapshot.json"
if "$repo_root/target/debug/anzen-prime-adapter" approve-hww-recovery \
    "$work_root/early-hww-recovery-snapshot.json" \
    "$work_root/early-hww-recovery-result.json" \
    "$DEVELOPMENT_SEED"; then
    echo "immature HWW recovery snapshot was unexpectedly approved" >&2
    exit 1
fi
test ! -e "$work_root/early-hww-recovery-result.json"

"$repo_root/target/debug/anzen-prime-adapter" approve-hww-recovery \
    "$work_root/hww-recovery-snapshot.json" \
    "$work_root/hww-recovery-result.json" \
    "$DEVELOPMENT_SEED"
recovery_hex=$(jq -r .transaction_hex "$work_root/hww-recovery-result.json")
expected_recovery_txid=$(jq -r .txid "$work_root/hww-recovery-result.json")
actual_recovery_txid=$(bitcoin_cli sendrawtransaction "$recovery_hex")
test "$actual_recovery_txid" = "$expected_recovery_txid"

printf 'Luke upstream: %s\n' "$UPSTREAM_COMMIT"
printf 'Proposal SHA-256: '
sha256sum "$work_root/policy.json" | cut -d' ' -f1
printf 'Approved SHA-256: '
sha256sum "$work_root/approved-policy.json" | cut -d' ' -f1
printf 'Sweep proposal SHA-256: '
sha256sum "$work_root/sweep.json" | cut -d' ' -f1
printf 'Approved sweep SHA-256: '
sha256sum "$work_root/approved-sweep.json" | cut -d' ' -f1
printf 'Rotation proposal SHA-256: '
sha256sum "$work_root/rotation.json" | cut -d' ' -f1
printf 'Approved rotation SHA-256: '
sha256sum "$work_root/approved-rotation.json" | cut -d' ' -f1
printf 'Friend-enrolled backup SHA-256: '
sha256sum "$work_root/friend-added-backup.json" | cut -d' ' -f1
printf 'Rotated friend backup SHA-256: '
sha256sum "$work_root/rotated-phone-backup.json" | cut -d' ' -f1
printf 'HWW recovery result SHA-256: '
sha256sum "$work_root/hww-recovery-result.json" | cut -d' ' -f1
printf 'Real regtest policy-package, recovery-friend, phone-rotation, cooperative-sweep, and delayed-HWW-recovery round trips passed.\n'
