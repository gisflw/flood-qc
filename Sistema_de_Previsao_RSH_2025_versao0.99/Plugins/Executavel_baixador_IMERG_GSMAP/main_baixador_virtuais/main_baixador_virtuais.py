"""

Interface para download de dados de estações virtuais do GSMAP e IMERG 
Ingrid Petry
25/02/2026

"""

import sys
from PyQt5.QtWidgets import QApplication, QMainWindow, QMessageBox, QDialog, QFileDialog
from PyQt5.uic import loadUi
from pathlib import Path

from PyQt5.QtCore import QDateTime

import os
import io
import xarray as xr
import requests
import pandas as pd
import numpy as np
from urllib.parse import urlsplit, unquote
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo
from ftplib import FTP
import gzip
import random
import re

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


        # Define important paths
        self.ESTACOES_PLUV_PATH = self.DATA_DIR / "EstacoesPluv.txt"
        self.DATABASE_DIR = self.DATA_DIR / "database"


        ui_path = self.BASE_DIR / "interface_baixador_virtuais.ui"
        loadUi(str(ui_path), self)


        self.textEdit.setText('Starting application...')

        print(f"BASE_DIR: {self.BASE_DIR}")
        self.textEdit.append(f"BASE_DIR: {self.BASE_DIR}")

        print(f"DATA_DIR: {self.DATA_DIR}")
        self.textEdit.append(f"DATA_DIR: {self.DATA_DIR}")


        self.setup_connections()

        self.progressBar.setValue(0)
        self.progressBar_2.setValue(0)
        self.progressBar_3.setValue(0)
        self.progressBar_4.setValue(0)

        # # Disable buttons initially
        self.pushButton_2.setEnabled(False) 
        self.pushButton_4.setEnabled(False)   
        self.pushButton_5.setEnabled(False)   
        self.pushButton_6.setEnabled(False)   
        self.pushButton_7.setEnabled(False)   
        self.pushButton_8.setEnabled(False)
        self.pushButton_9.setEnabled(False)
        self.lineEdit.setEnabled(False)
        self.checkBox.setEnabled(False)   

        self.dateTimeEdit.setDisplayFormat("yyyy/MM/dd HH:00")
        
        dt = QDateTime.fromString("2026/01/01 00:00", "yyyy/MM/dd HH:mm")

        self.dateTimeEdit.setEnabled(False)
        
        self.dateTimeEdit.setDateTime(dt)   # ← correct method


        # Offset fixo UTC-3
        self.LOCAL_TZ = timezone(timedelta(hours=-3), name="UTC-3")
        self.UTC = timezone.utc

        self.setWindowTitle("Baixador IMERG e GSMAP")


    def setup_connections(self):
        """
        Connect UI elements (buttons, inputs, etc) to functions here

        """

        self.pushButton.clicked.connect(self.le_estacoesPluv)
        self.pushButton_5.clicked.connect(self.periodo_database)
        self.checkBox.toggled.connect(self.on_checkbox_toggled)
        self.pushButton_2.clicked.connect(self.baixar)
        self.pushButton_7.clicked.connect(self.limpar_temp)
        self.pushButton_4.clicked.connect(self.extrair_virtuais)
        self.pushButton_6.clicked.connect(self.atualizar_database)
        self.pushButton_8.clicked.connect(self.on_pushButton_8_clicked)
        self.pushButton_3.clicked.connect(self.close)
        self.pushButton_9.clicked.connect(self.find_file_imerg_subset)

        pass



    def le_estacoesPluv(self):

        if not self.ESTACOES_PLUV_PATH.exists():
            self.label.setText(f"EstacoesPluv nao encontrado: {self.ESTACOES_PLUV_PATH.resolve()}")
            return []

        estacoes_pluv = pd.read_csv(
            self.ESTACOES_PLUV_PATH,
            sep=";",
            encoding="latin1"
        )

        # Detect type using NOME
        mask_pim_nome = estacoes_pluv["NOME"].str.startswith("PIM", na=False)
        mask_pgm_nome = estacoes_pluv["NOME"].str.startswith("PGM", na=False)

        count_virt_imerg = mask_pim_nome.sum()
        count_virt_gsmap = mask_pgm_nome.sum()


        if count_virt_imerg > 0:
            self.base = 'IMERG'

            self.label.setStyleSheet("color: green; background-color: white;")
            self.label.setText(f"{count_virt_imerg} estacoes virtuais do IMERG")

            self.ESTACOES_PLUV_VIRTUAIS_PATH = self.BASE_DIR / f'Estacoes_virtuais_{self.base}.txt'

            estacoes_pluv_virt = pd.read_csv(
                self.ESTACOES_PLUV_VIRTUAIS_PATH,
                sep=";",
                encoding="latin1"
            )

            mask_codigo = estacoes_pluv_virt["CODIGO"].str.startswith("IM", na=False)
            estacoes_baixar = estacoes_pluv_virt.loc[mask_codigo, "CODIGO"].tolist()

            self.raster_dir = Path(self.BASE_DIR / "temp_imerg_raster")  
            self.virtuais_dir = Path(self.BASE_DIR / "OUTPUT_imerg_virtuais")

            
            # habilita proximo botao 
            self.pushButton_5.setEnabled(True)
            self.pushButton_7.setEnabled(True)



        elif count_virt_gsmap > 0:
            self.base = 'GSMAP'

            self.label.setStyleSheet("color: green; background-color: white;")
            self.label.setText(f"{count_virt_gsmap} estacoes virtuais do GSMAP")

            self.ESTACOES_PLUV_VIRTUAIS_PATH = self.BASE_DIR / f'Estacoes_virtuais_{self.base}.txt'

            estacoes_pluv_virt = pd.read_csv(
                self.ESTACOES_PLUV_VIRTUAIS_PATH,
                sep=";",
                encoding="latin1"
            )

            mask_codigo = estacoes_pluv_virt["CODIGO"].str.startswith("GM", na=False)
            estacoes_baixar = estacoes_pluv_virt.loc[mask_codigo, "CODIGO"].tolist()
            
            self.raster_dir = Path(self.BASE_DIR / "temp_gsmap_raster")
            self.virtuais_dir = Path(self.BASE_DIR / "OUTPUT_gsmap_virtuais")

            # habilita proximo botao 
            self.pushButton_5.setEnabled(True)
            self.pushButton_7.setEnabled(True)

        else:
            estacoes_pluv_virt = pd.DataFrame()
            self.label.setStyleSheet("color: red; background-color: white;")
            self.label.setText("Nenhuma estação virtual identificada - Substitua o arquivo EstacoesPluv.txt")
            estacoes_baixar = []
            self.base = 'nenhuma'


        self.estacoes_baixar = estacoes_baixar

        self.estacoes_virtuais_pluv = estacoes_pluv_virt.copy()
        


    def periodo_database(self):

        # self.now = datetime.now(self.LOCAL_TZ)

        files_iter = 20
        codigos_random = random.sample(self.estacoes_baixar, files_iter)

        step = pd.Timedelta(hours=1)

        start_dates = []
        end_dates = []
        per_file_missing = []   # list[set[pd.Timestamp]] (missing datetimes where virtu == -1)

        self.progressBar.setMaximum(files_iter)
        self.progressBar.setValue(0)

        for i, codigo in enumerate(codigos_random):
            telem_file_path = self.DATABASE_DIR / f"P{codigo}.txt"

            est_n = pd.read_csv(
                telem_file_path,
                sep=r"\s+",
                encoding="latin1",
                header=None,
                skiprows=1,
                names=["Dia", "Mes", "Ano", "Hora", "virtu"]
            )

            est_n["datetime"] = pd.to_datetime(
                est_n[["Ano", "Mes", "Dia", "Hora"]].rename(
                    columns={"Ano": "year", "Mes": "month", "Dia": "day", "Hora": "hour"}
                ),
                errors="coerce"
            )

            # If DB is LOCAL time but stored as naive, keep naive here.
            # (If you prefer LOCAL-aware everywhere, localize here instead.)
            est_n["datetime"] = est_n["datetime"].dt.tz_localize(self.LOCAL_TZ)

            est_n["virtu"] = pd.to_numeric(est_n["virtu"], errors="coerce")

            # Find the start (first available timestamp)
            first_valid_dt = est_n["datetime"].min()
            start_dates.append(first_valid_dt)

            # Find the last (most recent) valid value (ie. virtu != -1), ignoring -1s
            valid_rows = est_n[est_n["virtu"] != -1]
            if not valid_rows.empty:
                last_valid_dt = valid_rows["datetime"].max()
            else:
                last_valid_dt = pd.NaT
            end_dates.append(last_valid_dt)

            # Collect all datetimes where virtu == -1
            missing_dt = set(est_n.loc[est_n["virtu"] == -1, "datetime"].dropna().unique())
            per_file_missing.append(missing_dt)

            self.progressBar.setValue(i + 1)
            QApplication.processEvents()

        # ----- Determine DB period (based on the sampled files) -----
        all_same_start = len(set(start_dates)) == 1
        all_same_end = len(set(end_dates)) == 1

        if all_same_start and all_same_end:
            self.start_date_database = start_dates[0]
            self.end_date_database = end_dates[0]
        else:
            valid_starts = [d for d in start_dates if pd.notna(d)]
            valid_ends = [d for d in end_dates if pd.notna(d)]
            self.start_date_database = min(valid_starts) if valid_starts else pd.NaT
            self.end_date_database = min(valid_ends) if valid_ends else pd.NaT
        
        self.label_2.setText(f"Início da atualização: {self.end_date_database.strftime('%Y/%m/%d %H:%M')}")
        # self.label_12.setText(f"Início download: {self.end_date_database.strftime('%Y/%m/%d %H:%M')}")

        print(f"End date database: {self.end_date_database}")
        self.textEdit.append(f"End date database: {self.end_date_database}")

        self.end_date_atualizar = self.end_date_database
        

        self.pushButton_2.setEnabled(True)        
        self.pushButton_4.setEnabled(True)
        self.pushButton_6.setEnabled(True)

        if self.base == 'IMERG':
            self.pushButton_9.setEnabled(True)
            self.lineEdit.setEnabled(True)
            self.label_13.setText(f"Arquivo links IMERG:")

        self.checkBox.setEnabled(True)


    def subset_imerg_old(self):

        base = "https://gpm1.gesdisc.eosdis.nasa.gov/opendap/GPM_L3/GPM_3IMERGHHE.07"
        query = "precipitation[0:0][1184:1325][517:672],time,lon[1184:1325],lat[517:672]"

        def imerg_filename(dt):
            year = dt.strftime("%Y")
            doy = dt.strftime("%j")
            ymd = dt.strftime("%Y%m%d")

            S = dt.strftime("%H%M%S")
            E_dt = dt + timedelta(minutes=29, seconds=59)
            E = E_dt.strftime("%H%M%S")

            minutes_from_midnight = dt.hour * 60 + dt.minute
            offset = f"{minutes_from_midnight:04d}"

            filename = (
                f"3B-HHR-E.MS.MRG.3IMERG.{ymd}-S{S}-E{E}.{offset}.V07B.HDF5.SUB.nc4"
            )

            url = f"{base}/{year}/{doy}/{filename}?{query}"
            return filename, url

        def generate_urls():

            dt = self.start_dt  # already UTC

            while dt <= self.end_dt:

                filename, url = imerg_filename(dt)

                local_path = Path(self.raster_dir) / filename

                if not local_path.exists():
                    yield url
                # else: skip silently

                dt += timedelta(minutes=30)

        urls = list(generate_urls())
        n_download = len(urls)

        return urls, n_download


    def find_file_imerg_subset(self):
        """
        Select a file and save its path.
        """

        downloads_dir = str(Path.home() / "Downloads")

        file_path, _ = QFileDialog.getOpenFileName(
            self,
            "Selecione um arquivo .txt",
            downloads_dir,
            "Text Files (*.txt);;All Files (*)"
        )

        if not file_path:
            return

        self.file_path_imerg_subset = file_path

        filename = Path(file_path).name
        self.lineEdit.setText(filename)

        print(f"Selected file: {self.file_path_imerg_subset}")

   


    def read_imerg_subeset_file(self):
        """
        Read the previously selected file and extract URLs.
        """


        file_path = getattr(self, "file_path_imerg_subset", None)

        if not file_path:
            print("No file selected.")
            return [], 0

        try:
            with open(file_path, "r", encoding="utf-8") as f:
                text = f.read()

            urls = re.findall(r"https?://\S+", text)

            print(f"Número de URLs: {len(urls)}")
            self.textEdit.append(f"Número de URLs: {len(urls)}")

            return urls, len(urls)

        except Exception as e:
            print(f"Erro ao ler o arquivo: {e}")
            return [], 0

    def baixador_imerg(self, urls, n_download):

        print(f"Preparing to download {n_download} files...")
        self.textEdit.append(f"Preparing to download {n_download} files...")
        failed_urls = []

        os.makedirs(self.raster_dir, exist_ok=True)

        for i, url in enumerate(urls):

            from urllib.parse import urlsplit, parse_qs, unquote

            parts = urlsplit(url)
            query = parse_qs(parts.query)

            if "LABEL" in query:
                filename = unquote(query["LABEL"][0])
            elif "FILENAME" in query:
                filename = os.path.basename(unquote(query["FILENAME"][0]))
            else:
                filename = unquote(os.path.basename(parts.path)) or "downloaded_file"

            filepath = os.path.join(self.raster_dir, filename)
            filepath = os.path.join(self.raster_dir, filename)

            try:
                # Se já existe, pula
                if os.path.exists(filepath):
                    print(f"File already exists, skipping: {filepath}")
                    self.textEdit.append(f"File already exists, skipping: {filepath}")
                else:
                    r = requests.get(url, timeout=30)
                    r.raise_for_status()

                    with open(filepath, "wb") as f:
                        f.write(r.content)

                    print(f"Downloaded: {filepath}")
                    self.textEdit.append(f"Downloaded: {filepath}")
            except Exception as e:
                print(f"Failed to download {url}: {e}")
                self.textEdit.append(f"Failed to download {url}: {e}")
                failed_urls.append(url)

            self.progressBar_2.setValue(i + 1)
            QApplication.processEvents()

        return failed_urls


    def files_gsmap(self):

        """
        Returns a list of dicts with remote_dir, filename, and local_path for each hourly file.
        """

        ftp_base = '/realtime_ver/v8/hourly'

        tasks = []

        dt_i = self.start_dt.astimezone(self.UTC)


        while dt_i <= self.end_dt:
            ymd = dt_i.strftime("%Y%m%d")
            hour = dt_i.strftime("%H%M")
            remote_dir = f"{ftp_base}/{dt_i.year:04d}/{dt_i.month:02d}/{dt_i.day:02d}"
            filename = f"gsmap_nrt.{ymd}.{hour}.dat.gz"
            local_path = self.raster_dir / filename

            if local_path.exists():
                dt_i += timedelta(hours=1)
                continue

            tasks.append(
                {
                    "remote_dir": remote_dir,
                    "filename": filename,
                    "local_path": local_path,
                    "dt": dt_i,  # optional, useful for logging
                }
            )

            dt_i += timedelta(hours=1)

        return tasks, len(tasks)


    def read_ftp_credentials(self, filepath):
        creds = {}

        try:
            with open(filepath, "r") as f:
                for line in f:
                    line = line.strip()

                    if not line or "=" not in line:
                        continue

                    key, value = line.split("=", 1)
                    creds[key.strip()] = value.strip()

        except FileNotFoundError:
            print(f"[WARNING] Credential file not found: {filepath}")
            self.textEdit.append(f"[WARNING] Credential file not found: {filepath}")
            return None, None

        except PermissionError:
            print(f"[WARNING] No permission to read credential file: {filepath}")
            self.textEdit.append(f"[WARNING] No permission to read credential file: {filepath}")
            return None, None

        except Exception as e:
            print(f"[WARNING] Error reading credential file: {e}")
            self.textEdit.append(f"[WARNING] Error reading credential file: {e}")
            return None, None

        if "ftp_user" not in creds or "ftp_pass" not in creds:
            print("[WARNING] Credential file is missing ftp_user or ftp_pass.")
            self.textEdit.append("[WARNING] Credential file is missing ftp_user or ftp_pass.")  
            return None, None

        return creds["ftp_user"], creds["ftp_pass"]
    


    def baixar_gsmap(self, tasks, n_download):

        # ftp_host = 'hokusai.eorc.jaxa.jp'
        # ftp_user, ftp_pass = self.read_ftp_credentials("jaxa_gsmap_ftp_credentials.txt")


        ftp_user =	"rainmap"
        ftp_pass = "Niskur+1404"
        ftp_host = 'hokusai.eorc.jaxa.jp'

        print(f"FTP user: {ftp_user}")
        print(f"FTP pass: {ftp_pass}")
        self.textEdit.append(f"FTP user: {ftp_user}")
        self.textEdit.append(f"FTP pass: {ftp_pass}")

        print(f"Preparing to download {n_download} files...")
        self.textEdit.append(f"Preparing to download {n_download} files...")

        failed_files = []  # ← lista de falhas

        with FTP(ftp_host) as ftp:
            ftp.login(ftp_user, ftp_pass)

            for i, t in enumerate(tasks):
                remote_dir = t["remote_dir"]
                filename = t["filename"]
                local_path = Path(t["local_path"])

                try:
                    if local_path.exists():
                        print(f"Skipping {filename} (already exists)")
                        self.textEdit.append(f"Skipping {filename} (already exists)")
                    else:

                        ftp.cwd(remote_dir)

                        local_path.parent.mkdir(parents=True, exist_ok=True)

                        with open(local_path, "wb") as f:
                            ftp.retrbinary(f"RETR {filename}", f.write)

                        print(f'Downloaded {filename}')
                        self.textEdit.append(f'Downloaded {filename}')

                except Exception as e:
                    print(f"Failed to download {filename}: {e}")
                    self.textEdit.append(f"Failed to download {filename}: {e}")
                    failed_files.append(f"{remote_dir}/{filename}")

                self.progressBar_2.setValue(i + 1)
                QApplication.processEvents()

        return failed_files


    def _floor_to_30min(self, dt):
        """Floor an aware datetime to previous 30-min boundary."""
        minute_rounded = (dt.minute // 30) * 30
        return dt.replace(minute=minute_rounded, second=0, microsecond=0)


    def inicio_manual(self):
        self.end_date_atualizar = self.dateTimeEdit.dateTime().toPyDateTime()


    def inicio_identificado(self):    
        self.end_date_atualizar = self.end_date_database


    def configurar_limite(self):
        limite = QDateTime(self.end_date_database)
        self.dateTimeEdit.setMaximumDateTime(limite)

    def on_pushButton_8_clicked(self):
        if self.dateTimeEdit.isEnabled():
            self.dateTimeEdit.clearFocus()  # garante que a edição foi aplicada
            self.end_date_atualizar = self.dateTimeEdit.dateTime().toPyDateTime()
            print('Nova data inicial: ',self.end_date_atualizar)
            self.textEdit.append(f'Nova data inicial: {self.end_date_atualizar}')
            # self.label_12.setText(f"Início download: {self.end_date_atualizar.strftime('%Y/%m/%d %H:%M')}")
            

    def on_checkbox_toggled(self, checked):
        if checked:
            self.pushButton_8.setEnabled(True)
            self.dateTimeEdit.setEnabled(True)
            self.configurar_limite()
        else:
            self.dateTimeEdit.setEnabled(False)
            self.inicio_identificado()

    def baixar(self):

        # Always compute end_dt in UTC (more consistent for URL/file naming)
        now_utc = datetime.now(self.UTC)

        end_utc = self._floor_to_30min(now_utc) - timedelta(hours=4)  # latency

        self.end_dt = end_utc

        # start comes from DB (naive LOCAL)
        start_local = self.end_date_atualizar

        print(f'Start local time: {start_local}')
        self.textEdit.append(f'Start local time: {start_local}')
        # comecar a baixar na mesma hora do ultimo valor para conseguir fazer right label -  transforma em UTC
        self.start_dt = start_local.astimezone(self.UTC) 

        print(f'UTC: start UTC {self.start_dt} - end UTC {self.end_dt} -----')
        self.textEdit.append(f'UTC: start UTC {self.start_dt} - end UTC {self.end_dt} -----')
        #################################### Chama baixador IMERG

        print(f'Downloading {self.base}...')
        self.textEdit.append(f'Downloading {self.base}...')
        if self.base == 'IMERG':

            urls, n_download = self.read_imerg_subeset_file()

            self.progressBar_2.setMaximum(n_download)
            self.progressBar_2.setValue(0)

            if n_download == 0:

                self.progressBar_2.setRange(0, 100)
                self.progressBar_2.setValue(100)

                self.label_3.setStyleSheet("color: green; background-color: white;")
                self.label_3.setText("Nada para baixar (arquivos já existem).")

                self.pushButton_4.setEnabled(True)
                return
            
            else:
                    
                reply = QMessageBox.question(
                    self,
                    "Autorizar download",
                    f"{n_download} arquivos serao baixados.\nQuer continuar?",
                    QMessageBox.Yes | QMessageBox.No,
                    QMessageBox.No
                )

                if reply != QMessageBox.Yes:
                    return  # interrompe aqui se a pessoa clicar "No"

            self.label_3.setStyleSheet("color: blue; background-color: white;")
            self.label_3.setText(f"Downloading {n_download} IMERG Early Run V07B...")

            missing_files = self.baixador_imerg(urls, n_download)

            # Conta arquivos dentro da pasta raster_dir
            n_files_raster_dir = len([
                f for f in os.listdir(self.raster_dir)
                if os.path.isfile(os.path.join(self.raster_dir, f))
            ])

            if n_files_raster_dir < n_download:
                print('Segunda tentativa')
                self.textEdit.append('Segunda tentativa')
                missing_files = self.baixador_imerg(urls, n_download)

                n_files_raster_dir = len([
                    f for f in os.listdir(self.raster_dir)
                    if os.path.isfile(os.path.join(self.raster_dir, f))
                ])

            elif n_files_raster_dir >= n_download:
                print('Tentativa finalizada')
                self.textEdit.append('Tentativa finalizada')

        elif self.base == 'GSMAP':

            urls, n_download = self.files_gsmap()

        

            self.progressBar_2.setMaximum(n_download)
            self.progressBar_2.setValue(0)

            if n_download == 0:

                self.progressBar_2.setRange(0, 100)
                self.progressBar_2.setValue(100)

                self.label_3.setStyleSheet("color: green; background-color: white;")
                self.label_3.setText("Nada para baixar (arquivos já existem).")
                self.pushButton_4.setEnabled(True)
                return
            
            else:

                reply = QMessageBox.question(
                    self,
                    "Autorizar download",
                    f"{n_download} arquivos serao baixados.\nQuer continuar?",
                    QMessageBox.Yes | QMessageBox.No,
                    QMessageBox.No
                )

                if reply != QMessageBox.Yes:
                    return  # interrompe aqui se a pessoa clicar "No"


            self.label_3.setStyleSheet("color: blue; background-color: white;")
            self.label_3.setText(f"Downloading {n_download} GSMAP Near Real Time V8...")

            missing_files = self.baixar_gsmap(urls, n_download)

            
            # Conta arquivos dentro da pasta raster_dir
            n_files_raster_dir = len([
                f for f in os.listdir(self.raster_dir)
                if os.path.isfile(os.path.join(self.raster_dir, f))
            ])

            if n_files_raster_dir < n_download:
                print('Segunda tentativa')
                self.textEdit.append('Segunda tentativa')
                missing_files = self.baixar_gsmap(urls, n_download)

                n_files_raster_dir = len([
                    f for f in os.listdir(self.raster_dir)
                    if os.path.isfile(os.path.join(self.raster_dir, f))
                ])

            elif n_files_raster_dir >= n_download:
                print('Tentativa finalizada')
                self.textEdit.append('Tentativa finalizada')
        # -----------------------
        print(missing_files)
        self.textEdit.append(f'missing_files: {missing_files}')
        self.label_3.setStyleSheet("color: black; background-color: white;")
        self.label_3.setText(f"Finalizado - {len(missing_files)} arquivos faltando")


        # ----------------------- Formatting file structure -----------------------
    def _header_line(self, station_name: str) -> str:
        return (
            f"{'Dia':<{6}}"
            f"{'Mês':<{6}}"
            f"{'Ano':<{6}}"
            f"{'Hora':<{6}}"
            f"{station_name}\n"
        )

    def _data_line(self, dia: int, mes: int, ano: int, hora: int, valor: float) -> str:
        s_dia  = f"{int(dia):02d}".rjust(6)
        s_mes  = f"{int(mes):02d}".rjust(6)
        s_ano  = f"{int(ano)}".rjust(6)
        s_hora = f"{int(hora):02d}".rjust(6)
        s_val  = f"{float(valor):{16}.6f}"
        return f"{s_dia}{s_mes}{s_ano}{s_hora}{s_val}\n"


    def extrair_virtuais(self):

        # ----------------------- Guard -----------------------
        if not any(self.raster_dir.iterdir()):
            self.label_9.setText("Sem arquivos de chuva para extrair")
            return
        
        virtu_path = self.virtuais_dir

        if not virtu_path.exists():
            QMessageBox.warning(self, "Erro", "Um dos diretórios não existe.")
            return

        virtu_files = [f for f in virtu_path.iterdir() if f.is_file()]

        files = virtu_files

        # ---- Delete files ----
        deleted = 0

        for file in files:
            try:
                file.unlink()
                deleted += 1
            except Exception as e:
                print(f"Erro ao excluir {file}: {e}")
                self.textEdit.append(f"Erro ao excluir {file}: {e}")

        # ----------------------- Output files -----------------------
        output_files: dict[str, io.TextIOWrapper] = {}
        for _, row in self.estacoes_virtuais_pluv.iterrows():
            station_id = str(row["CODIGO"])
            out_path = self.virtuais_dir / f"P{station_id}.txt"
            f = open(out_path, "w", encoding="utf-8", newline="\n")
            f.write(self._header_line(station_id))
            output_files[station_id] = f

        # =====================================================================
        # Branch: IMERG
        # aqui os arquivos representam meia hora de dados de acordo com o nome do arquivo. 
        # por exemplo, o acumulado de 0900-0929, 0930-0959 serã escrito como a chuva das 10h. 
        # =====================================================================

        if self.base == "IMERG":

            def _open_grid(path_nc4: Path) -> np.ndarray:
                with xr.open_dataset(path_nc4) as ds:
                    if "precipitation" not in ds:
                        raise ValueError(f"'precipitation' variable not found in {path_nc4}")
                    data = ds["precipitation"].fillna(0).values
                if data.ndim == 3:
                    data = data[0, :, :]
                return data

            def _filename_for_imerg(_dt: datetime) -> Path:

                dt_utc = _dt

                start_minute = 0 if dt_utc.minute < 30 else 30
                start_utc = dt_utc.replace(minute=start_minute, second=0, microsecond=0)
                end_utc = start_utc + timedelta(minutes=29, seconds=59)

                date_str = start_utc.strftime("%Y%m%d")
                start_str = f"S{start_utc.strftime('%H%M%S')}"
                end_str   = f"E{end_utc.strftime('%H%M%S')}"

                offset_minutes = start_utc.hour * 60 + start_utc.minute
                offset_str = f"{offset_minutes:04d}"

                # filename = (
                #     f"3B-HHR-E.MS.MRG.3IMERG.{date_str}-{start_str}-{end_str}."
                #     f"{offset_str}.V07B.HDF5.nc4"
                # ) #old version before march 2026
                
                filename = (
                    f"3B-HHR-E.MS.MRG.3IMERG.{date_str}-{start_str}-{end_str}."
                    f"{offset_str}.V07C.HDF5.SUB.nc4"
                )

                return self.raster_dir / filename

            def _accumulate_hourly(first_half: Path, second_half: Path) -> np.ndarray:
                d1 = _open_grid(first_half)
                d2 = _open_grid(second_half)
                if d1.shape != d2.shape:
                    raise ValueError(f"Shape mismatch: {first_half.name} vs {second_half.name}")
                return np.nanmean(np.stack([d1, d2]), axis=0)

            def _get_reference_grid() -> tuple[np.ndarray, np.ndarray]:
                nc_files = sorted(self.raster_dir.glob("*.nc4"))
                if not nc_files:
                    raise FileNotFoundError(f"No .nc4 files found in {self.raster_dir}")
                sample_file = nc_files[0]
                with xr.open_dataset(sample_file) as ds:
                    if "lat" in ds:
                        lats = ds["lat"].values
                    elif "latitude" in ds:
                        lats = ds["latitude"].values
                    else:
                        raise KeyError("Latitude variable not found in file.")

                    if "lon" in ds:
                        lons = ds["lon"].values
                    elif "longitude" in ds:
                        lons = ds["longitude"].values
                    else:
                        raise KeyError("Longitude variable not found in file.")
                return lats, lons
            

            lats, lons = _get_reference_grid()

            station_idx: list[tuple[str, int, int]] = []

            print(f'Extraindo {len(self.estacoes_virtuais_pluv)} estacoes')
            self.textEdit.append(f'Extraindo {len(self.estacoes_virtuais_pluv)} estacoes')
            for _, row in self.estacoes_virtuais_pluv.iterrows():
                station_id = str(row["CODIGO"])
                lat = float(row["LAT"])
                lon = float(row["LON"])
                lat_idx = int(np.argmin(np.abs(lats - lat)))
                lon_idx = int(np.argmin(np.abs(lons - lon)))
                station_idx.append((station_id, lat_idx, lon_idx))

            def _value_at(data: np.ndarray, lat_idx: int, lon_idx: int) -> float:
                # IMPORTANT: IMERG commonly stores as [lat, lon]. Keep this consistent.
                # If your files are actually [lon, lat], change ONLY here.
                return float(data[lat_idx, lon_idx])

            date_i = self.start_dt
            n_missing = 0
            n_ok = 0

            total_steps = int(((self.end_dt - self.start_dt).total_seconds() // 3600) + 1)

            if total_steps < 0:
                total_steps = 0
            self.progressBar_3.setMinimum(0)
            self.progressBar_3.setMaximum(total_steps)
            self.progressBar_3.setValue(0)

            step = 0

            while date_i <= self.end_dt:

                # print('------------------------------------------------------------')
                # print(f'date_i {date_i} - self.end_date {self.end_dt}')

                # date_i tem que estar em UTC. confirmar isso 
                # monta e busca um arquivo de nome especifico. nao vai ler todos arquivos da pasta

                first_half_path  = _filename_for_imerg(date_i.replace(minute=0))
                second_half_path = _filename_for_imerg(date_i.replace(minute=30))
                print(f'first_half_path {first_half_path}')
                print(f'second_half_path {second_half_path}')

                data = None  # None => missing/failure for this hour

                if (not first_half_path.exists()) or (not second_half_path.exists()):
                    n_missing += 1
                else:
                    try:
                        data = _accumulate_hourly(first_half_path, second_half_path)
                    except Exception:
                        print('Erro ao acumular')
                        self.textEdit.append('Failed to accumulate rainfall data')
                        n_missing += 1
                        data = None

                # soma uma hora para que a flag seja right labeled. 
                # por exemplo, o acumulado de 0900-0929, 0930-0959 serã escrito como a chuva das 10h. 

                date_local = date_i.astimezone(self.LOCAL_TZ) + timedelta(hours=1)

                # print(f'escrito na date_local {date_local}')

                day, month, year, hour = date_local.day, date_local.month, date_local.year, date_local.hour

                for station_id, lat_idx, lon_idx in station_idx:
                    precip = -1.0 if data is None else _value_at(data, lat_idx, lon_idx)
                    output_files[station_id].write(self._data_line(day, month, year, hour, precip))

                # se "n_ok" for “horas escritas”, ok incrementar sempre
                if data is not None:
                    n_ok += 1

                date_i += timedelta(hours=1)
                step += 1
                self.progressBar_3.setValue(step)
                QApplication.processEvents()

        # =====================================================================
        # Branch: GSMAP
        # Aqui os arquivos ja sao o valor da ultima hora, entao o label esta correto.
        # ou seja, o arquivo das 0900 (utc) ira ser escrito na linha da hora 6 (UTC-3)
        # =====================================================================

        elif self.base == "GSMAP":

            # GSMaP fixed grid
            cols, rows = 3600, 1200
            lat_grid = np.linspace(-60, 60, rows)
            lon_grid = np.linspace(-180, 180, cols)

            station_idx: list[tuple[str, int, int]] = []
            for _, row in self.estacoes_virtuais_pluv.iterrows():
                station_id = str(row["CODIGO"])
                lat = float(row["LAT"])
                lon = float(row["LON"])
                lat_idx = int(np.argmin(np.abs(lat_grid - lat)))
                lon_idx = int(np.argmin(np.abs(lon_grid - lon)))
                station_idx.append((station_id, lat_idx, lon_idx))

            def _open_grid(path_gz: Path) -> np.ndarray:
                with gzip.open(path_gz, "rb") as f:
                    buf = f.read()
                data = np.frombuffer(buf, dtype="<f4")
                expected = rows * cols
                if data.size != expected:
                    raise ValueError(f"Unexpected grid size: {data.size} vs {expected}")
                data = data.reshape((rows, cols))

                # replicate your proven transforms
                data = np.flipud(data)
                data = np.hstack((data[:, 1800:], data[:, :1800]))
                return data

            def _filename_for_gsmap(_dt: datetime) -> Path:
                dt_utc = _dt
                day_str = dt_utc.strftime("%Y%m%d")
                hour_str = dt_utc.strftime("%H")
                return self.raster_dir / f"gsmap_nrt.{day_str}.{hour_str}00.dat.gz"

            def _value_at(data: np.ndarray, lat_idx: int, lon_idx: int) -> float:
                return float(data[lat_idx, lon_idx])

            # aumenta uma hora pq ja esta como right label, ou seja, comeca uma hora depois do inicio
            date_i = self.start_dt + timedelta(hours=1)
            n_missing = 0
            n_ok = 0

            total_steps = int(((self.end_dt - date_i).total_seconds() // 3600) + 1)
            if total_steps < 0:
                total_steps = 0
            self.progressBar_3.setMinimum(0)
            self.progressBar_3.setMaximum(total_steps)
            self.progressBar_3.setValue(0)

            step = 0
            
            self.textEdit.append(f'Extraindo...')

            while date_i <= self.end_dt:

                gz_path = _filename_for_gsmap(date_i)

                # print('------------------------------------------------------------')

                # print(gz_path)

                # print(f'date_i {date_i} - self.end_date {self.end_dt}')

                data = None  # None => missing/failure for this hour

                if not gz_path.exists():
                    print('nao existe')
                    n_missing += 1
                else:
                    try:
                        data = _open_grid(gz_path)
                    except Exception:
                        print('nao abriu')
                        n_missing += 1
                        data = None

                date_local = date_i.astimezone(self.LOCAL_TZ)

                print(f'escrito na date_local {date_local}')
                # self.textEdit.append(f'escrito na date_local {date_local}')
                day, month, year, hour = date_local.day, date_local.month, date_local.year, date_local.hour

                for station_id, lat_idx, lon_idx in station_idx:
                    precip = -1.0 if data is None else _value_at(data, lat_idx, lon_idx)
                    output_files[station_id].write(self._data_line(day, month, year, hour, precip))

                # se n_ok for “horas escritas”, incrementa sempre; se for “horas válidas”, só quando data != None
                if data is not None:
                    n_ok += 1

                date_i += timedelta(hours=1)
                step += 1
                self.progressBar_3.setValue(step)
                QApplication.processEvents()

        # ----------------------- Close files -----------------------
        for f in output_files.values():
            try:
                f.close()
            except Exception:
                pass

        self.label_9.setText(f"{n_ok} horas extraídas e {n_missing} arquivos faltando/inválidos")
        print(f"{n_ok} horas extraídas e {n_missing} arquivos faltando/inválidos")
        self.textEdit.append(f"{n_ok} horas extraídas e {n_missing} arquivos faltando/inválidos")




    def _line_to_dt_and_value(self, line: str):
        parts = line.split()
        dia = int(parts[0]); mes = int(parts[1]); ano = int(parts[2]); hora = int(parts[3])
        valor = float(parts[4])  # a 5ª coluna é o valor
        dt = datetime(ano, mes, dia, hora)
        return dt, dia, mes, ano, hora, valor


    def atualizar_database(self):

        virtuais_dir = Path(self.virtuais_dir)
        virtual_files = sorted(virtuais_dir.glob("*.txt"))

        self.progressBar_4.setMinimum(0)
        self.progressBar_4.setMaximum(len(virtual_files))
        self.progressBar_4.setValue(0)

        encodings_to_try = ["utf-8", "utf-8-sig", "cp1252", "latin-1"]

        for i, virtual_path in enumerate(virtual_files):
            db_path = self.DATABASE_DIR / virtual_path.name

            # --- lê NOVO ---
            with virtual_path.open("r", encoding="utf-8") as f:
                new_lines = f.readlines()

            if not new_lines:
                self.progressBar_4.setValue(i + 1)
                QApplication.processEvents()
                continue

            header_new = new_lines[0]
            new_data = [ln for ln in new_lines[1:] if ln.strip()]
            if not new_data:
                self.progressBar_4.setValue(i + 1)
                QApplication.processEvents()
                continue

            dt_new_start, *_ = self._line_to_dt_and_value(new_data[0])

            # --- se DB não existe/está vazio: cria com header + dados ---
            if (not db_path.exists()) or db_path.stat().st_size == 0:
                db_path.parent.mkdir(parents=True, exist_ok=True)
                with db_path.open("w", encoding="utf-8") as out:
                    out.write(header_new)
                    out.writelines(new_data)
                self.progressBar_4.setValue(i + 1)
                QApplication.processEvents()
                continue

            # --- lê DB com fallback de encoding ---
            db_lines = None
            used_encoding = None
            for enc in encodings_to_try:
                try:
                    with db_path.open("r", encoding=enc) as f:
                        db_lines = f.readlines()
                    used_encoding = enc
                    break
                except UnicodeDecodeError:
                    continue

            if db_lines is None:
                with db_path.open("r", encoding="utf-8", errors="replace") as f:
                    db_lines = f.readlines()
                used_encoding = "utf-8"

            header_db = db_lines[0] if db_lines else header_new
            db_data = [ln for ln in (db_lines[1:] if len(db_lines) > 1 else []) if ln.strip()]

            # --- monta mapa dt -> linha do DB ---
            db_map = {}
            for ln in db_data:
                try:
                    dt, dia, mes, ano, hora, valor = self._line_to_dt_and_value(ln)
                    db_map[dt] = ln
                except Exception:
                    continue

            dt_db_end = max(db_map.keys()) if db_map else None


            # # --- DEBUG VISUAL ANTES DE ESCREVER ---

            # print("\n" + "="*60)
            # print(f"Arquivo DB: {db_path.name}")
            # print(f"Arquivo NOVO: {virtual_path.name}")

            # # Últimas 5 linhas válidas do DB
            # if db_map:
            #     print("\n--- FINAL DO DATABASE (últimas 5 horas) ---")
            #     for dt in sorted(db_map.keys())[-5:]:
            #         print(db_map[dt].rstrip())
            # else:
            #     print("\nDATABASE vazio.")

            # # Primeiras 5 linhas do NOVO
            # print("\n--- INÍCIO DO ARQUIVO NOVO (primeiras 5 horas) ---")
            # for ln in new_data[:5]:
            #     print(ln.rstrip())

            # # Datas comparativas
            # if db_map:
            #     dt_db_end = max(db_map.keys())
            #     dt_new_start, *_ = self._line_to_dt_and_value(new_data[0])

            #     print("\n--- COMPARAÇÃO DE DATETIMES ---")
            #     print(f"Último datetime DB  : {dt_db_end}")
            #     print(f"Primeiro datetime NOVO: {dt_new_start}")
            #     print(f"Diferença (horas)     : {(dt_new_start - dt_db_end).total_seconds()/3600}")

            # print("="*60 + "\n")

            # --- preencher gaps se novo começa depois do último do DB ---
            if dt_db_end is not None:
                expected = dt_db_end + timedelta(hours=1)
                if dt_new_start > expected:
                    cur = expected
                    while cur < dt_new_start:
                        if cur not in db_map:
                            db_map[cur] = self._data_line(
                                cur.day, cur.month, cur.year, cur.hour, -1.0
                            )
                        cur += timedelta(hours=1)

            # --- aplica NOVO (sobrescreve se já existir) ---
            for ln in new_data:
                try:
                    dt, dia, mes, ano, hora, valor = self._line_to_dt_and_value(ln)
                    db_map[dt] = ln
                except Exception:
                    continue

            # --- regrava ordenado ---
            with db_path.open("w", encoding=used_encoding) as out:
                out.write(header_db if header_db.endswith("\n") else header_db + "\n")
                for dt in sorted(db_map.keys()):
                    out.write(db_map[dt])

            self.progressBar_4.setValue(i + 1)
            QApplication.processEvents()

        self.label_10.setText("Concluído!")
        self.textEdit.append(f'Extração concluída com sucesso!')



    def limpar_temp(self):

        raster_path = self.raster_dir
        virtu_path = self.virtuais_dir

        # ---- Check dirs ----
        if not raster_path.exists() or not virtu_path.exists():
            QMessageBox.warning(self, "Erro", "Um dos diretórios não existe.")
            return

        # ---- Count files ----
        raster_files = [f for f in raster_path.iterdir() if f.is_file()]
        virtu_files = [f for f in virtu_path.iterdir() if f.is_file()]

        files = raster_files + virtu_files
        total_files = len(files)

        if total_files == 0:
            QMessageBox.information(self, "Limpeza", "Nenhum arquivo para excluir.")
            return

        # ---- Confirmation Message ----
        msg = QMessageBox(self)
        msg.setIcon(QMessageBox.Warning)
        msg.setWindowTitle("Confirmação")
        msg.setText("Tem certeza que deseja excluir os arquivos temporários?")
        msg.setInformativeText(
            f"{len(raster_files)} arquivos em Raster\n"
            f"{len(virtu_files)} arquivos em Virtuais\n\n"
            f"Total: {total_files} arquivos serão excluídos."
        )
        msg.setStandardButtons(QMessageBox.Yes | QMessageBox.No)
        msg.setDefaultButton(QMessageBox.No)

        result = msg.exec_()

        if result != QMessageBox.Yes:
            return

        # ---- Delete files ----
        deleted = 0

        for file in files:
            try:
                file.unlink()
                deleted += 1
            except Exception as e:
                print(f"Erro ao excluir {file}: {e}")
                self.textEdit.append(f"Erro ao excluir {file}: {e}")
        # ---- Feedback ----
        QMessageBox.information(self, "Concluído", f"{deleted} arquivos excluídos.")
        self.textEdit.append(f'{deleted} arquivos excluídos.')


def main():
    app = QApplication(sys.argv)

    window = MainWindow()
    window.show()


    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
