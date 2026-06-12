from __future__ import annotations

import sqlite3
from datetime import datetime
from pathlib import Path

import numpy as np
import pandas as pd
import pytest

from apps.ops_dashboard.support import data as ops_dashboard_data
from mgb_ops.storage.db_bootstrap import apply_schema


REPO_ROOT = Path(__file__).resolve().parents[2]
HISTORY_SCHEMA_PATH = REPO_ROOT / "src" / "mgb_ops" / "assets" / "sql" / "history_schema.sql"
MODEL_OUTPUTS_SCHEMA_PATH = REPO_ROOT / "src" / "mgb_ops" / "assets" / "sql" / "model_outputs_schema.sql"


def initialize_history_db(path: Path) -> Path:
    apply_schema(path, HISTORY_SCHEMA_PATH)
    return path


def initialize_model_outputs_db(path: Path) -> Path:
    apply_schema(path, MODEL_OUTPUTS_SCHEMA_PATH)
    return path


def write_parhig(path: Path, *, start_time: datetime, nc: int, dt_seconds: int = 3600) -> None:
    path.write_text(
        "\n".join(
            [
                "ARQUIVO DE INFORMACOES GERAIS PARA O MODELO DE GRANDES BACIAS",
                "!",
                "       DIA       MES       ANO      HORA          !INICIO DA SIMULACAO",
                f"        {start_time.day:02d}       {start_time.month:02d}       {start_time.year:04d}        {start_time.hour:02d}",
                "",
                "        NT        DT       !NUMERO DE INTERVALOS DE TEMPO E TAMANHO DO INTERVALO EM SEGUNDOS",
                f"         1     {dt_seconds}.",
                "",
                "        NC        NU        NB      NCLI     !NUMERO DE CELULAS, USOS, BACIAS E POSTOS CLIMA",
                f"         {nc}         1         1         1",
            ]
        )
        + "\n",
        encoding="latin-1",
    )


def write_mini(path: Path, mini_ids: list[int]) -> None:
    lines = ["CatID Mini"]
    for index, mini_id in enumerate(mini_ids, start=1):
        lines.append(f"{index} {mini_id}")
    path.write_text("\n".join(lines) + "\n", encoding="latin-1")


def write_output(path: Path, values: np.ndarray) -> None:
    np.asarray(values, dtype=np.float32).tofile(path)


