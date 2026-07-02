from __future__ import annotations

import logging

from mgb_ops.utils.logging import configure_run_logger


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
