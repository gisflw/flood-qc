from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Literal

import pandas as pd

from mgb_ops.adapters import get_forecast_adapter
from mgb_ops.analysis.forecast import forecast_interval_boundaries
from mgb_ops.assets.forecast_registry import list_forecast_assets
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.edit.forcing import ForecastCorrectionInstruction, validate_instruction
from mgb_ops.workflows.forecast import list_enabled_forecast_providers

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


def _utc_naive(value: datetime | pd.Timestamp | str) -> pd.Timestamp:
    timestamp = pd.Timestamp(value)
    if timestamp.tzinfo is not None:
        return timestamp.tz_convert("UTC").tz_localize(None)
    return timestamp


def _parse_cycle(value: object) -> pd.Timestamp | None:
    if value in (None, ""):
        return None
    try:
        return _utc_naive(value)
    except (TypeError, ValueError):
        return None


def _instruction(row: dict[str, object]) -> ForecastCorrectionInstruction:
    return validate_instruction(
        ForecastCorrectionInstruction(
            asset_id=str(row["asset_id"]),
            t0_step=int(row["t0_step"]),
            t1_step=int(row["t1_step"]),
            shift_lat=float(row["shift_lat"]),
            shift_lon=float(row["shift_lon"]),
            rotation_deg=float(row["rotation_deg"]),
            multiplication_factor=float(row["multiplication_factor"]),
            editor=str(row["editor"]) if row.get("editor") else None,
            reason=str(row.get("reason") or ""),
        )
    )


def derive_forecast_scenarios(
    database_path: Path,
    workspace_path: Path,
    *,
    required_start: datetime,
    required_end: datetime,
) -> tuple[ForecastScenario, ...]:
    """Derive scenarios from each provider's newest usable forecast cycle."""
    enabled = list_enabled_forecast_providers(database_path)
    for provider in enabled:
        get_forecast_adapter(provider)

    start, end = _utc_naive(required_start), _utc_naive(required_end)
    assets = list_forecast_assets(database_path, workspace_path=workspace_path)
    eligible_by_provider: dict[str, list[dict[str, object]]] = {
        provider: [] for provider in enabled
    }
    for row in assets.to_dict("records"):
        provider = str(row.get("provider_code") or "").strip().lower()
        cycle = _parse_cycle(row.get("cycle_time"))
        if provider not in eligible_by_provider or cycle is None:
            continue
        valid_from, valid_to = (
            _utc_naive(row["valid_from"]),
            _utc_naive(row["valid_to"]),
        )
        if valid_from > start or valid_to < end:
            continue
        path = Path(row["asset_path"])
        if not path.is_file():
            continue
        eligible_by_provider[provider].append(row)

    missing_providers = sorted(
        provider for provider, rows in eligible_by_provider.items() if not rows
    )
    if missing_providers:
        raise RuntimeError(
            "No registered, on-disk forecast asset covers the runtime window for enabled "
            f"providers: {missing_providers}."
        )

    selected: list[dict[str, object]] = []
    for provider, rows in eligible_by_provider.items():
        latest_cycle = max(_parse_cycle(row["cycle_time"]) for row in rows)
        latest_rows = [
            row for row in rows if _parse_cycle(row["cycle_time"]) == latest_cycle
        ]
        if len(latest_rows) != 1:
            asset_ids = sorted(str(row["asset_id"]) for row in latest_rows)
            raise RuntimeError(
                f"Multiple forecast assets were registered for provider {provider!r} "
                f"and latest cycle {latest_cycle.isoformat()}: {asset_ids}."
            )
        selected.append(latest_rows[0])

    scenarios: list[ForecastScenario] = [
        ForecastScenario("zero", "Zero-rain horizon", "zero")
    ]
    with HistoryRepository(Path(database_path)) as repository:
        for row in sorted(
            selected,
            key=lambda item: (str(item["provider_code"]), str(item["asset_id"])),
        ):
            asset_id = str(row["asset_id"])
            provider = str(row["provider_code"])
            asset_path = Path(row["asset_path"])
            scenarios.append(
                ForecastScenario(
                    scenario_id=f"raw:{asset_id}",
                    label=f"{provider.upper()} raw - {asset_id}",
                    kind="raw",
                    provider_code=provider,
                    asset_id=asset_id,
                    asset_path=asset_path,
                )
            )
            boundaries = forecast_interval_boundaries(asset_path)
            valid_steps = set(boundaries["start_step_hours"].astype(int))
            valid_steps.update(boundaries["end_step_hours"].astype(int))
            for edit in repository.list_forecast_manual_edits(asset_id):
                instruction = _instruction(edit)
                if instruction.t1_step <= instruction.t0_step:
                    raise ValueError(
                        f"Correction {edit['manual_edit_id']} must satisfy t0_step < t1_step."
                    )
                if (
                    instruction.t0_step not in valid_steps
                    or instruction.t1_step not in valid_steps
                ):
                    raise ValueError(
                        f"Correction {edit['manual_edit_id']} boundaries do not align with asset {asset_id}."
                    )
                correction_id = int(edit["manual_edit_id"])
                scenarios.append(
                    ForecastScenario(
                        scenario_id=f"corrected:{asset_id}:{correction_id}",
                        label=f"{provider.upper()} corrected #{correction_id} - {asset_id}",
                        kind="corrected",
                        provider_code=provider,
                        asset_id=asset_id,
                        asset_path=asset_path,
                        correction_id=correction_id,
                        correction=instruction,
                    )
                )
    return tuple(scenarios)
