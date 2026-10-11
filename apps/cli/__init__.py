"""Command-line adapters for the library's operational workflows."""

from __future__ import annotations

import argparse
from copy import deepcopy
from dataclasses import asdict
import json
import os
from pathlib import Path
import sys

from mgb_ops.config.runtime import RuntimeContext, build_runtime_context
from mgb_ops.utils.time import build_horizon_window, resolve_forecast_cycle, resolve_reference_time

COMMANDS = {
    "runtime": "Show resolved runtime paths and timing",
    "download-observed": "Initialize history, download and ingest ANA/INMET observations",
    "download-forecast": "Update enabled forecast providers",
    "build-current-run": "Build the current-run artifact from workspace settings",
    "execute-current-run": "Execute the saved current-run artifact",
    "run-all": "Download observations and forecasts, then build and execute",
    "generate-report": "Generate an offline HTML or PDF report from a saved run",
    "panel": "Serve the dashboard and open a browser",
}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="mgb-ops", description=__doc__)
    # Accept runtime options both before and after the command. Suppressed
    # subparser defaults preserve options supplied on the parent parser.
    parser.add_argument(
        "--workspace", help="Regional workspace (defaults to runtime environment or cwd)"
    )
    parser.add_argument("--dotenv", help="Explicit dotenv file")
    commands = parser.add_subparsers(dest="command", required=True)
    for name, description in COMMANDS.items():
        command = commands.add_parser(name, help=description, description=description)
        command.add_argument("--workspace", default=argparse.SUPPRESS)
        command.add_argument("--dotenv", default=argparse.SUPPRESS)
        if name not in ("panel", "execute-current-run"):
            command.add_argument(
                "--reference-time",
                help=(
                    "Report reference time; defaults to the selected current run"
                    if name == "generate-report"
                    else "ISO time or 'now'; defaults to run.reference_time"
                ),
            )
        if name in ("execute-current-run", "run-all"):
            command.add_argument(
                "--executable", help="MGB executable, relative to workspace or absolute"
            )
        if name == "panel":
            command.add_argument("--address", default="localhost")
            command.add_argument("--port", type=_port, default=5006)
            command.add_argument("--no-show", action="store_true", help="Do not open a browser")
        if name == "generate-report":
            command.add_argument("--history-db", help="History SQLite path, relative to workspace")
            command.add_argument("--spatial-gpkg", help="Mini-catchment GeoPackage path")
            command.add_argument("--run-artifact", help="Current-run SQLite artifact path")
            command.add_argument("--cache-dir", help="Scenario cache root path")
            command.add_argument("--output-dir", required=True, help="Directory for the generated report")
            command.add_argument("--output-basin-name", required=True, help="Filename basin label")
            command.add_argument("--format", choices=("html", "pdf"), default="html")
            command.add_argument("--outlet-mini-id", required=True, type=int)
            command.add_argument("--map-station-ids", nargs="+", required=True)
            command.add_argument("--chart-station-ids", nargs="+", required=True)
            command.add_argument("--markdown", help="UTF-8 Markdown summary file")
            command.add_argument("--title", required=True)
            command.add_argument("--author", required=True)
            command.add_argument("--basin-name", required=True)
            command.add_argument("--window-start", help="ISO window start")
            command.add_argument("--window-end", help="ISO exclusive window end")
            command.add_argument("--scenario-ids", nargs="+", help="Ordered canonical scenario IDs")
            command.add_argument("--table-columns", help="JSON file with ordered numeric table column specs")
    return parser


def _port(value: str) -> int:
    port = int(value)
    if not 1 <= port <= 65535:
        raise argparse.ArgumentTypeError("port must be between 1 and 65535")
    return port


def _context(args: argparse.Namespace) -> RuntimeContext:
    context = build_runtime_context(workspace=args.workspace, dotenv_path=args.dotenv)
    if args.command in ("panel", "execute-current-run", "generate-report"):
        return context
    settings = deepcopy(context.settings)
    reference = resolve_reference_time(
        args.reference_time or str(settings["run"]["reference_time"])
    )
    settings["run"]["reference_time"] = reference.isoformat(timespec="seconds")
    # Validate alignment before starting any download or database mutation.
    build_horizon_window(
        reference, days_before=0, timestep_hours=int(settings["run"]["timestep_hours"])
    )
    return RuntimeContext(context.paths, settings, context.env)


