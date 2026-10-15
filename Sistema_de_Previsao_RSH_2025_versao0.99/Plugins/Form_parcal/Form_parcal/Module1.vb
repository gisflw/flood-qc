Imports System.Windows.Forms
Imports System.Drawing

Module Module1

    Public aux As Integer '-Variável para auxílio

    'Variáveis do Datagrid
    Public datagrid1 As New DataGrid
    Public T_ini As DataTable
    Public Arquivo As DataColumn = New DataColumn("Arquivo")
    Public Lon As DataColumn = New DataColumn("Longitude")
    Public Lat As DataColumn = New DataColumn("Latitude")
    Public Alt As DataColumn = New DataColumn("Cota (m)")
    'Public dados As DataColumn = New DataColumn("Dados")
    'Public D_I As DataColumn = New DataColumn("Dados Indisponíveis")
    Public observ As DataColumn = New DataColumn("Observações")
    Public Linhas2() As DataRow
    Public dataset2 As New DataSet

    Public sel_cel As Windows.Forms.DataGridCell
    Public sel_item As Object
    Public row_compl As Boolean
    '- Fim das variáveis do Datagrid

    'Variáveis dos postos
    Public Arqu(), obs() As String
    Public pmax, p As Integer
    Public x(), y(), Z() As Single
    Public chuva(,,,), chuvaI(,,,) As Single
    Public dataIp(), dataFp() As Date
    Public dd, mm, aaaa As Integer
    Public Const AI = 1880
    Public data(), dataF, dataI, datatemp() As Date
    Public xc(), yc(), A_km2(), xx, yy As Single 'dados das celulas

    Public coisa() As String
    Public jatemtabelachuva As Boolean
    '-Fim das variáveis dos postos

    'Variáveis que carregam a localização dos postos pluviométricos
    Public codigo(24915), nome(24915), linhas(4) As String
    Public lats(24915), lons(24915) As String
    Public POSTOS_BASE As Integer
    '- Fim das variáveis que carregam a localização dos postos pluviométricos

    'Variáveis do Gráfico GANTT - FORM4
    'VARIAVEIS DE GRAFICO
    Public graficoArea, graficoArea2 As Graphics
    Public bmpArea, bmparea2 As Bitmap
    Public Esp As Integer 'espacamento
    Public Larg_Rec As Single 'largura do retangulo

    Public precmax As Single 'prec max
    Public h, yloc, xloc As Single 'altura e localizacao do rectangulo

    Public xi, xf, yi, yf As Single 'localizacao do local a graficar
    'xi = x inicial, xf = x final , etc...
    '-Fim das variáveis do GANTT - FORM4

    'Variáveis das células
    Public numcels As Integer
    '-Fim das variáveis das células

    'Variáveis de tempo
    Public tmaxx As Integer
    'Fim das variáveis de tempo

    'Variáveis de diretório
    Public DIR As String
    '-Fim das variáveis de diretório


    'Variáveis do FORM_Flu
    'Variáveis do Datagrid 
    Public datagridq As New DataGrid
    Public T_iniq As DataTable
    Public Postoq As DataColumn = New DataColumn("Gauge")
    Public Minibacq As DataColumn = New DataColumn("Catchment")
    Public Acaoq As DataColumn = New DataColumn("Action")

    Public Linhasq() As DataRow
    Public datasetq As New DataSet

    Public sel_celq As Windows.Forms.DataGridCell
    Public sel_itemq As Object
    Public row_complq As Boolean
    '- Fim das variáveis do Datagrid

    Public jatemtabelaq As Boolean

    'Variáveis de Vazãoobs
    Public Qobs(,,,) As Single
    Public Numqobs As Integer
    Public Numqtot, Numqsubst As Integer

    'Variáveis da sugestão do CELL
    Public Mini_flu(), numcell_flu As Integer
    Public Xcen_flu(), Ycen_flu() As Single

    'Fim da variáveis do FORM5 

    'Variáveis do FORM9(Clima)

    'Variáveis de médias mensaids
    Public Chuvam(,,) As Single
    Public Nummeschuv As Integer
    Public Chuvaposto(,) As Single

    Public Tempm(,,) As Single
    Public NummesTemp(,) As Integer
    Public Tempposto(,) As Single

    Public Umim(,,) As Single
    Public NummesUmi(,) As Integer
    Public Umiposto(,) As Single

    Public Insom(,,) As Single
    Public NummesInso(,) As Integer
    Public Insoposto(,) As Single

    Public Presm(,,) As Double
    Public NummesPres(,) As Integer
    Public Presposto(,) As Double

    Public Ventom(,,) As Single
    Public NummesVento(,) As Integer
    Public Ventoposto(,) As Single

    'Variáveis do Datagrid 
    Public datagridclima As New DataGrid
    Public T_iniclima As DataTable
    Public Mesclima As DataColumn = New DataColumn("Month")
    Public Temperatura As DataColumn = New DataColumn("Temperature")
    Public Umidadere As DataColumn = New DataColumn("Humidity")
    Public Sol As DataColumn = New DataColumn("Insolation")
    Public Vento As DataColumn = New DataColumn("Wind")
    Public Pressão As DataColumn = New DataColumn("Pressure")


    Public Linhasclima() As DataRow
    Public datasetclima As New DataSet

    Public sel_celclima As Windows.Forms.DataGridCell
    Public sel_itemclima As Object
    Public row_complclima As Boolean
    '- Fim das variáveis do Datagrid

    Public jatemtabelaclima As Boolean

    Public nomes() As String 'nome dos postos

    Public usaposto() As Boolean

    'Variáveis dos mapas
    Public lastafield As String
    Public Tamanhopontoelinha As Integer
    '- Fim das variáveis do FORM9 (Clima)

    'Variáveis para as ferramentas de conversão
    Public dataIcon As Date  'datas iniciais e finais
    Public dataFcon As Date
    Public coisacon() As String ' somente nome do arquivo
    Public chuvacon(,,,) As Single
    Public pmaxcon As Integer
    'Fim das variáveis para conversão

    'Variáveis adicionadas para a conversão do arquivo de clima!!
    Public Tempmedcon(,,,) As Single
    Public Tempmaxcon(,,,) As Single
    Public Tempmincon(,,,) As Single
    Public Tempaguamedcon(,,,) As Single
    Public Tempaguamaxcon(,,,) As Single
    Public Tempaguamincon(,,,) As Single
    Public Tempsolomedcon(,,,) As Single
    Public Tempsolomaxcon(,,,) As Single
    Public Tempsolomincon(,,,) As Single
    Public Anemtanqcon(,,,) As Single
    Public Anemtanqlcon(,,,) As Single
    Public Anemelevcon(,,,) As Single
    Public Anemelevlcon(,,,) As Single
    Public Evappichecon(,,,) As Single
    Public PicheLeituracon(,,,) As Single
    Public Pichecomplecon(,,,) As Single
    Public Evaptanqcon(,,,) As Single
    Public Evaptanqlcon(,,,) As Single
    Public Evaptanqccon(,,,) As Single
    Public Chuvaógrafo(,,,) As Single
    Public Insolação(,,,) As Single
    Public Umidade(,,,) As Single
    Public Tseco(,,,) As Single
    Public Tumido(,,,) As Single
    Public Patm(,,,) As Single
    '-Fim das variáveis adicionadas para coversão de arquivos de clima

    'Variáveis para o cálculo do BFI e do hidrograma
    Public Datainicial, Datafinal As Date 'data inicial e final do arquivo
    Public coefk, coefa As Double 'coeficientes k e a 
    Public vazao(,,,), bi(,,,), bifinal(), bfifinal() As Double 'vazão real, escoamento de base calculado, escoamento base final
    Public postoselecionado As String
    Public datacon As Date
    Public tamint As Integer
    Public vazaoun(), biun() As Double
    'Public bifinal() As Double



    'Variáveis do FORM7
    Public numerodeblocoscal As Integer
    Public numpar, numbacia As Integer
    Public Parametro(,,) As Single
    Public nomeblocos() As String

    'Variáveis do Datagrid 
    Public datagriduso As New DataGrid
    Public T_iniuso As DataTable
    Public Blocouso As DataColumn = New DataColumn("HRC")
    Public Wm As DataColumn = New DataColumn("Wm")
    Public b As DataColumn = New DataColumn("b")
    Public Kbas As DataColumn = New DataColumn("Kbas")
    Public Kint As DataColumn = New DataColumn("Kint")
    Public XL As DataColumn = New DataColumn("XL")
    Public CAP As DataColumn = New DataColumn("CAP")
    Public Wc As DataColumn = New DataColumn("Wc")


    Public Linhasuso() As DataRow
    Public datasetuso As New DataSet

    Public sel_celuso As Windows.Forms.DataGridCell
    Public sel_itemuso As Object
    Public row_compluso As Boolean
    '- Fim das variáveis do Datagrid

    Public jatemtabelauso As Boolean
    '-Fim das Variáveis do FORM7

    'Variáveis do FORM10
    Public nomedoprojeto As String
    Public diretóriodoprojeto As String
    Public projeto As String
    Public NC, NU, NB, NCLI, Nqobsf, Nqsubstf As Integer
    Public IndReserv As Integer 'Índice para simular reservatórios e que indica número de reservatórios
    Public DirCotaVol As String 'Diretório das tabelas cota-volume da rotina reservatórios
    Public celpostoscomdados, substitui(10) As String

    Public podematar As Boolean

    Public arqvazao As String
    'Fim da variáveis do FORM10

    'Variáveis do FORM Proj
    Public tb As String

    Public jasalvo As String
    Public nomedoproj As String
    Public dadosclidia() As String
    Public numcli As Integer
    Public numcel As Integer
    Public numusos As Integer
    Public numbac As Integer
    Public numqobs1 As Integer
    Public ncelqobs() As String
    Public numqsubst1 As Integer
    Public ncelqsubst() As String

    'Fim das variáveis do FORM Proj


    'Variáveis das ferramentas
    Public BFI, Hidrograma, resultado, visuresultado, visuresultadolevel, BFImax, curv_permanencia, result_permanencia, Pluviograma, visuresultadoarea As Boolean
    Public soresult_permanencia As Boolean

    'Variáveis dos arquivos de saída do MGB
    Public VazaoHIG(,), QPROPHIG(,), VazaoMini(,), NivelHIG(,), FloodedAreaHIG() As Single
    Public Começoresultado As Date
    Public tamdoresultado As Integer
    Public numerohidrogramas As Integer
    Public ajustehig(,) As Single
    Public ajusteok As Integer

    'variáveis da curva de permanência
    Public vazãoord(), Qpercent() As Single
    Public temvazao As Integer

    'Parâmetros para a calibração
    Public Parametrome(,,), Parametroma(,,) As Single
    Public Parametromedio(,), Parametromediomen(,), Parametromediomai(,) As Single


    'Essa parte de baixo elimina o erro causado pelo uso ao mesmo tempo da tela de vazão e de chuva

    Public vazão(,,,), chuvareal(,,,) As Double


    'Variáveis do formulário de lançamento de poulentes no rio (TCC)
    Public TotalMINI As Integer 'Número de minibacias, descoberto em uma leitura inicial do arquivo de minibacias
    Public MINIBACIA As Integer 'Identificador da Minibacia
    Public Area_MINI() As Double 'area em km² de cada minibacia
    Public Subwatershed() As Integer 'subbacias
    Public Ltr_MINI() As Double 'comprimento do trecho de rio na minibacia
    Public Str_MINI() As Double 'declividade do trecho de rio na minibacia
    Public MiniJus() As Integer 'minibacia a jusante da bacia selecionada
    Public Confluência() As Integer 'Outra minibacia que tem a mesma mini_bacia como jusante
    'Public Vazao_MINI() As Double 'Vazão calculada por simulação hidrológica para cada minibacia
    Public MINI_CatID() As Integer 'CatID da MINIBACIA
    Public SHPIDX_Cat_ID() As Integer
    Public CatID_MINI() As Integer 'MINIBACIA da CatID
    Public Ordem_MINI() As Integer 'ordem da minibacia (cabeceira, secundária...)
    Public concentraMAX() As Double
    Public Lx As Integer
    Public Q90_MINI() As Double
    Public Q70_MINI() As Double
    Public Q50_MINI() As Double
    Public Q30_MINI() As Double
    Public Q10_MINI() As Double
    Public U_MINI() As Double
    Public U90_MINI() As Double
    Public U70_MINI() As Double
    Public U50_MINI() As Double
    Public U30_MINI() As Double
    Public U10_MINI() As Double
    Public RHIDR_MINI() As Double
    Public H_MINI() As Double
    Public B_MINI() As Double
    Public A_MINI() As Double
    Public El_MINI() As Double

    'Variável que diz qual bacia estamos propagando
    Public BACIA_PROPAGA As Integer

    'Variável com os resultados da simulação
    Public y_MINI(,) As Double
    Public y2_MINI(,) As Double 'variável auxiliar para mais de um lançamento

    'Variável da Convolução
    Public Convolução_c(,) As Double
    Public Distância

    'Variável com resultados da calibração
    Public Calibra(,) As Single
    Public Calibrasn As Integer
    Public Multiplica(,) As Integer
    Public Coluna() As String
    Public Resultado_Evolution() As Single
    Public Colunas_Evolution As Integer
    Public Parametro_Calibrado(,,) As Single


    Public LocalCELL As String 'path do CELL
    Public LocalRESUL As String 'path do arquivo com os resultados
    Public VolLanc As Double ' Volume do lançamento
    Public ConcLanc As Double 'Concentração do lançamento
    Public MassaLanc
    Public VazaoLanc As Double
    Public Decaimento As Double 'coeficiente de decaimento
    Public Subs As String 'Substância
    Public Diff As Double 'coeficiente de difusão
    Public Qmed, Vmed, Amed, Hmed, Bmed As Double 'Vazão, velocidade e área da seção médias do curso de água
    Public Limite_subs As Double 'limite imposto para a substância
    Public Tempo_simu As Double 'tempo de simulação, quanto tempo ficamos parado a pluma passar
    Public ConcMAX As Double 'Concentração máxima
    Public Vdilui As Double 'Volume que devo fazer a diluição, é a soma das vazões de todas as drenagens do meu rio principal
    Public Conc_Natural As Double 'Concentração natural do poluente no curso de água

    Public Lançamento, Receptor, SalvaCenário As Boolean 'variáveis do class para saber se clicamos nos botões de lançamento acidental

    Public Número_Lançamento As Integer 'Se é o primeiro, segundo, terceiro... lançamento

    Public Nível_água_rio As Integer 'Se é seca, cheia, etc...

    Public Local_lançamento As Integer 'minibac lançamento
    Public Local_receptor As Integer 'minibac receptora
    Public Xrecep As Double 'distância do receptor


    'Variáveis do form dos resultados do MGB

    Public Total_CELL As Integer
    Public Q90_CELL() As Double
    Public Q70_CELL() As Double
    Public Q50_CELL() As Double
    Public Q30_CELL() As Double
    Public Q10_CELL() As Double
    Public U_CELL() As Double
    Public U10_CELL() As Double
    Public U30_CELL() As Double
    Public U50_CELL() As Double
    Public U70_CELL() As Double
    Public U90_CELL() As Double

    Public B_CELL() As Double

    'Variável para o cálculo da vazão das propagações
    Public Q_prop, Q_bac, Q_conf As Double

    'Variável para concertar concentrações
    Public Concentra_concerta As Double

    'Licença estudante
    Public ESTUDANTE As Integer

    'Variaveis que possibilitam carregar os resultados de uma simulação
    Public Resultados_carregados As Integer

    'Variável que define o nome do arquivo de mini_bacias no form_plu
    Public MINI_Nome_Plu As String

    'Variáveis de Reoxigenação
    Public TEMPERATURA_reoxi As Double
    Public CODsat_reoxi As Double
    Public ka_reoxi As Double
    Public kd_reoxi As Double
    Public defcit_oxi(,) As Double
    Public OD(,) As Double
    Public Dist_percorrida As Double
    Public Tempo_viagem As Double
    Public Q_dilui_oxi As Double
    Public concentra_dilui_oxi As Double
    Public DBO As Double

    Public linha As String '-variável de leitura

    Public numSubWatersheds As Integer 'número de sub-baciass

    Public CRU_index As Integer 'se usa dados no formato CRU ou INMET de clima
    Public flagInertial As Boolean 'indica se foi selecionado simulação inercial no form_simu

    Public Function Ano(ByVal aaaa As Integer) As Integer
        Ano = aaaa - AI
    End Function
End Module
