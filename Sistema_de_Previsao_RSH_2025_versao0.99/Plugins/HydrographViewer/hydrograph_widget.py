# -*- coding: utf-8 -*-
"""
Widget principal para o aplicativo Hydrograph Viewer (standalone).
Versão com viés centralizado no engine e duas chamadas para aplicar viés na primeira geração.
Inclui spinbox geral que atua como offset global em tempo real.
Permite arrastar as caixinhas de anotação (múltiplas) com o mouse.
Agora com seleção de cores por previsão via QComboBox, checkboxes individuais para anotações,
e suporte para exibição de logo personalizado no gráfico, com arraste da logo.
"""
import os
import sys
import re
import csv
import numpy as np
import pandas as pd
from datetime import datetime

from PyQt5.QtCore import Qt, pyqtSignal, QThread, QSortFilterProxyModel, QAbstractListModel, QModelIndex, QDateTime, QProcess
from PyQt5.QtWidgets import (
    QWidget, QFileDialog, QMessageBox, QVBoxLayout, QHBoxLayout, QSizePolicy,
    QCompleter, QApplication, QDialog, QPushButton, QLabel, QCheckBox, QDoubleSpinBox,
    QStackedWidget, QComboBox, QProgressDialog
)
from PyQt5.QtGui import QPixmap, QIcon, QColor
from PyQt5 import uic
from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg
from matplotlib.dates import date2num, num2date, DateFormatter, DayLocator, HourLocator
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import matplotlib.patheffects as path_effects

from hydrograph_engine import HydrographEngine
from csv_exporter import CSVExporter

# ----------------------------------------------------------------------
# Configuração de diretórios
# ----------------------------------------------------------------------
if getattr(sys, 'frozen', False):
    BASE_DIR = os.path.dirname(sys.executable)
else:
    BASE_DIR = os.path.dirname(os.path.abspath(__file__))

CSV_MAPEAMENTO = os.path.join(BASE_DIR, '..', '..', '..', 'EstacoesNivel.txt')
PLANILHA_DEFESA_CIVIL = os.path.join(BASE_DIR, 'mapeamento_entrada', 'COTAS_monitoramento3.xlsx')

PASTA_DADOS_OBSERVADOS = os.path.abspath(os.path.join(BASE_DIR, '..', '..', '..', 'database'))
EXPORT_PATH = os.path.abspath(os.path.join(BASE_DIR, '..', '..', '..', 'Arquivos do sistema', '11_03_05 Export'))
BACIA_DIR = os.path.join(BASE_DIR, 'bacias')

UI_FILE = os.path.join(BASE_DIR, 'waterlevel_viewer_dockwidget_base.ui')
FORM_CLASS, _ = uic.loadUiType(UI_FILE)

# ================================================================
# CONSTANTES VISUAIS
# ================================================================
CONFIG_GRAFICO = {
    'figsize': (12, 7),
    'estilo': 'seaborn-v0_8-whitegrid',
    'intervalo_eixo_y': 1.0,
    'subtick_eixo_y': 0.5,
}
CORES_SERIES = {
    'simulado_atual': 'blue',
    'observado': 'black'
}
CORES_ALERTA = {
    'ATENCAO': '#FFD700',
    'ALERTA': '#FFA500',
    'INUNDACAO': '#FF4500',
    'maior_INUNDACAO': '#800080'
}
LABELS_ALERTA = {
    'ATENCAO': 'Atenção',
    'ALERTA': 'Alerta',
    'INUNDACAO': 'Inundação',
    'maior_INUNDACAO': 'Cota Máxima Histórica'
}

# Cores disponíveis para seleção
CORES_DISPONIVEIS = {
    'Vermelho': '#9E0003',
    'Verde': '#006400',
    'Amarelo': '#B8860B',
    'Cinza': '#808080'
}
CORES_PADRAO = ['Vermelho', 'Verde', 'Amarelo', 'Cinza']

# Estilos de linha para cada previsão:
LINESTYLES_PREVISOES = [
    '--',                          # Previsão 1 (Base)
    '-.',                          # Previsão 2 (Overlay 1)
    (0, (8, 2, 4, 2)),             # Previsão 3 (Overlay 2)
    (0, (8, 2.6, 8, 2.6, 1, 2.6))  # Previsão 4 (Overlay 3)
]

LOGO_PATH = os.path.join(BASE_DIR, 'logo_estudo.png')
LOG_PATH = os.path.join(BASE_DIR, 'log.txt')
# ================================================================


def hex_to_rgb(hex_code):
    hex_code = hex_code.lstrip('#')
    return tuple(int(hex_code[i:i+2], 16)/255.0 for i in (0,2,4))

# ================================================================
# FUNÇÕES AUXILIARES PARA FORMATAÇÃO DE LEGENDA (CHUVA ZERO)
# ================================================================
def _is_zero_legenda(legenda):
    """Retorna True se a legenda indicar 'Chuva Zero' (contém 'zero' ou 'chuva zero')."""
    if not legenda:
        return False
    return 'zero' in legenda.lower() or 'chuva zero' in legenda.lower()

def _format_label_curva(legenda):
    """Retorna o texto a ser usado como label da curva."""
    if _is_zero_legenda(legenda):
        return "Cota Prevista sem previsão de chuva"
    else:
        return f"Cota Prevista com {legenda}"

def _format_texto_anotacao(legenda):
    """Retorna o texto a ser usado dentro da caixinha de anotação."""
    if _is_zero_legenda(legenda):
        return "sem chuva"
    else:
        return legenda

