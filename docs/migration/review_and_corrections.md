# Review and corrections

## Owns

- Observation QC, availability views, spatial consistency diagnostics, and
  leave-one-out rainfall analysis.
- Explicit observation replacements and station exclusions, including manual
  rainfall and flow editing capabilities from the references.
- Forecast rainfall translation, rotation, multiplication, and preview.
- Persisted review instructions used consistently by preparation and reporting.

## Starting references

Python: `src/mgb_ops/qc/`, `src/mgb_ops/edit/forcing.py`,
`src/mgb_ops/analysis/precipitation.py`, `src/mgb_ops/assets/effective_observations.py`,
and the dashboard's observed-QC and forecast-correction views.

VB.NET: `Form6`, `Form7`, `Form9`, `Form13`, and spatial-consistency/rainfall-editor
plugins. See the [reference map](../../references/vbnet_map.md).

## Implementation direction

Keep original observations and forecasts intact. Apply documented review
instructions to create effective inputs. Analysis owns QC, correction evaluation,
transformations, diagnostics, and preview data, using store's canonical contracts
without adapters or model internals. Store owns persisted instructions and data;
app coordinates loading, saving, and preparation. GUI controls keep their own
interactive state and call the same library operations as the CLI.

At execution startup, app freezes the selected sources and applied correction
instructions as part of the execution snapshot. Analysis prepares canonical
forcing from that snapshot before model-specific serialization. Later review
edits affect a new snapshot, never the inputs of an existing execution.

## Evidence of completion

A small saved case demonstrates detection, review, persistence, and preparation
using the corrected values. Check transformation boundaries, missing values,
and conservation/aggregation behavior where relevant.
