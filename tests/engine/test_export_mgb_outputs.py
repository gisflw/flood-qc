from __future__ import annotations

import sqlite3
from datetime import datetime
from pathlib import Path

import numpy as np
import pytest

from mgb_ops.model.export_mgb_outputs import export_mgb_outputs


REPO_ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = REPO_ROOT / "src" / "mgb_ops" / "assets" / "sql" / "model_outputs_schema.sql"


def configure_export_logging(tmp_path: Path, monkeypatch) -> Path:
    monkeypatch.setattr("mgb_ops.model.export_mgb_outputs.build_execution_id", lambda: "20260101T120000")
    return tmp_path / "logs" / "export_mgb_outputs" / "20260101T120000.log"


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
        ),
        encoding="latin-1",
    )


def write_mini(path: Path, mini_ids: list[int]) -> None:
    lines = ["CatID Mini"]
    for index, mini_id in enumerate(mini_ids, start=1):
        lines.append(f"{index} {mini_id}")
    path.write_text("\n".join(lines) + "\n", encoding="latin-1")


def write_output(path: Path, values: np.ndarray) -> None:
    np.asarray(values, dtype=np.float32).tofile(path)


def build_dataset(
    tmp_path: Path,
    *,
    mini_ids: list[int] | None = None,
    total_nt: int = 1440,
    y_total_nt: int | None = None,
) -> dict[str, Path | list[int] | datetime | int]:
    input_dir = tmp_path / "input"
    output_dir = tmp_path / "output"
    input_dir.mkdir(parents=True, exist_ok=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    start_time = datetime(2026, 1, 1, 0, 0, 0)
    mini_values = mini_ids or [101, 202]
    nc = len(mini_values)
    y_total_nt = total_nt if y_total_nt is None else y_total_nt

    write_parhig(input_dir / "PARHIG.hig", start_time=start_time, nc=nc)
    write_mini(input_dir / "MINI.gtp", mini_values)

    q_values = np.arange(nc * total_nt, dtype=np.float32).reshape(total_nt, nc)
    y_values = (200000 + np.arange(nc * y_total_nt, dtype=np.float32)).reshape(y_total_nt, nc)

    write_output(output_dir / "QTUDO_Inercial_Atual.MGB", q_values)
    write_output(output_dir / "YTUDO.MGB", y_values)

    return {
        "parhig_path": input_dir / "PARHIG.hig",
        "mini_gtp_path": input_dir / "MINI.gtp",
        "output_dir": output_dir,
        "start_time": start_time,
        "mini_ids": mini_values,
        "total_nt": total_nt,
    }

def test_export_mgb_outputs_creates_expected_sqlite(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path)
    output_db_path = tmp_path / "data" / "interim" / "model_outputs.sqlite"
    output_db_path.parent.mkdir(parents=True, exist_ok=True)
    log_file = configure_export_logging(tmp_path, monkeypatch)

    connection = sqlite3.connect(output_db_path)
    try:
        connection.execute("CREATE TABLE obsolete (value INTEGER)")
        connection.execute("INSERT INTO obsolete (value) VALUES (1)")
        connection.commit()
    finally:
        connection.close()

    summary = export_mgb_outputs(
        reference_time=datetime(2026, 2, 9, 23, 0, 0),
        parhig_path=dataset["parhig_path"],
        mini_gtp_path=dataset["mini_gtp_path"],
        output_dir=dataset["output_dir"],
        output_db_path=output_db_path,
        schema_path=SCHEMA_PATH,
        logs_dir=tmp_path / "logs",
        output_days_before=30,
        forecast_horizon_days=15,
        chunk_hours=24,
    )

    assert summary.database_path == output_db_path
    assert summary.reference_time == datetime(2026, 2, 9, 23, 0, 0)
    assert summary.window_start == datetime(2026, 1, 10, 0, 0, 0)
    assert summary.window_end_exclusive == datetime(2026, 2, 25, 0, 0, 0)
    assert summary.nt_current == 960
    assert summary.nt_forecast == 480
    assert summary.series_count == 8
    assert summary.value_count == 4416
    assert log_file.exists()

    log_text = log_file.read_text(encoding="utf-8")
    assert "export_start" in log_text
    assert "chunk_written variable=q prev=sim" in log_text
    assert "chunk_written variable=q prev=for" in log_text
    assert "nt_resolved nt_total=1440 nt_current=960 nt_forecast=480" in log_text
    assert "export_done" in log_text

    with sqlite3.connect(output_db_path) as connection:
        tables = {
            row[0]
            for row in connection.execute("SELECT name FROM sqlite_master WHERE type = 'table'").fetchall()
        }
        assert "obsolete" not in tables
        assert {"metadata", "variable", "output_series", "output_value"}.issubset(tables)

        metadata_row = connection.execute(
            "SELECT reference_time, reference_date, window_start, window_end_exclusive, dt_seconds, nc, nt_current, nt_forecast FROM metadata"
        ).fetchone()
        assert metadata_row == (
            "2026-02-09T23:00:00",
            "2026-02-09",
            "2026-01-10T00:00:00",
            "2026-02-25T00:00:00",
            3600,
            2,
            960,
            480,
        )

        series_rows = connection.execute(
            "SELECT series_id, variable_code, mini_id, prev_flag "
            "FROM output_series ORDER BY variable_code, mini_id, prev_flag"
        ).fetchall()
        assert ("0101.q.sim", "q", 101, 0) in series_rows
        assert ("0101.q.for", "q", 101, 1) in series_rows

        output_series_count = connection.execute("SELECT COUNT(*) FROM output_series").fetchone()[0]
        output_value_count = connection.execute("SELECT COUNT(*) FROM output_value").fetchone()[0]
        assert output_series_count == 8
        assert output_value_count == 4416

        current_dt_bounds = connection.execute(
            "SELECT MIN(v.dt), MAX(v.dt) "
            "FROM output_value v "
            "JOIN output_series s ON s.series_id = v.series_id "
            "WHERE s.variable_code = 'q' AND s.mini_id = 101 AND s.prev_flag = 0"
        ).fetchone()
        assert current_dt_bounds == ("2026-01-10T00:00:00", "2026-02-09T23:00:00")

        forecast_dt_bounds = connection.execute(
            "SELECT MIN(v.dt), MAX(v.dt) "
            "FROM output_value v "
            "JOIN output_series s ON s.series_id = v.series_id "
            "WHERE s.variable_code = 'q' AND s.mini_id = 101 AND s.prev_flag = 1"
        ).fetchone()
        assert forecast_dt_bounds == ("2026-02-10T00:00:00", "2026-02-24T23:00:00")

        current_first_value = connection.execute(
            "SELECT v.value "
            "FROM output_value v "
            "JOIN output_series s ON s.series_id = v.series_id "
            "WHERE s.series_id = '0101.q.sim' "
            "ORDER BY v.dt LIMIT 1"
        ).fetchone()[0]
        forecast_first_value = connection.execute(
            "SELECT v.value "
            "FROM output_value v "
            "JOIN output_series s ON s.series_id = v.series_id "
            "WHERE s.series_id = '0101.q.for' "
            "ORDER BY v.dt LIMIT 1"
        ).fetchone()[0]
        assert current_first_value == 432.0
        assert forecast_first_value == 1920.0


def test_export_mgb_outputs_uses_explicit_reference_time(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path)
    configure_export_logging(tmp_path, monkeypatch)

    summary = export_mgb_outputs(
        reference_time=datetime(2026, 2, 9, 23, 0, 0),
        parhig_path=dataset["parhig_path"],
        mini_gtp_path=dataset["mini_gtp_path"],
        output_dir=dataset["output_dir"],
        output_db_path=tmp_path / "model_outputs.sqlite",
        schema_path=SCHEMA_PATH,
        logs_dir=tmp_path / "logs",
        output_days_before=30,
        forecast_horizon_days=15,
    )

    assert summary.reference_time == datetime(2026, 2, 9, 23, 0, 0)
    assert summary.nt_current == 960
    assert summary.nt_forecast == 480


def test_build_output_series_id_zero_pads_mini_id() -> None:
    from mgb_ops.model.export_mgb_outputs import build_output_series_id

    assert build_output_series_id(539, "q", 0) == "0539.q.sim"
    assert build_output_series_id(539, "q", 1) == "0539.q.for"

def test_export_mgb_outputs_requires_single_source_file(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path)
    configure_export_logging(tmp_path, monkeypatch)
    Path(dataset["output_dir"]).joinpath("YTUDO.MGB").unlink()

    with pytest.raises(FileNotFoundError, match="YTUDO"):
        export_mgb_outputs(
            reference_time=datetime(2026, 2, 9, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            output_dir=dataset["output_dir"],
            output_db_path=tmp_path / "model_outputs.sqlite",
            schema_path=SCHEMA_PATH,
            output_days_before=30,
            forecast_horizon_days=15,
        )


def test_export_mgb_outputs_rejects_duplicate_mini_ids(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, mini_ids=[101, 101])
    configure_export_logging(tmp_path, monkeypatch)

    with pytest.raises(ValueError, match="duplicated Mini ids"):
        export_mgb_outputs(
            reference_time=datetime(2026, 2, 9, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            output_dir=dataset["output_dir"],
            output_db_path=tmp_path / "model_outputs.sqlite",
            schema_path=SCHEMA_PATH,
            output_days_before=30,
            forecast_horizon_days=15,
        )


def test_export_mgb_outputs_rejects_inconsistent_nt_between_variables(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, total_nt=1440, y_total_nt=120)
    configure_export_logging(tmp_path, monkeypatch)

    with pytest.raises(ValueError, match="Inconsistent NT across outputs"):
        export_mgb_outputs(
            reference_time=datetime(2026, 2, 9, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            output_dir=dataset["output_dir"],
            output_db_path=tmp_path / "model_outputs.sqlite",
            schema_path=SCHEMA_PATH,
            output_days_before=30,
            forecast_horizon_days=15,
        )


def test_export_mgb_outputs_allows_cutoff_at_last_available_timestamp(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, total_nt=48)
    configure_export_logging(tmp_path, monkeypatch)
    output_db_path = tmp_path / "model_outputs.sqlite"

    summary = export_mgb_outputs(
        reference_time=datetime(2026, 1, 2, 23, 0, 0),
        parhig_path=dataset["parhig_path"],
        mini_gtp_path=dataset["mini_gtp_path"],
        output_dir=dataset["output_dir"],
        output_db_path=output_db_path,
        schema_path=SCHEMA_PATH,
        logs_dir=tmp_path / "logs",
        output_days_before=1,
        forecast_horizon_days=0,
    )

    assert summary.nt_current == 48
    assert summary.nt_forecast == 0

    with sqlite3.connect(output_db_path) as connection:
        forecast_rows = connection.execute(
            "SELECT COUNT(*) FROM output_value v "
            "JOIN output_series s ON s.series_id = v.series_id "
            "WHERE s.prev_flag = 1"
        ).fetchone()[0]
        assert forecast_rows == 0

def test_export_mgb_outputs_rejects_cutoff_before_available_range(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, total_nt=48)
    configure_export_logging(tmp_path, monkeypatch)

    with pytest.raises(ValueError, match="before the available output start"):
        export_mgb_outputs(
            reference_time=datetime(2025, 12, 31, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            output_dir=dataset["output_dir"],
            output_db_path=tmp_path / "model_outputs.sqlite",
            schema_path=SCHEMA_PATH,
            output_days_before=1,
            forecast_horizon_days=0,
        )


def test_export_mgb_outputs_rejects_cutoff_after_available_range(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, total_nt=48)
    configure_export_logging(tmp_path, monkeypatch)

    with pytest.raises(ValueError, match="exceeds the available output end"):
        export_mgb_outputs(
            reference_time=datetime(2026, 1, 3, 0, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            output_dir=dataset["output_dir"],
            output_db_path=tmp_path / "model_outputs.sqlite",
            schema_path=SCHEMA_PATH,
            output_days_before=1,
            forecast_horizon_days=0,
        )
