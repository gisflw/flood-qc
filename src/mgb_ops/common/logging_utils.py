from __future__ import annotations

import logging


def configure_logging() -> None:
    """Configura logging basico fixo em codigo."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s | %(levelname)s | %(name)s | %(message)s",
    )
