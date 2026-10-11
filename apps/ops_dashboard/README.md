# Ops Dashboard

Panel interface layer for operational monitoring and manual forecast
correction. The dashboard uses native Panel reactivity, DeckGL maps, Plotly
charts, and Tabulator editing. UI/session behavior stays here while reusable
data access, model, ingestion, and correction logic lives in `mgb_ops`.

Install and run:

```bash
uv sync --extra dashboard
uv run panel serve apps/ops_dashboard/serve.py --show --args --workspace scratch/rs_hydro
```

The dashboard consumes:

- `<workspace>/data/history.sqlite` for station registry and observed series;
- `<workspace>/data/cache/forecast_scenarios/` containing direct per-scenario NetCDF files for MGB series and, for raw/corrected scenarios, persisted forecast working grids used by the operational map;
- registered canonical forecast NetCDF assets for forecast precipitation;
- observed rainfall in `history.sqlite`, materialized through the asset-owned
  canonical precipitation cache;
- the GeoPackage configured by `spatial.gpkg_path` (default
  `<workspace>/data/source/rs_hydro.gpkg`) for clickable mini segments and catchments.

Set `spatial_grid.bbox: [west, south, east, north]` and
`spatial_grid.resolution_degrees` in `<workspace>/config/custom.yaml`.
Observed rainfall maps are read from
`<workspace>/data/cache/precipitations_mgb_observed.nc`.

Additional behavior:

- each browser session owns its selections, forecast draft, messages, and applied preview;
- the dashboard supports English (default) and Brazilian Portuguese; language choice lasts for the current browser session only;
- dashboard-owned labels and messages switch languages, while user-authored content, raw technical keys, built-in table pagination and chart toolbar controls, and external diagnostic details stay in their original language;
- manual refresh re-versions source files and refreshes only the active session;
- original and corrected forecast maps share their DeckGL view state.

Behind a reverse proxy, allow the public origin and forward WebSocket upgrades:

```bash
uv run panel serve apps/ops_dashboard/serve.py \
  --allow-websocket-origin dashboard.example.org \
  --prefix /hydrology \
  --args --workspace /srv/rs_hydro
```

The reverse proxy must preserve the `/hydrology` prefix (or rewrite it
consistently) and forward `Upgrade` and `Connection` headers.

`factory.py` creates session state and the responsive shell. Cached reads and
presentation adapters live in `services/`; Panel composition and callbacks live
in `views/`. Reusable file contracts and loading belong in `mgb_ops.assets`,
read-only hydrological calculations and projections in `mgb_ops.analysis`, and
persistence in `mgb_ops.assets`. Dashboard services may cache or reshape those
results for widgets, but must not become the owner of domain rules. The
package-level `create_dashboard` export is the supported Python embedding
interface.


The Monitoring tab selects the forecast scenario displayed on the operational map and can overlay multiple scenario results in the selected-area comparison. Forecast asset preview and correction editing continue to read
the registered asset catalog and `data/cache/current_run.sqlite` artifact directly.

The installed package also supports `mgb-ops panel --workspace /path/to/workspace`.
It opens a browser by default; pass `--no-show`
for headless serving and `--address` / `--port` to select the listening endpoint.
