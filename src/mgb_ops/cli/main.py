from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

from common.paths import (
    SQL_DIR,
    ensure_standard_dirs,
    runtime_paths,
    set_workspace,
)
from common.settings import load_settings
from common.time_utils import resolve_reference_time

DEFAULT_ANA_BASE_URL = "http://telemetriaws1.ana.gov.br/serviceana.asmx/DadosHidrometeorologicos"
DEFAULT_INMET_BASE_URL = "https://api-bndmet.decea.mil.br/v1"
RAINFALL_DEFAULT_CHUNK_HOURS = 720
EXPORT_DEFAULT_CHUNK_HOURS = 720


def _print_json(value: object) -> None:
    print(json.dumps(value, indent=2, default=str))


def _settings(args: argparse.Namespace) -> dict[str, object]:
    return load_settings(workspace=args.workspace, require_custom=False)


def cmd_bootstrap_history(args: argparse.Namespace) -> int:
    from storage.db_bootstrap import initialize_history_db

    paths = runtime_paths(args.workspace)
    ensure_standard_dirs(args.workspace)
    target = args.history_path or paths.history_db
    inventory = args.inventory_csv or paths.interim_dir / "history_station_inventory.csv"
    print(initialize_history_db(target, inventory))
    return 0


def cmd_bootstrap_run(args: argparse.Namespace) -> int:
    from storage.db_bootstrap import initialize_run_db

    ensure_standard_dirs(args.workspace)
    print(initialize_run_db(args.run_id, args.run_path))
    return 0


def cmd_ingest_ana(args: argparse.Namespace) -> int:
    from ingest.fetch_observed_ana import ingest_observed_ana

    paths = runtime_paths(args.workspace)
    settings = _settings(args)
    ingest_settings = settings["ingest"]
    reference_time = resolve_reference_time(settings["run"]["reference_time"])
    summary = ingest_observed_ana(
        args.history_db or paths.history_db,
        base_url=args.base_url,
        reference_time=reference_time,
        request_days=int(args.request_days or ingest_settings["request_days"]),
        timeout_seconds=float(args.timeout_seconds or ingest_settings["timeout_seconds"]),
        station_codes=args.station_code,
        interim_dir=paths.interim_dir,
        logs_dir=paths.logs_dir,
    )
    _print_json(summary)
    return 0


def cmd_ingest_inmet(args: argparse.Namespace) -> int:
    from ingest.fetch_observed_inmet import ingest_observed_inmet

    paths = runtime_paths(args.workspace)
    settings = _settings(args)
    ingest_settings = settings["ingest"]
    reference_time = resolve_reference_time(settings["run"]["reference_time"])
    summary = ingest_observed_inmet(
        args.history_db or paths.history_db,
        reference_time=reference_time,
        request_days=int(args.request_days or ingest_settings["request_days"]),
        timeout_seconds=float(args.timeout_seconds or ingest_settings["timeout_seconds"]),
        station_codes=args.station_code,
        interim_dir=paths.interim_dir,
        logs_dir=paths.logs_dir,
        base_url=args.base_url,
    )
    _print_json(summary)
    return 0


def cmd_ingest_forecast_grid(args: argparse.Namespace) -> int:
    from ingest.forecast_grid import ingest_forecast_grids

    paths = runtime_paths(args.workspace)
    settings = _settings(args)
    reference_time = resolve_reference_time(settings["run"]["reference_time"])
    summary = ingest_forecast_grids(
        args.history_db or paths.history_db,
        reference_time=reference_time,
        interim_dir=paths.interim_dir,
        logs_dir=paths.logs_dir,
    )
    _print_json(summary.__dict__)
    return 0


def cmd_model_prepare_meta(args: argparse.Namespace) -> int:
    from model.prepare_mgb_meta import rewrite_mgb_meta_from_config

    paths = runtime_paths(args.workspace)
    summary = rewrite_mgb_meta_from_config(
        parhig_path=args.parhig or paths.mgb_input_dir / "PARHIG.hig",
        logs_dir=paths.logs_dir,
        workspace=args.workspace,
    )
    _print_json(summary.__dict__)
    return 0


