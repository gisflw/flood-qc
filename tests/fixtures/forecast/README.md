# Forecast fixture provenance

The 2×2 synthetic fixtures retain the cycle and parameter metadata of the
regional saved sources. `ecmwf.grib2` uses CCSDS packing; the NOAA files use
simple packing. Their SHA-256 hashes are:

- `ecmwf.grib2`: `df1acb2350500441e8bd739751bbb89b0ac9cca5e138b63fddc2788ba2cb0724`
- `noaa-missing-clipped.grib2`: `5ceb0589423049bccd0f0b3da121cc9cd3b05c8a590f44829d7a43c36a53f652`
- `noaa.grib2`: `6d51645a2de94a11bcf4484d927fc601ad702707d1aef5604b98db9b49520fad`
- `noaa-duplicate-rolling.grib2`: `27888ec28afa920b98496410efd4066212999aa971c55d5b318ec257a4403a34`
- `noaa-conflicting.grib2`: `7a9b0ddb3def155d476caae8601ece2ec9339539560f3757cd79ad825ed8f00e`
- `noaa-changed-grid.grib2`: `caf8d60877a206930732f12fc55d05227d19420c495f47ce1ecf04ccfed4043b`

Source SHA-256: ECMWF `6a46222b3d1ca77009b397f8b995792442b2ba70b07fbe55621b7e001ce58893`;
NOAA `2c01f36868b35d92c1aa1b8a04cd0b9bdd6135cdc9a27c8dcc2c5ec10d57f7c4`.
The command below uses ecCodes 2.28.0 and the preserved files described in the
[regional case](../../../references/regression/rs-hourly/README.md). Routine
tests use the checked-in bytes and do not require Python or ecCodes.

```sh
../legacy_python/.venv/bin/python - <<'PY'
import eccodes as e
from pathlib import Path
root = Path('tests/fixtures/forecast')
base = Path('/workspace/regression-rs-hourly/capture/fixtures/sources')
for kind, file in [('noaa', 'gfs_fc_20260720T000000Z.grib2'),
                   ('ecmwf', 'ifs_fc_20260720T000000Z.grib2')]:
    with (base / file).open('rb') as f:
        h = e.codes_grib_new_from_file(f)
        for key, value in [('Ni', 2), ('Nj', 2),
                           ('latitudeOfFirstGridPointInDegrees', 0),
                           ('longitudeOfFirstGridPointInDegrees', 0),
                           ('latitudeOfLastGridPointInDegrees', 0.25),
                           ('longitudeOfLastGridPointInDegrees', 0.25),
                           ('jScansPositively', 1)]:
            e.codes_set(h, key, value)
        with (root / (kind + '.grib2')).open('wb') as out:
            for step, values in [(3, [.001, .002, .003, .004]),
                                 (6, [.003, .004, .006, .008])]:
                e.codes_set(h, 'stepRange', '0-' + str(step))
                e.codes_set_values(h, values)
                e.codes_write(h, out)
        e.codes_release(h)
with (root / 'noaa.grib2').open('rb') as f:
    a, b = e.codes_grib_new_from_file(f), e.codes_grib_new_from_file(f)
with (root / 'noaa-duplicate-rolling.grib2').open('wb') as out:
    e.codes_write(a, out); e.codes_write(a, out); e.codes_write(b, out)
    e.codes_set(b, 'stepRange', '3-6')
    e.codes_set_values(b, [99., 99., 99., 99.]); e.codes_write(b, out)
with (root / 'noaa-conflicting.grib2').open('wb') as out:
    e.codes_write(a, out); e.codes_set_values(a, [9., 9., 9., 9.]); e.codes_write(a, out)
with (root / 'noaa-changed-grid.grib2').open('wb') as out:
    e.codes_write(b, out); e.codes_set(b, 'longitudeOfFirstGridPointInDegrees', 1.)
    e.codes_set(b, 'longitudeOfLastGridPointInDegrees', 1.25); e.codes_write(b, out)
e.codes_release(a); e.codes_release(b)
with (root / 'noaa.grib2').open('rb') as f:
    h = e.codes_grib_new_from_file(f)
with (root / 'noaa-missing-clipped.grib2').open('wb') as out:
    for step, values in [('0', [0., 0., 0., 0.]),
                         ('0-3', [3., 6., 0., 9999.]),
                         ('0-6', [9., 3., 0., 2.])]:
        e.codes_set(h, 'stepRange', step); e.codes_set(h, 'missingValue', 9999.)
        e.codes_set(h, 'bitmapPresent', 1); e.codes_set_values(h, values)
        e.codes_write(h, out)
e.codes_release(h)
PY
```
