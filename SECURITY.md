# Security policy

## Reporting a vulnerability

Please do not open a public issue for a suspected vulnerability or accidental
secret exposure. Use this repository's **Security** tab and choose
**Report a vulnerability** to start a private report.

Include the affected commit, the proof boundary involved (host, Windows
preview, Prime simulator, or physical device), reproduction steps, and the
potential impact. Do not include real wallet seeds, signing keys, or private
device data in the report.

This repository is currently a compatibility proof, not a production wallet.
That distinction limits its intended use but does not reduce the importance of
responsible disclosure.

## Policy-package proof boundary

The host policy engine caps imported packages at 128 KiB, requires the exact
manifest-to-PSBT set, validates the descriptor-bound recovery policy and full
transaction graph, verifies phone signatures before signing, and writes an
approved development file only after validation succeeds. These controls are
compatibility safeguards, not a production security audit.

The checked-in seed, mnemonic, UTXO, PSBTs, and addresses used by the fixture
and regtest CI are deterministic disposable test vectors. They must never be
reused for mainnet funds. The development adapter's seed argument exists only
to reproduce the host proof; production KeyOS integration must obtain the
app-isolated seed from the platform and must not expose it through files,
arguments, logs, or transport.
