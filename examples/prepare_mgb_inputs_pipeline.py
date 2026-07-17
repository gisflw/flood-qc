# %% [markdown]
# # Prepare MGB inputs from observed rainfall and ECMWF forecast rainfall
#
# This example is written as a linear, notebook-style pipeline. Open it in an
# editor that understands `# %%` cells, edit the constants below, then run the
# cells from top to bottom.

# %%
from __future__ import annotations

from datetime import timedelta, timezone
from pathlib import Path

from mgb_ops.adapters import get_forecast_adapter
from mgb_ops.adapters.observed_inmet import INMET_API_KEY_ENV
from mgb_ops.assets.schemas import SQL_DIR
from mgb_ops.config.runtime import build_runtime_context
from mgb_ops.utils.time import TIMEZONE, build_horizon_window, resolve_forecast_cycle, resolve_reference_time
from mgb_ops.workflows.forecast import ingest_forecast_grids
from mgb_ops.workflows.observed import (
    discover_observed_provider_csvs,
    fetch_observed_provider,
    load_observed_provider_csvs,
)
from mgb_ops.model.prepare_mgb_meta import rewrite_mgb_meta
from mgb_ops.assets.forecast_registry import find_required_forecast_asset
from mgb_ops.model.prepare_mgb_rainfall import prepare_mgb_rainfall
from mgb_ops.assets.databases import initialize_history_db

# %% [markdown]
# ## 1. Choose workspace and providers
#
# `WORKSPACE` should point at a regional workspace with `config/`, `data/`,
# `logs/`, and `mgb_runner/`. The default observed provider is ANA. To enable
# INMET, add `"inmet"` to `OBSERVED_PROVIDERS` and provide `INMET_API_KEY` in
# the environment or workspace `.env`.

# %%
WORKSPACE = Path("/path/to/regional/workspace")

OBSERVED_PROVIDERS = ("ana",)
OBSERVED_STATION_CODES_BY_PROVIDER = {
    "ana": None,
    "inmet": None,
}
FETCH_OBSERVED_PROVIDERS = True
OBSERVED_DOWNLOAD_RUN_ID = None

INITIALIZE_HISTORY = False
HISTORY_STATION_INVENTORY_CSV = WORKSPACE / "data" / "source" / "history_station_inventory.csv"

PARHIG_PATH = WORKSPACE / "mgb_runner" / "Input" / "PARHIG.hig"
MINI_GTP_PATH = WORKSPACE / "mgb_runner" / "Input" / "MINI.gtp"
CHUVABIN_PATH = WORKSPACE / "mgb_runner" / "Input" / "chuvabin.hig"

# %% [markdown]
# ## 2. Load runtime paths and settings
#
# Settings are read from `<workspace>/config/custom.yaml` over the package
# defaults. This example requires the usual MGB timing settings and, when
# forecast rainfall is enabled, `spatial_grid.bbox` and
# `spatial_grid.resolution_degrees`.

# %%
context = build_runtime_context(workspace=WORKSPACE)
paths = context.paths
settings = context.settings
paths.ensure_standard_dirs()

print(f"workspace: {paths.workspace}")
print(f"history db: {paths.history_db}")
print(f"MGB input dir: {paths.mgb_input_dir}")

# %% [markdown]
# ## 3. Optionally initialize `history.sqlite`
#
# Initialization creates the operational schema and loads station inventory. Run
# this only when starting a new workspace database or intentionally refreshing
# inventory metadata.

# %%
if INITIALIZE_HISTORY:
    initialize_history_db(
        paths.history_db,
        HISTORY_STATION_INVENTORY_CSV,
        SQL_DIR / "history_schema.sql",
    )
    print(f"initialized history db: {paths.history_db}")
else:
    print("history initialization skipped")

if not paths.history_db.exists():
    raise FileNotFoundError(
        f"history.sqlite was not found at {paths.history_db}. "
        "Set INITIALIZE_HISTORY=True or create the database before continuing."
    )

# %% [markdown]
# ## 4. Resolve reference, fetch, and MGB horizon windows
#
# `run.reference_time` may be an ISO timestamp, `now`, or `yesterday`. MGB input
# timing is derived from that reference time and the configured MGB observed-history
# and forecast horizon lengths. Observed ingestion uses the separate request window.

# %%
reference_time = resolve_reference_time(str(settings["run"]["reference_time"]))
timestep_hours = int(settings["run"]["timestep_hours"])
mgb_settings = settings["mgb"]
fetch_window = build_horizon_window(
    reference_time,
    days_before=int(settings["ingest"]["request_days"]) - 1,
    timestep_hours=timestep_hours,
)
mgb_window = build_horizon_window(
    reference_time,
    days_before=int(mgb_settings["input_days_before"]),
    horizon_days=int(mgb_settings["forecast_horizon_days"]),
    timestep_hours=timestep_hours,
)

