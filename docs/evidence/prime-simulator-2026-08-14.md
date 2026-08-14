# Passport Prime simulator evidence — 2026-08-14

This record distinguishes the signed device-target build from the hosted
Passport Prime simulator run. Both were produced from source commit `9076eca`.

## Source and SDK

- Repository: `owenkemeys/KeyOS-anzen`
- Source commit: `9076eca` (`fix: use a valid 16-byte KeyOS app ID`)
- Foundation SDK package: `0.4.0-x86_64-unknown-linux-gnu`
- Foundation CLI: `foundation 0.1.0`
- Upstream Anzen workload commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Source-transfer archive SHA-256:
  `c4e981fa5a95d8d2e70d9586316a50c17507148b93d0eb5b50da63426da10102`

## Signed KeyOS device build

`scripts/prime-sdk.sh build` completed for
`armv7a-unknown-xous-elf` and produced a signed app bundle. The bundle is kept
outside Git because it is reproducible build output.

- `app.elf` size: `4,129,172` bytes
- `app.elf` SHA-256:
  `80f7251476d6ee8a28311000ca7a0b6e19fbcd00f1d75d1d73344953643d6e89`
- `manifest.json` size: `617` bytes
- `manifest.json` SHA-256:
  `86b2d39f2077019fa21efc57b369de1157cbbdda49ca48a9e07af7d9e36ab07d`
- App ID: `0x416e7a656e5072696d6550726f6f6631`
- Declared security capability: `os/security:GetAppSeed`

## Hosted simulator run

`scripts/prime-sdk.sh sim` performed a separate hosted build. The simulator
control path reported:

```text
PID 29 is launching app 0x416e7a656e5072696d6550726f6f6631
launched app anzen-prime-proof with pid 19
Switching to initial window, PID=19
```

The visible app action then completed with:

```text
Anzen benchmark complete: 28 transactions · 39 Schnorr signatures /
All 39 signatures verified / Transcript 39f68814ada6617a...
```

The selected image was produced with the simulator control panel's own
Screenshot button:

- [`screenshots/prime-simulator.png`](../../screenshots/prime-simulator.png)
- Size: `86,874` bytes
- SHA-256:
  `a3d4ce6e3472e156308f1b972b87996bb5fa204ddfb9c76d6a8538ddfe8aa4dd`

## Device-framed capture

The repository's primary publishing image is the owner's device-framed capture
of the same completed live simulator run:

- [`screenshots/prime-simulator-device-frame.png`](../../screenshots/prime-simulator-device-frame.png)
- Dimensions: `573 x 1,071` pixels
- Size: `192,162` bytes
- SHA-256:
  `41db51bf702985a247a42264df27e969ad1ad3fd41d7461e2ef658ade3343bda`

The raw 480 x 800 simulator screenshot above remains the primary evidence
surface; the device-framed capture is the clearer image for public sharing.

## Repository verification

`scripts/verify.ps1` completed after the evidence files were added. It checked
formatting, ran all 10 workspace tests in release mode, ran Clippy across all
workspace targets with warnings denied, and checked the Git diff for whitespace
errors.

## Proof boundary

This proves that the KeyOS app builds for the Prime device target and that its
hosted simulator build visibly runs Luke Childs' current Anzen benchmark through
the Prime-shaped approval flow. The benchmark uses fake outpoints and amounts,
but constructs the same version-2 Bitcoin transactions and BIP341 script-path
signature messages as a real annual policy; all 39 resulting BIP340 signatures
were cryptographically verified. It does not prove installation or execution on
physical Passport Prime hardware, nor does it implement Anzen's phone protocol,
policy-package transport, persistence, live-input signing, broadcasting, or
funds movement.
