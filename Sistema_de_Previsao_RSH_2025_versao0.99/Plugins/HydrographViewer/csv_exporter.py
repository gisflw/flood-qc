"""
Exportador de dados do hidrograma para CSV.
Agora com suporte a múltiplas previsões e exportação em batelada.
"""
import csv
import pandas as pd
import numpy as np
import logging
from datetime import datetime

logger = logging.getLogger(__name__)

class CSVExporter:
    """Gerencia a exportação de dados do hidrograma para CSV."""

    @staticmethod
    def exportar_hidrograma_completo(dados_base, previsoes, caminho_saida):
        """
        Exporta dados do hidrograma (base + múltiplas previsões) para CSV.

        Args:
            dados_base (dict): Dicionário com os dados da simulação base:
                - 'datas_atual': Lista de datetime (simulado atual)
                - 'cotas_atual': Lista de valores (simulado atual)
                - 'datas_obs': Lista de datetime (observados)
                - 'cotas_obs': Lista de valores (observados)
            previsoes (list): Lista de dicionários, cada um com:
                - 'legenda': str (nome da previsão)
                - 'datas': Lista de datetime
                - 'valores': Lista de valores
            caminho_saida (str): Caminho para salvar o CSV.

        Returns:
            str: Caminho do arquivo gerado.
        """
        try:
            logger.info(f"Iniciando exportação para CSV: {caminho_saida}")

            # Coletar todas as datas únicas de todas as séries
            todas_datas = set(dados_base.get('datas_atual', []))
            todas_datas.update(dados_base.get('datas_obs', []))
            for prev in previsoes:
                todas_datas.update(prev['datas'])

            if not todas_datas:
                raise ValueError("Nenhum dado para exportar.")

            datas_ordenadas = sorted(todas_datas)

            # Criar dicionários para acesso rápido
            dict_obs = dict(zip(dados_base.get('datas_obs', []), dados_base.get('cotas_obs', [])))
            dict_atual = dict(zip(dados_base.get('datas_atual', []), dados_base.get('cotas_atual', [])))

            # Preparar linhas
            linhas = []
            for data in datas_ordenadas:
                linha = {'Data_Hora': data}
                linha['Observado'] = dict_obs.get(data)
                linha['Simulado_Atual'] = dict_atual.get(data)

                # Adicionar cada previsão
                for prev in previsoes:
                    # Criar dicionário para busca rápida dentro da previsão
                    dict_prev = dict(zip(prev['datas'], prev['valores']))
                    linha[prev['legenda']] = dict_prev.get(data)

                linhas.append(linha)

            # Converter para DataFrame
            df = pd.DataFrame(linhas)

            # Formatar coluna de data
            df['Data_Hora'] = df['Data_Hora'].apply(lambda x: x.strftime('%d/%m/%Y %Hh'))

            # Formatar colunas numéricas com 2 casas decimais
            colunas_numericas = ['Observado', 'Simulado_Atual'] + [p['legenda'] for p in previsoes]
            for col in colunas_numericas:
                if col in df.columns:
                    df[col] = df[col].apply(lambda x: f"{x:.2f}" if pd.notnull(x) else '')

            # Reordenar colunas: Data, Observado, Simulado_Atual, depois as previsões na ordem
            colunas_finais = ['Data_Hora', 'Observado', 'Simulado_Atual'] + [p['legenda'] for p in previsoes]
            df = df[colunas_finais]

            # Salvar
            df.to_csv(caminho_saida, index=False, encoding='utf-8')
            logger.info(f"CSV exportado com sucesso: {caminho_saida}")
            return caminho_saida

        except Exception as e:
            logger.error(f"Erro na exportação CSV: {str(e)}")
            raise

    @staticmethod
    def gerar_preview_csv(dados_base, previsoes, max_linhas=20):
        """
        Gera uma preview dos dados em formato de texto para exibição.
        """
        try:
            # Coletar todas as datas
            todas_datas = set(dados_base.get('datas_atual', []))
            todas_datas.update(dados_base.get('datas_obs', []))
            for prev in previsoes:
                todas_datas.update(prev['datas'])

            if not todas_datas:
                return "Nenhum dado disponível."

            datas_ordenadas = sorted(todas_datas)[:max_linhas]

            # Criar dicionários
            dict_obs = dict(zip(dados_base.get('datas_obs', []), dados_base.get('cotas_obs', [])))
            dict_atual = dict(zip(dados_base.get('datas_atual', []), dados_base.get('cotas_atual', [])))

            linhas = []
            cabecalho = "Data/Hora               | Observado | Simulado_Atual"
            for prev in previsoes:
                cabecalho += f" | {prev['legenda']}"
            linhas.append(cabecalho)
            linhas.append("-" * len(cabecalho))

            for data in datas_ordenadas:
                linha = data.strftime('%d/%m/%Y %Hh')
                obs = dict_obs.get(data)
                linha += f" | {obs:.2f}" if obs is not None else " |       "
                atual = dict_atual.get(data)
                linha += f" | {atual:.2f}" if atual is not None else " |       "

                for prev in previsoes:
                    dict_prev = dict(zip(prev['datas'], prev['valores']))
                    val = dict_prev.get(data)
                    linha += f" | {val:.2f}" if val is not None else " |       "

                linhas.append(linha)

            return "\n".join(linhas)

        except Exception as e:
            logger.error(f"Erro ao gerar preview: {str(e)}")
            return "Erro ao gerar prévia dos dados."

    # ================================================================
    # NOVO MÉTODO: EXPORTAÇÃO EM BATELADA
    # ================================================================
    @staticmethod
    def exportar_batelada(dados_por_estacao, caminho_saida, data_inicio=None, data_fim=None):
        """
        Exporta dados de múltiplas estações em um único CSV.

        Args:
            dados_por_estacao (list): Lista de dicionários, cada um contendo:
                - 'codigo': str
                - 'nome': str
                - 'datas_sim': list (datas do simulado)
                - 'simulado': list (valores do simulado)
                - 'datas_obs': list (datas dos observados)
                - 'observado': list (valores dos observados)
                - 'previsoes': list (listas com 'legenda', 'datas', 'valores')
            caminho_saida (str): Caminho para salvar o CSV.
            data_inicio (datetime, opcional): Apenas para logging.
            data_fim (datetime, opcional): Apenas para logging.

        Returns:
            str: Caminho do arquivo gerado.
        """
        try:
            logger.info(f"Iniciando exportação em batelada para: {caminho_saida}")

            # Coletar todas as datas de todas as séries de todas as estações
            todas_datas = set()
            for estacao in dados_por_estacao:
                todas_datas.update(estacao['datas_sim'])
                todas_datas.update(estacao['datas_obs'])
                for prev in estacao['previsoes']:
                    for dt in prev['datas']:
                        todas_datas.add(pd.to_datetime(dt).replace(tzinfo=None))

            if not todas_datas:
                raise ValueError("Nenhuma data encontrada para exportação em batelada.")

            grade_datas = sorted(todas_datas)

            # Preparar estrutura por estação com dicionários de acesso rápido
            estacoes_dict = []
            for estacao in dados_por_estacao:
                dict_obs = {}
                if estacao['datas_obs'] and estacao['observado']:
                    for dt, val in zip(estacao['datas_obs'], estacao['observado']):
                        dt_clean = pd.to_datetime(dt).replace(tzinfo=None)
                        dict_obs[dt_clean] = val

                dict_sim = {}
                for dt, val in zip(estacao['datas_sim'], estacao['simulado']):
                    dt_clean = pd.to_datetime(dt).replace(tzinfo=None)
                    dict_sim[dt_clean] = val

                dict_prevs = []
                for prev in estacao['previsoes']:
                    d = {}
                    for dt, val in zip(prev['datas'], prev['valores']):
                        dt_clean = pd.to_datetime(dt).replace(tzinfo=None)
                        d[dt_clean] = val
                    dict_prevs.append(d)

                estacoes_dict.append({
                    'codigo': estacao['codigo'],
                    'nome': estacao['nome'],
                    'dict_obs': dict_obs,
                    'dict_sim': dict_sim,
                    'dict_prevs': dict_prevs,
                    'legendas': [p['legenda'] for p in estacao['previsoes']]
                })

            # Construir cabeçalhos
            header1 = []
            header2 = []
            for ed in estacoes_dict:
                header1.append(f"{ed['codigo']} - {ed['nome']}")
                num_prev = len(ed['dict_prevs'])
                for _ in range(num_prev + 2):
                    header1.append("")
                header1.append("")

                header2.append("Data_Hora")
                header2.append("Observado_m")
                header2.append("Simulado_Atual_m")
                for leg in ed['legendas']:
                    header2.append(leg)
                header2.append("")

            if header1 and header1[-1] == "":
                header1.pop()
                header2.pop()

            # Escrever CSV
            with open(caminho_saida, 'w', newline='', encoding='utf-8') as f:
                writer = csv.writer(f, delimiter=',')
                writer.writerow(header1)
                writer.writerow(header2)

                for dt in grade_datas:
                    row = []
                    for ed in estacoes_dict:
                        row.append(dt.strftime('%d/%m/%Y %Hh'))
                        val = ed['dict_obs'].get(dt)
                        row.append(f"{float(val):.2f}" if val is not None else "")
                        val = ed['dict_sim'].get(dt)
                        row.append(f"{float(val):.2f}" if val is not None else "")
                        for dprev in ed['dict_prevs']:
                            val = dprev.get(dt)
                            row.append(f"{float(val):.2f}" if val is not None else "")
                        row.append("")
                    if row and row[-1] == "":
                        row.pop()
                    writer.writerow(row)

            logger.info(f"CSV em batelada exportado com sucesso: {caminho_saida}")
            return caminho_saida

        except Exception as e:
            logger.error(f"Erro na exportação em batelada: {str(e)}")
            raise