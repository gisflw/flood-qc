from __future__ import annotations

import sqlite3
from datetime import datetime

import pytest

from mgb_ops.assets.history_queries import find_asset


@pytest.fixture
def asset_connection() -> sqlite3.Connection:
    connection = sqlite3.connect(":memory:")
    connection.execute(
        """
        CREATE TABLE asset (
            asset_id TEXT PRIMARY KEY,
            asset_kind TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            provider_code TEXT NOT NULL,
            valid_from TEXT,
            valid_to TEXT,
            created_at TEXT NOT NULL
        )
        """
    )
    connection.execute(
        """
        INSERT INTO asset (
            asset_id, asset_kind, relative_path, provider_code,
            valid_from, valid_to, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        """,
        (
            "ecmwf:20260311T000000",
            "forecast_precipitation_grid",
            "data/downloads/ecmwf/forecast.nc",
            "ecmwf",
            "2026-03-11T00:00:00",
            "2026-03-13T00:00:00",
            "2026-03-11T01:00:00",
        ),
    )
    try:
        yield connection
    finally:
        connection.close()


def _find_asset(connection: sqlite3.Connection) -> dict[str, object] | None:
    return find_asset(
        connection,
        provider_code="ecmwf",
        asset_kind="forecast_precipitation_grid",
        valid_from_at_most=datetime(2026, 3, 11),
        valid_to_at_least=datetime(2026, 3, 12),
    )


def test_find_asset_supports_default_tuple_rows(asset_connection: sqlite3.Connection) -> None:
    assert _find_asset(asset_connection) == {
        "asset_id": "ecmwf:20260311T000000",
        "relative_path": "data/downloads/ecmwf/forecast.nc",
        "valid_from": "2026-03-11T00:00:00",
        "valid_to": "2026-03-13T00:00:00",
    }


def test_find_asset_supports_sqlite_rows(asset_connection: sqlite3.Connection) -> None:
    asset_connection.row_factory = sqlite3.Row

    assert _find_asset(asset_connection) == {
        "asset_id": "ecmwf:20260311T000000",
        "relative_path": "data/downloads/ecmwf/forecast.nc",
        "valid_from": "2026-03-11T00:00:00",
        "valid_to": "2026-03-13T00:00:00",
    }


def test_find_asset_returns_none_when_no_asset_matches(asset_connection: sqlite3.Connection) -> None:
    result = find_asset(
        asset_connection,
        provider_code="noaa",
        asset_kind="forecast_precipitation_grid",
        valid_from_at_most=datetime(2026, 3, 11),
        valid_to_at_least=datetime(2026, 3, 12),
    )

    assert result is None
