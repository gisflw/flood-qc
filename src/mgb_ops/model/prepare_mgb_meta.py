from __future__ import annotations

import argparse
import logging
import sys
from dataclasses import dataclass
from datetime import datetime, time, timedelta
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = REPO_ROOT / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

from mgb_ops.common.time_utils import resolve_reference_time

DEFAULT_DT_SECONDS = 3600
LOGGER_NAME = "floodqc.model.prepare_mgb_meta"


@dataclass(frozen=True, slots=True)
class MgbWindow:
    reference_time: datetime
    start_time: datetime
    forecast_start_time: datetime
    forecast_nt: int
    nt: int
    dt_seconds: int
    input_days_before: int
    forecast_horizon_days: int


@dataclass(frozen=True, slots=True)
class MgbMetaUpdateSummary:
    parhig_path: Path
    reference_time: datetime
    start_time: datetime
    nt: int
    dt_seconds: int
    input_days_before: int
    forecast_horizon_days: int


def script_stem() -> str:
    return Path(__file__).stem


def build_execution_id() -> str:
    return datetime.now().strftime("%Y%m%dT%H%M%S")


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
    import re

    return re.findall(r"[-+]?\d+(?:[.,]\d+)?", text)


def _next_data_line_index(lines: list[str], start_idx: int) -> int:
    for idx in range(start_idx + 1, len(lines)):
        stripped = lines[idx].strip()
        if not stripped or stripped.startswith("!"):
            continue
        return idx
    raise ValueError("Could not find a data line after the header line.")


def _find_header_index(lines: list[str], required_tokens: tuple[str, ...]) -> int:
    for idx, raw_line in enumerate(lines):
        upper = raw_line.upper()
        if all(token in upper for token in required_tokens):
            return idx
    raise ValueError(f"Could not find header containing tokens: {required_tokens}")


def _format_start_time_line(start_time: datetime) -> str:
    return f"        {start_time.day:02d}       {start_time.month:02d}       {start_time.year:04d}        {start_time.hour:02d}"


def _format_nt_dt_line(nt: int, dt_seconds: int) -> str:
    return f"{nt:10d}     {dt_seconds}."


