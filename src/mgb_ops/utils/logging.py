from __future__ import annotations

import logging
import os
from pathlib import Path
import sys
import threading


class ObservedDownloadConsoleHandler(logging.Handler):
    """Render structured observed-download events as compact console progress."""

    _SPINNER = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"

    def __init__(self, stream=None, *, spinner_interval: float = 0.1) -> None:
        super().__init__(logging.INFO)
        self.stream = stream if stream is not None else sys.stdout
        self.spinner_interval = spinner_interval
        self._terminal = bool(
            getattr(self.stream, "isatty", lambda: False)()
            and os.environ.get("TERM", "") != "dumb"
        )
        self._display_lock = threading.RLock()
        self._active_event: dict[str, object] | None = None
        self._timestamp = ""
        self._display_open = False
        self._spinner_stop: threading.Event | None = None
        self._spinner_thread: threading.Thread | None = None
        self._spinner_index = -1
        self.setFormatter(logging.Formatter(datefmt="%Y-%m-%d %H:%M:%S"))

    def emit(self, record: logging.LogRecord) -> None:
        event = getattr(record, "observed_progress", None)
        if not isinstance(event, dict):
            return
        try:
            if event.get("event") in {"station_complete", "station_failure"}:
                self._finish(event, record)
            elif self._terminal:
                self._update(event, record)
        except Exception:
            self.handleError(record)

    def _timestamp_for(self, record: logging.LogRecord) -> str:
        return self.formatter.formatTime(record, self.formatter.datefmt)

    @staticmethod
    def _field(event: dict[str, object], name: str, fallback: str = "-") -> str:
        value = event.get(name)
        return fallback if value is None else str(value)

    def _first_line(self) -> str:
        assert self._active_event is not None
        event = self._active_event
        return (
            f"{self._timestamp} | {self._field(event, 'station_id')} | update "
            f"{self._field(event, 'window_start')}, {self._field(event, 'window_end')} | "
            f"rows {self._field(event, 'rows', '0')}"
        )

    def _status_line(self) -> str:
        assert self._active_event is not None
        self._spinner_index = (self._spinner_index + 1) % len(self._SPINNER)
        status = self._field(self._active_event, "status", "requesting")
        return f"{self._SPINNER[self._spinner_index]} {status}"

    def _draw_locked(self) -> None:
        first = self._first_line()
        second = self._status_line()
        if self._display_open:
            self.stream.write(f"\r\x1b[1A\x1b[2K{first}\n\x1b[2K{second}")
        else:
            self.stream.write(f"{first}\n{second}")
            self._display_open = True
        self.stream.flush()

    def _update(self, event: dict[str, object], record: logging.LogRecord) -> None:
        with self._display_lock:
            self._active_event = event
            self._timestamp = self._timestamp_for(record)
            self._draw_locked()
            if self._spinner_thread is None:
                self._spinner_stop = threading.Event()
                self._spinner_thread = threading.Thread(
                    target=self._spin,
                    name="observed-download-spinner",
                    daemon=True,
                )
                self._spinner_thread.start()

    def _spin(self) -> None:
        stop = self._spinner_stop
        if stop is None:
            return
        while not stop.wait(self.spinner_interval):
            with self._display_lock:
                if not self._display_open or self._active_event is None:
                    return
                self._draw_locked()

    def _stop_spinner(self) -> None:
        stop = self._spinner_stop
        thread = self._spinner_thread
        if stop is not None:
            stop.set()
        if thread is not None and thread is not threading.current_thread():
            thread.join(timeout=max(self.spinner_interval * 3, 0.2))
        self._spinner_stop = None
        self._spinner_thread = None

    def _summary(self, event: dict[str, object], record: logging.LogRecord) -> str:
        timestamp = self._timestamp_for(record)
        return (
            f"{timestamp} | {self._field(event, 'station_id')} | "
            f"{self._field(event, 'status', 'complete')} | update "
            f"{self._field(event, 'window_start')}, {self._field(event, 'window_end')} | "
            f"rows {self._field(event, 'rows', '0')}"
        )

    def _finish(self, event: dict[str, object], record: logging.LogRecord) -> None:
        self._stop_spinner()
        with self._display_lock:
            if self._terminal and self._display_open:
                self.stream.write("\r\x1b[1A\x1b[2K\r\n\x1b[2K\r\x1b[1A\r")
                self._display_open = False
                self.stream.flush()
            self._active_event = None
            self.stream.write(self._summary(event, record) + "\n")
            self.stream.flush()

    def close(self) -> None:
        self._stop_spinner()
        with self._display_lock:
            if self._terminal and self._display_open:
                self.stream.write("\r\x1b[1A\x1b[2K\r\n\x1b[2K\r\x1b[1A\r")
                self.stream.flush()
            self._display_open = False
            self._active_event = None
        super().close()


def log_observed_progress(
    logger: logging.Logger | None,
    *,
    event: str,
    provider_code: str,
    station_id: str,
    window_start: str | None,
    window_end: str | None,
    rows: int,
    status: str,
) -> None:
    """Write a structured progress event for the observed-download console."""
    if logger is None:
        return
    progress = {
        "event": event,
        "provider_code": provider_code,
        "station_id": station_id,
        "window_start": window_start,
        "window_end": window_end,
        "rows": rows,
        "status": status,
    }
    logger.info(
        "observed_progress event=%s station_id=%s window_start=%s window_end=%s rows=%s status=%s",
        event,
        station_id,
        window_start,
        window_end,
        rows,
        status,
        extra={"observed_progress": progress},
    )


def configure_run_logger(
    name: str,
    log_file: Path,
    *,
    console: bool = True,
    observed_progress: bool = False,
) -> logging.Logger:
    """Configure a dedicated file logger, optionally mirrored to stdout."""
    target = Path(log_file)
    target.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger(name)
    logger.setLevel(logging.INFO)
    for handler in logger.handlers[:]:
        handler.close()
        logger.removeHandler(handler)
    logger.propagate = False

    formatter = logging.Formatter("%(asctime)s | %(levelname)s | %(message)s")
    file_handler = logging.FileHandler(target, mode="w", encoding="utf-8")
    file_handler.setFormatter(formatter)
    logger.addHandler(file_handler)
    if observed_progress:
        logger.addHandler(ObservedDownloadConsoleHandler())
    elif console:
        stream_handler = logging.StreamHandler(sys.stdout)
        stream_handler.setFormatter(formatter)
        logger.addHandler(stream_handler)
    return logger


__all__ = ["ObservedDownloadConsoleHandler", "configure_run_logger", "log_observed_progress"]
