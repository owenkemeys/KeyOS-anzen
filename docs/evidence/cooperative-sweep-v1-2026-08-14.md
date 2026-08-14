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

## Honest boundary

This is host and regtest interoperability evidence. The Slint/KeyOS source path
is wired for the same review and approval flow, but this slice does not claim a
new hosted-simulator run, signed Prime target build, or physical-device execution.
It adds no Bluetooth, QR, USB protocol, mainnet release, or Prime-side broadcast.
