# Delayed HWW recovery v1 evidence

## Upstream boundary

- Luke repository: `https://github.com/lukechilds/anzen`
- Commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Operation: `hww recover`
- Delay: 65,535 blocks

## Represented boundary

Passport Prime can import a bounded development snapshot containing the current
Luke vault descriptor, destination, chain tip, and discovered vault UTXOs. It
reconstructs Luke's HWW-recovery leaf, rejects mismatched or immature inputs,
reviews the eligible amount and fee, derives the app-isolated HWW identity only
after approval, signs only the HWW leaf, and atomically exports a raw Bitcoin
transaction plus its txid.

The snapshot is explicit test plumbing for host and regtest compatibility. It
does not define a phone transport, chain backend, or production persistence
mechanism for Prime.

## Evidence

- TDD began with a failing unresolved recovery import. Focused engine and Prime
  flow tests cover descriptor/outpoint/script binding, exact next-block CSV
  maturity, malformed and early snapshots, signing, atomic export, and seed
  failure.
- Public CI run `31856267991` passed all 52 release tests, formatting, and
  strict clippy in 48s. Its unchanged Luke/Bitcoin Core regtest job passed in
  16m09s after generating the literal recovery delay with consensus-valid
  mock-time epochs.
- Luke's unchanged `hww recover` broadcast transaction
  `ca158b655333148ffffe8cb093c5f7d3b41fd3f9c2fad33d8bd754f86baf0c7f`
  from one mature input, sending 999,854 sats with a 146-sat fee at 1 sat/vB.
- The adapter rejected a snapshot one block before eligibility and wrote no
  approved package. At tip 65,741 it accepted the mature snapshot, signed one
  HWW input, and produced an artifact with SHA-256
  `a384957e64fab145197ec3c11d49f004eebd2928b16407939d599f3373557d69`.
  Bitcoin Core accepted the raw transaction and returned the adapter's exact
  txid.

## Honest boundary

This is host and Luke/Bitcoin Core regtest evidence. It is not a signed target
build, simulator run, or physical-device execution. The development snapshot
does not prove how a real Prime receives chain state or returns a transaction.
No Bluetooth, QR, USB protocol, mainnet release, Prime-side chain access, or
Prime-side broadcast is added.
