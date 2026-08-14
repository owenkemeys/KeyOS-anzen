# PolicyPackage v4 Prime target evidence — 2026-08-14

This record covers a signed Passport Prime device-target build of the real
PolicyPackage v4 import/review/approval app. It is target compatibility evidence,
not runtime evidence.

## Source and protocol

- Repository: `owenkemeys/KeyOS-anzen`
- Branch: `codex/prime-policy-v4-simulator`
- Source commit: `4507a43ce0f2d0b04e61295b865136c2eee28b39`
- Tracked issue: `#7`
- Foundation SDK package: `0.4.0-x86_64-unknown-linux-gnu`
- Target: `armv7a-unknown-xous-elf`
- Luke Childs' Anzen commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`

## Compatibility finding

The PolicyPackage engine depends on rust-bitcoin's bundled `secp256k1-sys` C
library. Foundation SDK v0.4.0 includes the correct `arm-none-eabi-gcc` and
`arm-none-eabi-ar` tools, but `cc-rs` does not select them automatically for the
custom KeyOS target. `scripts/prime-sdk.sh build` now supplies only the standard
target-specific `CC_armv7a_unknown_xous_elf` and
`AR_armv7a_unknown_xous_elf` mappings.

With those mappings, the complete app—including Bitcoin, Miniscript,
secp256k1, bounded import flow, KeyOS filesystem adapter, KeyOS app-seed adapter,
and Slint review UI—compiled successfully. Foundation's build then stripped the
binary, generated `manifest.json`, and signed the app bundle.

## Signed output

- `app.elf` SHA-256:
  `86b0aad0c542b8744ff64afb691171fc58e6b0a306d374c9caa6cb9c11e65a12`
- `manifest.json` SHA-256:
  `e00480e86bf8bb05cd3fdaf577433cbff1519864f90d389cfaf1f460e9bb1c27`
- Exact source archive SHA-256:
  `29953eaf75d99ad12f601cf47a7095b7a9854971935b6be5bb157731b8764d1e`

The first clean rebuild exhausted the 13 GiB then available while compiling and
stopped before packaging. The failed partial project target was removed. Nix
then identified 281 unreachable store paths and garbage-collected 62.9 GiB of
reproducible cache/toolchain material. The required toolchain was restored from
the official Nix cache and the exact-source rebuild completed with 45 GiB free.
The resulting isolated project target occupied 6.5 GiB.

A later incremental build from source commit `4507a43` added monotonic timing
around independent validation, HWW signing, and the complete approve-to-write
operation. That exact source again compiled, stripped, manifested, and signed
with 45 GiB free. This proves target compatibility of the timing code, not the
speed of physical hardware.

## Host safety checks before target build

- Seven focused Prime-flow tests passed, including import bounding,
  review-before-seed ordering, identity mismatch, and write/commit failure.
- The complete release workspace test suite passed.
- Workspace Clippy passed with warnings denied.
- The existing host PolicyPackage and Luke CLI/regtest proof remained unchanged.

## Proof boundary

This proves the complete source is compatible with the Prime device target and
can be packaged as a signed KeyOS app. It does not prove the app launches, reads
the device USB folder, or executes on physical Passport Prime hardware. The
separate hosted-simulator record covers visible fixture import, review, approval,
signing, and app-private output without extending that evidence to USB, phone
transport, a hardware seed, or physical hardware.
