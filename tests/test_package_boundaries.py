from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


def test_only_ops_dashboard_app_imports_streamlit() -> None:
    offenders: list[str] = []
    for path in (REPO_ROOT / "src").rglob("*.py"):
        text = path.read_text(encoding="utf-8-sig")
        if "import streamlit" in text or "streamlit_folium" in text:
            offenders.append(path.relative_to(REPO_ROOT).as_posix())

    assert offenders == []


def test_root_sql_and_config_directories_are_not_present() -> None:
    assert not (REPO_ROOT / "sql").exists()
    assert not (REPO_ROOT / "config").exists()
