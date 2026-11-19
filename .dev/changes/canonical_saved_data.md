# Canonical saved-data migration

This increment follows the [migration checklist](../migration/summary.md). The operator path starts with Rust canonical data. Source acquisition, legacy-source adapters, and assistant integration remain later phases.

## Shared workflows

- Ingest canonical saved inputs and review rainfall and flow through ordered, auditable instructions. Preserve v1 and v2 observation reads and require an explicit upgrade for stored flow edits.
- Freeze generated forcing, edited geometry and soil, and the engine selection in the existing preparation flow. Qualify serializers and state capture against the selected RS executable. Engine-dependent options fail before launch when the selected capability is unavailable.
- Compare retained runs and generate portable analysis, bulletin and GIS artifacts through shared library calls. Stage run bundles and report packages with no destination replacement. Regenerate from an explicitly matched preparation and local engine.
- Run saved-data jobs through the foreground CLI or desktop scheduler. Preserve independent stage and email outcomes. Local credentials stay outside run exports.

## Acceptance

Use focused fixtures for store rollback and compatibility, edits and engine formats, full report packages, run sharing, and scheduler slot/delivery failures. Rerun the saved RS release regression. On Linux, verify the selected calibration reaches the actual engine and that a state request produces a complete retained bundle. Record the limitations of the supplied executable and the platform evidence in the migration checklist and regression record.
