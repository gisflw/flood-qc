from __future__ import annotations

from datetime import datetime

import pytest

from mgb_ops.storage.db_bootstrap import initialize_history_db
from mgb_ops.storage.history_repository import HistoryRepository


def test_history_repository_upserts_and_finds_ecmwf_asset(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        asset = repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            asset_kind="forecast_grib_rs_buffered",
            format="GRIB2",
            relative_path="data/interim/ecmwf/fc_2026-03-11_00_IFS_rsbuf.grib2",
            provider_code="ecmwf",
            valid_from="2026-03-11T03:00:00",
            valid_to="2026-03-26T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z"},
        )
        same_path = repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            asset_kind="forecast_grib_rs_buffered",
            format="GRIB2",
            relative_path="data/interim/ecmwf/fc_2026-03-11_00_IFS_rsbuf.grib2",
            provider_code="ecmwf",
            valid_from="2026-03-11T03:00:00",
            valid_to="2026-03-27T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z", "bbox": [-72.0, -44.0, -36.0, -17.0]},
        )
        found = repository.find_latest_ecmwf_asset(
            datetime(2026, 3, 11, 12, 0, 0),
            asset_kind="forecast_grib_rs_buffered",
        )
        listed = repository.list_ecmwf_assets(asset_kind="forecast_grib_rs_buffered")

    assert asset["asset_id"] == "ecmwf.ifs.fc.20260311T000000Z.rsbuf"
    assert same_path["valid_to"] == "2026-03-27T00:00:00"
    assert found is not None
    assert found["relative_path"] == "data/interim/ecmwf/fc_2026-03-11_00_IFS_rsbuf.grib2"
    assert listed[0]["asset_id"] == "ecmwf.ifs.fc.20260311T000000Z.rsbuf"


def test_history_repository_replaces_forecast_manual_edits(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            asset_kind="forecast_grib_rs_buffered",
            format="GRIB2",
            relative_path="data/interim/ecmwf/fc_2026-03-11_00_IFS_rsbuf.grib2",
            provider_code="ecmwf",
            valid_from="2026-03-11T03:00:00",
            valid_to="2026-03-26T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z"},
        )
        first = repository.replace_forecast_manual_edits(
            "ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            [
                {
                    "t0_step": 0,
                    "t1_step": 24,
                    "shift_lat": 2.0,
                    "shift_lon": -1.0,
                    "rotation_deg": 5.0,
                    "multiplication_factor": 1.2,
                    "editor": "tester",
                    "reason": "operational adjustment",
                    "metadata": {"mode_label": "acumulado_nativo"},
                }
            ],
        )
        replaced = repository.replace_forecast_manual_edits(
            "ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            [
                {
                    "t0_step": 0,
                    "t1_step": 24,
                    "shift_lat": 3.0,
                    "shift_lon": -2.0,
                    "rotation_deg": 1.0,
                    "multiplication_factor": 1.1,
                    "editor": "tester",
                    "reason": "updated adjustment",
                    "metadata": {"mode_label": "acumulado_nativo"},
                },
                {
                    "t0_step": 24,
                    "t1_step": 48,
                    "shift_lat": 0.0,
                    "shift_lon": 0.0,
                    "rotation_deg": 0.0,
                    "multiplication_factor": 0.9,
                    "editor": "tester",
                    "reason": "segunda janela",
                    "metadata": {},
                },
            ],
        )

    assert len(first) == 1
    assert len(replaced) == 2
    assert replaced[0]["t0_step"] == 0
    assert replaced[0]["shift_lat"] == 3.0
    assert replaced[0]["reason"] == "updated adjustment"
    assert replaced[1]["t0_step"] == 24
    assert replaced[1]["reason"] == "segunda janela"


def test_history_repository_rejects_overlapping_forecast_manual_edits(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            asset_kind="forecast_grib_rs_buffered",
            format="GRIB2",
            relative_path="data/interim/ecmwf/fc_2026-03-11_00_IFS_rsbuf.grib2",
            provider_code="ecmwf",
            valid_from="2026-03-11T03:00:00",
            valid_to="2026-03-26T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z"},
        )
        with pytest.raises(ValueError, match="Sobreposicao"):
            repository.replace_forecast_manual_edits(
                "ecmwf.ifs.fc.20260311T000000Z.rsbuf",
                [
                    {
                        "t0_step": 0,
                        "t1_step": 24,
                        "shift_lat": 0.0,
                        "shift_lon": 0.0,
                        "rotation_deg": 0.0,
                        "multiplication_factor": 1.0,
                        "editor": "tester",
                        "reason": "primeira",
                        "metadata": {},
                    },
                    {
                        "t0_step": 12,
                        "t1_step": 48,
                        "shift_lat": 0.0,
                        "shift_lon": 0.0,
                        "rotation_deg": 0.0,
                        "multiplication_factor": 1.0,
                        "editor": "tester",
                        "reason": "sobreposta",
                        "metadata": {},
                    },
                ],
            )


def test_history_repository_allows_touching_forecast_manual_edits(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            asset_kind="forecast_grib_rs_buffered",
            format="GRIB2",
            relative_path="data/interim/ecmwf/fc_2026-03-11_00_IFS_rsbuf.grib2",
            provider_code="ecmwf",
            valid_from="2026-03-11T03:00:00",
            valid_to="2026-03-26T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z"},
        )
        rows = repository.replace_forecast_manual_edits(
            "ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            [
                {
                    "t0_step": 0,
                    "t1_step": 24,
                    "shift_lat": 0.0,
                    "shift_lon": 0.0,
                    "rotation_deg": 0.0,
                    "multiplication_factor": 1.0,
                    "editor": "tester",
                    "reason": "janela 1",
                    "metadata": {},
                },
                {
                    "t0_step": 24,
                    "t1_step": 48,
                    "shift_lat": 1.0,
                    "shift_lon": 0.0,
                    "rotation_deg": 0.0,
                    "multiplication_factor": 1.1,
                    "editor": "tester",
                    "reason": "janela 2",
                    "metadata": {},
                },
            ],
        )

    assert [row["t0_step"] for row in rows] == [0, 24]
