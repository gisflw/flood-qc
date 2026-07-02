from __future__ import annotations

import logging
from pathlib import Path
import sys


def configure_run_logger(name: str, log_file: Path) -> logging.Logger:
    """Configure a dedicated file/stdout logger for one operational run."""
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
    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    logger.addHandler(stream_handler)
    return logger


__all__ = ["configure_run_logger"]
