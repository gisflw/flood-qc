# Operacao e convencoes

## Setup local

1. Criar ambiente virtual com `Python 3.11+`.
2. Instalar dependencias com `pip install -e .[dev,data,geo,ui]`.
3. Ajustar `config/default.yaml` quando necessario para defaults operacionais.
4. Usar `<workspace>/config/custom.yaml` para overrides regionais opcionais.

Setup tipico em Linux/macOS:

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .[dev,data,geo,ui]
```

Setup tipico em Windows PowerShell:

```powershell
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install -e .[dev,data,geo,ui]
```

## Configuracao operacional

O runtime le:

- `config/default.yaml` como default empacotado;
- `<workspace>/config/custom.yaml` quando existir;
- `config/custom.yaml` como fallback de compatibilidade local.

O workspace regional e informado por `mgb-ops --workspace PATH`, por `MGB_OPS_WORKSPACE`, ou pelo diretorio atual. Cada workspace deve conter `data/`, `logs/` e `mgb_runner/`. A eventual migracao para `.toml` segue em avaliacao.

## Entry points usuais

```bash
mgb-ops --workspace examples/rs_hydro bootstrap history
mgb-ops --workspace examples/rs_hydro ingest ana
mgb-ops --workspace examples/rs_hydro ingest inmet
mgb-ops --workspace examples/rs_hydro ingest forecast-grid
mgb-ops --workspace examples/rs_hydro model prepare-meta
mgb-ops --workspace examples/rs_hydro model prepare-rainfall
mgb-ops --workspace examples/rs_hydro model run --dry-run
mgb-ops --workspace examples/rs_hydro model export-outputs
mgb-ops --workspace examples/rs_hydro dashboard
```

`mgb-ops ingest inmet` requer `INMET_API_KEY` no ambiente local ou em `.env`.

## Convencoes de nomes

- `run_id`: preferencialmente `YYYYMMDDTHHMMSS`
- `history.sqlite`: banco historico unico
- `<workspace>/data/runs/<run_id>.sqlite`: um arquivo por run
- assets externos com paths relativos sempre que possivel

## Estados de maturidade

- `raw`: dado ingerido sem revisao completa
- `curated`: dado tratado por regras automaticas ou pre-processamento
- `approved`: dado liberado para uso operacional

O schema e o consumo do dashboard ja respeitam essa convencao, embora o fluxo automatico de promocao entre estados ainda esteja pendente.

## Artefato completo vs run

O fluxo operacional atual usa diretamente os artefatos completos do runner:

- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB`
- `<workspace>/mgb_runner/Output/YTUDO.MGB`
- `<workspace>/mgb_runner/Input/PARHIG.hig`
- `<workspace>/mgb_runner/Input/MINI.gtp`

O schema de run continua previsto para guardar o subset operacional e o contexto fechado do ciclo, mas essa etapa ainda nao esta completa no pipeline atual.

## Paths de raster e vetores

- guardar path relativo no banco sempre que possivel
- nao armazenar raster como blob em SQLite
- preservar `data/spatial/` como destino canonico de camadas tratadas, mesmo que parte do consumo atual ainda use artefatos legados

## Edicao destrutiva e auditoria

- nao sobrescrever dado de origem
- registrar flags e edicoes de forma append-only quando aplicavel
- criar run manual derivado em vez de alterar um run automatico em lugar

Toda transformacao relevante deve explicitar:

- origem do dado ou asset
- momento da alteracao
- operador ou servico responsavel
- motivo da alteracao
- relacao com o run impactado, quando houver
