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

The [input-set contract](../contracts/inputs.md) specifies saved inputs and the
required installed engine identity. The [operation
contract](../contracts/operations.md#active-run-and-rainfall-cache) specifies
active results and archived attempt manifests. The [sharing
contract](../contracts/saved-data.md#sharing-and-automation) covers bundle
export, import and regeneration.

## Evidence of completion

Compare executed outputs with a repeat from independently saved inputs after
removing acquisition assets from the disposable workspace. Check export and
import both with and without inputs, engine mismatch, incomplete or modified
payloads, and preservation of the previous active results on failure. Track
platform qualification and open checks in the [migration checklist](summary.md).
