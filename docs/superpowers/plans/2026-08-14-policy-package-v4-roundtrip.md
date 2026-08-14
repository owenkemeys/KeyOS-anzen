# PolicyPackage v4 Round Trip Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Approve a real Anzen `PolicyPackage` v4 with a Passport Prime app-seed-derived key and return JSON that Luke Childs' unchanged CLI accepts on regtest.

**Architecture:** Add a KeyOS-free Rust policy engine that owns bounded JSON import, exact upstream protocol types, independent descriptor/PSBT graph validation, phone-signature verification, deterministic app-seed key derivation, and HWW signing. A thin host adapter reads and writes files for interoperability tests; the Prime app shell and any transport remain out of this host-first issue.

**Tech Stack:** Rust 2021, `serde`/`serde_json`, `bitcoin` 0.32, `miniscript` 12, BIP39/BIP32 key derivation, existing PowerShell/Bash verification scripts, Luke Childs' Anzen CLI pinned to commit `01794bb14d34d01413e3b539c7e5496ecbce0c87`.

## Global Constraints

- Track the work in public issue `#5` on `owenkemeys/KeyOS-anzen` and branch `codex/policy-package-v4-roundtrip`.
- Preserve Luke Childs' MIT attribution and exact upstream commit provenance.
- Reject unsupported package versions, unsafe or mismatched PSBT sets, oversized imports, descriptor/key mismatches, invalid phone signatures, fee changes, and any transaction-graph mutation before producing HWW signatures.
- Derive the Anzen HWW identity deterministically from KeyOS app-isolated seed material; never persist or log the seed or derived private key.
- Treat fixture/file import as development transport only. Do not claim phone connectivity, simulator proof, target proof, hardware proof, mainnet readiness, persistence hardening, or funds movement.
- Keep the Foundation SDK VM powered off until host parity is proved and a targeted build question is stated.
- Do not create unbounded build output; inspect disk space before any later target build and remove only explicitly identified reproducible project output.

---

### Task 1: Exact protocol fixture and parser boundary

**Files:**
- Create: `crates/anzen-policy-engine/Cargo.toml`
- Create: `crates/anzen-policy-engine/src/lib.rs`
- Create: `crates/anzen-policy-engine/src/package.rs`
- Create: `crates/anzen-policy-engine/tests/policy_package_v4.rs`
- Create: `fixtures/policy-package-v4/regtest-proposal.json`
- Create: `fixtures/policy-package-v4/README.md`
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: `PolicyPackage::parse_bounded(bytes: &[u8]) -> Result<PolicyPackage, PolicyError>`.
- Produces: exact serde field names for `PolicyPackage`, `BatchManifest`, `BatchTransaction`, `AllowanceStep`, and `EmergencyAccessPolicy`.
- Enforces: `MAX_PACKAGE_BYTES`, package version `4`, kind `vault-policy`, manifest version `4`, and an exact manifest-to-PSBT key set.

- [ ] **Step 1: Add a failing real-fixture parser test**

```rust
#[test]
fn parses_luke_cli_policy_package_v4_and_preserves_the_exact_psbt_set() {
    let bytes = include_bytes!("../../../fixtures/policy-package-v4/regtest-proposal.json");
    let package = PolicyPackage::parse_bounded(bytes).unwrap();
    assert_eq!(package.version, 4);
    assert_eq!(package.kind, "vault-policy");
    assert_eq!(package.manifest.network, "regtest");
    assert_eq!(package.manifest.allowance_count, 12);
    assert_eq!(package.psbts.len(), 28);
}
```

- [ ] **Step 2: Run `cargo test -p anzen-policy-engine --test policy_package_v4` and verify the test fails because the engine/API does not exist.**
- [ ] **Step 3: Add the crate, exact protocol structs, bounded parse function, supported-version checks, safe relative PSBT-name checks, and exact PSBT-set validation.**
- [ ] **Step 4: Add malformed JSON, oversized input, unsupported version/kind, unsafe path, missing PSBT, and extra PSBT tests one at a time, observing each expected failure before the minimal implementation.**
- [ ] **Step 5: Run the focused parser test suite and commit the passing parser/fixture slice.**

