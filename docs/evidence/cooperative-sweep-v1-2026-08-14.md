# Cooperative sweep v1 evidence

## Upstream boundary

- Luke repository: `https://github.com/lukechilds/anzen`
- Commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Operation: `hww confirm-sweep`
- Package: version 1, kind `cooperative-sweep`

## Implemented boundary

Passport Prime can review Luke's destination, sent amount, fee, vault-input
count, and network without requesting app-seed material. Explicit approval then
derives the app-isolated HWW identity, independently revalidates the descriptor,
every witness UTXO, the exact one-output transaction, the one-sat/vB fee, and
every phone signature, adds the HWW signatures, and atomically exports Luke's
unchanged JSON shape.

The development file bridge uses `anzen-sweep-v1.json` and writes
`anzen-sweep-v1-approved.json`. It is not a phone transport. Luke's phone CLI
still creates the proposal and broadcasts the approved transaction.

## Evidence

- Focused engine tests cover bounded parsing, owner-visible summary, descriptor,
  destination, amount, fee, input, approval-state, and signature tampering.
- Prime-flow tests prove review-before-seed ordering, one seed request, HWW
  signing, atomic export, and preservation of an older approved file on identity
  mismatch.
- Adapter tests prove the host bridge emits an HWW-approved Luke-compatible
  package without printing the deterministic development seed.
- Public CI uses Luke's unchanged CLI and disposable Bitcoin Core regtest to
  create the sweep, approve it with this engine, and accept and broadcast it.
- A targeted Foundation SDK build compiled the exact public PR head
  `263882a63e0ca9d9b3c1267a05ff418d489738e1` for
  `armv7a-unknown-xous-elf`, generated the manifest, and signed the app package.
  The resulting `app.elf` was 6,923,568 bytes with SHA-256
  `358681ac23bd1244cc2214c83643da30b7890dc5851a11b3fa690036dc0e215d`;
  `manifest.json` was 804 bytes with SHA-256
  `e00480e86bf8bb05cd3fdaf577433cbff1519864f90d389cfaf1f460e9bb1c27`.

## Honest boundary

This is host, regtest interoperability, and signed Prime target-build evidence.
It does not claim a hosted-simulator run or physical-device execution. It adds no
Bluetooth, QR, USB protocol, mainnet release, or Prime-side broadcast.
