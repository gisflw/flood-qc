"""Forecast scenarios resolved into, and read from, current-run artifacts."""
from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Literal, Mapping, Any

import pandas as pd

from mgb_ops.adapters import get_forecast_adapter
from mgb_ops.assets.current_run import (
    CurrentRunArtifact, ForecastScenarioReference, ObservedReplacement, create_current_artifact,
    load_current_artifact,
)
from mgb_ops.assets.forecast_registry import list_forecast_assets
from mgb_ops.edit.forcing import ForecastCorrectionInstruction
from mgb_ops.workflows.forecast import list_enabled_forecast_providers
from mgb_ops.analysis.observations import load_preferred_rainfall_observations
from mgb_ops.qc.precipitation import PrecipitationQCPolicy, detect_suspect_precipitation
from mgb_ops.utils.time import build_forecast_coverage_window_utc

ScenarioKind = Literal["zero", "raw", "corrected"]


@dataclass(frozen=True, slots=True)
class ForecastScenario:
    scenario_id: str
    label: str
    kind: ScenarioKind
    provider_code: str | None = None
    asset_id: str | None = None
    asset_path: Path | None = None
    correction_id: int | None = None
    correction: ForecastCorrectionInstruction | None = None
    corrections: tuple[ForecastCorrectionInstruction, ...] = ()


def _utc_naive(value: datetime | pd.Timestamp | str) -> pd.Timestamp:
    timestamp = pd.Timestamp(value)
    return timestamp.tz_convert("UTC").tz_localize(None) if timestamp.tzinfo else timestamp


def _parse_cycle(value: object) -> pd.Timestamp | None:
    if value in (None, ""):
        return None
    try:
        return _utc_naive(value)
    except (TypeError, ValueError):
        return None


def resolve_forecast_scenario_references(
    history_database_path: Path,
    workspace_path: Path,
    *,
    required_start: datetime,
    required_end: datetime,
) -> tuple[ForecastScenarioReference, ...]:
    """Resolve raw and zero scenarios once, while constructing an artifact."""
    enabled = list_enabled_forecast_providers(history_database_path)
    for provider in enabled:
        get_forecast_adapter(provider)
    start, end = _utc_naive(required_start), _utc_naive(required_end)
    eligible: dict[str, list[dict[str, object]]] = {provider: [] for provider in enabled}
    for row in list_forecast_assets(history_database_path, workspace_path=workspace_path).to_dict("records"):
        provider, cycle = str(row.get("provider_code") or "").strip().lower(), _parse_cycle(row.get("cycle_time"))
        if provider not in eligible or cycle is None:
            continue
        if _utc_naive(row["valid_from"]) <= start and _utc_naive(row["valid_to"]) >= end and Path(row["asset_path"]).is_file():
            eligible[provider].append(row)
    missing = sorted(provider for provider, rows in eligible.items() if not rows)
    if missing:
        raise RuntimeError(f"No registered, on-disk forecast asset covers the runtime window for enabled providers: {missing}.")
    selected: list[dict[str, object]] = []
    for provider, rows in eligible.items():
        latest = max(_parse_cycle(row["cycle_time"]) for row in rows)
        matches = [row for row in rows if _parse_cycle(row["cycle_time"]) == latest]
        if len(matches) != 1:
            raise RuntimeError(f"Multiple forecast assets for provider {provider!r} and cycle {latest.isoformat()}.")
        selected.append(matches[0])
    refs = [ForecastScenarioReference("zero", 0, "zero", "Zero-rain horizon")]
    for position, row in enumerate(sorted(selected, key=lambda x: (str(x["provider_code"]), str(x["asset_id"]))), start=1):
        asset_id, provider = str(row["asset_id"]), str(row["provider_code"])
        refs.append(ForecastScenarioReference(
            scenario_id=f"raw:{asset_id}", position=position, kind="raw",
            label=f"{provider.upper()} raw - {asset_id}", provider_code=provider,
            source_asset_id=asset_id, source_asset_path=str(row["asset_path"]),
        ))
    return tuple(refs)


