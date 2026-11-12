# Completed runs and sharing

## Owns

Closing, archiving, importing/exporting, and reopening a completed operation with
a recorded recipe for regenerating its inputs and model outputs when referenced
dependencies are available. Automated archive management is outside the current
operational increment.

## Starting references

Python history, current-run artifacts, correction instructions, scenario
execution, and reporting. VB.NET batch exports and archived forecast loading
provide additional operational examples.

## Implementation direction

The first operational increment generates a per-run JSON manifest and numerical
outputs for CLI/desktop consumption. It records selected
settings, ordered replacement/correction values, input identities/references,
scenario outcomes and output locations. Actual working inputs are frozen before
execution so later acquisitions/edits cannot change that execution.

Operators may copy the artifact, input dependencies, results and cache outputs
elsewhere for long-term storage. Add no retention flags, cleanup policy or archive
manager to the saved-run increment. Copying only the recipe does not preserve the
referenced bytes; regeneration requires those inputs and a compatible executable
to remain available. Caches may avoid recomputation but do not recover missing
source dependencies by implication.

Later completed-run sharing can build on these generated files. Imported runs
must not silently overwrite local data. Each execution has independent outcomes;
failed runs preserve diagnostics and successful siblings' outputs without an
official-set pointer. Any future automated export/import capability must state
which dependencies it packages rather than promise unconditional regeneration.

## Evidence of completion

Verify the GUI can consume the generated run JSON manifest and outputs. For a copied
case with all referenced dependencies and a compatible engine available,
regenerate inputs/results and compare against the saved evidence. Missing
references fail explicitly; a recipe alone does not guarantee regeneration.
Sharing/import acceptance belongs to that later capability.
