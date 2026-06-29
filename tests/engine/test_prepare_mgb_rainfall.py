from __future__ import annotations

from datetime import datetime

import numpy as np
import pandas as pd
import pytest

from mgb_ops.assets.forecast_grid import write_forecast_precipitation_grid
from mgb_ops.model.prepare_mgb_rainfall import (
    extend_station_matrix_with_forecast,
    load_required_forecast_precipitation_grid,
    prepare_mgb_rainfall,
)


PARHIG_TEMPLATE = """\
ARQUIVO DE INFORMACOES GERAIS PARA O MODELO DE GRANDES BACIAS
!
Projeto Teste
!
       DIA       MES       ANO      HORA          !INICIO DA SIMULACAO
        09       03       2026        00

        NT        DT       !NUMERO DE INTERVALOS DE TEMPO E TAMANHO DO INTERVALO EM SEGUNDOS
       121     3600.

        NC        NU        NB      NCLI     !NUMERO DE CELULAS, USOS, BACIAS E POSTOS CLIMA
         2         1         1         1
"""


MINI_TEMPLATE = """\
Mini Xcen Ycen
1 -51.5 -29.5
2 -52.5 -30.5
"""


PARHIG_3H_TEMPLATE = """\
ARQUIVO DE INFORMACOES GERAIS PARA O MODELO DE GRANDES BACIAS
!
Projeto Teste
!
       DIA       MES       ANO      HORA          !INICIO DA SIMULACAO
        09       03       2026        00

        NT        DT       !NUMERO DE INTERVALOS DE TEMPO E TAMANHO DO INTERVALO EM SEGUNDOS
        41     10800.

        NC        NU        NB      NCLI     !NUMERO DE CELULAS, USOS, BACIAS E POSTOS CLIMA
         2         1         1         1
"""


def test_extend_station_matrix_with_forecast_zeroes_future_block() -> None:
    station_matrix = np.array([[1.0, 2.0, 3.0, 4.0]], dtype=np.float64)

    extended = extend_station_matrix_with_forecast(
        station_matrix,
        total_nt=4,
        forecast_nt=2,
        use_forecast_data=False,
    )

    assert extended.tolist() == [[1.0, 2.0, 0.0, 0.0]]


def test_extend_station_matrix_with_forecast_rejects_unimplemented_forecast() -> None:
    station_matrix = np.array([[1.0, 2.0, 3.0, 4.0]], dtype=np.float64)

    with pytest.raises(NotImplementedError, match="Forecast rainfall ingestion is not implemented yet"):
        extend_station_matrix_with_forecast(
            station_matrix,
            total_nt=4,
            forecast_nt=2,
            use_forecast_data=True,
        )


def test_prepare_mgb_rainfall_zeroes_forecast_period(tmp_path, monkeypatch) -> None:
    history_db = tmp_path / "history.sqlite"
    parhig_path = tmp_path / "PARHIG.hig"
    mini_gtp_path = tmp_path / "MINI.gtp"
    output_path = tmp_path / "CHUVABIN.hig"
    parhig_path.write_text(PARHIG_TEMPLATE, encoding="latin-1")
    mini_gtp_path.write_text(MINI_TEMPLATE, encoding="latin-1")
    history_db.write_bytes(b"sqlite placeholder")

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall.build_execution_id", lambda: "20260311T230000")

    class FakeConnection:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return None

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall._connect_history_read_only", lambda _: FakeConnection())
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_preferred_rain_stations",
        lambda _: pd.DataFrame(
            {
                "series_id": ["s1"],
                "station_id": [1],
                "state": ["raw"],
                "created_at": [""],
                "lat": [-29.5],
                "lon": [-51.5],
            }
        ),
    )
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_rain_values",
        lambda *args, **kwargs: pd.DataFrame(
            {
                "station_id": [1, 1, 1],
                "observed_at": [
                    "2026-03-09 00:00",
                    "2026-03-10 00:00",
                    "2026-03-11 23:00",
                ],
                "value": [1.0, 2.0, 3.0],
            }
        ),
    )

    captured: dict[str, np.ndarray] = {}

    def fake_write_mini_rainfall_atomic(output_path, *, mini_matrix, chunk_hours):
        captured["matrix"] = mini_matrix.copy()
        np.asarray(mini_matrix, dtype=np.float32).tofile(output_path)

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall.write_mini_rainfall_atomic", fake_write_mini_rainfall_atomic)

    summary = prepare_mgb_rainfall(
        history_db=history_db,
        parhig_path=parhig_path,
        mini_gtp_path=mini_gtp_path,
        output_path=output_path,
        reference_time=datetime(2026, 3, 11, 23, 0, 0),
        input_days_before=2,
        forecast_horizon_days=2,
        use_forecast_data=False,
        nearest_stations=1,
        power=2.0,
        chunk_hours=24,
        logs_dir=tmp_path / "logs",
    )

    matrix = captured["matrix"]
    assert summary.nt == 121
    assert summary.forecast_hours == 49
    assert matrix.shape == (2, 121)
    assert matrix[0, 71] == 3.0
    assert matrix[1, 71] == pytest.approx(3.0)
    assert np.allclose(matrix[0, 72:], 0.0)
    assert np.allclose(matrix[1, 72:], 0.0)
    assert output_path.exists()