### Task 2: Human-reviewable policy summary

**Files:**
- Create: `crates/anzen-policy-engine/src/summary.rs`
- Modify: `crates/anzen-policy-engine/src/lib.rs`
- Modify: `crates/anzen-policy-engine/tests/policy_package_v4.rs`

**Interfaces:**
- Produces: `PolicyPackage::summary() -> PolicySummary`.
- `PolicySummary` exposes network, vault address, total input, monthly limit, emergency amount/delay, fee rate, allowance count/delay, rollover txid/fee, rollover output values, and the descriptor's phone/HWW recovery delays without private material.

- [ ] **Step 1: Add a failing test with hand-checked literal summary values from the checked-in Luke fixture.**
- [ ] **Step 2: Run the focused test and verify it fails because `PolicySummary` is missing.**
- [ ] **Step 3: Implement the immutable redacted summary with no PSBT bodies, signatures, seed, or private key fields.**
- [ ] **Step 4: Add failing tests for inconsistent allowance/emergency metadata and implement only the summary-side structural rejection needed for safe review.**
- [ ] **Step 5: Run the focused suite and commit the passing summary slice.**

### Task 3: Independent descriptor and transaction-graph validation

**Files:**
- Create: `crates/anzen-policy-engine/src/policy.rs`
- Create: `crates/anzen-policy-engine/src/validation.rs`
- Modify: `crates/anzen-policy-engine/src/lib.rs`
- Modify: `crates/anzen-policy-engine/tests/policy_package_v4.rs`

**Interfaces:**
- Produces: `PolicyPackage::validate(&self, expected_hww: XOnlyPublicKey) -> Result<ValidatedPolicy, PolicyError>`.
- `ValidatedPolicy` is the only input accepted by signing.
- Validates the descriptor/address/network binding; expected HWW key; rollover inputs/outputs/fees; twelve authorization/revocation relationships; emergency trigger/withdrawal/cancellation relationships; fixed one sat/vB fees; PSBT `witness_utxo`, transaction IDs, sequence values, output scripts/amounts; and all phone Taproot script-path signatures.

- [ ] **Step 1: Add a failing valid-fixture test that expects one `ValidatedPolicy` containing 28 decoded PSBTs.**
- [ ] **Step 2: Run it and confirm the failure is the missing validation API.**
- [ ] **Step 3: Port the smallest policy/transaction validation path from upstream `src/core/{policy,ceremony,transactions}.rs`, adapted to in-memory package PSBTs and carrying MIT provenance comments.**
- [ ] **Step 4: Add one tamper test at a time for manifest totals, descriptor/HWW key, rollover txid, output amount/script, fee, allowance linkage/sequence/hot address, emergency linkage/sequence/hot address, missing witness UTXO, and invalid phone signature; observe RED before each minimal guard.**
- [ ] **Step 5: Run all engine tests, format, Clippy with warnings denied, and commit the independent-validation slice.**

### Task 4: App-seed identity and approved-package signing

**Files:**
- Create: `crates/anzen-policy-engine/src/keys.rs`
- Create: `crates/anzen-policy-engine/src/signing.rs`
- Modify: `crates/anzen-policy-engine/src/lib.rs`
- Modify: `crates/anzen-policy-engine/tests/policy_package_v4.rs`

