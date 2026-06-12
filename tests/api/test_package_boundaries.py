from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


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



def _core_library_paths() -> list[Path]:
    roots = [
        REPO_ROOT / "src" / "mgb_ops" / name
        for name in ("storage", "ingest", "qc", "model")
    ]
    return [path for root in roots for path in root.rglob("*.py")]


def _call_name(node) -> str | None:
    import ast

    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return node.attr
    return None


def test_core_library_does_not_load_settings_or_set_workspace() -> None:
    import ast

    forbidden = {"load_settings", "set_workspace"}
    offenders: list[str] = []
    for path in _core_library_paths():
        tree = ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.Call) and _call_name(node.func) in forbidden:
                rel_path = path.relative_to(REPO_ROOT).as_posix()
                offenders.append(f"{rel_path}:{node.lineno}")

    assert offenders == []


def test_core_library_has_no_import_time_workspace_path_defaults() -> None:
    import ast

    workspace_helpers = {
        "history_db_path",
        "mgb_input_dir",
        "mgb_output_dir",
        "logs_dir",
        "interim_dir",
        "runtime_paths",
        "set_workspace",
    }
    offenders: list[str] = []
    for path in _core_library_paths():
        tree = ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
        for node in tree.body:
            value = None
            if isinstance(node, ast.Assign):
                value = node.value
            elif isinstance(node, ast.AnnAssign):
                value = node.value
            if isinstance(value, ast.Call) and _call_name(value.func) in workspace_helpers:
                rel_path = path.relative_to(REPO_ROOT).as_posix()
                offenders.append(f"{rel_path}:{node.lineno}")

    assert offenders == []