def test_prepare_mgb_rainfall_loads_ecmwf_forecast_asset(tmp_path, monkeypatch) -> None:
    history_db = tmp_path / "history.sqlite"
    parhig_path = tmp_path / "PARHIG.hig"
    mini_gtp_path = tmp_path / "MINI.gtp"
    output_path = tmp_path / "CHUVABIN.hig"
    forecast_netcdf = tmp_path / "forecast.nc"
    parhig_path.write_text(PARHIG_TEMPLATE, encoding="latin-1")
    mini_gtp_path.write_text(MINI_TEMPLATE, encoding="latin-1")
    history_db.write_bytes(b"sqlite placeholder")
    forecast_netcdf.write_bytes(b"netcdf placeholder")

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall.build_execution_id", lambda: "20260311T230000")

    class FakeConnection:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return None

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall._connect_history_read_only", lambda _: FakeConnection())
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_preferred_rain_stations",
        lambda _: pd.DataFrame(
            {
                "series_id": ["s1"],
                "station_id": [1],
                "state": ["raw"],
                "created_at": [""],
                "lat": [-29.5],
                "lon": [-51.5],
            }
        ),
    )
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_rain_values",
        lambda *args, **kwargs: pd.DataFrame(
            {
                "station_id": [1, 1, 1],
                "observed_at": [
                    "2026-03-09 00:00",
                    "2026-03-10 00:00",
                    "2026-03-11 23:00",
                ],
                "value": [1.0, 2.0, 3.0],
            }
        ),
    )
    forecast_nt = 49
    hourly_grids = np.zeros((forecast_nt, 2, 2), dtype=np.float64)
    hourly_grids[:, 0, 0] = 10.0
    hourly_grids[:, 1, 1] = 20.0
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_required_forecast_precipitation_grid",
        lambda *args, **kwargs: (
            np.array([-29.5, -30.5], dtype=np.float64),
            np.array([-51.5, -52.5], dtype=np.float64),
            hourly_grids,
        ),
    )

    captured: dict[str, np.ndarray] = {}

    def fake_write_mini_rainfall_atomic(output_path, *, mini_matrix, chunk_hours):
        captured["matrix"] = mini_matrix.copy()
        np.asarray(mini_matrix, dtype=np.float32).tofile(output_path)

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall.write_mini_rainfall_atomic", fake_write_mini_rainfall_atomic)

    summary = prepare_mgb_rainfall(
        history_db=history_db,
        parhig_path=parhig_path,
        mini_gtp_path=mini_gtp_path,
        output_path=output_path,
        reference_time=datetime(2026, 3, 11, 23, 0, 0),
        input_days_before=2,
        forecast_horizon_days=2,
        use_forecast_data=True,
        forecast_asset_path=forecast_netcdf,
        nearest_stations=1,
        power=2.0,
        chunk_hours=24,
        logs_dir=tmp_path / "logs",
    )

    matrix = captured["matrix"]
    assert summary.nt == 121
    assert summary.forecast_hours == forecast_nt
    assert matrix.shape == (2, 121)
    assert matrix[0, 72] == 10.0
    assert matrix[1, 72] == 20.0
    assert output_path.exists()


