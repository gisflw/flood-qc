from __future__ import annotations

from datetime import datetime

from mgb_ops.model.prepare_mgb_meta import build_mgb_window, rewrite_mgb_meta_from_config


PARHIG_TEMPLATE = """\
ARQUIVO DE INFORMACOES GERAIS PARA O MODELO DE GRANDES BACIAS
!
Projeto Teste
!
       DIA       MES       ANO      HORA          !INICIO DA SIMULACAO
        01       01       2018        01

        NT        DT       !NUMERO DE INTERVALOS DE TEMPO E TAMANHO DO INTERVALO EM SEGUNDOS
         1     3600.

        NC        NU        NB      NCLI     !NUMERO DE CELULAS, USOS, BACIAS E POSTOS CLIMA
         2         1         1         1

linha final preservada
"""

def test_build_mgb_window_includes_forecast_horizon() -> None:
    window = build_mgb_window(
        datetime(2026, 3, 11, 23, 0, 0),
        input_days_before=2,
        forecast_horizon_days=2,
    )

    assert window.start_time == datetime(2026, 3, 9, 0, 0, 0)
    assert window.nt == 121


def test_rewrite_mgb_meta_from_config_updates_parhig(tmp_path, monkeypatch) -> None:
    parhig_path = tmp_path / "PARHIG.hig"
    parhig_path.write_text(PARHIG_TEMPLATE, encoding="latin-1")

    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_meta.load_settings",
        lambda **_: {
            "run": {"reference_time": "2026-03-11"},
            "ingest": {"request_days": 7, "timeout_seconds": 15},
            "summaries": {"forecast_days": [1], "accum_hours": [24], "selected_mini_ids": []},
            "mgb": {
                "input_days_before": 2,
                "output_days_before": 30,
                "forecast_horizon_days": 2,
                "use_forecast_data": False,
            },
            "rainfall_interpolation": {"nearest_stations": 5, "power": 2.0},
        },
    )
    monkeypatch.setattr("mgb_ops.model.prepare_mgb_meta.build_execution_id", lambda: "20260311T230000")
    monkeypatch.setattr("mgb_ops.model.prepare_mgb_meta.default_logs_dir", lambda: tmp_path / "logs")

    summary = rewrite_mgb_meta_from_config(
        parhig_path=parhig_path,
        logs_dir=tmp_path / "logs",
    )

    assert summary.reference_time == datetime(2026, 3, 11, 23, 0, 0)
    assert summary.start_time == datetime(2026, 3, 9, 0, 0, 0)
    assert summary.nt == 121

    updated_parhig = parhig_path.read_text(encoding="latin-1")
    assert "        09       03       2026        00" in updated_parhig
    assert "       121     3600." in updated_parhig
    assert "linha final preservada" in updated_parhig

    log_path = tmp_path / "logs" / "prepare_mgb_meta" / "20260311T230000.log"
    assert log_path.exists()
    assert "mgb_meta_updated" in log_path.read_text(encoding="utf-8")
