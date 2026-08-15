# Luke HWW parity on Passport Prime

This matrix fixes the compatibility boundary to Luke Childs' Anzen `main` at
commit `01794bb14d34d01413e3b539c7e5496ecbce0c87`. It covers the seven commands
under Luke's current `HwwCommand`; it does not propose a phone transport or
turn this repository into a production wallet.

## Current matrix

| Luke HWW operation | Prime representation | Strongest evidence | Status | Follow-up |
| --- | --- | --- | --- | --- |
| `hww init` | KeyOS supplies an app-isolated seed; the app derives and checks the descriptor-bound HWW identity only after explicit approval. | Host identity tests, owner-accepted hosted simulator approval, and a signed Prime target build. | Represented for the current approval flow; physical execution is unproved. | None |
| `hww confirm-policy` | Import and review PolicyPackage v4, independently validate all 28 PSBTs and phone signatures, add HWW signatures, and atomically export Luke-compatible JSON. | 35 host release tests; public Luke CLI/Bitcoin Core regtest round trip; owner-accepted hosted simulator flow; signed target build. | Represented. | None |
| `hww confirm-sweep` | Review a phone-signed immediate cooperative sweep, validate its descriptor, inputs, destination, amount, fee, and phone signatures, then add the HWW signatures. | Host engine/Prime-flow tests, public Luke CLI/Bitcoin Core regtest proposal-to-broadcast round trip, and signed Prime target build. | Represented; physical execution is unproved. | None |
| `hww confirm-rotation` | Review one phone-key rotation, its old-to-new vault sweep, preserved recovery-friend set, and optional renewed policy; approve the bound package in one ceremony. | Host tests, public unchanged-Luke proposal-to-activation and friend-decryption round trip, fresh-wrapper assertion, post-rotation sweep, and signed Prime target build. | Represented for Luke's current friend-preserving path; physical execution is unproved. | None |
| `hww recover` | Review a destination and mature HWW-only recovery sweep using the 65,535-block path; sign only eligible vault UTXOs. | 52 host release tests; public unchanged-Luke and Bitcoin Core regtest recovery; early-snapshot rejection; Core acceptance of the Prime-signed raw transaction. | Represented on host/regtest; target, simulator, physical execution, and production chain transport are unproved. | None |
| `hww decrypt-phone-backup` | Authenticate and decrypt Luke's HWW-wrapped cloud envelope, validate the phone key and descriptor binding, and export the portable recovery package. | 55 host release tests; unchanged-Luke semantic recovery-package comparison; signed Prime target build. | Represented; simulator and physical execution are unproved. | None |
| `hww add-recovery-friend` | Review the trust expansion, validate the OpenPGP public key, wrap the existing backup key for that fingerprint, and preserve the authenticated friend manifest. | 61 host release tests; unchanged-Luke friend decryption before and after fresh-key rotation; signed Prime target build with KeyOS hardware RNG; [physical test card](prime-recovery-friend-physical-test.md). | Represented; physical execution is unproved. | None |

## Evidence labels

- **Host tests** prove parsing, validation, signing, state transitions, and safe
  failure behavior in ordinary Rust builds.
- **Luke CLI/regtest** proves Luke's unchanged reference wallet accepts an
  output and can exercise it against disposable Bitcoin Core state.
- **Hosted simulator** proves the labelled KeyOS UI/platform path that was
  actually run. Its fixture seed and app-private files are not physical Prime
  evidence.
- **Signed target build** proves compilation and packaging for
  `armv7a-unknown-xous-elf`; it does not prove execution.
- **Physical Prime** requires a completed owner-run sideload and playtest. No
  operation in this matrix currently carries that label.

## Bounded follow-up slices

### Cooperative sweep review and signing

Implemented for Luke's current `confirm-sweep` package and validation boundary.
The phone remains responsible for proposal creation and final broadcast.

### Phone-key rotation and renewed policy

Implemented for Luke's current combined rotation package, including the
cooperative sweep, descriptor change, authenticated replacement cloud envelope,
optional renewed PolicyPackage, and fresh symmetric-key OpenPGP re-wrapping for
every authenticated recovery friend. Do not generalize the operation into
account or key management.

### Delayed HWW recovery sweep

Implemented for Luke's current 65,535-block HWW recovery path with host and
regtest proof, including fail-closed early maturity and Bitcoin Core acceptance.
The development snapshot remains test plumbing; Prime chain-state transport is
not represented.

### Descriptor-bound phone-backup decryption

Implemented for Luke's current cloud envelope and version-2 recovery package.
Recovery material remains bounded, absent from review/log output, and exported
only after explicit approval. The file bridge is not a production transport or
cloud service.

### Recovery-friend wrapping and management

Implemented for Luke's current add-friend operation: one validated OpenPGP
recipient wrapper, sorted duplicate-free friend set, and authenticated friend
manifest. Rotation renews the symmetric key and every stored wrapper. Luke's
current HWW surface has no friend removal or threshold command, so neither is
represented.

## Stopping line

The parity program stops at these current upstream commands. It excludes an
invented phone protocol, Bluetooth or QR design, production persistence,
mainnet enablement, app-catalog release, generalized wallet productization, and
claims based on a signed build or simulator that require a physical Prime run.
