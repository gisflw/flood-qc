from __future__ import annotations

import pytest

from common import settings as settings_module


DEFAULT_CONFIG = """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1, 3, 10, 30]
  accum_hours: [24, 72, 240, 720]
  selected_mini_ids: [\"7601\"]

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
"""


CUSTOM_CONFIG = """\
ingest:
  timeout_seconds: 30

summaries:
  selected_mini_ids: [\"7601\", \"7612\"]

mgb:
  input_days_before: 45
  forecast_horizon_days: 20

rainfall_interpolation:
  power: 3.0
"""


EMPTY_CUSTOM = """\
# local overrides
"""


def write_config(tmp_path, *, default_text: str | None = DEFAULT_CONFIG, custom_text: str | None = EMPTY_CUSTOM) -> None:
    if default_text is not None:
        (tmp_path / "default.yaml").write_text(default_text, encoding="utf-8")
    if custom_text is not None:
        (tmp_path / "custom.yaml").write_text(custom_text, encoding="utf-8")


def test_load_settings_merges_default_and_custom(tmp_path, monkeypatch) -> None:
    write_config(tmp_path, custom_text=CUSTOM_CONFIG)
    monkeypatch.setattr(settings_module, "CONFIG_DIR", tmp_path)

    settings = settings_module.load_settings()

    assert settings["run"]["reference_time"] == "2026-03-11T00:00:00"
    assert settings["ingest"]["request_days"] == 7
    assert settings["ingest"]["timeout_seconds"] == 30
    assert settings["summaries"]["forecast_days"] == [1, 3, 10, 30]
    assert settings["summaries"]["accum_hours"] == [24, 72, 240, 720]
    assert settings["summaries"]["selected_mini_ids"] == ["7601", "7612"]
    assert settings["mgb"]["input_days_before"] == 45
    assert settings["mgb"]["output_days_before"] == 30
    assert settings["mgb"]["forecast_horizon_days"] == 20
    assert settings["mgb"]["use_forecast_data"] is False
    assert settings["rainfall_interpolation"]["nearest_stations"] == 5
    assert settings["rainfall_interpolation"]["power"] == 3.0


def test_load_settings_accepts_now_and_yesterday(tmp_path, monkeypatch) -> None:
    write_config(
        tmp_path,
        default_text="""\
run:
  reference_time: \"now\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
    )
    monkeypatch.setattr(settings_module, "CONFIG_DIR", tmp_path)

    settings = settings_module.load_settings()

    assert settings["run"]["reference_time"] == "now"

    write_config(
        tmp_path,
        default_text="""\
run:
  reference_time: "yesterday"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
    )

    settings = settings_module.load_settings()

    assert settings["run"]["reference_time"] == "yesterday"


def test_load_settings_accepts_missing_custom_yaml(tmp_path, monkeypatch) -> None:
    write_config(tmp_path, custom_text=None)
    monkeypatch.setattr(settings_module, "CONFIG_DIR", tmp_path)

    settings = settings_module.load_settings()

    assert settings["run"]["reference_time"] == "2026-03-11T00:00:00"


@pytest.mark.parametrize(
    ("default_text", "custom_text", "expected_error"),
    [
        (None, EMPTY_CUSTOM, "Config file not found"),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
            EMPTY_CUSTOM,
            "missing required keys",
        ),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"
  mode: \"operational\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
            EMPTY_CUSTOM,
            "contains unsupported keys",
        ),
        (
            """\
run:
  reference_time: \"\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
            EMPTY_CUSTOM,
            "cannot be empty",
        ),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 0
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
            EMPTY_CUSTOM,
            "inteiro >= 1",
        ),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 0
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
            EMPTY_CUSTOM,
            "inteiro >= 1",
        ),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false
  source: \"runner\"

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
            EMPTY_CUSTOM,
            "contains unsupported keys",
        ),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 0
  power: 2.0
""",
            EMPTY_CUSTOM,
            "inteiro >= 1",
        ),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: false

rainfall_interpolation:
  nearest_stations: 5
  power: 0
""",
            EMPTY_CUSTOM,
            "numero > 0",
        ),
        (
            """\
run:
  reference_time: \"2026-03-11T00:00:00\"

ingest:
  request_days: 7
  timeout_seconds: 15

summaries:
  forecast_days: [1]
  accum_hours: [24]
  selected_mini_ids: []

mgb:
  input_days_before: 30
  output_days_before: 30
  forecast_horizon_days: 15
  use_forecast_data: "no"

rainfall_interpolation:
  nearest_stations: 5
  power: 2.0
""",
            EMPTY_CUSTOM,
            "booleano",
        ),
        ("- item", EMPTY_CUSTOM, "esperado um objeto YAML"),
    ],
)
def test_load_settings_rejects_invalid_configs(
    tmp_path,
    monkeypatch,
    default_text: str | None,
    custom_text: str | None,
    expected_error: str,
) -> None:
    write_config(tmp_path, default_text=default_text, custom_text=custom_text)
    monkeypatch.setattr(settings_module, "CONFIG_DIR", tmp_path)

    with pytest.raises((FileNotFoundError, ValueError), match=expected_error):
        settings_module.load_settings()
