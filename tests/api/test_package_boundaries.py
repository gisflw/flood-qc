from __future__ import annotations

import ast
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def test_package_does_not_import_ui_frameworks() -> None:
    offenders: list[str] = []
    for path in (REPO_ROOT / "src").rglob("*.py"):
        text = path.read_text(encoding="utf-8-sig")
        if (
            "import streamlit" in text
            or "streamlit_folium" in text
            or "import panel" in text
            or "import param" in text
        ):
            offenders.append(path.relative_to(REPO_ROOT).as_posix())

    assert offenders == []


def test_root_sql_and_config_directories_are_not_present() -> None:
    assert not (REPO_ROOT / "sql").exists()
    assert not (REPO_ROOT / "config").exists()



def _domain_library_paths() -> list[Path]:
    roots = [
        REPO_ROOT / "src" / "mgb_ops" / name
        for name in ("storage", "adapters", "workflows", "analysis", "edit", "qc", "model")
    ]
    return [path for root in roots for path in root.rglob("*.py")]


def _call_name(node) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        owner = _call_name(node.value)
        return f"{owner}.{node.attr}" if owner else node.attr
    return None


FORBIDDEN_DOMAIN_RUNTIME_NAMES = {
    "SQL_DIR",
    "build_run_db_path",
    "history_db_path",
    "history_station_inventory_csv_path",
    "interim_dir",
    "runtime_paths",
    "set_workspace",
    "get_workspace",
    "resolve_workspace",
    "relative_to_repo",
    "spatial_dir",
    "timeseries_dir",
    "load_settings",
    "load_dotenv",
    "parse_dotenv",
    "load_workspace_env",
    "resolve_env_value",
    "build_runtime_context",
    "load_runtime_env",
    "resolve_workspace_from_runtime_env",
}

FORBIDDEN_DOMAIN_CALLS = FORBIDDEN_DOMAIN_RUNTIME_NAMES | {
    "os.getenv",
    "Path.cwd",
}


def test_domain_modules_do_not_import_ambient_runtime_helpers() -> None:
    offenders: list[str] = []
    for path in _domain_library_paths():
        rel_path = path.relative_to(REPO_ROOT).as_posix()
        tree = ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.ImportFrom):
                module = node.module or ""
                if module == "mgb_ops.common.paths":
                    for alias in node.names:
                        if alias.name in FORBIDDEN_DOMAIN_RUNTIME_NAMES:
                            offenders.append(f"{rel_path}:{node.lineno} imports {alias.name}")
                if module == "mgb_ops.common.settings":
                    offenders.append(f"{rel_path}:{node.lineno} imports {module}")
                if module in {"mgb_ops.common.env", "mgb_ops.common.runtime"} or "dotenv" in module:
                    offenders.append(f"{rel_path}:{node.lineno} imports {module}")
            elif isinstance(node, ast.Import):
                for alias in node.names:
                    if "dotenv" in alias.name:
                        offenders.append(f"{rel_path}:{node.lineno} imports {alias.name}")

    assert offenders == []


def test_domain_modules_do_not_call_or_read_ambient_runtime_state() -> None:
    offenders: list[str] = []
    for path in _domain_library_paths():
        rel_path = path.relative_to(REPO_ROOT).as_posix()
        tree = ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.Call):
                call_name = _call_name(node.func)
                short_name = call_name.rsplit(".", 1)[-1] if call_name else None
                if call_name in FORBIDDEN_DOMAIN_CALLS or short_name in FORBIDDEN_DOMAIN_CALLS:
                    offenders.append(f"{rel_path}:{node.lineno} calls {call_name}")
            elif isinstance(node, ast.Attribute):
                attr_name = _call_name(node)
                if attr_name == "os.environ":
                    offenders.append(f"{rel_path}:{node.lineno} reads os.environ")
            elif isinstance(node, ast.Name) and node.id == "SQL_DIR":
                offenders.append(f"{rel_path}:{node.lineno} uses SQL_DIR")

    assert offenders == []


def test_provider_specific_symbols_are_confined_to_adapters() -> None:
    provider_names = ("ana", "inmet", "ecmwf")
    offenders: list[str] = []
    roots = (REPO_ROOT / "src" / "mgb_ops", REPO_ROOT / "apps")
    for root in roots:
        for path in root.rglob("*.py"):
            if (REPO_ROOT / "src" / "mgb_ops" / "adapters") in path.parents:
                continue
            tree = ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
            for node in ast.walk(tree):
                names: list[str] = []
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                    names.append(node.name)
                elif isinstance(node, ast.Name):
                    names.append(node.id)
                elif isinstance(node, ast.Attribute):
                    names.append(node.attr)
                elif isinstance(node, ast.alias):
                    names.extend((node.name, node.asname or ""))
                for name in names:
                    name_parts = name.lower().replace(".", "_").split("_")
                    if any(provider in name_parts for provider in provider_names):
                        offenders.append(
                            f"{path.relative_to(REPO_ROOT).as_posix()}:{getattr(node, 'lineno', 0)} uses {name}"
                        )

    assert offenders == []


def test_domain_modules_have_no_direct_script_entrypoints() -> None:
    forbidden_text = (
        "argparse",
        "ArgumentParser",
        "parse_args",
        'if __name__ == "__main__"',
        "if __name__ == '__main__'",
        "sys.path.insert",
    )
    offenders: list[str] = []

    for path in _domain_library_paths():
        rel_path = path.relative_to(REPO_ROOT).as_posix()
        text = path.read_text(encoding="utf-8-sig")
        for forbidden in forbidden_text:
            if forbidden in text:
                offenders.append(f"{rel_path}: contains {forbidden}")

        tree = ast.parse(text, filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef) and node.name == "main":
                offenders.append(f"{rel_path}:{node.lineno} defines main()")

    assert offenders == []
