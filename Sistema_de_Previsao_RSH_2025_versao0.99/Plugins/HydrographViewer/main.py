# -*- coding: utf-8 -*-
"""
Ponto de entrada principal para o aplicativo Hydrograph Viewer (standalone).
"""
import sys
import os
from PyQt5.QtWidgets import QApplication, QMainWindow, QMessageBox
from PyQt5.QtCore import Qt

# Adiciona o diretório atual ao path para importar os módulos do plugin
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from hydrograph_widget import HydrographWidget

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Hydrograph Viewer - Sistema de Previsão RSH")
        self.setGeometry(100, 100, 1200, 800)
        self.setWindowFlags(self.windowFlags() | Qt.WindowMaximizeButtonHint)

        # Cria o widget central
        self.hydrograph_widget = HydrographWidget(self)
        self.setCentralWidget(self.hydrograph_widget)

def main():
    app = QApplication(sys.argv)
    app.setApplicationName("Hydrograph Viewer")
    app.setOrganizationName("RSH")

    window = MainWindow()
    window.show()

    sys.exit(app.exec_())

if __name__ == '__main__':
    main()