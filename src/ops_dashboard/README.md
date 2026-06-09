# Ops Dashboard

Entry point principal da interface operacional em Streamlit.

Uso esperado:

```bash
mgb-ops --workspace examples/rs_hydro dashboard
```

O dashboard consome:

- `<workspace>/data/history.sqlite` para cadastro de estacoes e series observadas;
- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB` e `<workspace>/mgb_runner/Output/YTUDO.MGB` para series MGB;
- `<workspace>/mgb_runner/Input/PARHIG.hig` e `<workspace>/mgb_runner/Input/MINI.gtp` para metadados e mapeamento das minis;
- `<workspace>/data/interim/accum_*h.tif` para rasters de chuva acumulada;
- `<workspace>/data/legacy/app_layers/rios_mini.geojson` para clique nas minis MGB.

Comportamento adicional:

- tema Streamlit em `.streamlit/config.toml`;
- atualizacao manual via botao `Atualizar dados` na sidebar para limpar caches e recarregar os artefatos operacionais.
