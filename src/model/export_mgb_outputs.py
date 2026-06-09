from __future__ import annotations

import logging
import re
import sqlite3
import sys
from dataclasses import dataclass
from datetime import date, datetime, time, timedelta
from pathlib import Path
from uuid import uuid4

import numpy as np

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = REPO_ROOT / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

from common.paths import SQL_DIR, interim_dir, logs_dir as default_logs_dir, mgb_input_dir, mgb_output_dir
from common.settings import load_settings
from common.time_utils import resolve_reference_time


DEFAULT_PARHIG = mgb_input_dir() / "PARHIG.hig"
DEFAULT_MINI_GTP = mgb_input_dir() / "MINI.gtp"
DEFAULT_OUTPUT_DIR = mgb_output_dir()
DEFAULT_OUTPUT_DB = interim_dir() / "model_outputs.sqlite"
DEFAULT_SCHEMA_PATH = SQL_DIR / "model_outputs_schema.sql"
DEFAULT_CHUNK_HOURS = 720
NUMBER_PATTERN = re.compile(r"[-+]?\d+(?:[.,]\d+)?")
LOGGER_NAME = "floodqc.model.export_mgb_outputs"


@dataclass(frozen=True, slots=True)
class VariableSpec:
    source_filename: str
    variable_code: str
    display_name: str
    unit: str


@dataclass(frozen=True, slots=True)
class OutputSource:
    variable_code: str
    path: Path
    nt_total: int


@dataclass(frozen=True, slots=True)
class ExportWindow:
    reference_time: datetime
    reference_date: date
    window_start: datetime
    window_end_exclusive: datetime


@dataclass(frozen=True, slots=True)
class ExportSummary:
    database_path: Path
    reference_time: datetime
    window_start: datetime
    window_end_exclusive: datetime
    nc: int
    nt_current: int
    nt_forecast: int
    series_count: int
    value_count: int


VARIABLE_SPECS = (
    VariableSpec(source_filename="QTUDO_Inercial_Atual.MGB", variable_code="q", display_name="QTUDO", unit="m3/s"),
    VariableSpec(source_filename="YTUDO.MGB", variable_code="y", display_name="YTUDO", unit="m"),
)


def script_stem() -> str:
    return Path(__file__).stem


def build_execution_id() -> str:
    return datetime.now().strftime("%Y%m%dT%H%M%S")


def prev_flag_label(prev_flag: int) -> str:
    return "sim" if prev_flag == 0 else "for"


def configure_run_logger(log_file: Path) -> logging.Logger:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger(LOGGER_NAME)
    logger.setLevel(logging.INFO)
    for handler in logger.handlers[:]:
        handler.close()
        logger.removeHandler(handler)
    logger.propagate = False

    formatter = logging.Formatter("%(asctime)s | %(levelname)s | %(message)s")

    file_handler = logging.FileHandler(log_file, mode="w", encoding="utf-8")
    file_handler.setFormatter(formatter)
    logger.addHandler(file_handler)

    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    logger.addHandler(stream_handler)
    return logger


def _extract_numbers(text: str) -> list[str]:
    return NUMBER_PATTERN.findall(text)


def _next_data_line(lines: list[str], start_idx: int) -> str:
    for raw_line in lines[start_idx + 1 :]:
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("!"):
            continue
        return raw_line
    raise ValueError("Could not find a data line after the header line.")


def _first_int(text: str) -> int:
    numbers = _extract_numbers(text)
    if not numbers:
        raise ValueError(f"No integer found in line: {text!r}")
    return int(float(numbers[0].replace(",", ".")))


def _isoformat_seconds(value: datetime) -> str:
    return value.isoformat(timespec="seconds")


