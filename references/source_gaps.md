# Source gaps

These deployed tools have executable files but no application source in the inspected source folder.
Do not treat their binaries as a reproducible source implementation.

- `Plugins/Chuva_boletim4`: `0_main.exe`, `A_Chuva.exe`.
- `Plugins/Chuva_boletim6`: `0_main.exe`, `A_Chuva.exe`.
- `Plugins/Download_API_ANA`: `telemetria.exe`.
- `Plugins/EditorPrevChuva`: `EditorPrevChuva.exe`.
- `Plugins/ONS`: `ONS_Horarios_Process.exe`, `ONS_Horarios_Process.vshost.exe`.
- `MGB/`: deployed Fortran model executables; no Fortran source was found.
- `Plugins/Figuras_Boletim_Chuva_Acum/`: the map names this location, but this directory is absent from the supplied snapshot.

The original operational folder retains these deployed artifacts. No binaries are versioned here.
