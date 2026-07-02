from __future__ import annotations

import logging
import re
from dataclasses import dataclass
from datetime import date, datetime, time, timedelta, timezone
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path
from uuid import uuid4

import numpy as np
from mgb_ops.assets.model_outputs import write_model_outputs_netcdf
from mgb_ops.utils.logging import configure_run_logger as _configure_run_logger

DEFAULT_CHUNK_HOURS = 720
NUMBER_PATTERN = re.compile(r"[-+]?\d+(?:[.,]\d+)?")
LOGGER_NAME = "model.export_mgb_outputs"


@dataclass(frozen=True, slots=True)
class VariableSpec:
    source_filename: str
    variable_code: str
    display_name: str
    unit: str
    netcdf_unit: str
    long_name: str
    standard_name: str | None = None


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
    netcdf_path: Path
    reference_time: datetime
    window_start: datetime
    window_end_exclusive: datetime
    nc: int
    nt_current: int
    nt_forecast: int
    variable_count: int
    value_count: int


VARIABLE_SPECS = (
    VariableSpec(
        source_filename="CHUVABIN.hig",
        variable_code="precipitation",
        display_name="Precipitation",
        unit="mm",
        netcdf_unit="mm",
        long_name="MGB precipitation forcing",
        standard_name="lwe_thickness_of_precipitation_amount",
    ),
    VariableSpec(
        source_filename="QTUDO_Inercial_Atual.MGB",
        variable_code="flow",
        display_name="Flow",
        unit="m3/s",
        netcdf_unit="m3 s-1",
        long_name="MGB river discharge",
        standard_name="water_volume_transport_in_river_channel",
    ),
    VariableSpec(
        source_filename="YTUDO.MGB",
        variable_code="level",
        display_name="Level",
        unit="cm",
        netcdf_unit="cm",
        long_name="MGB river stage",
    ),
)


def script_stem() -> str:
    return Path(__file__).stem


def build_execution_id() -> str:
    return datetime.now().strftime("%Y%m%dT%H%M%S")


def configure_run_logger(log_file: Path) -> logging.Logger:
    return _configure_run_logger(LOGGER_NAME, log_file)


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


def discover_output_sources(
    output_dir: Path,
    *,
    chuvabin_path: Path,
    nc: int,
) -> dict[str, OutputSource]:
    sources: dict[str, OutputSource] = {}
    logger = logging.getLogger(LOGGER_NAME)

    for spec in VARIABLE_SPECS:
        source_path = chuvabin_path if spec.variable_code == "precipitation" else output_dir / spec.source_filename
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


def _package_version() -> str:
    try:
        return version("mgb-ops")
    except PackageNotFoundError:
        return "unknown"


def _build_time_values(*, start_time: datetime, dt_seconds: int, start_offset: int, end_offset: int) -> np.ndarray:
    return np.array(
        [start_time + timedelta(seconds=offset * dt_seconds) for offset in range(start_offset, end_offset)],
        dtype="datetime64[ns]",
    )


def _build_time_segment(*, start_offset: int, end_offset: int, nt_current: int) -> np.ndarray:
    offsets = np.arange(start_offset, end_offset, dtype=np.int32)
    return np.where(offsets < nt_current, 0, 1).astype(np.int8)


def _build_variable_attrs(spec: VariableSpec) -> dict[str, str]:
    attrs = {
        "long_name": spec.long_name,
        "units": spec.netcdf_unit,
        "source_filename": spec.source_filename,
        "mgb_display_name": spec.display_name,
    }
    if spec.standard_name is not None:
        attrs["standard_name"] = spec.standard_name
    return attrs


def _build_global_attrs(
    *,
    start_time: datetime,
    dt_seconds: int,
    export_window: ExportWindow,
    nt_current: int,
    nt_forecast: int,
) -> dict[str, str | int]:
    created_at = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    return {
        "Conventions": "CF-1.11 ACDD-1.3",
        "title": "MGB operational model outputs",
        "summary": "Dense NetCDF export of selected MGB operational output variables by time and mini-basin.",
        "source": "MGB hydrological model binary output files",
        "history": f"{created_at} created by mgb_ops.model.export_mgb_outputs",
        "date_created": created_at,
        "reference_time": _isoformat_seconds(export_window.reference_time),
        "reference_date": export_window.reference_date.isoformat(),
        "mgb_start_time": _isoformat_seconds(start_time),
        "window_start": _isoformat_seconds(export_window.window_start),
        "window_end_exclusive": _isoformat_seconds(export_window.window_end_exclusive),
        "dt_seconds": dt_seconds,
        "nt_current": nt_current,
        "nt_forecast": nt_forecast,
        "package_name": "mgb-ops",
        "package_version": _package_version(),
    }


