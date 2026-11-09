# RS hourly regression reference, revision 1

The bundle is `/workspace/regression-rs-hourly-r1` (outside Git). Copy it with `cp -a /workspace/regression-rs-hourly-r1 /path/to/rs-hourly-r1`. The manifest inventory uses paths relative to that bundle; absolute paths inside execution records and the operation SQLite files record the original execution. Reproduction below creates new working databases from the frozen original history and rebases the two selected forecast paths to the new location. Do not use operational data as a fallback.

The frozen input set is stored once. Capture retains each scenario's changed inputs, the four raw flow and level streams needed for decoding, decoded NetCDF, working grids, logs and recovery evidence. Repeat retains its decoded outputs, logs and comparison results; its byte-identical inputs and raw streams are represented by capture's copies. Temporary decoding files, repeat grids and ancillary engine outputs are regenerated during reproduction. The unused `HAND_Flood.asc` is omitted from the frozen input set. `manifest.json` identifies the retained files, scenario definitions and limits; `expectations.json` and bundle `comparison.json` contain measured full-array comparisons and checkpoints. Capture and repeat matched exactly, including coordinates and missingness; packed output resolution is 0.001 in each exported unit. NetCDF `history` and `date_created` differ between runs. Forecast scenarios differ in rainfall, flow and level while their observed prefixes match.

The saved forecast metadata referred to October and a 361-step horizon. In each disposable input, `capture.py` updates only the date and horizon to July 20 01:00 and 121 steps. The forecast correction covers 41 three-hour source intervals from July 20 03:00 through July 25 06:00 UTC; 0–126 hours encompasses all 121 hourly forecast targets. `transformed-native.npz` retains their transformed values, NaN mask, bounds, coordinates and timestamps. Inputs are never written to the operational source.

The engine writes separate 241-step observed and 121-step forecast streams. It also rewinds `YTUDO.MGB` and overwrites its first 121 observed-level steps with forecast levels. `recover-level.py` reruns the zero scenario in a private workspace with the same initial `DATA.hot` and forcing. The small `snapshot.c` interception copies the real observed-level stream immediately before that rewind. It checks all 241 observed flow steps and the 120 unaffected observed-level steps against each scenario. The staged decoding joins that snapshot with each scenario's raw forecast-level stream and joins the raw flow streams. The retained streams are unchanged. The `rewind-trace.txt`, recovery snapshot and logs document this adaptation; recovery inputs and outputs identical to the zero scenario are stored there once. The historical file-open trace is preserved in `provenance/engine-open-trace.log`; it was recorded during the earlier aligned zero-forcing diagnostic. The saved state date and engine state consumption remain uncertain; this is an empirical regression reference, not evidence of hydrological validity.

## Reproduce in a fresh directory

Use Linux x86-64, the ELF dependencies and Python packages in `environment.json`, GCC, and a clean `legacy_python` checkout at `15981642a7476fb583f63aeb89cdb1472cfd6b30`. The frozen SQLite is the original consistent backup; no source SQLite connection is needed. The code invokes the existing legacy APIs directly. Commands below create a new location and do not alter the reference bundle:

```bash
mkdir -p /path/to/rs-hourly-reproduction
cp -a /workspace/regression-rs-hourly-r1/frozen /path/to/rs-hourly-reproduction/
cp /workspace/regression-rs-hourly-r1/{capture.py,recover-level.py,snapshot.c,evidence.py,compare.py} /path/to/rs-hourly-reproduction/
cd /workspace/legacy_python
gcc -shared -fPIC -O2 -o /path/to/rs-hourly-reproduction/snapshot.so /path/to/rs-hourly-reproduction/snapshot.c -ldl
.venv/bin/python -u /path/to/rs-hourly-reproduction/capture.py capture > /path/to/rs-hourly-reproduction/capture.log 2>&1
.venv/bin/python -u /path/to/rs-hourly-reproduction/capture.py repeat > /path/to/rs-hourly-reproduction/repeat.log 2>&1
.venv/bin/python -u /path/to/rs-hourly-reproduction/recover-level.py capture > /path/to/rs-hourly-reproduction/recover-level-capture.log 2>&1
.venv/bin/python -u /path/to/rs-hourly-reproduction/recover-level.py repeat > /path/to/rs-hourly-reproduction/recover-level-repeat.log 2>&1
.venv/bin/python /path/to/rs-hourly-reproduction/evidence.py
.venv/bin/python /path/to/rs-hourly-reproduction/compare.py /workspace/regression-rs-hourly-r1
```

The scripts assert the six persisted null replacements, finite observed grid, source interval coverage, independent recovery alignment, equality of repeated model arrays and selected raw and input checkpoints, and equality with the archived reference. Inventory hashes identify preserved bytes; exact array comparison provides the numerical contract. The original backup changed the operational SQLite shared-memory sidecar's modification time; its pre-backup bytes were not hashed. Acquisition, Windows validation, state generation and Rust comparisons remain outside this case.