# ================================================================
# FUNÇÃO UTILITÁRIA PARA PLOTAGEM
# ================================================================
def build_hydrograph_figure(dados_plot, parametros, previsoes_acumuladas, dados_base_previsao,
                            mostrar_simulado=True, mostrar_observados=True,
                            cores_previsoes=None, logo_path=None):
    """
    Constrói a figura do hidrograma a partir dos dados fornecidos.
    Retorna fig, ax, ax_logo (ax_logo pode ser None).
    """
    import matplotlib.pyplot as plt
    from matplotlib.dates import DateFormatter, DayLocator, HourLocator, date2num, num2date
    import matplotlib.ticker as ticker
    import matplotlib.patheffects as path_effects

    if cores_previsoes is None:
        cores_previsoes = ['#9E0003', '#006400', '#B8860B', '#808080']

    plt.style.use(CONFIG_GRAFICO['estilo'])
    fig, ax = plt.subplots(figsize=CONFIG_GRAFICO['figsize'])

    # ---- Base ----
    if mostrar_observados and len(dados_plot['valores_obs_filtrado']) > 0:
        ax.plot(dados_plot['datas_obs_filtrado'], dados_plot['valores_obs_filtrado'],
                color=CORES_SERIES['observado'], marker='o', markersize=0.5,
                linestyle='-', linewidth=2, alpha=0.7, label='Cota Observada')

    if mostrar_simulado:
        ax.plot(dados_plot['dates_atual_filtrado'], dados_plot['Q_atual_filtrado'],
                color=CORES_SERIES['simulado_atual'], linewidth=2.5,
                label='Cota Simulada Atual (MGB)')

    # Previsão base
    legenda_base = parametros.get('legenda_base', 'Previsão Base')
    cor_base = cores_previsoes[0] if cores_previsoes else '#9E0003'
    ax.plot(dados_plot['dates_prev_filtrado'], dados_plot['Q_prev_filtrado'],
            color=cor_base, linestyle=LINESTYLES_PREVISOES[0], linewidth=2,
            label=_format_label_curva(legenda_base))

    # Cotas de alerta
    cotas_vistas = set()
    for chave, cota in dados_plot['cotas_alerta'].items():
        if chave in CORES_ALERTA and cota not in cotas_vistas:
            cotas_vistas.add(cota)
            cor = CORES_ALERTA[chave]
            label = LABELS_ALERTA[chave]
            estilo = ':' if chave == 'maior_INUNDACAO' else '-'
            descricao = label if chave == 'maior_INUNDACAO' else f'Cota de {label}'
            ax.axhline(y=cota, color=cor, linestyle=estilo, linewidth=1.5,
                       label='_nolegend_', zorder=0)
            ax.text(dados_plot['dates_atual_filtrado'][0] if dados_plot['dates_atual_filtrado'] else parametros['data_inicio'],
                    cota + 0.05, descricao,
                    color=cor, fontsize=11, fontweight='bold',
                    verticalalignment='bottom', horizontalalignment='left',
                    path_effects=[path_effects.Stroke(linewidth=1, foreground='black'),
                                  path_effects.Normal()])

    # Eixo X
    delta_dias = (parametros['data_fim'] - parametros['data_inicio']).days
    if delta_dias <= 0:
        delta_dias = 1
    num_ticks_desejado = 15
    intervalo_dias = max(1, delta_dias // num_ticks_desejado)
    if intervalo_dias <= 1:
        intervalo_dias = 1
    elif intervalo_dias <= 2:
        intervalo_dias = 2
    elif intervalo_dias <= 3:
        intervalo_dias = 3
    elif intervalo_dias <= 4:
        intervalo_dias = 4
    elif intervalo_dias <= 5:
        intervalo_dias = 5
    elif intervalo_dias <= 6:
        intervalo_dias = 6
    elif intervalo_dias <= 7:
        intervalo_dias = 7
    elif intervalo_dias <= 10:
        intervalo_dias = 10
    elif intervalo_dias <= 14:
        intervalo_dias = 14
    elif intervalo_dias <= 15:
        intervalo_dias = 15
    elif intervalo_dias <= 20:
        intervalo_dias = 20
    elif intervalo_dias <= 30:
        intervalo_dias = 30
    else:
        intervalo_dias = 30 * (intervalo_dias // 30)

    ax.xaxis.set_major_locator(DayLocator(interval=intervalo_dias))
    ax.xaxis.set_major_formatter(DateFormatter('%d/%m/%Y'))
    ax.xaxis.set_minor_locator(HourLocator(interval=6))
    plt.xticks(rotation=45, ha='right')

    # ---- Definir limites do eixo X antes de obter os ticks ----
    ax.set_xlim(parametros['data_inicio'], parametros['data_fim'])

    # ---- Obter ticks reais do locator ----
    locator = ax.xaxis.get_major_locator()
    xmin, xmax = ax.get_xlim()
    # Converte os limites numéricos para datetime
    dt_min = num2date(xmin)
    dt_max = num2date(xmax)
    ticks = locator.tick_values(dt_min, dt_max)
    # Filtrar ticks dentro dos limites (em unidades numéricas)
    ticks_validos = [t for t in ticks if xmin <= t <= xmax]
    if len(ticks_validos) > 1:
        for i in range(len(ticks_validos) - 1):
            meio = (ticks_validos[i] + ticks_validos[i+1]) / 2.0
            ax.axvline(x=meio, color='gray', linestyle='--', linewidth=0.6, alpha=0.4, zorder=0)

    # Título
    agora = datetime.now().replace(minute=0, second=0, microsecond=0)
    data_str = agora.strftime('%d/%m/%Y %Hh')
    ax.set_title(f'Previsão {data_str} - Estação {dados_plot["info_estacao"]["nome"]} ({dados_plot["info_estacao"]["codigo"]})',
                 fontsize=16, fontweight='bold', pad=20)

    ax.set_xlabel('Data', fontsize=12)
    ax.set_ylabel('Cota (m)', fontsize=12)

    ax.yaxis.set_major_locator(ticker.MultipleLocator(CONFIG_GRAFICO['intervalo_eixo_y']))
    ax.yaxis.set_minor_locator(ticker.MultipleLocator(CONFIG_GRAFICO['subtick_eixo_y']))
    ax.tick_params(axis='y', which='major', labelsize=10)

    if dados_plot['dates_prev_filtrado']:
        data_prev_fim = dados_plot['dates_prev_filtrado'][-1]
        max_index = np.nanargmax(dados_plot['Q_prev_filtrado'])
        data_prev_max = dados_plot['dates_prev_filtrado'][max_index]
        ax.axvline(x=data_prev_max, color='red', linestyle='--', linewidth=0.5, alpha=0.5)
        ax.axvline(x=data_prev_fim, color='red', linestyle='--', linewidth=0.5, alpha=0.5)

    # Overlays
    for i, prev in enumerate(previsoes_acumuladas):
        cor = cores_previsoes[i+1] if i+1 < len(cores_previsoes) else '#000000'
        ax.plot(prev['datas'], prev['valores'],
                color=cor, linestyle=LINESTYLES_PREVISOES[i+1], linewidth=2,
                label=_format_label_curva(prev['legenda']))

    # Anotações
    mostrar_caixinhas = parametros.get('mostrar_caixinhas', True)
    if mostrar_caixinhas:
        series = []
        # Base
        if len(dados_base_previsao['valores']) > 0:
            series.append({
                'label': dados_base_previsao['legenda'],
                'datas': dados_base_previsao['datas'],
                'valores': dados_base_previsao['valores'],
                'cor': cores_previsoes[0] if cores_previsoes else '#9E0003',
                'idx': 0
            })
        # Overlays
        for i, prev in enumerate(previsoes_acumuladas):
            if len(prev['valores']) > 0:
                cor = cores_previsoes[i+1] if i+1 < len(cores_previsoes) else '#000000'
                series.append({
                    'label': prev['legenda'],
                    'datas': prev['datas'],
                    'valores': prev['valores'],
                    'cor': cor,
                    'idx': i+1
                })

        for idx, serie in enumerate(series):
            max_idx = np.argmax(serie['valores'])
            max_data = serie['datas'][max_idx]
            max_val = serie['valores'][max_idx]
            x_peak = date2num(max_data)
            y_peak = max_val
            texto_anotacao = _format_texto_anotacao(serie['label'])
            texto = f"{texto_anotacao}\nMáx: {max_val:.2f}m\n{max_data.strftime('%d/%m %Hh')}"

            offset_y = 0.0
            x_text = x_peak + 1.0
            y_text = y_peak + offset_y

            ann = ax.annotate(
                texto,
                xy=(x_peak, y_peak),
                xytext=(x_text, y_text),
                textcoords='data',
                ha='center',
                va='bottom',
                color=serie['cor'],
                fontweight='bold',
                fontsize=9,
                bbox=dict(
                    boxstyle="round,pad=0.2",
                    fc="white",
                    alpha=0.9,
                    ec=serie['cor'],
                    lw=1.5
                ),
                picker=50,
                annotation_clip=True
            )

    # Logo - tamanho ajustado para [0.37, 0.68, 0.25, 0.25]
    ax_logo = None
    if logo_path and os.path.exists(logo_path):
        try:
            logo = plt.imread(logo_path)
            ax_logo = fig.add_axes([0.37, 0.68, 0.25, 0.25])
            ax_logo.imshow(logo)
            ax_logo.axis('off')
            ax_logo.patch.set_alpha(0)
        except Exception:
            pass

    ax.legend(loc='upper right', bbox_to_anchor=(0.99, 0.95),
              frameon=True, fancybox=True, shadow=True,
              handlelength=5, handletextpad=1.0)
    fig.tight_layout(rect=[0, 0, 1, 0.98])

    return fig, ax, ax_logo

# ----------------------------------------------------------------------
# Diálogo de seleção de bacias
# ----------------------------------------------------------------------
class BatchExportDialog(QDialog):
    estacoes_selecionadas = pyqtSignal(list)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Exportar CSV em Batelada")
        self.setModal(True)
        self.setMinimumWidth(400)
        self.selected_stations = []
        self.setup_ui()

    def setup_ui(self):
        layout = QVBoxLayout()
        label = QLabel("Selecione a lista de estações para exportar:")
        layout.addWidget(label)

        bacia_files = []
        if os.path.exists(BACIA_DIR):
            for f in os.listdir(BACIA_DIR):
                if f.endswith('.txt') and f != 'manual.txt':
                    bacia_files.append(f)
        bacia_files.sort()

        for filename in bacia_files:
            nome_bacia = os.path.splitext(filename)[0]
            btn = QPushButton(nome_bacia)
            btn.clicked.connect(lambda checked, f=filename: self.selecionar_bacia(f))
            layout.addWidget(btn)

        btn_todas = QPushButton("Selecionar todas")
        btn_todas.clicked.connect(self.selecionar_todas)
        layout.addWidget(btn_todas)

        btn_manual = QPushButton("Selecionar manualmente")
        btn_manual.clicked.connect(self.selecionar_manual)
        layout.addWidget(btn_manual)

        btn_cancel = QPushButton("Cancelar")
        btn_cancel.clicked.connect(self.reject)
        layout.addWidget(btn_cancel)

        self.setLayout(layout)

    def ler_estacoes_do_arquivo(self, caminho):
        if not os.path.exists(caminho):
            return []
        with open(caminho, 'r', encoding='utf-8') as f:
            linhas = [linha.strip() for linha in f.readlines() if linha.strip() and not linha.startswith('#')]
        return linhas

    def selecionar_bacia(self, filename):
        caminho = os.path.join(BACIA_DIR, filename)
        estacoes = self.ler_estacoes_do_arquivo(caminho)
        if not estacoes:
            QMessageBox.warning(self, "Aviso", f"O arquivo {filename} está vazio ou não foi encontrado.")
            return
        self.selected_stations = estacoes
        self.accept()

    def selecionar_todas(self):
        try:
            df = pd.read_csv(CSV_MAPEAMENTO, sep=';', encoding='latin-1')
            col_cod = None
            for col in df.columns:
                if 'COD' in col.upper():
                    col_cod = col
                    break
            if col_cod is None:
                QMessageBox.warning(self, "Erro", "Não foi possível identificar a coluna de código no mapeamento.")
                return
            estacoes = df[col_cod].astype(str).str.strip().tolist()
            if not estacoes:
                QMessageBox.warning(self, "Aviso", "Nenhuma estação encontrada no mapeamento.")
                return
            self.selected_stations = estacoes
            self.accept()
        except Exception as e:
            QMessageBox.critical(self, "Erro", f"Falha ao ler mapeamento:\n{str(e)}")

    def selecionar_manual(self):
        caminho_manual = os.path.join(BACIA_DIR, 'manual.txt')
        if not os.path.exists(caminho_manual):
            with open(caminho_manual, 'w', encoding='utf-8') as f:
                f.write("# Digite um código de estação por linha\n")
                f.write("# Exemplo:\n")
                f.write("# 86720000\n")
                f.write("# 86879300\n")

        try:
            process = QProcess()
            process.start('notepad.exe', [caminho_manual])
            process.waitForFinished(-1)
        except Exception as e:
            QMessageBox.warning(self, "Erro", f"Não foi possível abrir o editor:\n{str(e)}")
            return

        estacoes = self.ler_estacoes_do_arquivo(caminho_manual)
        if not estacoes:
            QMessageBox.warning(self, "Aviso", "O arquivo manual.txt está vazio. Nenhuma estação selecionada.")
            return
        self.selected_stations = estacoes
        self.accept()

    def get_selected_stations(self):
        return self.selected_stations

# ----------------------------------------------------------------------
# Modelo para lista de estações
# ----------------------------------------------------------------------
class EstacaoListModel(QAbstractListModel):
    def __init__(self, estacoes, parent=None):
        super().__init__(parent)
        self.estacoes = estacoes

    def rowCount(self, parent=QModelIndex()):
        return len(self.estacoes)

    def data(self, index, role=Qt.DisplayRole):
        if not index.isValid():
            return None
        if role == Qt.DisplayRole:
            codigo, nome = self.estacoes[index.row()]
            return f"{codigo} - {nome}"
        if role == Qt.UserRole:
            return self.estacoes[index.row()][0]
        return None

class EstacaoProxyModel(QSortFilterProxyModel):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.filter_text = ""

    def setFilterText(self, text):
        self.filter_text = text.lower()
        self.invalidateFilter()

    def filterAcceptsRow(self, source_row, source_parent):
        if not self.filter_text:
            return True
        index = self.sourceModel().index(source_row, 0)
        codigo_nome = self.sourceModel().data(index, Qt.DisplayRole).lower()
        return self.filter_text in codigo_nome

# ----------------------------------------------------------------------
# WORKER PARA EXPORTAÇÃO DE IMAGENS EM BATELADA
# ----------------------------------------------------------------------
class BatchImageWorker(QThread):
    progress = pyqtSignal(int, str)
    finished = pyqtSignal(bool, str)
    error = pyqtSignal(str)

    def __init__(self, estacoes, parametros_base, overlay_infos, output_dir, logo_path=None):
        super().__init__()
        self.estacoes = estacoes
        self.parametros_base = parametros_base
        self.overlay_infos = overlay_infos
        self.output_dir = output_dir
        self.logo_path = logo_path
        self._is_cancelled = False

    def cancel(self):
        self._is_cancelled = True

    def run(self):
        total = len(self.estacoes)
        erros = []

        for idx, codigo in enumerate(self.estacoes):
            if self._is_cancelled:
                self.finished.emit(False, "Exportação cancelada pelo usuário.")
                return

            self.progress.emit(idx, f"Processando estação {codigo} ({idx+1}/{total})...")

            try:
                params = self.parametros_base.copy()
                params['codigo_estacao'] = codigo
                params['legenda_base'] = self.parametros_base.get('legenda_base', 'Previsão Base')

                engine = HydrographEngine()
                resultado = engine.gerar_hidrograma(params)
                dados_plot = resultado['dados_plot']

                previsoes_acumuladas = []
                dados_base_previsao = {
                    'legenda': params['legenda_base'],
                    'datas': dados_plot['dates_prev_filtrado'],
                    'valores': dados_plot['Q_prev_filtrado']
                }

                for ov_info in self.overlay_infos:
                    ov_params = ov_info['params'].copy()
                    ov_params['codigo_estacao'] = codigo
                    ov_params['legenda_base'] = ov_info['legenda']
                    dados_prev = engine.processar_previsao(ov_params)
                    previsoes_acumuladas.append({
                        'legenda': ov_info['legenda'],
                        'datas': dados_prev['datas'],
                        'valores': dados_prev['valores']
                    })

                cores_previsoes = self.parametros_base.get('cores_previsoes', ['#9E0003', '#006400', '#B8860B', '#808080'])
                mostrar_simulado = self.parametros_base.get('mostrar_simulado', True)
                mostrar_observados = self.parametros_base.get('mostrar_observados', True)

                fig, ax, _ = build_hydrograph_figure(
                    dados_plot,
                    params,
                    previsoes_acumuladas,
                    dados_base_previsao,
                    mostrar_simulado=mostrar_simulado,
                    mostrar_observados=mostrar_observados,
                    cores_previsoes=cores_previsoes,
                    logo_path=self.logo_path
                )

                nome_arquivo = f"{codigo}.png"
                caminho_completo = os.path.join(self.output_dir, nome_arquivo)
                fig.savefig(caminho_completo, dpi=150, bbox_inches='tight')
                plt.close(fig)

                self.progress.emit(idx+1, f"Estação {codigo} salva com sucesso.")

            except Exception as e:
                msg = f"Erro ao processar estação {codigo}: {str(e)}"
                erros.append(msg)
                self.error.emit(msg)

        if erros:
            msg_final = f"Exportação concluída com {len(erros)} erro(s)."
            self.finished.emit(True, msg_final)
        else:
            self.finished.emit(True, f"Exportação concluída com sucesso. {total} imagens geradas.")

# ----------------------------------------------------------------------
# Workers para geração de gráfico
# ----------------------------------------------------------------------
class HydrographWorker(QThread):
    finished = pyqtSignal(object, object, object, float, str)
    error = pyqtSignal(str)
    def __init__(self, params, viesses=None):
        super().__init__()
        self.params = params
        self.viesses = viesses
    def run(self):
        try:
            engine = HydrographEngine()
            dados = engine.gerar_hidrograma(self.params, self.viesses)
            self.finished.emit(dados, self.params, engine.minibacia_index, engine.ultimo_bias, self.params['parhig_file'])
        except Exception as e:
            self.error.emit(str(e))

class HydrographOverlayWorker(QThread):
    finished = pyqtSignal(object)
    error = pyqtSignal(str)
    def __init__(self, params, minibacia_index, bias, parhig_file, bias_overlay=None):
        super().__init__()
        self.params = params
        self.minibacia_index = minibacia_index
        self.bias = bias
        self.parhig_file = parhig_file
        self.bias_overlay = bias_overlay
    def run(self):
        try:
            engine = HydrographEngine()
            engine.minibacia_index = self.minibacia_index
            engine.ultimo_bias = self.bias_overlay if self.bias_overlay is not None else self.bias
            self.params['parhig_file'] = self.parhig_file
            dados = engine.processar_previsao(self.params)
            self.finished.emit(dados)
        except Exception as e:
            self.error.emit(str(e))

# ----------------------------------------------------------------------
# Widget principal
# ----------------------------------------------------------------------
class HydrographWidget(QWidget, FORM_CLASS):
    def __init__(self, parent=None):
        super(HydrographWidget, self).__init__(parent)
        self.setupUi(self)

        # Definir datas padrão
        self._definir_datas_padrao()

        # Verificar arquivos fixos
        if not os.path.exists(CSV_MAPEAMENTO):
            QMessageBox.warning(self, "Aviso", f"Arquivo de mapeamento não encontrado:\n{CSV_MAPEAMENTO}")
        else:
            self._inicializar_completer_estacoes()

        if not os.path.exists(PLANILHA_DEFESA_CIVIL):
            QMessageBox.warning(self, "Aviso", f"Planilha da Defesa Civil não encontrada:\n{PLANILHA_DEFESA_CIVIL}")
        if not os.path.exists(PASTA_DADOS_OBSERVADOS):
            QMessageBox.warning(self, "Aviso", f"Pasta de dados observados não encontrada:\n{PASTA_DADOS_OBSERVADOS}")

        # Atributos
        self.worker = None
        self.current_fig = None
        self.current_data = None
        self.previsoes_acumuladas = []
        self.dados_previsoes = []
        self.dados_base_previsao = None
        self.figura_base = None
        self.ax = None
        self.dados_plot_base = None
        self.dados_base_brutos = None
        self.params_base = None
        self.last_minibacia_index = None
        self.last_bias = 0.0
        self.last_parhig_file = None
        self.last_codigo_estacao = ""

        self.dados_brutos = None

        # Anotações
        self.annotation_artists = []
        self.dragging = False
        self.picked = False
        self.dragged_artist = None
        self.drag_offset_x = 0.0
        self.drag_offset_y = 0.0
        self.cid_connect = []

        # Logo arrastável
        self.logo_ax = None
        self.dragging_logo = False
        self.drag_logo_offset_x = 0.0
        self.drag_logo_offset_y = 0.0

        # Conectar botões browse
        self.btn_browse_prev1.clicked.connect(lambda: self.browse_folder(self.txt_pasta_prev1, EXPORT_PATH))
        self.btn_browse_prev2.clicked.connect(lambda: self.browse_folder(self.txt_pasta_prev2, EXPORT_PATH))
        self.btn_browse_prev3.clicked.connect(lambda: self.browse_folder(self.txt_pasta_prev3, EXPORT_PATH))
        self.btn_browse_prev4.clicked.connect(lambda: self.browse_folder(self.txt_pasta_prev4, EXPORT_PATH))

        # Botão Gerar
        self.btn_gerar.clicked.connect(self.on_generate_clicked)

        # ================================================================
        # CHECKBOX "MOSTRAR SIMULADO"
        # ================================================================
        tab_config = self.tabWidget.widget(0)
        if tab_config:
            layout = tab_config.layout()
            if layout is None:
                layout = QVBoxLayout(tab_config)
            if not hasattr(self, 'chk_mostrar_simulado'):
                self.chk_mostrar_simulado = QCheckBox("Mostrar simulado")
                self.chk_mostrar_simulado.setChecked(True)
                layout.addWidget(self.chk_mostrar_simulado)

        # ================================================================
        # SPINBOXES
        # ================================================================
        if hasattr(self, 'spin_vies_simulado'):
            self.spin_vies_simulado = self.spin_vies_simulado
        else:
            self.spin_vies_simulado = QDoubleSpinBox()
            self.spin_vies_simulado.setRange(-100, 100)
            self.spin_vies_simulado.setSingleStep(0.1)
            self.spin_vies_simulado.setValue(0.0)
            if tab_config and layout:
                layout.addWidget(self.spin_vies_simulado)

        self.spin_vies_prev = []
        for i in range(1, 5):
            obj_name = f'spin_vies_prev{i}'
            if hasattr(self, obj_name):
                spin = getattr(self, obj_name)
            else:
                spin = QDoubleSpinBox()
                spin.setRange(-100, 100)
                spin.setSingleStep(0.1)
                spin.setValue(0.0)
                spin.setEnabled(False)
                if tab_config and layout:
                    layout.addWidget(spin)
            self.spin_vies_prev.append(spin)

        # ================================================================
        # SPINBOX GERAL
        # ================================================================
        if hasattr(self, 'spin_vies'):
            self.spin_vies.setEnabled(True)
            self.spin_vies.setToolTip("Valor geral adicionado a todos os viéses individuais (offset global).")
            self.spin_vies.setValue(0.0)
            self.spin_vies.valueChanged.connect(self._on_geral_changed)
            self._valor_anterior_geral = 0.0
        else:
            self.spin_vies = QDoubleSpinBox()
            self.spin_vies.setRange(-100, 100)
            self.spin_vies.setSingleStep(0.1)
            self.spin_vies.setValue(0.0)
            self.spin_vies.setToolTip("Valor geral adicionado a todos os viéses individuais (offset global).")
            if tab_config and layout:
                layout.addWidget(self.spin_vies)
            self.spin_vies.valueChanged.connect(self._on_geral_changed)
            self._valor_anterior_geral = 0.0

        # ================================================================
        # MENUS DE CORES
        # ================================================================
        self.cmb_cor_prev = []
        for i in range(1, 5):
            combo_name = f'cmb_cor_prev{i}'
            if hasattr(self, combo_name):
                combo = getattr(self, combo_name)
                combo.clear()
                for idx, (nome, hex_cor) in enumerate(CORES_DISPONIVEIS.items()):
                    pixmap = QPixmap(16, 16)
                    pixmap.fill(QColor(hex_cor))
                    icon = QIcon(pixmap)
                    combo.addItem(icon, nome)
                combo.setCurrentIndex(i-1)
                self.cmb_cor_prev.append(combo)
            else:
                spin = self.spin_vies_prev[i-1] if i-1 < len(self.spin_vies_prev) else None
                if spin:
                    parent_layout = spin.parent().layout()
                    if parent_layout:
                        hbox = QHBoxLayout()
                        label_cor = QLabel(f"Cor Prev {i}:")
                        hbox.addWidget(label_cor)
                        combo = QComboBox()
                        for idx, (nome, hex_cor) in enumerate(CORES_DISPONIVEIS.items()):
                            pixmap = QPixmap(16, 16)
                            pixmap.fill(QColor(hex_cor))
                            icon = QIcon(pixmap)
                            combo.addItem(icon, nome)
                        combo.setCurrentIndex(i-1)
                        hbox.addWidget(combo)
                        parent_layout.removeWidget(spin)
                        hbox.addWidget(spin)
                        parent_layout.addLayout(hbox)
                        self.cmb_cor_prev.append(combo)

        while len(self.cmb_cor_prev) < 4:
            combo = QComboBox()
            for idx, (nome, hex_cor) in enumerate(CORES_DISPONIVEIS.items()):
                pixmap = QPixmap(16, 16)
                pixmap.fill(QColor(hex_cor))
                icon = QIcon(pixmap)
                combo.addItem(icon, nome)
            combo.setCurrentIndex(len(self.cmb_cor_prev))
            self.cmb_cor_prev.append(combo)

        # ================================================================
        # CHECKBOXES INDIVIDUAIS PARA ANOTAÇÕES
        # ================================================================
        self.chk_show_annot_prev = []
        for i in range(1, 5):
            chk_name = f'chk_show_annotation_prev{i}'
            chk = self.findChild(QCheckBox, chk_name)
            if chk is not None:
                chk.setChecked(True)
                self.chk_show_annot_prev.append(chk)
            else:
                if tab_config and layout:
                    chk = QCheckBox(f"Anotar Prev {i}")
                    chk.setChecked(True)
                    layout.addWidget(chk)
                    self.chk_show_annot_prev.append(chk)
                else:
                    chk = QCheckBox(f"Anotação Prev {i}")
                    chk.setChecked(True)
                    self.layout().addWidget(chk)
                    self.chk_show_annot_prev.append(chk)

        while len(self.chk_show_annot_prev) < 4:
            chk = QCheckBox(f"Anotar Prev {len(self.chk_show_annot_prev)+1}")
            chk.setChecked(True)
            if tab_config and layout:
                layout.addWidget(chk)
            self.chk_show_annot_prev.append(chk)

        # ================================================================
        # ABA GRÁFICO
        # ================================================================
        tab_grafico = self.tabWidget.widget(1)
        if tab_grafico:
            novo_layout = QVBoxLayout(tab_grafico)
            novo_layout.setContentsMargins(0, 0, 0, 0)
            novo_layout.setSpacing(5)

            self.figure_canvas = FigureCanvasQTAgg(plt.Figure())
            self.figure_canvas.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Expanding)
            self.figure_canvas.setMinimumSize(800, 600)

            self.figure_canvas.setFocusPolicy(Qt.StrongFocus)
            self.figure_canvas.setMouseTracking(True)
            self.figure_canvas.setFocus()
            self.figure_canvas.raise_()
            self.figure_canvas.setVisible(True)
            self.figure_canvas.setAttribute(Qt.WA_TransparentForMouseEvents, False)

            novo_layout.addWidget(self.figure_canvas, 1)

            # ---- BOTÕES DA ABA GRÁFICO ----
            btn_layout = QHBoxLayout()
            btn_layout.setContentsMargins(0, 0, 0, 0)
            btn_layout.addStretch()

            if not hasattr(self, 'btn_salvar_png'):
                self.btn_salvar_png = QPushButton("Salvar PNG")
            btn_layout.addWidget(self.btn_salvar_png)
            self.btn_salvar_png.clicked.connect(self.on_save_png)

            if not hasattr(self, 'btn_exportar_csv'):
                self.btn_exportar_csv = QPushButton("Exportar CSV")
            btn_layout.addWidget(self.btn_exportar_csv)
            self.btn_exportar_csv.clicked.connect(self.on_export_csv)

            if not hasattr(self, 'btn_copiar_imagem'):
                self.btn_copiar_imagem = QPushButton("Copiar Imagem")
            self.btn_copiar_imagem.clicked.connect(self.on_copy_image)
            btn_layout.addWidget(self.btn_copiar_imagem)

            if not hasattr(self, 'btn_exportar_batelada'):
                self.btn_exportar_batelada = QPushButton("Exportar Batelada")
            self.btn_exportar_batelada.clicked.connect(self.on_export_batch)
            btn_layout.addWidget(self.btn_exportar_batelada)

            if hasattr(self, 'btn_exportar_imagens_batelada'):
                self.btn_exportar_imagens_batelada.clicked.connect(self.on_export_images_batch)
                btn_layout.addWidget(self.btn_exportar_imagens_batelada)
            else:
                self.btn_exportar_imagens_batelada = QPushButton("Salvar Imagens em Batelada")
                self.btn_exportar_imagens_batelada.clicked.connect(self.on_export_images_batch)
                btn_layout.addWidget(self.btn_exportar_imagens_batelada)

            btn_layout.addStretch()
            novo_layout.addLayout(btn_layout, 0)

            if hasattr(self, 'lbl_grafico') and self.lbl_grafico:
                self.lbl_grafico.setParent(None)
                self.lbl_grafico.deleteLater()
                self.lbl_grafico = None

            self.ax = self.figure_canvas.figure.subplots()
        else:
            # Fallback
            self.figure_canvas = FigureCanvasQTAgg(plt.Figure())
            self.figure_canvas.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Expanding)
            self.figure_canvas.setMinimumSize(800, 600)
            self.figure_canvas.setFocusPolicy(Qt.StrongFocus)
            self.figure_canvas.setMouseTracking(True)
            self.figure_canvas.setFocus()
            self.figure_canvas.raise_()
            self.figure_canvas.setVisible(True)
            self.figure_canvas.setAttribute(Qt.WA_TransparentForMouseEvents, False)

            parent_layout = self.lbl_grafico.parent().layout()
            if parent_layout:
                for i in range(parent_layout.count()):
                    if parent_layout.itemAt(i).widget() == self.lbl_grafico:
                        parent_layout.removeWidget(self.lbl_grafico)
                        parent_layout.insertWidget(i, self.figure_canvas)
                        break
            self.lbl_grafico.hide()
            self.ax = self.figure_canvas.figure.subplots()

        # Carregamento automático das pastas
        self._carregar_pastas_export()

        # Conexão para reset do viés quando código da estação muda
        self.txt_codigo_estacao.textChanged.connect(self.on_codigo_estacao_changed)

    # ------------------------------------------------------------------
    # Datas padrão
    # ------------------------------------------------------------------
    def _definir_datas_padrao(self):
        hoje = QDateTime.currentDateTime()
        data_inicio = hoje.addDays(-15)
        data_fim = hoje.addDays(15)
        self.dt_inicio.setDateTime(data_inicio)
        self.dt_fim.setDateTime(data_fim)

    # ------------------------------------------------------------------
    # Completer de estações
    # ------------------------------------------------------------------
    def _inicializar_completer_estacoes(self):
        try:
            df = pd.read_csv(CSV_MAPEAMENTO, sep=';', encoding='latin-1')
            col_cod = None
            col_nome = None
            for col in df.columns:
                if 'COD' in col.upper():
                    col_cod = col
                if 'NOME' in col.upper():
                    col_nome = col
            if col_cod is None or col_nome is None:
                raise ValueError("Colunas de código/nome não identificadas.")
            estacoes = []
            for _, row in df.iterrows():
                codigo = str(row[col_cod]).strip()
                nome = str(row[col_nome]).strip()
                if codigo and nome:
                    estacoes.append((codigo, nome))
            if not estacoes:
                raise ValueError("Nenhuma estação encontrada.")
        except Exception as e:
            QMessageBox.warning(self, "Aviso", f"Erro ao carregar estações:\n{str(e)}")
            return

        self.estacao_model = EstacaoListModel(estacoes, self)
        self.estacao_proxy = EstacaoProxyModel(self)
        self.estacao_proxy.setSourceModel(self.estacao_model)

        self.completer = QCompleter(self)
        self.completer.setModel(self.estacao_proxy)
        self.completer.setCaseSensitivity(Qt.CaseInsensitive)
        self.completer.setCompletionRole(Qt.DisplayRole)
        self.completer.setFilterMode(Qt.MatchContains)
        self.completer.setCompletionMode(QCompleter.PopupCompletion)
        self.completer.popup().setMinimumWidth(300)

        self.completer.activated.connect(self.on_estacao_selecionada)
        self.completer.activated[str].connect(self.on_estacao_selecionada)

        self.txt_codigo_estacao.setCompleter(self.completer)
        self.txt_codigo_estacao.textChanged.connect(self._atualizar_filtro_estacoes)
        self.txt_codigo_estacao.focusInEvent = self._on_focus_estacao

    def _atualizar_filtro_estacoes(self, text):
        self.estacao_proxy.setFilterText(text)

    def on_estacao_selecionada(self, item):
        if isinstance(item, QModelIndex):
            src_index = self.estacao_proxy.mapToSource(item)
            codigo = self.estacao_model.data(src_index, Qt.UserRole)
            nome = self.estacao_model.data(src_index, Qt.DisplayRole).split(' - ')[1] if ' - ' in self.estacao_model.data(src_index, Qt.DisplayRole) else ""
            if codigo:
                self.txt_codigo_estacao.setText(f"{codigo} - {nome}")
        elif isinstance(item, str):
            if ' - ' in item:
                codigo, nome = item.split(' - ', 1)
                self.txt_codigo_estacao.setText(f"{codigo.strip()} - {nome.strip()}")
            else:
                self.txt_codigo_estacao.setText(item.strip())

    def _on_focus_estacao(self, event):
        self.completer.complete()
        super(HydrographWidget, self).focusInEvent(event)

    # ================================================================
    # TROCA DE ESTAÇÃO: RESETA TUDO
    # ================================================================
    def on_codigo_estacao_changed(self, text):
        codigo = text.strip().split()[0] if text.strip() else ""
        if codigo != self.last_codigo_estacao:
            self.last_codigo_estacao = codigo
            self.spin_vies.setValue(0.0)
            self.spin_vies_simulado.setValue(0.0)
            for spin in self.spin_vies_prev:
                spin.setValue(0.0)
            if hasattr(self, 'cmb_cor_prev'):
                for idx, combo in enumerate(self.cmb_cor_prev):
                    combo.setCurrentIndex(idx)
            for chk in self.chk_show_annot_prev:
                chk.setChecked(True)

    # ------------------------------------------------------------------
    # Métodos auxiliares
    # ------------------------------------------------------------------
    def browse_folder(self, line_edit, start_dir=None):
        if start_dir is None:
            start_dir = EXPORT_PATH
        folder = QFileDialog.getExistingDirectory(self, "Selecionar pasta da simulação", start_dir)
        if folder:
            line_edit.setText(folder)

    def encontrar_arquivo_na_pasta(self, pasta, nome_arquivo):
        caminho = os.path.join(pasta, nome_arquivo)
        if os.path.exists(caminho):
            return caminho
        for sub in ['Input', 'Output']:
            sub_path = os.path.join(pasta, sub, nome_arquivo)
            if os.path.exists(sub_path):
                return sub_path
        return None

    def preparar_params_para_pasta(self, pasta, is_base=False):
        parhig = self.encontrar_arquivo_na_pasta(pasta, 'PARHIG.hig')
        meta = self.encontrar_arquivo_na_pasta(pasta, 'Previsao.meta')
        ytudo = self.encontrar_arquivo_na_pasta(pasta, 'YTUDO.MGB')
        ytudo_prev = self.encontrar_arquivo_na_pasta(pasta, 'YTUDO_prev.MGB')
        if is_base:
            if not (parhig and meta and ytudo and ytudo_prev):
                raise ValueError(f"Pasta base incompleta.\nPARHIG: {parhig}\nPrevisao.meta: {meta}\nYTUDO.MGB: {ytudo}\nYTUDO_prev.MGB: {ytudo_prev}")
        else:
            if not ytudo_prev:
                raise ValueError(f"Pasta de sobreposição sem YTUDO_prev.MGB: {pasta}")
        return {
            'parhig_file': parhig,
            'infoprevi_file': meta,
            'qtudo_atual_file': ytudo,
            'qtudo_prev_file': ytudo_prev
        }

    def get_parameters(self):
        params = {}
        params['csv_file'] = CSV_MAPEAMENTO
        params['planilha_defesa_civil'] = PLANILHA_DEFESA_CIVIL
        params['pasta_dados_observados'] = PASTA_DADOS_OBSERVADOS
        codigo_raw = self.txt_codigo_estacao.text().strip()
        if ' ' in codigo_raw:
            codigo_raw = codigo_raw.split(' ')[0]
        params['codigo_estacao'] = codigo_raw
        params['data_inicio'] = self.dt_inicio.dateTime().toPyDateTime()
        params['data_fim'] = self.dt_fim.dateTime().toPyDateTime()
        params['mostrar_observados'] = self.chk_mostrar_obs.isChecked()
        params['mostrar_caixinhas'] = self.chk_show_annotations.isChecked()
        if hasattr(self, 'chk_mostrar_simulado'):
            params['mostrar_simulado'] = self.chk_mostrar_simulado.isChecked()
        else:
            params['mostrar_simulado'] = True
        cores_selecionadas = []
        for combo in self.cmb_cor_prev:
            nome_cor = combo.currentText()
            cores_selecionadas.append(CORES_DISPONIVEIS[nome_cor])
        params['cores_previsoes'] = cores_selecionadas
        params['mostrar_anotacoes_prev'] = [chk.isChecked() for chk in self.chk_show_annot_prev]
        return params

    # ------------------------------------------------------------------
    # Carregamento automático das pastas da Export
    # ------------------------------------------------------------------
    def _extrair_modelo_do_nome_pasta(self, nome_pasta):
        padrao = r'Previsão_Inercial_\d+_\d+_\d+_\d+_(.+)'
        match = re.search(padrao, nome_pasta)
        if match:
            parte_modelo = match.group(1)
            partes = parte_modelo.split('_')
            modelo = partes[-1] if partes else parte_modelo
            if modelo == "Zero":
                return "Chuva Zero"
            return modelo
        return nome_pasta

    def _extrair_data_do_nome_pasta(self, nome_pasta):
        padrao = r'Previsão_Inercial_(\d+)_(\d+)_(\d+)_(\d+)_'
        match = re.search(padrao, nome_pasta)
        if match:
            ano, mes, dia, hora = map(int, match.groups())
            try:
                return datetime(ano, mes, dia, hora)
            except ValueError:
                return None
        return None

    def _listar_pastas_export(self):
        if not os.path.exists(EXPORT_PATH):
            return []
        pastas = []
        for item in os.listdir(EXPORT_PATH):
            caminho = os.path.join(EXPORT_PATH, item)
            if os.path.isdir(caminho):
                if self.encontrar_arquivo_na_pasta(caminho, 'YTUDO_prev.MGB') is None:
                    continue
                data = self._extrair_data_do_nome_pasta(item)
                if data is None:
                    data = datetime.fromtimestamp(os.path.getmtime(caminho))
                legenda = self._extrair_modelo_do_nome_pasta(item)
                pastas.append((caminho, legenda, data))
        pastas.sort(key=lambda x: x[2], reverse=True)
        return pastas

    def _carregar_pastas_export(self):
        todas_pastas = self._listar_pastas_export()

        for i in range(1, 5):
            getattr(self, f'txt_pasta_prev{i}').clear()
            getattr(self, f'txt_legenda_prev{i}').clear()
            getattr(self, f'chk_prev{i}').setChecked(False)
            if i-1 < len(self.spin_vies_prev):
                self.spin_vies_prev[i-1].setEnabled(False)

        if not todas_pastas:
            return

        for idx, (caminho, legenda, _) in enumerate(todas_pastas[:4]):
            campo_pasta = getattr(self, f'txt_pasta_prev{idx+1}')
            campo_legenda = getattr(self, f'txt_legenda_prev{idx+1}')
            checkbox = getattr(self, f'chk_prev{idx+1}')
            spin = self.spin_vies_prev[idx] if idx < len(self.spin_vies_prev) else None

            campo_pasta.setText(caminho)
            campo_legenda.setText(legenda)
            checkbox.setChecked(True)
            if spin:
                spin.setEnabled(True)

    # ================================================================
    # SLOT PARA SPINBOX GERAL
    # ================================================================
    def _on_geral_changed(self, novo_valor):
        delta = novo_valor - self._valor_anterior_geral
        if delta == 0:
            return

        self.spin_vies_simulado.blockSignals(True)
        for spin in self.spin_vies_prev:
            spin.blockSignals(True)

        novo_valor_sim = self.spin_vies_simulado.value() + delta
        self.spin_vies_simulado.setValue(novo_valor_sim)

        for spin in self.spin_vies_prev:
            novo_valor_prev = spin.value() + delta
            spin.setValue(novo_valor_prev)

        self.spin_vies_simulado.blockSignals(False)
        for spin in self.spin_vies_prev:
            spin.blockSignals(False)

        self._valor_anterior_geral = novo_valor

    # ------------------------------------------------------------------
    # Geração do gráfico
    # ------------------------------------------------------------------
    def on_generate_clicked(self):
        pastas = []
        legendas = []
        for i in range(1, 5):
            pasta = getattr(self, f'txt_pasta_prev{i}').text().strip()
            legenda = getattr(self, f'txt_legenda_prev{i}').text().strip()
            checkbox = getattr(self, f'chk_prev{i}')
            if pasta and checkbox.isChecked():
                pastas.append(pasta)
                if legenda:
                    legendas.append(legenda)
                else:
                    nome_pasta = os.path.basename(pasta)
                    if nome_pasta == "Zero":
                        legendas.append("Chuva Zero")
                    else:
                        legendas.append(nome_pasta)
            elif i == 1 and not pasta:
                QMessageBox.warning(self, "Erro", "A pasta base (Previsão 1) é obrigatória e deve estar ativa.")
                return
        if not pastas:
            QMessageBox.warning(self, "Erro", "Nenhuma pasta selecionada ou ativada.")
            return

        params = self.get_parameters()
        base_folder = pastas[0]
        try:
            base_files = self.preparar_params_para_pasta(base_folder, is_base=True)
        except Exception as e:
            QMessageBox.critical(self, "Erro", str(e))
            return
        params.update(base_files)

        viesses_atuais = {
            'simulado': self.spin_vies_simulado.value(),
            'base': self.spin_vies_prev[0].value() if len(self.spin_vies_prev) > 0 else 0.0,
            'overlay': [spin.value() for spin in self.spin_vies_prev[1:]]
        }

        self.previsoes_acumuladas = []
        self.dados_previsoes = []
        self.figura_base = None

        self.btn_gerar.setEnabled(False)
        self.btn_gerar.setText("Processando...")

        self.worker = HydrographWorker(params, viesses_atuais)
        self.worker.finished.connect(
            lambda dados, p, idx, bias, parhig: self._on_first_worker_done(dados, p, idx, bias, parhig, pastas, legendas, params, viesses_atuais)
        )
        self.worker.error.connect(self.on_processing_error)
        self.worker.start()

    def _on_first_worker_done(self, dados, params, minibacia_index, bias, parhig_file, pastas, legendas, original_params, viesses_atuais):
        self.dados_brutos = dados.get('dados_brutos')
        self.last_bias = bias

        precisa_atualizar = False
        if self.dados_brutos:
            bias_calc = self.dados_brutos['bias_calculado']
            if self.spin_vies_simulado.value() == 0.0:
                self.spin_vies_simulado.setValue(bias_calc)
                precisa_atualizar = True
            for spin in self.spin_vies_prev:
                if spin.value() == 0.0:
                    spin.setValue(bias_calc)
                    precisa_atualizar = True

        if precisa_atualizar:
            novos_viesses = {
                'simulado': self.spin_vies_simulado.value(),
                'base': self.spin_vies_prev[0].value() if len(self.spin_vies_prev) > 0 else 0.0,
                'overlay': [spin.value() for spin in self.spin_vies_prev[1:]]
            }
            self.worker = HydrographWorker(original_params, novos_viesses)
            self.worker.finished.connect(
                lambda dados_final, p, idx, bias_final, parhig_final: self.on_base_processed(
                    dados_final, p, idx, bias_final, parhig_final, pastas, legendas
                )
            )
            self.worker.error.connect(self.on_processing_error)
            self.worker.start()
        else:
            self.on_base_processed(dados, params, minibacia_index, bias, parhig_file, pastas, legendas)

    def on_base_processed(self, dados, params, minibacia_index, bias, parhig_file, pastas, legendas):
        self.last_minibacia_index = minibacia_index
        self.last_bias = bias
        self.last_parhig_file = parhig_file

        legenda_base = legendas[0]
        params['legenda_base'] = legenda_base

        self.dados_plot_base = dados['dados_plot']
        self.params_base = params.copy()

        self.dados_base_brutos = {
            'info_estacao': dados['dados_plot']['info_estacao'],
            'cotas_alerta': dados['dados_plot']['cotas_alerta'],
            'dates_atual_filtrado': dados['dados_plot']['dates_atual_filtrado'],
            'Q_atual_filtrado': dados['dados_plot']['Q_atual_filtrado'],
            'dates_prev_filtrado': dados['dados_plot']['dates_prev_filtrado'],
            'Q_prev_filtrado': dados['dados_plot']['Q_prev_filtrado'],
            'datas_obs_filtrado': dados['dados_plot']['datas_obs_filtrado'],
            'valores_obs_filtrado': dados['dados_plot']['valores_obs_filtrado'],
            'bias_vies': bias
        }
        self.dados_base_previsao = {
            'legenda': legenda_base,
            'datas': dados['dados_plot']['dates_prev_filtrado'],
            'valores': dados['dados_plot']['Q_prev_filtrado']
        }
        self.dados_previsoes = [self.dados_base_previsao.copy()]
        self.current_data = dados['dados_export']

        self.pastas_adicionais = pastas[1:]
        self.legendas_adicionais = legendas[1:]
        self.overlay_index = 0
        self.process_next_overlay()

    def process_next_overlay(self):
        if self.overlay_index >= len(self.pastas_adicionais):
            self.finalize_generation()
            return
        pasta = self.pastas_adicionais[self.overlay_index]
        legenda = self.legendas_adicionais[self.overlay_index]
        try:
            overlay_files = self.preparar_params_para_pasta(pasta, is_base=False)
        except Exception as e:
            QMessageBox.warning(self, "Aviso", f"Erro na pasta adicional {pasta}: {e}")
            self.overlay_index += 1
            self.process_next_overlay()
            return

        overlay_params = self.get_parameters()
        overlay_params['parhig_file'] = overlay_files.get('parhig_file') or self.params_base['parhig_file']
        overlay_params['infoprevi_file'] = overlay_files.get('infoprevi_file') or self.params_base['infoprevi_file']
        overlay_params['qtudo_atual_file'] = overlay_files.get('qtudo_atual_file') or self.params_base['qtudo_atual_file']
        overlay_params['qtudo_prev_file'] = overlay_files['qtudo_prev_file']
        overlay_params['legenda_base'] = legenda

        idx_spin = self.overlay_index + 1
        if idx_spin < len(self.spin_vies_prev):
            bias_overlay = self.spin_vies_prev[idx_spin].value()
        else:
            bias_overlay = 0.0

        self.overlay_worker = HydrographOverlayWorker(
            overlay_params,
            self.last_minibacia_index,
            self.last_bias,
            overlay_params['parhig_file'],
            bias_overlay=bias_overlay
        )
        self.overlay_worker.finished.connect(self.on_overlay_finished)
        self.overlay_worker.error.connect(self.on_processing_error)
        self.overlay_worker.start()

    def on_overlay_finished(self, dados_prev):
        legenda = self.legendas_adicionais[self.overlay_index]
        self.previsoes_acumuladas.append({
            'legenda': legenda,
            'datas': dados_prev['datas'],
            'valores': dados_prev['valores']
        })
        self.dados_previsoes.append({
            'legenda': legenda,
            'datas': dados_prev['datas'],
            'valores': dados_prev['valores']
        })
        self.overlay_index += 1
        self.process_next_overlay()

    # ================================================================
    # CONSTRUÇÃO DA FIGURA (com suporte a logo arrastável)
    # ================================================================
    def _construir_figura_completa(self, dados_plot, parametros, mostrar_simulado=True, mostrar_observados=True):
        fig, ax, ax_logo = build_hydrograph_figure(
            dados_plot,
            parametros,
            self.previsoes_acumuladas,
            self.dados_base_previsao,
            mostrar_simulado=mostrar_simulado,
            mostrar_observados=mostrar_observados,
            cores_previsoes=parametros.get('cores_previsoes', ['#9E0003', '#006400', '#B8860B', '#808080']),
            logo_path=LOGO_PATH
        )
        self.logo_ax = ax_logo
        return fig, ax

    # ------------------------------------------------------------------
    # Função para escrever no log
    # ------------------------------------------------------------------
    def _write_log(self, estacao_codigo, estacao_nome, mini, data_hora_str):
        try:
            with open(LOG_PATH, 'a', encoding='utf-8') as f:
                f.write(f"[{data_hora_str}] Estação: {estacao_nome} ({estacao_codigo}) - Mini: {mini}\n")
        except Exception as e:
            print(f"Erro ao escrever log: {e}")

    # ------------------------------------------------------------------
    # Coleta das anotações para arraste
    # ------------------------------------------------------------------
    def _collect_annotations(self, ax):
        """Percorre os elementos do eixo e armazena as anotações com picker."""
        self.annotation_artists = []
        for child in ax.get_children():
            if isinstance(child, plt.Annotation) and child.get_picker() is not None:
                self.annotation_artists.append(child)
        print(f"Coletadas {len(self.annotation_artists)} anotações para arraste.")

    # ------------------------------------------------------------------
    # Finalização
    # ------------------------------------------------------------------
    def finalize_generation(self):
        for cid in self.cid_connect:
            try:
                self.figure_canvas.mpl_disconnect(cid)
            except:
                pass
        self.cid_connect.clear()
        self.annotation_artists = []
        self.picked = False
        self.dragging = False
        self.dragged_artist = None
        self.logo_ax = None
        self.dragging_logo = False

        mostrar_simulado = self.params_base.get('mostrar_simulado', True)
        mostrar_observados = self.params_base.get('mostrar_observados', True)

        fig, ax = self._construir_figura_completa(
            self.dados_plot_base,
            self.params_base,
            mostrar_simulado=mostrar_simulado,
            mostrar_observados=mostrar_observados
        )

        self._collect_annotations(ax)

        self.current_fig = fig
        self.ax = ax
        self.figura_base = fig
        self.display_graph(fig)
        self.display_data(self.dados_previsoes)

        # Log
        try:
            info_estacao = self.dados_plot_base['info_estacao']
            codigo = info_estacao.get('codigo', 'N/A')
            nome = info_estacao.get('nome', 'N/A')
            mini = self.last_minibacia_index + 1 if self.last_minibacia_index is not None else 'N/A'
            agora = datetime.now().replace(minute=0, second=0, microsecond=0)
            data_hora_str = agora.strftime('%d/%m/%Y %Hh')
            self._write_log(codigo, nome, mini, data_hora_str)
        except Exception as e:
            print(f"Erro ao registrar log: {e}")

        self.btn_gerar.setEnabled(True)
        self.btn_gerar.setText("Gerar Gráfico")
        self.tabWidget.setCurrentIndex(1)

        tab_grafico = self.tabWidget.widget(1)
        if tab_grafico:
            tab_grafico.show()
            tab_grafico.setVisible(True)
            tab_grafico.raise_()

        stacked = self.tabWidget.findChild(QStackedWidget)
        if stacked:
            stacked.setCurrentIndex(1)
            if stacked.widget(1):
                stacked.widget(1).show()
                stacked.widget(1).setVisible(True)

    # ================================================================
    # EXPORTAÇÃO DE IMAGENS EM BATELADA
    # ================================================================
    def on_export_images_batch(self):
        if self.params_base is None:
            QMessageBox.warning(self, "Aviso", "Gere um gráfico base primeiro antes de exportar imagens em batelada.")
            return

        dialog = BatchExportDialog(self)
        if dialog.exec_() != QDialog.Accepted:
            return

        estacoes = dialog.get_selected_stations()
        if not estacoes:
            QMessageBox.warning(self, "Aviso", "Nenhuma estação selecionada.")
            return

        output_dir = QFileDialog.getExistingDirectory(
            self,
            "Selecione a pasta onde as imagens serão salvas",
            BASE_DIR
        )
        if not output_dir:
            output_dir = BASE_DIR

        agora = datetime.now().replace(minute=0, second=0, microsecond=0)
        timestamp = agora.strftime('%Y_%m_%d_%H')
        nome_pasta = f"PrevisaoMGB_{timestamp}"
        caminho_completo = os.path.join(output_dir, nome_pasta)
        try:
            os.makedirs(caminho_completo, exist_ok=True)
        except Exception as e:
            QMessageBox.critical(self, "Erro", f"Não foi possível criar o diretório:\n{str(e)}")
            return

        base_params = self.params_base.copy()
        base_params.pop('codigo_estacao', None)
        base_params.pop('legenda_base', None)
        base_params.pop('bias_manual', None)

        overlay_infos = []
        for pasta, legenda in zip(self.pastas_adicionais, self.legendas_adicionais):
            try:
                overlay_files = self.preparar_params_para_pasta(pasta, is_base=False)
            except Exception as e:
                print(f"  ATENÇÃO: Erro ao preparar parâmetros para {legenda}: {e}")
                continue
            ov_params = base_params.copy()
            ov_params['parhig_file'] = overlay_files.get('parhig_file') or base_params.get('parhig_file')
            ov_params['infoprevi_file'] = overlay_files.get('infoprevi_file') or base_params.get('infoprevi_file')
            ov_params['qtudo_atual_file'] = overlay_files.get('qtudo_atual_file') or base_params.get('qtudo_atual_file')
            ov_params['qtudo_prev_file'] = overlay_files['qtudo_prev_file']
            ov_params['legenda_base'] = legenda
            overlay_infos.append({
                'legenda': legenda,
                'params': ov_params
            })

        self.batch_worker = BatchImageWorker(
            estacoes,
            base_params,
            overlay_infos,
            caminho_completo,
            logo_path=LOGO_PATH
        )

        self.batch_worker.progress.connect(self._on_batch_progress)
        self.batch_worker.finished.connect(self._on_batch_finished)
        self.batch_worker.error.connect(self._on_batch_error)

        self.progress_dialog = QProgressDialog(
            "Iniciando exportação...",
            "Cancelar",
            0,
            len(estacoes),
            self
        )
        self.progress_dialog.setWindowTitle("Exportando imagens")
        self.progress_dialog.setModal(True)
        self.progress_dialog.canceled.connect(self.batch_worker.cancel)
        self.progress_dialog.show()

        self.batch_worker.start()

    def _on_batch_progress(self, value, message):
        if hasattr(self, 'progress_dialog') and self.progress_dialog:
            self.progress_dialog.setValue(value)
            self.progress_dialog.setLabelText(message)

    def _on_batch_finished(self, success, message):
        if hasattr(self, 'progress_dialog') and self.progress_dialog:
            self.progress_dialog.close()
        if success:
            QMessageBox.information(self, "Sucesso", message)
        else:
            QMessageBox.warning(self, "Aviso", message)

    def _on_batch_error(self, error_msg):
        print(f"Erro na exportação em batelada: {error_msg}")
        if hasattr(self, 'progress_dialog') and self.progress_dialog:
            self.progress_dialog.setLabelText(f"Erro: {error_msg}")

    # ================================================================
    # MÉTODOS PARA ARRASTE DA LOGO
    # ================================================================
    def _is_point_in_logo(self, x, y):
        """Verifica se o ponto (em coordenadas da figura) está dentro do eixo do logo."""
        if self.logo_ax is None:
            return False
        try:
            pos = self.logo_ax.get_position()
            if pos.x0 <= x <= pos.x1 and pos.y0 <= y <= pos.y1:
                return True
        except Exception:
            pass
        return False

    # ================================================================
    # EVENTOS DE MOUSE PARA ANOTAÇÕES E LOGO
    # ================================================================
    def _get_annotation_center_data(self, artist):
        if artist is None or self.figure_canvas.renderer is None:
            return None
        try:
            bbox = artist.get_window_extent(self.figure_canvas.renderer)
            x0, y0 = self.ax.transData.inverted().transform((bbox.x0, bbox.y0))
            x1, y1 = self.ax.transData.inverted().transform((bbox.x1, bbox.y1))
            return (x0 + x1) / 2, (y0 + y1) / 2
        except Exception:
            return None

    def _connect_events(self):
        for cid in self.cid_connect:
            try:
                self.figure_canvas.mpl_disconnect(cid)
            except:
                pass
        self.cid_connect.clear()

        cid_test = self.figure_canvas.mpl_connect('button_press_event', self._test_click)
        cid_pick = self.figure_canvas.mpl_connect('pick_event', self._on_pick)
        cid_press = self.figure_canvas.mpl_connect('button_press_event', self._on_press)
        cid_release = self.figure_canvas.mpl_connect('button_release_event', self._on_release)
        cid_motion = self.figure_canvas.mpl_connect('motion_notify_event', self._on_motion)

        self.cid_connect = [cid_test, cid_pick, cid_press, cid_release, cid_motion]
        print("Eventos Matplotlib conectados. IDs:", self.cid_connect)

    def _test_click(self, event):
        print(f"TESTE (Matplotlib): Clique detectado no canvas! Posição: x={event.x}, y={event.y}, botão={event.button}")

    def _on_pick(self, event):
        print("PICK EVENT chamado")
        if event.artist in self.annotation_artists:
            self.picked = True
            self.dragged_artist = event.artist
            self.figure_canvas.setCursor(Qt.PointingHandCursor)
            center = self._get_annotation_center_data(event.artist)
            if center is None:
                return
            cx, cy = center
            x_click, y_click = self.ax.transData.inverted().transform((event.mouseevent.x, event.mouseevent.y))
            self.drag_offset_x = x_click - cx
            self.drag_offset_y = y_click - cy
            if event.mouseevent.button == 1:
                self.dragging = True
                print("Arraste iniciado via pick_event")
        else:
            self.picked = False
            self.dragged_artist = None

    def _on_press(self, event):
        # Verificar se o clique foi no logo (antes de qualquer outra lógica)
        if self.logo_ax is not None:
            fig_coords = self.figure_canvas.figure.transFigure.inverted().transform((event.x, event.y))
            if self._is_point_in_logo(fig_coords[0], fig_coords[1]):
                self.dragging_logo = True
                pos = self.logo_ax.get_position()
                self.drag_logo_offset_x = fig_coords[0] - pos.x0
                self.drag_logo_offset_y = fig_coords[1] - pos.y0
                self.figure_canvas.setCursor(Qt.PointingHandCursor)
                print("Logo arrastável iniciado")
                return

        # Lógica de arraste das anotações
        print("PRESS EVENT chamado")
        if event.inaxes != self.ax:
            return
        if not self.annotation_artists:
            return

        for artist in self.annotation_artists:
            contains, _ = artist.contains(event)
            if contains:
                self.picked = True
                self.dragged_artist = artist
                self.figure_canvas.setCursor(Qt.PointingHandCursor)
                center = self._get_annotation_center_data(artist)
                if center is None:
                    return
                cx, cy = center
                x_click, y_click = self.ax.transData.inverted().transform((event.x, event.y))
                self.drag_offset_x = x_click - cx
                self.drag_offset_y = y_click - cy
                if event.button == 1:
                    self.dragging = True
                    print("Arraste iniciado via contains")
                return

        x_click, y_click = self.ax.transData.inverted().transform((event.x, event.y))
        best_dist = float('inf')
        best_artist = None
        for artist in self.annotation_artists:
            center = self._get_annotation_center_data(artist)
            if center is None:
                continue
            cx, cy = center
            dist = np.sqrt((x_click - cx)**2 + (y_click - cy)**2)
            if dist < best_dist:
                best_dist = dist
                best_artist = artist
        if best_artist is not None and best_dist < 1.5 and event.button == 1:
            self.picked = True
            self.dragged_artist = best_artist
            self.dragging = True
            center = self._get_annotation_center_data(best_artist)
            if center is None:
                return
            cx, cy = center
            self.drag_offset_x = x_click - cx
            self.drag_offset_y = y_click - cy
            self.figure_canvas.setCursor(Qt.PointingHandCursor)
            print("Arraste iniciado via fallback (distância)")

    def _on_motion(self, event):
        # Verificar se está arrastando o logo
        if self.dragging_logo and self.logo_ax is not None:
            fig_coords = self.figure_canvas.figure.transFigure.inverted().transform((event.x, event.y))
            new_x = fig_coords[0] - self.drag_logo_offset_x
            new_y = fig_coords[1] - self.drag_logo_offset_y
            bbox = self.logo_ax.get_position()
            width = bbox.width
            height = bbox.height
            new_x = max(0, min(1 - width, new_x))
            new_y = max(0, min(1 - height, new_y))
            self.logo_ax.set_position([new_x, new_y, width, height])
            self.figure_canvas.draw_idle()
            return

        # Caso contrário, tratar arraste de anotações
        if event.inaxes != self.ax:
            return
        if self.dragging and self.dragged_artist is not None:
            x_data, y_data = self.ax.transData.inverted().transform((event.x, event.y))
            x_text, y_text = self.dragged_artist.get_position()
            center = self._get_annotation_center_data(self.dragged_artist)
            if center is None:
                return
            cx_atual, cy_atual = center
            offset_cx = cx_atual - x_text
            offset_cy = cy_atual - y_text

            novo_cx = x_data - self.drag_offset_x
            novo_cy = y_data - self.drag_offset_y
            novo_x_text = novo_cx - offset_cx
            novo_y_text = novo_cy - offset_cy

            self.dragged_artist.set_position((novo_x_text, novo_y_text))
            self.figure_canvas.draw_idle()
        else:
            # Mudar cursor se estiver sobre alguma anotação ou logo
            cursor_changed = False
            if self.annotation_artists:
                x_click, y_click = self.ax.transData.inverted().transform((event.x, event.y))
                for artist in self.annotation_artists:
                    center = self._get_annotation_center_data(artist)
                    if center is None:
                        continue
                    cx, cy = center
                    dist = np.sqrt((x_click - cx)**2 + (y_click - cy)**2)
                    if dist < 1.5:
                        self.figure_canvas.setCursor(Qt.PointingHandCursor)
                        cursor_changed = True
                        break
            # Verificar se está sobre o logo
            if not cursor_changed and self.logo_ax is not None:
                fig_coords = self.figure_canvas.figure.transFigure.inverted().transform((event.x, event.y))
                if self._is_point_in_logo(fig_coords[0], fig_coords[1]):
                    self.figure_canvas.setCursor(Qt.PointingHandCursor)
                    cursor_changed = True
            if not cursor_changed:
                self.figure_canvas.setCursor(Qt.ArrowCursor)

    def _on_release(self, event):
        print("RELEASE EVENT chamado")
        if self.dragging_logo:
            self.dragging_logo = False
            self.figure_canvas.setCursor(Qt.ArrowCursor)
            print("Logo arrastado finalizado")
            return

        if self.dragging:
            self.dragging = False
            self.dragged_artist = None
            self.figure_canvas.setCursor(Qt.ArrowCursor)
            self.picked = False
            print("Arraste finalizado")

    # ------------------------------------------------------------------
    # Exibição e salvamento
    # ------------------------------------------------------------------
    def display_graph(self, fig):
        self.figure_canvas.figure = fig
        self.figure_canvas.draw()
        self.ax = fig.axes[0]

        self.figure_canvas.show()
        self.figure_canvas.setVisible(True)
        self.figure_canvas.setAttribute(Qt.WA_TransparentForMouseEvents, False)
        self.figure_canvas.raise_()

        parent = self.figure_canvas.parent()
        while parent:
            parent.show()
            parent.setVisible(True)
            parent = parent.parent()

        stacked = self.tabWidget.findChild(QStackedWidget)
        if stacked:
            stacked.setCurrentIndex(1)
            if stacked.widget(1):
                stacked.widget(1).show()
                stacked.widget(1).setVisible(True)

        self.figure_canvas.setFocus()
        self.figure_canvas.setFocusPolicy(Qt.StrongFocus)
        self.figure_canvas.repaint()
        self.figure_canvas.draw_idle()
        self.figure_canvas.flush_events()

        self._connect_events()

        print("=== APÓS display_graph ===")
        print("Canvas geometry:", self.figure_canvas.geometry())
        print("Canvas parent:", self.figure_canvas.parent())
        print("Figure ID:", id(fig))
        print("Canvas figure:", self.figure_canvas.figure)
        print("Canvas isVisible:", self.figure_canvas.isVisible())
        print("Canvas isVisibleToParent:", self.figure_canvas.isVisibleTo(self.figure_canvas.parent()))

    def display_data(self, dados_previsoes):
        if hasattr(self, 'txt_preview_dados'):
            texto = "Dados carregados:\n"
            for p in dados_previsoes:
                texto += f"{p['legenda']}: {len(p['datas'])} registros\n"
            self.txt_preview_dados.setText(texto)

    def on_save_png(self):
        if self.current_fig is None:
            QMessageBox.warning(self, "Aviso", "Nenhum gráfico gerado ainda.")
            return
        filename, _ = QFileDialog.getSaveFileName(self, "Salvar PNG", "hidrograma.png", "PNG files (*.png)")
        if filename:
            try:
                self.current_fig.savefig(filename, dpi=150, bbox_inches='tight')
                QMessageBox.information(self, "Sucesso", f"Gráfico salvo em:\n{filename}")
            except Exception as e:
                QMessageBox.critical(self, "Erro", f"Falha ao salvar PNG:\n{str(e)}")

    def on_export_csv(self):
        if self.dados_plot_base is None:
            QMessageBox.warning(self, "Aviso", "Nenhum dado gerado ainda.")
            return
        dados_base = self.current_data
        filename, _ = QFileDialog.getSaveFileName(self, "Exportar CSV", "dados_hidrograma.csv", "CSV files (*.csv)")
        if filename:
            try:
                CSVExporter.exportar_hidrograma_completo(dados_base, self.dados_previsoes, filename)
                QMessageBox.information(self, "Sucesso", f"Dados exportados para:\n{filename}")
            except Exception as e:
                QMessageBox.critical(self, "Erro", f"Falha ao exportar CSV:\n{str(e)}")

    def on_copy_image(self):
        if self.current_fig is None:
            QMessageBox.warning(self, "Aviso", "Nenhum gráfico gerado ainda.")
            return
        try:
            import io
            from PyQt5.QtGui import QPixmap
            from PyQt5.QtWidgets import QApplication
            buffer = io.BytesIO()
            self.current_fig.savefig(buffer, format='png', dpi=150, bbox_inches='tight')
            buffer.seek(0)
            pixmap = QPixmap()
            pixmap.loadFromData(buffer.getvalue())
            clipboard = QApplication.clipboard()
            clipboard.setPixmap(pixmap)
            QMessageBox.information(self, "Sucesso", "Imagem copiada para a área de transferência!")
        except Exception as e:
            QMessageBox.critical(self, "Erro", f"Falha ao copiar imagem:\n{str(e)}")

    # ======================================================================
    # EXPORTAÇÃO EM BATELADA (CSV)
    # ======================================================================
    def on_export_batch(self):
        if self.params_base is None:
            QMessageBox.warning(self, "Aviso", "Gere um gráfico base primeiro antes de exportar em batelada.")
            return

        dialog = BatchExportDialog(self)
        if dialog.exec_() == QDialog.Accepted:
            estacoes = dialog.get_selected_stations()
            if not estacoes:
                QMessageBox.warning(self, "Aviso", "Nenhuma estação selecionada.")
                return
            filename, _ = QFileDialog.getSaveFileName(
                self, "Salvar CSV em Batelada", "exportacao_batelada.csv", "CSV files (*.csv)"
            )
            if filename:
                self.iniciar_exportacao_batelada(estacoes, filename)

    def iniciar_exportacao_batelada(self, estacoes, arquivo_saida):
        try:
            if self.params_base is None:
                QMessageBox.warning(self, "Aviso", "Gere um gráfico base primeiro.")
                return

            data_inicio = self.params_base.get('data_inicio')
            data_fim = self.params_base.get('data_fim')
            if not data_inicio or not data_fim:
                QMessageBox.warning(self, "Aviso", "Datas do gráfico não encontradas.")
                return

            base_params = self.params_base.copy()
            base_params.pop('codigo_estacao', None)
            base_params.pop('legenda_base', None)
            base_params.pop('bias_manual', None)

            legenda_base = self.params_base.get('legenda_base', 'Previsão Base')

            overlay_infos = []
            for pasta, legenda in zip(self.pastas_adicionais, self.legendas_adicionais):
                try:
                    overlay_files = self.preparar_params_para_pasta(pasta, is_base=False)
                except Exception as e:
                    print(f"  ATENÇÃO: Erro ao preparar parâmetros para {legenda}: {e}")
                    continue
                ov_params = base_params.copy()
                ov_params['parhig_file'] = overlay_files.get('parhig_file') or base_params.get('parhig_file')
                ov_params['infoprevi_file'] = overlay_files.get('infoprevi_file') or base_params.get('infoprevi_file')
                ov_params['qtudo_atual_file'] = overlay_files.get('qtudo_atual_file') or base_params.get('qtudo_atual_file')
                ov_params['qtudo_prev_file'] = overlay_files['qtudo_prev_file']
                ov_params['legenda_base'] = legenda
                overlay_infos.append({
                    'legenda': legenda,
                    'params': ov_params
                })

            print("\n=== INICIANDO EXPORTAÇÃO EM BATELADA ===")
            print(f"Estações a processar: {estacoes}")
            print(f"Data início: {data_inicio}, Data fim: {data_fim}")
            print(f"Número de sobreposições: {len(overlay_infos)}")

            dados_por_estacao = []
            for codigo in estacoes:
                print(f"\n--- Processando estação: {codigo} ---")
                try:
                    params = base_params.copy()
                    params['codigo_estacao'] = codigo
                    params['legenda_base'] = legenda_base

                    engine = HydrographEngine()
                    print(f"  Chamando engine.gerar_hidrograma para {codigo}...")
                    resultado = engine.gerar_hidrograma(params)
                    dados_plot = resultado['dados_plot']
                    info_estacao = dados_plot['info_estacao']
                    nome = info_estacao['nome'] if info_estacao else codigo

                    chaves_obs = [k for k in dados_plot.keys() if 'obs' in k.lower()]
                    print(f"  Chaves com 'obs': {chaves_obs}")

                    datas_obs = None
                    valores_obs = None

                    for key_date in ['dates_obs_filtrado', 'datas_obs_filtrado', 'date_obs', 'data_obs']:
                        if key_date in dados_plot:
                            datas_obs = dados_plot[key_date]
                            for key_val in ['valores_obs_filtrado', 'values_obs_filtrado', 'valor_obs', 'Q_obs']:
                                if key_val in dados_plot:
                                    valores_obs = dados_plot[key_val]
                                    break
                            if valores_obs is not None:
                                break

                    if datas_obs is None:
                        for key in dados_plot.keys():
                            if 'obs' in key.lower() and ('date' in key.lower() or 'data' in key.lower()):
                                datas_obs = dados_plot[key]
                                val_key = key.replace('date', 'valor').replace('dates', 'valores').replace('data', 'valor')
                                if val_key in dados_plot:
                                    valores_obs = dados_plot[val_key]
                                else:
                                    for suffix in ['_obs', '_filtrado', '']:
                                        for prefix in ['Q_', 'valor_', 'valores_']:
                                            test_key = prefix + key.replace('date', '').replace('data', '').strip('_') + suffix
                                            if test_key in dados_plot:
                                                valores_obs = dados_plot[test_key]
                                                break
                                        if valores_obs is not None:
                                            break
                                if valores_obs is not None:
                                    break

                    if datas_obs is None:
                        datas_obs = []
                        valores_obs = []
                        print("  ATENÇÃO: Não foi possível encontrar chaves para dados observados.")
                    else:
                        print(f"  Encontrado: datas_obs com {len(datas_obs)} registros, valores_obs com {len(valores_obs)} registros")

                    simulado_corrigido = list(dados_plot['Q_atual_filtrado'])
                    datas_sim = list(dados_plot['dates_atual_filtrado'])

                    prev_base_corrigido = list(dados_plot['Q_prev_filtrado'])
                    datas_prev_base = list(dados_plot['dates_prev_filtrado'])

                    previsoes = []
                    previsoes.append({
                        'legenda': params['legenda_base'],
                        'datas': datas_prev_base,
                        'valores': prev_base_corrigido
                    })

                    for overlay in overlay_infos:
                        ov_params = overlay['params'].copy()
                        ov_params['codigo_estacao'] = codigo
                        ov_params['legenda_base'] = overlay['legenda']
                        print(f"  Processando sobreposição: {overlay['legenda']}")
                        dados_prev = engine.processar_previsao(ov_params)
                        prev_corrigido = list(dados_prev['valores'])
                        print(f"    datas: {len(dados_prev['datas'])}, valores: {len(prev_corrigido)}")
                        previsoes.append({
                            'legenda': overlay['legenda'],
                            'datas': dados_prev['datas'],
                            'valores': prev_corrigido
                        })

                    def to_datetime_list(dt_list):
                        if not dt_list:
                            return []
                        return [pd.to_datetime(d).replace(tzinfo=None) for d in dt_list]

                    datas_sim_clean = to_datetime_list(datas_sim)
                    datas_obs_clean = to_datetime_list(datas_obs)

                    if valores_obs is not None:
                        if len(valores_obs) != len(datas_obs_clean):
                            print(f"  Ajustando tamanhos: valores_obs ({len(valores_obs)}) vs datas_obs ({len(datas_obs_clean)})")
                            min_len = min(len(valores_obs), len(datas_obs_clean))
                            valores_obs = list(valores_obs[:min_len])
                            datas_obs_clean = datas_obs_clean[:min_len]
                        else:
                            valores_obs = list(valores_obs)
                    else:
                        valores_obs = []

                    dados_estacao = {
                        'codigo': codigo,
                        'nome': nome,
                        'datas_sim': datas_sim_clean,
                        'simulado': simulado_corrigido,
                        'datas_obs': datas_obs_clean,
                        'observado': valores_obs,
                        'previsoes': previsoes
                    }
                    dados_por_estacao.append(dados_estacao)

                except Exception as e:
                    print(f"  ERRO ao processar estação {codigo}: {e}")
                    import traceback
                    traceback.print_exc()
                    continue

            if not dados_por_estacao:
                QMessageBox.warning(self, "Erro", "Nenhuma estação pôde ser processada.")
                return

            print(f"\nTotal de estações processadas com sucesso: {len(dados_por_estacao)}")
            for i, estacao in enumerate(dados_por_estacao):
                print(f"  {i+1}: {estacao['codigo']} - datas_sim: {len(estacao['datas_sim'])}, datas_obs: {len(estacao['datas_obs'])}")

            CSVExporter.exportar_batelada(dados_por_estacao, arquivo_saida, data_inicio, data_fim)
            QMessageBox.information(self, "Sucesso", f"CSV em batelada salvo em:\n{arquivo_saida}")

        except Exception as e:
            import traceback
            traceback.print_exc()
            QMessageBox.critical(self, "Erro", f"Falha na exportação em batelada:\n{str(e)}")

    def on_processing_error(self, error_msg):
        self.btn_gerar.setEnabled(True)
        self.btn_gerar.setText("Gerar Gráfico")
        QMessageBox.critical(self, "Erro", f"Falha no processamento:\n{error_msg}")