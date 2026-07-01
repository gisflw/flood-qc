from __future__ import annotations

from datetime import datetime

import pytest

from db_helpers import initialize_history_db
from mgb_ops.assets.forecast_registry import (
    find_forecast_asset_by_cycle,
    list_forecast_assets,
    register_forecast_asset,
    resolve_forecast_asset,
)


def test_forecast_asset_catalog_registers_lists_and_resolves(tmp_path) -> None:
    database_path = initialize_history_db(tmp_path / "history.sqlite")
    asset_path = tmp_path / "data" / "downloads" / "ecmwf" / "forecast.nc"
    asset_path.parent.mkdir(parents=True)
    asset_path.touch()
    cycle_time = datetime(2026, 3, 12)

    registered = register_forecast_asset(
        database_path,
        asset_id="ecmwf.ifs.fc.20260312T000000Z.precipitation_grid",
        format="NetCDF",
        path=asset_path,
        asset_base_dir=tmp_path,
        provider_code="ecmwf",
        valid_from=datetime(2026, 3, 12, 1),
        valid_to=datetime(2026, 3, 12, 3),
        metadata={"cycle_time": "2026-03-12T00:00:00Z"},
    )

    assert registered["relative_path"] == "data/downloads/ecmwf/forecast.nc"
    assets = list_forecast_assets(database_path, workspace_path=tmp_path)
    assert assets["asset_id"].tolist() == [registered["asset_id"]]
    row, resolved_path = resolve_forecast_asset(
        registered["asset_id"],
        database_path=database_path,
        workspace_path=tmp_path,
    )
    assert row["provider_code"] == "ecmwf"
    assert resolved_path == asset_path
    cycle_match = find_forecast_asset_by_cycle(
        database_path,
        workspace_path=tmp_path,
        provider_code="ecmwf",
        cycle_time=cycle_time,
    )
    assert cycle_match is not None
    assert cycle_match[1] == asset_path


def test_resolve_forecast_asset_rejects_missing_registered_file(tmp_path) -> None:
    database_path = initialize_history_db(tmp_path / "history.sqlite")
    asset_path = tmp_path / "missing.nc"
    register_forecast_asset(
        database_path,
        asset_id="ecmwf.missing",
        format="NetCDF",
        path=asset_path,
        asset_base_dir=tmp_path,
        provider_code="ecmwf",
        valid_from=datetime(2026, 3, 12, 1),
        valid_to=datetime(2026, 3, 12, 3),
        metadata={"cycle_time": "2026-03-12T00:00:00Z"},
    )

    with pytest.raises(FileNotFoundError, match="was not found"):
        resolve_forecast_asset(
            "ecmwf.missing",
            database_path=database_path,
            workspace_path=tmp_path,
        )
