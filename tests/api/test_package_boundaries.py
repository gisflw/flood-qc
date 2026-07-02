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
        for name in ("assets", "adapters", "workflows", "analysis", "edit", "qc", "model")
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
                if module == "mgb_ops.config.workspace":
                    for alias in node.names:
                        if alias.name in FORBIDDEN_DOMAIN_RUNTIME_NAMES:
                            offenders.append(f"{rel_path}:{node.lineno} imports {alias.name}")
                if module == "mgb_ops.config.settings":
                    offenders.append(f"{rel_path}:{node.lineno} imports {module}")
                if (
                    module in {"mgb_ops.config.env", "mgb_ops.config.runtime"}
                    and "/workflows/" not in f"/{rel_path}"
                ) or "dotenv" in module:
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


def test_assets_is_the_only_canonical_persistence_layer() -> None:
    """Canonical database/CSV/NetCDF I/O must remain behind mgb_ops.assets."""
    forbidden_imports = {"sqlite3"}
    forbidden_calls = {
        "sqlite3.connect",
        "pd.read_sql_query",
        "pandas.read_sql_query",
        "xr.open_dataset",
        "xarray.open_dataset",
        "dataset.to_netcdf",
        "csv.DictReader",
        "csv.DictWriter",
    }
    offenders: list[str] = []
    package_root = REPO_ROOT / "src" / "mgb_ops"
    for path in package_root.rglob("*.py"):
        if package_root / "assets" in path.parents:
            continue
        rel_path = path.relative_to(REPO_ROOT).as_posix()
        tree = ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    if alias.name in forbidden_imports:
                        offenders.append(f"{rel_path}:{node.lineno} imports {alias.name}")
            elif isinstance(node, ast.Call):
                call_name = _call_name(node.func)
                if call_name in forbidden_calls or (
                    call_name is not None and call_name.endswith(".to_netcdf")
                ):
                    offenders.append(f"{rel_path}:{node.lineno} calls {call_name}")
    assert offenders == []


def test_removed_storage_package_is_not_referenced() -> None:
    package_root = REPO_ROOT / "src" / "mgb_ops"
    assert not (package_root / "storage").exists()
    offenders = []
    for path in (REPO_ROOT / "src").rglob("*.py"):
        if "mgb_ops.storage" in path.read_text(encoding="utf-8-sig"):
            offenders.append(path.relative_to(REPO_ROOT).as_posix())
    assert offenders == []


def _internal_imports(path: Path) -> list[tuple[int, str]]:
    tree = ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
    imports: list[tuple[int, str]] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module:
            imports.append((node.lineno, node.module))
        elif isinstance(node, ast.Import):
            imports.extend((node.lineno, alias.name) for alias in node.names)
    return imports


def test_foundation_dependency_direction() -> None:
    package_root = REPO_ROOT / "src" / "mgb_ops"
    offenders: list[str] = []
    forbidden_by_root = {
        "config": ("mgb_ops.assets", "mgb_ops.adapters", "mgb_ops.analysis", "mgb_ops.workflows"),
        "utils": ("mgb_ops.config", "mgb_ops.assets", "mgb_ops.adapters", "mgb_ops.analysis", "mgb_ops.workflows"),
        "assets": ("mgb_ops.config", "mgb_ops.adapters", "mgb_ops.analysis", "mgb_ops.workflows", "apps."),
        "analysis": ("mgb_ops.config", "mgb_ops.adapters", "mgb_ops.workflows", "apps."),
    }
    for root_name, forbidden_prefixes in forbidden_by_root.items():
        for path in (package_root / root_name).rglob("*.py"):
            for line, module in _internal_imports(path):
                if module.startswith(forbidden_prefixes):
                    offenders.append(
                        f"{path.relative_to(REPO_ROOT).as_posix()}:{line} imports {module}"
                    )
    assert offenders == []


def test_removed_module_boundaries_are_absent() -> None:
    package_root = REPO_ROOT / "src" / "mgb_ops"
    assert not list((package_root / "common").glob("*.py"))
    assert not (package_root / "analysis" / "spatial.py").exists()
    assert not (package_root / "workflows" / "spatial_grid.py").exists()
    offenders: list[str] = []
    forbidden = ("mgb_ops.common", "mgb_ops.analysis.spatial", "mgb_ops.workflows.spatial_grid")
    for root in (REPO_ROOT / "src", REPO_ROOT / "apps", REPO_ROOT / "tests", REPO_ROOT / "examples"):
        for path in root.rglob("*.py"):
            text = path.read_text(encoding="utf-8-sig")
            if path == Path(__file__):
                continue
            if any(value in text for value in forbidden):
                offenders.append(path.relative_to(REPO_ROOT).as_posix())
    assert offenders == []
