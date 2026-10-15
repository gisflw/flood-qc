import os
import requests
import datetime
import time
import pandas as pd
import pytz
from tkinter import Tk, Label, Entry, Button, filedialog, messagebox
from tkinter import ttk
from pydantic import DirectoryPath, HttpUrl
from pydantic_settings import BaseSettings
from decouple import config as env
from tkinter import PhotoImage
from PIL import Image, ImageTk

class Settings(BaseSettings):
    data_path: DirectoryPath = os.path.dirname(os.path.abspath(__file__))

settings = Settings()

class Request:
    @classmethod
    def get(cls, url: HttpUrl, params: dict = {}, timeout: int = 1, session: requests.Session = None):
        if not session:
            session = requests.Session()
        response = session.get(url=url, params=params, timeout=timeout)
        return response

products = [
    ('I175', 'chuva'),
]

class BndmetAPI:
    url_base: HttpUrl = "https://api-bndmet.decea.mil.br/v1"
    x_api_key: str = '<REDACTED>'
    # x_api_key: str = '<REDACTED>'
    
    @classmethod
    def get_data_by_station(cls, estacao_id, id_products, start_date: datetime.date, end_date: datetime.date):
        url = f"{cls.url_base}/estacoes/{estacao_id}/fenomenos/{id_products}"
        params = {'dataInicio': start_date.isoformat(), 'dataFinal': end_date.isoformat()}

        session = requests.Session()
        session.headers = {'accept': 'application/json', 'x-api-key': cls.x_api_key}

        retry_attempts = 5
        for attempt in range(retry_attempts):
            try:
                response = Request.get(url=url, params=params, session=session)
                response.raise_for_status()
                json_data = response.json()
                # print(json_data)
                data = json_data.get('data', {}).get('data', [])
                
                # Convertendo o valor de chuva para float
                for item in data:
                    if isinstance(item, dict):
                        value = item.get('value')
                        if value is not None:
                            item['value'] = float(value)
                        else:
                            item['value'] = None

                return data
            except requests.Timeout:
                print(f'*** Timeout. Station: {estacao_id}:{attempt + 1}-{id_products}')
                time.sleep(5)
            except requests.RequestException as e:
                print(f'*** Error. Station: {estacao_id}:{attempt + 1}-{id_products} - {e}')
                time.sleep(5)
        
        print(f'*** Failed to fetch data for Station {estacao_id}:{id_products} after {retry_attempts} attempts.')
        return []

def stations_data(estacao_id, start_date: datetime.datetime, end_date: datetime.datetime):
    result = {}
    local_tz = pytz.timezone('America/Sao_Paulo')
    for p in products:
        data = BndmetAPI.get_data_by_station(estacao_id, p[0], start_date, end_date)
        for d in data:
            if not d[0] in result:
                timestamp_ms = d[0]
                timestamp_s = timestamp_ms / 1000
                dataHora = datetime.datetime.fromtimestamp(timestamp_s)
                dataHora = dataHora.astimezone(local_tz)
                dataHora_formatada = dataHora.strftime('%d/%m/%Y %H:%M')
                result[d[0]] = {'codigo': estacao_id, 'dataHora': dataHora_formatada}
            result[d[0]].update({p[1]: d[1]})
    return result

def define_cols(data: list):
    result = []
    for i in data:
        date = i['dataHora']
        chuva_value = i.get('chuva', -1.0)
        chuva_value = -1.0 if chuva_value is None else float(chuva_value)
        row = {
            'data': date,
            'chuva': chuva_value,
        }
        result.append(row)
    return result

def load_station_codes(file_path):
    with open(file_path, 'r') as file:
        codes = [line.strip() for line in file.readlines()]
    return codes

