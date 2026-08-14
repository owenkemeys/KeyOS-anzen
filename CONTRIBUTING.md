# Contributing

KeyOS Anzen is a small public compatibility proof. Changes should remain easy
to audit, reproduce, and attribute.

## Workflow

1. Open or reference a GitHub issue for material work.
2. Create a focused branch from `main`.
3. Keep commits small enough to explain and review.
4. Run `scripts/verify.sh` on Linux/macOS or `scripts/verify.ps1` on Windows.
5. Open a pull request; do not push feature work directly to `main`.
6. Record what the evidence proves: host test, Windows preview, Prime simulator,
   or physical Passport Prime. Do not treat one level as proof of another.

## Provenance

Changes to `vendor/anzen-cold-signer` must preserve its upstream license and
update `vendor/anzen-cold-signer/UPSTREAM.md` with the exact upstream commit.
Avoid unreviewable bulk rewrites of vendored code.

## Security and privacy

Never commit seed material, signing keys, local certificate identities,
device logs containing private data, machine-specific SSH configuration, or
private filesystem paths. Use deterministic test seeds only in host previews
and label them clearly.

Potential vulnerabilities should be reported through GitHub's private
vulnerability reporting for this repository instead of a public issue.