print(f"reference_time: {mgb_window.reference_time.isoformat(timespec='seconds')}")
print(f"fetch start: {fetch_window.start_time.isoformat(timespec='seconds')}")
print(f"MGB input start: {mgb_window.start_time.isoformat(timespec='seconds')}")
print(f"forecast start: {mgb_window.forecast_start_time.isoformat(timespec='seconds')}")
print(f"forecast hours: {mgb_window.forecast_nt}")
print(f"total MGB hours: {mgb_window.nt}")

# %% [markdown]
# ## 5. Fetch/load observed providers into SQLite
#
# Fetch writes normalized CSVs under `data/downloads/`. Loading is separate so
# existing CSVs can be re-imported after load logic or timestep settings change.
# Set `FETCH_OBSERVED_PROVIDERS=False` to skip provider requests and load already
# downloaded CSVs. ANA includes rain, level, and flow series. INMET imports rain.

# %%
observed_summaries = []
for provider_code in OBSERVED_PROVIDERS:
    provider = provider_code.strip().lower()
    credential = None
    if provider == "inmet":
        credential = context.env.get(INMET_API_KEY_ENV)
        if FETCH_OBSERVED_PROVIDERS and not credential:
            raise RuntimeError(f"Set {INMET_API_KEY_ENV} before enabling INMET ingestion.")

    if FETCH_OBSERVED_PROVIDERS:
        fetch_summary = fetch_observed_provider(
            provider,
            database_path=paths.history_db,
            window_start=fetch_window.start_time,
            window_end=fetch_window.reference_time,
            downloads_dir=paths.downloads_dir,
            logs_dir=paths.logs_dir,
            station_codes=OBSERVED_STATION_CODES_BY_PROVIDER.get(provider),
            timeout_seconds=float(settings["ingest"]["timeout_seconds"]),
            fetch_window_days=int(settings["ingest"]["fetch_window_days"]),
            credential=credential,
        )
        csv_paths = fetch_summary.csv_paths
        run_id = fetch_summary.run_id
    else:
        csv_paths = discover_observed_provider_csvs(
            paths.downloads_dir,
            provider,
            run_id=OBSERVED_DOWNLOAD_RUN_ID,
            station_codes=OBSERVED_STATION_CODES_BY_PROVIDER.get(provider),
        )
        run_id = OBSERVED_DOWNLOAD_RUN_ID or "existing"

    import_summary = load_observed_provider_csvs(
        provider,
        database_path=paths.history_db,
        csv_paths=csv_paths,
        timestep_hours=timestep_hours,
        observed_aggregation=dict(settings["ingest"]["observed_aggregation"]),
    )
    observed_summaries.append((provider, run_id, len(csv_paths), import_summary))
    print(
        f"{provider}: loaded {len(csv_paths)} CSV files; "
        f"imported {import_summary.rows_imported} rows"
    )

# %% [markdown]
# ## 6. Find the configured forecast asset for this forecast window
#
# The lookup is by the expected deterministic forecast cycle and by full coverage of
# the MGB forecast window, not by the latest registered asset.

# %%
use_forecast_data = bool(mgb_settings["use_forecast_data"])
forecast_asset = None
forecast_provider = str(settings["forecast"]["provider"]).strip().lower()
forecast_cycle = resolve_forecast_cycle(mgb_window.reference_time)
forecast_start_utc = mgb_window.forecast_start_time.replace(
    tzinfo=TIMEZONE
).astimezone(timezone.utc)
forecast_end_utc = (
    mgb_window.forecast_start_time
    + timedelta(hours=timestep_hours * (mgb_window.forecast_nt - 1))
).replace(tzinfo=TIMEZONE).astimezone(timezone.utc)

if use_forecast_data:
    forecast_asset = find_required_forecast_asset(
        paths.history_db,
        workspace_path=paths.workspace,
        provider_code=forecast_provider,
        cycle_time=forecast_cycle,
        required_start=forecast_start_utc,
        required_end=forecast_end_utc,
    )

    if forecast_asset is None:
        print(f"{forecast_provider} asset missing for cycle {forecast_cycle:%Y-%m-%dT%H:%M:%SZ}")
    else:
        print(f"{forecast_provider} asset found: {forecast_asset.asset_id}")
        print(f"{forecast_provider} file: {forecast_asset.asset_path}")
else:
    print("forecast rainfall disabled by settings; forecast block will be zero-filled")

