# ANA, ECMWF, and NOAA acquisition

This increment implements the three providers from the retained Python
acquisition workflow. The [migration acquisition scope](../../docs/migration/acquisition.md)
remains the reference for subsequent adapters, including INMET.

- ANA bootstraps canonical stations from the regional inventory, requests gaps
  by station and variable, and refreshes the reference day. Raw XML and
  normalized CSV are retained. Replay and live acquisition use the same hourly
  aggregation and transactional raw-series revision path.
- ECMWF and NOAA keep the legacy products, cycle schedules, lead schedules,
  and buffered bounds. NOAA acquisition follows the provider's current request
  spacing. Complete verified GRIB2 and native artifacts use the immutable
  forecast catalog.
- Standalone CLI and desktop actions share the acquisition workflow.
  Acquisition-enabled Run All resolves provider scenarios to concrete asset IDs,
  validates and freezes the input selection, then uses the existing execution
  and publication path. Explicitly pinned scenarios keep their selections.

Acceptance covers source request shape, gaps and window selection, raw retention,
CSV replay, revisions, full forecast schedules and reuse, partial failure and
cancellation, and active-generation preservation. Run both Rust check sets,
exercise the desktop path on Linux with software rendering, and record provider
qualification separately from offline fixtures. The
[acquisition contract](../../docs/contracts/acquisition.md) defines the operator
interfaces.