def build_mgb_window(
    reference_time: datetime,
    *,
    input_days_before: int,
    forecast_horizon_days: int,
) -> MgbWindow:
    if input_days_before < 1:
        raise ValueError("input_days_before must be >= 1.")
    if forecast_horizon_days < 1:
        raise ValueError("forecast_horizon_days must be >= 1.")
    if reference_time.minute != 0 or reference_time.second != 0 or reference_time.microsecond != 0:
        raise ValueError("reference_time must be aligned to the hour for MGB hourly inputs.")

    start_date = reference_time.date() - timedelta(days=input_days_before)
    start_time = datetime.combine(start_date, time.min)
    forecast_start_time = reference_time + timedelta(hours=1)
    forecast_nt = forecast_horizon_days * 24 + 1
    forecast_end_time = forecast_start_time + timedelta(hours=forecast_nt - 1)
    nt = int((forecast_end_time - start_time).total_seconds() // DEFAULT_DT_SECONDS) + 1
    if nt < 1:
        raise ValueError(f"Invalid NT calculated from reference_time={reference_time} and start_time={start_time}.")
    return MgbWindow(
        reference_time=reference_time,
        start_time=start_time,
        forecast_start_time=forecast_start_time,
        forecast_nt=forecast_nt,
        nt=nt,
        dt_seconds=DEFAULT_DT_SECONDS,
        input_days_before=input_days_before,
        forecast_horizon_days=forecast_horizon_days,
    )


def update_parhig_text(text: str, *, start_time: datetime, nt: int, dt_seconds: int = DEFAULT_DT_SECONDS) -> str:
    lines = text.splitlines()
    start_header_idx = _find_header_index(lines, ("DIA", "MES", "ANO", "HORA"))
    nt_header_idx = _find_header_index(lines, ("NT", "DT"))

    start_data_idx = _next_data_line_index(lines, start_header_idx)
    nt_data_idx = _next_data_line_index(lines, nt_header_idx)

    lines[start_data_idx] = _format_start_time_line(start_time)
    lines[nt_data_idx] = _format_nt_dt_line(nt, dt_seconds)
    return "\n".join(lines) + "\n"


def read_time_settings_from_parhig(parhig_path: Path) -> tuple[datetime, int, int]:
    lines = parhig_path.read_text(encoding="latin-1").splitlines()
    start_time: datetime | None = None
    nt: int | None = None
    dt_seconds: int | None = None

    for idx, raw_line in enumerate(lines):
        upper = raw_line.upper()
        if start_time is None and all(token in upper for token in ("DIA", "MES", "ANO", "HORA")):
            numbers = _extract_numbers(lines[_next_data_line_index(lines, idx)])
            if len(numbers) >= 4:
                day = int(float(numbers[0].replace(",", ".")))
                month = int(float(numbers[1].replace(",", ".")))
                year = int(float(numbers[2].replace(",", ".")))
                hour = int(float(numbers[3].replace(",", ".")))
                start_time = datetime(year, month, day, hour)
        if nt is None and dt_seconds is None and "NT" in upper and "DT" in upper:
            numbers = _extract_numbers(lines[_next_data_line_index(lines, idx)])
            if len(numbers) >= 2:
                nt = int(float(numbers[0].replace(",", ".")))
                dt_seconds = int(float(numbers[1].replace(",", ".")))
        if start_time is not None and nt is not None and dt_seconds is not None:
            break

    if start_time is None or nt is None or dt_seconds is None:
        raise ValueError(
            f"Could not read timing from {parhig_path}. Expected PARHIG to provide DIA/MES/ANO/HORA and NT/DT."
        )
    if nt <= 0 or dt_seconds <= 0:
        raise ValueError(f"Invalid PARHIG timing values: nt={nt}, dt_seconds={dt_seconds}")
    return start_time, nt, dt_seconds


def rewrite_mgb_meta(
    *,
    parhig_path: Path,
    reference_time: datetime,
    input_days_before: int,
    forecast_horizon_days: int,
    logs_dir: Path | None = None,
    logger: logging.Logger | None = None,
) -> MgbMetaUpdateSummary:
    window = build_mgb_window(
        reference_time,
        input_days_before=input_days_before,
        forecast_horizon_days=forecast_horizon_days,
    )
    run_logger = logger
    if run_logger is None and logs_dir is not None:
        execution_id = build_execution_id()
        run_logger = configure_run_logger(logs_dir / script_stem() / f"{execution_id}.log")

    original_parhig_text = parhig_path.read_text(encoding="latin-1")
    updated_parhig_text = update_parhig_text(
        original_parhig_text,
        start_time=window.start_time,
        nt=window.nt,
        dt_seconds=window.dt_seconds,
    )
    parhig_path.write_text(updated_parhig_text, encoding="latin-1")

    if run_logger is not None:
        run_logger.info(
            "mgb_meta_updated parhig=%s reference_time=%s start_time=%s nt=%s dt_seconds=%s input_days_before=%s forecast_horizon_days=%s",
            parhig_path,
            window.reference_time.isoformat(timespec="seconds"),
            window.start_time.isoformat(timespec="seconds"),
            window.nt,
            window.dt_seconds,
            input_days_before,
            forecast_horizon_days,
        )
    return MgbMetaUpdateSummary(
        parhig_path=parhig_path,
        reference_time=window.reference_time,
        start_time=window.start_time,
        nt=window.nt,
        dt_seconds=window.dt_seconds,
        input_days_before=input_days_before,
        forecast_horizon_days=forecast_horizon_days,
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Rewrite PARHIG.hig timing metadata.")
    parser.add_argument("--parhig", type=Path, required=True, help="PARHIG.hig file to rewrite.")
    parser.add_argument("--reference-time", required=True, help="Reference time for the MGB window.")
    parser.add_argument("--input-days-before", type=int, required=True)
    parser.add_argument("--forecast-horizon-days", type=int, required=True)
    parser.add_argument("--logs-dir", type=Path, default=None)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    summary = rewrite_mgb_meta(
        parhig_path=args.parhig,
        reference_time=resolve_reference_time(args.reference_time),
        input_days_before=args.input_days_before,
        forecast_horizon_days=args.forecast_horizon_days,
        logs_dir=args.logs_dir,
    )
    print(
        "mgb_meta_ready "
        f"parhig={summary.parhig_path} "
        f"reference_time={summary.reference_time.isoformat(timespec='seconds')} "
        f"start_time={summary.start_time.isoformat(timespec='seconds')} "
        f"nt={summary.nt}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
