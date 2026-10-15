# downloader.py
import requests
import datetime
import time
import pandas as pd
import numpy as np
from pydantic import HttpUrl

class BndmetAPI:
    url_base: HttpUrl = "https://api-bndmet.decea.mil.br/v1"
    x_api_key: str = '<REDACTED>'

    @classmethod
    def _request(cls, url, params=None, timeout=30, max_retries=5):
        session = requests.Session()
        session.headers = {'accept': 'application/json', 'x-api-key': cls.x_api_key}
        for attempt in range(max_retries):
            try:
                response = session.get(url, params=params, timeout=timeout)
                response.raise_for_status()
                return response.json()
            except requests.Timeout:
                print(f"Timeout tentativa {attempt+1}/{max_retries}")
                time.sleep(5)
            except Exception as e:
                print(f"Erro: {e} tentativa {attempt+1}/{max_retries}")
                time.sleep(5)
        return None

    @classmethod
    def get_wind_data(cls, estacao_id, start_date: datetime.date, end_date: datetime.date):
        produtos = {'velocidade': 'I111', 'direcao': 'I113'}
        result = {}
        for nome, cod in produtos.items():
            url = f"{cls.url_base}/estacoes/{estacao_id}/fenomenos/{cod}"
            params = {'dataInicio': start_date.isoformat(), 'dataFinal': end_date.isoformat()}
            json_data = cls._request(url, params)
            if json_data:
                data_list = json_data.get('data', {}).get('data', [])
                result[nome] = data_list
            else:
                result[nome] = []
        return result

    @classmethod
    def baixar_completo(cls, estacao_id, start_date, end_date, log_callback=None):
        if log_callback:
            log_callback(f"Baixando {estacao_id}...")
        raw = cls.get_wind_data(estacao_id, start_date, end_date)
        
        vel_dict = {}
        dir_dict = {}
        for ts, val in raw.get('velocidade', []):
            vel_dict[ts] = val if val is not None else -1.0
        for ts, val in raw.get('direcao', []):
            dir_dict[ts] = val if val is not None else -1.0
        
        all_ts = sorted(set(vel_dict.keys()) | set(dir_dict.keys()))
        if not all_ts:
            return pd.DataFrame(columns=['data', 'velocidade', 'direcao'])
        
        dados = []
        for ts in all_ts:
            dt = datetime.datetime.fromtimestamp(ts / 1000)
            dados.append({
                'data': dt,
                'velocidade': vel_dict.get(ts, -1.0),
                'direcao': dir_dict.get(ts, -1.0)
            })
        df = pd.DataFrame(dados)
        df = df.sort_values('data').reset_index(drop=True)
        if log_callback:
            log_callback(f"  {estacao_id}: {len(df)} registros.")
        return df