def write_output_netcdf(
    *,
    netcdf_path: Path,
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

    window_nt = global_end_offset - global_start_offset
    time_values = _build_time_values(
        start_time=start_time,
        dt_seconds=dt_seconds,
        start_offset=global_start_offset,
        end_offset=global_end_offset,
    )
    time_segment = _build_time_segment(
        start_offset=global_start_offset,
        end_offset=global_end_offset,
        nt_current=nt_current,
    )

    data_values: dict[str, np.ndarray] = {}
    variable_attrs: dict[str, dict[str, str]] = {}
    value_count = 0
    for spec in VARIABLE_SPECS:
        source = sources[spec.variable_code]
        overlap_start = global_start_offset
        overlap_end = min(global_end_offset, source.nt_total)
        if overlap_start >= overlap_end:
            logger.info("variable_skip variable=%s reason=no_overlap", spec.variable_code)
            continue

        values = np.full((window_nt, len(mini_ids)), np.nan, dtype=np.float32)
        matrix = np.memmap(source.path, dtype=np.float32, mode="r", shape=(source.nt_total, len(mini_ids)))
        logger.info(
            "variable_start variable=%s local_start=%s local_end=%s source_nt=%s",
            spec.variable_code,
            overlap_start,
            overlap_end,
            source.nt_total,
        )

        try:
            for chunk_start in range(overlap_start, overlap_end, chunk_hours):
                chunk_end = min(chunk_start + chunk_hours, overlap_end)
                values_slice_start = chunk_start - global_start_offset
                values_slice_end = chunk_end - global_start_offset
                values[values_slice_start:values_slice_end, :] = np.asarray(
                    matrix[chunk_start:chunk_end, :],
                    dtype=np.float32,
                )
                chunk_value_count = len(mini_ids) * (chunk_end - chunk_start)
                value_count += chunk_value_count
                logger.info(
                    "chunk_written variable=%s chunk_start=%s chunk_end=%s values=%s dt_start=%s dt_end=%s",
                    spec.variable_code,
                    chunk_start,
                    chunk_end,
                    chunk_value_count,
                    _isoformat_seconds(start_time + timedelta(seconds=chunk_start * dt_seconds)),
                    _isoformat_seconds(start_time + timedelta(seconds=(chunk_end - 1) * dt_seconds)),
                )
        finally:
            del matrix
        if spec.variable_code == "level":
            values = values.astype(np.float64) * 100.0
        data_values[spec.variable_code] = values
        variable_attrs[spec.variable_code] = _build_variable_attrs(spec)
        logger.info("variable_done variable=%s", spec.variable_code)

    write_model_outputs_netcdf(
        path=netcdf_path,
        variables=data_values,
        variable_attrs=variable_attrs,
        time_values=time_values,
        time_segment=time_segment,
        mini_ids=mini_ids,
        global_attrs=_build_global_attrs(
            start_time=start_time,
            dt_seconds=dt_seconds,
            export_window=export_window,
            nt_current=nt_current,
            nt_forecast=nt_forecast,
        ),
    )
    logger.info("netcdf_written path=%s variables=%s values=%s", netcdf_path, len(data_values), value_count)

    return ExportSummary(
        netcdf_path=netcdf_path,
        reference_time=export_window.reference_time,
        window_start=export_window.window_start,
        window_end_exclusive=export_window.window_end_exclusive,
        nc=len(mini_ids),
        nt_current=nt_current,
        nt_forecast=nt_forecast,
        variable_count=len(data_values),
        value_count=value_count,
    )


def export_mgb_outputs(
    *,
    reference_time: datetime,
    output_days_before: int,
    forecast_horizon_days: int,
    parhig_path: Path,
    mini_gtp_path: Path,
    chuvabin_path: Path,
    output_dir: Path,
    output_nc_path: Path,
    chunk_hours: int = DEFAULT_CHUNK_HOURS,
    logs_dir: Path | None = None,
    logger: logging.Logger | None = None,
) -> ExportSummary:
    run_logger = logger
    if run_logger is None and logs_dir is not None:
        run_logger = configure_run_logger(logs_dir / script_stem() / f"{build_execution_id()}.log")
    if run_logger is None:
        run_logger = logging.getLogger(LOGGER_NAME)
    run_logger.info(
        "export_start parhig=%s mini_gtp=%s chuvabin=%s output_dir=%s output_nc=%s chunk_hours=%s",
        parhig_path,
        mini_gtp_path,
        chuvabin_path,
        output_dir,
        output_nc_path,
        chunk_hours,
    )
    if chunk_hours <= 0:
        raise ValueError(f"chunk_hours must be > 0, got {chunk_hours}")

    if output_days_before < 0 or forecast_horizon_days < 0:
        raise ValueError("output_days_before and forecast_horizon_days must be >= 0.")

    nc = read_nc_from_parhig(parhig_path)
    start_time, dt_seconds = read_time_settings_from_parhig(parhig_path)
    mini_ids = read_mini_ids(mini_gtp_path, nc=nc)
    run_logger.info(
        "inputs_loaded nc=%s start_time=%s dt_seconds=%s mini_count=%s reference_time=%s",
        nc,
        _isoformat_seconds(start_time),
        dt_seconds,
        len(mini_ids),
        _isoformat_seconds(reference_time),
    )
    raw_sources = discover_output_sources(output_dir, chuvabin_path=chuvabin_path, nc=nc)
    sources, nt_total = validate_source_lengths(raw_sources)
    nt_current, nt_forecast = compute_nt_current(
        start_time=start_time,
        dt_seconds=dt_seconds,
        reference_time=reference_time,
        nt_total=nt_total,
    )
    run_logger.info("nt_resolved nt_total=%s nt_current=%s nt_forecast=%s", nt_total, nt_current, nt_forecast)

    export_window = build_export_window(
        reference_time,
        output_days_before=output_days_before,
        forecast_horizon_days=forecast_horizon_days,
    )
    run_logger.info(
        "export_window reference_time=%s window_start=%s window_end_exclusive=%s",
        _isoformat_seconds(reference_time),
        _isoformat_seconds(export_window.window_start),
        _isoformat_seconds(export_window.window_end_exclusive),
    )

    output_nc_path.parent.mkdir(parents=True, exist_ok=True)
    temp_nc_path = output_nc_path.with_name(f"{output_nc_path.stem}.{uuid4().hex[:8]}.tmp{output_nc_path.suffix}")
    run_logger.info("netcdf_temp_path path=%s", temp_nc_path)

    try:
        summary = write_output_netcdf(
            netcdf_path=temp_nc_path,
            mini_ids=mini_ids,
            sources=sources,
            start_time=start_time,
            dt_seconds=dt_seconds,
            export_window=export_window,
            nt_current=nt_current,
            nt_forecast=nt_forecast,
            chunk_hours=chunk_hours,
        )
        temp_nc_path.replace(output_nc_path)
        run_logger.info("netcdf_finalized path=%s", output_nc_path)
    except Exception:
        if temp_nc_path.exists():
            temp_nc_path.unlink()
        run_logger.exception("export_failed")
        raise

    final_summary = ExportSummary(
        netcdf_path=output_nc_path,
        reference_time=summary.reference_time,
        window_start=summary.window_start,
        window_end_exclusive=summary.window_end_exclusive,
        nc=summary.nc,
        nt_current=summary.nt_current,
        nt_forecast=summary.nt_forecast,
        variable_count=summary.variable_count,
        value_count=summary.value_count,
    )
    run_logger.info(
        "export_done netcdf=%s reference_time=%s window_start=%s window_end_exclusive=%s variable_count=%s value_count=%s",
        final_summary.netcdf_path,
        _isoformat_seconds(final_summary.reference_time),
        _isoformat_seconds(final_summary.window_start),
        _isoformat_seconds(final_summary.window_end_exclusive),
        final_summary.variable_count,
        final_summary.value_count,
    )
    return final_summary
