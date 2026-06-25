# Examples

## `prepare_mgb_inputs_pipeline.py`

This file is a notebook-style Python script. Open it in an editor that supports
percent cells, such as VS Code, Spyder, or Jupyter-compatible tooling, and run
the `# %%` cells from top to bottom.

Edit the constants near the top before running:

- `WORKSPACE`: regional MGB operations workspace.
- `OBSERVED_PROVIDERS`: defaults to `("ana",)`. Add `"inmet"` only when
  `INMET_API_KEY` is available in the environment or workspace `.env`.
- `OBSERVED_STATION_CODES_BY_PROVIDER`: optional station-code filters; leave
  values as `None` to use all registered stations for each provider.
- `INITIALIZE_HISTORY` and `HISTORY_STATION_INVENTORY_CSV`: optional bootstrap
  for a new `data/history.sqlite`.
- `PARHIG_PATH`, `MINI_GTP_PATH`, and `CHUVABIN_PATH`: MGB input file paths.

The workspace should keep the standard local-first layout:

```text
<workspace>/
  config/custom.yaml
  data/
  logs/
  mgb_runner/Input/PARHIG.hig
  mgb_runner/Input/MINI.gtp
```

When `mgb.use_forecast_data: true`, `config/custom.yaml` must include the
forecast grid crop settings:

```yaml
forecast_grid:
  bbox: [west, south, east, north]
  buffer_fraction: 0.25
```

ECMWF forecast ingestion writes canonical CF-style NetCDF assets to
`data/downloads/ecmwf/` and registers only those `.nc` files in history. GRIB2 is
downloaded, cropped, and read inside `mgb_ops.adapters.forecast_ecmwf`; downstream
rainfall preparation consumes the NetCDF contract through `mgb_ops.model`.

Install the optional forecast dependencies in the runtime environment:

```bash
python -m pip install -e ".[forecast]"
```

- `ecmwf-opendata` for downloading deterministic ECMWF source GRIB assets.
- `eccodes` and its Python bindings for adapter-internal GRIB2 reading and cropping.
- The usual data dependencies such as `numpy`, `pandas`, `xarray`, `netCDF4`, and `requests`.
