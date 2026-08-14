# PolicyPackage v4 Passport Prime simulator evidence — 2026-08-14

This record covers the hosted Passport Prime simulator run of the real
PolicyPackage v4 import/review/approval flow. It is simulator runtime evidence,
not physical-device, phone-transport, hardware-seed, or real-funds evidence.

## Source and environment

- Repository: `owenkemeys/KeyOS-anzen`
- Branch: `codex/prime-policy-v4-simulator`
- Source commit: `2e753fdc8426980cc2007cc138dff921ddcb0e21`
- Tracked issue: `#7`
- Foundation SDK package: `0.4.0-x86_64-unknown-linux-gnu`
- Luke Childs' Anzen commit: `01794bb14d34d01413e3b539c7e5496ecbce0c87`
- Hosted app process reported by the simulator: PID `19`

## Transport and seed boundary

Foundation's hosted simulator does not mount the Prime USB volume. The
simulator build therefore imported the checked-in, Luke-generated regtest
fixture and wrote approved JSON to app-private simulator storage. The UI labelled
this boundary `PRIME SIMULATOR · REGTEST FIXTURE + TEST SEED`.

The first run reached review but approval failed because the hosted security
service rejected `GetAppSeed` without a logged-in hardware session. The corrected
simulator-only build used deterministic test seed `[0x42; 32]`. The device-target
build retained the real bounded USB adapter and KeyOS-isolated `GetAppSeed` call.

## Owner-observed flow

The owner exercised the corrected hosted app and accepted the visible result:

1. Imported the real PolicyPackage v4 fixture.
2. Reviewed the 2.00000000 BTC vault, 0.10000000 BTC monthly allowance for 12
   months, 0.50000000 BTC emergency amount after 7 days, regtest network,
   1 sat/vB fee rate, and 28 PSBT count.
3. Approved the package after the explicit review boundary.
4. Reached `Policy approved` and `Approved package written`.
5. Saw `28 PSBTs validated · 28 HWW signatures added` and confirmation that the
   approved JSON was saved in simulator app storage.

The runtime log independently reported:

```text
Anzen package approved: 28 PSBTs, 28 HWW signatures
```

## Captured artifact

- Screenshot: [`screenshots/prime-policy-v4-simulator-approved.png`](../../screenshots/prime-policy-v4-simulator-approved.png)
- Screenshot SHA-256:
  `1748ADD506D707E03F429E990B894363A4A182EA7A9874D6828F65D3C9419C00`

## Proof boundary

This proves that the hosted simulator visibly executed the labelled fixture
import, owner review, independent validation, deterministic test-key signing,
and app-private approved-package write. It does not prove the device USB bridge,
KeyOS hardware-backed seed access, Anzen phone connectivity, physical Passport
Prime execution, mainnet safety, broadcasting from Prime, or real-funds use.

The independent host round-trip record separately proves that Luke's unchanged
CLI accepts and activates output produced by the same policy engine on disposable
Bitcoin Core regtest state. The signed target record separately proves that the
accepted source compiles, strips, manifests, and signs for the Prime target.
