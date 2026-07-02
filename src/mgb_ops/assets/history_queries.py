from __future__ import annotations

import sqlite3
from datetime import datetime
from pathlib import Path

import pandas as pd


def open_history_read_only(database_path: Path, *, timeout: float = 30.0) -> sqlite3.Connection:
    path = Path(database_path)
    if not path.exists():
        raise FileNotFoundError(f"History database not found: {path}")
    connection = sqlite3.connect(f"{path.resolve().as_uri()}?mode=ro", uri=True, timeout=timeout)
    connection.row_factory = sqlite3.Row
    connection.execute(f"PRAGMA busy_timeout = {int(timeout * 1000)}")
    connection.execute("PRAGMA query_only = ON")
    return connection


def read_station_catalog_tables(
    database_path: Path, *, start_time: datetime, end_time: datetime
) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    with open_history_read_only(database_path) as connection:
        stations = pd.read_sql_query(
            """SELECT station_id, station_code, provider_code, station_name,
                      mini_id, latitude AS lat, longitude AS lon
               FROM station WHERE latitude IS NOT NULL AND longitude IS NOT NULL
               ORDER BY provider_code, station_code""",
            connection,
        )
        series = pd.read_sql_query(
            "SELECT series_id, station_id, variable_code, state, created_at FROM observed_series",
            connection,
        )
        values = pd.read_sql_query(
            """SELECT os.series_id, os.station_id, os.variable_code,
                      ov.observed_at AS datetime, ov.value
               FROM observed_series os JOIN observed_value ov USING (series_id)
               WHERE ov.observed_at >= ? AND ov.observed_at <= ?""",
            connection,
            params=(start_time.strftime("%Y-%m-%d %H:%M:%S"), end_time.strftime("%Y-%m-%d %H:%M:%S")),
        )
    return stations, series, values


def read_station_observed_tables(
    station_id: str, database_path: Path, *, start_time: datetime, end_time: datetime
) -> tuple[pd.DataFrame, pd.DataFrame]:
    with open_history_read_only(database_path) as connection:
        series = pd.read_sql_query(
            """SELECT series_id, station_id, variable_code, state, created_at
               FROM observed_series WHERE station_id = ?""",
            connection,
            params=(str(station_id),),
        )
        values = pd.read_sql_query(
            """SELECT ov.series_id, os.variable_code, ov.observed_at AS datetime, ov.value
               FROM observed_value ov JOIN observed_series os USING (series_id)
               WHERE os.station_id = ? AND ov.observed_at >= ? AND ov.observed_at <= ?
               ORDER BY ov.observed_at, os.variable_code""",
            connection,
            params=(str(station_id), start_time.strftime("%Y-%m-%d %H:%M:%S"), end_time.strftime("%Y-%m-%d %H:%M:%S")),
        )
    return series, values


def read_rain_series(connection: sqlite3.Connection) -> pd.DataFrame:
    return pd.read_sql_query(
        """SELECT os.series_id, os.station_id, os.variable_code, os.state, os.created_at,
                  st.provider_code, st.latitude AS lat, st.longitude AS lon
           FROM observed_series os JOIN station st USING (station_id)
           WHERE os.variable_code='rain' AND st.latitude IS NOT NULL AND st.longitude IS NOT NULL""",
        connection,
    )


def read_observed_values(
    connection: sqlite3.Connection,
    series_ids: list[str],
    *,
    start_time: datetime,
    end_time: datetime,
    end_inclusive: bool = False,
    batch_size: int = 400,
) -> pd.DataFrame:
    if not series_ids:
        return pd.DataFrame(columns=["series_id", "station_id", "observed_at", "value"])
    operator = "<=" if end_inclusive else "<"
    frames: list[pd.DataFrame] = []
    for offset in range(0, len(series_ids), batch_size):
        chunk = series_ids[offset : offset + batch_size]
        placeholders = ",".join("?" for _ in chunk)
        frames.append(pd.read_sql_query(
            f"""SELECT ov.series_id, os.station_id, ov.observed_at, ov.value
                FROM observed_value ov JOIN observed_series os USING (series_id)
                WHERE ov.series_id IN ({placeholders})
                  AND ov.observed_at >= ? AND ov.observed_at {operator} ?
                ORDER BY ov.observed_at""",
            connection,
            params=(*chunk, start_time.strftime("%Y-%m-%d %H:%M"), end_time.strftime("%Y-%m-%d %H:%M")),
        ))
    return pd.concat(frames, ignore_index=True)


def find_asset(
    connection: sqlite3.Connection,
    *,
    asset_id: str | None = None,
    provider_code: str,
    asset_kind: str,
    valid_from_at_most: datetime,
    valid_to_at_least: datetime,
) -> dict[str, object] | None:
    asset_clause = "AND asset_id = ?" if asset_id is not None else ""
    params: list[object] = [provider_code, asset_kind]
    if asset_id is not None:
        params.append(asset_id)
    params.extend((valid_from_at_most.isoformat(timespec="seconds"), valid_to_at_least.isoformat(timespec="seconds")))
    cursor = connection.execute(
        f"""SELECT asset_id, relative_path, valid_from, valid_to
            FROM asset WHERE provider_code = ? AND asset_kind = ? {asset_clause}
              AND valid_from IS NOT NULL AND valid_to IS NOT NULL
              AND valid_from <= ? AND valid_to >= ?
            ORDER BY valid_from DESC, created_at DESC LIMIT 1""",
        params,
    )
    row = cursor.fetchone()
    if row is None:
        return None
    columns = (description[0] for description in cursor.description)
    return dict(zip(columns, row, strict=True))
