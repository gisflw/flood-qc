from __future__ import annotations

from datetime import datetime
from pathlib import Path

import numpy as np
import pytest
import xarray as xr
from netCDF4 import Dataset

from mgb_ops.assets.model_outputs import NETCDF_ZLIB_COMPLEVEL
from mgb_ops.model.export_mgb_outputs import export_mgb_outputs


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
    y_values = (2000 + np.arange(nc * y_total_nt, dtype=np.float32)).reshape(y_total_nt, nc)
    precipitation_values = (
        np.arange(nc * total_nt, dtype=np.float32).reshape(total_nt, nc) / 10
    )

    write_output(output_dir / "QTUDO_Inercial_Atual.MGB", q_values)
    write_output(output_dir / "YTUDO.MGB", y_values)
    write_output(input_dir / "CHUVABIN.hig", precipitation_values)

    return {
        "parhig_path": input_dir / "PARHIG.hig",
        "mini_gtp_path": input_dir / "MINI.gtp",
        "chuvabin_path": input_dir / "CHUVABIN.hig",
        "output_dir": output_dir,
        "start_time": start_time,
        "mini_ids": mini_values,
        "total_nt": total_nt,
    }


def test_export_mgb_outputs_creates_expected_netcdf(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path)
    output_nc_path = tmp_path / "data" / "processed" / "model_outputs.nc"
    output_nc_path.parent.mkdir(parents=True, exist_ok=True)
    output_nc_path.write_bytes(b"stale netcdf placeholder")
    log_file = configure_export_logging(tmp_path, monkeypatch)

    summary = export_mgb_outputs(
        reference_time=datetime(2026, 2, 9, 23, 0, 0),
        parhig_path=dataset["parhig_path"],
        mini_gtp_path=dataset["mini_gtp_path"],
        chuvabin_path=dataset["chuvabin_path"],
        output_dir=dataset["output_dir"],
        output_nc_path=output_nc_path,
        logs_dir=tmp_path / "logs",
        output_days_before=30,
        forecast_horizon_days=15,
        chunk_hours=24,
        scenario_metadata={
            "scenario_id": "raw:asset",
            "scenario_kind": "raw",
            "scenario_label": "Raw asset",
        },
    )

    assert summary.netcdf_path == output_nc_path
    assert summary.reference_time == datetime(2026, 2, 9, 23, 0, 0)
    assert summary.window_start == datetime(2026, 1, 10, 0, 0, 0)
    assert summary.window_end_exclusive == datetime(2026, 2, 25, 0, 0, 0)
    assert summary.nt_current == 960
    assert summary.nt_forecast == 480
    assert summary.variable_count == 3
    assert summary.value_count == 6624
    assert log_file.exists()

    log_text = log_file.read_text(encoding="utf-8")
    assert "export_start" in log_text
    assert "chunk_written variable=flow" in log_text
    assert "nt_resolved nt_total=1440 nt_current=960 nt_forecast=480" in log_text
    assert "netcdf_finalized" in log_text
    assert "export_done" in log_text

    with xr.open_dataset(output_nc_path) as exported:
        assert exported.sizes == {"time": 1104, "mini": 2}
        assert set(exported.data_vars) == {"precipitation", "level", "flow", "time_segment"}
        assert exported.attrs["Conventions"] == "CF-1.11 ACDD-1.3"
        assert exported.attrs["reference_time"] == "2026-02-09T23:00:00"
        assert exported.attrs["reference_date"] == "2026-02-09"
        assert exported.attrs["window_start"] == "2026-01-10T00:00:00"
        assert exported.attrs["scenario_id"] == "raw:asset"
        assert exported.attrs["scenario_kind"] == "raw"
        assert exported.attrs["window_end_exclusive"] == "2026-02-25T00:00:00"
        assert exported.attrs["mgb_start_time"] == "2026-01-01T00:00:00"
        assert exported.attrs["dt_seconds"] == 3600
        assert exported.attrs["nt_current"] == 960
        assert exported.attrs["nt_forecast"] == 480
        assert exported.attrs["package_name"] == "mgb-ops"

        np.testing.assert_array_equal(exported["mini_id"].values, np.array([101, 202], dtype=np.int32))
        assert exported["time"].values[0] == np.datetime64("2026-01-10T00:00:00")
        assert exported["time"].values[-1] == np.datetime64("2026-02-24T23:00:00")
        assert exported["time_segment"].attrs["flag_meanings"] == "current_simulation forecast"
        assert exported["time_segment"].values[0] == 0
        assert exported["time_segment"].values[743] == 0
        assert exported["time_segment"].values[744] == 1
        assert exported["time_segment"].values[-1] == 1

        assert exported["flow"].attrs["long_name"] == "MGB river discharge"
        assert exported["flow"].attrs["standard_name"] == "water_volume_transport_in_river_channel"
        assert exported["flow"].attrs["units"] == "m3 s-1"
        assert exported["flow"].attrs["source_filename"] == "QTUDO_Inercial_Atual.MGB"
        assert exported["level"].attrs["long_name"] == "MGB river stage"
        assert exported["level"].attrs["units"] == "cm"
        assert exported["level"].attrs["source_filename"] == "YTUDO.MGB"
        assert exported["precipitation"].attrs["units"] == "mm"
        assert exported["precipitation"].attrs["source_filename"] == "CHUVABIN.hig"

        assert float(exported["flow"].isel(time=0, mini=0)) == 432.0
        assert float(exported["flow"].isel(time=744, mini=0)) == 1920.0
        assert float(exported["level"].isel(time=0, mini=0)) == 243200.0
        assert float(exported["level"].isel(time=744, mini=0)) == 392000.0
        assert float(exported["precipitation"].isel(time=0, mini=0)) == 43.2
        assert float(exported["precipitation"].isel(time=744, mini=0)) == 192.0

    with Dataset(output_nc_path) as exported:
        for code in ("precipitation", "level", "flow"):
            variable = exported.variables[code]
            variable.set_auto_maskandscale(False)
            assert variable.dtype == np.dtype("int32")
            assert variable.scale_factor == 0.001
            assert variable.filters()["zlib"] is True
            assert variable.filters()["complevel"] == NETCDF_ZLIB_COMPLEVEL
        assert exported.variables["precipitation"][0, 0] == 43200


