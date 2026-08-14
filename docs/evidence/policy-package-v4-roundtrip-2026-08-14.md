# Anzen PolicyPackage v4 host round trip

## Scope

This milestone replaces the benchmark-only host path with a real Anzen policy
package on the host. It does not add a phone transport or claim that the policy
engine runs on Passport Prime.

## Pinned inputs

- Luke Childs' Anzen repository commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Package version and kind: `4`, `vault-policy`
- Checked fixture SHA-256: `e54ac6aa0738ee833982368fa673e3b59ba46a5f047f334c18321143b17c3c62`
- Fixture policy: 2 BTC input, 0.1 BTC monthly limit, twelve allowance pairs, 0.5 BTC emergency access, fixed 1 sat/vB
- Development HWW x-only key: `906979e124fa1d21104bbcfce84d825cb1269fa1dc95a0a27e9bfe120d88e97c`

The fixture is deterministic disposable regtest data created through Luke's
`build_policy_proposal` and `package_from_batch` path. It contains no mnemonic,
seed, extended private key, live wallet data, or real funds.

## Independent host checks

Before adding any HWW signature, `anzen-policy-engine` checks:

- the 128 KiB import cap, package/manifest version, kind, safe relative paths,
  and exact manifest-to-PSBT key set;
- regtest network, canonical NUMS-key descriptor, vault address, phone key, HWW
  key, and both fixed recovery paths and delays;
- rollover inputs, outputs, amounts, transaction ID, sequence, and 1 sat/vB fee;
- all twelve authorization/revocation pairs, chain outpoints, sequences,
  destinations, amounts, dust limits, and fees;
- emergency trigger, withdrawal, cancellation, destination, delay, outpoints,
  amounts, and fees;
- every PSBT witness UTXO and every phone Taproot script-path signature.

Only the resulting `ValidatedPolicy` type can reach HWW signing. The signer
derives Luke's BIP86 vault path from app-isolated seed material, verifies the
derived HWW key against the descriptor, inserts and immediately verifies every
HWW Schnorr signature, sets `hww_approved`, and serializes the same v4 fields.

The tamper suite was mutation-checked: bypassing descriptor, graph, amount,
destination, fee, and phone-signature validation made the rejection tests fail;
restoring the validator made them pass.

## Luke CLI/regtest acceptance

Status: passed in the public **Luke CLI regtest round trip** job for
[pull request #6](https://github.com/owenkemeys/KeyOS-anzen/pull/6),
[Actions run 31830009457](https://github.com/owenkemeys/KeyOS-anzen/actions/runs/31830009457).

That job pins Luke's commit, starts the upstream Bitcoin Core 28.1 regtest
service, funds a fresh vault, invokes Luke's unchanged `phone set-policy`,
replaces only the simulated HWW approval step with `anzen-prime-adapter`, then
invokes Luke's unchanged `phone activate-policy`. Success requires a rollover
broadcast, a 10,000,000-sat active monthly limit, twelve encrypted allowance
pairs, and 50,000,000-sat active emergency access.

The successful disposable run recorded:

- proposal SHA-256: `a1b985b9b2875924bc66b8b776150a254a35b073d1403fabee81d6124f0c3586`;
- approved-package SHA-256: `917cf71971aa9cacdcd3c9a08a3d76af0d580f2922db264d9043bf102c5b3d0d`;
- rollover transaction ID: `70796700c1af7f8d8c0c125482169cfea970983d6bc6655999b1e1687a3fa9c6`;
- 200,000,000-sat input, 10,000,000-sat monthly limit, twelve allowance
  pairs, 50,000,000-sat emergency access, and 28 HWW-signed PSBTs.

## Proof boundary

This record can prove host parsing, independent validation, signing, JSON
compatibility, and unchanged-CLI activation on disposable Bitcoin Core
regtest. It cannot prove Anzen phone connectivity, Prime target compatibility,
Prime simulator execution, physical-device execution, secure persistence,
mainnet readiness, or real-funds safety. The Foundation SDK VM remained off
during this host-first phase.
