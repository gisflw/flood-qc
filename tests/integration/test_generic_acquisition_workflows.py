from __future__ import annotations

import csv
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import pytest

from db_helpers import initialize_history_db
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.assets.observations import CANONICAL_OBSERVED_COLUMNS
from mgb_ops.assets.spatial_grid import write_spatial_grid
from mgb_ops.config.env import RuntimeEnv
from mgb_ops.config.runtime import RuntimeContext
from mgb_ops.config.settings import DEFAULT_SETTINGS
from mgb_ops.config.workspace import RuntimePaths
from mgb_ops.workflows.forecast import ingest_forecast_asset
from mgb_ops.workflows.observed import ingest_from_csv


def _context(tmp_path: Path) -> RuntimeContext:
    workspace = tmp_path
    paths = RuntimePaths(workspace)
    paths.source_dir.mkdir(parents=True)
    initialize_history_db(paths.history_db)
    return RuntimeContext(paths=paths, settings=DEFAULT_SETTINGS, env=RuntimeEnv({}))


def test_ingest_from_canonical_csv_applies_authoritative_state_atomically(tmp_path) -> None:
    context = _context(tmp_path)
    csv_path = tmp_path / "canonical.csv"
    with csv_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=CANONICAL_OBSERVED_COLUMNS)
        writer.writeheader()
        writer.writerow(
            {
                "provider_code": "ana",
                "station_code": "74100000",
                "observed_at": "2026-03-10 00:00",
                "variable_code": "rain",
                "value": "2.5",
            }
        )

    summary = ingest_from_csv(context, csv_path, state="approved")

    assert summary.rows_imported == 1
    with HistoryRepository(context.paths.history_db) as repository:
        row = repository.connection.execute(
            "SELECT station_id, state FROM observed_series"
        ).fetchone()
    assert tuple(row) == ("ana:74100000", "approved")


def test_ingest_from_csv_rolls_back_when_a_late_row_is_invalid(tmp_path) -> None:
    context = _context(tmp_path)
    csv_path = tmp_path / "invalid.csv"
    with csv_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=CANONICAL_OBSERVED_COLUMNS)
        writer.writeheader()
        writer.writerow(
            {
                "provider_code": "ana",
                "station_code": "74100000",
                "observed_at": "2026-03-10 00:00",
                "variable_code": "rain",
                "value": "2.5",
            }
        )
        writer.writerow(
            {
                "provider_code": "ana",
                "station_code": "74100000",
                "observed_at": "2026-03-10 01:00",
                "variable_code": "rain",
                "value": "nan",
            }
        )

    with pytest.raises(ValueError, match="finite"):
        ingest_from_csv(context, csv_path)

    with HistoryRepository(context.paths.history_db) as repository:
        assert repository.connection.execute("SELECT COUNT(*) FROM observed_value").fetchone()[0] == 0


def test_ingest_forecast_asset_is_idempotent_and_detects_changed_content(tmp_path) -> None:
    context = _context(tmp_path)
    asset_path = context.paths.assets_dir / "ecmwf" / "forecast.nc"
    times = [
        datetime(2026, 3, 12, hour, tzinfo=timezone.utc)
        for hour in (1, 2)
    ]
    write_spatial_grid(
        asset_path,
        variable="precipitation",
        grid_type="forecast",
        source="resampled_from_grid",
        providers=["ecmwf"],
        units="mm",
        bbox=(-52.0, -30.0, -51.0, -29.0),
        resolution_degrees=1.0,
        times_utc=times,
        latitudes=np.array([-29.5]),
        longitudes=np.array([-51.5]),
        values=np.ones((2, 1, 1)),
        processing_metadata={
            "provider": "ecmwf",
            "model": "ifs",
            "product_type": "fc",
            "source_format": "GRIB2",
            "source_cycle_time": "2026-03-12T00:00:00Z",
            "source_resolution": "0p25",
            "source_parameter": "tp",
            "model_bbox": [-51.75, -29.75, -51.25, -29.25],
            "buffered_bbox": [-52.75, -30.75, -50.25, -28.25],
            "requested_bbox": [-52.75, -30.75, -50.25, -28.25],
            "effective_bbox": [-52.0, -30.0, -51.0, -29.0],
            "buffer_fraction": 2.0,
        },
    )

    first = ingest_forecast_asset(context, asset_path)
    second = ingest_forecast_asset(context, asset_path)

    assert first["asset_id"] == second["asset_id"]
    asset_path.write_bytes(asset_path.read_bytes() + b"changed")
    with pytest.raises((ValueError, OSError)):
        ingest_forecast_asset(context, asset_path)


def test_ingest_forecast_asset_replaces_obsolete_unbuffered_registration(tmp_path) -> None:
    context = _context(tmp_path)
    asset_path = context.paths.assets_dir / "ecmwf" / "forecast.nc"
    write_spatial_grid(
        asset_path,
        variable="precipitation",
        grid_type="forecast",
        source="cropped_from_native_grid",
        providers=["ecmwf"],
        units="mm",
        bbox=(-52.0, -30.0, -51.0, -29.0),
        resolution_degrees=1.0,
        times_utc=[datetime(2026, 3, 12, 3, tzinfo=timezone.utc)],
        latitudes=np.array([-29.5]),
        longitudes=np.array([-51.5]),
        values=np.ones((1, 1, 1)),
        processing_metadata={
            "provider": "ecmwf",
            "model": "ifs",
            "product_type": "fc",
            "source_format": "GRIB2",
            "source_cycle_time": "2026-03-12T00:00:00Z",
            "source_resolution": "0p25",
            "source_parameter": "tp",
            "model_bbox": [-51.75, -29.75, -51.25, -29.25],
            "buffered_bbox": [-52.75, -30.75, -50.25, -28.25],
            "requested_bbox": [-52.75, -30.75, -50.25, -28.25],
            "effective_bbox": [-52.0, -30.0, -51.0, -29.0],
            "buffer_fraction": 2.0,
        },
    )
    asset_id = "ecmwf.ifs.fc.20260312T000000Z.precipitation_grid"
    relative_path = asset_path.relative_to(context.paths.workspace).as_posix()
    with HistoryRepository(context.paths.history_db) as repository:
        repository.upsert_asset(
            asset_id=asset_id,
            asset_kind="spatial_grid",
            format="NetCDF",
            relative_path=relative_path,
            provider_code="ecmwf",
            valid_from="2026-03-12T00:00:00",
            valid_to="2026-03-12T03:00:00",
            metadata={"type": "forecast", "cycle_time": "2026-03-12T00:00:00Z"},
        )

    replaced = ingest_forecast_asset(context, asset_path)

    assert replaced["asset_id"] == asset_id
    assert '"buffer_fraction": 2.0' in replaced["metadata_json"]
