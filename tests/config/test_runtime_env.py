from __future__ import annotations

from pathlib import Path

import pytest

from mgb_ops.common.env import INMET_API_KEY_ENV, parse_dotenv, resolve_env_value
from mgb_ops.common.paths import MGB_OPS_WORKSPACE_ENV, MGB_REMOTE_WORKSPACE_ENV
from mgb_ops.common.runtime import build_runtime_context


def test_parse_dotenv_supports_workspace_values(tmp_path) -> None:
    dotenv_path = tmp_path / ".env"
    dotenv_path.write_text(
        """# local workspace secrets
INMET_API_KEY=from-dotenv
MGB_OPS_REMOTE_WORKSPACE='C:/mgb hora'
""",
        encoding="utf-8",
    )

    assert parse_dotenv(dotenv_path) == {
        INMET_API_KEY_ENV: "from-dotenv",
        MGB_REMOTE_WORKSPACE_ENV: "C:/mgb hora",
    }


def test_runtime_context_loads_workspace_dotenv(tmp_path, monkeypatch) -> None:
    workspace = tmp_path / "scratch" / "rs_hydro"
    workspace.mkdir(parents=True)
    (workspace / ".env").write_text(
        """INMET_API_KEY=workspace-key
MGB_OPS_REMOTE_WORKSPACE=C:/from-dotenv
""",
        encoding="utf-8",
    )
    monkeypatch.delenv(INMET_API_KEY_ENV, raising=False)
    monkeypatch.delenv(MGB_REMOTE_WORKSPACE_ENV, raising=False)

    context = build_runtime_context(workspace=workspace)

    assert context.paths.workspace == workspace.resolve()
    assert context.paths.remote_workspace_root == Path("C:/from-dotenv")
    assert context.env.require_inmet_api_key() == "workspace-key"


def test_runtime_context_precedence_is_explicit_env_dotenv_default(tmp_path, monkeypatch) -> None:
    workspace = tmp_path / "region"
    workspace.mkdir()
    (workspace / ".env").write_text(
        """INMET_API_KEY=dotenv-key
MGB_OPS_REMOTE_WORKSPACE=C:/dotenv
""",
        encoding="utf-8",
    )
    monkeypatch.setenv(INMET_API_KEY_ENV, "process-key")
    monkeypatch.setenv(MGB_REMOTE_WORKSPACE_ENV, "C:/process")

    context = build_runtime_context(workspace=workspace, remote_workspace="C:/explicit")

    assert context.paths.remote_workspace_root == Path("C:/explicit")
    assert context.env.require_inmet_api_key() == "process-key"
    assert context.env.require_inmet_api_key(explicit="explicit-key") == "explicit-key"


def test_workspace_can_be_resolved_from_dotenv_when_no_env(tmp_path, monkeypatch) -> None:
    workspace = tmp_path / "from-dotenv"
    workspace.mkdir()
    dotenv_path = tmp_path / ".env"
    dotenv_path.write_text(f"{MGB_OPS_WORKSPACE_ENV}={workspace}\n", encoding="utf-8")
    monkeypatch.delenv(MGB_OPS_WORKSPACE_ENV, raising=False)

    context = build_runtime_context(dotenv_path=dotenv_path)

    assert context.paths.workspace == workspace.resolve()


def test_resolve_env_value_precedence(monkeypatch) -> None:
    monkeypatch.setenv("EXAMPLE_VALUE", "process")

    assert resolve_env_value("EXAMPLE_VALUE", explicit="explicit", dotenv_values={"EXAMPLE_VALUE": "dotenv"}) == "explicit"
    assert resolve_env_value("EXAMPLE_VALUE", dotenv_values={"EXAMPLE_VALUE": "dotenv"}) == "process"
    monkeypatch.delenv("EXAMPLE_VALUE")
    assert resolve_env_value("EXAMPLE_VALUE", dotenv_values={"EXAMPLE_VALUE": "dotenv"}, default="default") == "dotenv"
    assert resolve_env_value("EXAMPLE_VALUE", dotenv_values={}, default="default") == "default"


def test_inmet_api_key_missing_raises(tmp_path, monkeypatch) -> None:
    workspace = tmp_path / "region"
    workspace.mkdir()
    monkeypatch.delenv(INMET_API_KEY_ENV, raising=False)

    context = build_runtime_context(workspace=workspace)

    with pytest.raises(RuntimeError, match=INMET_API_KEY_ENV):
        context.env.require_inmet_api_key()
