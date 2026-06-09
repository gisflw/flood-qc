# Ops Dashboard

Main entry point for the operational Streamlit interface.

Expected usage:

```bash
mgb-ops --workspace examples/rs_hydro dashboard
```

The dashboard consumes:

- `<workspace>/data/history.sqlite` for station registry and observed series;
- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB` and `<workspace>/mgb_runner/Output/YTUDO.MGB` for MGB series;
- `<workspace>/mgb_runner/Input/PARHIG.hig` and `<workspace>/mgb_runner/Input/MINI.gtp` for mini metadata and mapping;
- `<workspace>/data/interim/accum_*h.tif` for accumulated rainfall rasters;
- `<workspace>/data/legacy/app_layers/rios_mini.geojson` for clicking MGB minis.

Additional behavior:

- Streamlit theme in `.streamlit/config.toml`;
- manual refresh through the `Refresh data` button in the sidebar to clear caches and reload operational artifacts.
