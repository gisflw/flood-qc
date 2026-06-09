# Ops Dashboard

Entry point principal da interface operacional em Streamlit.

Uso esperado:

```bash
streamlit run apps/ops_dashboard/app.py
```

O dashboard consome:

- `data/history.sqlite` para cadastro de estacoes e series observadas;
- `apps/mgb_runner/Output/QTUDO_Inercial_Atual.MGB` e `apps/mgb_runner/Output/YTUDO.MGB` para series MGB;
- `apps/mgb_runner/Input/PARHIG.hig` e `apps/mgb_runner/Input/MINI.gtp` para metadados e mapeamento das minis;
- `data/interim/accum_*h.tif` para rasters de chuva acumulada;
- `data/legacy/app_layers/rios_mini.geojson` para clique nas minis MGB.

Comportamento adicional:

- tema Streamlit em `.streamlit/config.toml`;
- atualizacao manual via botao `Atualizar dados` na sidebar para limpar caches e recarregar os artefatos operacionais.
