from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path
from typing import Any

import pandas as pd

from mgb_ops.assets.forecast_grid import FORECAST_PRECIPITATION_GRID_ASSET_KIND
from mgb_ops.storage.history_repository import HistoryRepository


def _parse_cycle_time(value: object) -> pd.Timestamp | None:
    if not value:
        return None
    try:
        timestamp = pd.Timestamp(value)
    except (TypeError, ValueError):
        return None
    if timestamp.tzinfo is not None:
        return timestamp.tz_convert("UTC").tz_localize(None)
    return timestamp


def build_relative_asset_path(path: Path, *, asset_base_dir: Path) -> str:
    resolved_path = Path(path).resolve()
    resolved_base = Path(asset_base_dir).resolve()
    try:
        return resolved_path.relative_to(resolved_base).as_posix()
    except ValueError:
        return Path(path).as_posix()


def register_forecast_asset(
    database_path: Path,
    *,
    asset_id: str,
    format: str,
    path: Path,
    asset_base_dir: Path,
    provider_code: str,
    valid_from: datetime,
    valid_to: datetime,
    metadata: dict[str, Any],
    asset_kind: str = FORECAST_PRECIPITATION_GRID_ASSET_KIND,
) -> dict[str, Any]:
    relative_path = build_relative_asset_path(path, asset_base_dir=asset_base_dir)
    with HistoryRepository(database_path) as repository:
        return repository.upsert_asset(
            asset_id=asset_id,
            asset_kind=asset_kind,
            format=format,
            relative_path=relative_path,
            provider_code=provider_code,
            valid_from=valid_from.isoformat(timespec="seconds"),
            valid_to=valid_to.isoformat(timespec="seconds"),
            metadata=metadata,
        )


def list_forecast_assets(
    database_path: Path,
    *,
    workspace_path: Path | None = None,
) -> pd.DataFrame:
    database = Path(database_path)
    if not database.exists():
        raise FileNotFoundError(f"History database not found: {database}")
    with HistoryRepository(database) as repository:
        rows = repository.list_assets(asset_kind=FORECAST_PRECIPITATION_GRID_ASSET_KIND)

    frame = pd.DataFrame(rows)
    if frame.empty:
        return frame.assign(asset_path=pd.Series(dtype=object), display_label=pd.Series(dtype=str))

    root = Path(workspace_path).resolve() if workspace_path is not None else database.parent.parent.resolve()
    frame["asset_path"] = frame["relative_path"].map(
        lambda value: Path(value) if Path(value).is_absolute() else root / Path(value)
    )
    frame["metadata"] = frame["metadata_json"].map(lambda value: json.loads(value) if value else {})
    frame["cycle_time"] = frame["metadata"].map(
        lambda value: value.get("cycle_time") if isinstance(value, dict) else None
    )
    frame["display_label"] = frame.apply(
        lambda row: f"{row['asset_id']} | cycle {row['cycle_time'] or row['valid_from'] or 'unknown'}",
        axis=1,
    )
    return frame.drop(columns="metadata")


def resolve_forecast_asset(
    asset_id: str,
    *,
    database_path: Path,
    workspace_path: Path | None = None,
) -> tuple[dict[str, object], Path]:
    assets = list_forecast_assets(database_path, workspace_path=workspace_path)
    selected = assets[assets["asset_id"] == asset_id]
    if selected.empty:
        raise ValueError(f"Canonical forecast asset {asset_id!r} was not found.")
    row = selected.iloc[0].to_dict()
    path = Path(row["asset_path"])
    if not path.exists():
        raise FileNotFoundError(f"Forecast NetCDF registered for {asset_id!r} was not found: {path}")
    return row, path


def find_forecast_asset_by_cycle(
    database_path: Path,
    *,
    workspace_path: Path,
    provider_code: str,
    cycle_time: datetime,
) -> tuple[dict[str, object], Path] | None:
    assets = list_forecast_assets(database_path, workspace_path=workspace_path)
    if assets.empty:
        return None
    assets = assets[assets["provider_code"] == provider_code]
    expected = pd.Timestamp(cycle_time)
    matches = assets[
        assets["cycle_time"].map(_parse_cycle_time).map(lambda value: value == expected)
    ]
    if matches.empty:
        return None
    row = matches.iloc[0].to_dict()
    return row, Path(row["asset_path"])
