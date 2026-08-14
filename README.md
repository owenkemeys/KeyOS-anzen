# KeyOS Anzen

## Anzen Prime Proof

A minimal Passport Prime app that runs the largest hardware-wallet benchmark
currently published by [Luke Childs' Anzen](https://github.com/lukechilds/anzen):
28 Taproot transactions and 39 BIP340 Schnorr signatures in one approval.

The point is deliberately narrow: Anzen's current Ledger and Trezor work is a
benchmark, and the same benchmark can run as a native KeyOS app on Passport
Prime.

![Anzen Prime Proof running in the Passport Prime simulator](screenshots/prime-simulator-device-frame.png)

_Owner-captured Passport Prime simulator result. The app completed Luke Childs'
28-transaction workload, produced 39 Schnorr signatures, and verified all 39._

## What the green result means

This is a completed cryptographic benchmark run, not a live vault. The benchmark
uses deterministic fake outpoints and amounts, while constructing the same
version-2 Bitcoin transactions and BIP341 script-path signature messages as
Anzen's annual policy. The Prime app derived a demo key from the simulator's
app-isolated seed, signed all 39 benchmark messages with BIP340 Schnorr, verified
all 39 signatures, and committed the run to the displayed transcript hash.

The on-screen phrase **Policy signed on Passport Prime** is compact demo wording.
No real Anzen policy package, wallet UTXO, transaction, or funds were involved.
The [raw 480 x 800 simulator capture](screenshots/prime-simulator.png) is also
retained as evidence.

The earlier [Windows preview](screenshots/windows-preview.png) remains available
as a separately labelled host-rendering reference.

## PolicyPackage v4 host interoperability

The repository also contains a host-side implementation of Luke's current
version-4 policy ceremony. It imports the exact JSON and PSBT package produced
by Anzen, presents the high-level policy, independently validates the static
descriptor and recovery paths, every amount and fixed fee, all allowance and
emergency transaction relationships, and every phone signature, then derives
the Anzen HWW identity from KeyOS-style app-isolated seed material and adds the
HWW signatures.

The checked-in fixture is a deterministic, disposable regtest proposal created
by Luke's code at commit `01794bb14d34d01413e3b539c7e5496ecbce0c87`.
The public **Luke CLI regtest round trip** CI job goes further: Luke's unchanged
CLI creates and phone-signs a freshly funded policy, this repository's adapter
approves it, and Luke's unchanged CLI validates, activates, and broadcasts the
rollover on a disposable Bitcoin Core regtest chain.

The dated [PolicyPackage v4 evidence record](docs/evidence/policy-package-v4-roundtrip-2026-08-14.md)
keeps that host proof separate from the earlier Prime simulator benchmark.

## What the demo proves

- Uses Anzen's actual allocation-free `anzen-cold-signer` benchmark, pinned to
  upstream commit `01794bb14d34d01413e3b539c7e5496ecbce0c87`.
- Derives a demo signing identity from KeyOS's app-isolated seed.
- Reconstructs Anzen's 28-transaction annual policy workload.
- Produces 39 real BIP340 Schnorr signatures for the 12-input case.
- Verifies every signature and commits the run to a transcript hash.
- Exposes only the public key and transcript; the app seed and private key stay
  inside the app process.
- Builds as a signed KeyOS device bundle and runs as a hosted app in the
  Passport Prime simulator.

## What it does not claim

This is a compatibility proof, not a production Anzen wallet. Policy import and
export are a development file bridge, not Anzen phone connectivity. The real
policy-package proof runs on the host against disposable regtest funds; it is
not evidence that this larger policy engine builds or runs on Passport Prime.
Production transport, persistence hardening, mobile integration, mainnet
safety, physical-device execution, and real-funds use remain unimplemented and
unproved.

## Repository layout

- `crates/anzen-policy-engine` - host-testable PolicyPackage v4 validation and signing.
- `tools/anzen-prime-adapter` - development-only file adapter used for interoperability proof.
- `fixtures/policy-package-v4` - deterministic regtest proposal and provenance.
- `vendor/anzen-cold-signer` — verbatim MIT-licensed upstream snapshot.
- `crates/anzen-prime-core` — host-testable Prime signing integration.
- `app` — thin native KeyOS/Slint shell.

## Test the signing core

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

With Docker available, run the pinned Luke CLI/Bitcoin Core round trip:

```sh
./scripts/verify-luke-roundtrip.sh
```

The script uses only disposable regtest state and deletes its isolated Docker
project, named volume, and temporary files on exit.

## Run the Windows preview

The desktop preview uses the same signing core and the same 480×800 Slint UI,
but substitutes a deterministic test app seed because KeyOS's app-isolated
seed API exists only on Passport Prime:

```powershell
cargo run -p anzen-prime-preview --release
```

It is intentionally labeled **Windows preview** on screen. The genuine Prime
simulator evidence is the primary screenshot at the top of this page.

## Build for Passport Prime

Install the current [Foundation Passport Prime SDK](https://foundation.xyz/developers),
create a local signing identity named `KeyOS Anzen Developer`, then run from
the repository root:

```sh
scripts/prime-sdk.sh doctor
scripts/prime-sdk.sh build
scripts/prime-sdk.sh sim
```

The Prime SDK is currently a public beta and Foundation's supported host path
is Linux or macOS. This repository was developed from Windows using an Ubuntu
VM for the SDK build and simulator.

The genuine simulator milestone is recorded in
[`docs/evidence/prime-simulator-2026-08-14.md`](docs/evidence/prime-simulator-2026-08-14.md)
and tracked publicly in
[#1 Prime simulator proof](https://github.com/owenkemeys/KeyOS-anzen/issues/1).

## Contributing

Material changes use an issue, a focused branch, automated checks, and a pull
request. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the workflow and proof
labelling rules.

## Attribution

Anzen and `anzen-cold-signer` are by Luke Childs and licensed under MIT. The
vendored snapshot retains Luke's license and has an explicit provenance note.
The Prime integration is an independent proof and is not an official Anzen or
Foundation product.