def resolve_executable(
    context: RuntimeContext,
    override: str | None = None,
    artifact_settings: dict[str, object] | None = None,
) -> Path:
    if override:
        return context.paths.resolve_path(override)
    artifact_executable = (artifact_settings or {}).get("executable_path")
    if artifact_executable:
        return context.paths.resolve_path(str(artifact_executable))
    standard = context.paths.mgb_executable_path
    return standard if standard.exists() else context.paths.mgb_runner_dir / "MGB_Inercial_Prev_RSH"


def runtime(context: RuntimeContext) -> None:
    settings, paths = context.settings, context.paths
    reference = resolve_reference_time(str(settings["run"]["reference_time"]))
    timestep = int(settings["run"]["timestep_hours"])
    fetch = build_horizon_window(
        reference, days_before=int(settings["ingest"]["request_days"]) - 1, timestep_hours=timestep
    )
    model = build_horizon_window(
        reference,
        days_before=int(settings["mgb"]["observed_horizon_days"]),
        horizon_days=int(settings["mgb"]["forecast_horizon_days"]),
        timestep_hours=timestep,
    )
    for label, value in (
        ("workspace", paths.workspace),
        ("history db", paths.history_db),
        ("current run artifact", paths.current_run_db),
        ("MGB input dir", paths.mgb_input_dir),
        ("MGB output dir", paths.mgb_output_dir),
        ("MGB executable", resolve_executable(context)),
        ("reference time", reference.isoformat()),
        ("forecast cycle UTC", resolve_forecast_cycle(reference).isoformat()),
        ("timestep hours", timestep),
        ("fetch start", fetch.start_time.isoformat()),
        ("MGB input start", model.start_time.isoformat()),
        ("forecast start", model.forecast_start_time.isoformat()),
        ("forecast timesteps", model.forecast_nt),
        ("total MGB timesteps", model.nt),
        ("scenario cache root", paths.cache_dir / "forecast_scenarios"),
    ):
        print(f"{label}: {value}")


def download_observed(context: RuntimeContext) -> None:
    from mgb_ops.assets.databases import initialize_history_db
    from mgb_ops.assets.schemas import SQL_DIR
    from mgb_ops.workflows import download_observed_data, ingest_from_csv

    paths = context.paths
    paths.ensure_standard_dirs()
    initialize_history_db(
        paths.history_db,
        paths.station_inventory_csv_path,
        SQL_DIR / "history_schema.sql",
        paths.station_level_reference_csv_path,
        paths.registered_floods_csv_path,
    )
    summary = download_observed_data(
        context,
        ("ana", "inmet"),
        reference_time=resolve_reference_time(str(context.settings["run"]["reference_time"])),
    )
    for path in summary.normalized_csv_paths:
        imported = ingest_from_csv(context, path, state="raw")
        print(
            f"loaded {path}: read {imported.rows_total} rows; imported {imported.rows_imported} rows"
        )
    print(f"observed download complete: {len(summary.normalized_csv_paths)} files")


def download_forecast(context: RuntimeContext) -> None:
    from mgb_ops.workflows import update_enabled_forecast_providers

    context.paths.ensure_standard_dirs()
    summary = update_enabled_forecast_providers(
        context,
        reference_time=resolve_reference_time(str(context.settings["run"]["reference_time"])),
    )
    for path in summary.new_asset_paths:
        print(f"registered forecast asset: {path}")
    for path in summary.reused_asset_paths:
        print(f"reused forecast asset: {path}")


def build_current_run(context: RuntimeContext) -> None:
    from mgb_ops.analysis.windows import build_analysis_window
    from mgb_ops.assets.history import HistoryRepository
    from mgb_ops.workflows.scenarios import build_current_artifact

    paths, settings = context.paths, context.settings
    paths.ensure_standard_dirs()
    with HistoryRepository(paths.history_db) as repository:
        providers = tuple(
            str(row["provider_code"]) for row in repository.list_enabled_providers("observed")
        )
    if not providers:
        raise RuntimeError("No enabled observed providers were found.")
    reference = resolve_reference_time(str(settings["run"]["reference_time"]))
    window = build_analysis_window(
        reference,
        observed_horizon_days=int(settings["mgb"]["observed_horizon_days"]),
        forecast_horizon_days=int(settings["mgb"]["forecast_horizon_days"]),
    )
    build_current_artifact(
        paths.current_run_db,
        paths.history_db,
        paths.workspace,
        reference_time=reference.isoformat(timespec="seconds"),
        forecast_end_exclusive=window.forecast_end_exclusive.isoformat(timespec="seconds"),
        timestep_hours=int(settings["run"]["timestep_hours"]),
        mgb_settings=settings["mgb"],
        spatial_settings=settings["spatial_grid"],
        interpolation_settings=settings["rainfall_interpolation"],
        review_window={
            "start": window.start_time.isoformat(timespec="seconds"),
            "end": window.cutoff_time.isoformat(timespec="seconds"),
        },
        observed_providers=providers,
        precipitation_qc_settings=settings["precipitation_qc"],
    )
    print(f"created current run artifact: {paths.current_run_db}")