def test_prepare_mgb_rainfall_uses_configured_timestep(tmp_path, monkeypatch) -> None:
    history_db = tmp_path / "history.sqlite"
    parhig_path = tmp_path / "PARHIG.hig"
    mini_gtp_path = tmp_path / "MINI.gtp"
    output_path = tmp_path / "CHUVABIN.hig"
    parhig_path.write_text(PARHIG_3H_TEMPLATE, encoding="latin-1")
    mini_gtp_path.write_text(MINI_TEMPLATE, encoding="latin-1")
    history_db.write_bytes(b"sqlite placeholder")

    class FakeConnection:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return None

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall._connect_history_read_only", lambda _: FakeConnection())
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_preferred_rain_stations",
        lambda _: pd.DataFrame(
            {
                "series_id": ["s1"],
                "station_id": [1],
                "state": ["raw"],
                "created_at": [""],
                "lat": [-29.5],
                "lon": [-51.5],
            }
        ),
    )
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_rain_values",
        lambda *args, **kwargs: pd.DataFrame(
            {
                "station_id": [1, 1],
                "observed_at": ["2026-03-09 00:00", "2026-03-11 21:00"],
                "value": [1.0, 3.0],
            }
        ),
    )

    captured: dict[str, np.ndarray] = {}

    def fake_write_mini_rainfall_atomic(output_path, *, mini_matrix, chunk_hours):
        captured["matrix"] = mini_matrix.copy()
        np.asarray(mini_matrix, dtype=np.float32).tofile(output_path)

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall.write_mini_rainfall_atomic", fake_write_mini_rainfall_atomic)

    summary = prepare_mgb_rainfall(
        history_db=history_db,
        parhig_path=parhig_path,
        mini_gtp_path=mini_gtp_path,
        output_path=output_path,
        reference_time=datetime(2026, 3, 11, 21, 0, 0),
        input_days_before=2,
        forecast_horizon_days=2,
        use_forecast_data=False,
        nearest_stations=1,
        power=2.0,
        timestep_hours=3,
    )

    matrix = captured["matrix"]
    assert summary.nt == 41
    assert summary.forecast_hours == 17
    assert matrix.shape == (2, 41)
    assert matrix[0, 0] == 1.0
    assert matrix[0, 23] == 3.0
    assert np.allclose(matrix[:, 24:], 0.0)


def test_prepare_mgb_rainfall_rejects_off_grid_db_values(tmp_path, monkeypatch) -> None:
    history_db = tmp_path / "history.sqlite"
    parhig_path = tmp_path / "PARHIG.hig"
    mini_gtp_path = tmp_path / "MINI.gtp"
    output_path = tmp_path / "CHUVABIN.hig"
    parhig_path.write_text(PARHIG_TEMPLATE, encoding="latin-1")
    mini_gtp_path.write_text(MINI_TEMPLATE, encoding="latin-1")
    history_db.write_bytes(b"sqlite placeholder")

    class FakeConnection:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return None

    monkeypatch.setattr("mgb_ops.model.prepare_mgb_rainfall._connect_history_read_only", lambda _: FakeConnection())
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_preferred_rain_stations",
        lambda _: pd.DataFrame(
            {
                "series_id": ["s1"],
                "station_id": [1],
                "state": ["raw"],
                "created_at": [""],
                "lat": [-29.5],
                "lon": [-51.5],
            }
        ),
    )
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.load_rain_values",
        lambda *args, **kwargs: pd.DataFrame(
            {
                "station_id": [1],
                "observed_at": ["2026-03-09 00:30"],
                "value": [1.0],
            }
        ),
    )

    with pytest.raises(ValueError, match="already be normalized"):
        prepare_mgb_rainfall(
            history_db=history_db,
            parhig_path=parhig_path,
            mini_gtp_path=mini_gtp_path,
            output_path=output_path,
            reference_time=datetime(2026, 3, 11, 23, 0, 0),
            input_days_before=2,
            forecast_horizon_days=2,
            use_forecast_data=False,
            nearest_stations=1,
            power=2.0,
        )


def test_load_required_forecast_precipitation_grid_matches_local_window_to_utc_netcdf(tmp_path) -> None:
    netcdf_path = tmp_path / "forecast.nc"
    write_forecast_precipitation_grid(
        netcdf_path,
        times_utc=[datetime(2026, 3, 18, 3, 0, 0)],
        latitudes=np.array([-29.5], dtype=np.float64),
        longitudes=np.array([-51.5], dtype=np.float64),
        precipitation_mm=np.array([[[1.0]]], dtype=np.float64),
        provider_code="ecmwf",
        source_format="GRIB2",
        source_cycle_time=datetime(2026, 3, 18, 0, 0, 0),
    )

    latitudes, longitudes, hourly_grids = load_required_forecast_precipitation_grid(
        netcdf_path,
        forecast_start_time=datetime(2026, 3, 18, 0, 0, 0),
        forecast_nt=1,
    )

    assert latitudes.tolist() == [-29.5]
    assert longitudes.tolist() == [-51.5]
    assert hourly_grids.shape == (1, 1, 1)
    assert hourly_grids[0, 0, 0] == pytest.approx(1.0)
