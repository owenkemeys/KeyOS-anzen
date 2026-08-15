# Phone-backup review fixtures

These files contain only a non-decryptable review envelope and a synthetic
VaultConfig. They exercise bounded, pre-seed owner review without checking a
phone mnemonic or decryptable recovery secret into the repository.

End-to-end decryption compatibility is proved in public CI by generating a
fresh backup with Luke's unchanged CLI and comparing normalized recovery JSON.
