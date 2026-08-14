# Prime simulator proof checklist

Tracking issue: [#1 Prime simulator proof](https://github.com/owenkemeys/KeyOS-anzen/issues/1)

This checklist prevents a host preview, a device-target build, and a hosted
Passport Prime simulator run from being conflated.

## Device-target build

- [x] Record the Foundation SDK version or commit.
- [x] Run `scripts/prime-sdk.sh doctor` successfully in the SDK environment.
- [x] Build from the public branch with `scripts/prime-sdk.sh build`.
- [x] Preserve the signed `app.elf` and `manifest.json` checksums outside Git.
- [x] Confirm no private signing material, credential, or owner-specific path
      appears in committed files.

## Hosted simulator

- [x] Build through `scripts/prime-sdk.sh sim`; do not substitute the device
      `app.elf`.
- [x] Confirm the simulator control channel reports the app launched.
- [x] Confirm the Anzen screen is visibly rendered, not merely a live process.
- [x] Run the proof and show 28 transactions, 39 signatures, and 39 verified.
- [x] Capture the screen with the simulator's own Screenshot control.
- [x] Commit only the selected screenshot and its public evidence note.

## Publication

- [x] Label the screenshot "Passport Prime simulator".
- [x] Keep the Windows preview separately and label it accurately.
- [x] State that hardware installation and physical-device execution remain
      unproven unless they have independently occurred.
- [x] Run the repository verification scripts.
- [x] Attach the evidence to a pull request linked to issue #1.

The dated evidence and checksums are in
[`evidence/prime-simulator-2026-08-14.md`](evidence/prime-simulator-2026-08-14.md).
