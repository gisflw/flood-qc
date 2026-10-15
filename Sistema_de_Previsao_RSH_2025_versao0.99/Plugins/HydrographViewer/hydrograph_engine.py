"""
Motor principal para geração de hidrogramas.
Versão com suporte a viéses individuais e fornecimento de dados brutos.
Agora focado apenas em processamento de dados, sem lógica de plotagem.
Otimizações: leitura única de dados observados (2.2) e fatiamento com bisect (2.3).
"""
import os
import logging
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from bisect import bisect_left, bisect_right  # ---- OTIMIZAÇÃO 2.3 ----

logger = logging.getLogger(__name__)

class HydrographEngine:
    def __init__(self):
        self.dados_processados = None
        self.figura = None
        self.logger = logger
        self.ultimo_bias = 0.0
        self.minibacia_index = None
        self.ultimo_parametros = None
        self.dados_brutos_internos = None

    # ================================================================
    # MÉTODO PRINCIPAL (com suporte a viéses individuais)
    # ================================================================
    def gerar_hidrograma(self, parametros, viesses=None):
        """
        Processa os dados e retorna os dados plotáveis e brutos.

        Parâmetros:
        - viesses: dicionário com chaves:
            'simulado': float (viés para série simulada)
            'base': float (viés para previsão base)
            'overlay': list (viéses para sobreposições)
        """
        self.logger.info("Processando dados...")
        self.dados_processados = self._processar_dados(parametros, viesses)
        self.ultimo_parametros = parametros
        dados_export = self._preparar_dados_exportacao()
        dados_brutos = self._obter_dados_brutos()
        return {
            'dados_plot': self.dados_processados,
            'dados_export': dados_export,
            'dados_brutos': dados_brutos
        }

    def processar_previsao(self, parametros):
        """
        Processa uma previsão adicional (sobreposição) usando o bias geral.
        Mantido para compatibilidade com o código existente.
        """
        try:
            self.logger.info("Processando previsão para sobreposição...")
            if self.minibacia_index is None:
                raise ValueError("É necessário gerar um gráfico base primeiro.")
            (NT_atual, NT_prev, nMini,
             start_date_MGB, start_dataprev_MGB) = self._ler_metadados_mgb(
                parametros['parhig_file'],
                parametros['infoprevi_file']
            )
            dados_prev = self._carregar_dados_mgb(
                parametros['qtudo_prev_file'], NT_prev, nMini, self.minibacia_index
            )
            datas_prev = self._criar_serie_temporal(start_dataprev_MGB, NT_prev)
            dados_prev = dados_prev + self.ultimo_bias
            datas_filt, valores_filt = self._fatiar_serie(
                datas_prev, dados_prev,
                parametros['data_inicio'], parametros['data_fim']
            )
            return {'datas': datas_filt, 'valores': valores_filt}
        except Exception as e:
            self.logger.error(f"Erro ao processar previsão: {e}")
            raise

    # ================================================================
    # VALIDAÇÃO DE PARÂMETROS
    # ================================================================
    def _validar_parametros(self, parametros):
        obrigatorios = [
            'csv_file', 'planilha_defesa_civil', 'codigo_estacao',
            'data_inicio', 'data_fim', 'qtudo_atual_file',
            'qtudo_prev_file', 'parhig_file', 'infoprevi_file'
        ]
        for key in obrigatorios:
            if key not in parametros:
                raise ValueError(f"Parâmetro obrigatório ausente: {key}")

    # ================================================================
    # PROCESSAMENTO PRINCIPAL (com aplicação de viéses individuais)
    # ================================================================
    def _processar_dados(self, parametros, viesses=None):
        self.logger.info("Processando dados...")

        # 1. Informações da estação
        info_estacao = self._carregar_informacoes_estacao(
            parametros['planilha_defesa_civil'],
            parametros['codigo_estacao']
        )
        if not info_estacao:
            raise ValueError(f"Estação {parametros['codigo_estacao']} não encontrada")

        # 2. Mapeamento para minibacia
        minibacia_id, minibacia_index = self._mapear_estacao(
            parametros['csv_file'],
            parametros['codigo_estacao']
        )
        if minibacia_index is None:
            raise ValueError(f"Estação {parametros['codigo_estacao']} não mapeada para minibacia")
        self.minibacia_index = minibacia_index

        # 3. Metadados MGB
        (NT_atual, NT_prev, nMini,
         start_date_MGB, start_dataprev_MGB) = self._ler_metadados_mgb(
            parametros['parhig_file'],
            parametros['infoprevi_file']
        )

        # 4. Séries temporais
        dates_MGB_atual = self._criar_serie_temporal(start_date_MGB, NT_atual)
        dates_MGB_prev = self._criar_serie_temporal(start_dataprev_MGB, NT_prev)

        # 5. Dados brutos (sem correção)
        Q_atual_completo = self._carregar_dados_mgb(
            parametros['qtudo_atual_file'], NT_atual, nMini, minibacia_index
        )
        Q_prev_completo = self._carregar_dados_mgb(
            parametros['qtudo_prev_file'], NT_prev, nMini, minibacia_index
        )

        # ---- OTIMIZAÇÃO 2.2: Leitura única dos dados observados ----
        datas_obs_completo = []
        valores_obs_completo = []
        bias_calc = 0.0
        if parametros.get('pasta_dados_observados'):
            caminho_obs = os.path.join(
                parametros['pasta_dados_observados'],
                f"N{parametros['codigo_estacao']}.txt"
            )
            datas_obs_completo, valores_obs_completo = self._ler_serie_observada_com_datas(caminho_obs)
            if valores_obs_completo and len(valores_obs_completo) > 0:
                # Calcular viés usando os últimos 3 pontos (ou menos)
                n_ultimos = min(3, len(valores_obs_completo))
                ultimos_obs = np.array(valores_obs_completo[-n_ultimos:])
                ultimas_datas_obs = datas_obs_completo[-n_ultimos:]
                # Interpolar valores simulados para essas datas
                sim_interpolados = []
                for dt in ultimas_datas_obs:
                    idx = np.searchsorted(dates_MGB_atual, dt)
                    if idx == 0:
                        sim_val = Q_atual_completo[0]
                    elif idx >= len(dates_MGB_atual):
                        sim_val = Q_atual_completo[-1]
                    else:
                        dt0 = dates_MGB_atual[idx-1]
                        dt1 = dates_MGB_atual[idx]
                        val0 = Q_atual_completo[idx-1]
                        val1 = Q_atual_completo[idx]
                        if dt1 > dt0:
                            frac = (dt - dt0) / (dt1 - dt0)
                            sim_val = val0 + frac * (val1 - val0)
                        else:
                            sim_val = val0
                    sim_interpolados.append(sim_val)
                sim_interpolados = np.array(sim_interpolados)
                media_obs = np.mean(ultimos_obs)
                media_sim = np.mean(sim_interpolados)
                bias_calc = media_obs - media_sim
                logger.info(f"Viés calculado usando últimos {n_ultimos} pontos: {bias_calc:.3f} m (Observado - Simulado)")
            else:
                logger.warning("Nenhum dado observado válido. Viés zero aplicado.")
        self.ultimo_bias = bias_calc
        # --------------------------------------------

        # 6. Aplicar viéses individuais (se fornecidos) ou viés geral
        if viesses is not None:
            vies_sim = viesses.get('simulado', 0.0)
            Q_atual_corrigido = Q_atual_completo + vies_sim
            vies_base = viesses.get('base', 0.0)
            Q_prev_corrigido = Q_prev_completo + vies_base
            self.ultimo_viesses = viesses
        else:
            if parametros.get('pasta_dados_observados'):
                if 'bias_manual' in parametros:
                    bias_vies = parametros['bias_manual']
                    Q_atual_corrigido = Q_atual_completo + bias_vies
                    Q_prev_corrigido = Q_prev_completo + bias_vies
                else:
                    # Usa o bias_calc já calculado (se houver dados observados)
                    Q_atual_corrigido = Q_atual_completo + bias_calc
                    Q_prev_corrigido = Q_prev_completo + bias_calc
            else:
                Q_atual_corrigido = Q_atual_completo
                Q_prev_corrigido = Q_prev_completo

        # 7. Filtrar pelo intervalo de datas (usando bisect - OTIMIZAÇÃO 2.3)
        dates_atual_filtrado, Q_atual_filtrado = self._fatiar_serie(
            dates_MGB_atual, Q_atual_corrigido,
            parametros['data_inicio'], parametros['data_fim']
        )
        dates_prev_filtrado, Q_prev_filtrado = self._fatiar_serie(
            dates_MGB_prev, Q_prev_corrigido,
            parametros['data_inicio'], parametros['data_fim']
        )

        # 8. Dados observados (já filtrados) - usando a lista lida uma única vez
        datas_obs_filtrado = []
        valores_obs_filtrado = []
        if parametros.get('pasta_dados_observados') and datas_obs_completo:
            for i, dt in enumerate(datas_obs_completo):
                if parametros['data_inicio'] <= dt <= parametros['data_fim']:
                    datas_obs_filtrado.append(dt)
                    valores_obs_filtrado.append(valores_obs_completo[i])

        # 9. Montar dicionário de dados processados
        dados_processados = {
            'info_estacao': info_estacao,
            'minibacia_id': minibacia_id,
            'minibacia_index': minibacia_index,
            'dates_atual_filtrado': dates_atual_filtrado,
            'Q_atual_filtrado': Q_atual_filtrado,
            'dates_prev_filtrado': dates_prev_filtrado,
            'Q_prev_filtrado': Q_prev_filtrado,
            'datas_obs_filtrado': datas_obs_filtrado,
            'valores_obs_filtrado': valores_obs_filtrado,
            'bias_vies': bias_calc,
            'cotas_alerta': info_estacao['cotas'] if info_estacao else {}
        }

        # 10. Armazenar dados brutos (antes da correção) para uso externo
        self.dados_brutos_internos = {
            'Q_atual_bruto': Q_atual_completo,
            'Q_prev_bruto': Q_prev_completo,
            'dates_atual_bruto': dates_MGB_atual,
            'dates_prev_bruto': dates_MGB_prev,
            'bias_calculado': bias_calc
        }

        return dados_processados

    # ================================================================
    # OBTENÇÃO DE DADOS BRUTOS
    # ================================================================
    def _obter_dados_brutos(self):
        return self.dados_brutos_internos

    # ================================================================
    # CÁLCULO DO VIÉS (agora integrado no _processar_dados, mantido para compatibilidade)
    # ================================================================
    def _calcular_e_aplicar_correcao(self, serie_simulada, datas_simuladas, codigo_estacao, pasta_dados):
        """
        Mantido para compatibilidade com possíveis chamadas externas.
        O cálculo principal agora é feito em _processar_dados.
        """
        caminho_observado = os.path.join(pasta_dados, f"N{codigo_estacao}.txt")
        datas_obs, valores_obs = self._ler_serie_observada_com_datas(caminho_observado)

        if not valores_obs or len(valores_obs) == 0:
            bias_vies = 0.0
            logger.warning("Nenhum dado observado válido. Viés zero aplicado.")
            return serie_simulada + bias_vies, bias_vies, "Zero"

        n_ultimos = min(3, len(valores_obs))
        ultimos_obs = np.array(valores_obs[-n_ultimos:])
        ultimas_datas_obs = datas_obs[-n_ultimos:]

        sim_interpolados = []
        for dt in ultimas_datas_obs:
            idx = np.searchsorted(datas_simuladas, dt)
            if idx == 0:
                sim_val = serie_simulada[0]
            elif idx >= len(datas_simuladas):
                sim_val = serie_simulada[-1]
            else:
                dt0 = datas_simuladas[idx-1]
                dt1 = datas_simuladas[idx]
                val0 = serie_simulada[idx-1]
                val1 = serie_simulada[idx]
                if dt1 > dt0:
                    frac = (dt - dt0) / (dt1 - dt0)
                    sim_val = val0 + frac * (val1 - val0)
                else:
                    sim_val = val0
            sim_interpolados.append(sim_val)

        sim_interpolados = np.array(sim_interpolados)
        media_obs = np.mean(ultimos_obs)
        media_sim = np.mean(sim_interpolados)
        bias_vies = media_obs - media_sim

        logger.info(f"Viés calculado usando últimos {n_ultimos} pontos: {bias_vies:.3f} m (Observado - Simulado)")
        return serie_simulada + bias_vies, bias_vies, "Observado"

    # ================================================================
    # MÉTODOS AUXILIARES DE LEITURA DE DADOS
    # ================================================================
    @staticmethod
    def _criar_serie_temporal(data_inicio, num_pontos):
        return [data_inicio + timedelta(hours=i) for i in range(num_pontos)]

    @staticmethod
    def _fatiar_serie(dates_completo, serie_completa, data_inicio, data_fim):
        """
        Fatiamento otimizado com bisect (OTIMIZAÇÃO 2.3).
        Assume que dates_completo é uma lista ordenada cronologicamente.
        """
        if not dates_completo:
            return [], []
        start = bisect_left(dates_completo, data_inicio)
        end = bisect_right(dates_completo, data_fim)
        if start >= len(dates_completo) or end <= start:
            return [], []
        return dates_completo[start:end], serie_completa[start:end]

    @staticmethod
    def _ler_serie_observada(caminho_arquivo, intervalo_datas=None):
        """
        Lê dados observados e retorna (datas, valores).
        Se intervalo_datas for fornecido, filtra os dados dentro do intervalo.
        """
        datas, valores = [], []
        try:
            with open(caminho_arquivo, 'r', encoding='latin-1') as file:
                for linha in file:
                    linha = linha.strip()
                    if not linha:
                        continue
                    partes = linha.split()
                    if len(partes) < 5:
                        continue
                    try:
                        dia, mes, ano, hora = map(int, partes[:4])
                        valor = float(partes[-1])
                        if valor in [-99999.000000, -1] or valor < 0:
                            continue
                        valor_metros = valor / 100.0
                        data_obs = datetime(ano, mes, dia, hora)
                        if intervalo_datas:
                            data_inicio, data_fim = intervalo_datas
                            if data_inicio <= data_obs <= data_fim:
                                datas.append(data_obs)
                                valores.append(valor_metros)
                        else:
                            datas.append(data_obs)
                            valores.append(valor_metros)
                    except (ValueError, IndexError):
                        continue
        except FileNotFoundError:
            return [], []
        except Exception:
            return [], []

        if datas:
            dados_ordenados = sorted(zip(datas, valores))
            datas_ordenadas, valores_ordenados = zip(*dados_ordenados)
            return list(datas_ordenadas), list(valores_ordenados)
        return [], []

    def _ler_serie_observada_com_datas(self, caminho_arquivo):
        """Retorna (datas, valores) completos, sem filtro de datas."""
        return self._ler_serie_observada(caminho_arquivo, intervalo_datas=None)

    # ================================================================
    # INFORMAÇÕES DA ESTAÇÃO E MAPEAMENTO
    # ================================================================
    @staticmethod
    def _carregar_informacoes_estacao(planilha_path, codigo_estacao):
        try:
            df = pd.read_excel(planilha_path)
            coluna_codigo = None
            for nome in ['Codigo', 'CODIGO', 'Código', 'codigo']:
                if nome in df.columns:
                    coluna_codigo = nome
                    break
            if coluna_codigo is None:
                return None
            df['Codigo_str'] = df[coluna_codigo].astype(str).str.strip()
            linha_estacao = df[df['Codigo_str'] == str(codigo_estacao).strip()]
            if linha_estacao.empty:
                return None
            cotas_alerta = {}
            mapeamento = {
                'Cota Atenção': 'ATENCAO',
                'Cota Alerta': 'ALERTA',
                'Cota Inundação': 'INUNDACAO',
                'Cota Máxima': 'maior_INUNDACAO'
            }
            for col_plan, chave in mapeamento.items():
                if col_plan in df.columns:
                    valor = linha_estacao[col_plan].iloc[0]
                    if pd.notna(valor):
                        try:
                            cotas_alerta[chave] = float(valor) / 100
                        except:
                            pass
            return {
                'codigo': codigo_estacao,
                'nome': linha_estacao['Nome'].iloc[0],
                'cotas': cotas_alerta
            }
        except:
            return None

    @staticmethod
    def _mapear_estacao(csv_file, codigo_estacao):
        try:
            df_map = pd.read_csv(csv_file, sep=';', encoding='latin-1')
            linha_estacao = df_map[df_map['CODIGO'].astype(str) == codigo_estacao]
            if linha_estacao.empty:
                return None, None
            col_mini = None
            for col in df_map.columns:
                if col.upper() == 'MINI':
                    col_mini = col
                    break
            if col_mini is None:
                if 'Mini' in df_map.columns:
                    col_mini = 'Mini'
                else:
                    return None, None
            minibacia_id = int(linha_estacao[col_mini].iloc[0])
            return minibacia_id, minibacia_id - 1
        except Exception as e:
            logger.error(f"Erro no mapeamento: {e}")
            return None, None

    # ================================================================
    # LEITURA DE METADADOS MGB
    # ================================================================
    @staticmethod
    def _ler_metadados_mgb(parhig_file, infoprevi_file):
        try:
            with open(parhig_file, 'r', encoding='latin-1') as file:
                lines = file.readlines()
                NT_total = None
                nMini = None
                start_date_MGB = None
                for i in range(min(15, len(lines))):
                    line = lines[i].strip()
                    if "NT" in line and i+1 < len(lines):
                        NT_total = int(lines[i+1].split()[0])
                    elif "NC" in line and i+1 < len(lines):
                        nMini = int(lines[i+1].split()[0])
                    elif "DIA" in line and i+1 < len(lines):
                        dia, mes, ano, hora = map(int, lines[i+1].split())
                        start_date_MGB = datetime(ano, mes, dia, hora)
            with open(infoprevi_file, 'r') as file:
                lines = file.readlines()
                NT_prev = None
                start_dataprev_MGB = None
                for i in range(min(10, len(lines))):
                    if "!ANO" in lines[i].strip() and i+1 < len(lines):
                        partes = lines[i+1].split()
                        if len(partes) >= 5:
                            ano_prev, mes_prev, dia_prev, hora_prev, NT_prev = map(int, partes[:5])
                            start_dataprev_MGB = datetime(ano_prev, mes_prev, dia_prev, hora_prev)
                            break
            if None in [NT_total, nMini, start_date_MGB, NT_prev, start_dataprev_MGB]:
                raise ValueError("Metadados incompletos")
            NT_atual = NT_total - NT_prev
            return NT_atual, NT_prev, nMini, start_date_MGB, start_dataprev_MGB
        except Exception as e:
            raise

    @staticmethod
    def _carregar_dados_mgb(caminho_arquivo, NT, nMini, minibacia_index):
        try:
            data = np.fromfile(caminho_arquivo, '<f4').reshape(NT, nMini)
            return data[:, minibacia_index].ravel()
        except Exception as e:
            raise

    # ================================================================
    # PREPARAÇÃO PARA EXPORTAÇÃO
    # ================================================================
    def _preparar_dados_exportacao(self):
        dados = self.dados_processados
        return {
            'datas_atual': dados['dates_atual_filtrado'],
            'cotas_atual': dados['Q_atual_filtrado'],
            'datas_prev': dados['dates_prev_filtrado'],
            'cotas_prev': dados['Q_prev_filtrado'],
            'datas_obs': dados['datas_obs_filtrado'],
            'cotas_obs': dados['valores_obs_filtrado'],
            'info_estacao': dados['info_estacao'],
            'bias_vies': dados['bias_vies']
        }