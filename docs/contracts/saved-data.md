# Saved-data workflows

These workflows are available in the shared Rust library, the CLI, and the optional desktop. The desktop keeps the map open while its workers run. Each command that accepts `--request` reads an explicit JSON file unless its existing contract specifies TOML. New destinations are explicit.

| To work with | Go to |
| --- | --- |
| Observation and forecast intake, availability, or spatial QC | [Canonical data and review](#canonical-data-and-review) |
| Reservoir releases, wind, state capture, or geometry and soil edits | [Forcing, state, and calibration](#forcing-state-and-calibration) |
| Hydrographs, bulletin exports, or return-period maps | [Analysis and bulletins](#analysis-and-bulletins) |
| Run bundles, scheduled jobs, or email delivery | [Sharing and automation](#sharing-and-automation) |

## Canonical data and review

- `mgb-ops workspace init --workspace /region` initializes the workspace database. `observation import --database ... --input batch.json` accepts the serialized `ObservationBatch`. An identical row is a safe retry. Conflicting raw series or samples reject the batch. `observation inventory` accepts an array of `Station` records and updates metadata for existing provider and station identities.
- Point and period edits target a station and variable. Selected instructions apply in order, with the last matching instruction winning. A null edit excludes a value. Effective flow uses exact hourly labels and shows unchanged off-hour observations in series views.
- `mgb-ops forecast import-canonical --workspace ... --request forecast.json` accepts fields `source`, `catalog`, `artifact_root`, and `metadata`. `metadata` contains `provider`, `model`, `product`, and RFC3339 `cycle_utc`. Paths follow workspace-relative forecast rules. The source is a canonical native forecast NetCDF grid. The catalog retains the source bytes and recorded digest. Legacy-normalized GRIB assets remain readable.
- `mgb-ops observation availability --request review.json` accepts an `ObservationReviewRequest`. Select the database, station IDs or provider codes, variable, local `end`, and `window.kind` (`recent72_hours`, `month`, or `explicit` with `start`). The month query uses the calendar month containing `end`. Select series and ordered replacement IDs or inline instruction values when needed. The result distinguishes missing rows, nulls, exclusions, and valid values.
- `mgb-ops observation spatial-qc --request review.json --interpolation idw.json` compares each target station's raw rainfall to IDW from other stations' effective rainfall. Use `--station` for one target. Totals and residuals use matching valid hourly pairs. The report includes expected, observed, and matched hours. Findings do not change stored measurements.

Use **Import data**, **Observation review**, and **Review instructions** in the desktop. Draft previews have a source generation and prompt for explicit source acceptance when the selected data change. Forecast preview uses the same intervals and color scale for the original and corrected maps.

Export and edit the workspace's ordered instructions as CSV.

```sh
mgb-ops edits export --workspace /region --output replacements.csv
mgb-ops edits import --workspace /region --input replacements.csv
mgb-ops edits export --workspace /region --forecast ecmwf@2026-07-20T00:00:00Z --output corrections.csv
mgb-ops edits import --workspace /region --forecast ecmwf@2026-07-20T00:00:00Z --input corrections.csv
```

Import replaces the selected observation or forecast edit list in a single
transaction and preserves row order. A header-only CSV clears that list. An empty
observation value excludes the measurement. A forecast selection identifies one
catalog cycle. Export writes to a new path.

## Forcing, state, and calibration

The optional fields below extend operation TOML and draft JSON. Paths resolve against the workspace. Baseline operation settings remain valid when these fields are omitted.

```toml
[forcing]
reservoir_schedules = "data/reservoir.json"
wind_observations = "data/wind.json"

[[forcing.mappings.qobs]]
mini_id = 123
series_id = "station-observed-flow"

[[forcing.mappings.reservoirs]]
mini_id = 456
schedule_id = "planned-release"

[[forcing.mappings.wind]]
block = 1
series_id = "lake-wind"

[calibration]
use_edited_geometry = true
soil = "calibration/soil-edited.hig"

[hot_start]
save_at = "2026-07-11T01:00:00"
```

- Canonical schedules contain `schema_version = 1`, `time_zone`, and `samples` with `series_id`, `at`, and `flow_m3s`. Canonical wind replaces `flow_m3s` with `wind` containing `speed_mps` and `direction_degrees`. The `at` labels are local hours. Use `null` for missing values. `mgb-ops forcing import --kind reservoirs|wind --input ... --output ...` validates and stores canonical files. Select them in **Forcing and state** or in the operation document.
- Reservoir mappings can include `observed_series_id`, `schedule_id`, or `extrapolation` with `method`, `start_at`, `end_at`, `start_m3s`, and `end_m3s`. Selection order is effective observation, schedule, then extrapolation. `forcing.mappings.disable_reservoirs = true` is the explicit no-substitution mode. Mapping a forcing file replaces its engine inputs for this preparation. Missing slots receive the engine's missing sentinel. The qualified RS engine consumes wind block 1.
- Hot-start capture writes `scenarios/<id>/state/` with six verified state files and `state.json`. The supported RS save time is an hour within history on a date after the first simulation label, with hour of day at or after that label's hour. The engine build is checked at preparation and execution. `hot_start.use_bundle` identifies a requested restart. The retained RS engine rejects it because its execution path does not load state. The alternate `muskingum_cunge` routing mode also requires a qualified engine. [RS evidence](../../references/regression/rs-hourly/rust-qualification.md)
- `calibration geometry` accepts `--input`, an optional `--edits` JSON array or `--channel-table`, and `--output`. The channel table uses `mini;width;depth`. `calibration soil` accepts `--geometry`, `--hrc`, optional `--input`, optional `--edits`, and `--output`. Save to a new file. The desktop **Geometry editor** and **Soil editor** expose the same edits and can select the saved version in the draft. Geometry selection uses `geometry.mini_gtp` with `calibration.use_edited_geometry`. Preparation stages the validated files in the engine's private input and records their integrity references.

## Analysis and bulletins

- `mgb-ops series analyze --request hydrograph.json [--output graph.csv|graph.png]` accepts a regular series request in `series`, an optional `comparison_station`, `adjustment`, and `thresholds`. The request may include multiple retained runs and scenarios. Bias kinds are `none`, `recent` with `max_age_hours`, and `ar1` with optional `alpha` (0.97 by default). The adjustment may contain an additive `offset`, `band_percent`, and `rating_curve` points (`level_cm`, `flow_m3s`). Thresholds have `name` and `value`. Series exports include raw and analyzed values, dates, identity, and reported gaps. `series export-batch` accepts an array of `{name, request}` entries and publishes a new directory with CSV and PNG per entry.
- `mgb-ops report generate --request bulletin.json` accepts `trial`, `scenario_id`, `observations`, `geometry` (MINI topology), `profile` (TOML), `output`, and optional `map_geometry` (vector source). The profile selects `title`, `basin`, `outlet_mini_id`, `history_hours`, `forecast_hours`, `forecast_lead_hours`, signed `rainfall_windows`, and `[[stations]]`. Set `comparison_scenarios` to include min/max forecasts and coverage at each requested lead. Each station supplies `station_id` and can specify `mini_id`, labels, observation series, and adjustment. Set `catchment_layer` with `map_geometry` to include a catchment map. The new output directory holds PDF, offline HTML, Markdown, PNG figures, and relevant CSVs. The desktop **Bulletins** page can preview the tables and figures before export.
- `mgb-ops report return-period --request map.json` accepts `trial`, `scenario_id`, `topology` (MINI), `geometry` (vector source), `layer`, threshold TOML `thresholds`, `forecast_day` (first day is 1), and `.geojson` `destination`. Each `[[mini]]` sets `mini_id` and nested `[[mini.thresholds]]` set `years` and `flow_m3s`. The export uses 24 exact hourly forecast cells. Incomplete coverage omits the return-period class.

## Sharing and automation

- `mgb-ops run export --run ... --output ... [--prepared ...]` writes a verified directory bundle. Export before replacing the active run to keep charts, raw results, and inundation products. Include the exact retained preparation to enable source-independent regeneration with a matching local executable. The bundle excludes the executable.
- `mgb-ops run import --workspace ... --bundle ... --output ...` verifies a bundle and copies it to an explicitly selected location outside the managed cache. Open that directory as a comparison run in the desktop. `run regenerate --workspace ... --bundle ... --engine ...` republishes the active run after successful execution with a matching local executable.
- `mgb-ops run replay --workspace ... --manifest runs/history/<run-id>.json` prepares and executes the recorded recipe using the referenced inputs. The command validates the recorded identities and reports missing or changed inputs before replacing the active run. Earlier manifests can only verify identities they recorded. Archived manifests do not contain the input data.
- `mgb-ops automation validate --workspace ... --config ...` checks a scheduler TOML. Set `schema_version = 1` and one or more `[[jobs]]` with `id`, `time_zone`, `hour`, optional `minute`, and `[[jobs.steps]]`. Step `kind` is `prepare`, `execute`, or `run_all`. Select `draft` for the draft actions or `prepared` for execute. A draft step can set `rolling = true` to create a separate draft selection at each scheduled local reference and accept its sources afresh. It preserves staged instruction values and dates and leaves the saved draft unchanged. An execution step can select `export = true`, `email = true`, and `[jobs.steps.report]` with `profile`, `observations`, `geometry`, `scenario_id`, and optional `map_geometry`.
- Run `automation serve --workspace ... --config ...` in a foreground process or start scheduling in the desktop. `automation status --workspace ...` lists durable per-slot outcomes. `automation run-now --workspace ... --config ... --job ...` runs an explicit job. The scheduler uses the workspace lock and records interrupted or missed slots without automatically rerunning them. An elapsed slot is not replayed on startup.
- Configure a local `[smtp]` table with `host`, `tls` (`implicit` or `starttls`), `from`, `to`, optional `port`, and paired `username` and `password_env` when authentication is required. Keep the password in the named environment variable. Configured execution steps send their attachments after successful production. `automation send` takes a subject, body file, and optional repeated attachment paths to deliver existing files. SMTP delivery has its own recorded outcome.
