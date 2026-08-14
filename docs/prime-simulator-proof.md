# Prime simulator proof checklist

Tracking issue: [#1 Prime simulator proof](https://github.com/owenkemeys/KeyOS-anzen/issues/1)

This checklist prevents a host preview, a device-target build, and a hosted
Passport Prime simulator run from being conflated.

## Device-target build

- [ ] Record the Foundation SDK version or commit.
- [ ] Run `scripts/prime-sdk.sh doctor` successfully in the SDK environment.
- [ ] Build from the public branch with `scripts/prime-sdk.sh build`.
- [ ] Preserve the signed `app.elf` and `manifest.json` checksums outside Git.
- [ ] Confirm no local signing identity or private path appears in committed files.

## Hosted simulator

- [ ] Build through `scripts/prime-sdk.sh sim`; do not substitute the device
      `app.elf`.
- [ ] Confirm the simulator control channel reports the app launched.
- [ ] Confirm the Anzen screen is visibly rendered, not merely a live process.
- [ ] Run the proof and show 28 transactions, 39 signatures, and 39 verified.
- [ ] Capture the screen with the simulator's own Screenshot control.
- [ ] Commit only the selected screenshot and its public evidence note.

## Publication

- [ ] Label the screenshot "Passport Prime simulator".
- [ ] Keep the Windows preview separately and label it accurately.
- [ ] State that hardware installation and physical-device execution remain
      unproven unless they have independently occurred.
- [ ] Run the repository verification scripts.
- [ ] Attach the evidence to a pull request linked to issue #1.