def build_current_artifact(
    database_path: Path,
    history_database_path: Path,
    workspace_path: Path,
    *,
    reference_time: datetime | str,
    forecast_end_exclusive: datetime | str,
    timestep_hours: int,
    mgb_settings: Mapping[str, Any],
    spatial_settings: Mapping[str, Any],
    interpolation_settings: Mapping[str, Any],
    review_window: Mapping[str, Any],
    observed_providers: tuple[str, ...] | list[str],
    responsible_person: str | None = None,
    reason: str | None = None,
    precipitation_qc_settings: Mapping[str, Any] | None = None,
) -> CurrentRunArtifact:
    coverage_start, coverage_end = build_forecast_coverage_window_utc(
        pd.Timestamp(reference_time).to_pydatetime(),
        horizon_days=int(mgb_settings["forecast_horizon_days"]),
        timestep_hours=int(timestep_hours),
    )
    refs = resolve_forecast_scenario_references(
        history_database_path,
        workspace_path,
        required_start=coverage_start,
        required_end=coverage_end,
    )
    policy = PrecipitationQCPolicy.from_mapping(precipitation_qc_settings)
    seeded_replacements: tuple[ObservedReplacement, ...] = ()
    if not Path(database_path).exists():
        review_start, review_end = pd.Timestamp(review_window["start"]), pd.Timestamp(review_window["end"])
        observations = load_preferred_rainfall_observations(
            history_database_path, start_time=review_start.to_pydatetime(),
            end_time=review_end.to_pydatetime(), providers=observed_providers,
        )
        matches = detect_suspect_precipitation(observations, timestep_hours=int(timestep_hours), policy=policy)
        values: dict[tuple[str, str], float | None] = {}
        for match in matches.matches:
            values[(match.station_id, match.observed_at)] = None if match.match_type == "threshold" else 0.0
        seeded_replacements = tuple(ObservedReplacement(station, observed_at, value) for (station, observed_at), value in sorted(values.items()))
    return create_current_artifact(database_path, CurrentRunArtifact(
        reference_time=str(reference_time), forecast_end_exclusive=str(forecast_end_exclusive),
        timestep_hours=int(timestep_hours), mgb_settings=dict(mgb_settings), spatial_settings=dict(spatial_settings),
        interpolation_settings=dict(interpolation_settings), review_window=dict(review_window),
        observed_providers=tuple(observed_providers), precipitation_qc_settings=policy.as_dict(),
        responsible_person=responsible_person, reason=reason or "default precipitation replacements",
        scenarios=refs, observed_replacements=seeded_replacements,
    ))


def scenarios_from_artifact(artifact: CurrentRunArtifact) -> tuple[ForecastScenario, ...]:
    output: list[ForecastScenario] = []
    for reference in artifact.scenarios:
        corrections = tuple(reference.corrections)
        output.append(ForecastScenario(
            scenario_id=reference.scenario_id, label=reference.label, kind=reference.kind, provider_code=reference.provider_code,
            asset_id=reference.source_asset_id, asset_path=Path(reference.source_asset_path) if reference.source_asset_path else None,
            correction=corrections[0] if len(corrections) == 1 else None, corrections=corrections,
        ))
    return tuple(output)


def derive_forecast_scenarios(
    database_path: Path,
    workspace_path: Path | None = None,
    *,
    required_start: datetime | None = None,
    required_end: datetime | None = None,
) -> tuple[ForecastScenario, ...]:
    """Read the resolved scenario snapshot from ``current_run.sqlite``.

    ``workspace_path`` and window arguments are retained only for a gentle API
    transition; they are intentionally not consulted to derive fresh state.
    """
    del workspace_path, required_start, required_end
    return scenarios_from_artifact(load_current_artifact(database_path))
