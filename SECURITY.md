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

The policy engine caps imported packages at 128 KiB, requires the exact
manifest-to-PSBT set, validates the descriptor-bound recovery policy and full
transaction graph, verifies phone signatures before signing, and writes an
approved development file only after validation succeeds. These controls are
compatibility safeguards, not a production security audit.

The Prime app reads only the fixed development filename
`anzen-policy-v4.json` from the USB filesystem. Import and review do not request
app-seed material. A separate approval action obtains KeyOS's app-isolated seed,
zeroizes the copied seed after identity derivation, revalidates before signing,
writes a temporary file, and atomically renames it to
`anzen-policy-v4-approved.json`. It does not log seed material, private keys,
PSBT bodies, or approved JSON.

The checked-in seed, mnemonic, UTXO, PSBTs, and addresses used by the fixture
and regtest CI are deterministic disposable test vectors. They must never be
reused for mainnet funds. The host adapter's seed argument and the Prime USB
file bridge exist only to reproduce development proofs; neither is a production
transport or authorization design.
