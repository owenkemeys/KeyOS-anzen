# Prime PolicyPackage v4 simulator integration plan

**Goal:** Replace the benchmark screen with a development-only Prime file flow
that imports, reviews, independently validates, signs, and exports Luke Childs'
real PolicyPackage v4 format.

**Tracked work:** Public issue `#7`, branch
`codex/prime-policy-v4-simulator`, stacked on host-engine PR `#6`.

## Constraints

- Keep Luke's protocol pinned to `01794bb14d34d01413e3b539c7e5496ecbce0c87`.
- Treat USB files as development transport, never phone connectivity.
- Label the hosted simulator's embedded real fixture and app-private output;
  Foundation's hosted environment does not mount a USB volume.
- Do not request app-seed material during import or review.
- Sign only after an explicit owner approval and full independent validation.
- Bound imports to 128 KiB, avoid secret/PSBT logging, and use temporary-write
  plus atomic rename for approved output.
- Keep target/simulator output bounded and save the VM whenever it is not needed.

## Delivery slices

- [x] Add failing tests for real import, approval ordering, identity mismatch,
  Luke-compatible approved JSON, and non-destructive export failure.
- [x] Implement the host-testable review/approval/export state boundary.
- [x] Replace the benchmark UI with import, review, approve, and success states.
- [x] Add the KeyOS USB filesystem and app-seed adapters.
- [x] Prove the complete app builds as a signed `armv7a-unknown-xous-elf` bundle.
- [ ] Run and visually verify the hosted Prime simulator flow with the real fixture.
- [ ] Have the owner complete the compact simulator playtest card.
- [ ] Push the focused PR, wait for CI, and record final durable evidence.
