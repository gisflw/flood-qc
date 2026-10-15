# converter.py
import os
import numpy as np

def salvar_arquivo_mgb(df, output_dir, tipo, prefixo=""):
    if tipo == 'vel':
        nome_arquivo = f"{prefixo}vento_vel.txt"
        coluna_valor = 'velocidade'
    else:
        nome_arquivo = f"{prefixo}vento_dir.txt"
        coluna_valor = 'direcao'
    
    caminho = os.path.join(output_dir, nome_arquivo)
    with open(caminho, 'w', encoding='utf-8') as f:
        f.write("Dia   Mês   Ano  Hora            Valor\n")
        for _, row in df.iterrows():
            dt = row['data']
            valor = row[coluna_valor]
            if np.isnan(valor) or valor is None:
                valor = -1.0
            f.write(f"{dt.day:02d}{'':>4}{dt.month:02d}{'':>4}{dt.year:04d}{'':>2}{dt.hour:02d}{'':>4}{valor:16.6f}\n")
    return caminho