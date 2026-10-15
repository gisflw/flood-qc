Imports System.Globalization
Imports System.IO
Imports System.Net
Imports System.Reflection
Imports System.Runtime.CompilerServices
Imports System.Text
Imports System.Threading
Imports System.Windows.Forms
Imports System.Xml

Public Class Form7
    Private numcels As Integer 'número de células
    Private linha As String 'variável de leitura
    Private X_MINI() As Single 'Double 'FMF 2018 06 23 'longitude da minibacia
    Private Y_MINI() As Single 'Double 'FMF 2018 06 23 'latitude da minibacia
    Private Distância_Mínima() As Single 'Double 'FMF 2018 06 23 'Distância mínima do raio de busca de cada posto
    Public Chuvabin(,) As Single 'Double 'FMF 2018 06 23
    Private Total_MiniBacias As Integer
    Public sf As MapWinGIS.Shapefile
    Private diretorio_previsao As String 'diretório onde deve ser salvo o arquivo de chuvas observadas
    Private diretorio_arquivos As String 'diretório arquivos telemétricos downlodeados
    Private diretorio_arquivos2 As String 'diretório arquivos telemétricos downlodeados no IPH-SAT novo
    Private diretorio_arquivos3 As String 'diretório arquivos telemétricos downlodeados no IPH-SAT novo
    Private ChuvaInterpoladaLida(,) As Single 'Chuva já interpolada

    'Variáveis globais do programa
    'Variáveis de leitura do chuvabin existente
    'Public NC As Integer 'número de células Minibacias
    'Public NT As Integer 'número de intervalos de tempo
    'Public DataI As Date 'data inicial do arquivo de chuva
    'Public DataI_string As String 'data inicial do arquivo de chuva na forma de string
    'Public Dataf As Date 'data final do arquivo de chuva
    'Public Dataf_string As String 'data inicial do arquivo de chuva na forma de string
    'Public ChuvaMatriz(,) As Single 'ChuvaMatriz(NC, NT)

    'Variáveis de leitura dos dados da SOMAR
    Public opendiagSOMAR As New Windows.Forms.OpenFileDialog
    Public DataIsomar, DataFsomar As Date
    Public NT_SOMAR As Integer
    Public ChuvaMatrizSOMAR(,) As Single 'ChuvaMatriz(NC, NT)

    'Variáveis do novo CHUVABIN
    Public NOVO_NT As Integer 'número de intervalos de tempo

    Private Fonte_Dados_Automaticos As Integer 'fonte dos dados automáticos de chuva e vazão

    '-- Fim das variáveis globais


    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        ofd.Multiselect = False
        'OFD.Filter = "Arquivo com dados de geometria do MGB(.MGB)|*.MGB"
        ofd.Filter = "Arquivo com as mini-bacias para o MGB(.mgb;.txt)|*.mgb;*.txt"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            tb_mini.Text = ofd.FileName
            numcels = -1
            FileOpen(1, ofd.FileName, OpenMode.Input)
            Do Until EOF(1)
                linha = LineInput(1)
                numcels = numcels + 1
                If numcels >= 1 Then
                End If
            Loop
            FileClose()

            Total_MiniBacias = numcels
            ReDim X_MINI(Total_MiniBacias)
            ReDim Y_MINI(Total_MiniBacias)

            numcels = -1
            FileOpen(1, ofd.FileName, OpenMode.Input)
            Do Until EOF(1)
                linha = LineInput(1)
                numcels = numcels + 1
                If numcels >= 1 Then
                    X_MINI(numcels) = linha.Substring(22, 9)
                    Y_MINI(numcels) = linha.Substring(37, 9)
                End If
            Loop
            FileClose()

            Me.VERIFICA_D_MIN()

            MsgBox("Leitura do arquivo Mini.MGB realizada com sucesso!", MsgBoxStyle.Information)
            Button6.Enabled = True
        End If
    End Sub

    Private Sub Form7_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        'Carrega a tabela principal, com as informações das estações
        Me.CARREGA_TABELA_ESTAÇÕES()
        'Atualiza os seletores de datas
        Me.ATUALIZA_DATA()
        'Carrega o arquivo MINI.MGB de forma automática
        Me.CARREGA_MINI()
        'Carrega a base de dados de chuva interpolada
        Me.CARREGA_CHUVA_INTERPOLADA()
        'Atualiza novamente a data final com base na chuva interpolada
        Me.RECALCULA_DATA_FINAL_SOMAR()
        'Carrega o diretório de previsão
        Me.CARREGA_DIRETÓRIOS()

        'Tenta atualizar os dados automaticamente
        'Try
        'Me.ATUALIZA_AUTOMATICAMENTE()
        'Me.ATUALIZA_AUTOMATICAMENTE2()
        'Me.ATUALIZA_AUTOMATICAMENTE3()

        ' Catch ex As Exception
        'MsgBox(ex.Message)
        'ToolStripStatusLabel1.Text = "Não foi possível atualizar os dados automaticamente."
        ' End Try

        'Se o automático estiver ligado
        If TotalCommander = True Then


            Dim log_file As String
            log_file = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Log_operação_automatica.txt"
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Abriu a janela de chuva observada e carregou todos os arquivos de entrada")
            FileClose(99)

            Me.CARREGA_OPCAO_DADOS()

            If Fonte_Dados_Automaticos = 0 Then
                FileOpen(99, log_file, OpenMode.Append)
                PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Atualizando dados online...")
                FileClose(99)
                Me.ATUALIZAR_ONLINE()
                FileOpen(99, log_file, OpenMode.Append)
                PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Atualizou dados online.")
                FileClose(99)
            ElseIf Fonte_Dados_Automaticos = 1 Then
                FileOpen(99, log_file, OpenMode.Append)
                PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Atualizando dados de uma pasta...")
                FileClose(99)
                Me.CARREGAR_DADOS_P_NOVO_DE_UMA_PASTA()
                FileOpen(99, log_file, OpenMode.Append)
                PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Atualizou dados de  uma pasta.")
                FileClose(99)
            End If
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Interpolando a chuva...")
            FileClose(99)
            Me.EXECUTAR_BOTAO_INTERPOLAR()
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Chuva interpolada com sucesso, próxima etapa: Chuva prevista.")
            FileClose(99)
            Me.Close()
        End If


        ''Se o automático estiver ligado
        'If TotalCommander = True Then
        '    Me.ATUALIZAR_ONLINE()

        '    Me.EXECUTAR_BOTAO_INTERPOLAR()
        '    Me.Close()
        'End If

    End Sub

    Sub CARREGA_CHUVA_INTERPOLADA()
        Dim ArquivoChuvaInterpolada As String
        ArquivoChuvaInterpolada = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\ChuvaInterpolada\CHUVA_OBS.MGB"
        'Declara as variáveis
        Dim nome_metadados As String
        nome_metadados = System.IO.Path.GetFileNameWithoutExtension(ArquivoChuvaInterpolada)
        Dim diretório As String
        diretório = System.IO.Path.GetDirectoryName(ArquivoChuvaInterpolada)

        Dim arquivo_metadados As String
        arquivo_metadados = diretório & "\" & nome_metadados & ".meta"

        FileOpen(1, arquivo_metadados, OpenMode.Input)

        Dim datainiciopob As Date
        Dim dataFimpob As Date

        Dim ll As Integer
        ll = 0
        Dim linha As String

        Do Until EOF(1)
            linha = LineInput(1)
            ll = ll + 1
            If ll = 10 Then
                datainiciopob = linha.Substring(0, 4) & "/" & linha.Substring(8, 2) & "/" & linha.Substring(14, 2) & " " & linha.Substring(20, 2) & ":00:00"
                Dim addhour As Integer
                addhour = linha.Substring(22, 8)
                addhour = addhour - 1
                dataFimpob = datainiciopob.AddHours(addhour)
                ' ElseIf ll = 13 Then
                ' total_mini = linha
            End If
        Loop

        Dim dia, mes, hora As String
        dia = datainiciopob.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datainiciopob.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datainiciopob.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        lbInterpDt1.Text = dia & "/" & mes & "/" & datainiciopob.Year & " " & hora & ":00:00"

        dia = dataFimpob.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = dataFimpob.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = dataFimpob.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        lbInterpDt2.Text = dia & "/" & mes & "/" & dataFimpob.Year & " " & hora & ":00:00"
        FileClose()

        Dim tmaxx1 As Integer
        tmaxx1 = (DateDiff(DateInterval.Hour, datainiciopob, dataFimpob) + 1)
        'tmaxx = dif. de tempo

        ReDim ChuvaInterpoladaLida(Total_MiniBacias, tmaxx1)

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(ArquivoChuvaInterpolada, FileMode.Open)
        Dim bw As New System.IO.BinaryReader(fs)

        '  FileOpen(2, "teste.txt", OpenMode.Output, OpenAccess.Write)

        Dim numcells, cell, tempobin As Integer
        numcells = Total_MiniBacias
        cell = 0
        tempobin = tmaxx1

        For tempo As Integer = 1 To tempobin
            For cells As Integer = 1 To numcells

                ChuvaInterpoladaLida(cells, tempo) = bw.ReadSingle

            Next
        Next
        bw.Close()

    End Sub

    Sub CARREGA_TABELA_ESTAÇÕES()
        ' Try
        'Verifica quantas estações existem na base de dados interna
        Dim arquivo_est As String 'nome do arquivo que contém as estações
        Dim linha As String 'linha de leitura
        total_estações = 0 'número de estações

        'define o arquivo
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesPluv.txt"

        'Abre e faz a leituara
        FileOpen(1, arquivo_est, OpenMode.Input)
        Do Until EOF(1)
            linha = LineInput(1)
            total_estações = total_estações + 1
        Loop
        FileClose()
        total_estações = total_estações - 1 'desconta a primeira linha, que é cabeçalho
        '-- Fim da verificação do total de estações

        'Redimensiona as variáveis
        ReDim ID(total_estações)
        ReDim Estação(total_estações)
        ReDim LAT(total_estações), LON(total_estações), ALT(total_estações)
        ReDim NOME(total_estações)
        ReDim A(total_estações), B(total_estações), C(total_estações)
        ReDim Chuva(total_estações, 24, 31, 12, Ano(AF))
        ReDim dataIp(total_estações)
        ReDim dataFp(total_estações)
        ReDim HoraIp(total_estações)
        ReDim HoraFp(total_estações)
        ReDim AssimilaP(total_estações)
        '--

        'Zera as variáveis de data
        For p As Integer = 0 To total_estações
            dataIp(p) = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
            dataFp(p) = "01/01/" & AI & " " & "00:00:00"
            HoraIp(p) = 23
            HoraFp(p) = 0
        Next
        '--

        'Zera as variáveis de vazão e nível
        For p As Integer = 0 To total_estações
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Chuva(p, hh, dd, mm, Ano(aaaa)) = -1.0
                        Next
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesPluv.txt"
        FileOpen(1, arquivo_est, OpenMode.Input)
        Dim est As Integer
        est = -1

        Do Until EOF(1)

            linha = LineInput(1)
            est = est + 1
            If est > 0 Then

                Dim splt
                splt = Split(linha, ";")

                ID(est) = splt(0)
                Estação(est) = splt(1)
                LAT(est) = AjustarDecimal(splt(2))
                LON(est) = AjustarDecimal(splt(3))
                ALT(est) = AjustarDecimal(splt(4))
                NOME(est) = splt(5)
                A(est) = AjustarDecimal(splt(6))
                B(est) = AjustarDecimal(splt(7))
                C(est) = AjustarDecimal(splt(8))
                AssimilaP(est) = AjustarDecimal(splt(9))

                '-desativado em 21 de outubro de 2025 para acelerar  Me.LE_DATABASE(est, Estação(est))

                Me.LE_DATABASE_Fast(est, Estação(est))

                'Adiciona os dados à tabela

                Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

                ' Preenche os valores do registro
                nova_linha.ItemArray = New Object() {Estação(est), dataIp(est), dataFp(est), LON(est), LAT(est), NOME(est), AssimilaP(est)}
                Try
                    DataSet1.Tables(0).Rows.Add(nova_linha)
                Catch ex As Exception
                    MsgBox(" erro: " & ex.Message)
                End Try

            End If
        Loop

        FileClose()
        'Carregou dados internos

        'Catch ex As Exception
        '    MsgBox(ex.Message)
        'End Try

    End Sub

    Sub LE_DATABASE(ByVal numest As Integer, ByVal codest As String)
        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\P" & codest & ".txt"

        FileOpen(2, arquivo_est2, OpenMode.Input)

        'Declara variáveis de leitura
        Dim contador As Integer
        contador = 0
        Dim linha2 As String
        '--

        Do Until EOF(2)

            linha2 = LineInput(2)

            contador = contador + 1

            If contador > 1 Then
                Chuva(numest, linha2.Substring(22, 2), linha2.Substring(4, 2), linha2.Substring(10, 2), Ano(linha2.Substring(14, 4))) = AjustarDecimal(linha2.Substring(26, 10))

                If Chuva(numest, linha2.Substring(22, 2), linha2.Substring(4, 2), linha2.Substring(10, 2), Ano(linha2.Substring(14, 4))) > 120 Then
                    Chuva(numest, linha2.Substring(22, 2), linha2.Substring(4, 2), linha2.Substring(10, 2), Ano(linha2.Substring(14, 4))) = -1
                End If

                If linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00" < dataIp(numest) Then
                    dataIp(numest) = linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00"
                    If dataIp(numest) < dataI Then
                        dataI = dataIp(numest)
                    End If
                End If

                If linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00" > dataFp(numest) Then
                    dataFp(numest) = linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00"
                    If dataFp(numest) > dataF Then
                        dataF = dataFp(numest)
                    End If
                End If
            End If
        Loop

        FileClose(2)


    End Sub

    Sub LE_DATABASE_Fast(ByVal numest As Integer, ByVal codest As String)
        ' Arquivo
        Dim basePath = Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly().Location)
        Dim arquivo As String = Path.Combine(basePath, $"database\P{codest}.txt")

        ' Inits de datas (garanta que fora daqui você inicializa dataIp(numest) = DateTime.MaxValue e dataFp(numest) = DateTime.MinValue)
        Dim dataMin As Date = Date.MaxValue
        Dim dataMax As Date = Date.MinValue

        ' Leitor com buffer maior (1 MB) e UTF8 sem BOM (ajuste encoding se necessário)
        Using sr As New StreamReader(arquivo, New UTF8Encoding(False), detectEncodingFromByteOrderMarks:=True, bufferSize:=1024 * 1024)
            Dim linha As String
            Dim isHeader As Boolean = True

            While True
                linha = sr.ReadLine()
                If linha Is Nothing Then Exit While
                If isHeader Then
                    isHeader = False
                    Continue While ' pula a 1ª linha (cabeçalho)
                End If

                If linha.Length < 26 Then Continue While ' linha inválida/curta

                ' --- Parse de campos (fixo 6/6/6/6/16) ---
                ' último par de dígitos do dia/mês/hora; ano em 4 dígitos
                Dim dia As Integer = Integer.Parse(linha.Substring(4, 2), CultureInfo.InvariantCulture)
                Dim mes As Integer = Integer.Parse(linha.Substring(10, 2), CultureInfo.InvariantCulture)
                Dim ano_calend As Integer = Integer.Parse(linha.Substring(14, 4), CultureInfo.InvariantCulture)
                Dim hora As Integer = Integer.Parse(linha.Substring(22, 2), CultureInfo.InvariantCulture)

                ' Valor: pegue da 5ª coluna em diante e faça Trim()
                Dim valStr As String = linha.Substring(24).Trim()
                Dim valor As Double
                If Not Double.TryParse(valStr, NumberStyles.Float Or NumberStyles.AllowThousands, CultureInfo.InvariantCulture, valor) Then
                    valor = -1.0R
                End If

                ' QC específico da chuva (o seu código usa 120 como teto)
                If valor > 200 Then valor = -1.0R

                ' Índice do "Ano(...)": calcule uma única vez
                Dim idxAno As Integer = Ano(ano_calend)

                ' Grava no seu cubo Chuva(estação, hora, dia, mês, anoIndex)
                Chuva(numest, hora, dia, mes, idxAno) = valor

                ' DataTime para comparações rápidas (sem conversão implícita de string)
                Dim dt As New DateTime(ano_calend, mes, dia, hora, 0, 0, DateTimeKind.Unspecified)

                If dt < dataMin Then dataMin = dt
                If dt > dataMax Then dataMax = dt
            End While
        End Using

        ' Atualiza datas da estação e globais
        dataIp(numest) = dataMin
        dataFp(numest) = dataMax

        ' Atualize globais (supondo dataI é o mínimo global e dataF o máximo global)
        If dataMin < dataI Then dataI = dataMin
        If dataMax > dataF Then dataF = dataMax
    End Sub


    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        Me.ATUALIZAR_CHUVA_OFFLINE_TRIPA()
    End Sub

    Sub Atualizar_Chuva_Offline_old()
        ofd.Multiselect = True
        ofd.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            Dim s As String
            Dim pmax As Integer
            pmax = ofd.FileNames.Length
            Dim Arqu(pmax) As String
            Dim p As Integer
            p = 0

            statusbar.Maximum = pmax
            statusbar.Value = 0



            For Each s In ofd.FileNames
                p = p + 1
                Arqu(p) = s

                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) '.Substring(0, 8)

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo
                    Dim linha4 As String
                    Dim pl As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        'Dim splt
                        'splt = Split(linha4, ";")
                        'Lê a chuva
                        Try
                            pl = AjustarDecimal(linha4.Substring(20, 10))
                        Catch ex As Exception
                            pl = -1
                        End Try

                        If pl >= 0 And pl < 9999 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = pl
                        ElseIf pl < 0 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        ElseIf pl >= 9999 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                            dataIp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataIp(CodigoLocalizado) < dataI Then
                                dataI = dataIp(CodigoLocalizado)
                            End If
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                            dataFp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataFp(CodigoLocalizado) > dataF Then
                                dataF = dataFp(CodigoLocalizado)
                            End If
                        End If


                    Loop
                    FileClose(4)



                End If


                statusbar.Increment(1)
            Next

            Me.AtualizaDataTabela()
            Me.ATUALIZA_ARQUIVOS_CHUVA()
            Me.ATUALIZA_DATA()
            ToolStripStatusLabel1.Text = "Atualização finalizada"


        End If


    End Sub

    Sub ATUALIZAR_CHUVA_OFFLINE_TRIPA()
        ofd.Multiselect = True
        ofd.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            Dim s As String
            Dim pmax As Integer
            pmax = ofd.FileNames.Length
            Dim Arqu(pmax) As String
            Dim p As Integer
            p = 0

            statusbar.Maximum = pmax
            statusbar.Value = 0

            For Each s In ofd.FileNames
                p = p + 1
                Arqu(p) = s

                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) '.Substring(0, 8)

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = "P" & Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo
                    Dim linha4 As String
                    Dim pl As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Dim contador As Integer
                    contador = -1
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        contador += 1
                        If contador < 1 Then Continue Do

                        pl = AjustarDecimal(linha4.Substring(26, 10))

                        If pl < 0 Then
                            pl = -1
                        ElseIf pl >= 9999 Then
                            pl = -1
                        End If

                        Chuva(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = pl

                        If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                            dataIp(CodigoLocalizado) = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            If dataIp(CodigoLocalizado) < dataI Then
                                dataI = dataIp(CodigoLocalizado)
                            End If
                        End If

                        If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                            dataFp(CodigoLocalizado) = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            If dataFp(CodigoLocalizado) > dataF Then
                                dataF = dataFp(CodigoLocalizado)
                            End If
                        End If

                    Loop
                    FileClose(4)
                End If
                statusbar.Increment(1)
            Next

            Me.AtualizaDataTabela()
            'Me.ATUALIZA_ARQUIVOS_CHUVA() 'Desativada em 20 de outubro de 2025
            Me.ATUALIZA_ARQUIVOS_CHUVA_Legado() 'nova rotina de atualização
            Me.ATUALIZA_DATA()
            ToolStripStatusLabel1.Text = "Atualização finalizada"


        End If

    End Sub

    '--- NOVA ROTINA DE ESCRITA DE DADOS DE CHUVA
    Private Function Alinha(valor As String, largura As Integer) As String
        Return valor.PadLeft(largura)
    End Function

    Private Function FormataChuva(valor As Double) As String
        Const LARGURA As Integer = 16

        If Double.IsNaN(valor) OrElse Double.IsInfinity(valor) Then
            Return Alinha("-1.000000", LARGURA)
        End If

        ' Qualidade: >300 mm/h é inválido
        If valor > 300 Then valor = -1.0
        If valor < 0 Then Return Alinha("-1.000000", LARGURA)

        ' 6 casas decimais, ponto como separador
        Dim s = valor.ToString("0.000000", CultureInfo.InvariantCulture)
        Return Alinha(s, LARGURA)
    End Function

    ' Reaproveite Alinha e FormataChuva acima

    Sub ATUALIZA_ARQUIVOS_CHUVA_Legado()
        Const L6 As Integer = 6

        For p As Integer = 1 To total_estações
            Dim tmaxx As Integer = CInt(DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
            Dim dt0 As Date = dataIp(p)
            Dim arquivo As String =
            System.IO.Path.GetDirectoryName(Reflection.Assembly.GetExecutingAssembly().Location()) &
            "\database\P" & Estação(p) & ".txt"

            FileOpen(2, arquivo, OpenMode.Output)

            PrintLine(2,
            Alinha("Dia", L6) &
            Alinha("Mês", L6) &
            Alinha("Ano", L6) &
            Alinha("Hora", L6) &
            Alinha(NOME(p), 16)
        )

            For i As Integer = 0 To tmaxx - 1
                Dim dt As Date = dt0.AddHours(i)

                Dim dia = Alinha(dt.Day.ToString("00"), L6)
                Dim mes = Alinha(dt.Month.ToString("00"), L6)
                Dim ano_calend = Alinha(dt.Year.ToString(), L6)
                Dim hora = Alinha(dt.Hour.ToString("00"), L6)

                Dim px As Double = Chuva(p, dt.Hour, dt.Day, dt.Month, Ano(dt.Year))
                Dim pFmt As String = FormataChuva(px)

                PrintLine(2, dia & mes & ano_calend & hora & pFmt)
            Next

            FileClose(2)
        Next
    End Sub



    '--FIM NOVAS ROTINAS ESCRITA DADOS DE CHUVA


    Sub AtualizaDataTabela()

        '-- Desativado em 11ago2026 - FMF
        'For p As Integer = 1 To total_estações
        '    DataGridView1.Item(1, p - 1).Value = dataIp(p)
        '    DataGridView1.Item(2, p - 1).Value = dataFp(p)
        'Next

        'Percorre as linhas na ordem atual do DataGridView
        For i As Integer = 0 To DataGridView1.Rows.Count - 1

            'Ignora a linha vazia de inclusão, caso exista
            If Not DataGridView1.Rows(i).IsNewRow Then

                If DataGridView1.Item(0, i).Value IsNot Nothing Then

                    'Descobre qual estação está nesta linha
                    Dim codigoEstacao As String
                    codigoEstacao = DataGridView1.Item(0, i).Value.ToString.Trim

                    'Localiza o índice verdadeiro da estação nos vetores
                    Dim est As Integer
                    est = LOCALIZA_ESTACAO_NA_BASE(codigoEstacao)

                    'Se encontrou, atualiza os dados da linha correta
                    If est > 0 Then

                        DataGridView1.Item(1, i).Value = dataIp(est)
                        DataGridView1.Item(2, i).Value = dataFp(est)

                    End If

                End If

            End If

        Next


    End Sub

    Private Function LOCALIZA_ESTACAO_NA_BASE(ByVal codigoEstacao As String) As Integer
        For est As Integer = 1 To total_estações
            If Estação(est).ToString.Trim = codigoEstacao.Trim Then
                Return est
            End If
        Next
        'Não encontrou
        Return -1
    End Function

    Sub ATUALIZA_ARQUIVOS_CHUVA()
        For p As Integer = 1 To total_estações
            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
            'tmaxx = dif. de tempo

            Dim datacomeço As Date
            datacomeço = dataIp(p)
            'datacomeço.AddDays(1)

            'define o arquivo
            Dim arquivo_est2 As String
            arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\P" & Estação(p) & ".txt"

            FileOpen(2, arquivo_est2, OpenMode.Output)

            Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho

            'escreve o arquivo no cabeçalho
            For espaço As Integer = 1 To 12 - NOME(p).Length
                'primeiro esquece os espaços
                Print(2, " ")
            Next
            'escreve o nome do arquivo
            Print(2, NOME(p))
            PrintLine(2) 'finaliza a linha do cabeçalho


            For aux = 1 To tmaxx
                If aux <> 1 Then
                    datacomeço = datacomeço.AddHours(1)
                End If
                If datacomeço.Day < 10 Then
                    Print(2, "    " & "0" & datacomeço.Day)
                ElseIf datacomeço.Day >= 10 Then
                    Print(2, "    " & datacomeço.Day)
                End If
                If datacomeço.Month < 10 Then
                    Print(2, "    " & "0" & datacomeço.Month)
                ElseIf datacomeço.Month >= 10 Then
                    Print(2, "    " & datacomeço.Month)

                End If

                Print(2, "  " & datacomeço.Year)

                If datacomeço.Hour.ToString.Length = 1 Then
                    Print(2, "    " & "0" & datacomeço.Hour.ToString)
                ElseIf datacomeço.Hour.ToString.Length = 2 Then
                    Print(2, "    " & datacomeço.Hour.ToString)
                End If

                'Escreveu a data

                'Agora vai escrever os números

                'If FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 0 Then
                '    Print(2, "       " & "-1.000000")
                'ElseIf FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 0 And FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 10 Then
                '    Print(2, "        " & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2))
                'ElseIf FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 10 And FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 100 Then
                '    Print(2, "       " & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2))
                'ElseIf FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 100 And FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 1000 Then
                '    Print(2, "      " & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2))
                'ElseIf FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 1000 And FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 10000 Then
                '    Print(2, "     " & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(0, 1) & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(2, 10))
                'ElseIf FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 10000 And FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 100000 Then
                '    Print(2, "    " & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(0, 2) & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(3, 10))
                'ElseIf FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 100000 And FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 1000000 Then
                '    Print(2, "   " & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(0, 3) & FormatNumber(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(4, 10))
                '    'não diminui dois espaços pois não tem o separador de milhar
                'End If

                'Controle de qualidade dos dados de chuva antes de gravar
                If Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) > 300 Then
                    Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = -1.0
                End If

                If SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                    Print(2, "       " & "-1.000000")
                ElseIf SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                    Print(2, "        " & SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                    Print(2, "       " & SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                    Print(2, "      " & SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                    Print(2, "     " & SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                    Print(2, "    " & SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 Then
                    'Print(2, "   " & SubstituirSeparador(Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                    'não diminui dois espaços pois não tem o separador de milhar
                    Print(2, "       " & "-1.000000")

                Else
                    Print(2, "       " & "-1.000000")
                End If


                PrintLine(2)

            Next

            FileClose()

        Next

    End Sub


    Sub ATUALIZA_DATA()

        'Faz uma primeira tentativa
        dtpqob1.Value = dataIp(1)
        dtpqob2.Value = dataFp(1)
        tb_hora_ini.Text = tb_hora_ini.Items(dataIp(1).Hour)
        tb_hora_fim.Text = tb_hora_fim.Items(dataFp(1).Hour)


        'Agora percorre todos
        Dim DataFinalObs As Date
        DataFinalObs = dataFp(1)

        For p = 1 To total_estações
            If dataFp(p) > DataFinalObs Then
                DataFinalObs = dataFp(p)
            End If
        Next


        dtpqob2.Value = DataFinalObs
        tb_hora_fim.Text = tb_hora_fim.Items(DataFinalObs.Hour)

        If DataFinalObs > Now.Date Then
            dtpqob2.Value = Now.Date
            tb_hora_fim.Text = tb_hora_fim.Items(Now.Hour)
        End If

        Dim datahoje As Date
        datahoje = Now.Date
        If dtpqob1.Value < "01/07/" & datahoje.AddYears(-1).Year Then
            dtpqob1.Value = "01/07/" & datahoje.AddYears(-1).Year
            tb_hora_ini.Text = "00"
        End If



    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        Me.FALSE_ALL()
        Pluviograma = True

        '--Desativado em 11ago26 - FMF
        'Estação_GrafQ = DataGridView1.CurrentCell.RowIndex + 1

        '--Nova rotina em 11ago26 - FMF
        '--------------------------------------------------------
        'Descobre qual estação está selecionada no DataGridView
        '--------------------------------------------------------
        If DataGridView1.CurrentRow Is Nothing Then
            MsgBox("Nenhuma estação foi selecionada.")
            Exit Sub
        End If

        Dim codigoEstacao As String
        codigoEstacao = DataGridView1.CurrentRow.Cells(0).Value.ToString.Trim

        'Procura a estação na base original
        Dim est As Integer
        est = LOCALIZA_ESTACAO_NA_BASE(codigoEstacao)

        If est = -1 Then
            MsgBox("A estação selecionada não foi encontrada na base de dados.")
            Exit Sub
        End If

        'Agora usa o índice correto da estação,
        'independentemente da ordenação do DataGridView
        Estação_GrafQ = est
        '--Fim nova rotina em 11ago26 - FMF 

        Dim frmgraf As New Form4
        frmgraf.Show()
    End Sub

    Sub FALSE_ALL()
        Hidrograma = False
        Cotagrama = False
        Pluviograma = False
        Resultado = False
        ResultadoLimiares = False
        VisuResultado = False
        ChuvaSUBS = False
        Padrão = False
        Grava = False
        ONS = False
        Grava_nível = False
        ResultadoNível = False
        ResultadoNívelpp = False
        ResultadoVazaopp = False
        ResultadoNívelref = False
    End Sub


    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click
        'Condiociona e abre a caixa de diálogo
        Dim tamarq As Integer
        Dim coordx(), coordy() As Double
        Dim posto() As String

        sf = New MapWinGIS.Shapefile
        ' OFD.Multiselect = False
        ' OFD.Filter = "ASCII com coordenadas xy de pontos gerado no Excel(.txt)|*.txt"
        ' If OFD.ShowDialog() = DialogResult.OK Then

        'Abre o arquivo selecionado no diálogo
        '  FileOpen(1, OFD.FileName, OpenMode.Input)
        'Dimensiona as vairáveis que vão guardar o tamanho do arquivo e a linha de leitura
        '  Dim linhau As String
        tamarq = DataGridView1.RowCount

        'Le o arquivo para descobrir o tamanho
        'Do Until EOF(1)
        ' linhau = LineInput(1)
        ' tamarq = tamarq + 1
        ' Loop
        'Fecha o arquivo
        'FileClose()
        'Abre o arquivo novamente para pegar as coordenas desta vez
        'FileOpen(1, OFD.FileName, OpenMode.Input)
        'As variáveis coordxy são as coordenas, elas tem o tamanho do arquivo
        ReDim coordx(tamarq + 1), coordy(tamarq + 1)
        ReDim posto(tamarq + 1)
        ' Dim splt 'usado para quebrar a linha em duas
        Dim contaarq As Integer
        contaarq = 0
        'Percorre o arquivo e salva as coordenas
        For variável As Integer = 1 To tamarq - 1
            contaarq = contaarq + 1
            ' linhau = LineInput(1)
            ' splt = Split(linhau, " ")

            coordx(contaarq) = LON(contaarq)
            coordy(contaarq) = LAT(contaarq)
            posto(contaarq) = NOME(contaarq)

        Next


        'AxMap1.NewDrawing(MapWinGIS.tkDrawReferenceList.dlSpatiallyReferencedList)
        'Draw a red point where the mouse up event occured in map1
        ' AxMap1.DrawPoint(coordx(contaarq), coordy(contaarq), 5, System.Convert.ToUInt32(RGB(255, 0, 0)))
        ' MsgBox(coordx(contaarq) & " " & coordy(contaarq))
        ' Loop
        'Fecha o arquivo
        '  FileClose()
        'Abre o diálogo para selecionar o local onde vai salvar o novo shape
        sfd.Filter = "Shapefile com dos pontos selecionados(.shp)|*.shp"
        If sfd.ShowDialog() = Windows.Forms.DialogResult.OK Then

            'Declaração das variáveis
            'Dim sf As New MapWinGIS.Shapefile
            Dim f As New MapWinGIS.Field
            Dim pt As New MapWinGIS.Point
            Dim sh As New MapWinGIS.Shape
            Dim shp, pnt As Long
            Dim Result As Boolean


            Result = sf.CreateNew(sfd.FileName, MapWinGIS.ShpfileType.SHP_POINT)
            ' If Result = False Then GoTo ERRORHANDLER
            'Start Editing it...
            Result = sf.StartEditingShapes(True)
            'If Result = False Then GoTo ERRORHANDLER

            'Set up three new fields of type string, double and integer
            f = New MapWinGIS.Field
            f.Type = MapWinGIS.FieldType.STRING_FIELD
            f.Name = "Nome Posto"
            f.Width = 20
            Result = sf.EditInsertField(f, 0)
            'If Result = False Then GoTo ERRORHANDLER



            Dim i As Integer

            For i = 0 To tamarq - 2
                'initialize the shape
                sh = New MapWinGIS.Shape
                sh.ShapeType = MapWinGIS.ShpfileType.SHP_POINT

                'Set up the point
                pt = New MapWinGIS.Point
                '  MsgBox(coordx(i) & " " & coordy(i))
                'contad = contad + 1
                'MsgBox(contad)

                pt.x = coordx(i + 1)
                pt.y = coordy(i + 1)

                'AxMap1.ProjToPixel(coordx(i), coordy(i), pt.x, pt.y)
                'MsgBox(pt.x & " " & pt.y)

                'Add the point to a shape
                Result = sh.InsertPoint(pt, 0)
                '  If Result = False Then GoTo ERRORHANDLER

                'Add the shape to the shapefile
                Result = sf.EditInsertShape(sh, i)
                'If Result = False Then GoTo ERRORHANDLER

                'Edit the attributes for this shape record
                Result = sf.EditCellValue(0, i, posto(i + 1)) 'insert some string value
                'If Result = False Then GoTo ERRORHANDLER

            Next


            Result = sf.StopEditingShapes(True, True)
            MsgBox("Shapefile gerado com sucesso!", MsgBoxStyle.Information, "Vazões Observadas")

        End If
    End Sub

    Private Sub Button5_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button5.Click
        Try
            ReDim Qdispo(total_estações, 24, 31, 12, Ano(AF))

            'Zera as variáveis de vazão e nível
            For p As Integer = 0 To total_estações
                For aaaa = AI To AF
                    For mm = 1 To 12
                        For dd = 1 To 31
                            For hh As Integer = 0 To 23
                                Qdispo(p, hh, dd, mm, Ano(aaaa)) = Chuva(p, hh, dd, mm, Ano(aaaa))
                            Next
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis

            dataI = dataIp(1)
            dataF = dataFp(1)

            Dim frmdispo As New Form6
            frmdispo.Show()
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try


    End Sub

    Sub VERIFICA_D_MIN()
        ReDim Distância_Mínima(Total_MiniBacias)
        For Mini As Integer = 1 To Total_MiniBacias
            Distância_Mínima(Mini) = 999999
        Next

        For Mini As Integer = 1 To Total_MiniBacias
            For p As Integer = 1 To total_estações
                If distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) < Distância_Mínima(Mini) Then
                    Distância_Mínima(Mini) = distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p))
                End If
            Next
        Next

    End Sub

    Private Function distância(ByVal x_minibac As Double, ByVal Y_minibac As Double, ByVal lon_p As Double, ByVal lat_p As Double)
        Dim dist As Double
        dist = ((x_minibac - lon_p) ^ 2 + (Y_minibac - lat_p) ^ 2) ^ 0.5
        Return dist
    End Function

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        Me.ATUALIZA_TABELA_ESTAÇÕES_PLUV() 'Esta rotina atualiza as estações pluv, obviamente só é feita se a janela for mudada manualmente

        Me.EXECUTAR_BOTAO_INTERPOLAR()
        MsgBox("Interpolação Finalizada!", MsgBoxStyle.Information)

        Me.Close()
    End Sub

    Sub VERIFICA_QUEM_ASSIMILA()
        'Esta rotina verifica se algum posto não vai ser usado na interpolação e zera os calores dele

        'ReDim AssimilaP(total_estações)

        ''Verifica quem assimila
        'For posto As Integer = 1 To total_estações
        '    AssimilaP(posto) = DataGridView1.Item(6, posto - 1).Value
        'Next

        'Zera as variáveis dque não serão usadas
        For p As Integer = 0 To total_estações

            If AssimilaP(p) <= 0 Then
                For aaaa = AI To AF
                    For mm = 1 To 12
                        For dd = 1 To 31
                            For hh As Integer = 0 To 23
                                Chuva(p, hh, dd, mm, Ano(aaaa)) = -1.0
                            Next
                        Next
                    Next
                Next
            End If

        Next


    End Sub

    Sub EXECUTAR_BOTAO_INTERPOLAR()
        Me.ATUALIZA_EDITADOS_MANUALMENTE() 'Atualiza os dados de chuva, lendo o database, caso eles tenham sido editados manualmente
        'ToolStripStatusLabel1.Text = "Interpolando dados de chuva"
        Me.VERIFICA_QUEM_ASSIMILA()
        'Me.INTERPOLAR_V2() 'Faz a interpolação dos dados pluviométricos
        'Me.INTERPOLAR() 'Rotina simplificada de interpolação
        If CheckBox4.Checked = True Then
            'Me.INTERPOLAR_V5() 'Faz a interpolação dos dados  por vizinho mais próximo
            Me.INTERPOLAR_V6()

        ElseIf CheckBox4.Checked = False Then
            'Me.INTERPOLAR_V4() 'Faz a interpolação dos dados  por vizinho mais próximo
            Me.INTERPOLAR_V7()
        End If
        Me.SALVAR_CHUVABIN_AUTOMÁTICO() 'Salva o arquivo binário com dados de chuva no diretório especificado
        'Me.SALVAR_CHUVABIN() 'Salva o ChuvaBin usando o save file dialog, está desligado em troca do automático
        If CheckBox2.Checked Then 'Salva o chuvatexto, caso requisitado
            Me.SALVA_CHUVATEXTO()
        End If
        Me.ESCREVER_METADADOS() 'Escreve o metadados

        If CheckBox3.Checked Then
            Me.SALVAR_DATABASECHUVABIN_AUTOMÁTICO()
        End If

        'Me.Close()
        'Apaga a chuva
        Chuva = Nothing
        Erase Chuva
        Chuvabin = Nothing
        Erase Chuvabin
        'Me.Close()
    End Sub

    Sub SALVAR_CHUVABIN()

        sfd.Filter = "Arquivo de chuva binário para o MGB(.MGB)|*.MGB"

        If sfd.ShowDialog = Windows.Forms.DialogResult.OK Then

            Dim nomeArquivo As String
            nomeArquivo = sfd.FileName

            ' abre o arquivo e iniciao o objeto StreamReader
            Dim fs As FileStream
            fs = New FileStream(nomeArquivo, FileMode.Create, FileAccess.ReadWrite)
            Dim bw As New System.IO.BinaryWriter(fs)


            Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

            For tempo As Integer = 1 To tmaxx
                For cells As Integer = 1 To Total_MiniBacias
                    Dim numero As Single
                    numero = Chuvabin(cells, tempo)
                    bw.Write(numero)
                    'MsgBox(cells & " " & tempo & " " & chuvabin(cells, tempo))
                Next
            Next

            bw.Close()

            MsgBox("Arquivo salvo com sucesso!", MsgBoxStyle.Information)
            'Me.Close()

        End If
    End Sub

    Sub INTERPOLAR() 'Não está sendo utilizada

        Dim multiplicador_min As Integer
        multiplicador_min = TextBox1.Text
        Dim expoente As Double
        expoente = TextBox2.Text

        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo

        ReDim Chuvabin(Total_MiniBacias, tmaxx)
        Dim somadist(Total_MiniBacias, tmaxx) As Integer
        For Mini As Integer = 0 To Total_MiniBacias
            For t As Integer = 0 To tmaxx
                Chuvabin(Mini, t) = 0
                somadist(Mini, t) = 0
            Next
        Next

        statusbar.Value = 0
        statusbar.Maximum = total_estações * Total_MiniBacias

        For Mini As Integer = 1 To Total_MiniBacias 'Para cada mini_bacia
            For p As Integer = 1 To total_estações 'Percorre todo o conjunto de estações
                If distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) <= multiplicador_min * Distância_Mínima(Mini) Then 'se a estação for próxima ao centróide da mini-bacia
                    Dim datacomeço As Date 'define a data inicial da interpolação
                    datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

                    For aux = 1 To tmaxx 'para cada intervalo de tempo que compõem este posto
                        If aux <> 1 Then
                            datacomeço = datacomeço.AddHours(1) 'adiciona o intervalo de tempo
                        End If

                        If Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then 'se existir chuva
                            'ele contabiliza a chuva neste intervalo de tempo
                            Chuvabin(Mini, aux) = Chuvabin(Mini, aux) + (Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) / (distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) ^ expoente))
                            somadist(Mini, aux) = somadist(Mini, aux) + (1 / (distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) ^ expoente))
                        End If
                    Next
                End If
                statusbar.Increment(1)
            Next
        Next

        'Agora finaliza a interpolação
        For Mini As Integer = 1 To Total_MiniBacias 'Para cada mini_bacia
            For aux = 1 To tmaxx  'para cada intervalo de tempo que compõem este posto
                Chuvabin(Mini, aux) = Chuvabin(Mini, aux) / somadist(Mini, aux)
            Next
        Next

        MsgBox("Interpolação Finalizada!", MsgBoxStyle.Information)

    End Sub

    Sub INTERPOLAR_V2()

        Dim multiplicador_min As Integer
        multiplicador_min = TextBox1.Text
        Dim expoente As Double
        expoente = TextBox2.Text

        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo

        ReDim Chuvabin(Total_MiniBacias, tmaxx)
        Dim somadist(Total_MiniBacias, tmaxx) As Single 'Double 'FMF 2018 06 23
        For Mini As Integer = 0 To Total_MiniBacias
            For t As Integer = 0 To tmaxx
                Chuvabin(Mini, t) = 0
                somadist(Mini, t) = 0
            Next
        Next

        statusbar.Value = 0
        statusbar.Maximum = tmaxx * Total_MiniBacias

        'Acerta a distância
        ReDim Distância_Mínima(Total_MiniBacias)
        For Mini As Integer = 1 To Total_MiniBacias
            Distância_Mínima(Mini) = 999999
        Next

        For Mini As Integer = 1 To Total_MiniBacias 'Para cada mini_bacia

            Dim datacomeço As Date 'define a data inicial da interpolação
            datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

            For aux = 1 To tmaxx 'para cada intervalo de tempo que compõem este posto
                If aux <> 1 Then
                    datacomeço = datacomeço.AddHours(1) 'adiciona o intervalo de tempo
                End If

                'Calcula a distãncia mínima!!
                Distância_Mínima(Mini) = 999999 'zera a variável de distãncia mínima
                For p As Integer = 1 To total_estações 'Percorre todo o conjunto de estações
                    If Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then 'se existir chuva
                        If distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) < Distância_Mínima(Mini) Then
                            Distância_Mínima(Mini) = distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p))
                        End If
                    End If
                Next
                'Calculou a distância mínima para este intervalo de tempo

                For p As Integer = 1 To total_estações 'Percorre todo o conjunto de estações
                    If distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) <= multiplicador_min * Distância_Mínima(Mini) Then 'se a estação for próxima ao centróide da mini-bacia
                        If Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then 'se existir chuva
                            'ele contabiliza a chuva neste intervalo de tempo
                            Chuvabin(Mini, aux) = Chuvabin(Mini, aux) + (Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) / (distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) ^ expoente))
                            somadist(Mini, aux) = somadist(Mini, aux) + (1 / (distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) ^ expoente))
                        End If
                    End If
                Next
                statusbar.Increment(1)
                If somadist(Mini, aux) > 0 Then
                    Chuvabin(Mini, aux) = Chuvabin(Mini, aux) / somadist(Mini, aux)
                ElseIf somadist(Mini, aux) <= 0 Then
                    If CheckBox1.Checked Then
                        MsgBox("Interpolação interrompida, não existem dados em " & Mini & datacomeço, MsgBoxStyle.Critical)
                    End If
                    Chuvabin(Mini, aux) = 0
                End If


            Next
        Next


        'MsgBox("Interpolação Finalizada!", MsgBoxStyle.Information)

    End Sub

    Sub INTERPOLAR_V4() 'Faz a interpolação por vizinho mais próximo

        Dim multiplicador_min As Integer
        multiplicador_min = TextBox1.Text
        Dim expoente As Double
        expoente = TextBox2.Text

        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo

        ReDim Chuvabin(Total_MiniBacias, tmaxx)
        Dim somadist As Double

        For Mini As Integer = 0 To Total_MiniBacias
            For t As Integer = 0 To tmaxx
                Chuvabin(Mini, t) = 0
            Next
        Next

        statusbar.Value = 0
        statusbar.Maximum = tmaxx * Total_MiniBacias

        'Acerta a distância
        Dim Distância_MínimaP As Double
        'For Mini As Integer = 1 To Total_MiniBacias
        '    Distância_Mínima(Mini) = 999999999
        'Next

        Dim PmaisPróximo As Integer

        For Mini As Integer = 1 To Total_MiniBacias 'Para cada mini_bacia

            Dim datacomeço As Date 'define a data inicial da interpolação
            datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            datacomeço = datacomeço.AddHours(-1)

            For aux = 1 To tmaxx 'para cada intervalo de tempo que compõem este posto

                datacomeço = datacomeço.AddHours(1) 'adiciona o intervalo de tempo

                'Calcula a distãncia mínima!!
                Distância_MínimaP = 99999999999 'zera a variável de distãncia mínima
                PmaisPróximo = -1

                For p As Integer = 1 To total_estações 'Percorre todo o conjunto de estações
                    If Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then 'se existir chuva
                        If distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) < Distância_MínimaP Then
                            Distância_MínimaP = distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p))
                            PmaisPróximo = p
                        End If
                    End If
                Next
                'Calculou a distância mínima para este intervalo de tempo, para este ponto

                If PmaisPróximo > 0 Then
                    Chuvabin(Mini, aux) = (Chuva(PmaisPróximo, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))
                ElseIf PmaisPróximo <= 0 Then
                    Chuvabin(Mini, aux) = 0
                End If

            Next
        Next


    End Sub

    Sub INTERPOLAR_V5() 'Faz a interpolação só a partir de onde não tem chuva já interpolada salva

        Dim multiplicador_min As Integer
        multiplicador_min = TextBox1.Text
        Dim expoente As Double
        expoente = TextBox2.Text

        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo

        'MsgBox("Vai dimensionar a variável de chuva")

        ReDim Chuvabin(Total_MiniBacias, tmaxx)

        For Mini As Integer = 0 To Total_MiniBacias
            For t As Integer = 0 To tmaxx
                Chuvabin(Mini, t) = 0
            Next
        Next

        '---Percorre as datas do chuta interpolada que existem
        Dim dataOexiste As Date = lbInterpDt1.Text
        Dim dataEexiste As Date = lbInterpDt2.Text

        Dim tmaxxExiste As Integer
        tmaxxExiste = (DateDiff(DateInterval.Hour, dataOexiste, dataEexiste) + 1)

        Dim datacomeçoExiste As Date 'define a data inicial da interpolação
        datacomeçoExiste = dataOexiste
        datacomeçoExiste = datacomeçoExiste.AddHours(-1)

        Dim contador_t As Integer
        contador_t = 0


        For t As Integer = 1 To tmaxxExiste
            datacomeçoExiste = datacomeçoExiste.AddHours(+1)

            If datacomeçoExiste >= dataO And datacomeçoExiste <= dataE Then
                contador_t += 1
                For Mini As Integer = 0 To Total_MiniBacias
                    Chuvabin(Mini, contador_t) = ChuvaInterpoladaLida(Mini, t)
                Next
            End If
        Next
        '---Terminou de percorrer as datas do chuta interpolada que existem

        'Se todo o intervalo já existir, encerra a subrotina
        If contador_t >= tmaxx Then Exit Sub

        statusbar.Value = 0
        statusbar.Maximum = tmaxx * Total_MiniBacias

        'Acerta a distância
        Dim Distância_MínimaP As Double
        'For Mini As Integer = 1 To Total_MiniBacias
        '    Distância_Mínima(Mini) = 999999999
        'Next

        Dim PmaisPróximo As Integer

        For Mini As Integer = 1 To Total_MiniBacias 'Para cada mini_bacia

            Dim datacomeço As Date 'define a data inicial da interpolação
            datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            datacomeço = datacomeço.AddHours(-1)
            datacomeço = datacomeço.AddHours(contador_t)

            For aux = (contador_t + 1) To tmaxx 'para cada intervalo de tempo que compõem este posto

                statusbar.Value += 1

                datacomeço = datacomeço.AddHours(1) 'adiciona o intervalo de tempo

                'Calcula a distãncia mínima!!
                Distância_MínimaP = 99999999999 'zera a variável de distãncia mínima
                PmaisPróximo = -1

                For p As Integer = 1 To total_estações 'Percorre todo o conjunto de estações
                    If Chuva(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then 'se existir chuva
                        If distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p)) < Distância_MínimaP Then
                            Distância_MínimaP = distância(X_MINI(Mini), Y_MINI(Mini), LON(p), LAT(p))
                            PmaisPróximo = p
                        End If
                    End If
                Next
                'Calculou a distância mínima para este intervalo de tempo, para este ponto

                If PmaisPróximo > 0 Then
                    Chuvabin(Mini, aux) = (Chuva(PmaisPróximo, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))
                ElseIf PmaisPróximo <= 0 Then
                    Chuvabin(Mini, aux) = 0
                End If

            Next
        Next


    End Sub


    Sub ESCREVER_METADADOS()
        'Calcula as datas
        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

        'Declara as variáveis
        Dim nome_metadados As String
        nome_metadados = System.IO.Path.GetFileNameWithoutExtension(sfd.FileName)
        Dim diretório As String
        diretório = System.IO.Path.GetDirectoryName(sfd.FileName)

        Dim arquivo_metadados As String
        arquivo_metadados = diretório & "\" & nome_metadados & ".meta"

        'Abre o arquivo e começa a escrever

        FileOpen(3, arquivo_metadados, OpenMode.Output)

        PrintLine(3, "!Metadados de Arquivo de Previsao de Chuva")
        PrintLine(3, "!Projeto")
        PrintLine(3, "Chuva Observada")
        PrintLine(3)
        PrintLine(3, "!Nome do Arquivo Binario")
        PrintLine(3, nome_metadados & ".mgb")
        PrintLine(3)
        PrintLine(3, "!Data de Inicio")
        PrintLine(3, "!ANO   MES   DIA  HORA   HORIZ(NT)")
        Print(3, dtpqob1.Value.Year & "    ")

        Dim mes As String
        mes = dtpqob1.Value.Month
        If mes.Length = 1 Then
            mes = "0" & mes
        End If
        Print(3, mes & "    ")

        Dim dia As String
        dia = dtpqob1.Value.Day
        If dia.Length = 1 Then
            dia = "0" & dia
        End If
        Print(3, dia & "    ")

        Dim hora As String
        hora = tb_hora_ini.Text
        If hora.Length = 1 Then
            hora = "0" & hora
        End If
        Print(3, hora)

        For branco As Integer = 1 To 7 - tmaxx.ToString.Length
            Print(3, " ")
        Next

        PrintLine(3, tmaxx)
        PrintLine(3)
        PrintLine(3, "!Numero de Mini-Bacias")
        PrintLine(3, Total_MiniBacias.ToString)
        PrintLine(3)
        PrintLine(3, "!Lista de coordenadas dos centroides")
        PrintLine(3, "!  IC	 LAT_dg    LONG_dg")

        For Mini As Integer = 1 To Total_MiniBacias
            For Branco As Integer = 1 To 5 - Mini.ToString.Length
                Print(3, " ")
            Next
            Print(3, Mini & "  ")
            Print(3, FormatNumber(Y_MINI(Mini), 5))
            Print(3, "  ")
            Print(3, FormatNumber(X_MINI(Mini), 5))
            PrintLine(3)
        Next

        FileClose()


    End Sub

    Sub SALVA_CHUVATEXTO()

        sfd.Filter = "Arquivo de chuva ASCII para verificação(.txt)|*.txt"

        If sfd.ShowDialog = Windows.Forms.DialogResult.OK Then

            Dim nomeArquivo As String
            nomeArquivo = sfd.FileName


            FileOpen(4, nomeArquivo, OpenMode.Output)

            Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

            For tempo As Integer = 1 To tmaxx
                For cells As Integer = 1 To Total_MiniBacias
                    Dim numero As Single
                    numero = Chuvabin(cells, tempo)
                    Print(4, "    " & FormatNumber(numero, 2))
                    'MsgBox(cells & " " & tempo & " " & chuvabin(cells, tempo))
                Next
                PrintLine(4)
            Next

            FileClose()

            MsgBox("Arquivo salvo com sucesso!", MsgBoxStyle.Information)


        End If

    End Sub

    Sub SALVAR_DATABASECHUVABIN_AUTOMÁTICO()

        Dim ArquivoChuvaInterpolada As String
        ArquivoChuvaInterpolada = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\ChuvaInterpolada\CHUVA_OBS.MGB"
        'Declara as variáveis
        Dim nome_metadados As String
        nome_metadados = System.IO.Path.GetFileNameWithoutExtension(ArquivoChuvaInterpolada)
        Dim diretório As String
        diretório = System.IO.Path.GetDirectoryName(ArquivoChuvaInterpolada)

        Dim arquivo_metadados As String
        arquivo_metadados = diretório & "\" & nome_metadados & ".meta"


        'sfd.Filter = "Arquivo de chuva binário para o MGB(.MGB)|*.MGB"

        'If sfd.ShowDialog = Windows.Forms.DialogResult.OK Then

        ' sfd.FileName = diretorio_previsao & "\CHUVA_OBS.MGB"

        Dim nomeArquivo As String
        nomeArquivo = ArquivoChuvaInterpolada

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Create, FileAccess.ReadWrite)
        Dim bw As New System.IO.BinaryWriter(fs)


        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim dataOexiste As Date = lbInterpDt1.Text
        Dim dataEexiste As Date = lbInterpDt2.Text

        Dim tmaxx1 As Integer
        tmaxx1 = (DateDiff(DateInterval.Hour, dataOexiste, dataO))

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)


        For tempo As Integer = 1 To (tmaxx + tmaxx1)
            For cells As Integer = 1 To Total_MiniBacias
                Dim numero As Single
                If tempo <= tmaxx1 Then
                    numero = ChuvaInterpoladaLida(cells, tempo)
                ElseIf tempo > tmaxx1 Then
                    numero = Chuvabin(cells, tempo - tmaxx1)
                End If

                bw.Write(numero)
                'MsgBox(cells & " " & tempo & " " & chuvabin(cells, tempo))
            Next
        Next

        bw.Close()


        Dim dataInicial As Date
        dataInicial = lbInterpDt1.Text

        FileOpen(3, arquivo_metadados, OpenMode.Output)

        PrintLine(3, "!Metadados de Arquivo de Previsao de Chuva")
        PrintLine(3, "!Projeto")
        PrintLine(3, "Chuva Observada")
        PrintLine(3)
        PrintLine(3, "!Nome do Arquivo Binario")
        PrintLine(3, nome_metadados & ".mgb")
        PrintLine(3)
        PrintLine(3, "!Data de Inicio")
        PrintLine(3, "!ANO   MES   DIA  HORA   HORIZ(NT)")
        Print(3, dataInicial.Year & "    ")

        Dim mes As String
        mes = dataInicial.Month
        If mes.Length = 1 Then
            mes = "0" & mes
        End If
        Print(3, mes & "    ")

        Dim dia As String
        dia = dataInicial.Day
        If dia.Length = 1 Then
            dia = "0" & dia
        End If
        Print(3, dia & "    ")

        Dim hora As String
        hora = dataInicial.Hour
        If hora.Length = 1 Then
            hora = "0" & hora
        End If
        Print(3, hora)
        Print(3, "   ")
        Dim tempo_somado As Integer
        tempo_somado = tmaxx + tmaxx1
        PrintLine(3, tempo_somado.ToString.PadRight(20))
        PrintLine(3)
        PrintLine(3, "!Numero de Mini-Bacias")
        PrintLine(3, Total_MiniBacias.ToString)
        PrintLine(3)
        PrintLine(3, "!Lista de coordenadas dos centroides")
        PrintLine(3, "!  IC	 LAT_dg    LONG_dg")

        For Mini As Integer = 1 To Total_MiniBacias
            For Branco As Integer = 1 To 5 - Mini.ToString.Length
                Print(3, " ")
            Next
            Print(3, Mini & "  ")
            Print(3, FormatNumber(Y_MINI(Mini), 5))
            Print(3, "  ")
            Print(3, FormatNumber(X_MINI(Mini), 5))
            PrintLine(3)
        Next

        FileClose()


        'End If
    End Sub

    Private Sub Button7_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button7.Click
        Dim arquivo_est2 As String
        Dim num As Integer

        '--Nova rotina em 11ago26 - FMF
        '--------------------------------------------------------
        'Descobre qual estação está selecionada no DataGridView
        '--------------------------------------------------------
        If DataGridView1.CurrentRow Is Nothing Then
            MsgBox("Nenhuma estação foi selecionada.")
            Exit Sub
        End If

        Dim codigoEstacao As String
        codigoEstacao = DataGridView1.CurrentRow.Cells(0).Value.ToString.Trim

        'Procura a estação na base original
        Dim est As Integer
        est = LOCALIZA_ESTACAO_NA_BASE(codigoEstacao)

        If est = -1 Then
            MsgBox("A estação selecionada não foi encontrada na base de dados.")
            Exit Sub
        End If

        'Agora usa o índice correto da estação,
        'independentemente da ordenação do DataGridView
        num = est
        '--Fim nova rotina em 11ago26 - FMF 

        '--Deligado em 11ago26 - FMF 
        'num = DataGridView1.CurrentCell.RowIndex + 1

        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\P" & Estação(num) & ".txt"
        'System.Diagnostics.Process.Start("notepad.exe", arquivo_est2)


        ReDim Tabela(24, 31, 12, Ano(AF))

        Tb_chuva = True
        Tb_vazao = False
        Nome_arq = arquivo_est2
        Nome_p = NOME(num)

        For aaaa = AI To AF
            For mm = 1 To 12
                For dd = 1 To 31
                    For hh As Integer = 0 To 23
                        Tabela(hh, dd, mm, Ano(aaaa)) = Chuva(num, hh, dd, mm, Ano(aaaa))
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        dataI = dataIp(num)
        dataF = dataFp(num)

        Dim frmedit As New Form9
        frmedit.Show()


    End Sub

    Sub CARREGA_MINI()
        'Carrega o arquivo MINI.MGB de forma automática
        Dim arquivo_MINI As String 'localização do arquivo de mini bacias
        'Existe uma cópia do MINI dentro do arquivo Input do plugin
        arquivo_MINI = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\MINI.MGB"

        'ofd.Multiselect = False
        'OFD.Filter = "Arquivo com dados de geometria do MGB(.MGB)|*.MGB"
        'ofd.Filter = "Arquivo com as mini-bacias para o MGB(.mgb;.txt)|*.mgb;*.txt"
        'If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
        'Seleciona a caixa de texto como a localização do MINI
        tb_mini.Text = arquivo_MINI
        numcels = -1
        FileOpen(1, tb_mini.Text, OpenMode.Input)
        Do Until EOF(1)
            linha = LineInput(1)
            numcels = numcels + 1
            If numcels >= 1 Then
            End If
        Loop
        FileClose()

        Total_MiniBacias = numcels
        ReDim X_MINI(Total_MiniBacias)
        ReDim Y_MINI(Total_MiniBacias)

        numcels = -1
        FileOpen(1, tb_mini.Text, OpenMode.Input)
        Do Until EOF(1)
            linha = LineInput(1)
            numcels = numcels + 1
            If numcels >= 1 Then
                X_MINI(numcels) = AjustarDecimal(linha.Substring(22, 9))
                Y_MINI(numcels) = AjustarDecimal(linha.Substring(37, 9))
            End If
        Loop
        FileClose()

        Me.VERIFICA_D_MIN() 'Verifica as distâncias mínimas dos centróides

        'MsgBox("Leitura do arquivo Mini.MGB realizada com sucesso!", MsgBoxStyle.Information)
        Button6.Enabled = True
        'End If

    End Sub

    Sub CARREGA_DIRETÓRIOS()
        'Verifica base de dados interna
        Dim arquivo_est As String 'no do arquivo com as configurações
        Dim total_estações As Integer 'variável que faz a contagem das linhas
        Dim linha As String 'variável string que contém a linha
        total_estações = 0 'zera o contador de estações
        'determina o local do arquivo que contém os diretórios
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\config.txt"
        'abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Input)
        'faz a leitura do arquiv, linha por linha
        Do Until EOF(1)
            linha = LineInput(1)
            total_estações = total_estações + 1
            'Quando chegar na linha 7, faz a leitura do diretório onde será salvo o arquivo da previsão
            If total_estações = 1 Then
                diretorio_arquivos = linha
            ElseIf total_estações = 4 Then
                diretorio_previsao = linha
            ElseIf total_estações = 9 Then
                diretorio_arquivos2 = linha
            ElseIf total_estações = 10 Then
                diretorio_arquivos3 = linha
            End If
        Loop

        stlb.Text = "Diretório arquivos de previsão: " & diretorio_previsao
        'Fecha
        FileClose()

    End Sub

    Sub SALVAR_CHUVABIN_AUTOMÁTICO()

        'sfd.Filter = "Arquivo de chuva binário para o MGB(.MGB)|*.MGB"

        'If sfd.ShowDialog = Windows.Forms.DialogResult.OK Then

        sfd.FileName = diretorio_previsao & "\CHUVA_OBS.MGB"

        Dim nomeArquivo As String
        nomeArquivo = sfd.FileName

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Create, FileAccess.ReadWrite)
        Dim bw As New System.IO.BinaryWriter(fs)


        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

        For tempo As Integer = 1 To tmaxx
            For cells As Integer = 1 To Total_MiniBacias
                Dim numero As Single
                numero = Chuvabin(cells, tempo)
                bw.Write(numero)
                'MsgBox(cells & " " & tempo & " " & chuvabin(cells, tempo))
            Next
        Next

        bw.Close()

        'MsgBox("Arquivo salvo com sucesso!", MsgBoxStyle.Information)
        'Me.Close()

        'End If
    End Sub

    Private Sub Button8_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button8.Click
        Me.ATUALIZA_EDITADOS_MANUALMENTE()
        Me.AtualizaDataTabela()
    End Sub

    Sub ATUALIZA_EDITADOS_MANUALMENTE()

        Try
            'Verifica quantas estações existem na base de dados interna
            Dim arquivo_est As String 'nome do arquivo que contém as estações
            Dim linha As String 'linha de leitura
            total_estações = 0 'número de estações

            'define o arquivo
            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesPluv.txt"

            'Abre e faz a leituara
            FileOpen(1, arquivo_est, OpenMode.Input)
            Do Until EOF(1)
                linha = LineInput(1)
                total_estações = total_estações + 1
            Loop
            FileClose()
            total_estações = total_estações - 1 'desconta a primeira linha, que é cabeçalho
            '-- Fim da verificação do total de estações

            'Redimensiona as variáveis
            'ReDim ID(total_estações)
            'ReDim Estação(total_estações)
            'ReDim LAT(total_estações), LON(total_estações), ALT(total_estações)
            'ReDim NOME(total_estações)
            'ReDim A(total_estações), B(total_estações), C(total_estações)
            'ReDim Chuva(total_estações, 24, 31, 12, Ano(AF))
            'ReDim dataIp(total_estações)
            'ReDim dataFp(total_estações)
            'ReDim HoraIp(total_estações)
            'ReDim HoraFp(total_estações)
            '--

            'Zera as variáveis de data
            'For p As Integer = 0 To total_estações
            ' dataIp(p) = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
            ' dataFp(p) = "01/01/" & AI & " " & "00:00:00"
            ' HoraIp(p) = 23
            ' HoraFp(p) = 0
            ' Next
            '--

            'Zera as variáveis de vazão e nível
            For p As Integer = 0 To total_estações
                For aaaa = AI To AF
                    For mm = 1 To 12
                        For dd = 1 To 31
                            For hh As Integer = 0 To 23
                                'Chuva(p, hh, dd, mm, Ano(aaaa)) = -1.0
                            Next
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis

            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesPluv.txt"
            FileOpen(1, arquivo_est, OpenMode.Input)
            Dim est As Integer
            est = -1

            Do Until EOF(1)

                linha = LineInput(1)
                est = est + 1
                If est > 0 Then

                    Dim splt
                    splt = Split(linha, ";")

                    ID(est) = splt(0)
                    Estação(est) = splt(1)
                    LAT(est) = AjustarDecimal(splt(2))
                    LON(est) = AjustarDecimal(splt(3))
                    ALT(est) = AjustarDecimal(splt(4))
                    NOME(est) = splt(5)
                    A(est) = AjustarDecimal(splt(6))
                    B(est) = AjustarDecimal(splt(7))
                    C(est) = AjustarDecimal(splt(8))




                    '-sesativado em 21 de outubro 2025  Me.LE_DATABASE(est, Estação(est))
                    Me.LE_DATABASE_Fast(est, Estação(est))

                    'Adiciona os dados à tabela

                    'Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

                    ' Preenche os valores do registro
                    'nova_linha.ItemArray = New Object() {Estação(est), dataIp(est), dataFp(est), LON(est), LAT(est), NOME(est)}
                    'Try
                    ' DataSet1.Tables(0).Rows.Add(nova_linha)
                    ' Catch ex As Exception
                    ' MsgBox(" erro: " & ex.Message)
                    ' End Try

                End If
            Loop

            FileClose()
            'Carregou dados internos

        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

    End Sub

    Sub BOTAO1ANTIGO()

        ofd.Multiselect = True
        ofd.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            Dim s As String
            Dim pmax As Integer
            pmax = ofd.FileNames.Length
            Dim Arqu(pmax) As String
            Dim p As Integer
            p = 0

            statusbar.Maximum = pmax
            statusbar.Value = 0



            For Each s In ofd.FileNames
                p = p + 1
                Arqu(p) = s

                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)).Substring(14, 9)

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = "P" & Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo
                    Dim linha4 As String
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        'Lê a chuva
                        Chuva(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = linha4.Substring(26, 10)

                        If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                            dataIp(CodigoLocalizado) = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            If dataIp(CodigoLocalizado) < dataI Then
                                dataI = dataIp(CodigoLocalizado)
                            End If
                        End If

                        If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                            dataFp(CodigoLocalizado) = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            If dataFp(CodigoLocalizado) > dataF Then
                                dataF = dataFp(CodigoLocalizado)
                            End If
                        End If


                    Loop
                    FileClose(4)



                End If


                statusbar.Increment(1)
            Next

            Me.AtualizaDataTabela()
            Me.ATUALIZA_ARQUIVOS_CHUVA()
            Me.ATUALIZA_DATA()
            ToolStripStatusLabel1.Text = "Atualização finalizada"


        End If


    End Sub

    Sub ATUALIZA_AUTOMATICAMENTE()
        'Abre o arquivo de estações e percorre uma por uma
        Dim arquivo_est As String
        Dim linha_est As String

        Dim pmax As Integer
        pmax = 18
        Dim Arqu(pmax) As String
        Dim p As Integer
        p = 0

        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Estações.txt"
        FileOpen(15, arquivo_est, OpenMode.Input)
        Do Until EOF(15)
            linha_est = LineInput(15)
            Dim s As String
            s = diretorio_arquivos & "\" & linha_est & ".txt"

            p = p + 1
            Arqu(p) = s

            Try
                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) '.Substring(0, 8)

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo
                    Dim linha4 As String
                    Dim pl As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        'Lê a chuva
                        Try
                            pl = AjustarDecimal(linha4.Substring(20, 10))
                        Catch ex As Exception
                            pl = -1
                        End Try

                        If pl >= 0 And pl < 120 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = pl
                        ElseIf pl < 0 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        ElseIf pl >= 120 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                            dataIp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataIp(CodigoLocalizado) < dataI Then
                                dataI = dataIp(CodigoLocalizado)
                            End If
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                            dataFp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataFp(CodigoLocalizado) > dataF Then
                                dataF = dataFp(CodigoLocalizado)
                            End If
                        End If


                    Loop
                    FileClose(4)



                End If


                statusbar.Increment(1)
            Catch ex As Exception

            End Try




        Loop
        FileClose(15)
        '-- Fim da verificação do total de estações

        Me.AtualizaDataTabela()
        Me.ATUALIZA_ARQUIVOS_CHUVA()
        Me.ATUALIZA_DATA()
        ToolStripStatusLabel1.Text = "Dados atualizados automaticamente em " & Now.Date
        statusbar.Value = 0




    End Sub

    Sub ATUALIZA_AUTOMATICAMENTE2()
        'Abre o arquivo de estações e percorre uma por uma
        Dim arquivo_est As String
        Dim linha_est As String

        Dim pmax As Integer
        pmax = 18
        Dim Arqu(pmax) As String
        Dim p As Integer
        p = 0

        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Estações.txt"
        FileOpen(15, arquivo_est, OpenMode.Input)
        Do Until EOF(15)
            linha_est = LineInput(15)
            Dim s As String
            s = diretorio_arquivos2 & "\" & linha_est & ".txt"

            p = p + 1
            Arqu(p) = s

            Try
                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) '.Substring(0, 8)

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo
                    Dim linha4 As String
                    Dim pl As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        'Lê a chuva
                        Try
                            pl = AjustarDecimal(linha4.Substring(20, 10))
                        Catch ex As Exception
                            pl = -1
                        End Try

                        If pl >= 0 And pl < 120 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = pl
                        ElseIf pl < 0 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        ElseIf pl >= 120 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                            dataIp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataIp(CodigoLocalizado) < dataI Then
                                dataI = dataIp(CodigoLocalizado)
                            End If
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                            dataFp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataFp(CodigoLocalizado) > dataF Then
                                dataF = dataFp(CodigoLocalizado)
                            End If
                        End If


                    Loop
                    FileClose(4)



                End If


                statusbar.Increment(1)
            Catch ex As Exception

            End Try




        Loop
        FileClose(15)
        '-- Fim da verificação do total de estações

        Me.AtualizaDataTabela()
        Me.ATUALIZA_ARQUIVOS_CHUVA()
        Me.ATUALIZA_DATA()
        ToolStripStatusLabel1.Text = "Dados atualizados automaticamente em " & Now.Date
        statusbar.Value = 0




    End Sub

    Sub ATUALIZA_AUTOMATICAMENTE3()
        'Abre o arquivo de estações e percorre uma por uma
        Dim arquivo_est As String
        Dim linha_est As String

        Dim pmax As Integer
        pmax = 18
        Dim Arqu(pmax) As String
        Dim p As Integer
        p = 0

        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Estações.txt"
        FileOpen(15, arquivo_est, OpenMode.Input)
        Do Until EOF(15)
            linha_est = LineInput(15)
            Dim s As String
            s = diretorio_arquivos3 & "\" & linha_est & ".txt"

            p = p + 1
            Arqu(p) = s

            Try
                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) '.Substring(0, 8)

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo
                    Dim linha4 As String
                    Dim pl As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        'Lê a chuva
                        Try
                            pl = AjustarDecimal(linha4.Substring(20, 10))
                        Catch ex As Exception
                            pl = -1
                        End Try

                        If pl >= 0 And pl < 120 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = pl
                        ElseIf pl < 0 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        ElseIf pl >= 120 Then
                            Chuva(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                            dataIp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataIp(CodigoLocalizado) < dataI Then
                                dataI = dataIp(CodigoLocalizado)
                            End If
                        End If

                        If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                            dataFp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                            If dataFp(CodigoLocalizado) > dataF Then
                                dataF = dataFp(CodigoLocalizado)
                            End If
                        End If


                    Loop
                    FileClose(4)



                End If


                statusbar.Increment(1)
            Catch ex As Exception

            End Try




        Loop
        FileClose(15)
        '-- Fim da verificação do total de estações

        Me.AtualizaDataTabela()
        Me.ATUALIZA_ARQUIVOS_CHUVA()
        Me.ATUALIZA_DATA()
        ToolStripStatusLabel1.Text = "Dados atualizados automaticamente em " & Now.Date
        statusbar.Value = 0




    End Sub

#Region "Atualizar Online"

    Sub ATUALIZAR_ONLINE()
        'Avisa que está atualizando
        ToolStripStatusLabel1.Text = "Conectando e fazendo download..."

        'Zera a barra de status
        statusbar.Maximum = total_estações
        statusbar.Value = 0

        'Para cada uma das estações na tabela, em ordem, executa as rotinas de download
        For DadoEstação As Integer = 1 To total_estações
            'A data inicial de download é a final de dados do arquivo
            Dim data_começo As Date = dataFp(DadoEstação).Day & "/" & dataFp(DadoEstação).Month & "/" & dataFp(DadoEstação).Year & " " & dataFp(DadoEstação).Hour & ":00:00"

            'A data final de download é sempre o momento (agora)
            Dim data_final As Date = Now.Day & "/" & Now.Month & "/" & Now.Year & " " & Now.Hour & ":00:00"

            If Estação(DadoEstação).ToString.Length < 7 Then
                'ele pula aqui a tentativa de download de dados que não código do tamanho da ANA
                Continue For
                'Adiciona valor na barra de status
                statusbar.Increment(1)
            End If

            'Zera a chuva da estação do dia mais recente que tem dados, pois a rotina vai re-integrar os dados deste dia
            For hora As Integer = 0 To 24
                Chuva(DadoEstação, hora, data_começo.Day, data_começo.Month, Ano(data_começo.Year)) = -1
            Next

            'Executa a rotina de download
            Download_WebService(Estação(DadoEstação), data_começo, data_final, DadoEstação)

            'Adiciona valor na barra de status
            statusbar.Increment(1)
        Next

        'Executa as rotinas de atualização da base de dados
        Me.AtualizaDataTabela()
        ' Me.ATUALIZA_ARQUIVOS_CHUVA() 'desligada em 20 de outubro por ser mais lenta 2025
        Me.ATUALIZA_ARQUIVOS_CHUVA_Legado() 'nova rotina mais rápida

        Me.ATUALIZA_DATA()
        ToolStripStatusLabel1.Text = "Atualização finalizada"
    End Sub

    Sub Download_WebService(ByVal CodEstação As String, ByVal datai As Date, ByVal dataf As Date, ByVal posição As Integer) 'Faz download do webservice da ANA

        'define a data inicial e final de download, para a aquisição dos dados
        Dim dataO As Date = datai.Day & "/" & datai.Month & "/" & datai.Year & " " & datai.Hour & ":00:00"
        Dim dataE As Date = dataf.Day & "/" & dataf.Month & "/" & dataf.Year & " " & dataf.Hour & ":00:00"

        'número de dias dentro do intervalo
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        'Cria os valores de dia, mes e ano da maneira correta
        Dim dddi, dddf, mmmi, mmmf As String
        dddi = dataO.Day
        If dddi.ToString.Length < 2 Then
            dddi = "0" & dddi
        End If

        mmmi = dataO.Month
        If mmmi.ToString.Length < 2 Then
            mmmi = "0" & mmmi
        End If

        dddf = dataE.Day
        If dddf.ToString.Length < 2 Then
            dddf = "0" & dddf
        End If

        mmmf = dataE.Month
        If mmmf.ToString.Length < 2 Then
            mmmf = "0" & mmmf
        End If
        'Pegou os valores com o tamanho certo


        'informa o endereço do arquivo de cotas
        Dim arquivoFonte As String
        arquivoFonte = "http://telemetriaws1.ana.gov.br/serviceana.asmx/" & "DadosHidrometeorologicos?codEstacao=" & CodEstação & "&dataInicio=" & dddi & "/" & mmmi & "/" & dataO.Year & "&dataFim=" & dddf & "/" & mmmf & "/" & dataE.Year

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        Requisitar_TimeOut(arquivoFonte, CodEstação)
        Converter_Requisitado(CodEstação, posição)

    End Sub

    Sub Requisitar(ByVal arquivoFonte As String, ByVal CódigoEstação As String) 'Baixa o XML para diretório determinado

        'Pega a url do arquivo que deve ser baixado no webservice
        Dim url As String
        url = arquivoFonte

        'Cria o nome do arquivo que deve ser baixado
        Dim ArquivoGerado As String
        ArquivoGerado = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Telemetria\" & CódigoEstação & ".xml"

        ' ''Cria uma requisição com as credenciais do site
        ''Dim fWr As HttpWebRequest = DirectCast(WebRequest.Create(url), HttpWebRequest)
        ' '' fWr.Credentials = New NetworkCredential("supervisao@uhelajeado.com.br", "<REDACTED>")

        ' ''Resposta do http fornecido
        ''Dim response As HttpWebResponse = fWr.GetResponse

        ' ''Lê o script retornado com função de obtenção dos dados no database do site
        ''Dim inStream As New IO.StreamReader(response.GetResponseStream(), System.Text.Encoding.Default, True)

        ' ''Cria uma variável do tipo string onde salva todas as informações do script do site
        ''Dim l As String = inStream.ReadToEnd

        ' ''Abre o arquivo e salva todo o script dentro dele
        ''FileOpen(10, ArquivoGerado, OpenMode.Output)
        ''Print(10, l)
        ''FileClose(10)

        Dim request As WebRequest = _
              WebRequest.Create(url)
        ' If required by the server, set the credentials.  
        request.Credentials = CredentialCache.DefaultCredentials
        ' Get the response.  
        Dim response As WebResponse = request.GetResponse()
        ' Display the status.  
        Console.WriteLine(CType(response, HttpWebResponse).StatusDescription)
        ' Get the stream containing content returned by the server.  
        Dim dataStream As Stream = response.GetResponseStream()
        ' Open the stream using a StreamReader for easy access.  
        Dim reader As New StreamReader(dataStream)
        ' Read the content.  
        Dim responseFromServer As String = reader.ReadToEnd()
        ' Display the content.  
        Console.WriteLine(responseFromServer)
        ' Clean up the streams and the response.  
        reader.Close()
        response.Close()

        'Abre o arquivo e salva todo o script dentro dele
        FileOpen(10, ArquivoGerado, OpenMode.Output)
        Print(10, responseFromServer)
        FileClose(10)

        'Terminou a rotina

        'Terminou a rotina
    End Sub

    Sub Requisitar_TimeOut(ByVal arquivoFonte As String, ByVal CódigoEstação As String,
               Optional tentativas As Integer = 5,
               Optional timeoutMs As Integer = 120000,
               Optional readWriteTimeoutMs As Integer = 120000)

        ' Ajustes globais razoáveis (ok chamar mais de uma vez)
        ServicePointManager.DefaultConnectionLimit = Math.Max(20, ServicePointManager.DefaultConnectionLimit)
        ServicePointManager.Expect100Continue = False
        ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12 Or SecurityProtocolType.Tls13

        ' Pega a url do arquivo que deve ser baixado no webservice
        Dim url As String = arquivoFonte

        ' Caminho de saída
        Dim pastaSaida As String = Path.Combine(Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly().Location), "Telemetria")
        Dim arquivoDestino As String = Path.Combine(pastaSaida, CódigoEstação & ".xml")
        Dim arquivoTmp As String = Path.Combine(pastaSaida, CódigoEstação & ".tmp")

        ' Garante a pasta
        If Not Directory.Exists(pastaSaida) Then Directory.CreateDirectory(pastaSaida)

        Dim ultimaEx As Exception = Nothing

        For tentativa As Integer = 1 To Math.Max(1, tentativas)
            Dim req As HttpWebRequest = CType(WebRequest.Create(url), HttpWebRequest)
            req.Method = "GET"
            req.Credentials = CredentialCache.DefaultCredentials           ' mantém do seu código
            req.Timeout = timeoutMs                                        ' conectar + esperar cabeçalhos
            req.ReadWriteTimeout = readWriteTimeoutMs                      ' intervalo entre leituras do stream
            req.KeepAlive = False                                          ' evita reuso de conexão ruim
            req.Proxy = Nothing                                            ' se precisar de proxy, remova esta linha
            req.AutomaticDecompression = DecompressionMethods.GZip Or DecompressionMethods.Deflate
            req.UserAgent = "MeuApp/1.0"

            Try
                Using resp As HttpWebResponse = CType(req.GetResponse(), HttpWebResponse)
                    If resp.StatusCode <> HttpStatusCode.OK Then
                        Throw New WebException($"HTTP {(CInt(resp.StatusCode))} - {resp.StatusDescription}")
                    End If

                    ' Grava direto em arquivo temporário e move (swap atômico)
                    Using respStream As Stream = resp.GetResponseStream(),
                      fs As New FileStream(arquivoTmp, FileMode.Create, FileAccess.Write, FileShare.None, 1 << 20)
                        respStream.CopyTo(fs)
                    End Using
                End Using

                ' Move/renomeia de .tmp para .xml (substitui se existir)
                If File.Exists(arquivoDestino) Then File.Delete(arquivoDestino)
                File.Move(arquivoTmp, arquivoDestino)

                ' Opcional: log de sucesso
                ' Console.WriteLine("Download OK: " & arquivoDestino)
                Exit Sub

            Catch ex As WebException When _
            ex.Status = WebExceptionStatus.Timeout OrElse
            ex.Status = WebExceptionStatus.ConnectFailure OrElse
            ex.Status = WebExceptionStatus.ReceiveFailure OrElse
            ex.Status = WebExceptionStatus.NameResolutionFailure OrElse
            ex.Status = WebExceptionStatus.ProxyNameResolutionFailure OrElse
            ex.Status = WebExceptionStatus.KeepAliveFailure

                ' Erros transitórios → backoff e tenta de novo
                ultimaEx = ex
                Try : If File.Exists(arquivoTmp) Then File.Delete(arquivoTmp) : 

                Catch : End Try
                Dim esperaMs As Integer = CInt(750 * Math.Pow(1.8, tentativa - 1)) ' 750ms, 1350ms, 2430ms...
                Thread.Sleep(esperaMs)

            Catch ex As Exception
                ' Erro não-transitório → limpe tmp e propague (ou trate como quiser)
                Try : If File.Exists(arquivoTmp) Then File.Delete(arquivoTmp) : 

                Catch : End Try
                Throw
            End Try
        Next

        ' Esgotou tentativas
        Throw New TimeoutException($"Falha ao baixar {url} após {tentativas} tentativas.", ultimaEx)
    End Sub


    Sub Converter_Requisitado(ByVal CódigoEstação As String, ByVal numposição As Integer)
        'Cria o nome do arquivo que foi baixado
        Dim ArquivoGerado As String
        ArquivoGerado = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Telemetria\" & CódigoEstação & ".xml"

        'Variáveis necessárias para dimensionamento
        Dim hh As Integer 'hora
        Dim mes As Integer 'meses
        Dim dia As Integer 'dias
        Dim year As Integer 'anos

        'Define os valores das variáveis
        hh = 24
        mes = 12
        dia = 31
        year = AF

        Dim reader As XmlTextReader = New XmlTextReader(ArquivoGerado)
        Dim data, valor, vazao, chuvas, nivel As Integer

        Try
            Do While (reader.Read())
                Select Case reader.NodeType
                    Case XmlNodeType.Element 'Exibir o início do elemento.

                        'Verifica se o elemento lido é a data, dado de chuva, vazão, ou nível
                        If reader.Name = "DataHora" Then
                            data = 1
                            valor = 0
                            vazao = 0
                            chuvas = 0
                            nivel = 0
                        ElseIf reader.Name = "Chuva" Then
                            data = 0
                            valor = 0
                            vazao = 0
                            chuvas = 1
                            nivel = 0
                        ElseIf reader.Name = "Vazao" Then
                            data = 0
                            valor = 0
                            vazao = 1
                            chuvas = 0
                            nivel = 0
                        ElseIf reader.Name = "Nivel" Then
                            data = 0
                            valor = 0
                            vazao = 0
                            chuvas = 0
                            nivel = 1
                        End If

                    Case XmlNodeType.Text 'Exibir o texto em cada elemento.
                        If data = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuvas = 0
                            nivel = 0
                            hh = reader.Value.Substring(11, 2)
                            mes = reader.Value.Substring(5, 2)
                            dia = reader.Value.Substring(8, 2)
                            year = reader.Value.Substring(0, 4)

                        ElseIf valor = 1 Then
                            'data = 0
                            'valor = 0
                            'PrintLine(1, reader.Value)
                        ElseIf vazao = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuvas = 0
                            nivel = 0
                        ElseIf chuvas = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuvas = 0
                            nivel = 0

                            If Chuva(numposição, hh, dia, mes, Ano(year)) < 0 Then
                                Chuva(numposição, hh, dia, mes, Ano(year)) = 0
                            End If

                            'Chuva(numposição, hh, dia, mes, Ano(year)) = 0 'reader.Value

                            If Chuva(numposição, hh, dia, mes, Ano(year)) > 1000 Then
                                'MsgBox("Chuva = " & reader.Value)
                                Chuva(numposição, hh, dia, mes, Ano(year)) = -1.0
                            Else
                                Chuva(numposição, hh, dia, mes, Ano(year)) += AjustarDecimal(reader.Value)
                            End If

                            If Chuva(numposição, hh, dia, mes, Ano(year)) >= 0 Then
                                'Faz o acerto das datas
                                If dia & "/" & mes & "/" & year & " " & hh & ":00:00" < dataIp(numposição) Then
                                    dataIp(numposição) = dia & "/" & mes & "/" & year & " " & hh & ":00:00"
                                    If dataIp(numposição) < dataI Then
                                        dataI = dataIp(numposição)
                                    End If
                                End If

                                If dia & "/" & mes & "/" & year & " " & hh & ":00:00" > dataFp(numposição) Then
                                    dataFp(numposição) = dia & "/" & mes & "/" & year & " " & hh & ":00:00"
                                    If dataFp(numposição) > dataF Then
                                        dataF = dataFp(numposição)
                                    End If
                                End If

                                If dataFp(numposição) > Now.Day & "/" & Now.Month & "/" & Now.Year & " " & Now.Hour & ":00:00" Then
                                    dataFp(numposição) = Now.Day & "/" & Now.Month & "/" & Now.Year & " " & Now.Hour & ":00:00"
                                    dataF = Now.Day & "/" & Now.Month & "/" & Now.Year & " " & Now.Hour & ":00:00"
                                End If

                            End If

                        ElseIf nivel = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuvas = 0
                            nivel = 0
                        End If
                        'MsgBox(reader.Value)
                    Case XmlNodeType.EndElement 'Exibir o fim do elemento.
                        'MsgBox("</" + reader.Name)
                        'MsgBox(">")
                End Select
            Loop
        Catch ex As Exception

        End Try



    End Sub

