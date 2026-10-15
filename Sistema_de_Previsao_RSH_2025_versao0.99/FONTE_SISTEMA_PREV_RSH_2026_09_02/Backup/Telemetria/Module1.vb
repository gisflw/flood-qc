Imports System.Windows.Forms
Imports System.Drawing

Module Module1

    'Variáveis de dowload automatizado
    Public estreito As Boolean
    Public RHAMA_Plu As Boolean
    Public RHAMA_Flu As Boolean
    Public CPTEC As Boolean
    Public CPTEC_SISMADEM As Boolean
    Public Lajeado As Boolean
    Public UHE_Estreito As Boolean
    Public UHE_SerraDaMesa As Boolean
    Public UHE_CanaBrava As Boolean
    Public GEFS_ONS As Boolean
    Public ETA_ONS As Boolean
    '--

    'Variáveis de diretório dos arquivos
    Public DIR_RHAMA_Q As String
    Public DIR_RHAMA_P As String
    Public DIR_CPTEC As String
    Public DIR_LAJEADO As String
    Public DIR_ESTREITO As String
    Public DIR_SERRADAMESA As String
    Public DIR_CANABRAVA As String
    '--

    'Variáveis de vazões observadas
    Public total_estações As Integer
    Public ID() As Integer
    Public Estação() As String
    Public LAT(), LON(), ALT() As Double
    Public NOME() As String
    Public A(), B(), C() As Double
    Public Q(,,,,) As Single 'Double
    Public Niv(,,,,) As Single 'Double

    Public AssimilaQ() As Integer
    Public AssimilaP() As Integer

    Public dataIp() As Date
    Public dataFp() As Date
    Public HoraIp() As Integer
    Public HoraFp() As Integer
    Public dataI As Date
    Public dataF As Date
    Public HoraI As Integer
    Public HoraF As Integer
    '--

    'Variáveis de vazões substituídas
    Public Qsub(,,,,) As Single 'Double
    Public Q_inic() As Single 'Double
    Public Q_hori() As Single 'Double
    Public Q_ext(,,,,) As Single 'Double
    Public Q_opera(,,,,) As Single
    '--

    'Variáveis de chuva
    Public Chuva(,,,,) As Single 'Double
    '--

    'Variáveis dos graficos no zedgrah de vazões observadas
    Public Estação_GrafQ As Integer
    Public Hidrograma, Cotagrama, Pluviograma As Boolean
    '--

    'Variáveis do Gráfico GANTT - FORM6
    'VARIAVEIS DE GRAFICO
    Public graficoArea, graficoArea2 As Graphics
    Public bmpArea, bmparea2 As Bitmap
    Public Esp As Integer 'espacamento
    Public Larg_Rec As Single 'largura do retangulo

    Public precmax As Single 'prec max
    Public h, yloc, xloc As Single 'altura e localizacao do rectangulo

    Public Qdispo(,,,,) As Single 'Double

    Public xi, xf, yi, yf As Single 'localizacao do local a graficar
    'xi = x inicial, xf = x final , etc...
    '-Fim das variáveis do GANTT - FORM4

    '-Variáveis do FORM 13 de verificação das últimas horas
    Public Q_VERIFICA As Boolean
    'Fim das variáveis de verificação das últimas horas

    'Variáveis dos arquivos de saída do MGB
    Public VazaoHIG(,), QPROPHIG(,), VazaoMini(,), QOBS(,) As Single 'Double
    Public Começoresultado As Date
    Public tamdoresultado As Integer
    Public numerohidrogramas As Integer
    Public ajustehig(,) As Single 'Double
    Public ajusteok As Integer
    Public postoselecionado As String
    '--

    'Variáveis de gráficos de resultados
    Public Resultado As Boolean
    Public ResultadoLimiares As Boolean
    Public VisuResultado As Boolean
    Public ChuvaSUBS As Boolean
    Public Padrão As Boolean
    Public Grava As Boolean
    Public ONS As Boolean
    Public Grava_nível As Boolean
    '--

    'Variáveis da junção dos dois chuvabin
    Public chuvabin1(,), chuvabin2(,) As Double
    Public chuvasub1(,), chuvasub2(,) As Double
    Public tchuvabin1 As Integer
    '--

    'Variáveis interpolação chuva sub-bacia
    Public Sub_MINI() As Integer 'subbacia da minibacia
    '--

    'Variáveis da tabela de edição
    Public Tabela(,,,) As Double
    Public Tb_chuva As Boolean
    Public Tb_vazao As Boolean
    Public Nome_arq As String
    Public Nome_p As String
    '--

    'Variável que contéma  chuva no horozinte de previsão
    Public Chuva_no_horizonte As Double
    '--

    'Variáveis dos limiares
    Public Mini_Usa()
    Public Mini_TRs(,)

    'Variável para modelo rodar sozinho
    Public TotalCommander As Boolean
    Public ERRO_PREVISÃO As Integer

    'Variáveis dos gráficos padrão de previsão
    Public OpçãoGrafico As Integer

    Public AI As Integer
    Public AF As Integer

    'Public Const AI = 2015 '1880  'FMF 2018 06 23


    'Resultados de nível dos Ytudo do inercial
    Public CotaHIG(,), CotaPROPHIG(,) As Single 'Double



    Public Function Ano(ByVal aaaa As Integer) As Integer
        Ano = aaaa - AI
    End Function

End Module