# %% [markdown]
# ## 7. If missing, ingest/register the forecast grid and resolve it again
#
# Forecast ingestion downloads the deterministic total-precipitation GRIB inside
# the configured adapter, writes a canonical NetCDF, and registers only that
# NetCDF in `history.sqlite`.

# %%
if use_forecast_data and forecast_asset is None:
    spatial_grid_settings = settings["spatial_grid"]
    bbox = spatial_grid_settings["bbox"]
    if bbox is None:
        raise ValueError("Set spatial_grid.bbox in config/custom.yaml.")

    grid_summary = ingest_forecast_grids(
        paths.history_db,
        reference_time=mgb_window.reference_time,
        bbox=tuple(float(value) for value in bbox),
        resolution_degrees=float(spatial_grid_settings["resolution_degrees"]),
        downloads_dir=paths.downloads_dir,
        logs_dir=paths.logs_dir,
        asset_base_dir=paths.workspace,
        timestep_hours=timestep_hours,
        adapter=get_forecast_adapter(forecast_provider),
    )
    print(f"registered {forecast_provider} asset: {grid_summary.asset_id}")

    forecast_asset = find_required_forecast_asset(
        paths.history_db,
        workspace_path=paths.workspace,
        provider_code=forecast_provider,
        cycle_time=forecast_cycle,
        required_start=forecast_start_utc,
        required_end=forecast_end_utc,
    )
    if forecast_asset is None:
        raise RuntimeError(f"{forecast_provider} ingestion finished, but no asset covers the required forecast window.")

# %% [markdown]
# ## 8. Rewrite `PARHIG.hig`
#
# This updates MGB start date, `NT`, and `DT` in place based on the resolved
# window. The rest of the file is preserved.

# %%
meta_summary = rewrite_mgb_meta(
    parhig_path=PARHIG_PATH,
    reference_time=mgb_window.reference_time,
    input_days_before=int(mgb_settings["input_days_before"]),
    forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
    timestep_hours=timestep_hours,
    logs_dir=paths.logs_dir,
)

print(f"updated PARHIG: {meta_summary.parhig_path}")
print(f"PARHIG start={meta_summary.start_time.isoformat(timespec='seconds')} nt={meta_summary.nt}")

# %% [markdown]
# ## 9. Write `chuvabin.hig`
#
# Observed rainfall is interpolated from station series. If forecast rainfall is
# enabled, matching ECMWF rainfall fills the forecast period; otherwise the
# forecast block is zero-filled.

# %%
rainfall_settings = settings["rainfall_interpolation"]
spatial_settings = settings["spatial_grid"]
if spatial_settings["bbox"] is None:
    raise ValueError("spatial_grid.bbox is required for MGB rainfall preparation.")
rainfall_summary = prepare_mgb_rainfall(
    history_db=paths.history_db,
    parhig_path=PARHIG_PATH,
    mini_gtp_path=MINI_GTP_PATH,
    output_path=CHUVABIN_PATH,
    reference_time=mgb_window.reference_time,
    input_days_before=int(mgb_settings["input_days_before"]),
    forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
    use_forecast_data=use_forecast_data,
    forecast_asset_path=forecast_asset.asset_path if forecast_asset is not None else None,
    cache_dir=paths.cache_dir,
    spatial_bbox=tuple(float(value) for value in spatial_settings["bbox"]),
    spatial_resolution_degrees=float(spatial_settings["resolution_degrees"]),
    observed_providers=list(OBSERVED_PROVIDERS),
    nearest_stations=int(rainfall_settings["nearest_stations"]),
    power=float(rainfall_settings["power"]),
    timestep_hours=timestep_hours,
    logs_dir=paths.logs_dir,
)

print(f"wrote rainfall input: {rainfall_summary.output_path}")
print(f"stations used: {rainfall_summary.station_count}")
print(f"hours written: {rainfall_summary.nt}")

# %% [markdown]
# ## 10. Concise summaries
#
# The produced files are ready under `mgb_runner/Input/`. Database updates are
# already committed by the ingestion functions.

# %%
print("produced files")
print(f"- {PARHIG_PATH}")
print(f"- {CHUVABIN_PATH}")

print("database updates")
for provider, run_id, csv_file_count, import_summary in observed_summaries:
    print(
        f"- {provider}: run={run_id}, "
        f"csv_files={csv_file_count}, "
        f"rows_imported={import_summary.rows_imported}"
    )
if forecast_asset is not None:
    print(f"- ecmwf: asset={forecast_asset.asset_id}, file={forecast_asset.asset_path}")
else:
    print("- ecmwf: no forecast asset used")