def test_export_mgb_outputs_uses_explicit_reference_time(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path)
    configure_export_logging(tmp_path, monkeypatch)

    summary = export_mgb_outputs(
        reference_time=datetime(2026, 2, 9, 23, 0, 0),
        parhig_path=dataset["parhig_path"],
        mini_gtp_path=dataset["mini_gtp_path"],
        chuvabin_path=dataset["chuvabin_path"],
        output_dir=dataset["output_dir"],
        output_nc_path=tmp_path / "model_outputs.nc",
        logs_dir=tmp_path / "logs",
        output_days_before=30,
        forecast_horizon_days=15,
    )

    assert summary.reference_time == datetime(2026, 2, 9, 23, 0, 0)
    assert summary.nt_current == 960
    assert summary.nt_forecast == 480


def test_export_mgb_outputs_requires_single_source_file(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path)
    configure_export_logging(tmp_path, monkeypatch)
    Path(dataset["output_dir"]).joinpath("YTUDO.MGB").unlink()

    with pytest.raises(FileNotFoundError, match="YTUDO"):
        export_mgb_outputs(
            reference_time=datetime(2026, 2, 9, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            chuvabin_path=dataset["chuvabin_path"],
            output_dir=dataset["output_dir"],
            output_nc_path=tmp_path / "model_outputs.nc",
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
            chuvabin_path=dataset["chuvabin_path"],
            output_dir=dataset["output_dir"],
            output_nc_path=tmp_path / "model_outputs.nc",
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
            chuvabin_path=dataset["chuvabin_path"],
            output_dir=dataset["output_dir"],
            output_nc_path=tmp_path / "model_outputs.nc",
            output_days_before=30,
            forecast_horizon_days=15,
        )


def test_export_mgb_outputs_rejects_malformed_chuvabin(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path)
    configure_export_logging(tmp_path, monkeypatch)
    Path(dataset["chuvabin_path"]).write_bytes(b"\x00" * 7)

    with pytest.raises(ValueError, match="not divisible by 4"):
        export_mgb_outputs(
            reference_time=datetime(2026, 2, 9, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            chuvabin_path=dataset["chuvabin_path"],
            output_dir=dataset["output_dir"],
            output_nc_path=tmp_path / "model_outputs.nc",
            output_days_before=30,
            forecast_horizon_days=15,
        )


def test_export_mgb_outputs_rejects_packed_integer_overflow(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, total_nt=48)
    configure_export_logging(tmp_path, monkeypatch)
    values = np.fromfile(dataset["output_dir"] / "QTUDO_Inercial_Atual.MGB", dtype=np.float32)
    values[0] = 3_000_000
    write_output(dataset["output_dir"] / "QTUDO_Inercial_Atual.MGB", values)

    with pytest.raises(OverflowError, match="flow exceeds the int32 packing range"):
        export_mgb_outputs(
            reference_time=datetime(2026, 1, 2, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            chuvabin_path=dataset["chuvabin_path"],
            output_dir=dataset["output_dir"],
            output_nc_path=tmp_path / "model_outputs.nc",
            output_days_before=1,
            forecast_horizon_days=0,
        )


def test_export_mgb_outputs_allows_cutoff_at_last_available_timestamp(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, total_nt=48)
    configure_export_logging(tmp_path, monkeypatch)
    output_nc_path = tmp_path / "model_outputs.nc"

    summary = export_mgb_outputs(
        reference_time=datetime(2026, 1, 2, 23, 0, 0),
        parhig_path=dataset["parhig_path"],
        mini_gtp_path=dataset["mini_gtp_path"],
        chuvabin_path=dataset["chuvabin_path"],
        output_dir=dataset["output_dir"],
        output_nc_path=output_nc_path,
        logs_dir=tmp_path / "logs",
        output_days_before=1,
        forecast_horizon_days=0,
    )

    assert summary.nt_current == 48
    assert summary.nt_forecast == 0

    with xr.open_dataset(output_nc_path) as exported:
        assert exported.sizes == {"time": 48, "mini": 2}
        assert set(exported["time_segment"].values.tolist()) == {0}


def test_export_mgb_outputs_rejects_cutoff_before_available_range(tmp_path, monkeypatch) -> None:
    dataset = build_dataset(tmp_path, total_nt=48)
    configure_export_logging(tmp_path, monkeypatch)

    with pytest.raises(ValueError, match="before the available output start"):
        export_mgb_outputs(
            reference_time=datetime(2025, 12, 31, 23, 0, 0),
            parhig_path=dataset["parhig_path"],
            mini_gtp_path=dataset["mini_gtp_path"],
            chuvabin_path=dataset["chuvabin_path"],
            output_dir=dataset["output_dir"],
            output_nc_path=tmp_path / "model_outputs.nc",
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
            chuvabin_path=dataset["chuvabin_path"],
            output_dir=dataset["output_dir"],
            output_nc_path=tmp_path / "model_outputs.nc",
            output_days_before=1,
            forecast_horizon_days=0,
        )
