# Phone-backup decryption v1 evidence

## Upstream boundary

- Luke repository: `https://github.com/lukechilds/anzen`
- Commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Operation: `hww decrypt-phone-backup`
- Output: version 2, kind `phone-recovery`

## Represented boundary

Passport Prime can review the backup's network, descriptor-bound vault address,
and authenticated recovery-friend count before requesting app-seed material.
Approval verifies the configured HWW identity, authenticates Luke's wrapped
symmetric key and friend manifest, decrypts the recovery payload, re-derives
the phone key from its mnemonic and key index, binds every vault field, and
atomically exports Luke's portable recovery package. Recovery words are not
shown in the review or logs.

The labeled Prime file bridge reads the current backup and VaultConfig shapes.
It does not define a phone transport, cloud service, or persistence system.

## Evidence

- TDD began with missing `PhoneBackupSummary` and `review_for_recovery` symbols,
  then added focused pre-seed review, valid decryption, wrong-HWW,
  tampered-manifest, and no-write-on-failure coverage.
- Public CI run `31858548099` passed 55 release tests, formatting, and strict
  clippy in 1m25s. Its unchanged-Luke job passed in 25m59s. After Luke's real
  phone-key rotation, Luke and the adapter independently decrypted the same
  current backup; normalized version-2 recovery JSON matched exactly. Only the
  network, vault address, and zero authenticated friends were logged.
- A targeted Foundation SDK build compiled exact public head
  `07080f013814538b52f3c89c19cb877bee9e3e62` for
  `armv7a-unknown-xous-elf`, generated the manifest, and signed the app package.
  `app.elf` was 7,953,608 bytes with SHA-256
  `c5626bc1c9f36baf064020db03dd40a5d2152ea9a14594b7c9131923ef422caa`;
  `manifest.json` was 804 bytes with SHA-256
  `e00480e86bf8bb05cd3fdaf577433cbff1519864f90d389cfaf1f460e9bb1c27`.

## Honest boundary

This is host, unchanged-Luke CI, and signed Prime target-build evidence. It is
not simulator or physical-device execution. No recovery secret is bundled in
the simulator. No Bluetooth, QR, USB protocol beyond the labeled development
file bridge, cloud product, mainnet release, or physical recovery claim is
added.
