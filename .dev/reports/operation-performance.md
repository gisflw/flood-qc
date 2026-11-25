# RS hourly operation performance

On November 25, 2026, release builds from `460ed7f` and this change ran
three paired offline trials on the [frozen July 20 RS case](../../references/regression/rs-hourly/README.md).
The case has 8,649 mini-basins, 241 historical and 121 forecast hours, one
history-only MGB process and four forecast scenarios. Both builds used Rust
1.99.1, the frozen Linux MGB executable, four physical cores of a Xeon
w3-2535, one OpenMP thread per MGB process, and the same local ext4 storage.

Both runs in each pair used the same local inputs, including a copy of the
canonical observation fixture with the current schema and two locally
normalized saved forecasts. Each trial timed preparation, then timed execution
from one shared saved preparation into a fresh initialized SQLite workspace.
Preparation and execution are wall-clock measurements. Engine time is the
history engine duration plus the longest forecast engine duration, from
execution records. Linux process sampling at 50 ms intervals measured the
gap after history and observed four forecast engines running simultaneously
in every trial.

| Stage | Baseline median (range) | Optimized median (range) | Median change |
| --- | ---: | ---: | ---: |
| Prepare | 22.9 s (22.5–55.5) | 20.3 s (14.8–20.6) | -11% |
| Execute saved preparation | 62.7 s (60.5–78.2) | 34.7 s (31.5–35.0) | -45% |
| Prepare + execute | 85.6 s (83.0–133.8) | 55.0 s (46.3–55.7) | -36% |
| History engine exit to first forecast start | 16.4 s (15.7–23.4) | 0.9 s (0.9–1.0) | -94% |
| History plus longest forecast engine | 21.1 s (20.2–21.5) | 20.6 s (19.3–21.3) | -3% |

The execution changes buffer CHUVABIN writes and stage each scenario in its
own worker. Retained copies sync when the enclosing stage is finalized.
Combined prepare-and-run workflows reuse the preparation they create inside
the private attempt. Rainfall interpolation sorts only the selected grid
neighbors and keeps the original tie break and summation order.

All trials matched the SHA256 identities of 192 prepared members and
50 published payloads, including raw flow and stage streams, basin and
station results, flood sources, and rainfall cache data. The four scenario
CHUVABIN digests matched the frozen capture. Every execution published a
completed attempt and its active generation in SQLite.

These timings characterize the saved Linux case on this host. The synthetic
test suite used separate cores and shared storage during part of the timing
window. Timings exclude data acquisition and the combined prepare-and-run
workflow. Windows remains unverified.