def cmd_model_prepare_rainfall(args: argparse.Namespace) -> int:
    from model.prepare_mgb_rainfall import prepare_mgb_rainfall

    paths = runtime_paths(args.workspace)
    settings = _settings(args)
    rainfall_settings = settings["rainfall_interpolation"]
    summary = prepare_mgb_rainfall(
        history_db=args.history_db or paths.history_db,
        parhig_path=args.parhig or paths.mgb_input_dir / "PARHIG.hig",
        mini_gtp_path=args.mini_gtp or paths.mgb_input_dir / "MINI.gtp",
        output_path=args.output or paths.mgb_input_dir / "chuvabin.hig",
        nearest_stations=int(args.nearest_stations or rainfall_settings["nearest_stations"]),
        power=float(args.power or rainfall_settings["power"]),
        chunk_hours=int(args.chunk_hours),
        logs_dir=paths.logs_dir,
        workspace=args.workspace,
    )
    _print_json(summary.__dict__)
    return 0


def cmd_model_run(args: argparse.Namespace) -> int:
    from model.mgb_execution import execute_mgb_plan, prepare_mgb_execution
    from model.run_mgb import build_run_metadata, build_summary

    paths = runtime_paths(args.workspace)
    metadata = build_run_metadata()
    plan = prepare_mgb_execution(
        metadata,
        executable_path=str(args.executable or paths.mgb_executable_path),
        input_dir=str(args.input_dir or paths.mgb_input_dir),
        output_dir=str(args.output_dir or paths.mgb_output_dir),
        workspace_root=str(args.remote_workspace or paths.remote_workspace_root),
    )
    result = execute_mgb_plan(plan, dry_run=args.dry_run, logs_dir=paths.logs_dir)
    _print_json(build_summary(plan, result, dry_run=args.dry_run))
    return 0


def cmd_model_export_outputs(args: argparse.Namespace) -> int:
    from model.export_mgb_outputs import export_mgb_outputs

    paths = runtime_paths(args.workspace)
    summary = export_mgb_outputs(
        parhig_path=args.parhig or paths.mgb_input_dir / "PARHIG.hig",
        mini_gtp_path=args.mini_gtp or paths.mgb_input_dir / "MINI.gtp",
        output_dir=args.output_dir or paths.mgb_output_dir,
        output_db_path=args.output_db or paths.interim_dir / "model_outputs.sqlite",
        schema_path=SQL_DIR / "model_outputs_schema.sql",
        chunk_hours=args.chunk_hours,
        logs_dir=paths.logs_dir,
        workspace=args.workspace,
    )
    _print_json(summary.__dict__)
    return 0