def _ceil_div(numerator: int, denominator: int) -> int:
    return -(-numerator // denominator)


def build_output_series_id(mini_id: int, variable_code: str, prev_flag: int) -> str:
    output_type = "sim" if prev_flag == 0 else "for"
    return f"{mini_id:04d}.{variable_code}.{output_type}"


def apply_schema(database_path: Path, schema_path: Path) -> None:
    database_path.parent.mkdir(parents=True, exist_ok=True)
    schema_sql = schema_path.read_text(encoding="utf-8")
    connection = sqlite3.connect(database_path)
    try:
        connection.executescript(schema_sql)
        connection.commit()
    finally:
        connection.close()


def read_nc_from_parhig(parhig_path: Path) -> int:
    lines = parhig_path.read_text(encoding="latin-1").splitlines()
    for idx, raw_line in enumerate(lines):
        upper = raw_line.upper()
        if "NC" in upper and "NU" in upper:
            return _first_int(_next_data_line(lines, idx))
    raise ValueError(f"Could not read NC from {parhig_path}")


def read_time_settings_from_parhig(parhig_path: Path) -> tuple[datetime, int]:
    lines = parhig_path.read_text(encoding="latin-1").splitlines()
    start_time: datetime | None = None
    dt_seconds: int | None = None

    for idx, raw_line in enumerate(lines):
        upper = raw_line.upper()

        if start_time is None and all(token in upper for token in ("DIA", "MES", "ANO", "HORA")):
            numbers = _extract_numbers(_next_data_line(lines, idx))
            if len(numbers) >= 4:
                day = int(float(numbers[0].replace(",", ".")))
                month = int(float(numbers[1].replace(",", ".")))
                year = int(float(numbers[2].replace(",", ".")))
                hour = int(float(numbers[3].replace(",", ".")))
                start_time = datetime(year, month, day, hour)

        if dt_seconds is None and "NT" in upper and "DT" in upper:
            numbers = _extract_numbers(_next_data_line(lines, idx))
            if len(numbers) >= 2:
                dt_seconds = int(float(numbers[1].replace(",", ".")))

        if start_time is not None and dt_seconds is not None:
            break

    if start_time is None or dt_seconds is None:
        raise ValueError(
            f"Could not read start_time/dt_seconds from {parhig_path}. "
            "Expected PARHIG to provide DIA/MES/ANO/HORA and NT/DT."
        )
    if dt_seconds <= 0:
        raise ValueError(f"dt_seconds must be > 0, got {dt_seconds}")
    return start_time, dt_seconds


def read_mini_ids(mini_gtp_path: Path, *, nc: int) -> list[int]:
    header: list[str] | None = None
    mini_column_index: int | None = None
    mini_ids: list[int] = []

    with mini_gtp_path.open("r", encoding="latin-1") as handle:
        for raw_line in handle:
            stripped = raw_line.strip()
            if not stripped:
                continue

            parts = stripped.split()
            if header is None:
                header = parts
                if "Mini" not in header:
                    raise ValueError(f"MINI.gtp missing required column 'Mini': {mini_gtp_path}")
                mini_column_index = header.index("Mini")
                continue

            assert mini_column_index is not None
            if len(parts) <= mini_column_index:
                raise ValueError(f"Invalid MINI.gtp row: {raw_line.rstrip()}")
            mini_ids.append(int(float(parts[mini_column_index].replace(",", "."))))
            if len(mini_ids) == nc:
                break

    if len(mini_ids) < nc:
        raise ValueError(f"MINI.gtp has {len(mini_ids)} rows, smaller than NC={nc}")

    seen: set[int] = set()
    duplicated: list[int] = []
    for mini_id in mini_ids:
        if mini_id in seen and mini_id not in duplicated:
            duplicated.append(mini_id)
        seen.add(mini_id)
    if duplicated:
        sample = ", ".join(str(value) for value in duplicated[:5])
        raise ValueError(f"MINI.gtp has duplicated Mini ids (sample: {sample})")

    return mini_ids


def infer_nt_from_binary(file_path: Path, *, nc: int) -> int:
    size_bytes = file_path.stat().st_size
    if size_bytes % 4 != 0:
        raise ValueError(
            f"Invalid binary size in {file_path.name}: {size_bytes} bytes is not divisible by 4 (float32)."
        )
    total_floats = size_bytes // 4
    if total_floats % nc != 0:
        raise ValueError(
            f"Invalid shape in {file_path.name}: {total_floats} float32 values are not divisible by NC={nc}."
        )
    nt = total_floats // nc
    if nt <= 0:
        raise ValueError(f"Invalid NT inferred for {file_path.name}: NT={nt}")
    return int(nt)


def discover_output_sources(output_dir: Path, *, nc: int) -> dict[str, OutputSource]:
    sources: dict[str, OutputSource] = {}
    logger = logging.getLogger(LOGGER_NAME)

    for spec in VARIABLE_SPECS:
        source_path = output_dir / spec.source_filename
        if not source_path.exists():
            raise FileNotFoundError(
                f"Required output file for variable_code={spec.variable_code!r} was not found: {source_path}"
            )
        if not source_path.is_file():
            raise FileNotFoundError(
                f"Required output path for variable_code={spec.variable_code!r} is not a file: {source_path}"
            )

        source = OutputSource(
            variable_code=spec.variable_code,
            path=source_path,
            nt_total=infer_nt_from_binary(source_path, nc=nc),
        )
        sources[spec.variable_code] = source
        logger.info(
            "source variable=%s filename=%s path=%s nt_total=%s",
            spec.variable_code,
            spec.source_filename,
            source.path,
            source.nt_total,
        )

    return sources


def validate_source_lengths(sources: dict[str, OutputSource]) -> tuple[dict[str, OutputSource], int]:
    nt_values = {variable_code: source.nt_total for variable_code, source in sources.items()}
    nt_set = set(nt_values.values())
    if len(nt_set) != 1:
        raise ValueError(f"Inconsistent NT across outputs: {nt_values}")
    return sources, nt_set.pop()


def build_export_window(reference_time: datetime, *, output_days_before: int, forecast_horizon_days: int) -> ExportWindow:
    reference_date = reference_time.date()
    window_start = datetime.combine(reference_date - timedelta(days=output_days_before), time.min)
    window_end_exclusive = datetime.combine(reference_date + timedelta(days=forecast_horizon_days + 1), time.min)
    return ExportWindow(
        reference_time=reference_time,
        reference_date=reference_date,
        window_start=window_start,
        window_end_exclusive=window_end_exclusive,
    )


def compute_global_row_bounds(
    *,
    start_time: datetime,
    dt_seconds: int,
    window_start: datetime,
    window_end_exclusive: datetime,
    total_nt: int,
) -> tuple[int, int]:
    start_delta_seconds = int((window_start - start_time).total_seconds())
    end_delta_seconds = int((window_end_exclusive - start_time).total_seconds())
    start_offset = max(0, _ceil_div(start_delta_seconds, dt_seconds))
    end_offset = min(total_nt, _ceil_div(end_delta_seconds, dt_seconds))
    if start_offset >= end_offset:
        raise ValueError("Configured window does not intersect available MGB outputs.")
    return start_offset, end_offset


def build_series_rows(
    mini_ids: list[int],
) -> tuple[list[tuple[str, str, int, int, str]], dict[tuple[str, int], dict[int, str]]]:
    rows: list[tuple[str, str, int, int, str]] = []
    lookup: dict[tuple[str, int], dict[int, str]] = {}

    for spec in VARIABLE_SPECS:
        for prev_flag in (0, 1):
            mapping: dict[int, str] = {}
            for mini_id in mini_ids:
                series_id = build_output_series_id(mini_id, spec.variable_code, prev_flag)
                rows.append((series_id, spec.variable_code, mini_id, prev_flag, spec.unit))
                mapping[mini_id] = series_id
            lookup[(spec.variable_code, prev_flag)] = mapping

    return rows, lookup


def iter_value_rows(
    values_chunk: np.ndarray,
    *,
    dt_values: list[str],
    mini_ids: list[int],
    series_ids_by_mini: dict[int, str],
):
    for row_index, dt_value in enumerate(dt_values):
        row = values_chunk[row_index, :]
        for column_index, mini_id in enumerate(mini_ids):
            raw_value = float(row[column_index])
            value = raw_value if np.isfinite(raw_value) else None
            yield (series_ids_by_mini[mini_id], dt_value, value)


def load_output_window_from_settings(*, workspace: str | Path | None = None) -> tuple[int, int]:
    settings = load_settings(workspace=workspace, require_custom=False if workspace is not None else None)
    mgb_settings = settings["mgb"]
    return int(mgb_settings["output_days_before"]), int(mgb_settings["forecast_horizon_days"])


def load_simulation_reference_time_from_settings(*, workspace: str | Path | None = None) -> datetime:
    settings = load_settings(workspace=workspace, require_custom=False if workspace is not None else None)
    return resolve_reference_time(settings["run"]["reference_time"])


def compute_nt_current(
    *,
    start_time: datetime,
    dt_seconds: int,
    reference_time: datetime,
    nt_total: int,
) -> tuple[int, int]:
    if reference_time < start_time:
        raise ValueError(
            f"Configured reference_time {reference_time.isoformat(timespec='seconds')} "
            f"is before the available output start {start_time.isoformat(timespec='seconds')}."
        )

    delta_seconds = int((reference_time - start_time).total_seconds())
    if delta_seconds % dt_seconds != 0:
        raise ValueError(
            f"Configured reference_time {reference_time.isoformat(timespec='seconds')} "
            f"is not aligned to dt_seconds={dt_seconds}."
        )

    nt_current = delta_seconds // dt_seconds + 1
    if nt_current < 1:
        raise ValueError(f"Invalid nt_current computed from reference_time={reference_time.isoformat(timespec='seconds')}.")
    if nt_current > nt_total:
        last_available_time = start_time + timedelta(seconds=(nt_total - 1) * dt_seconds)
        raise ValueError(
            f"Configured reference_time {reference_time.isoformat(timespec='seconds')} "
            f"exceeds the available output end {last_available_time.isoformat(timespec='seconds')}."
        )

    nt_forecast = nt_total - nt_current
    if nt_forecast < 0:
        raise ValueError(f"Invalid nt_forecast computed from nt_total={nt_total} and nt_current={nt_current}.")
    return nt_current, nt_forecast

def write_output_database(
    *,
    database_path: Path,
    schema_path: Path,
    mini_ids: list[int],
    sources: dict[str, OutputSource],
    start_time: datetime,
    dt_seconds: int,
    export_window: ExportWindow,
    nt_current: int,
    nt_forecast: int,
    chunk_hours: int,
) -> ExportSummary:
    logger = logging.getLogger(LOGGER_NAME)
    apply_schema(database_path, schema_path)
    logger.info("schema_applied database=%s", database_path)

    total_nt = nt_current + nt_forecast
    global_start_offset, global_end_offset = compute_global_row_bounds(
        start_time=start_time,
        dt_seconds=dt_seconds,
        window_start=export_window.window_start,
        window_end_exclusive=export_window.window_end_exclusive,
        total_nt=total_nt,
    )
    logger.info(
        "window_bounds global_start=%s global_end=%s window_start=%s window_end_exclusive=%s",
        global_start_offset,
        global_end_offset,
        _isoformat_seconds(export_window.window_start),
        _isoformat_seconds(export_window.window_end_exclusive),
    )

    series_rows, series_lookup = build_series_rows(mini_ids)
    value_count = 0
    connection = sqlite3.connect(database_path)
    try:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.execute(
            "INSERT INTO metadata (reference_time, reference_date, window_start, window_end_exclusive, dt_seconds, nc, nt_current, nt_forecast) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                _isoformat_seconds(export_window.reference_time),
                export_window.reference_date.isoformat(),
                _isoformat_seconds(export_window.window_start),
                _isoformat_seconds(export_window.window_end_exclusive),
                dt_seconds,
                len(mini_ids),
                nt_current,
                nt_forecast,
            ),
        )
        connection.executemany(
            "INSERT INTO variable (variable_code, display_name, unit) VALUES (?, ?, ?)",
            [(spec.variable_code, spec.display_name, spec.unit) for spec in VARIABLE_SPECS],
        )
        connection.executemany(
            "INSERT INTO output_series (series_id, variable_code, mini_id, prev_flag, unit) VALUES (?, ?, ?, ?, ?)",
            series_rows,
        )
        logger.info("series_inserted count=%s", len(series_rows))

        for spec in VARIABLE_SPECS:
            source = sources[spec.variable_code]
            overlap_start = global_start_offset
            overlap_end = min(global_end_offset, source.nt_total)
            if overlap_start >= overlap_end:
                logger.info("series_skip variable=%s reason=no_overlap", spec.variable_code)
                continue

            matrix = np.memmap(source.path, dtype=np.float32, mode="r", shape=(source.nt_total, len(mini_ids)))
            logger.info(
                "series_start variable=%s local_start=%s local_end=%s source_nt=%s",
                spec.variable_code,
                overlap_start,
                overlap_end,
                source.nt_total,
            )

            try:
                for chunk_start in range(overlap_start, overlap_end, chunk_hours):
                    chunk_end = min(chunk_start + chunk_hours, overlap_end)
                    values_chunk = np.asarray(matrix[chunk_start:chunk_end, :], dtype=np.float32)

                    sim_chunk_end = min(chunk_end, nt_current)
                    if chunk_start < sim_chunk_end:
                        sim_dt_values = [
                            _isoformat_seconds(start_time + timedelta(seconds=offset * dt_seconds))
                            for offset in range(chunk_start, sim_chunk_end)
                        ]
                        connection.executemany(
                            "INSERT INTO output_value (series_id, dt, value) VALUES (?, ?, ?)",
                            iter_value_rows(
                                values_chunk[: sim_chunk_end - chunk_start, :],
                                dt_values=sim_dt_values,
                                mini_ids=mini_ids,
                                series_ids_by_mini=series_lookup[(spec.variable_code, 0)],
                            ),
                        )
                        sim_value_count = len(mini_ids) * (sim_chunk_end - chunk_start)
                        value_count += sim_value_count
                        logger.info(
                            "chunk_written variable=%s prev=%s chunk_start=%s chunk_end=%s values=%s dt_start=%s dt_end=%s",
                            spec.variable_code,
                            prev_flag_label(0),
                            chunk_start,
                            sim_chunk_end,
                            sim_value_count,
                            sim_dt_values[0],
                            sim_dt_values[-1],
                        )

                    forecast_chunk_start = max(chunk_start, nt_current)
                    if forecast_chunk_start < chunk_end:
                        forecast_dt_values = [
                            _isoformat_seconds(start_time + timedelta(seconds=offset * dt_seconds))
                            for offset in range(forecast_chunk_start, chunk_end)
                        ]
                        forecast_slice_start = forecast_chunk_start - chunk_start
                        connection.executemany(
                            "INSERT INTO output_value (series_id, dt, value) VALUES (?, ?, ?)",
                            iter_value_rows(
                                values_chunk[forecast_slice_start:, :],
                                dt_values=forecast_dt_values,
                                mini_ids=mini_ids,
                                series_ids_by_mini=series_lookup[(spec.variable_code, 1)],
                            ),
                        )
                        forecast_value_count = len(mini_ids) * (chunk_end - forecast_chunk_start)
                        value_count += forecast_value_count
                        logger.info(
                            "chunk_written variable=%s prev=%s chunk_start=%s chunk_end=%s values=%s dt_start=%s dt_end=%s",
                            spec.variable_code,
                            prev_flag_label(1),
                            forecast_chunk_start,
                            chunk_end,
                            forecast_value_count,
                            forecast_dt_values[0],
                            forecast_dt_values[-1],
                        )
            finally:
                del matrix
            logger.info("series_done variable=%s", spec.variable_code)

        connection.commit()
    finally:
        connection.close()

    return ExportSummary(
        database_path=database_path,
        reference_time=export_window.reference_time,
        window_start=export_window.window_start,
        window_end_exclusive=export_window.window_end_exclusive,
        nc=len(mini_ids),
        nt_current=nt_current,
        nt_forecast=nt_forecast,
        series_count=len(series_rows),
        value_count=value_count,
    )

