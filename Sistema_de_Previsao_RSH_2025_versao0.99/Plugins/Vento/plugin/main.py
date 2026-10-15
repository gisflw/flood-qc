# main.py
import sys
import os
import datetime
import traceback
from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QMessageBox, QFileDialog, QWidget, QVBoxLayout
)
from PyQt5.QtCore import QThread, pyqtSignal, Qt
from PyQt5.QtGui import QPixmap
from PyQt5 import uic
from matplotlib.backends.backend_qt5agg import NavigationToolbar2QT as NavigationToolbar

from downloader import BndmetAPI
from converter import salvar_arquivo_mgb
from plotter import WindPlotCanvas

# ------------------------------
# Tratamento de exceções não capturadas
def excepthook(exc_type, exc_value, exc_tb):
    msg = ''.join(traceback.format_exception(exc_type, exc_value, exc_tb))
    QMessageBox.critical(None, "Erro fatal", f"O programa encontrou um erro e será fechado.\n\n{msg}")
    sys.exit(1)

sys.excepthook = excepthook
# ------------------------------

def resource_path(relative_path):
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = os.path.abspath(".")
    return os.path.join(base_path, relative_path)

UI_FILE = resource_path('vento_gui.ui')

class DownloadThread(QThread):
    finished = pyqtSignal(dict)  # {codigo: df}
    error = pyqtSignal(str)
    log = pyqtSignal(str)
    progress = pyqtSignal(int, int)

    def __init__(self, estacoes, start_date, end_date):
        super().__init__()
        self.estacoes = estacoes
        self.start_date = start_date
        self.end_date = end_date

    def run(self):
        resultados = {}
        total = len(self.estacoes)
        for i, codigo in enumerate(self.estacoes):
            self.log.emit(f"Baixando estação {codigo}...")
            try:
                df = BndmetAPI.baixar_completo(codigo, self.start_date, self.end_date, self.log.emit)
                resultados[codigo] = df
                self.log.emit(f"  {codigo}: {len(df) if df is not None else 0} registros.")
            except Exception as e:
                self.log.emit(f"  ERRO em {codigo}: {str(e)}")
                resultados[codigo] = None
            self.progress.emit(i+1, total)
        self.finished.emit(resultados)

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        uic.loadUi(UI_FILE, self)
        self.setWindowTitle("Baixador de Dados de Vento - MGB")

        # Diretórios
        if getattr(sys, 'frozen', False):
            base_dir = os.path.dirname(sys.executable)
        else:
            base_dir = os.path.dirname(os.path.abspath(__file__))
        self.output_dir = os.path.join(base_dir, 'result')
        os.makedirs(self.output_dir, exist_ok=True)

        self.estacoes_file = os.path.join(base_dir, 'Input', 'Estacoes.txt')

        # Carregar imagem (se existir)
        img_path = resource_path('vento_ilust.png')
        if hasattr(self, 'img_logo') and os.path.exists(img_path):
            pixmap = QPixmap(img_path)
            pixmap = pixmap.scaled(self.img_logo.size(), Qt.KeepAspectRatio, Qt.SmoothTransformation)
            self.img_logo.setPixmap(pixmap)
        elif hasattr(self, 'img_logo'):
            self.img_logo.setText("(imagem não encontrada)")

        # Configurar gráfico
        self.plot_canvas = WindPlotCanvas(self.plot_widget)
        layout = self.plot_widget.layout()
        if layout is None:
            layout = QVBoxLayout(self.plot_widget)
        layout.addWidget(self.plot_canvas)
        self.toolbar = NavigationToolbar(self.plot_canvas, self.plot_widget)
        layout.addWidget(self.toolbar)

        # Conexões
        self.btn_baixar.clicked.connect(self.on_download)
        self.log_text.setReadOnly(True)
        self.cb_estacao_grafico.currentIndexChanged.connect(self.on_estacao_selecionada)

        # Armazenar dados
        self.dfs_estacoes = {}

        # Datas padrão
        self.dt_inicio.setDate(datetime.date(2018, 1, 1))
        self.dt_fim.setDate(datetime.date.today())

    def ler_estacoes(self):
        if not os.path.exists(self.estacoes_file):
            QMessageBox.critical(self, "Erro", f"Arquivo de estações não encontrado:\n{self.estacoes_file}")
            return []
        with open(self.estacoes_file, 'r', encoding='utf-8') as f:
            codigos = [linha.strip() for linha in f if linha.strip()]
        if not codigos:
            QMessageBox.warning(self, "Aviso", "Nenhuma estação encontrada no arquivo.")
        return codigos

    def on_download(self):
        estacoes = self.ler_estacoes()
        if not estacoes:
            return
        start_date = self.dt_inicio.date().toPyDate()
        end_date = self.dt_fim.date().toPyDate()
        if start_date > end_date:
            QMessageBox.warning(self, "Período inválido", "Data de início posterior à data de fim.")
            return

        self.log_text.clear()
        self.log_text.append(f"Iniciando download para {len(estacoes)} estações...")
        self.log_text.append(f"Período: {start_date} a {end_date}")
        self.log_text.append(f"Diretório de saída: {self.output_dir}")

        self.btn_baixar.setEnabled(False)
        self.thread = DownloadThread(estacoes, start_date, end_date)
        self.thread.log.connect(self.log_text.append)
        self.thread.finished.connect(self.on_download_finished)
        self.thread.error.connect(self.on_download_error)
        self.thread.start()

    def on_download_finished(self, resultados):
        self.btn_baixar.setEnabled(True)
        self.dfs_estacoes = {}
        estacoes_ok = []

        for codigo, df in resultados.items():
            if df is None or df.empty:
                continue
            self.dfs_estacoes[codigo] = df
            try:
                salvar_arquivo_mgb(df, self.output_dir, 'vel', prefixo=codigo)
                salvar_arquivo_mgb(df, self.output_dir, 'dir', prefixo=codigo)
                estacoes_ok.append(codigo)
            except Exception as e:
                self.log_text.append(f"Erro ao salvar arquivos para {codigo}: {e}")

        # Pop-up de conclusão
        if estacoes_ok:
            msg = f"Dados baixados com sucesso para:\n\n" + "\n".join(estacoes_ok)
            QMessageBox.information(self, "Download concluído", msg)
        else:
            QMessageBox.warning(self, "Download concluído", "Nenhum dado foi obtido.")

        # Atualizar combobox do gráfico
        self.cb_estacao_grafico.clear()
        self.cb_estacao_grafico.addItems(estacoes_ok)
        if estacoes_ok:
            self.cb_estacao_grafico.setCurrentIndex(0)

    def on_download_error(self, error_msg):
        self.btn_baixar.setEnabled(True)
        self.log_text.append(f"ERRO: {error_msg}")
        QMessageBox.critical(self, "Erro", f"Falha no download:\n{error_msg}")

    def on_estacao_selecionada(self, index):
        if index < 0:
            return
        codigo = self.cb_estacao_grafico.currentText()
        df = self.dfs_estacoes.get(codigo)
        if df is not None:
            self.plot_canvas.update_plot(df, codigo)
            self.tabWidget.setCurrentIndex(1)

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec_())