# Passport Prime Anzen hardware timing test

Use this card only after the timing-enabled app can be sideloaded onto a physical
Passport Prime. Simulator and Windows-preview timings are deliberately labelled
as non-hardware and must not be reported as Prime performance.

## Test material

- App source commit: `4507a43ce0f2d0b04e61295b865136c2eee28b39`
- Input file: `fixtures/policy-package-v4/regtest-proposal.json`
- Expected workload: 28 independently validated PSBTs and 28 added HWW signatures
- Network: regtest
- No real funds

## Run five approvals

1. Start the sideloaded app after a fresh app launch.
2. Import `anzen-policy-v4.json` from the development USB folder.
3. Confirm the review shows 2.00000000 BTC protected, 0.10000000 BTC monthly
   access for 12 months, and 0.50000000 BTC emergency access after 7 days.
4. Press **Approve and export** once.
5. Photograph or transcribe the three displayed measurements:
   **Validation**, **Signing**, and **Total approval**.
6. Confirm the screen says **PHYSICAL PRIME TIMING**, 28 PSBTs validated, and
   28 HWW signatures added.
7. Choose **Import another package** and repeat until five successful runs are
   recorded. Treat run 1 as the cold run and runs 2–5 as warm runs.

| Run | Validation | Signing | Total approval | Result |
| --- | ---: | ---: | ---: | --- |
| 1 cold |  |  |  |  |
| 2 warm |  |  |  |  |
| 3 warm |  |  |  |  |
| 4 warm |  |  |  |  |
| 5 warm |  |  |  |  |

## Report to Luke

Report the cold result plus the warm signing range and median. Include the exact
source commit and state that total approval also includes KeyOS app-seed access,
serialization, and the atomic approved-file write. Do not describe simulator
results, a signed build, or a successful sideload as physical signing evidence;
only completed on-device approvals count.