def export_mgb_outputs(
    *,
    parhig_path: Path = DEFAULT_PARHIG,
    mini_gtp_path: Path = DEFAULT_MINI_GTP,
    output_dir: Path = DEFAULT_OUTPUT_DIR,
    output_db_path: Path = DEFAULT_OUTPUT_DB,
    schema_path: Path = DEFAULT_SCHEMA_PATH,
    output_days_before: int | None = None,
    forecast_horizon_days: int | None = None,
    chunk_hours: int = DEFAULT_CHUNK_HOURS,
    logs_dir: Path | None = None,
    workspace: str | Path | None = None,
) -> ExportSummary:
    logger = configure_run_logger((logs_dir or default_logs_dir()) / script_stem() / f"{build_execution_id()}.log")
    logger.info(
        "export_start parhig=%s mini_gtp=%s output_dir=%s output_db=%s schema=%s chunk_hours=%s",
        parhig_path,
        mini_gtp_path,
        output_dir,
        output_db_path,
        schema_path,
        chunk_hours,
    )
    if chunk_hours <= 0:
        raise ValueError(f"chunk_hours must be > 0, got {chunk_hours}")

    if output_days_before is None or forecast_horizon_days is None:
        default_before, default_after = load_output_window_from_settings(workspace=workspace)
        if output_days_before is None:
            output_days_before = default_before
        if forecast_horizon_days is None:
            forecast_horizon_days = default_after

    if output_days_before < 0 or forecast_horizon_days < 0:
        raise ValueError("output_days_before and forecast_horizon_days must be >= 0.")

    nc = read_nc_from_parhig(parhig_path)
    start_time, dt_seconds = read_time_settings_from_parhig(parhig_path)
    reference_time = load_simulation_reference_time_from_settings(workspace=workspace)
    mini_ids = read_mini_ids(mini_gtp_path, nc=nc)
    logger.info(
        "inputs_loaded nc=%s start_time=%s dt_seconds=%s mini_count=%s reference_time=%s",
        nc,
        _isoformat_seconds(start_time),
        dt_seconds,
        len(mini_ids),
        _isoformat_seconds(reference_time),
    )
    raw_sources = discover_output_sources(output_dir, nc=nc)
    sources, nt_total = validate_source_lengths(raw_sources)
    nt_current, nt_forecast = compute_nt_current(
        start_time=start_time,
        dt_seconds=dt_seconds,
        reference_time=reference_time,
        nt_total=nt_total,
    )
    logger.info("nt_resolved nt_total=%s nt_current=%s nt_forecast=%s", nt_total, nt_current, nt_forecast)

    export_window = build_export_window(
        reference_time,
        output_days_before=output_days_before,
        forecast_horizon_days=forecast_horizon_days,
    )
    logger.info(
        "export_window reference_time=%s window_start=%s window_end_exclusive=%s",
        _isoformat_seconds(reference_time),
        _isoformat_seconds(export_window.window_start),
        _isoformat_seconds(export_window.window_end_exclusive),
    )

    output_db_path.parent.mkdir(parents=True, exist_ok=True)
    temp_db_path = output_db_path.with_name(f"{output_db_path.stem}.{uuid4().hex[:8]}.tmp{output_db_path.suffix}")
    logger.info("database_temp_path path=%s", temp_db_path)

    try:
        summary = write_output_database(
            database_path=temp_db_path,
            schema_path=schema_path,
            mini_ids=mini_ids,
            sources=sources,
            start_time=start_time,
            dt_seconds=dt_seconds,
            export_window=export_window,
            nt_current=nt_current,
            nt_forecast=nt_forecast,
            chunk_hours=chunk_hours,
        )
        temp_db_path.replace(output_db_path)
        logger.info("database_finalized path=%s", output_db_path)
    except Exception:
        if temp_db_path.exists():
            temp_db_path.unlink()
        logger.exception("export_failed")
        raise

    final_summary = ExportSummary(
        database_path=output_db_path,
        reference_time=summary.reference_time,
        window_start=summary.window_start,
        window_end_exclusive=summary.window_end_exclusive,
        nc=summary.nc,
        nt_current=summary.nt_current,
        nt_forecast=summary.nt_forecast,
        series_count=summary.series_count,
        value_count=summary.value_count,
    )
    logger.info(
        "export_done database=%s reference_time=%s window_start=%s window_end_exclusive=%s series_count=%s value_count=%s",
        final_summary.database_path,
        _isoformat_seconds(final_summary.reference_time),
        _isoformat_seconds(final_summary.window_start),
        _isoformat_seconds(final_summary.window_end_exclusive),
        final_summary.series_count,
        final_summary.value_count,
    )
    return final_summary


def main() -> int:
    try:
        export_mgb_outputs()
        return 0
    except Exception:
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
