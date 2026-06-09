# Arquitetura

## Visao geral

A base e local-first, orientada por arquivos e organizada em torno de artefatos reproduziveis em disco. O fluxo principal hoje depende de:

- `<workspace>/data/history.sqlite` como banco historico persistente;
- `<workspace>/mgb_runner/Input` e `<workspace>/mgb_runner/Output` como espelho local do runner do MGB;
- `<workspace>/data/interim/` para artefatos coletados ou intermediarios;
- `mgb-ops dashboard` como entrada da interface operacional.

Os componentes seguem separados por dominio:

- `src/ingest/`: coleta e registro de observados e forecast;
- `src/model/`: preparacao de insumos e execucao do MGB;
- `src/storage/`: bootstrap e contratos SQLite;
- `src/reporting/`: suporte ao dashboard e produtos de consulta;
- `src/qc/`: regras de QC e revisao, ainda incompletas nesta fase.

## Estado atual implementado

Hoje o repositorio ja entrega:

- bootstrap do historico e do schema de run;
- ingest operacional de observados ANA;
- ingest de grade ECMWF, recorte espacial e registro do GRIB no historico;
- preparacao da chuva horaria para o MGB a partir de observados e forecast ECMWF;
- execucao real ou dry-run do runner MGB via `mgb-ops model run`;
- dashboard Streamlit para observados, series MGB e preview/correcao manual de forecast ECMWF.

Ainda nao entrega ponta a ponta:

- ingest operacional de INMET;
- QC automatico de observados;
- correcao manual de chuva observada;
- montagem completa de runs em `<workspace>/data/runs/<run_id>.sqlite`;
- relatorios operacionais.

## Decisoes arquiteturais

### SQLite como baseline

SQLite e o baseline operacional para reduzir dependencia externa, manter backup simples e preservar auditabilidade local. O historico e o schema de run ficam explicitos em SQL.

### Historico + run por arquivo

O contrato continua sendo:

- `<workspace>/data/history.sqlite` para historico persistente;
- `<workspace>/data/runs/<run_id>.sqlite` para contexto fechado de um run.

O bootstrap desse modelo esta implementado. A materializacao operacional completa do run ainda nao esta fechada.

### Observados em formato long

Observados entram em formato long, com uma serie por combinacao relevante de estacao, variavel e estado. Esse desenho ja esta em uso no historico e no dashboard.

### Assets externos fora do banco

Rasters, vetores e binarios MGB permanecem fora do SQLite. O banco guarda metadados e paths relativos. Isso vale tanto para GRIB ECMWF quanto para outputs completos do MGB.

### Streamlit como UI principal

O Streamlit segue como interface principal para triagem operacional. Hoje ele consome diretamente:

- `<workspace>/data/history.sqlite`;
- binarios MGB do runner;
- rasters acumulados em `<workspace>/data/interim/`;
- artefatos espaciais legados ainda usados pelo mapa.

### QGIS como complementar

QGIS continua como cliente complementar sobre artefatos gerados. O layout canonico reserva `data/spatial/` para camadas tratadas estaveis, embora essa consolidacao ainda esteja incompleta.

### Runner MGB isolado

O executavel e os artefatos do MGB permanecem isolados em `<workspace>/mgb_runner`, sob responsabilidade do usuario/regiao, enquanto a logica do runner e dos preparos fica em `src/model/` e e acionada pelo CLI `mgb-ops`.

## Arquitetura alvo vs estado real

Algumas decisoes seguem como alvo canonico, mas ainda nao estao totalmente materializadas:

- `data/spatial/` como local dos assets espaciais tratados;
- `data/timeseries/` como local de series tratadas operacionais;
- `<workspace>/data/runs/` como artefato efetivamente usado no ciclo operacional diario;
- `.toml` como possivel formato futuro de config, ainda em avaliacao.

Enquanto isso, o sistema ainda preserva e consome alguns artefatos legados, especialmente no dominio espacial.
