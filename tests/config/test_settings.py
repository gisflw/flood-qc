from __future__ import annotations

import pytest

from mgb_ops.config import settings as settings_module


CUSTOM_CONFIG = """\
run:
  timestep_hours: 3

forecast:
  buffer_fraction: 1.5

ingest:
  timeout_seconds: 30
  fetch_window_days: 14

spatial_grid:
  bbox: [-60.0, -35.0, -48.0, -26.0]
  resolution_degrees: 0.25

summaries:
  selected_mini_ids: ["7601", "7612"]

mgb:
  input_days_before: 45
  forecast_horizon_days: 20

rainfall_interpolation:
  power: 3.0
"""


def write_workspace_custom(workspace, text: str) -> None:
    config_dir = workspace / "config"
    config_dir.mkdir(parents=True)
    (config_dir / "custom.yaml").write_text(text, encoding="utf-8")


def test_load_settings_uses_in_code_defaults_without_workspace_config(tmp_path) -> None:
    settings = settings_module.load_settings(workspace=tmp_path)

    assert settings == settings_module.DEFAULT_SETTINGS
    assert settings["run"]["reference_time"] == "now"
    assert settings["run"]["timestep_hours"] == 1
    assert settings["forecast"]["lookback_cycles"] == 12
    assert settings["forecast"]["buffer_fraction"] == 2.0
    assert settings["ingest"]["request_days"] == 90
    assert settings["ingest"]["fetch_window_days"] == 30
    assert settings["ingest"]["observed_aggregation"] == {"rain": "sum", "level": "mean", "flow": "mean"}
    assert settings["mgb"]["use_forecast_data"] is True


def test_load_settings_merges_workspace_custom_yaml(tmp_path) -> None:
    write_workspace_custom(tmp_path, CUSTOM_CONFIG)

    settings = settings_module.load_settings(workspace=tmp_path)

    assert settings["run"]["reference_time"] == "now"
    assert settings["run"]["timestep_hours"] == 3
    assert settings["ingest"]["request_days"] == 90
    assert settings["ingest"]["timeout_seconds"] == 30
    assert settings["ingest"]["fetch_window_days"] == 14
    assert settings["forecast"]["buffer_fraction"] == 1.5
    assert settings["spatial_grid"]["bbox"] == [-60.0, -35.0, -48.0, -26.0]
    assert settings["spatial_grid"]["resolution_degrees"] == 0.25
    assert settings["summaries"]["forecast_days"] == [1, 3, 7, 14]
    assert settings["summaries"]["selected_mini_ids"] == ["7601", "7612"]
    assert settings["mgb"]["input_days_before"] == 45
    assert settings["mgb"]["output_days_before"] == 28
    assert settings["mgb"]["forecast_horizon_days"] == 20
    assert settings["rainfall_interpolation"]["nearest_stations"] == 5
    assert settings["rainfall_interpolation"]["power"] == 3.0


def test_load_settings_requires_workspace_custom_when_requested(tmp_path) -> None:
    with pytest.raises(FileNotFoundError, match="Config file not found"):
        settings_module.load_settings(workspace=tmp_path, require_custom=True)


def test_load_settings_rejects_legacy_config_dir(tmp_path) -> None:
    with pytest.raises(ValueError, match="config_dir is no longer supported"):
        settings_module.load_settings(config_dir=tmp_path)


@pytest.mark.parametrize(
    ("custom_text", "expected_error"),
    [
        (
            """\
ingest:
  timeout_seconds: 0
""",
            "number > 0",
        ),
        (
            """\
ingest:
  fetch_window_days: 0
""",
            "integer >= 1",
        ),
        (
            """\
run:
  reference_time: ""
""",
            "cannot be empty",
        ),
        (
            """\
run:
  timestep_hours: 0
""",
            "integer >= 1",
        ),
        (
            """\
run:
  timestep_hours: 5
""",
            "divide 24",
        ),
        (
            """\
ingest:
  observed_aggregation:
    rain: median
    level: mean
    flow: mean
""",
            "one of",
        ),
        (
            """\
mgb:
  use_forecast_data: "no"
""",
            "boolean",
        ),
        (
            """\
spatial_grid:
  bbox: [-60.0, -35.0, -60.0, -26.0]
""",
            "west < east",
        ),
        (
            """\
spatial_grid:
  resolution_degrees: -1
""",
            "number > 0",
        ),
        (
            """\
run:
  reference_time: yesterday
""",
            "valid ISO string or \x27now\x27",
        ),
        (
            """\
forecast:
  provider: noaa
""",
            "unsupported keys",
        ),
        (
            """\
unknown:
  value: true
""",
            "unsupported keys",
        ),
        ("- item", "expected a YAML object"),
    ],
)
def test_load_settings_rejects_invalid_workspace_custom_yaml(
    tmp_path,
    custom_text: str,
    expected_error: str,
) -> None:
    write_workspace_custom(tmp_path, custom_text)

    with pytest.raises(ValueError, match=expected_error):
        settings_module.load_settings(workspace=tmp_path)
