# Passport Prime recovery-friend test

Use this card only with disposable regtest material. A signed target build proves
compilation and packaging, not physical execution or recovery.

## Signed kit identity

- Source: `4daaaed3e943fc36ce4ee922139ea46b97a1af65`
- `app.elf`: 11,424,512 bytes; SHA-256
  `836763a57d5c4373e89acdb2c06b3ca247b0e4625eb462655e82dd594404bc41`
- `manifest.json`: 823 bytes; SHA-256
  `eb9b15ddef28d7ac8c8623322f6b821f10b86ecf277b68d0f3778a8336b209a8`

## Maintainer preparation

Put these disposable files in Prime's development USB folder:

- `anzen-current-phone-backup-v1.json`
- `anzen-current-config-v1.json`
- `anzen-recovery-friend-public.asc`

Keep the matching friend private key off Prime. Give the owner the expected
OpenPGP fingerprint and regtest vault address through a separate trusted view.
Never use mainnet data, real funds, or a real recovery identity for this test.

## Owner steps

1. Sideload the signed app and open **Anzen Proof**.
2. Choose **Add recovery friend**.
3. Compare every character of the displayed fingerprint and vault address with
   the maintainer's expected values. Stop if either differs.
4. Confirm the displayed friend count will increase by one.
5. Choose **Approve and export** once.
6. Confirm the app reports **Recovery friend added** and exports
   `anzen-phone-backup-friend-added-v1.json`.
7. Return that file privately to the maintainer without opening or publishing
   it. Do not send a mnemonic, app seed, private key, or recovery JSON in chat.

## Acceptance

The physical test counts only after both of these are true:

1. Prime completed the review and atomic export on the physical device.
2. Unchanged Luke at `01794bb14d34d01413e3b539c7e5496ecbce0c87`
   decrypted the exported backup with the matching friend private key and
   produced the descriptor-bound version-2 recovery package.

Until then, report the result as either `signed target build only` or
`physical UI/export only`. Neither label is physical recovery parity.
