# plotter.py
import matplotlib
matplotlib.use('Qt5Agg')
import matplotlib.pyplot as plt
from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg
import numpy as np
import pandas as pd

class WindPlotCanvas(FigureCanvasQTAgg):
    def __init__(self, parent=None):
        self.fig, (self.ax1, self.ax2) = plt.subplots(2, 1, figsize=(8, 6), sharex=True)
        super().__init__(self.fig)
        self.setParent(parent)
        self.fig.subplots_adjust(hspace=0.3)
        self.ax1.set_ylabel('Velocidade (m/s)')
        self.ax2.set_ylabel('Direção (graus)')
        self.ax2.set_xlabel('Data')
        self.ax1.grid(True, alpha=0.3)
        self.ax2.grid(True, alpha=0.3)
        self.fig.autofmt_xdate()

    def update_plot(self, df, estacao_codigo=""):
        if df is None or df.empty:
            return
        hoje = pd.Timestamp.now().normalize()
        inicio = hoje - pd.Timedelta(days=60)
        df_filtrado = df[df['data'] >= inicio].copy()
        if df_filtrado.empty:
            # Se não há dados nos últimos 60 dias, usa todos os dados disponíveis
            df_filtrado = df.copy()
            if df_filtrado.empty:
                return

        vel_original = df_filtrado['velocidade'].values
        dir_original = df_filtrado['direcao'].values
        vel_plot = np.where(vel_original == -1.0, np.nan, vel_original)
        dir_plot = np.where(dir_original == -1.0, np.nan, dir_original)

        self.ax1.clear()
        self.ax2.clear()

        # Velocidade
        self.ax1.plot(df_filtrado['data'], vel_plot, color='#1f77b4', linewidth=1.5, label='_nolegend_')
        mask_falha_vel = (vel_original == -1.0)
        if mask_falha_vel.any():
            self.ax1.plot(df_filtrado.loc[mask_falha_vel, 'data'],
                          np.zeros(mask_falha_vel.sum()),
                          color='#555555', marker='x', markersize=4, linestyle='None', label='_nolegend_')

        # Direção
        self.ax2.plot(df_filtrado['data'], dir_plot, color='#ff7f0e', linewidth=1.5, label='_nolegend_')
        mask_falha_dir = (dir_original == -1.0)
        if mask_falha_dir.any():
            self.ax2.plot(df_filtrado.loc[mask_falha_dir, 'data'],
                          np.zeros(mask_falha_dir.sum()),
                          color='#555555', marker='x', markersize=4, linestyle='None', label='_nolegend_')

        self.ax1.set_ylabel('Velocidade (m/s)')
        self.ax2.set_ylabel('Direção (graus)')
        self.ax2.set_xlabel('Data')
        self.ax1.grid(True, alpha=0.3)
        self.ax2.grid(True, alpha=0.3)

        self.ax2.set_yticks(np.arange(0, 361, 45))
        self.ax2.set_yticklabels([f'{i}°' for i in np.arange(0, 361, 45)])

        # Título dinâmico
        data_atual = hoje.strftime('%d/%m/%Y')
        dias = (df_filtrado['data'].max() - df_filtrado['data'].min()).days
        if dias < 60:
            titulo = f'Vento – Estação {estacao_codigo} – Últimos registros (até {data_atual})'
        else:
            titulo = f'Vento – Estação {estacao_codigo} – Últimos 60 dias (até {data_atual})'
        self.ax1.set_title(titulo)

        self.fig.autofmt_xdate()
        self.draw()