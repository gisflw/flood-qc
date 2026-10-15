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

Share completed operations between independent workspaces. Imported runs must
not silently overwrite local operational state. The exact archive format and
storage layout can be settled during implementation.

All caches are optional. Do not make any cache part of the regeneration contract
or spend this planning phase defining cache export options.

## Evidence of completion

Export a completed case and import it into a clean workspace with the compatible
MGB executable available. Regenerate its inputs and outputs without the original
workspace, provider access, or caches.