**Interfaces:**
- Produces: `AnzenIdentity::from_app_seed(seed: &[u8; 32], network: Network) -> Result<AnzenIdentity, PolicyError>` using domain-separated BIP39/BIP32 derivation compatible with Luke's vault key path `m/86'/coin_type'/100'/0/0`.
- Produces: `ValidatedPolicy::approve(identity: &AnzenIdentity) -> Result<ApprovedPolicyPackage, PolicyError>`.
- Produces: `ApprovedPolicyPackage::to_json() -> Result<Vec<u8>, PolicyError>`.

- [ ] **Step 1: Add a failing literal-vector test proving a fixed app seed derives the fixture's HWW x-only public key without exposing the key.**
- [ ] **Step 2: Implement deterministic app-seed-to-mnemonic/BIP32 derivation, zeroize temporary seed material where supported, and verify the vector passes.**
- [ ] **Step 3: Add a failing signing test that requires all 28 PSBTs to receive valid HWW cooperative-path signatures and `manifest.hww_approved` to become true.**
- [ ] **Step 4: Implement signing only on `ValidatedPolicy`; refuse identity mismatch and already-approved/unapproved-phone states.**
- [ ] **Step 5: Add a serialize/reparse test proving exact field names and PSBT keys survive the round trip and no private material enters JSON.**
- [ ] **Step 6: Run all engine tests, format, Clippy with warnings denied, and commit the signing slice.**

### Task 5: Thin host adapter and Luke CLI/regtest round trip

**Files:**
- Create: `tools/anzen-prime-adapter/Cargo.toml`
- Create: `tools/anzen-prime-adapter/src/main.rs`
- Create: `scripts/verify-luke-roundtrip.ps1`
- Create: `docs/evidence/policy-package-v4-roundtrip-2026-08-14.md`
- Modify: `Cargo.toml`
- Modify: `scripts/verify.ps1`
- Modify: `scripts/verify.sh`

**Interfaces:**
- CLI: `anzen-prime-adapter inspect <proposal.json>` prints only `PolicySummary`.
- CLI: `anzen-prime-adapter approve <proposal.json> <approved.json> --development-seed <hex>` is explicitly development-only and rejects seed logging.
- Script: creates an isolated temporary regtest/data directory, seeds Luke's simulated HWW state from the same deterministic development app seed, invokes Luke's unchanged `phone set-policy`, invokes this adapter, and invokes Luke's unchanged `phone activate-policy`.

- [ ] **Step 1: Add failing adapter integration tests for inspect, approve, invalid package, and no output file on failure.**
- [ ] **Step 2: Implement atomic approved-file output: write a sibling temporary file, flush/sync, and rename only after validation and serialization succeed.**
- [ ] **Step 3: Run the adapter tests and workspace verification.**
- [ ] **Step 4: Confirm Docker/Bitcoin Core availability without starting the Foundation VM; if available, run the bounded regtest script and capture commands, upstream SHA, transaction counts, signature counts, activation result, and proof boundary.**
- [ ] **Step 5: If live regtest is unavailable, keep the fixture tests green but mark CLI activation evidence honestly incomplete; do not substitute a mock acceptance claim.**
- [ ] **Step 6: Commit the host-adapter and evidence slice.**

### Task 6: Public documentation, CI, PR, and acceptance

**Files:**
- Modify: `README.md`
- Modify: `SECURITY.md`
- Modify: `CONTRIBUTING.md`
- Modify: `.github/workflows/rust.yml` only if the existing workspace command does not automatically include the new crates/tests.
- Modify: `docs/evidence/policy-package-v4-roundtrip-2026-08-14.md`

**Interfaces:**
- Public documentation states exactly what the host round trip proves and what Prime/transport/hardware evidence remains absent.
- PR closes issue `#5` only when all issue acceptance criteria are proved; otherwise it remains draft and the issue remains open.

- [ ] **Step 1: Update public docs with the real package flow, development file bridge, upstream commit, independent checks, and explicit non-claims.**
- [ ] **Step 2: Run `cargo test --workspace --all-targets`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, both verification scripts where supported, and `git diff --check`.**
- [ ] **Step 3: Review the final diff against every issue acceptance item and verify no seed, mnemonic, private path, generated regtest state, or build cache is tracked.**
- [ ] **Step 4: Push `codex/policy-package-v4-roundtrip`, open a focused PR linked to issue `#5`, and wait for required CI.**
- [ ] **Step 5: Inspect failures with the GitHub CI workflow, fix only evidenced causes, rerun local verification, push, and recheck until required CI is green or a concrete external blocker remains.**
- [ ] **Step 6: Record durable verified state and remaining proof boundary in the registered Jarvis Vault project home with the required change receipt, then report the PR and next proof.**
