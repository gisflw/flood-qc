from __future__ import annotations

import json
import sqlite3
from dataclasses import asdict, is_dataclass
from datetime import datetime
from pathlib import Path
from typing import Any


def build_observed_series_id(station_id: str, variable_code: str, state: str = "raw") -> str:
    return f"{station_id}.{variable_code}.{state}"


class HistoryRepository:
    def __init__(self, database_path: Path) -> None:
        self.database_path = Path(database_path)
        self.connection = sqlite3.connect(self.database_path, timeout=5.0)
        self.connection.row_factory = sqlite3.Row
        self.connection.execute("PRAGMA foreign_keys = ON")
        self.connection.execute("PRAGMA busy_timeout = 5000")
        self.connection.execute("PRAGMA journal_mode = WAL")
        self.connection.execute("PRAGMA synchronous = NORMAL")
        self._validate_expected_schema()

    def __enter__(self) -> HistoryRepository:
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()

    def close(self) -> None:
        self.connection.close()

    def _require_exact_columns(self, table_name: str, expected_columns: set[str]) -> None:
        found_columns = {
            row["name"]
            for row in self.connection.execute(f"PRAGMA table_info({table_name})").fetchall()
        }
        if found_columns != expected_columns:
            raise RuntimeError(
                f"History database is incompatible with the current schema for {table_name}. "
                f"Expected {sorted(expected_columns)}, found {sorted(found_columns)}. "
                f"Delete {self.database_path} and run `mgb-ops bootstrap history` "
                "to recreate the database."
            )

    def _validate_expected_schema(self) -> None:
        self._require_exact_columns(
            "asset",
            {
                "asset_id",
                "asset_kind",
                "format",
                "relative_path",
                "provider_code",
                "checksum",
                "valid_from",
                "valid_to",
                "metadata_json",
                "created_at",
            },
        )
        self._require_exact_columns(
            "observed_series",
            {
                "series_id",
                "station_id",
                "variable_code",
                "state",
                "created_at",
            },
        )
        self._require_exact_columns(
            "manual_edit",
            {
                "manual_edit_id",
                "asset_id",
                "t0_step",
                "t1_step",
                "shift_lat",
                "shift_lon",
                "rotation_deg",
                "multiplication_factor",
                "editor",
                "reason",
                "metadata_json",
                "created_at",
            },
        )

        variable_codes = {
            row["variable_code"]
            for row in self.connection.execute("SELECT variable_code FROM variable").fetchall()
        }
        expected_variables = {"rain", "level", "flow"}
        if not expected_variables.issubset(variable_codes):
            raise RuntimeError(
                "History database is incompatible with the current variable catalog. "
                f"Expected at least {sorted(expected_variables)}, found {sorted(variable_codes)}. "
                f"Delete {self.database_path} and run `mgb-ops bootstrap history` "
                "to recreate the database."
            )

        provider_codes = {
            row["provider_code"]
            for row in self.connection.execute("SELECT provider_code FROM provider").fetchall()
        }
        expected_providers = {"ana", "inmet", "ecmwf"}
        if not expected_providers.issubset(provider_codes):
            raise RuntimeError(
                "History database is incompatible with the current provider catalog. "
                f"Expected at least {sorted(expected_providers)}, found {sorted(provider_codes)}. "
                f"Delete {self.database_path} and run `mgb-ops bootstrap history` "
                "to recreate the database."
            )

    def get_provider_stations(self, provider_code: str) -> list[dict[str, Any]]:
        rows = self.connection.execute(
            """
            SELECT station_id, station_code, station_name, provider_code
            FROM station
            WHERE provider_code = ?
            ORDER BY station_code
            """,
            (provider_code,),
        ).fetchall()
        return [dict(row) for row in rows]

    def _get_observed_series_id(self, station_id: str, variable_code: str, state: str) -> str | None:
        row = self.connection.execute(
            """
            SELECT series_id
            FROM observed_series
            WHERE station_id = ? AND variable_code = ? AND state = ?
            """,
            (station_id, variable_code, state),
        ).fetchone()
        if row is None:
            return None
        return str(row["series_id"])

    def ensure_observed_series(self, station_id: str, variable_code: str, state: str = "raw") -> str:
        existing_series_id = self._get_observed_series_id(station_id, variable_code, state)
        if existing_series_id is not None:
            return existing_series_id

        series_id = build_observed_series_id(station_id, variable_code, state)
        self.connection.execute(
            """
            INSERT INTO observed_series (
                series_id,
                station_id,
                variable_code,
                state
            ) VALUES (?, ?, ?, ?)
            ON CONFLICT(station_id, variable_code, state) DO NOTHING
            """,
            (series_id, station_id, variable_code, state),
        )
        self.connection.commit()
        ensured_series_id = self._get_observed_series_id(station_id, variable_code, state)
        if ensured_series_id is None:
            raise RuntimeError(
                "Failed to ensure observed_series "
                f"station_id={station_id} variable_code={variable_code} state={state}."
            )
        return ensured_series_id

    def upsert_observed_values(self, series_id: str, rows: list[tuple[str, float]]) -> int:
        if not rows:
            return 0
        self.connection.executemany(
            """
            INSERT INTO observed_value (
                series_id,
                observed_at,
                value
            ) VALUES (?, ?, ?)
            ON CONFLICT(series_id, observed_at) DO UPDATE SET
                value = excluded.value
            """,
            [(series_id, observed_at, value) for observed_at, value in rows],
        )
        self.connection.commit()
        return len(rows)

    def get_asset_by_relative_path(self, relative_path: str) -> dict[str, Any] | None:
        row = self.connection.execute(
            """
            SELECT
                asset_id,
                asset_kind,
                format,
                relative_path,
                provider_code,
                checksum,
                valid_from,
                valid_to,
                metadata_json,
                created_at
            FROM asset
            WHERE relative_path = ?
            """,
            (relative_path,),
        ).fetchone()
        if row is None:
            return None
        return dict(row)

    def get_asset_by_id(self, asset_id: str) -> dict[str, Any] | None:
        row = self.connection.execute(
            """
            SELECT
                asset_id,
                asset_kind,
                format,
                relative_path,
                provider_code,
                checksum,
                valid_from,
                valid_to,
                metadata_json,
                created_at
            FROM asset
            WHERE asset_id = ?
            """,
            (asset_id,),
        ).fetchone()
        if row is None:
            return None
        return dict(row)

    def list_assets(
        self,
        *,
        provider_code: str | None = None,
        asset_kind: str | None = None,
    ) -> list[dict[str, Any]]:
        clauses: list[str] = []
        params: list[str] = []
        if provider_code is not None:
            clauses.append("provider_code = ?")
            params.append(provider_code)
        if asset_kind is not None:
            clauses.append("asset_kind = ?")
            params.append(asset_kind)
        where_sql = f"WHERE {' AND '.join(clauses)}" if clauses else ""
        rows = self.connection.execute(
            f"""
            SELECT
                asset_id,
                asset_kind,
                format,
                relative_path,
                provider_code,
                checksum,
                valid_from,
                valid_to,
                metadata_json,
                created_at
            FROM asset
            {where_sql}
            ORDER BY COALESCE(valid_from, created_at) DESC, created_at DESC
            """,
            params,
        ).fetchall()
        return [dict(row) for row in rows]

    def list_ecmwf_assets(self, *, asset_kind: str) -> list[dict[str, Any]]:
        return self.list_assets(provider_code="ecmwf", asset_kind=asset_kind)

    def upsert_asset(
        self,
        *,
        asset_id: str,
        asset_kind: str,
        format: str,
        relative_path: str,
        provider_code: str | None,
        checksum: str | None = None,
        valid_from: str | None = None,
        valid_to: str | None = None,
        metadata: dict | None = None,
    ) -> dict[str, Any]:
        metadata_json = json.dumps(metadata or {}, sort_keys=True, ensure_ascii=True)
        self.connection.execute(
            """
            INSERT INTO asset (
                asset_id,
                asset_kind,
                format,
                relative_path,
                provider_code,
                checksum,
                valid_from,
                valid_to,
                metadata_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(relative_path) DO UPDATE SET
                asset_id = excluded.asset_id,
                asset_kind = excluded.asset_kind,
                format = excluded.format,
                provider_code = excluded.provider_code,
                checksum = excluded.checksum,
                valid_from = excluded.valid_from,
                valid_to = excluded.valid_to,
                metadata_json = excluded.metadata_json
            """,
            (
                asset_id,
                asset_kind,
                format,
                relative_path,
                provider_code,
                checksum,
                valid_from,
                valid_to,
                metadata_json,
            ),
        )
        self.connection.commit()
        ensured_asset = self.get_asset_by_relative_path(relative_path)
        if ensured_asset is None:
            raise RuntimeError(f"Falha ao garantir asset relative_path={relative_path}.")
        return ensured_asset

    def find_latest_asset(
        self,
        reference_time: datetime | str,
        *,
        asset_kind: str,
        provider_code: str | None = None,
    ) -> dict[str, Any] | None:
        if isinstance(reference_time, datetime):
            reference_text = reference_time.isoformat(timespec="seconds")
        else:
            reference_text = str(reference_time)
        provider_clause = "AND provider_code = ?" if provider_code is not None else ""
        params: list[str] = [asset_kind]
        if provider_code is not None:
            params.append(provider_code)
        params.extend([reference_text, reference_text])
        row = self.connection.execute(
            f"""
            SELECT
                asset_id,
                asset_kind,
                format,
                relative_path,
                provider_code,
                checksum,
                valid_from,
                valid_to,
                metadata_json,
                created_at
            FROM asset
            WHERE asset_kind = ?
              {provider_clause}
              AND valid_from IS NOT NULL
              AND valid_to IS NOT NULL
              AND valid_from <= ?
              AND valid_to >= ?
            ORDER BY valid_from DESC, created_at DESC
            LIMIT 1
            """,
            params,
        ).fetchone()
        if row is None:
            return None
        return dict(row)

    def find_latest_ecmwf_asset(self, reference_time: datetime | str, *, asset_kind: str) -> dict[str, Any] | None:
        return self.find_latest_asset(reference_time, provider_code="ecmwf", asset_kind=asset_kind)

    def _normalize_forecast_manual_edit_row(self, asset_id: str, row: dict[str, Any], row_number: int) -> dict[str, Any]:
        if row.get("asset_id") not in (None, "", asset_id):
            raise ValueError(f"Row {row_number}: asset_id differs from the selected asset.")

        try:
            t0_step = int(row["t0_step"])
            t1_step = int(row["t1_step"])
        except (KeyError, TypeError, ValueError) as exc:
            raise ValueError(f"Row {row_number}: invalid t0_step/t1_step.") from exc

        if t1_step < t0_step:
            raise ValueError(f"Row {row_number}: t1_step must be >= t0_step.")

        try:
            shift_lat = float(row.get("shift_lat", 0.0))
            shift_lon = float(row.get("shift_lon", 0.0))
            rotation_deg = float(row.get("rotation_deg", 0.0))
            multiplication_factor = float(row.get("multiplication_factor", 1.0))
        except (TypeError, ValueError) as exc:
            raise ValueError(f"Row {row_number}: invalid numeric parameters.") from exc

        if multiplication_factor <= 0:
            raise ValueError(f"Row {row_number}: multiplication_factor must be > 0.")

        reason = str(row.get("reason", "") or "").strip()
        if not reason:
            raise ValueError(f"Row {row_number}: reason is required.")

        editor_raw = row.get("editor")
        if editor_raw is None:
            editor = None
        else:
            editor = str(editor_raw).strip() or None

        metadata_raw = row.get("metadata")
        if metadata_raw is None and "metadata_json" in row:
            metadata_json_raw = row.get("metadata_json")
            if metadata_json_raw in (None, ""):
                metadata_raw = {}
            elif isinstance(metadata_json_raw, str):
                try:
                    metadata_raw = json.loads(metadata_json_raw)
                except json.JSONDecodeError as exc:
                    raise ValueError(f"Row {row_number}: invalid metadata_json.") from exc
            elif isinstance(metadata_json_raw, dict):
                metadata_raw = metadata_json_raw
            else:
                raise ValueError(f"Row {row_number}: invalid metadata_json.")
        metadata = serialize_metadata_payload(metadata_raw)

        return {
            "asset_id": asset_id,
            "t0_step": t0_step,
            "t1_step": t1_step,
            "shift_lat": shift_lat,
            "shift_lon": shift_lon,
            "rotation_deg": rotation_deg,
            "multiplication_factor": multiplication_factor,
            "editor": editor,
            "reason": reason,
            "metadata_json": json.dumps(metadata, sort_keys=True, ensure_ascii=True),
        }

    @staticmethod
    def _ensure_no_forecast_overlap(rows: list[dict[str, Any]]) -> None:
        ordered = sorted(rows, key=lambda item: (int(item["t0_step"]), int(item["t1_step"])))
        for previous, current in zip(ordered, ordered[1:]):
            if int(current["t0_step"]) < int(previous["t1_step"]):
                raise ValueError(
                    "Sobreposicao de correcoes no mesmo asset: "
                    f"[{previous['t0_step']}, {previous['t1_step']}] x [{current['t0_step']}, {current['t1_step']}]."
                )

    def list_forecast_manual_edits(self, asset_id: str) -> list[dict[str, Any]]:
        rows = self.connection.execute(
            """
            SELECT
                manual_edit_id,
                asset_id,
                t0_step,
                t1_step,
                shift_lat,
                shift_lon,
                rotation_deg,
                multiplication_factor,
                editor,
                reason,
                metadata_json,
                created_at
            FROM manual_edit
            WHERE asset_id = ?
            ORDER BY t0_step, t1_step, manual_edit_id
            """,
            (asset_id,),
        ).fetchall()
        return [dict(row) for row in rows]

    def replace_forecast_manual_edits(self, asset_id: str, rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
        if self.get_asset_by_id(asset_id) is None:
            raise ValueError(f"Asset {asset_id!r} was not found in history.")

        normalized_rows = [
            self._normalize_forecast_manual_edit_row(asset_id, row, row_number=index + 1)
            for index, row in enumerate(rows)
        ]
        self._ensure_no_forecast_overlap(normalized_rows)

        with self.connection:
            self.connection.execute("DELETE FROM manual_edit WHERE asset_id = ?", (asset_id,))
            if normalized_rows:
                self.connection.executemany(
                    """
                    INSERT INTO manual_edit (
                        asset_id,
                        t0_step,
                        t1_step,
                        shift_lat,
                        shift_lon,
                        rotation_deg,
                        multiplication_factor,
                        editor,
                        reason,
                        metadata_json
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    [
                        (
                            row["asset_id"],
                            row["t0_step"],
                            row["t1_step"],
                            row["shift_lat"],
                            row["shift_lon"],
                            row["rotation_deg"],
                            row["multiplication_factor"],
                            row["editor"],
                            row["reason"],
                            row["metadata_json"],
                        )
                        for row in normalized_rows
                    ],
                )

        return self.list_forecast_manual_edits(asset_id)


def serialize_metadata_payload(value: Any) -> dict[str, Any]:
    if value is None:
        return {}
    if is_dataclass(value):
        return asdict(value)
    if isinstance(value, dict):
        return value
    raise TypeError(f"Unsupported metadata payload type: {type(value)!r}")
