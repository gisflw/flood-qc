# Local-only data and assets

The excluded files remain in this checkout at their original relative paths. The complete operational copy is also at `/workspace/Sistema_de_Previsao_RSH_2025_versao0.99/`. The full inventory is available locally at `references/inventory.full.csv`, and the external recovery bundle is under `/workspace/.codex-recovery-vbnet-20261021/`.

Key local-only locations under `Sistema_de_Previsao_RSH_2025_versao0.99/`:

- Model calibration, geometry, forcing, and hot starts: `MGB/Input/`, root `MINI*` and `*.hig`, plus `Cell*`.
- Map layers and return-period data: `Shapefiles/` and `Mapa/`.
- Historical and operational observations: `Arquivos do sistema/`.
- Plugin outputs and input layers: `Plugins/Chuva_boletim4/Shape_mini/`, `Plugins/Chuva_boletim6/Shape_mini/`, `Plugins/Executavel_baixador_IMERG_GSMAP/main_baixador_virtuais/OUTPUT_imerg_virtuais/`, `Plugins/Executavel_baixador_IMERG_GSMAP/main_baixador_virtuais/temp_imerg_raster/`, and `Plugins/Executavel_consistencia_espacial/Consistencia_espacial_v1/shape/`.
- Manuals and standalone visual assets: their original `manual/`, `FONTE_SISTEMA_PREV_RSH_2026_09_02/`, and `Plugins/` paths; all PDF, DOCX, PNG, ICO, BMP, GIF, JPG, TIFF, WEBP, and SVG files are local-only.
- Download credentials: `Plugins/Download_API_ANA/Input/login.txt` and `Plugins/Executavel_baixador_IMERG_GSMAP/main_baixador_virtuais/jaxa_gsmap_ftp_credentials.txt`.
- Identical copies under `FONTE_SISTEMA_PREV_RSH_2026_09_02/Backup/` remain local; differing backup source files remain tracked.

Git ignores the excluded paths with scoped rules. No local files were deleted.
