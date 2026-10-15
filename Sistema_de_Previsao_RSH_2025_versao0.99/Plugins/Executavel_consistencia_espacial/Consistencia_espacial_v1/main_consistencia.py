"""

Interface para download de dados de estações virtuais do GSMAP e IMERG 
Ingrid Petry

"""

import sys
from PyQt5.QtWidgets import QApplication, QMainWindow, QMessageBox, QDialog
from PyQt5.uic import loadUi
from pathlib import Path


from typing import Optional
import pandas as pd
import numpy as np
from collections import deque
from io import StringIO
from urllib.parse import urlsplit, unquote
from datetime import datetime, timedelta, timezone
import pyogrio
import matplotlib.pyplot as plt
import matplotlib as mpl
from scipy.spatial import distance


class MainWindow(QDialog):
    def __init__(self):
        super().__init__()

        def app_base_dir() -> Path:
            # Folder where the exe is located (frozen) or where the .py is (dev)
            if getattr(sys, "frozen", False):
                return Path(sys.executable).resolve().parent
            return Path(__file__).resolve().parent

        self.BASE_DIR = app_base_dir()

        # Go up 3 levels safely
        try:
            self.DATA_DIR = self.BASE_DIR.parents[2]  # 0=1 level up, 1=2 levels, 2=3 levels
        except IndexError:
            raise RuntimeError("Cannot go 3 folders up from BASE_DIR")

        print("BASE_DIR:", self.BASE_DIR)
        print("DATA_DIR:", self.DATA_DIR)

        # # Define important paths
        self.ESTACOES_PLUV_PATH = self.DATA_DIR / "EstacoesPluv.txt"
        self.DATABASE_DIR = self.DATA_DIR / "database"


        print("EstacoesPluv:", self.ESTACOES_PLUV_PATH)
        print("Database:", self.DATABASE_DIR)

        ui_path = self.BASE_DIR / "interface_consistencia.ui"
        loadUi(str(ui_path), self)

        self.setup_connections()

        # Offset fixo UTC-3
        self.LOCAL_TZ = timezone(timedelta(hours=-3), name="UTC-3")
        self.UTC = timezone.utc

        self.base_shape = self.BASE_DIR / "shape" / "mgb_rs_hidrologico_reproj.shp"
        print(self.base_shape)

        print("Full path:", self.base_shape)
        print("Exists?", self.base_shape.exists())

                
        if self.base_shape.exists():
            print("Arquivo encontrado ✅")
        else:
            print("Arquivo NÃO encontrado ❌")


        self.progressBar.setValue(0)
        self.pushButton_3.setEnabled(False)   
        self.pushButton_4.setEnabled(False)   



    def setup_connections(self):
        """
        Connect UI elements (buttons, inputs, etc) to functions here

        """
        self.pushButton_2.clicked.connect(self.limiares)
        self.pushButton_3.clicked.connect(self.plot_obs_accum_map)
        self.pushButton_4.clicked.connect(self.plot_error_map)
        self.pushButton_5.clicked.connect(self.close)

        pass


    def limiares(self):

        self.WINDOW_HOURS = int(self.lineEdit.text())
        self.ERROR_THRESHOLD = int(self.lineEdit_2.text())
        self.MISSING_THRESHOLD = int(self.lineEdit_3.text())
        self.MAX_DISTANCE = int(self.lineEdit_4.text())

        print(self.WINDOW_HOURS)
        print(self.ERROR_THRESHOLD)
        print(self.MISSING_THRESHOLD)
        print(self.MAX_DISTANCE)

        j = self.series_periodo()

        if j ==0:
            self.label_8.setText(f"2\\2 - Interrompido. Atualize o database")
        elif j==1:
            self.run_analysis()


    def _tail_text_lines(self, path, n_lines, encoding="latin1"):
        with open(path, "r", encoding=encoding, errors="ignore") as f:
            tail = deque(f, maxlen=n_lines)
        return "".join(tail)

    def read_station_last_hours(
        self,
        codigo: str,
        folder: Path,
        end_time: pd.Timestamp,                 # 👈 global reference
    ) -> Optional[pd.Series]:
        """Read rainfall for the last WINDOW_HOURS ending at end_time (global), for one station."""
        path = folder / f"P{codigo}.txt"
        if not path.exists():
            print(f"⚠️ Arquivo não encontrado: {path}")
            return None

        try:
            safety_margin = max(48, int(self.WINDOW_HOURS * 1.0))
            lines_to_read = self.WINDOW_HOURS + safety_margin

            raw = self._tail_text_lines(path, n_lines=lines_to_read)
            if not raw.strip():
                return None

            df = pd.read_csv(
                StringIO(raw),
                sep=r"\s+",
                header=None,
                engine="python",
                usecols=[0, 1, 2, 3, 4],
                names=["Dia", "Mes", "Ano", "Hora", "Valor"],
                on_bad_lines="skip",
            )

            # numeric coercion (drops header/junk lines)
            for c in ["Dia", "Mes", "Ano", "Hora", "Valor"]:
                df[c] = pd.to_numeric(df[c], errors="coerce")
            df = df.dropna(subset=["Dia", "Mes", "Ano", "Hora"])

            df["Datetime"] = pd.to_datetime(
                dict(
                    year=df["Ano"].astype(int),
                    month=df["Mes"].astype(int),
                    day=df["Dia"].astype(int),
                    hour=df["Hora"].astype(int),
                ),
                errors="coerce",
            )

            df["Valor"] = df["Valor"].replace(-1, np.nan)
            df = df.dropna(subset=["Datetime"]).set_index("Datetime").sort_index()

            if df.empty:
                return None

            # ✅ global window (same for every station)
            end_time = pd.Timestamp(end_time).floor("h")
            start_time = end_time - pd.Timedelta(hours=self.WINDOW_HOURS - 1)

            # Slice and align to full hourly index
            s = df["Valor"].loc[start_time:end_time]
            full_idx = pd.date_range(start=start_time, end=end_time, freq="h")
            s = s.reindex(full_idx)

            # if it's entirely empty, treat as missing station
            if s.isna().all():
                return None

            s.index.name = "Datetime"
            return s

        except Exception as e:
            print(f"❌ Erro ao ler {codigo}: {e}")
            return None



    def series_periodo(self):

        print("Iniciando processamento...\n")


        self.estacoes_df = pd.read_csv(self.ESTACOES_PLUV_PATH, sep=";", encoding="latin1")

        # Read all stations
        all_stations = {}

        self.label_8.setText(f"1\\2 - Lendo dados das últimas {self.WINDOW_HOURS} horas...")

        total = len(self.estacoes_df)

        self.progressBar.setMinimum(0)
        self.progressBar.setMaximum(total)
        self.progressBar.setValue(0)

        
        global_end = pd.Timestamp.now().floor("h")

        for i, codigo in enumerate(self.estacoes_df["CODIGO"].astype(str), start=1):
            codigo = codigo.strip()

            serie = self.read_station_last_hours(codigo, self.DATABASE_DIR, end_time=global_end)

            if serie is not None:
                all_stations[codigo] = serie

            self.progressBar.setValue(i)
            QApplication.processEvents()

        # ✅ Now everything is aligned already
        df_all = pd.DataFrame(all_stations)

        if df_all.empty:
            print("df_all está vazio")
            i = 0

        else:
                
            i = 1
            print("df_all tem dados")
            print(f'Início: {df_all.index[0]} - Fim: {df_all.index[-1]}')

            # Identify valid stations (you already did this)
            nan_counts = df_all.isna().sum()

            self.valid_codes = nan_counts[nan_counts <= self.MISSING_THRESHOLD].index

            # Create a new DataFrame keeping only valid stations
            self.df_valid = df_all.loc[:, self.valid_codes]

            # Identify valid and invalid stations
            nan_counts = df_all.isna().sum()
            valid_codes = nan_counts[nan_counts <= self.MISSING_THRESHOLD].index
            invalid_codes = nan_counts[nan_counts > self.MISSING_THRESHOLD].index

            # Filter metadata for valid and invalid stations
            self.estacoes_df_valid = self.estacoes_df[self.estacoes_df['CODIGO'].isin(self.valid_codes)]

            self.estacoes_df_invalid = self.estacoes_df[self.estacoes_df["CODIGO"].isin(invalid_codes)].copy()

            print(f'{len(invalid_codes)} estacoes invalidas')
            print(f'Codigos validos: {self.valid_codes}')

        return i

    def idw_interpolation(self, target_coord, used_coords, neigh_vals, power=2):
        """
        Inverse Distance Weighting interpolation.

        Parameters
        ----------
        target_coord : array-like
            [lat, lon] of the target point.
        used_coords : array-like
            Nx2 array of [lat, lon] for neighbors.
        neigh_vals : array-like
            N array of values at neighbors.
        power : float, optional
            Weighting power (default = 2).
        """
        # Compute distances
        dists = distance.cdist([target_coord], used_coords)[0]

        # Handle zero distance (same point)
        if np.any(dists == 0):
            return neigh_vals[dists == 0][0]

        # Compute weights
        weights = 1 / np.power(dists, power)
        weights /= np.sum(weights)

        # Weighted average
        int_val = np.sum(weights * neigh_vals)
        return int_val


    def interpolate_station_timeseries(self, station, df_p, estacoes, MAX_DISTANCE):
        """
        Interpolates the hourly time series for a given station using IDW.
        """
        coords = estacoes.set_index("CODIGO")[["LAT", "LON"]].to_dict("index")
        target_coord = [coords[station]["LAT"], coords[station]["LON"]]

        # Compute distances to all other stations
        all_stations = [c for c in coords.keys() if c != station]
        all_coords = np.array([[coords[c]["LAT"], coords[c]["LON"]] for c in all_stations])
        target = np.array([target_coord])
        dists = distance.cdist(target, all_coords)[0]

        # Filter neighbors within MAX_DISTANCE
        neighs = [s for s, d in zip(all_stations, dists) if d <= (MAX_DISTANCE/100)]

        interp_vals = []

        for t in df_p.index:
            neigh_vals = df_p.loc[t, neighs].dropna()
            if len(neigh_vals) == 0:
                interp_vals.append(np.nan)
                continue
            used_coords = np.array([[coords[n]["LAT"], coords[n]["LON"]] for n in neigh_vals.index])
            val = self.idw_interpolation(target_coord, used_coords, neigh_vals.values)
            interp_vals.append(val)

        return pd.Series(interp_vals, index=df_p.index, name="Interpolada"), neighs


    def compute_station_accum(self, series: pd.Series) -> tuple[float, float]:
        series = series.sort_index()
        missing_ = series.isna().sum()
        total = np.nan if series.notna().sum() == 0 else float(series.sum())
        return total, missing_


    def compute_station_error(self, obs_accum: float, interp_accum: float) -> dict:
        """Compute absolute error."""
        if np.isnan(obs_accum) or np.isnan(interp_accum):
            return {"ERRO_ABS": np.nan}
        erro_abs = abs(obs_accum - interp_accum)
        return {"ERRO_ABS": erro_abs}


    def plot_obs_accum_map(self):

        df = self.results_intermediario.copy()

        df["LAT"] = pd.to_numeric(df["LAT"], errors="coerce")
        df["LON"] = pd.to_numeric(df["LON"], errors="coerce")
        df["OBS_72H"] = pd.to_numeric(df["OBS_72H"], errors="coerce")

        df = df.replace([np.inf, -np.inf], np.nan).dropna(subset=["LAT", "LON", "OBS_72H"])

        if df.empty:
            QMessageBox.warning(self, "Plot", "No valid LAT/LON/OBS_72H to plot.")
            return

        fig, ax = plt.subplots(figsize=(6, 5))

        gdf_rs = pyogrio.read_dataframe(str(self.base_shape))  # returns GeoDataFrame
        gdf_rs.plot(ax=ax, color="none", edgecolor="gray", linewidth=0.6, zorder = 1)

        vals = df["OBS_72H"].to_numpy()

        # This line sets the size of each scatter plot point based on precipitation value.
        # If a station's value is zero, the point size is 10 (smaller); otherwise, it is 30 (larger).
        sizes = np.where(vals == 0, 5, 30)

        sc = ax.scatter(
            df["LON"].to_numpy(),
            df["LAT"].to_numpy(),
            c=vals,
            cmap="rainbow_r",
            s=sizes,
            marker="o",
            alpha=0.8,
            zorder=3
        )

        # --- overlay invalid stations (not used in analysis) ---
        if hasattr(self, "estacoes_df_invalid") and not self.estacoes_df_invalid.empty:
            inv = self.estacoes_df_invalid.copy()
            inv["LAT"] = pd.to_numeric(inv["LAT"], errors="coerce")
            inv["LON"] = pd.to_numeric(inv["LON"], errors="coerce")
            inv = inv.dropna(subset=["LAT", "LON"])

            ax.scatter(
                inv["LON"].to_numpy(),
                inv["LAT"].to_numpy(),
                s=10,
                marker=".",
                alpha=0.9,
                color = 'black',
                linewidths=1.2,
                label=f"Inválidas (>{self.MISSING_THRESHOLD}h de falha)", 
                zorder = 2
            )
            ax.legend(loc="lower right")

        cbar = fig.colorbar(sc, ax=ax)
        cbar.set_label("(mm)")

        ax.set_title(f"Chuva acumulada últimas {self.WINDOW_HOURS} horas")
        ax.set_xlabel("Longitude")
        ax.set_ylabel("Latitude")
        ax.grid(True, alpha=0.3)

        fig.tight_layout()
        plt.show()


    def plot_error_map(self):
        """Plot map of absolute error with discrete color bins (no GeoDataFrame)."""
        df = self.results_intermediario.copy()

        df["LAT"] = pd.to_numeric(df["LAT"], errors="coerce")
        df["LON"] = pd.to_numeric(df["LON"], errors="coerce")
        df["ERRO_ABS"] = pd.to_numeric(df["ERRO_ABS"], errors="coerce")

        df = df.replace([np.inf, -np.inf], np.nan).dropna(subset=["ERRO_ABS", "LAT", "LON"])

        if df.empty:
            QMessageBox.warning(self, "Plot", "No valid LAT/LON/ERRO_ABS to plot.")
            return


        thr = float(self.ERROR_THRESHOLD)

        # discrete bins: [0,thr), [thr,2thr), [2thr, inf)
        bounds = [0.0, thr, 2.0 * thr, np.inf]
        labels = [f"< {thr:g}", f"{thr:g}–{2*thr:g}", f"> {2*thr:g}"]
        colors = ["#FDE725", "#21908C", "#440154"]

        cmap = mpl.colors.ListedColormap(colors)
        norm = mpl.colors.BoundaryNorm(bounds, cmap.N)

        fig, ax = plt.subplots(figsize=(6, 5))
        
        gdf = pyogrio.read_dataframe(str(self.base_shape))  # returns GeoDataFrame
        gdf.plot(ax=ax, color="none", edgecolor="gray", linewidth=0.6)

        vals = df["ERRO_ABS"].to_numpy()

        # avoid zero-size markers
        vals_clean = np.nan_to_num(vals, nan=0.0)

        # scale sizes (tune these numbers if needed)
        min_size = 20
        max_size = 120

        if vals_clean.max() > 0:
            sizes = min_size + (vals_clean / vals_clean.max()) * (max_size - min_size)
        else:
            sizes = np.full_like(vals_clean, min_size)

        ax.scatter(
            df["LON"].to_numpy(),
            df["LAT"].to_numpy(),
            c=vals,
            cmap=cmap,
            norm=norm,
            s=sizes,
            marker="o",
            alpha=0.8,
            zorder=3
        )
        
        # --- overlay invalid stations (not used in analysis) ---
        if hasattr(self, "estacoes_df_invalid") and not self.estacoes_df_invalid.empty:
            inv = self.estacoes_df_invalid.copy()
            inv["LAT"] = pd.to_numeric(inv["LAT"], errors="coerce")
            inv["LON"] = pd.to_numeric(inv["LON"], errors="coerce")
            inv = inv.dropna(subset=["LAT", "LON"])

            ax.scatter(
                inv["LON"].to_numpy(),
                inv["LAT"].to_numpy(),
                s=10,
                marker=".",
                alpha=0.9,
                color = 'black',
                linewidths=1.2,
                label=f"Inválidas (>{self.MISSING_THRESHOLD}h de falha)",
                zorder = 2
            )
            ax.legend(loc="lower right")

        # discrete legend (faster/cleaner than plotting per-class subsets)
        handles = [
            mpl.lines.Line2D([], [], linestyle="none", marker="o",
                            markersize=7, markerfacecolor=col, markeredgecolor="none",
                            label=lab)
            for col, lab in zip(colors, labels)
        ]
        ax.legend(handles=handles,
                title=f"Diferença absoluta (mm / {self.WINDOW_HOURS}h)",
                loc="lower right")

        ax.set_title(f"Diferença absoluta – chuva acumulada ({self.WINDOW_HOURS}h)")
        ax.set_xlabel("Longitude")
        ax.set_ylabel("Latitude")
        ax.grid(True, linestyle="--", alpha=0.4)


        # --- label problematic stations (ERRO_ABS >= thr) ---
        # pick a station id/name column that exists in your df
        station_col = next(
            (c for c in ["ESTACAO", "NOME", "ID", "CODIGO", "STATION", "station"] if c in df.columns),
            None
        )

        if station_col is not None:
            prob = df[df["ERRO_ABS"] >= thr].copy()

            # optional: label only the top-N worst to keep the map readable
            max_labels = 25
            prob = prob.nlargest(max_labels, "ERRO_ABS")

            for _, r in prob.iterrows():
                ax.annotate(
                    f'{r[station_col]} ({r["ERRO_ABS"]:.1f})',
                    (r["LON"], r["LAT"]),
                    xytext=(4, 4),
                    textcoords="offset points",
                    fontsize=7,
                    ha="left",
                    va="bottom",
                    bbox=dict(boxstyle="round,pad=0.2", fc="white", ec="none", alpha=0.7),
                    zorder=5
                )
        else:
            print("No station id column found to label (ESTACAO/NOME/ID/CODIGO/...).")


        fig.tight_layout()
        plt.show()



    def process_station(self, station, df_p, estacoes, MAX_DISTANCE, MISSING_THRESHOLD):

        """Run full interpolation–accumulation–error workflow for one station.
        Adds neighbor list and their 72h accumulated precipitation."""


        obs_series = df_p[station]

        
        # Compute accumulations
        obs_accum, missing_hours = self.compute_station_accum(obs_series)


        # Interpolate hourly series
        interp_series, neighs = self.interpolate_station_timeseries(station, df_p, estacoes, MAX_DISTANCE)


        interp_accum, missing_hours_int = self.compute_station_accum(interp_series)

        # Compute neighbors’ accumulations
        neighbors_accum = {}
        for n in neighs:
            series_n = df_p[n]
            if series_n.isna().sum() > MISSING_THRESHOLD or len(series_n.dropna()) < self.WINDOW_HOURS:
                neighbors_accum[n] = np.nan
            else:
                neighbors_accum[n] = series_n.head(self.WINDOW_HOURS).sum()

        # Compute errors
        errors = self.compute_station_error(obs_accum, interp_accum)

        return {
            "CODIGO": station,
            "OBS_72H": obs_accum,
            "MISSING_N": missing_hours,
            "INTERP_72H": interp_accum,
            "ERRO_ABS": errors["ERRO_ABS"],
            "NEIGHBORS": ",".join(neighs) if neighs else None,
            "NEIGHBORS_ACCUM": neighbors_accum
        }


    def run_analysis(self):
        # Process each station

        self.label_8.setText(f"2\\2 - Verificando estacoes...")

        total = len(self.valid_codes)

        self.progressBar.setMinimum(0)
        self.progressBar.setMaximum(total)
        self.progressBar.setValue(0)



        results = []

        for i, station in enumerate(self.valid_codes, start=1):

            res = self.process_station(station, self.df_valid, self.estacoes_df_valid, self.MAX_DISTANCE, self.MISSING_THRESHOLD)

            if res:
                results.append(res)

                
            self.progressBar.setValue(i)
            QApplication.processEvents()
    

        results_df = pd.DataFrame(results)

        # Merge coordinates (LAT, LON) from estacoes_df
        self.estacoes_df["CODIGO"] = self.estacoes_df["CODIGO"].astype(str).str.strip()


        results_df = results_df.merge(
            self.estacoes_df[["CODIGO", "LAT", "LON", "NOME"]],
            on="CODIGO",
            how="left"
        )

        results_df.isna()

        self.results_intermediario = results_df

        print(results_df.head())
        

        print("\n✅ Cálculo concluído!")
        self.label_8.setText(f"Concluído")


        self.pushButton_3.setEnabled(True)   
        self.pushButton_4.setEnabled(True)   



        # Identify stations with large error
        bad_stations = results_df.loc[
            results_df["ERRO_ABS"] > self.ERROR_THRESHOLD, 
            ["CODIGO", "NOME"]
        ]

        # Convert to a list of tuples (or you can keep as a DataFrame)
        bad_stations_list = list(bad_stations.itertuples(index=False, name=None))

        print(f"\n⚠️ Estações com diferença ao interpolado > {self.ERROR_THRESHOLD} mm:")


        self.textEdit.clear()

        if not bad_stations_list:
            self.textEdit.append("Nenhuma estação identificada")
        else:
            for codigo, nome in bad_stations_list:
                self.textEdit.append(f"- {codigo}: {nome}")




def main():
    app = QApplication(sys.argv)

    window = MainWindow()
    window.show()


    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
