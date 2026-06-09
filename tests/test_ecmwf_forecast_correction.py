from __future__ import annotations

from datetime import datetime
from pathlib import Path

import numpy as np

from mgb_ops.ingest.forecast_grid import TpGribMessage
from mgb_ops.qc import ecmwf_forecast_correction


def _message(step_hours: int, value: float) -> TpGribMessage:
    return TpGribMessage(
        valid_time=datetime(2026, 3, 11, step_hours, 0, 0),
        step_hours=step_hours,
        latitudes=np.array([-29.5, -30.5], dtype=np.float64),
        longitudes=np.array([-51.5, -50.5], dtype=np.float64),
        values_mm=np.full((2, 2), value, dtype=np.float64),
    )


def test_build_corrected_cumulative_fields_applies_instruction_to_matching_steps(monkeypatch) -> None:
    messages = [_message(0, 0.0), _message(3, 1.0), _message(6, 3.0)]
    monkeypatch.setattr(ecmwf_forecast_correction, "read_tp_grib_messages", lambda _: messages)

    corrected = ecmwf_forecast_correction.build_corrected_cumulative_fields(
        Path("unused.grib2"),
        [
            ecmwf_forecast_correction.ForecastCorrectionInstruction(
                asset_id="asset",
                t0_step=0,
                t1_step=3,
                multiplication_factor=2.0,
            )
        ],
    )

    assert np.allclose(corrected[0], 0.0)
    assert np.allclose(corrected[1], 2.0)
    assert np.allclose(corrected[2], 4.0)


def test_write_corrected_grib2_sets_corrected_tp_values(tmp_path, monkeypatch) -> None:
    source_path = tmp_path / "source.grib2"
    target_path = tmp_path / "corrected.grib2"
    source_path.write_bytes(b"source")

    monkeypatch.setattr(
        ecmwf_forecast_correction,
        "build_corrected_cumulative_fields",
        lambda *args, **kwargs: [np.array([[1000.0]]), np.array([[3000.0]])],
    )

    class FakeEccodes:
        def __init__(self) -> None:
            self._gids = [{"shortName": "tp"}, {"shortName": "tp"}]
            self._idx = 0
            self.set_arrays: list[list[float]] = []

        def codes_grib_new_from_file(self, handle):
            if self._idx >= len(self._gids):
                return None
            gid = self._gids[self._idx]
            self._idx += 1
            return gid

        @staticmethod
        def codes_get(gid, key):
            return gid[key]

        def codes_set_array(self, gid, key, values):
            self.set_arrays.append(list(values))

        @staticmethod
        def codes_write(gid, handle):
            handle.write(b"g")

        @staticmethod
        def codes_release(gid):
            return None

    fake_eccodes = FakeEccodes()
    monkeypatch.setattr(ecmwf_forecast_correction, "_require_eccodes", lambda: fake_eccodes)

    summary = ecmwf_forecast_correction.write_corrected_grib2(
        source_path,
        target_path,
        [
            ecmwf_forecast_correction.ForecastCorrectionInstruction(
                asset_id="asset",
                t0_step=0,
                t1_step=3,
            )
        ],
    )

    assert summary.corrected_step_count == 2
    assert target_path.exists()
    assert fake_eccodes.set_arrays == [[1.0], [3.0]]
