# Phone-key rotation v1 evidence

## Upstream boundary

- Luke repository: `https://github.com/lukechilds/anzen`
- Commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Operation: `hww confirm-rotation`
- Package: version 1, kind `phone-key-rotation`

## Represented boundary

Passport Prime can review Luke's new vault destination, old-to-new cooperative
sweep, configured limits, renewed-policy workload, and recovery-friend count
before requesting app-seed material. Approval derives the app-isolated HWW
identity, binds it to the current descriptor, validates the pending phone key and
new vault, signs the sweep and renewed PolicyPackage, authenticates the current
cloud backup, and atomically exports Luke-compatible approved rotation JSON.

The development bridge reads Luke's exact current rotation proposal,
`VaultConfig`, pending-phone `DeviceFile`, and `CloudRecoveryBackup` shapes. It
does not define a phone transport.

## Evidence

- TDD began with a failing unresolved rotation import, followed by focused
  envelope and owner-summary tests.
- The final workspace has 47 passing release tests, clean formatting, and strict
  release clippy.
- Public CI run `31852635587` passed both jobs: Rust checks in 1m10s and Luke CLI
  regtest round trip in 4m03s. Luke's unchanged phone CLI created the rotation,
  accepted and activated the adapter-approved package, preserved the renewed
  monthly/emergency policy, and completed a post-rotation cooperative sweep.
- A targeted Foundation SDK build compiled exact public head
  `32bf2aadeaf940536143fc5a2eda6b9db927233d` for
  `armv7a-unknown-xous-elf`, generated the manifest, and signed the app package.
  `app.elf` was 7,110,512 bytes with SHA-256
  `21bd97b5864d94c4b8d115936603fad26c5df716477481ec4bfb4a5b57fe4627`;
  `manifest.json` was 804 bytes with SHA-256
  `e00480e86bf8bb05cd3fdaf577433cbff1519864f90d389cfaf1f460e9bb1c27`.

## Honest boundary

This is host, Luke/Bitcoin Core regtest, and signed Prime target-build evidence.
It is not simulator or physical-device execution. The later recovery-friend
slice closes the original no-friend limitation: public regtest now proves that
an enrolled friend keeps the same fingerprint but receives a different
OpenPGP wrapper under the rotated backup key, and unchanged Luke decrypts the
post-rotation package through both friend and HWW paths. No Bluetooth, QR, USB
protocol, mainnet release, Prime-side chain access, or Prime-side broadcast is
added.