def execute_current_run(context: RuntimeContext, executable: str | None = None) -> None:
    from mgb_ops.assets.current_run import validate_current_artifact
    from mgb_ops.workflows import execute_current_artifact

    artifact_path = context.paths.current_run_db
    if not artifact_path.is_file():
        raise FileNotFoundError(f"Current artifact does not exist: {artifact_path}")
    artifact = validate_current_artifact(artifact_path)
    executable_path = resolve_executable(context, executable, artifact.mgb_settings)
    if not executable_path.is_file():
        raise FileNotFoundError(f"MGB executable not found: {executable_path}")
    print(f"executing current run artifact: {artifact_path}")
    print(f"reference time: {artifact.reference_time}")
    for scenario in artifact.scenarios:
        print(f"scenario {scenario.position}: {scenario.scenario_id} | {scenario.label}")
    result = execute_current_artifact(
        context, executable_path=executable_path, execution_env=os.environ.copy()
    )
    print(f"published scenario batch: {result.batch_id}")
    print(f"scenario cache directory: {result.cache_dir}")
    for item in result.results:
        print(f"- {item.scenario.scenario_id}: {item.cache_path}")


def serve_panel(context: RuntimeContext, args: argparse.Namespace) -> None:
    import panel as pn
    from apps.ops_dashboard import create_dashboard

    pn.serve(
        lambda: create_dashboard(context.paths.workspace, context=context),
        title="Operational Hydrology",
        address=args.address,
        port=args.port,
        show=not args.no_show,
    )


def generate_report_command(context: RuntimeContext, args: argparse.Namespace) -> None:
    from mgb_ops.workflows.reporting import generate_report as build_report

    paths = context.paths
    spatial_path = args.spatial_gpkg or str(context.settings["spatial"]["gpkg_path"])
    table_columns = None
    if args.table_columns:
        value = json.loads(paths.resolve_path(args.table_columns).read_text(encoding="utf-8"))
        if not isinstance(value, list):
            raise ValueError("--table-columns must point to a JSON array.")
        table_columns = value
    result = build_report(
        history_path=paths.resolve_path(args.history_db or paths.history_db),
        spatial_path=paths.resolve_path(spatial_path),
        run_artifact_path=paths.resolve_path(args.run_artifact or paths.current_run_db),
        cache_dir=paths.resolve_path(args.cache_dir or paths.cache_dir),
        output_dir=paths.resolve_path(args.output_dir),
        output_basin_name=args.output_basin_name,
        format=args.format,
        outlet_mini_id=args.outlet_mini_id,
        map_station_ids=args.map_station_ids,
        chart_station_ids=args.chart_station_ids,
        markdown_path=paths.resolve_path(args.markdown) if args.markdown else None,
        title=args.title,
        author=args.author,
        basin_name=args.basin_name,
        reference_time=args.reference_time,
        window_start=args.window_start,
        window_end=args.window_end,
        scenarios=args.scenario_ids,
        table_columns=table_columns,
    )
    print(json.dumps({
        "path": str(result.path),
        "format": result.format,
        "warnings": [asdict(item) for item in result.warnings],
    }, ensure_ascii=False))


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        context = _context(args)
        if args.command == "runtime":
            runtime(context)
        elif args.command == "download-observed":
            download_observed(context)
        elif args.command == "download-forecast":
            download_forecast(context)
        elif args.command == "build-current-run":
            build_current_run(context)
        elif args.command == "execute-current-run":
            execute_current_run(context, args.executable)
        elif args.command == "generate-report":
            generate_report_command(context, args)
        elif args.command == "run-all":
            download_observed(context)
            download_forecast(context)
            build_current_run(context)
            execute_current_run(context, args.executable)
        else:
            serve_panel(context, args)
        return 0
    except KeyboardInterrupt:
        print("mgb-ops: interrupted", file=sys.stderr)
        return 130
    except ModuleNotFoundError as exc:
        extras = (
            "dashboard" if args.command == "panel"
            else "report" if args.command == "generate-report"
            else "data,forecast,geo"
        )
        print(
            f"mgb-ops: missing module {exc.name!r}; install with pip install 'mgb-ops[{extras}]'",
            file=sys.stderr,
        )
        return 1
    except Exception as exc:
        print(f"mgb-ops: {exc}", file=sys.stderr)
        return 1
