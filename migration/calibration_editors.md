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

Implement validated Rust read/edit/write operations first, then expose them in
the desktop editor and useful CLI commands. Preserve required file ordering and
formatting. Save edited calibration explicitly and associate the selected version
with a run; protect the supplied reference baseline from accidental overwrite.

## Evidence of completion

A representative geometry and soil file can be loaded and saved without losing
data. Apply a channel/Manning change and a soil-parameter change, then verify the
resulting files are accepted by the selected MGB executable.
