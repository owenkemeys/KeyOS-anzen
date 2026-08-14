# PolicyPackage v4 Prime target evidence — 2026-08-14

This record covers a signed Passport Prime device-target build of the real
PolicyPackage v4 import/review/approval app. It is target compatibility evidence,
not runtime evidence.

## Source and protocol

- Repository: `owenkemeys/KeyOS-anzen`
- Branch: `codex/prime-policy-v4-simulator`
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
  `2e6609fef02193fa0fe930452d0f01713e7eee360e4b4003ece21e91185b833d`
- `manifest.json` SHA-256:
  `e00480e86bf8bb05cd3fdaf577433cbff1519864f90d389cfaf1f460e9bb1c27`

The VM had 26 GiB free after the build. The existing isolated project target was
reused rather than creating a second unbounded cache, and the VM was returned to
saved state immediately after evidence capture.

## Host safety checks before target build

- Seven focused Prime-flow tests passed, including import bounding,
  review-before-seed ordering, identity mismatch, and write/commit failure.
- The complete release workspace test suite passed.
- Workspace Clippy passed with warnings denied.
- The existing host PolicyPackage and Luke CLI/regtest proof remained unchanged.

## Proof boundary

This proves the complete source is compatible with the Prime device target and
can be packaged as a signed KeyOS app. It does not prove the app launches, reads
the simulator USB folder, displays correctly, writes approved JSON at runtime,
or executes on physical Passport Prime hardware. Those claims require separate
simulator or hardware evidence.
