"""Opt-in Playwright smoke test for the Panel dashboard.

Run with ``RUN_DASHBOARD_BROWSER_TESTS=1 pytest tests/e2e`` after
``playwright install chromium``.
"""
from __future__ import annotations

from contextlib import closing
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import time
from types import SimpleNamespace
from urllib.request import urlopen

import pytest

from mgb_ops.assets.schemas import SQL_DIR
from mgb_ops.assets.databases import apply_schema


pytestmark = pytest.mark.skipif(
    os.getenv("RUN_DASHBOARD_BROWSER_TESTS") != "1",
    reason="set RUN_DASHBOARD_BROWSER_TESTS=1 to run Panel browser smoke tests",
)


def _free_port() -> int:
    with closing(socket.socket()) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def _controlled_workspace(root: Path) -> Path:
    (root / "config").mkdir(parents=True)
    (root / "config" / "custom.yaml").write_text(
        "run:\n"
        "  reference_time: '2026-06-29T00:00:00'\n"
        "spatial_grid:\n"
        "  bbox: [-52.5, -30.5, -51.5, -29.5]\n",
        encoding="utf-8",
    )
    database = root / "data" / "history.sqlite"
    apply_schema(database, SQL_DIR / "history_schema.sql")
    with sqlite3.connect(database) as connection:
        connection.execute(
            """
            INSERT INTO station (
                station_id, station_code, station_name, provider_code,
                latitude, longitude, altitude_m
            ) VALUES (1001, '1001', 'Known Station', 'ana', -30, -52, 10)
            """
        )
        connection.execute(
            """
            INSERT INTO observed_series (
                series_id, station_id, variable_code, state, created_at
            ) VALUES ('1001.rain.raw', 1001, 'rain', 'raw', '2026-06-29 00:00:00')
            """
        )
        connection.execute(
            """
            INSERT INTO observed_value (series_id, observed_at, value)
            VALUES ('1001.rain.raw', '2026-06-28 23:00:00', 12.5)
            """
        )
        connection.commit()
    return root


@pytest.fixture
def live_server(tmp_path: Path):
    workspace = _controlled_workspace(tmp_path / "workspace")
    port = _free_port()
    process = subprocess.Popen(
        [
            "panel",
            "serve",
            "apps/ops_dashboard/serve.py",
            "--address",
            "127.0.0.1",
            "--port",
            str(port),
            "--args",
            "--workspace",
            str(workspace),
        ],
        cwd=Path(__file__).resolve().parents[2],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    url = f"http://127.0.0.1:{port}/serve"
    try:
        for _ in range(100):
            if process.poll() is not None:
                output = process.stdout.read() if process.stdout else ""
                raise RuntimeError(f"Panel server exited early:\n{output}")
            try:
                with urlopen(url, timeout=0.2):
                    break
            except OSError:
                time.sleep(0.1)
        else:
            raise RuntimeError("Panel server did not become ready.")
        yield SimpleNamespace(url=url)
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()


def test_panel_dashboard_click_updates_station_summary(page, live_server) -> None:
    page.goto(live_server.url)
    page.get_by_text("RS Flood Alert System").wait_for()
    canvas = page.locator(".deck-container canvas").first
    canvas.wait_for()
    # The controlled station is exactly at the initial map center.
    box = canvas.bounding_box()
    assert box is not None
    canvas.click(position={"x": box["width"] / 2, "y": box["height"] / 2})
    page.get_by_text("Known Station").wait_for()
    page.get_by_text("Station 1001").wait_for()