def run_script(file_path, output_dir, start_date, end_date):
    try:
        os.environ['DATA_PATH'] = output_dir
        settings.data_path = output_dir
        
        station_codes = load_station_codes(file_path)
        
        for station_code in station_codes:
            result = stations_data(station_code, start_date, end_date)
            
            if type(result) is not list:
                result = [v for _, v in result.items()]
            
            result = define_cols(result)
            df = pd.DataFrame(result)
            filename = f"{output_dir}\P{station_code}.txt"
            local_tz = pytz.timezone('America/Sao_Paulo')
            now_floor = datetime.datetime.now(local_tz).replace(minute=0, second=0, microsecond=0)

            with open(filename, 'w') as file:
                file.write(f'Dia   Mês   Ano  Hora{station_code}\n')
                # df['data'] = pd.to_datetime(df['data'])
    
                # Itera sobre as linhas do DataFrame e escreva os dados no formato especificado
                for index, row in df.iterrows():
                    # corta horas futuras quando a data final for hoje
                    if end_date == now_floor.date():
                        dt_row = datetime.datetime.strptime(row['data'], '%d/%m/%Y %H:%M')
                        dt_row = local_tz.localize(dt_row)
                        if dt_row > now_floor:
                            continue
                                       
                    dia = int(row['data'].split("/")[0])
                    dia=f"{dia:02}"
                    
                    mes = int(row['data'].split("/")[1])
                    mes=f"{mes:02}"
            
                    ano =  int(row['data'].split("/")[2].split(" ")[0])
                    ano=f"{ano:04}"
            
                    hora = int(row['data'].split(" ")[-1].split(":")[0])
                    hora=f"{hora:02}"
                    
                    chuva = row['chuva']
            
                    if chuva == -1:
                        chuva = '-1.00' +'\n'
                    v = float(chuva)
                    out = f"{v:<7.6f}"
                    
                    # Salva no formato MGB
                    linha = '{}{}{}{}{}\n'.format(dia.rjust(6), mes.rjust(6),ano.rjust(6), hora.rjust(6), out.rjust(16))
                    file.write(linha)
            # df.to_csv(f"{output_dir}/{station_code}_inmet_data.csv", index=False)
        
        messagebox.showinfo("Success", "Data saved successfully!")
    except Exception as e:
        messagebox.showerror("Error", str(e))

def create_gui():
    root = Tk()
    root.title("Baixador de dados INMET")

    def select_file():
        file_path = filedialog.askopenfilename(filetypes=[("Text files", "*.txt")])
        if file_path:
            file_entry.delete(0, 'end')
            file_entry.insert(0, file_path)

    def select_output_dir():
        output_dir = filedialog.askdirectory()
        if output_dir:
            output_entry.delete(0, 'end')
            output_entry.insert(0, output_dir)
    
    # # # Get the directory where the script is located
    script_dir = os.path.dirname(os.path.abspath(__file__))
    
    # #    # Load image from the script directory and resize it
    # img_path = os.path.join(script_dir, "hge_logo.png")
    # img = Image.open(img_path)
    # img = img.resize((940, 200), Image.ANTIALIAS)  # Resize the image
    # img = ImageTk.PhotoImage(img)

    # img_label = Label(root, image=img)
    # img_label.grid(row=0, column=0, columnspan=3, pady=10)

    # Label for development info
    dev_info_label = Label(root, text="Aplicativo desenvolvido para as previsões operacionais - IPH 2026")
    dev_info_label.grid(row=1, column=0, columnspan=3, pady=10)

    # Entrada para o arquivo de códigos das estações
    Label(root, text="Arquivo de Códigos das Estações").grid(row=2, column=0, padx=10, pady=10)
    file_entry = Entry(root, width=50)
    file_entry.grid(row=2, column=1, padx=10, pady=10)
    Button(root, text="Selecionar Arquivo", command=select_file).grid(row=2, column=2, padx=10, pady=10)

    # Entrada para o diretório de saída
    Label(root, text="Diretório de Saída").grid(row=3, column=0, padx=10, pady=10)
    output_entry = Entry(root, width=50)
    output_entry.grid(row=3, column=1, padx=10, pady=10)
    Button(root, text="Selecionar Diretório", command=select_output_dir).grid(row=3, column=2, padx=10, pady=10)

    # Entrada para a data de início
    Label(root, text="Data de Início (dd/mm/yyyy)").grid(row=4, column=0, padx=10, pady=10)
    start_date_entry = Entry(root)
    start_date_entry.grid(row=4, column=1, padx=10, pady=10)

    # Entrada para a data de fim
    Label(root, text="Data de Fim (dd/mm/yyyy)").grid(row=5, column=0, padx=10, pady=10)
    end_date_entry = Entry(root)
    end_date_entry.grid(row=5, column=1, padx=10, pady=10)
    
    # Preencher automaticamente a data de fim com a data atual
    end_date_entry.insert(0, datetime.datetime.now().strftime('%d/%m/%Y'))

    # Botão para executar o script
    Button(root, text="Executar", command=lambda: run_script(
        file_entry.get(),
        output_entry.get(),
        datetime.datetime.strptime(start_date_entry.get(), '%d/%m/%Y').date(),
        datetime.datetime.strptime(end_date_entry.get(), '%d/%m/%Y').date()
    )).grid(row=6, column=0, columnspan=3, pady=20)

    root.mainloop()


if __name__ == "__main__":
    create_gui()
