# Workspace layout and operational database

Both interfaces select a workspace directory. The root contains `config.toml`
and `operation.sqlite`. Workspace-relative paths resolve from that directory.
Reading or inspecting a workspace leaves it intact.

| Directory | Contents |
| --- | --- |
| `downloads/` | Original provider payloads, grouped by provider and cycle or date |
| `model/` | MGB executable and engine input tree |
| `source/` | Regional geometry, rasters and CSV inputs |
| `assets/` | Normalized forecasts and other registered data files |
| `runs/` | Saved preparations, retained results and run history |
| `cache/` | Replaceable active results and temporary artifacts |

## Configuration and acquisition

- Extract settings, layout resolution and bounded inspection into
  `app::workspace`. One typed loader merges the root TOML over built-in
  defaults, validates effective settings, and reports unknown settings by
  key. **Omit `schema_version` from workspace configuration.**
- Keep regional paths, run defaults and acquisition settings in that file.
  Select scenarios when running. Keep observation aggregation in the adapters. Use one shared set of spatial
  bounds and one operation timezone. The prepared run freezes its reference,
  window, sources and settings.
- Choose observation and forecast providers in the TOML. Station inventory,
  enabled variables and reference data enter the database through validated CSV
  import. Keep provider schedules in the adapters. Explicit acquisition and
  acquisition-enabled runs retain original payloads under `downloads/` and
  register normalized forecast grids under `assets/`. Reuse verified local
  originals or normalized assets when they satisfy the requested inputs.

## Operational database

- Put stations, observation data, forecast metadata, corrections and run state
  in `operation.sqlite`. Initialize the full SQLite schema without version
  checks or version-based upgrades. Use internal keys for relationships and show users
  station codes and provider/product/cycle labels. Store file paths relative
  to the workspace. Share catalog access through `store` in both interfaces.
- Edit ordered replacements and corrections in the app or import/export CSV
  tables. Validate a table before applying it in one transaction. Keep raw
  observations intact. Preparation freezes the actual edits and selected
  station and forecast inputs. Refresh precipitation flags for the selected
  rainfall window. The final observation view masks flagged raw values before
  applying replacements. Freeze the flags in the preparation.
- Stage complete run files before updating the active run in SQLite. Failed or
  cancelled work leaves the current run readable. Store owns file and database
  lifecycle under the [architecture rules](../../docs/architecture.md#ownership).

## Acceptance

- CLI and desktop resolve the same locations and settings. Missing optional
  settings use defaults. Opening and inspection create no workspace files.
- Synthetic cases cover download and normalized-file reuse, ordered edits,
  snapshot isolation, and run publication under the new layout. Verify an
  offline RS preparation through the public entry points on Linux.
- Run the default and desktop Rust checks, including the standalone
  architecture gate. Qualify Windows separately. Update the
  [workspace contract](../../docs/contracts/workspace.md) with supported
  behavior after implementation.

The binary-local CLI modules and workspace catchment raster generation follow
configuration and database wiring.
