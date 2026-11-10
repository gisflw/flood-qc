# Completed runs and sharing

## Owns

Closing, archiving, importing/exporting, and reopening a completed operation with
enough information to regenerate its inputs and model outputs.

## Starting references

Python history, current-run artifacts, correction instructions, scenario
execution, and reporting. VB.NET batch exports and archived forecast loading
provide additional operational examples.

## Implementation direction

Preserve the selected observations and forecast inputs, settings, corrections,
calibration, initial model state, and executable identification needed to repeat
the operation. A rerun must not depend on a provider still serving the same data
or on another operator's mutable history database.

Build on the execution snapshot established by the first operational increment:
app freezes actual selected values or immutable versions at execution startup,
before later acquisitions or review edits can change them. Store owns those
contracts, provenance, and retention. Saved selectors, mutable paths, or checksums
without retained data are insufficient for regeneration. Exact archive and
storage layouts follow this capability; SQLite and NetCDF remain starting choices.

Shared app workflows coordinate closing, export, import, and reopening, using
store's persistence and file-lifecycle APIs. Model only produces staged canonical
outputs; app asks store to publish a complete scenario set. Failed attempts retain
their diagnostics/staged outputs and leave previous published results official.

Share completed operations between independent workspaces. Imported runs must
not silently overwrite local operational state. The exact archive format and
storage layout can be settled during implementation.

All caches are optional. Do not make any cache part of the regeneration contract
or spend this planning phase defining cache export options.

## Evidence of completion

Export a completed case and import it into a clean workspace with the compatible
MGB executable available. Regenerate its inputs and outputs without the original
workspace, provider access, or caches.