def cmd_dashboard(args: argparse.Namespace) -> int:
    app_spec = importlib.util.find_spec("ops_dashboard.app")
    if app_spec is None or app_spec.origin is None:
        raise RuntimeError("Could not locate ops_dashboard.app in the installed environment.")
    app_path = Path(app_spec.origin)
    command = ["streamlit", "run", str(app_path), "--", "--workspace", str(runtime_paths(args.workspace).workspace)]
    if not args.launch:
        print(" ".join(command))
        return 0
    env = os.environ.copy()
    env["MGB_OPS_WORKSPACE"] = str(runtime_paths(args.workspace).workspace)
    return subprocess.call(command, env=env)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="mgb-ops", description="MGB operational CLI.")
    parser.add_argument(
        "--workspace",
        type=Path,
        default=None,
        help="Regional workspace containing data/, logs/, and mgb_runner/. Defaults to MGB_OPS_WORKSPACE then cwd.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    bootstrap = subparsers.add_parser("bootstrap", help="Initialize SQLite stores.")
    bootstrap_sub = bootstrap.add_subparsers(dest="bootstrap_command", required=True)
    history = bootstrap_sub.add_parser("history", help="Initialize the history database.")
    history.add_argument("--history-path", type=Path, default=None)
    history.add_argument("--inventory-csv", type=Path, default=None)
    history.set_defaults(func=cmd_bootstrap_history)
    run = bootstrap_sub.add_parser("run", help="Initialize a run database.")
    run.add_argument("--run-id", required=True)
    run.add_argument("--run-path", type=Path, default=None)
    run.set_defaults(func=cmd_bootstrap_run)

    ingest = subparsers.add_parser("ingest", help="Ingest observed and forecast data.")
    ingest_sub = ingest.add_subparsers(dest="ingest_command", required=True)
    ana = ingest_sub.add_parser("ana", help="Ingest ANA observations.")
    ana.add_argument("--history-db", type=Path, default=None)
    ana.add_argument("--base-url", default=DEFAULT_ANA_BASE_URL)
    ana.add_argument("--request-days", type=int, default=None)
    ana.add_argument("--timeout-seconds", type=float, default=None)
    ana.add_argument("--station-code", action="append", default=None)
    ana.set_defaults(func=cmd_ingest_ana)
    inmet = ingest_sub.add_parser("inmet", help="Ingest INMET observations.")
    inmet.add_argument("--history-db", type=Path, default=None)
    inmet.add_argument("--base-url", default=DEFAULT_INMET_BASE_URL)
    inmet.add_argument("--request-days", type=int, default=None)
    inmet.add_argument("--timeout-seconds", type=float, default=None)
    inmet.add_argument("--station-code", action="append", default=None)
    inmet.set_defaults(func=cmd_ingest_inmet)
    forecast = ingest_sub.add_parser("forecast-grid", help="Ingest ECMWF forecast grids.")
    forecast.add_argument("--history-db", type=Path, default=None)
    forecast.set_defaults(func=cmd_ingest_forecast_grid)

    model = subparsers.add_parser("model", help="Prepare, run, and export MGB data.")
    model_sub = model.add_subparsers(dest="model_command", required=True)
    meta = model_sub.add_parser("prepare-meta", help="Rewrite PARHIG timing metadata.")
    meta.add_argument("--parhig", type=Path, default=None)
    meta.set_defaults(func=cmd_model_prepare_meta)
    rainfall = model_sub.add_parser("prepare-rainfall", help="Build chuvabin.hig.")
    rainfall.add_argument("--history-db", type=Path, default=None)
    rainfall.add_argument("--parhig", type=Path, default=None)
    rainfall.add_argument("--mini-gtp", type=Path, default=None)
    rainfall.add_argument("--output", type=Path, default=None)
    rainfall.add_argument("--nearest-stations", type=int, default=None)
    rainfall.add_argument("--power", type=float, default=None)
    rainfall.add_argument("--chunk-hours", type=int, default=RAINFALL_DEFAULT_CHUNK_HOURS)
    rainfall.set_defaults(func=cmd_model_prepare_rainfall)
    run_model = model_sub.add_parser("run", help="Run the MGB executable.")
    run_model.add_argument("--dry-run", action="store_true")
    run_model.add_argument("--executable", type=Path, default=None)
    run_model.add_argument("--input-dir", type=Path, default=None)
    run_model.add_argument("--output-dir", type=Path, default=None)
    run_model.add_argument("--remote-workspace", type=Path, default=None)
    run_model.set_defaults(func=cmd_model_run)
    export = model_sub.add_parser("export-outputs", help="Export MGB binary outputs to SQLite.")
    export.add_argument("--parhig", type=Path, default=None)
    export.add_argument("--mini-gtp", type=Path, default=None)
    export.add_argument("--output-dir", type=Path, default=None)
    export.add_argument("--output-db", type=Path, default=None)
    export.add_argument("--chunk-hours", type=int, default=EXPORT_DEFAULT_CHUNK_HOURS)
    export.set_defaults(func=cmd_model_export_outputs)

    dashboard = subparsers.add_parser("dashboard", help="Show or launch the Streamlit dashboard.")
    dashboard.add_argument("--launch", action="store_true", help="Launch Streamlit instead of printing the command.")
    dashboard.set_defaults(func=cmd_dashboard)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    args.workspace = set_workspace(args.workspace)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
