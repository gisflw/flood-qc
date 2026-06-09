# Sistema Operacional de Hidrologia e Previsao

Base operacional local-first para hidrologia, chuva e previsao, orientada por workspaces regionais, SQLite e um CLI instalavel.

## Estado atual

O repositorio ja possui base funcional para:

- bootstrap de `<workspace>/data/history.sqlite` e `<workspace>/data/runs/<run_id>.sqlite`;
- ingest de observados ANA para `rain`, `level` e `flow`;
- ingest de grade ECMWF e registro do GRIB canonico no historico;
- preparacao de metadados e chuva horaria para o MGB;
- execucao real ou dry-run do runner do MGB;
- dashboard Streamlit para monitoramento, series MGB e preview/correcao manual de forecast ECMWF.

Ainda estao pendentes nesta fase:

- ingest operacional de chuva do INMET;
- QC automatico de observados;
- correcao manual de chuva observada;
- materializacao completa de runs operacionais em `<workspace>/data/runs/`;
- geracao de relatorios operacionais.

## Principios

- artefatos locais primeiro;
- SQLite como baseline operacional;
- um banco historico persistente em `<workspace>/data/history.sqlite`;
- um arquivo SQLite por run em `<workspace>/data/runs/`;
- rasters e vetores fora do banco, com paths relativos e metadados;
- Streamlit como interface principal;
- QGIS como cliente complementar sobre artefatos gerados.

## Layout principal

```text
.
|-- config/
|-- docs/
|-- examples/
|   `-- rs_hydro/
|       |-- data/
|       |-- logs/
|       `-- mgb_runner/
|-- sql/
|-- src/
|   |-- mgb_ops/
|   |-- ops_dashboard/
|   |-- common/
|   |-- ingest/
|   |-- model/
|   |-- qc/
|   |-- reporting/
|   `-- storage/
`-- tests/
```

Importante: o usuario e responsavel por fornecer um workspace regional contendo `data/`, `logs/` e `mgb_runner/`. O repositorio inclui `examples/rs_hydro/` como workspace de teste com os artefatos do RS.

## Runtime e configuracao

- Contrato oficial de runtime: `Python >= 3.11`
- Configuracao canonica nesta fase:
  - `config/default.yaml` como default empacotado;
  - `<workspace>/config/custom.yaml` como override regional opcional;
  - `config/custom.yaml` continua aceito para compatibilidade local.
- Se `--workspace` nao for informado, o CLI usa `MGB_OPS_WORKSPACE` e depois o diretorio atual.
- A avaliacao de migracao da configuracao para `.toml` segue em aberto, sem mudanca de contrato por enquanto.

O inventario inicial de estacoes fica em `<workspace>/data/interim/history_station_inventory.csv`. Durante o bootstrap do historico, o sistema calcula `station_uid` como `1000000000 + codigo` para ANA e `2000000000 + codigo` para INMET, convertendo letras do codigo para numeros (`A=1`, `B=2`, etc.).

## Setup local

Crie um ambiente virtual e instale as dependencias para uso completo local:

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .[dev,data,geo,ui]
```

No Windows PowerShell:

```powershell
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install -e .[dev,data,geo,ui]
```

## Entry points canonicos

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

Para rodar a ingestao INMET, defina `INMET_API_KEY` no ambiente ou preencha `.env` a partir de `.env.example`.

## Componentes principais

- `src/ops_dashboard/`
  Dashboard operacional em Streamlit para monitoramento, series observadas, series MGB e preview/correcao de forecast ECMWF.
- `<workspace>/mgb_runner/`
  Artefatos regionais do MGB (`Input`, `Output` e `.exe`) fornecidos pelo usuario. O codigo do runner fica em `src/model/`.
- `src/mgb_ops/`
  CLI `mgb-ops` que executa os comandos headless e inicia/imprime o dashboard.
- `src/`
  Modulos por dominio, separados entre ingestao, QC, modelo, storage, reporting e utilitarios comuns.
- `sql/`
  Schemas explicitos de `history.sqlite` e `run.sqlite`.
- `docs/`
  Arquitetura, modelo de dados, operacao e workflows.

## Banco historico vs banco de run

- `<workspace>/data/history.sqlite`
  Guarda metadados de estacoes, observados, flags, edicoes e catalogo de runs.
- `<workspace>/data/runs/<run_id>.sqlite`
  Guarda o estado fechado de um run especifico.

O schema de run existe e o bootstrap esta implementado, mas a montagem operacional completa do run ainda nao esta concluida nesta fase.