def build_mgb_dataset(tmp_path: Path, *, nt_total: int = 72) -> dict[str, object]:
    input_dir = tmp_path / "apps" / "mgb_runner" / "Input"
    output_dir = tmp_path / "apps" / "mgb_runner" / "Output"
    input_dir.mkdir(parents=True, exist_ok=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    start_time = datetime(2026, 2, 1, 0, 0, 0)
    mini_ids = [101, 539]
    nc = len(mini_ids)

    write_parhig(input_dir / "PARHIG.hig", start_time=start_time, nc=nc)
    write_mini(input_dir / "MINI.gtp", mini_ids)

    q_values = np.vstack(
        [
            np.arange(nt_total, dtype=np.float32),
            1000.0 + np.arange(nt_total, dtype=np.float32),
        ]
    )
    y_values = np.vstack(
        [
            2000.0 + np.arange(nt_total, dtype=np.float32),
            3000.0 + np.arange(nt_total, dtype=np.float32),
        ]
    )
    write_output(output_dir / "QTUDO_Inercial_Atual.MGB", q_values)
    write_output(output_dir / "YTUDO.MGB", y_values)

    return {
        "input_dir": input_dir,
        "output_dir": output_dir,
        "parhig_path": input_dir / "PARHIG.hig",
        "mini_gtp_path": input_dir / "MINI.gtp",
        "start_time": start_time,
        "mini_ids": mini_ids,
        "nc": nc,
        "nt_total": nt_total,
    }


def mgb_runtime_kwargs(dataset: dict[str, object], *, reference_time: str) -> dict[str, object]:
    return {
        "parhig_path": Path(dataset["parhig_path"]),
        "mini_gtp_path": Path(dataset["mini_gtp_path"]),
        "output_dir": Path(dataset["output_dir"]),
        "reference_time": datetime.fromisoformat(reference_time),
    }


def mgb_metadata_kwargs(dataset: dict[str, object], *, reference_time: str) -> dict[str, object]:
    return {
        **mgb_runtime_kwargs(dataset, reference_time=reference_time),
        "output_days_before": 30,
        "forecast_horizon_days": 15,
    }


def insert_station(connection: sqlite3.Connection, *, station_uid: int, station_code: str, station_name: str) -> None:
    connection.execute(
        """
        INSERT INTO station (
            station_uid,
            station_code,
            station_name,
            provider_code,
            latitude,
            longitude,
            altitude_m
        ) VALUES (?, ?, ?, 'ana', -29.5, -53.5, 10)
        """,
        (station_uid, station_code, station_name),
    )


def insert_observed_series(
    connection: sqlite3.Connection,
    *,
    series_id: str,
    station_uid: int,
    variable_code: str,
    state: str,
    created_at: str = "2026-03-17 12:00:00",
) -> None:
    connection.execute(
        """
        INSERT INTO observed_series (series_id, station_uid, variable_code, state, created_at)
        VALUES (?, ?, ?, ?, ?)
        """,
        (series_id, station_uid, variable_code, state, created_at),
    )


def insert_observed_value(connection: sqlite3.Connection, *, series_id: str, observed_at: str, value: float | None) -> None:
    connection.execute(
        "INSERT INTO observed_value (series_id, observed_at, value) VALUES (?, ?, ?)",
        (series_id, observed_at, value),
    )


def test_select_preferred_series_rows_uses_state_precedence() -> None:
    series = pd.DataFrame(
        [
            {"series_id": "rain.raw", "station_uid": 1, "variable_code": "rain", "state": "raw", "created_at": "2026-01-01 00:00:00"},
            {"series_id": "rain.curated", "station_uid": 1, "variable_code": "rain", "state": "curated", "created_at": "2026-01-02 00:00:00"},
            {"series_id": "rain.approved", "station_uid": 1, "variable_code": "rain", "state": "approved", "created_at": "2026-01-03 00:00:00"},
            {"series_id": "level.raw", "station_uid": 1, "variable_code": "level", "state": "raw", "created_at": "2026-01-01 00:00:00"},
        ]
    )

    preferred = ops_dashboard_data.select_preferred_series_rows(series)

    assert preferred["series_id"].tolist() == ["level.raw", "rain.approved"]


def test_derive_station_kind_from_variable_coverage() -> None:
    assert ops_dashboard_data.derive_station_kind(["rain"]) == "rain"
    assert ops_dashboard_data.derive_station_kind(["level"]) == "level"
    assert ops_dashboard_data.derive_station_kind(["flow"]) == "level"
    assert ops_dashboard_data.derive_station_kind(["rain", "flow"]) == "mixed"


def test_load_station_catalog_classifies_status_from_observed_values(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    now = datetime(2026, 3, 17, 12, 0, 0)

    with sqlite3.connect(db_path) as connection:
        insert_station(connection, station_uid=1001, station_code="1001", station_name="OK")
        insert_station(connection, station_uid=1002, station_code="1002", station_name="ISSUE")
        insert_station(connection, station_uid=1003, station_code="1003", station_name="NODATA")

        insert_observed_series(connection, series_id="1001.rain.raw", station_uid=1001, variable_code="rain", state="raw")
        insert_observed_series(connection, series_id="1002.rain.raw", station_uid=1002, variable_code="rain", state="raw")
        insert_observed_series(connection, series_id="1003.rain.raw", station_uid=1003, variable_code="rain", state="raw")

        insert_observed_value(connection, series_id="1001.rain.raw", observed_at="2026-03-16 00:00:00", value=5.0)
        insert_observed_value(connection, series_id="1002.rain.raw", observed_at="2026-03-16 00:00:00", value=None)
        insert_observed_value(connection, series_id="1003.rain.raw", observed_at="2026-01-01 00:00:00", value=2.0)
        connection.commit()

    catalog = ops_dashboard_data.load_station_catalog(db_path, days=30, now=now)
    status_by_station = dict(zip(catalog["station_uid"], catalog["status"]))

    assert status_by_station == {
        1001: "ok",
        1002: "data_issue",
        1003: "no_data",
    }
    assert set(catalog.columns).issuperset(
        {"station_uid", "station_code", "provider_code", "station_name", "lat", "lon", "kind", "status", "status_reason"}
    )


def test_load_station_catalog_handles_all_stations_without_recent_values(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    now = datetime(2026, 3, 17, 12, 0, 0)

    with sqlite3.connect(db_path) as connection:
        insert_station(connection, station_uid=1001, station_code="1001", station_name="NODATA")
        insert_observed_series(connection, series_id="1001.rain.raw", station_uid=1001, variable_code="rain", state="raw")
        insert_observed_value(connection, series_id="1001.rain.raw", observed_at="2026-01-01 00:00:00", value=2.0)
        connection.commit()

    catalog = ops_dashboard_data.load_station_catalog(db_path, days=30, now=now)

    assert catalog["station_uid"].tolist() == [1001]
    assert catalog["kind"].tolist() == ["rain"]
    assert catalog["status"].tolist() == ["no_data"]
    assert catalog["rows_recent"].tolist() == [0]


def test_load_observed_series_returns_only_preferred_state_for_station(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    now = datetime(2026, 3, 17, 12, 0, 0)

    with sqlite3.connect(db_path) as connection:
        insert_station(connection, station_uid=1001, station_code="1001", station_name="TESTE")
        insert_observed_series(
            connection,
            series_id="1001.rain.raw",
            station_uid=1001,
            variable_code="rain",
            state="raw",
            created_at="2026-03-10 00:00:00",
        )
        insert_observed_series(
            connection,
            series_id="1001.rain.curated",
            station_uid=1001,
            variable_code="rain",
            state="curated",
            created_at="2026-03-11 00:00:00",
        )
        insert_observed_series(connection, series_id="1001.level.raw", station_uid=1001, variable_code="level", state="raw")

        insert_observed_value(connection, series_id="1001.rain.raw", observed_at="2026-03-16 01:00:00", value=1.0)
        insert_observed_value(connection, series_id="1001.rain.curated", observed_at="2026-03-16 01:00:00", value=2.5)
        insert_observed_value(connection, series_id="1001.level.raw", observed_at="2026-03-16 01:00:00", value=120.0)
        connection.commit()

    observed = ops_dashboard_data.load_observed_series(1001, db_path, days=30, now=now)

    assert observed.to_dict(orient="records") == [
        {"datetime": pd.Timestamp("2026-03-16 01:00:00"), "variable_code": "level", "value": 120.0},
        {"datetime": pd.Timestamp("2026-03-16 01:00:00"), "variable_code": "rain", "value": 2.5},
    ]


def test_load_mgb_series_splits_current_and_forecast(tmp_path, monkeypatch) -> None:
    dataset = build_mgb_dataset(tmp_path, nt_total=72)
    series = ops_dashboard_data.load_mgb_series(
        539,
        "q",
        **mgb_runtime_kwargs(dataset, reference_time="2026-02-02T23:00:00"),
        days_window=1,
    )

    assert series["prev_flag"].tolist() == ([0] * 25) + ([1] * 24)
    assert series["value"].tolist()[0] == 1023.0
    assert series["value"].tolist()[-1] == 1071.0
    assert series["display_name"].tolist()[0] == "QTUDO"
    assert series["unit"].tolist()[0] == "m3/s"
    assert series["dt"].iloc[0] == pd.Timestamp("2026-02-01 23:00:00")
    assert series["dt"].iloc[24] == pd.Timestamp("2026-02-02 23:00:00")
    assert series["dt"].iloc[25] == pd.Timestamp("2026-02-03 00:00:00")


def test_list_model_variables_returns_static_mgb_catalog() -> None:
    variables = ops_dashboard_data.list_model_variables()

    assert variables.to_dict(orient="records") == [
        {"variable_code": "q", "display_name": "QTUDO", "unit": "m3/s"},
        {"variable_code": "y", "display_name": "YTUDO", "unit": "m"},
    ]


def test_load_model_metadata_is_derived_from_parhig_binaries_and_config(tmp_path, monkeypatch) -> None:
    dataset = build_mgb_dataset(tmp_path, nt_total=72)
    metadata = ops_dashboard_data.load_model_metadata(**mgb_metadata_kwargs(dataset, reference_time="2026-02-02T23:00:00"))

    assert metadata["reference_time"] == pd.Timestamp("2026-02-02 23:00:00")
    assert metadata["reference_date"] == pd.Timestamp("2026-02-02")
    assert metadata["window_start"] == pd.Timestamp("2026-01-03 00:00:00")
    assert metadata["window_end_exclusive"] == pd.Timestamp("2026-02-18 00:00:00")
    assert metadata["dt_seconds"] == 3600
    assert metadata["nc"] == 2
    assert metadata["nt_current"] == 48
    assert metadata["nt_forecast"] == 24


def test_build_mgb_mini_index_preserves_mini_row_order(tmp_path) -> None:
    dataset = build_mgb_dataset(tmp_path, nt_total=8)

    index = ops_dashboard_data._build_mgb_mini_index(
        mini_gtp_path=Path(dataset["mini_gtp_path"]),
        nc=int(dataset["nc"]),
    )

    assert index == {101: 0, 539: 1}


def test_load_mgb_series_rejects_unknown_mini_id(tmp_path, monkeypatch) -> None:
    dataset = build_mgb_dataset(tmp_path, nt_total=24)
    with pytest.raises(ValueError, match="Mini 999 was not found"):
        ops_dashboard_data.load_mgb_series(
            999,
            "q",
            **mgb_runtime_kwargs(dataset, reference_time="2026-02-01T23:00:00"),
            days_window=1,
        )


def test_load_model_metadata_rejects_inconsistent_nt_between_variables(tmp_path, monkeypatch) -> None:
    dataset = build_mgb_dataset(tmp_path, nt_total=24)
    write_output(Path(dataset["output_dir"]) / "YTUDO.MGB", np.arange(2 * 12, dtype=np.float32).reshape(2, 12))
    with pytest.raises(ValueError, match="Inconsistent NT across MGB binary outputs"):
        ops_dashboard_data.load_model_metadata(**mgb_metadata_kwargs(dataset, reference_time="2026-02-01T23:00:00"))


def test_load_mgb_series_rejects_binary_incompatible_with_nc(tmp_path, monkeypatch) -> None:
    dataset = build_mgb_dataset(tmp_path, nt_total=24)
    write_output(Path(dataset["output_dir"]) / "QTUDO_Inercial_Atual.MGB", np.arange(25, dtype=np.float32))
    with pytest.raises(ValueError, match="not divisible by NC=2"):
        ops_dashboard_data.load_mgb_series(
            101,
            "q",
            **mgb_runtime_kwargs(dataset, reference_time="2026-02-01T23:00:00"),
            days_window=1,
        )


def test_load_mgb_series_rejects_reference_time_after_available_range(tmp_path, monkeypatch) -> None:
    dataset = build_mgb_dataset(tmp_path, nt_total=24)
    with pytest.raises(ValueError, match="exceeds the available output end"):
        ops_dashboard_data.load_mgb_series(
            101,
            "q",
            **mgb_runtime_kwargs(dataset, reference_time="2026-02-03T00:00:00"),
            days_window=1,
        )


def test_list_accumulation_rasters_catalogs_expected_horizons(tmp_path) -> None:
    (tmp_path / "accum_72h.tif").touch()
    (tmp_path / "accum_24h.tif").touch()
    (tmp_path / "accum_720h.tif").touch()
    (tmp_path / "accum_240h.tif").touch()
    (tmp_path / "other.tif").touch()

    catalog = ops_dashboard_data.list_accumulation_rasters(tmp_path)

    assert [item["horizon_label"] for item in catalog] == ["24h", "72h", "240h", "720h"]
    assert [item["name"] for item in catalog] == ["accum_24h", "accum_72h", "accum_240h", "accum_720h"]
