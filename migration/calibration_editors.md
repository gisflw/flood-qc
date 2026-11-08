# Calibration editors

## Owns

- Geometry-file loading, validation, editing, and saving.
- Channel width/depth equations and table-based changes.
- Global and subbasin Manning-coefficient changes.
- Soil-parameter loading, creation, editing, and copying between subbasins.

## Starting references

`Plugins/Form_Editor/Form_Editor/` and `Plugins/Form_parcal/Form_parcal/` in the
VB.NET reference, together with the selected `MINI.gtp` and soil-parameter files.

## Implementation direction

Port these editors as functionality in the Rust package. They do not require a
regression target or replacement of the runtime's existing calibration/geometry.
Define their design, specification, functional acceptance, and review within
their implementation increment.

Implement validated Rust read/edit/write operations first, then expose them in
the desktop editor and useful CLI commands. Preserve required file ordering and
formatting. Save edited calibration explicitly and associate the selected version
with a run; protect the supplied reference baseline from accidental overwrite.

## Evidence of completion

Demonstrate loading, editing, and saving representative geometry and soil files
without losing data, using the functional acceptance criteria agreed during
implementation. Preserve required model-format compatibility and protect the
supplied baseline. No regression comparison is required for these editors.
