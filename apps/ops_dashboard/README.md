# Ops Dashboard

Panel interface layer for operational monitoring and manual ECMWF forecast
correction. The dashboard uses native Panel reactivity, DeckGL maps, Plotly
charts, and Tabulator editing. UI/session behavior stays here while reusable
data access, model, ingestion, and correction logic lives in `mgb_ops`.

Install and run:

```bash
python -m pip install -e '.[dashboard]'
panel serve apps/ops_dashboard/serve.py --show --args --workspace scratch/rs_hydro
```

The dashboard consumes:

- `<workspace>/data/history.sqlite` for station registry and observed series;
- `<workspace>/data/processed/model_outputs.nc` for MGB series;
- registered canonical forecast NetCDF assets for forecast precipitation;
- observed rainfall in `history.sqlite`, accumulated and interpolated in memory;
- the GeoPackage configured by `spatial.gpkg_path` (default
  `<workspace>/data/source/rs_hydro.gpkg`) for clickable mini segments and catchments.

Set `forecast_grid.bbox: [west, south, east, north]` in
`<workspace>/config/custom.yaml`. The common map resolution is controlled by
`summaries.grid_resolution_degrees` and defaults to `0.1`.

Additional behavior:

- each browser session owns its selections, forecast draft, messages, and applied preview;
- manual refresh re-versions source files and refreshes only the active session;
- original and corrected forecast maps share their DeckGL view state.

Behind a reverse proxy, allow the public origin and forward WebSocket upgrades:

```bash
panel serve apps/ops_dashboard/serve.py \
  --allow-websocket-origin dashboard.example.org \
  --prefix /hydrology \
  --args --workspace /srv/rs_hydro
```

The reverse proxy must preserve the `/hydrology` prefix (or rewrite it
consistently) and forward `Upgrade` and `Connection` headers.

`factory.py` creates session state and the responsive shell. Cached reads and
pure domain transforms live in `services/`; Panel composition and callbacks
live in `views/`. The package-level `create_dashboard` export is the supported
Python embedding interface.
