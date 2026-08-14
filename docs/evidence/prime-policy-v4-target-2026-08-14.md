# PolicyPackage v4 Prime target evidence — 2026-08-14

This record covers a signed Passport Prime device-target build of the real
PolicyPackage v4 import/review/approval app. It is target compatibility evidence,
not runtime evidence.

## Source and protocol

- Repository: `owenkemeys/KeyOS-anzen`
- Branch: `codex/prime-policy-v4-simulator`
- Source commit: `2e753fdc8426980cc2007cc138dff921ddcb0e21`
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
  `872745740ddab432b8e8cf3f9c537616e2bd95d6e412bcee1441dda4beeb53f8`
- `manifest.json` SHA-256:
  `e00480e86bf8bb05cd3fdaf577433cbff1519864f90d389cfaf1f460e9bb1c27`
- Exact source archive SHA-256:
  `59142224b5c01a65e2b6ccbd048c0a997cc5d910f3c131dc460cffba566ff889`

The first clean rebuild exhausted the 13 GiB then available while compiling and
stopped before packaging. The failed partial project target was removed. Nix
then identified 281 unreachable store paths and garbage-collected 62.9 GiB of
reproducible cache/toolchain material. The required toolchain was restored from
the official Nix cache and the exact-source rebuild completed with 45 GiB free.
The resulting isolated project target occupied 6.5 GiB.

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
