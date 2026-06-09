# Modelo conceitual de dados

## Estado atual

O modelo canonico continua dividido entre:

- historico persistente em `<workspace>/data/history.sqlite`;
- run por arquivo em `<workspace>/data/runs/<run_id>.sqlite`.

Hoje o historico esta em uso real por ANA, ECMWF, dashboard e correcoes manuais de forecast. O schema de run ja existe, mas sua materializacao operacional ainda nao esta completa.

## Entidades principais do historico

### `provider`

Catalogo das origens operacionais, incluindo pelo menos `ana`, `inmet` e `ecmwf`.

### `variable`

Catalogo das variaveis canonicas. Nesta fase, o historico trabalha com:

- `rain`
- `level`
- `flow`

### `station`

Cadastro operacional unificado das estacoes. A identidade logica continua sendo `provider_code + station_code`.

O inventario inicial vem de `<workspace>/data/interim/history_station_inventory.csv`. O bootstrap calcula `station_uid` por provider, incluindo codigos alfanumericos do INMET.

### `observed_series`

Define uma serie observada por combinacao de:

- `station_uid`
- `variable_code`
- `state`

Os estados canonicos continuam sendo:

- `raw`
- `curated`
- `approved`

No estado atual do repositorio, o historico em uso ainda esta predominantemente em `raw`.

### `observed_value`

Tabela temporal em formato long, com um valor por `series_id + observed_at`.

### `asset`

Registro generico de arquivos externos. Hoje ele ja e usado de forma operacional para assets ECMWF.

### `qc_flag`

Estrutura canonica para flags de qualidade sem sobrescrever o dado original. O schema esta implementado, mas o QC automatico ainda nao popula essa tabela de forma operacional.

### `manual_edit`

No historico atual, esta tabela esta sendo usada para correcoes manuais de forecast ECMWF por asset e janela temporal. Ainda nao existe contrato equivalente implementado para correcao manual de chuva observada.

### `run_catalog`

Indice de runs publicados ou disponiveis. O schema existe, mas o catalogo ainda nao esta sendo alimentado no fluxo atual.

## Entidades principais do run

O banco de run continua modelado para guardar:

- cabecalho do run em `run`;
- copia local de inputs em `run_input_series` e `run_input_value`;
- assets do run em `run_asset`;
- derivados operacionais em `derived_series` e `derived_value`;
- execucao do modelo em `model_execution`;
- subset operacional dos outputs do MGB em `mgb_output_series` e `mgb_output_value`;
- flags, edicoes e relatorios locais.

Esse contrato permanece valido, mas a camada de repositorio e montagem do run ainda esta incompleta nesta fase.

## Separacao entre historico e outputs completos

O output completo do MGB continua fora do SQLite, nos binarios canonicos do runner:

- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB`
- `<workspace>/mgb_runner/Output/YTUDO.MGB`

O dashboard le esses binarios diretamente com apoio de:

- `<workspace>/mgb_runner/Input/PARHIG.hig`
- `<workspace>/mgb_runner/Input/MINI.gtp`

Esse comportamento ja esta implementado e e o caminho operacional atual para visualizacao do modelo.

## Assets espaciais e raster

O contrato segue sendo:

- rasters e vetores ficam fora do SQLite;
- o banco guarda apenas metadados e paths relativos.

`<workspace>/data/spatial/` continua sendo o destino canonico dos assets espaciais tratados, mas o mapa do dashboard ainda depende de material legado em `<workspace>/data/legacy/app_layers/`.

## Configuracao

A configuracao operacional do repositorio continua em:

- `config/default.yaml`
- `<workspace>/config/custom.yaml` quando existir

A possivel migracao para `.toml` segue em avaliacao e ainda nao altera o modelo de dados nem o contrato de runtime desta fase.

## Schemas canonicos

Os schemas canonicos implementados seguem em:

- `sql/history_schema.sql`
- `sql/run_schema.sql`
