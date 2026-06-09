# Workflows operacionais

## Fluxo implementado hoje

### 1. Bootstrap do historico

1. Inicializar `<workspace>/data/history.sqlite`.
2. Carregar o inventario de estacoes em `station`.
3. Garantir catalogos basicos de `provider` e `variable`.

### 2. Ingest de observados ANA

1. Ler `config/default.yaml` e o override opcional em `<workspace>/config/custom.yaml`.
2. Buscar dados hidrometeorologicos por estacao e dia.
3. Salvar XML bruto em `<workspace>/data/interim/ana/`.
4. Persistir observados em `observed_series` e `observed_value`.
5. Registrar logs em `logs/fetch_observed_ana/`.

### 2b. Ingest de chuva INMET

1. Ler `config/default.yaml` e o override opcional em `<workspace>/config/custom.yaml`.
2. Ler a chave local em `INMET_API_KEY` ou em `.env`.
3. Consultar a API operacional de chuva por estacao e dia.
4. Salvar payload bruto em `<workspace>/data/interim/inmet/`.
5. Persistir chuva em `observed_series` e `observed_value`.
6. Registrar logs em `logs/fetch_observed_inmet/`.

### 3. Ingest de forecast ECMWF

1. Resolver o ciclo a partir do `reference_time`.
2. Baixar o GRIB do ECMWF.
3. Recortar a grade para o bbox operacional.
4. Registrar o asset canonico em `<workspace>/data/history.sqlite`.
5. Registrar logs em `logs/forecast_grid/`.

### 4. Preparacao do MGB

1. Reescrever `PARHIG.hig` a partir da configuracao atual.
2. Carregar chuva observada do historico.
3. Normalizar chuva para grade horaria e interpolar para as minis.
4. Quando habilitado, incorporar a chuva horaria do ECMWF no bloco de forecast.
5. Escrever `<workspace>/mgb_runner/Input/chuvabin.hig`.

### 5. Execucao e consumo do modelo

1. Preparar workspace do runner.
2. Executar o binario do MGB ou rodar em dry-run.
3. Espelhar o output de volta para `<workspace>/mgb_runner/Output`.
4. Ler binarios do MGB diretamente no dashboard para visualizacao.

### 6. Dashboard

1. Ler `<workspace>/data/history.sqlite` para cadastro e observados.
2. Ler binarios MGB do runner para series de mini.
3. Ler rasters acumulados em `<workspace>/data/interim/`.
4. Permitir preview e persistencia de correcoes manuais de forecast ECMWF.

## Fluxos ainda incompletos

### QC automatico

O schema e os estados existem, mas o fluxo de:

- gerar flags em `qc_flag`
- promover `raw -> curated -> approved`
- liberar automaticamente insumos aprovados

ainda nao esta operacional.

### Run operacional materializado

O schema de run existe, mas o fluxo que copia inputs, outputs, derivados, flags e lineage para `<workspace>/data/runs/<run_id>.sqlite` ainda nao esta fechado.

### Revisao manual de observados

Hoje existe correcao manual para forecast ECMWF no historico. A revisao manual de chuva observada ainda nao esta implementada.

### Relatorios

A geracao de `report_artifact` e a publicacao no `run_catalog` seguem pendentes.

## Direcao arquitetural mantida

Mesmo com lacunas na implementacao, a direcao canonica continua sendo:

- historico persistente em SQLite;
- um arquivo SQLite por run;
- assets espaciais tratados em `data/spatial/`;
- series tratadas em `data/timeseries/`;
- configuracao atual em YAML, com `.toml` ainda em avaliacao.
