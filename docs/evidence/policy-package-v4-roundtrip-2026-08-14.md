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

Status: pending the public **Luke CLI regtest round trip** CI job on the pull
request for issue [#5](https://github.com/owenkemeys/KeyOS-anzen/issues/5).

That job pins Luke's commit, starts the upstream Bitcoin Core 28.1 regtest
service, funds a fresh vault, invokes Luke's unchanged `phone set-policy`,
replaces only the simulated HWW approval step with `anzen-prime-adapter`, then
invokes Luke's unchanged `phone activate-policy`. Success requires a rollover
broadcast, a 10,000,000-sat active monthly limit, twelve encrypted allowance
pairs, and 50,000,000-sat active emergency access.

## Proof boundary

This record can prove host parsing, independent validation, signing, JSON
compatibility, and unchanged-CLI activation on disposable Bitcoin Core
regtest. It cannot prove Anzen phone connectivity, Prime target compatibility,
Prime simulator execution, physical-device execution, secure persistence,
mainnet readiness, or real-funds safety. The Foundation SDK VM remained off
during this host-first phase.
