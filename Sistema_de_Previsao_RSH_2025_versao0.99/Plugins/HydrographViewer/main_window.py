# main_window.py
import sys
from PyQt5.QtWidgets import QMainWindow, QApplication
from hydrograph_widget import HydrographWidget

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Hydrograph Viewer")
        self.setCentralWidget(HydrographWidget(self))
        self.resize(1350, 850)          # tamanho inicial razoável
        self.setMinimumSize(1350, 850)   # tamanho mínimo para evitar cortes

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec_())