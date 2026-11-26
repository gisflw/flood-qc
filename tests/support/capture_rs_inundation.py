"""Independent GDAL/NumPy capture of decoded RS mask expectations.

This qualification tool is not used by the application or ordinary Rust tests.
Hashes concatenate row-major 1024-pixel spatial tiles, with row-major pixels
inside each tile. Overview tiles use the same footprint, divided by their scale.
"""

import argparse
import hashlib
import json
from pathlib import Path
import sys

import numpy as np
from osgeo import gdal


TILE = 1024
LEVELS = (("inundation_mask", 1), ("inundation_mask_4x", 4), ("inundation_mask_16x", 16))
SCENARIOS = ("zero", "ecmwf-raw", "noaa-raw", "ecmwf-corrected")


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def overview(pixels, factor):
    height, width = pixels.shape
    rows, columns = (height + factor - 1) // factor, (width + factor - 1) // factor
    padded = np.full((rows * factor, columns * factor), 255, dtype=np.uint8)
    padded[:height, :width] = pixels
    blocks = padded.reshape(rows, factor, columns, factor)
    wet = (blocks == 1).sum(axis=(1, 3), dtype=np.uint32)
    covered = (blocks != 255).any(axis=(1, 3))
    row_sizes = np.minimum(factor, height - np.arange(rows) * factor)
    column_sizes = np.minimum(factor, width - np.arange(columns) * factor)
    denominator = row_sizes[:, None] * column_sizes[None, :]
    result = ((100 * wet + denominator // 2) // denominator).astype(np.uint8)
    result[~covered] = 255
    return result


def self_check():
    np.testing.assert_array_equal(
        overview(np.array([[0, 1, 255], [1, 255, 0]], dtype=np.uint8), 2),
        [[50, 0]],
    )
    np.testing.assert_array_equal(overview(np.full((3, 5), 255, dtype=np.uint8), 4), [[255, 255]])
    np.testing.assert_array_equal(overview(np.array([[1, 255, 0]], dtype=np.uint8), 4), [[33]])
    stages = np.subtract(np.array([3, 3, 3], dtype=np.float32), np.float32(2))
    hand = np.array([10, 0, -10], dtype=np.float32).astype(np.float64) / 10
    np.testing.assert_array_equal((stages.astype(np.float64) > hand).astype(np.uint8), [0, 1, 1])


def capture(reference, hand_path, catchments_path):
    gdal.UseExceptions()
    hand = gdal.Open(str(hand_path), gdal.GA_ReadOnly)
    catchments = gdal.Open(str(catchments_path), gdal.GA_ReadOnly)
    width, height = hand.RasterXSize, hand.RasterYSize
    assert (width, height) == (catchments.RasterXSize, catchments.RasterYSize)
    assert hand.GetGeoTransform() == catchments.GetGeoTransform()
    assert hand.GetProjection() == catchments.GetProjection()
    assert hand.RasterCount == catchments.RasterCount == 1
    cell_path = reference / "frozen/runner/Input/Cell.hig"
    cell = np.loadtxt(cell_path, skiprows=1, usecols=(1, 15))
    mini_ids = cell[:, 0].astype(np.int64)
    assert np.array_equal(mini_ids, cell[:, 0])
    assert len(np.unique(mini_ids)) == len(mini_ids) == 8649
    depths = cell[:, 1].astype(np.float32)
    order = np.argsort(mini_ids)
    sorted_ids = mini_ids[order]
    source_paths = {"hand": hand_path, "catchments": catchments_path, "cell": cell_path}
    segments = []
    for name in ("history",) + SCENARIOS:
        path = reference / f"capture/runs/{name}/Output/HANDTUDO.MGB"
        source_paths[name] = path
        raw = np.fromfile(path, dtype="<f4").reshape(241, len(mini_ids))
        assert np.isfinite(raw).all() and (raw >= 0).all()
        if name != "history":
            raw = raw[:121]
        segments.append((name, np.subtract(raw, depths[None, :], dtype=np.float32)))
    identities = {name: sha256(path) for name, path in source_paths.items()}
    digests = {
        name: {variable: [hashlib.sha256() for _ in stages] for variable, _ in LEVELS}
        for name, stages in segments
    }
    coverage = {"eligible": 0, "positive_hand": 0, "zero_hand": 0, "negative_hand": 0}
    hand_band, catchment_band = hand.GetRasterBand(1), catchments.GetRasterBand(1)
    tile_count = ((width + TILE - 1) // TILE) * ((height + TILE - 1) // TILE)
    number = 0
    for y in range(0, height, TILE):
        rows = min(TILE, height - y)
        for x in range(0, width, TILE):
            columns = min(TILE, width - x)
            heights = hand_band.ReadAsArray(x, y, columns, rows)
            ids = catchment_band.ReadAsArray(x, y, columns, rows).astype(np.int64)
            valid = np.isfinite(heights) & (heights != hand_band.GetNoDataValue()) & (ids > 0)
            valid &= hand_band.GetMaskBand().ReadAsArray(x, y, columns, rows) != 0
            valid &= catchment_band.GetMaskBand().ReadAsArray(x, y, columns, rows) != 0
            if catchment_band.GetNoDataValue() is not None:
                valid &= ids != catchment_band.GetNoDataValue()
            positions = np.searchsorted(sorted_ids, ids[valid])
            assert (positions < len(sorted_ids)).all()
            assert np.array_equal(sorted_ids[positions], ids[valid])
            positions = order[positions]
            hand_metres = heights[valid].astype(np.float64) / 10
            coverage["eligible"] += int(valid.sum())
            for key, selection in (
                ("positive_hand", heights > 0),
                ("zero_hand", heights == 0),
                ("negative_hand", heights < 0),
            ):
                coverage[key] += int((selection & valid).sum())
            pixels = np.full((rows, columns), 255, dtype=np.uint8)
            for name, stages in segments:
                for time, frame in enumerate(stages):
                    stage = frame[positions]
                    values = (stage.astype(np.float64) > hand_metres).astype(np.uint8)
                    values[~np.isfinite(stage)] = 255
                    pixels[valid] = values
                    for variable, factor in LEVELS:
                        values = pixels if factor == 1 else overview(pixels, factor)
                        digests[name][variable][time].update(values.tobytes(order="C"))
            number += 1
            print(f"RS oracle tile {number}/{tile_count}", file=sys.stderr, flush=True)
    for name, path in source_paths.items():
        assert sha256(path) == identities[name], f"input changed during capture: {name}"
    return {
        "schema_version": 1,
        "rule": "float64(float32(HANDTUDO - float32(channel_depth))) > float64(HAND) / 10; valid nonpositive HAND included",
        "hash_order": "spatial tiles in y/x order; row-major pixels within each tile; one digest per variable and time",
        "tile_size": TILE,
        "oracle": {
            "script_sha256": sha256(Path(__file__)),
            "gdal": gdal.VersionInfo("RELEASE_NAME"),
            "numpy": np.__version__,
        },
        "grid": {"width": width, "height": height, "epsg": int(hand.GetSpatialRef().GetAuthorityCode(None)), "transform": hand.GetGeoTransform()},
        "timeline": {"time_zone": "America/Sao_Paulo", "first_end": "2026-07-10T00:00:00", "historical_count": 241, "forecast_count": 121},
        "sources": identities,
        "coverage": coverage,
        "scenarios": [
            {
                "id": name,
                "sha256": {
                    variable: [digest.hexdigest() for digest in digests["history"][variable]]
                    + [digest.hexdigest() for digest in digests[name][variable]]
                    for variable, _ in LEVELS
                },
            }
            for name in SCENARIOS
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path)
    parser.add_argument("--hand", type=Path)
    parser.add_argument("--catchments", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    self_check()
    if args.self_check:
        print("RS inundation oracle self-check passed")
        return
    if any(value is None for value in (args.reference, args.hand, args.catchments, args.output)):
        parser.error("--reference, --hand, --catchments and --output are required")
    result = capture(args.reference, args.hand, args.catchments)
    with args.output.open("x") as output:
        json.dump(result, output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
