from __future__ import annotations

import csv
from pathlib import Path
import sqlite3

import pytest
import pandas as pd

import mgb_ops.adapters.inventory_ana as inventory_ana
from mgb_ops.adapters.inventory_ana import fetch_station_metadata, parse_station_metadata
from mgb_ops.adapters.inventory_inmet import parse_inventory
from mgb_ops.adapters.observed_ana import write_normalized_csv
from mgb_ops.assets.databases import initialize_history_db, load_history_station_inventory
from mgb_ops.assets.schemas import SQL_DIR


ANA_XML = """<NewDataSet><Table>
<Codigo>02650035</Codigo><Nome>Água Verde</Nome><RioNome>Rio Verde</RioNome>
<AreaDrenagem>1987.123456</AreaDrenagem><Latitude>-26.81923456</Latitude>
<Longitude>-50.98567891</Longitude><Altitude>41.18</Altitude>
<nmMunicipio>São José</nmMunicipio><nmEstado>SC</nmEstado>
<ResponsavelSigla>ANA</ResponsavelSigla><OperadoraSigla>OPERADORA</OperadoraSigla>
</Table></NewDataSet>"""


def _write_inventory(path: Path, rows: list[dict[str, str]]) -> None:
    fields = (
        "provider_code", "station_code", "station_name", "mini_id", "latitude", "longitude",
        "altitude_m", "observed_variables", "river_name", "drainage_area_km2", "municipality",
        "state", "responsible_agency", "operator",
    )
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def test_ana_inventory_fetch_requests_padded_exact_station_and_preserves_precision() -> None:
    class Response:
        text = ANA_XML

        def raise_for_status(self) -> None:
            pass

    class Session:
        calls = []

        def get(self, url, **kwargs):
            self.calls.append((url, kwargs))
            return Response()

    session = Session()
    metadata = fetch_station_metadata("2650035", session=session)

    url, request = session.calls[0]
    assert url.endswith("HidroInventario")
    assert request["params"]["codEstDE"] == request["params"]["codEstATE"] == "02650035"
    assert request["params"]["tpEst"] == ""
    assert {
        "nmEst", "nmRio", "codSubBacia", "codBacia", "nmMunicipio", "nmEstado",
        "sgResp", "sgOper", "telemetrica",
    }.issubset(request["params"])
    assert metadata["station_code"] == "2650035"
    assert metadata["station_name"] == "Água Verde"
    assert metadata["latitude"] == -26.81923456
    assert metadata["altitude_m"] == 41.18
    assert metadata["drainage_area_km2"] == 1987.123456
    assert metadata["municipality"] == "São José"


def test_ana_inventory_requires_exact_station_and_rejects_conflicting_records() -> None:
    wrong = ANA_XML.replace("02650035", "02650036")
    with pytest.raises(ValueError, match="0 records"):
        parse_station_metadata(wrong, "2650035")

    table = ANA_XML[ANA_XML.index("<Table>"):ANA_XML.index("</Table>") + len("</Table>")]
    duplicate = ANA_XML.replace("</NewDataSet>", table + "</NewDataSet>")
    with pytest.raises(ValueError, match="2 records"):
        parse_station_metadata(duplicate, "2650035")


@pytest.mark.parametrize(("retry_after", "expected_delay"), [("999", 30.0), ("-1", 1.0), ("nan", 1.0)])
def test_ana_inventory_bounds_rate_limit_delay(monkeypatch, retry_after: str, expected_delay: float) -> None:
    class Response:
        def __init__(self, status_code: int) -> None:
            self.status_code = status_code
            self.headers = {"Retry-After": retry_after}
            self.text = ANA_XML

        def raise_for_status(self) -> None:
            if self.status_code != 200:
                raise RuntimeError("rate limited")

    class Session:
        calls = 0

        def get(self, url, **kwargs):
            self.calls += 1
            return Response(429 if self.calls == 1 else 200)

    delays: list[float] = []
    monkeypatch.setattr(inventory_ana.time, "sleep", delays.append)
    fetch_station_metadata("2650035", session=Session())
    assert delays == [expected_delay]


def test_inmet_inventory_keeps_fractional_altitude_and_null_optional_metadata() -> None:
    records = parse_inventory({"data": [{
        "codEstacao": "A801", "nome": "Estação A", "estado": "RS",
        "latitude": -30.1234567, "longitude": -51.2345678, "altitudeEmMetros": 41.18,
        "entidadeResponsavel": "DECEA",
    }]})
    station = records["A801"]
    assert station["station_name"] == "Estação A"
    assert station["altitude_m"] == 41.18
    assert station["latitude"] == -30.1234567
    assert station["river_name"] is station["drainage_area_km2"] is None
    assert station["municipality"] is station["operator"] is None
    assert station["responsible_agency"] == "DECEA"

    with pytest.raises(ValueError, match="duplicate"):
        parse_inventory({"data": [
            {"codEstacao": "A801", "nome": "one"},
            {"codEstacao": "A801", "nome": "two"},
        ]})


def test_ana_writer_emits_only_station_capabilities(tmp_path: Path) -> None:
    output = tmp_path / "observed.csv"
    frame = pd.DataFrame([{
        "observed_at": pd.Timestamp("2026-10-16 12:00"),
        "rain": 1.5, "level": 22.0, "flow": 3.25,
    }])
    write_normalized_csv(
        frame,
        output_path=output,
        station_id="ana:2650035",
        provider_code="ana",
        station_code="2650035",
        variable_columns=("rain",),
    )
    with output.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    assert [row["variable_code"] for row in rows] == ["rain"]


def test_history_inventory_accepts_duplicate_mini_and_preserves_api_values(tmp_path: Path) -> None:
    inventory = tmp_path / "inventory.csv"
    shared = {
        "provider_code": "ana", "station_name": "Água Verde", "mini_id": "42",
        "latitude": "-26.81923456", "longitude": "-50.98567891", "altitude_m": "41.18",
        "observed_variables": "rain,level,flow", "river_name": "Rio Verde",
        "drainage_area_km2": "1987.123456", "municipality": "São José", "state": "SC",
        "responsible_agency": "ANA", "operator": "Operadora",
    }
    _write_inventory(inventory, [
        {**shared, "station_code": "02650035"},
        {**shared, "station_code": "74100000"},
    ])
    database = initialize_history_db(tmp_path / "history.sqlite", inventory, SQL_DIR / "history_schema.sql")

    with sqlite3.connect(database) as connection:
        stations = connection.execute(
            "SELECT station_code, station_name, latitude, altitude_m, drainage_area_km2, river_name "
            "FROM station ORDER BY station_code"
        ).fetchall()
    assert [row[0] for row in stations] == ["2650035", "74100000"]
    assert stations[0][1:] == ("Água Verde", -26.81923456, 41.18, 1987.123456, "Rio Verde")

    _write_inventory(inventory, [{**shared, "station_code": "74100000"}, {**shared, "station_code": "74100000"}])
    with pytest.raises(ValueError, match="Duplicate station"):
        load_history_station_inventory(database, inventory)
