# VB.NET reference snapshot

This parentless branch keeps the VB.NET application and plugin source as a reference for porting its code and behavior. Original relative paths are preserved beneath `Sistema_de_Previsao_RSH_2025_versao0.99/`; the operational folder is untouched.

- [Responsibility map](references/vbnet_map.md)
- [Local-only data and asset paths](references/local_data.md)
- [Retained-file inventory](references/inventory.csv), with SHA-256 hashes for every tracked file except the inventory itself.
- [Redaction record](references/redactions.json); credential values are not included in the reference.
- [Executable-only components and missing source](references/source_gaps.md)

Source code, project definitions, form resources, manuals referenced by code, and small configuration/behavior examples remain available in this checkout. Large operational datasets, manuals, standalone images, generated/downloaded outputs, and credentials are excluded from Git and kept as local files. Their categories and original paths are listed in `references/local_data.md`. The full original-file inventory is retained locally as `references/inventory.full.csv`.

A clean clone contains the porting reference but not the local-only material. Some legacy project files refer to those assets. Secret values found in source were replaced with `<REDACTED>` only in this copy; original files remain unchanged.

## Model baseline

The selected external runner is `/workspace/rs_hydro_forecast/mgb_runner/MGB_Inercial_Prev_RSH`, SHA-256 `263381e450484e5bc345e269f21c54e84d89afdff97ca2d9752eb951907f88eb`. The selected calibration/geometry source is the original `MGB/Input/` folder.

The two operational input folders and their hot-start files must not be treated as interchangeable. Regenerate and validate representative cases during model implementation.
