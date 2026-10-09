from __future__ import annotations

import logging
from io import StringIO

from mgb_ops.utils.logging import (
    ObservedDownloadConsoleHandler,
    configure_run_logger,
    log_observed_progress,
)


def test_configure_run_logger_replaces_handlers_and_writes_file(tmp_path) -> None:
    path = tmp_path / "run.log"
    logger = configure_run_logger("tests.run_logger", path)
    logger.info("first")
    replacement = configure_run_logger("tests.run_logger", path)
    replacement.info("second")
    for handler in replacement.handlers:
        handler.flush()

    assert logger is replacement
    assert len(replacement.handlers) == 2
    assert "second" in path.read_text(encoding="utf-8")

    for handler in replacement.handlers[:]:
        handler.close()
        replacement.removeHandler(handler)
    replacement.setLevel(logging.NOTSET)


class _TerminalStream(StringIO):
    def isatty(self) -> bool:
        return True


def _progress(
    logger,
    event: str,
    *,
    status: str,
    rows: int = 0,
    station_id: str = "inmet:A801",
) -> None:
    log_observed_progress(
        logger,
        event=event,
        provider_code="inmet",
        station_id=station_id,
        window_start="2026-03-11",
        window_end="2026-03-11",
        rows=rows,
        status=status,
    )


def test_observed_console_handler_redraws_and_retains_compact_station_summary(monkeypatch) -> None:
    monkeypatch.setenv("TERM", "xterm")
    stream = _TerminalStream()
    handler = ObservedDownloadConsoleHandler(stream, spinner_interval=60)
    logger = logging.getLogger("tests.observed_progress_terminal")
    logger.handlers.clear()
    logger.propagate = False
    logger.setLevel(logging.INFO)
    logger.addHandler(handler)

    _progress(logger, "request_start", status="requesting")
    _progress(logger, "retry", status="timed out; retrying 2/5")
    _progress(logger, "parsed", status="loading", rows=3)
    _progress(logger, "station_complete", status="complete", rows=3)
    _progress(logger, "request_start", status="requesting", station_id="inmet:B802")
    _progress(logger, "station_complete", status="no data", station_id="inmet:B802")

    output = stream.getvalue()
    assert "inmet:A801 | update 2026-03-11, 2026-03-11 | rows 0" in output
    assert "timed out; retrying 2/5" in output
    assert "rows 3" in output
    assert output.count("inmet:A801 | complete | update") == 1
    assert output.count("inmet:B802 | no data | update") == 1
    assert "\x1b[2K" in output
    assert handler._spinner_thread is None

    logger.removeHandler(handler)
    handler.close()
    logger.setLevel(logging.NOTSET)


def test_observed_console_handler_uses_one_plain_summary_when_redirected(monkeypatch) -> None:
    monkeypatch.setenv("TERM", "xterm")
    stream = StringIO()
    handler = ObservedDownloadConsoleHandler(stream)
    logger = logging.getLogger("tests.observed_progress_redirected")
    logger.handlers.clear()
    logger.propagate = False
    logger.setLevel(logging.INFO)
    logger.addHandler(handler)

    _progress(logger, "request_start", status="requesting")
    _progress(logger, "station_complete", status="no data")

    output = stream.getvalue()
    assert output.count("inmet:A801") == 1
    assert "no data | update 2026-03-11, 2026-03-11 | rows 0" in output
    assert "\x1b[" not in output
    assert "⠋" not in output

    logger.removeHandler(handler)
    handler.close()
    logger.setLevel(logging.NOTSET)


def test_observed_console_handler_stops_spinner_when_closed(monkeypatch) -> None:
    monkeypatch.setenv("TERM", "xterm")
    stream = _TerminalStream()
    handler = ObservedDownloadConsoleHandler(stream, spinner_interval=0.01)
    logger = logging.getLogger("tests.observed_progress_cleanup")
    logger.handlers.clear()
    logger.propagate = False
    logger.setLevel(logging.INFO)
    logger.addHandler(handler)

    _progress(logger, "request_start", status="requesting")
    spinner = handler._spinner_thread
    assert spinner is not None and spinner.is_alive()
    handler.close()

    assert handler._spinner_thread is None
    assert not spinner.is_alive()
    assert "\x1b[2K" in stream.getvalue()

    logger.removeHandler(handler)
    logger.setLevel(logging.NOTSET)
