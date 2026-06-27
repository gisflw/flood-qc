# Ops Dashboard

Streamlit interface layer for operational monitoring and manual ECMWF forecast
correction. The dashboard should stay thin: Streamlit rendering and session
state live here, while reusable data access, model, ingestion, and correction
logic should live in `mgb_ops` library modules.

Install and run:

```bash
python -m pip install -e '.[dashboard]'
python -m streamlit run apps/ops_dashboard/app.py -- --workspace scratch/rs_hydro
```

The dashboard consumes:

- `<workspace>/data/history.sqlite` for station registry and observed series;
- `<workspace>/data/processed/model_outputs.nc` for MGB series;
- registered canonical forecast NetCDF assets for forecast precipitation;
- observed rainfall in `history.sqlite`, accumulated and interpolated in memory;
- `<workspace>/data/legacy/app_layers/rios_mini.geojson` for clicking MGB minis.

Set `forecast_grid.bbox: [west, south, east, north]` in
`<workspace>/config/custom.yaml`. The common map resolution is controlled by
`summaries.grid_resolution_degrees` and defaults to `0.1`.

Additional behavior:

- Streamlit theme in `.streamlit/config.toml`;
- manual refresh through the `Refresh data` button in the sidebar to clear caches and reload operational artifacts.

Dashboard support helpers live in `apps/ops_dashboard/support/`.
