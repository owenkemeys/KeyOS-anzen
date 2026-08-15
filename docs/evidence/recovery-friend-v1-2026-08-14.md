# Recovery-friend v1 evidence

## Upstream boundary

- Luke repository: `https://github.com/lukechilds/anzen`
- Commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Operation: `hww add-recovery-friend`
- Recovery model: one OpenPGP wrapper per friend, 1-of-N rather than threshold

## Represented boundary

Passport Prime can review the exact OpenPGP fingerprint, descriptor-bound vault
address, network, and resulting friend count before requesting app-seed
material. Approval validates the signed public key and encryption subkey,
checks the configured HWW identity, authenticates the existing recovery payload
and friend manifest, rejects duplicates, wraps the current symmetric backup key,
sorts the friend set by fingerprint, renews the authenticated manifest, and
atomically exports Luke-compatible backup JSON.

Phone rotation now derives a fresh symmetric backup key and creates a new
OpenPGP wrapper for every authenticated stored public key. The fingerprint set
is preserved; the old symmetric key and old wrappers are not reused.

## Evidence

- TDD began with unresolved recovery-friend review symbols. Focused tests then
  covered a valid enrollment, invalid OpenPGP input, wrong HWW identity,
  tampered manifest, duplicate enrollment, no write after failed approval, and
  fresh-key wrapper replacement during rotation.
- Public Actions run `31862775404` passed 61 release tests, formatting, and
  strict clippy in 1m21s. Its unchanged-Luke job passed in 19m40s at exact
  upstream `01794bb14d34d01413e3b539c7e5496ecbce0c87`.
- Luke generated the OpenPGP friend key. Prime enrolled one friend. Unchanged
  Luke decrypted the resulting backup before rotation. Prime then approved a
  real phone-key rotation preserving one fingerprint, asserted that the
  encrypted friend wrapper changed, and unchanged Luke decrypted the rotated
  backup through both friend and HWW paths. The normalized friend, Luke-HWW,
  and Prime-HWW version-2 recovery packages matched without logging recovery
  words.
- The run recorded friend-enrolled backup SHA-256
  `e581adb69b8b6be4e698405b4f7c79e47927d564841a2ba5bd1bd7bcf2d715a5`
  and rotated backup SHA-256
  `1f989ed6c5854bd07b43969056d07f03a8cacde7c5b0578e22efc063c79b2a7c`.
- A targeted Foundation SDK build compiled exact public app source
  `4daaaed3e943fc36ce4ee922139ea46b97a1af65` for
  `armv7a-unknown-xous-elf`, stripped it, generated the manifest, and signed the
  package. The first target attempt correctly rejected host `getrandom`; the
  final app uses KeyOS `Security::get_random()` and explicitly declares only
  `GetAppSeed` and `GetRandom` security capabilities.
- `app.elf` was 11,424,512 bytes with SHA-256
  `836763a57d5c4373e89acdb2c06b3ca247b0e4625eb462655e82dd594404bc41`;
  `manifest.json` was 823 bytes with SHA-256
  `eb9b15ddef28d7ac8c8623322f6b821f10b86ecf277b68d0f3778a8336b209a8`.

## Honest boundary

This is host, unchanged-Luke/regtest, and signed Prime target-build evidence.
It is not simulator or physical-device execution. The development USB file
bridge is not a phone transport or cloud service. Luke's current HWW surface
has no friend-removal or threshold command, so neither is represented. No
Bluetooth, QR, mainnet release, generalized product, physical recovery, or
real-funds claim is added. Physical acceptance remains governed by
`docs/prime-recovery-friend-physical-test.md`.