#End Region

    Private Sub Button9_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button9.Click
        Me.ATUALIZAR_ONLINE()

    End Sub

    Private Sub Button10_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button10.Click
        LOAD_SOMAR_FILES()
        ProcessSOMARFiles()
        MESCLAR_SOMAR_CHUVALIDA()
        RECALCULA_DATA_FINAL_SOMAR()
    End Sub

    Sub LOAD_SOMAR_FILES()
        'opendiag.Filter = "Arquivo com chuva interpolada (.mgb;.hig)|*.mgb;*.hig"
        opendiagSOMAR.Reset()
        'lbSOMARfiles.Items.Clear()

        opendiagSOMAR.Filter = "Arquivo com chuva em uma hora da SOMAR (.txt;.plu;.asc)|*.txt;*.plu;*.asc"
        opendiagSOMAR.Multiselect = True
        Dim ArquivoSolo As String
        If opendiagSOMAR.ShowDialog = Windows.Forms.DialogResult.OK Then
            For Each s As String In opendiagSOMAR.FileNames
                ArquivoSolo = System.IO.Path.GetFileName(s)
                'lbSOMARfiles.Items.Add(ArquivoSolo)
            Next
        End If

        'ProcessSOMARFiles()
    End Sub

    Sub ProcessSOMARFiles()
        DataIsomar = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
        DataFsomar = "01/01/" & AI & " " & "00:00:00"

        Dim FileNameDate As String
        Dim DateFileName As Date
        Dim NC As Integer
        NC = Total_MiniBacias

        For Each s As String In opendiagSOMAR.FileNames
            FileNameDate = System.IO.Path.GetFileNameWithoutExtension(s)
            DateFileName = ConverteDataString_to_Date_Hour(FileNameDate)
            If DateFileName < DataIsomar Then
                DataIsomar = DateFileName
            End If
            If DateFileName > DataFsomar Then
                DataFsomar = DateFileName
            End If
        Next

        ' ''tbHSOMARi.Text = DataIsomar.Hour
        ' ''If tbHSOMARi.Text.Length = 1 Then tbHSOMARi.Text = "0" & tbHSOMARi.Text 'Caso os tamanhos não fecharem, coloca um zero na frente
        ' ''tbHSOMARf.Text = DataFsomar.Hour
        ' ''If tbHSOMARf.Text.Length = 1 Then tbHSOMARf.Text = "0" & tbHSOMARf.Text 'Caso os tamanhos não fecharem, coloca um zero na frente
        ' ''dtpSOMARi.Value = DataIsomar
        ' ''dtpSOMARf.Value = DataFsomar

        NT_SOMAR = (DateDiff(DateInterval.Hour, DataIsomar, DataFsomar) + 1)

        ReDim ChuvaMatrizSOMAR(NC, NT_SOMAR)

        Dim NC_counter As Integer
        Dim NT_counter As Integer
        NT_counter = 0
        Dim linhaSOMAR As String

        For Each s As String In opendiagSOMAR.FileNames
            Dim FileNameSOMAR As String = s
            FileOpen(3, FileNameSOMAR, OpenMode.Input)
            NT_counter += 1
            NC_counter = 0
            Do Until EOF(3)
                linhaSOMAR = LineInput(3)
                NC_counter += 1
                'If linhaSOMAR.Substring(18, 5) > 1 Then
                '    MsgBox(NC_counter & " " & NT_counter & " " & linhaSOMAR.Substring(18, 5))

                'End If
                ChuvaMatrizSOMAR(NC_counter, NT_counter) = linhaSOMAR.Substring(18, 5)
            Loop
            FileClose(3)
        Next


    End Sub

    Function ConverteDataString_to_Date_Hour(ByVal DataString As String)
        'Esta função converte datas do tipo "2007022820" em "28/02/2007 20:00:00"

        'Faz a leitura dos valores de dia, mes, ano e hora
        Dim iDia, iMes, iAno, iHora As String
        'iDia = DataString.Substring(6, 2)
        'iMes = DataString.Substring(4, 2)
        'iAno = DataString.Substring(0, 4)
        'iHora = DataString.Substring(8, 2)
        iDia = DataString.Substring(17, 2)
        iMes = DataString.Substring(15, 2)
        iAno = DataString.Substring(11, 4)
        iHora = DataString.Substring(19, 2)

        'Caso os tamanhos não fecharem, coloca um zero na frente
        If iDia.Length = 1 Then iDia = "0" & iDia
        If iMes.Length = 1 Then iMes = "0" & iMes
        If iHora.Length = 1 Then iHora = "0" & iHora

        'Cria a variável de saída
        Dim DataSaidaHour As Date
        DataSaidaHour = iDia & "/" & iMes & "/" & iAno & " " & iHora & ":00:00"

        Return DataSaidaHour

    End Function

    Function ConverteDate_to_DateString_Hour(ByVal DateInform As Date)
        'Esta função converte datas do tipo "2007022820" em "28/02/2007 20:00:00"

        'Faz a leitura dos valores de dia, mes, ano e hora
        Dim iDia, iMes, iAno, iHora As String
        iDia = DateInform.Day
        iMes = DateInform.Month
        iAno = DateInform.Year
        iHora = DateInform.Hour

        'Caso os tamanhos não fecharem, coloca um zero na frente
        If iDia.Length = 1 Then iDia = "0" & iDia
        If iMes.Length = 1 Then iMes = "0" & iMes
        If iHora.Length = 1 Then iHora = "0" & iHora

        'Cria a variável de saída
        Dim DataSaidaString As String
        DataSaidaString = iAno & iMes & iDia & iHora

        Return DataSaidaString

    End Function

    Sub MESCLAR_SOMAR_CHUVALIDA()

        Dim dataO As Date '= dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date '= dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim dataOexiste As Date = lbInterpDt1.Text
        Dim dataEexiste As Date = lbInterpDt2.Text

        Dim dataOSOMARexiste As Date = DataIsomar
        Dim dataESOMARexiste As Date = DataFsomar

        If dataOSOMARexiste < dataOexiste Then
            dataO = dataOSOMARexiste
        Else
            dataO = dataOexiste
        End If

        If dataESOMARexiste > dataEexiste Then
            dataE = dataESOMARexiste
        Else
            dataE = dataEexiste
        End If

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

        Dim ChuvaMescla(Total_MiniBacias, 24, 31, 12, Ano(dataE.Year)) As Single

        'Zera as variáveis
        For p As Integer = 0 To Total_MiniBacias
            For aaaa = AI To dataE.Year
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            ChuvaMescla(p, hh, dd, mm, Ano(aaaa)) = -1.0
                        Next
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        'MsgBox("Criou e dimensionou variável de chuva")


        Dim datacomeço As Date
        datacomeço = dataOexiste.AddHours(-1)

        Dim tmaxxExiste As Integer
        tmaxxExiste = (DateDiff(DateInterval.Hour, dataOexiste, dataEexiste) + 1)

        For tempo As Integer = 1 To tmaxxExiste
            datacomeço = datacomeço.AddHours(1)
            For cells As Integer = 1 To Total_MiniBacias
                ChuvaMescla(cells, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = ChuvaInterpoladaLida(cells, tempo)
            Next
        Next

        MsgBox("Primeira etapa do processamento concluída.")


        datacomeço = dataOSOMARexiste.AddHours(-1)

        Dim tmaxxSOMARxiste As Integer
        tmaxxSOMARxiste = (DateDiff(DateInterval.Hour, dataOSOMARexiste, dataESOMARexiste) + 1)

        For tempo As Integer = 1 To tmaxxSOMARxiste
            datacomeço = datacomeço.AddHours(1)
            For cells As Integer = 1 To Total_MiniBacias
                ChuvaMescla(cells, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = ChuvaMatrizSOMAR(cells, tempo)
            Next
        Next

        'MsgBox("Salvou a chuva da SOMAR")


        ReDim ChuvaInterpoladaLida(Total_MiniBacias, tmaxx)
        datacomeço = dataO.AddHours(-1)

        For tempo As Integer = 1 To tmaxx
            datacomeço = datacomeço.AddHours(1)
            For cells As Integer = 1 To Total_MiniBacias
                ChuvaInterpoladaLida(cells, tempo) = ChuvaMescla(cells, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
            Next
        Next

        MsgBox("Segunda etapa do processamento concluída.")


        ReDim ChuvaMescla(0, 0, 0, 0, 0)

        Dim dia, mes, hora As String
        dia = dataO.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = dataO.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = dataO.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        lbInterpDt1.Text = dia & "/" & mes & "/" & dataO.Year & " " & hora & ":00:00"

        dia = dataE.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = dataE.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = dataE.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        lbInterpDt2.Text = dia & "/" & mes & "/" & dataE.Year & " " & hora & ":00:00"


    End Sub

    Sub RECALCULA_DATA_FINAL_SOMAR()
        'Esta rotina atualiza a data final disponível com base nos dados existentes
        Dim DataFinalObs As Date
        DataFinalObs = dataFp(1)

        For p = 1 To total_estações
            If dataFp(p) > DataFinalObs Then
                DataFinalObs = dataFp(p)
            End If
        Next

        Dim DataChuvaDispo As Date
        DataChuvaDispo = lbInterpDt2.Text

        If DataFinalObs < DataChuvaDispo Then
            dtpqob2.Value = DataChuvaDispo
            tb_hora_fim.Text = tb_hora_fim.Items(DataChuvaDispo.Hour)
        End If

    End Sub

    Private Sub Button11_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button11.Click
        Try
            ReDim Qdispo(total_estações, 24, 31, 12, Ano(AF))

            'Zera as variáveis de vazão e nível
            For p As Integer = 0 To total_estações
                For aaaa = AI To AF
                    For mm = 1 To 12
                        For dd = 1 To 31
                            For hh As Integer = 0 To 23
                                Qdispo(p, hh, dd, mm, Ano(aaaa)) = Chuva(p, hh, dd, mm, Ano(aaaa))
                            Next
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis

            dataI = dataIp(1)
            dataF = dataFp(1)

            'Acerta a data final
            Dim DataFinalObs As Date
            DataFinalObs = dataFp(1)
            For p = 1 To total_estações
                If dataFp(p) > DataFinalObs Then
                    DataFinalObs = dataFp(p)
                End If
            Next
            dataF = DataFinalObs

            Q_VERIFICA = False 'Avisa que é chuva
            Dim frmVER As New Form13
            frmVER.Show()
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Private Sub Button12_Click(ByVal sender As System.Object, ByVal e As System.EventArgs)

    End Sub

    Sub CARREGA_OPCAO_DADOS()
        'Verifica base de dados interna
        Dim arquivo_est As String 'no do arquivo com as configurações
        Dim linha As String 'variável string que contém a linha
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Fonte_Dados_Automaticos.txt"
        'abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Input)
        linha = LineInput(1)
        Fonte_Dados_Automaticos = linha
        'Fecha
        FileClose()

    End Sub

    '--Desativada em 11ago26
    'Private Sub DataGridView1_CellContentClick(sender As Object, e As DataGridViewCellEventArgs) _
    'Handles DataGridView1.CellContentClick

    '    If e.RowIndex < 0 OrElse e.ColumnIndex < 0 Then Exit Sub ' ignora cabeçalho/fora da grade

    '    ' Checa se é a coluna de checkbox (ou use .Name = "Selecionado")
    '    If Not TypeOf DataGridView1.Columns(e.ColumnIndex) Is DataGridViewCheckBoxColumn Then Exit Sub

    '    Dim row = DataGridView1.Rows(e.RowIndex)
    '    Dim cell = CType(row.Cells(e.ColumnIndex), DataGridViewCheckBoxCell)

    '    ' Valor atual (antes do toggle). Trate Nothing/DBNull.
    '    Dim atual As Boolean = False
    '    If cell.Value IsNot Nothing AndAlso cell.Value IsNot DBNull.Value Then
    '        atual = Convert.ToBoolean(cell.Value)
    '    End If

    '    ' Calcula novo estado
    '    Dim novo As Boolean = Not atual

    '    ' Atualiza sua variável de controle (0/1) e avisa
    '    AssimilaP(e.RowIndex + 1) = If(novo, 1, 0)

    '    Dim acao As String = If(novo, "ligou", "desligou")
    '    MsgBox($"Você {acao} a utilização da estação {Estação(e.RowIndex + 1)}. Valor AssimilaP({e.RowIndex + 1}) = {AssimilaP(e.RowIndex + 1)}")

    '    ' Aplica o toggle manualmente na célula
    '    cell.Value = novo

    '    ' Se estiver ligado a DataTable/DataSet, force commit imediato
    '    DataGridView1.CommitEdit(DataGridViewDataErrorContexts.Commit)
    '    DataGridView1.EndEdit()
    'End Sub

    Private Sub DataGridView1_CellContentClick(ByVal sender As Object,
                                           ByVal e As DataGridViewCellEventArgs) _
                                           Handles DataGridView1.CellContentClick

        'Ignora cabeçalho ou clique fora da grade
        If e.RowIndex < 0 OrElse e.ColumnIndex < 0 Then Exit Sub

        'Checa se é uma coluna de checkbox
        If Not TypeOf DataGridView1.Columns(e.ColumnIndex) Is DataGridViewCheckBoxColumn Then Exit Sub

        '--------------------------------------------------------
        'Descobre qual estação realmente corresponde a esta linha
        '--------------------------------------------------------
        Dim codigoEstacao As String

        If DataGridView1.Item(0, e.RowIndex).Value Is Nothing Then Exit Sub

        codigoEstacao = DataGridView1.Item(0, e.RowIndex).Value.ToString.Trim

        Dim est As Integer
        est = LOCALIZA_ESTACAO_NA_BASE(codigoEstacao)

        If est = -1 Then
            MsgBox("A estação selecionada não foi encontrada na base de dados.")
            Exit Sub
        End If

        '--------------------------------------------------------
        'Trabalha com o checkbox
        '--------------------------------------------------------
        Dim row As DataGridViewRow
        row = DataGridView1.Rows(e.RowIndex)

        Dim cell As DataGridViewCheckBoxCell
        cell = CType(row.Cells(e.ColumnIndex), DataGridViewCheckBoxCell)

        'Valor atual, antes do toggle
        Dim atual As Boolean = False

        If cell.Value IsNot Nothing AndAlso cell.Value IsNot DBNull.Value Then
            atual = Convert.ToBoolean(cell.Value)
        End If

        'Calcula novo estado
        Dim novo As Boolean
        novo = Not atual

        '--------------------------------------------------------
        'Atualiza AssimilaP usando o índice REAL da estação
        '--------------------------------------------------------
        AssimilaP(est) = If(novo, 1, 0)

        Dim acao As String

        If novo Then
            acao = "ligou"
        Else
            acao = "desligou"
        End If

        MsgBox("Você " & acao &
           " a utilização da estação " & Estação(est) &
           ". Valor AssimilaP(" & est & ") = " & AssimilaP(est))

        'Aplica o toggle manualmente na célula
        cell.Value = novo

        'Força commit imediato
        DataGridView1.CommitEdit(DataGridViewDataErrorContexts.Commit)
        DataGridView1.EndEdit()

    End Sub

    Sub CARREGAR_DADOS_P_NOVO_DE_UMA_PASTA()


        Dim f2() As String = IO.Directory.GetFiles(diretorio_arquivos) 'este é o diretório com os arquivos, definido nas preferências


        'Cria a variável que guarda o número de arquivos carregados na caixa de diálogo
        Dim pmax As Integer
        'Preenche o número de arquivos
        pmax = UBound(f2) + 1
        'Cria a variável que contém o nome dos arquivos individualmente, já na dimensão necessária
        Dim Arqu(pmax) As String
        'Cria a varíavel p, que contém a informação de qual posto da lista de postos nós estamos utilizando
        Dim p As Integer
        'Inicialmente p = -1, pois não começamos a ler os postos ainda
        p = -1

        'Rotina de atualizaçãp de dados das estações das UHES em formato tripa
        total_estações = DataGridView1.Rows.Count - 1

        'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
        Dim CodigoLocalizado As Integer
        Dim cod_estação As String

        Dim NumArquivosAtualizados As Integer 'contador de quantos arquivos foram atualizados
        NumArquivosAtualizados = 0

        For i As Integer = 0 To UBound(f2)

            'somamos 1 à variável p, indicando qual arquivo estamos lendo
            p = p + 1
            'o nome do arquivo é guardado na variável Arqu(p)
            Arqu(p) = f2(i)

            ' cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)).Substring(0, 8)
            cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p))

            CodigoLocalizado = 0
            For po As Integer = 1 To total_estações
                If cod_estação = "P" & Estação(po) Then
                    CodigoLocalizado = po
                End If
            Next

            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo

                NumArquivosAtualizados += 1

                Dim linha4 As String
                Dim pl As Double
                FileOpen(4, Arqu(p), OpenMode.Input)
                Dim contador As Integer
                contador = -1
                Do Until EOF(4)
                    linha4 = LineInput(4)

                    contador += 1
                    If contador < 1 Then Continue Do

                    pl = AjustarDecimal(linha4.Substring(26, 10))

                    If pl < 0 Then
                        pl = -1
                    ElseIf pl >= 9999 Then
                        pl = -1
                    End If

                    Chuva(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = pl

                    If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                        dataIp(CodigoLocalizado) = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                        If dataIp(CodigoLocalizado) < dataI Then
                            dataI = dataIp(CodigoLocalizado)
                        End If
                    End If

                    If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                        dataFp(CodigoLocalizado) = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                        If dataFp(CodigoLocalizado) > dataF Then
                            dataF = dataFp(CodigoLocalizado)
                        End If
                    End If

                Loop
                FileClose(4)



            End If


        Next




        'atualiza os dados se atualizou pelo menos um arquivo
        If NumArquivosAtualizados > 0 Then
            Me.AtualizaDataTabela()
            Me.ATUALIZA_ARQUIVOS_CHUVA()
            Me.ATUALIZA_DATA()
            ToolStripStatusLabel1.Text = "Atualização finalizada"

        End If
    End Sub


    Sub ATUALIZA_TABELA_ESTAÇÕES_PLUV()
        ' Try
        'Verifica quantas estações existem na base de dados interna
        Dim arquivo_est As String 'nome do arquivo que contém as estações
        Dim linha As String 'linha de leitura
        total_estações = 0 'número de estações

        'define o arquivo
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesPluv.txt"

        'Abre e faz a leituara
        FileOpen(1, arquivo_est, OpenMode.Input)
        Do Until EOF(1)
            linha = LineInput(1)
            total_estações = total_estações + 1
        Loop
        FileClose()
        total_estações = total_estações - 1 'desconta a primeira linha, que é cabeçalho
        '-- Fim da verificação do total de estações

        '--

        FileOpen(1, arquivo_est, OpenMode.Output)
        Dim est As Integer
        PrintLine(1, "ID;CODIGO;LAT;LON;ALT;NOME;A;B;C;USAR (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)")

        For est = 1 To total_estações

            PrintLine(1, ID(est) & ";" & Estação(est) & ";" & LAT(est) & ";" & LON(est) & ";" & ALT(est) & ";" & NOME(est) & ";" & A(est) & ";" & B(est) & ";" & C(est) & ";" & AssimilaP(est))


        Next

        FileClose()


    End Sub


    ' Distância AO QUADRADO (evita sqrt; mesma ordem de proximidade)
    Private Shared Function Dist2(ByVal x1 As Double, ByVal y1 As Double,
                              ByVal x2 As Double, ByVal y2 As Double) As Double
        Dim dx = x1 - x2
        Dim dy = y1 - y2
        Return dx * dx + dy * dy
    End Function

    ' Pré-ordena, para cada mini, as estações por distância crescente
    ' Retorna dois vetores indexados por mini: vizIdx(mini) -> Integer(), vizD2(mini) -> Double()
    Private Sub PrecomputeVizinhancaOrdenada2(ByVal totalMini As Integer,
                                          ByVal totalEst As Integer,
                                          ByVal X_MINI() As Double,
                                          ByVal Y_MINI() As Double,
                                          ByVal LON() As Double,
                                          ByVal LAT() As Double,
                                          ByRef vizIdx()() As Integer,
                                          ByRef vizD2()() As Double)
        ReDim vizIdx(totalMini)
        ReDim vizD2(totalMini)

        For mini As Integer = 1 To totalMini
            Dim x = X_MINI(mini)
            Dim y = Y_MINI(mini)
            Dim pares As New List(Of KeyValuePair(Of Integer, Double))(totalEst)
            For p As Integer = 1 To totalEst
                pares.Add(New KeyValuePair(Of Integer, Double)(p, Dist2(x, y, LON(p), LAT(p))))
            Next
            pares.Sort(Function(a, b) a.Value.CompareTo(b.Value))
            vizIdx(mini) = pares.Select(Function(kv) kv.Key).ToArray()
            vizD2(mini) = pares.Select(Function(kv) kv.Value).ToArray()
        Next
    End Sub

    ' Peso IDW a partir de d² e expoente p (w = 1 / d^p = 1 / (d²)^(p/2))
    <MethodImpl(Runtime.CompilerServices.MethodImplOptions.AggressiveInlining)>
    Private Shared Function WeightFromD2(ByVal d2 As Double, ByVal p As Double) As Double
        If p = 2.0 Then
            Return 1.0 / d2           ' otimização comum
        Else
            Return 1.0 / Math.Pow(d2, p / 2.0)
        End If
    End Function



    '---NOVA ROTINA PRINCIPAL PARA INTERPOLAÇÃO
    Sub INTERPOLAR_V6() 'Faz a interpolação só a partir de onde não tem chuva já interpolada salva

        Dim multiplicador_min As Integer
        multiplicador_min = TextBox1.Text
        Dim expoente As Double
        expoente = TextBox2.Text

        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo

        'MsgBox("Vai dimensionar a variável de chuva")

        ReDim Chuvabin(Total_MiniBacias, tmaxx)

        For Mini As Integer = 0 To Total_MiniBacias
            For t As Integer = 0 To tmaxx
                Chuvabin(Mini, t) = 0
            Next
        Next

        '---Percorre as datas do chuta interpolada que existem
        Dim dataOexiste As Date = lbInterpDt1.Text
        Dim dataEexiste As Date = lbInterpDt2.Text

        Dim tmaxxExiste As Integer
        tmaxxExiste = (DateDiff(DateInterval.Hour, dataOexiste, dataEexiste) + 1)

        Dim datacomeçoExiste As Date 'define a data inicial da interpolação
        datacomeçoExiste = dataOexiste
        datacomeçoExiste = datacomeçoExiste.AddHours(-1)

        Dim contador_t As Integer
        contador_t = 0


        For t As Integer = 1 To tmaxxExiste
            datacomeçoExiste = datacomeçoExiste.AddHours(+1)

            If datacomeçoExiste >= dataO And datacomeçoExiste <= dataE Then
                contador_t += 1
                For Mini As Integer = 0 To Total_MiniBacias
                    Chuvabin(Mini, contador_t) = ChuvaInterpoladaLida(Mini, t)
                Next
            End If
        Next
        '---Terminou de percorrer as datas do chuta interpolada que existem

        'Se todo o intervalo já existir, encerra a subrotina
        If contador_t >= tmaxx Then Exit Sub

        statusbar.Value = 0
        statusbar.Maximum = tmaxx * Total_MiniBacias


        'Aqui começa a implementação da nova rotina, em comparação com a V5
        Dim totalMini As Integer = Total_MiniBacias
        Dim totalEst As Integer = total_estações

        ' Pré-ordena vizinhanças (1x por chamada)
        Dim vizIdx()() As Integer
        Dim vizD2()() As Double

        Dim Xd() As Double = Array.ConvertAll(X_MINI, Function(v As Single) CDbl(v))
        Dim Yd() As Double = Array.ConvertAll(Y_MINI, Function(v As Single) CDbl(v))
        Dim LONd() As Double = Array.ConvertAll(LON, Function(v As Single) CDbl(v))
        Dim LATd() As Double = Array.ConvertAll(LAT, Function(v As Single) CDbl(v))

        PrecomputeVizinhancaOrdenada2(totalMini, totalEst, Xd, Yd, LONd, LATd, vizIdx, vizD2)

        ' Data base
        Dim horaIni As Integer = CInt(tb_hora_ini.Text)
        Dim dataBase As New DateTime(dtpqob1.Value.Year, dtpqob1.Value.Month, dtpqob1.Value.Day, horaIni, 0, 0)

        ' Raio (opcional)
        Dim raio As Double = 0
        Dim usarRaio As Boolean = Double.TryParse(TextBox1.Text, NumberStyles.Float, CultureInfo.InvariantCulture, raio)
        If Not usarRaio Then usarRaio = Double.TryParse(TextBox1.Text, NumberStyles.Float, CultureInfo.CurrentCulture, raio)
        usarRaio = usarRaio AndAlso raio > 0
        Dim raio2 As Double = raio * raio

        ' Expoente (default = 2)
        Dim expoente_interpol As Double = 2.0
        Dim okExp As Boolean = Double.TryParse(TextBox2.Text, NumberStyles.Float, CultureInfo.InvariantCulture, expoente_interpol)
        If Not okExp Then okExp = Double.TryParse(TextBox2.Text, NumberStyles.Float, CultureInfo.CurrentCulture, expoente_interpol)
        If Not okExp OrElse expoente_interpol <= 0 Then expoente_interpol = 2.0   ' sanity check

        ' --- limiar (opcional): valores > limiar são considerados falha ---
        Dim limiar As Double = 0
        Dim usarLimiar As Boolean = Double.TryParse(tb_limiar.Text, NumberStyles.Float, CultureInfo.InvariantCulture, limiar)
        If Not usarLimiar Then usarLimiar = Double.TryParse(tb_limiar.Text, NumberStyles.Float, CultureInfo.CurrentCulture, limiar)
        usarLimiar = usarLimiar AndAlso limiar > 0

        Dim K As Integer
        K = TextBox3.Text
        Const EPS As Double = 0.000000000001

        For mini As Integer = 1 To totalMini
            Dim datacomeco As DateTime = dataBase.AddHours(contador_t)

            For aux As Integer = (contador_t + 1) To tmaxx
                datacomeco = datacomeco.AddHours(1)

                Dim hh As Integer = datacomeco.Hour
                Dim dd As Integer = datacomeco.Day
                Dim mm As Integer = datacomeco.Month
                Dim yyIdx As Integer = Ano(datacomeco.Year)

                Dim sumW As Double = 0.0
                Dim sumWV As Double = 0.0
                Dim usados As Integer = 0
                Dim valorDiretoSetado As Boolean = False

                Dim idxs() As Integer = vizIdx(mini)
                Dim d2s() As Double = vizD2(mini)

                For i As Integer = 0 To idxs.Length - 1
                    Dim p As Integer = idxs(i)
                    Dim d2 As Double = d2s(i)

                    ' Limite de raio
                    If usarRaio AndAlso d2 > raio2 Then Exit For

                    Dim v As Double = Chuva(p, hh, dd, mm, yyIdx)

                    ' 1) falhas nativas (<0)
                    If v < 0 Then
                        Continue For
                    End If

                    ' 2) filtro por limiar: > limiar => tratar como falha (ignorar)
                    If usarLimiar AndAlso v > limiar Then
                        Continue For
                    End If

                    ' 3) coincidência espacial (d²≈0) ainda precisa respeitar o limiar
                    If d2 <= EPS Then
                        Chuvabin(mini, aux) = v
                        valorDiretoSetado = True
                        Exit For
                    End If

                    ' 4) IDW normal
                    Dim w As Double = WeightFromD2(d2, expoente_interpol)
                    sumW += w
                    sumWV += w * v
                    usados += 1
                    If usados = K Then Exit For

                Next

                If Not valorDiretoSetado Then
                    If usados > 0 AndAlso sumW > 0 Then
                        Chuvabin(mini, aux) = sumWV / sumW
                    Else
                        Chuvabin(mini, aux) = 0
                    End If
                End If

                ' UI: atualiza com menor frequência
                If (aux And 31) = 0 Then
                    Try
                        statusbar.Value = Math.Min(statusbar.Maximum, statusbar.Value + 32)
                    Catch
                    End Try
                End If
            Next
        Next

    End Sub

    Private Sub Button12_Click_1(sender As Object, e As EventArgs) Handles Button12.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "main_consistencia.exe"

        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\\Executavel_consistencia_espacial\Consistencia_espacial_v1\" & ExecutavelFORTRAN)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Sub INTERPOLAR_V7() ' IDW 3 vizinhos + raio + expoente (otimizado)
        Dim K As Integer
        K = TextBox3.Text
        Const EPS As Double = 0.000000000001

        ' --- intervalos de tempo ---
        Dim dataO As New DateTime(dtpqob1.Value.Year, dtpqob1.Value.Month, dtpqob1.Value.Day, CInt(tb_hora_ini.Text), 0, 0)
        Dim dataE As New DateTime(dtpqob2.Value.Year, dtpqob2.Value.Month, dtpqob2.Value.Day, CInt(tb_hora_fim.Text), 0, 0)
        Dim tmaxx As Integer = DateDiff(DateInterval.Hour, dataO, dataE) + 1
        If tmaxx < 1 Then
            MessageBox.Show("Intervalo de tempo vazio.", "Interpolação", MessageBoxButtons.OK, MessageBoxIcon.Warning)
            Exit Sub
        End If

        ' --- saída ---
        ReDim Chuvabin(Total_MiniBacias, tmaxx)

        ' --- status UI ---
        Try
            statusbar.Value = 0
            statusbar.Maximum = tmaxx * Total_MiniBacias
        Catch
        End Try

        ' --- raio (opcional) ---
        Dim raio As Double = 0
        Dim usarRaio As Boolean = Double.TryParse(TextBox1.Text, NumberStyles.Float, CultureInfo.InvariantCulture, raio)
        If Not usarRaio Then usarRaio = Double.TryParse(TextBox1.Text, NumberStyles.Float, CultureInfo.CurrentCulture, raio)
        usarRaio = usarRaio AndAlso raio > 0
        Dim raio2 As Double = raio * raio

        ' --- expoente (default = 2) ---
        Dim expoente_interpol As Double = 2.0
        Dim okExp As Boolean = Double.TryParse(TextBox2.Text, NumberStyles.Float, CultureInfo.InvariantCulture, expoente_interpol)
        If Not okExp Then okExp = Double.TryParse(TextBox2.Text, NumberStyles.Float, CultureInfo.CurrentCulture, expoente_interpol)
        If Not okExp OrElse expoente_interpol <= 0 Then expoente_interpol = 2.0

        ' --- limiar (opcional): valores > limiar são considerados falha ---
        Dim limiar As Double = 0
        Dim usarLimiar As Boolean = Double.TryParse(tb_limiar.Text, NumberStyles.Float, CultureInfo.InvariantCulture, limiar)
        If Not usarLimiar Then usarLimiar = Double.TryParse(tb_limiar.Text, NumberStyles.Float, CultureInfo.CurrentCulture, limiar)
        usarLimiar = usarLimiar AndAlso limiar > 0

        ' --- prepara vizinhança (converte Single() -> Double() 1x) ---
        Dim Xd() As Double = Array.ConvertAll(Of Single, Double)(X_MINI, Function(v) CDbl(v))
        Dim Yd() As Double = Array.ConvertAll(Of Single, Double)(Y_MINI, Function(v) CDbl(v))
        Dim LONd() As Double = LON
        Dim LATd() As Double = LAT

        Dim vizIdx()() As Integer
        Dim vizD2()() As Double
        PrecomputeVizinhancaOrdenada2(Total_MiniBacias, total_estações, Xd, Yd, LONd, LATd, vizIdx, vizD2)

        ' --- laços principais ---
        For mini As Integer = 1 To Total_MiniBacias
            ' começa uma hora antes e anda 1h por passo (seguindo seu padrão)
            Dim datacomeco As DateTime = dataO.AddHours(-1)

            For aux As Integer = 1 To tmaxx
                datacomeco = datacomeco.AddHours(1)

                ' componentes de tempo 1x
                Dim hh As Integer = datacomeco.Hour
                Dim dd As Integer = datacomeco.Day
                Dim mm As Integer = datacomeco.Month
                Dim yyIdx As Integer = Ano(datacomeco.Year)

                Dim sumW As Double = 0.0
                Dim sumWV As Double = 0.0
                Dim usados As Integer = 0
                Dim valorDiretoSetado As Boolean = False

                Dim idxs() As Integer = vizIdx(mini)
                Dim d2s() As Double = vizD2(mini)

                ' varre vizinhos em ordem crescente de distância até juntar K válidos (ou coincidir)
                For i As Integer = 0 To idxs.Length - 1
                    Dim p As Integer = idxs(i)
                    Dim d2 As Double = d2s(i)

                    ' raio (se configurado)
                    If usarRaio AndAlso d2 > raio2 Then Exit For

                    Dim v As Double = Chuva(p, hh, dd, mm, yyIdx)

                    ' 1) falhas nativas (<0)
                    If v < 0 Then
                        Continue For
                    End If

                    ' 2) filtro por limiar: > limiar => tratar como falha (ignorar)
                    If usarLimiar AndAlso v > limiar Then
                        Continue For
                    End If

                    ' 3) coincidência espacial (d²≈0) ainda precisa respeitar o limiar
                    If d2 <= EPS Then
                        Chuvabin(mini, aux) = v
                        valorDiretoSetado = True
                        Exit For
                    End If

                    ' 4) IDW normal
                    Dim w As Double = WeightFromD2(d2, expoente_interpol)
                    sumW += w
                    sumWV += w * v
                    usados += 1
                    If usados = K Then Exit For

                Next

                If Not valorDiretoSetado Then
                    If usados > 0 AndAlso sumW > 0 Then
                        Chuvabin(mini, aux) = sumWV / sumW
                    Else
                        Chuvabin(mini, aux) = 0 ' ninguém válido (ou fora do raio)
                    End If
                End If

                ' UI: atualiza esparsamente
                If (aux And 31) = 0 Then
                    Try
                        statusbar.Value = Math.Min(statusbar.Maximum, statusbar.Value + 32)
                    Catch
                    End Try
                End If
            Next
        Next
    End Sub


End Class