Imports System.IO

Public Class Form19

    'Public Num_mini_reg() As Integer
    Private região(,) As Integer 'Arquivo de regiões, a primeira coluna é o número de regiões e a segunda é o número máximo de mini-bacias em cada região
    Private chuvabin(,) As Double
    Private linha_m As String 'variável de leitura
    Private X_MINI() As Double 'longitude da minibacia
    Private Y_MINI() As Double 'latitude da minibacia
    Private Total_MiniBacias As Integer
    Private diretorio_previsao As String 'diretório onde deve ser salvo o arquivo de chuvas observadas
    Private Total_SubBacias As Integer 'Número de subbacias no modelo
    Private Num_mini_reg() As Integer
    Private dataIniArquivo As Date 'DataInicial do arquivo de chuva
    Private Distância_Mínima() As Double
    Private PontoMaisPróximoda(), NumPontosSIMEPAR, NumHorasSIMEPAR As Integer
    Private ChuvaArquivo(,) As Single

    Private Sub Form19_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        Me.CARREGA_MINI()
        'Me.Dimensiona_Colunas()
        ' Me.CARREGA_REGIOES()

        Me.CARREGA_DIRETÓRIOS()
        Me.Carrega_Hora_Previsão()

        'Se o automático estiver ligado
        If TotalCommander = True Then
            Me.APERTA_BOTAO_INTERPOLAR()
            Me.Close()
        End If
    End Sub


    Sub CARREGA_REGIOES()
        Dim app_path_arq As String
        app_path_arq = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\MINI.MGB"

        'Dim Num_mini_reg(5) As Integer
        Dim linha As String
        Dim ll As Integer
        ll = -1
        'Dim região(5, 200) As Integer

        ReDim Num_mini_reg(Total_SubBacias)
        For T As Integer = 1 To Total_SubBacias
            Num_mini_reg(T) = 0
        Next

        FileOpen(1, app_path_arq, OpenMode.Input)
        ReDim região(Total_SubBacias, Total_MiniBacias)
        Do Until EOF(1)
            linha = LineInput(1)
            ll = ll + 1
            If ll >= 1 Then
                Num_mini_reg(linha.Substring(51, 3)) += 1
                região(linha.Substring(51, 3), Num_mini_reg(linha.Substring(51, 3))) = linha.Substring(8, 8)
            End If
        Loop

        FileClose()


    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        Me.APERTA_BOTAO_INTERPOLAR()
        MsgBox("Interpolação finalizada com sucesso!", MsgBoxStyle.Information, "MGB-IPH")
        Me.Close()
    End Sub

    Sub APERTA_BOTAO_INTERPOLAR()

        'Verifica qual a fonte do dado de previsão, se chuva zero ou de arquivo
        If rbChuvaZero.Checked = True Then
            Me.INTERPOLA_ZERO()
        Else
            Me.INTERPOLA_ARQUIVO()
        End If

        Me.SALVAR_CHUVABIN_AUTOMATICO()
        'Me.SALVAR_CHUVABIN() 'desligado, para usar o diretório automaticamente
        If CheckBox1.Checked Then
            Me.SALVA_CHUVATEXTO()
        End If
        Me.ESCREVER_METADADOS()


    End Sub

    Sub SALVA_CHUVATEXTO()

        sfd.Filter = "Arquivo de chuva ASCII para verificação(.txt)|*.txt"

        If sfd.ShowDialog = Windows.Forms.DialogResult.OK Then

            Dim nomeArquivo As String
            nomeArquivo = sfd.FileName


            FileOpen(4, nomeArquivo, OpenMode.Output)

            Dim dataO As Date = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            Dim dataE As Date = DateTimePicker2.Value.Day & "/" & DateTimePicker2.Value.Month & "/" & DateTimePicker2.Value.Year & " " & tb_hora_ini.Text & ":00:00"

            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

            For tempo As Integer = 1 To tmaxx
                For cells As Integer = 1 To Total_MiniBacias
                    Dim numero As Single
                    numero = chuvabin(cells, tempo)
                    Print(4, "    " & FormatNumber(numero, 2))
                    'MsgBox(cells & " " & tempo & " " & chuvabin(cells, tempo))
                Next
                PrintLine(4)
            Next

            FileClose()

            MsgBox("Arquivo salvo com sucesso!", MsgBoxStyle.Information)


        End If

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


            Dim dataO As Date = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            Dim dataE As Date = DateTimePicker2.Value.Day & "/" & DateTimePicker2.Value.Month & "/" & DateTimePicker2.Value.Year & " " & tb_hora_ini.Text & ":00:00"

            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

            For tempo As Integer = 1 To tmaxx
                For cells As Integer = 1 To Total_MiniBacias
                    Dim numero As Single
                    numero = chuvabin(cells, tempo)
                    bw.Write(numero)
                    'MsgBox(cells & " " & tempo & " " & chuvabin(cells, tempo))
                Next
            Next

            bw.Close()

            MsgBox("Arquivo salvo com sucesso!", MsgBoxStyle.Information)


        End If
    End Sub

    Sub ESCREVER_METADADOS()
        'Calcula as datas
        Dim dataO As Date = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = DateTimePicker2.Value.Day & "/" & DateTimePicker2.Value.Month & "/" & DateTimePicker2.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

        'Declara as variáveis
        Dim nome_metadados As String
        nome_metadados = System.IO.Path.GetFileNameWithoutExtension(sfd.FileName)
        Dim diretório As String
        diretório = System.IO.Path.GetDirectoryName(sfd.FileName)

        Dim arquivo_metadados As String
        'arquivo_metadados = diretório & "\" & nome_metadados & ".meta"
        arquivo_metadados = diretório & "\" & "Previsao.meta"

        '
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
        Print(3, DateTimePicker1.Value.Year & "    ")

        Dim mes As String
        mes = DateTimePicker1.Value.Month
        If mes.Length = 1 Then
            mes = "0" & mes
        End If
        Print(3, mes & "    ")

        Dim dia As String
        dia = DateTimePicker1.Value.Day
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
        PrintLine(3, Total_MiniBacias)
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

    Private Function distância(ByVal x_minibac As Double, ByVal Y_minibac As Double, ByVal lon_p As Double, ByVal lat_p As Double)
        Dim dist As Double
        dist = ((x_minibac - lon_p) ^ 2 + (Y_minibac - lat_p) ^ 2) ^ 0.5
        Return dist
    End Function

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        Dim numcels As Integer
        ofd.Multiselect = False
        'OFD.Filter = "Arquivo com dados de geometria do MGB(.MGB)|*.MGB"
        ofd.Filter = "Arquivo com as mini-bacias para o MGB(.mgb;.txt)|*.mgb;*.txt"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            tb_mini.Text = ofd.FileName
            numcels = -1
            FileOpen(1, ofd.FileName, OpenMode.Input)
            Do Until EOF(1)
                linha_m = LineInput(1)
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
                linha_m = LineInput(1)
                numcels = numcels + 1
                If numcels >= 1 Then
                    X_MINI(numcels) = linha_m.Substring(22, 9)
                    Y_MINI(numcels) = linha_m.Substring(37, 9)
                End If
            Loop
            FileClose()


            MsgBox("Leitura do arquivo Mini.MGB realizada com sucesso!", MsgBoxStyle.Information, "Previsão")
            Button2.Enabled = True
        End If
    End Sub

    Sub CARREGA_MINI()

        Dim numcels As Integer
        'Carrega o arquivo MINI.MGB de forma automática
        Dim arquivo_MINI As String 'localização do arquivo de mini bacias
        'Existe uma cópia do MINI dentro do arquivo Input do plugin
        arquivo_MINI = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\MINI.MGB"
        Total_SubBacias = 0
        tb_mini.Text = arquivo_MINI
        numcels = -1
        FileOpen(1, arquivo_MINI, OpenMode.Input)
        Do Until EOF(1)
            linha_m = LineInput(1)
            numcels = numcels + 1
            If numcels >= 1 Then
            End If
        Loop
        FileClose()

        Total_MiniBacias = numcels
        ReDim X_MINI(Total_MiniBacias)
        ReDim Y_MINI(Total_MiniBacias)

        numcels = -1
        FileOpen(1, arquivo_MINI, OpenMode.Input)
        Do Until EOF(1)
            linha_m = LineInput(1)
            numcels = numcels + 1
            If numcels >= 1 Then
                If Total_SubBacias < linha_m.Substring(51, 3) Then Total_SubBacias = linha_m.Substring(51, 3)
                X_MINI(numcels) = linha_m.Substring(22, 9)
                Y_MINI(numcels) = linha_m.Substring(37, 9)
            End If
        Loop
        FileClose()

        'MsgBox("Leitura do arquivo Mini.MGB realizada com sucesso!", MsgBoxStyle.Information, "Previsão")
        ' Button2.Enabled = True
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
            'Quando chegar na linha 4, faz a leitura do diretório onde será salvo o arquivo da previsão
            If total_estações = 7 Then 'para Itaipu é 4, para estreito é 7
                diretorio_previsao = linha
            End If
        Loop

        stlb.Text = "Diretório arquivos de previsão: " & diretorio_previsao
        'Fecha
        FileClose()

    End Sub

    Sub SALVAR_CHUVABIN_AUTOMATICO()

        'sfd.Filter = "Arquivo de chuva binário para o MGB(.MGB)|*.MGB"

        'If sfd.ShowDialog = Windows.Forms.DialogResult.OK Then


        sfd.FileName = diretorio_previsao & "\PrevisaoMINI_Estreito.bin"

        Dim nomeArquivo As String
        nomeArquivo = sfd.FileName

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Create, FileAccess.ReadWrite)
        Dim bw As New System.IO.BinaryWriter(fs)


        Dim dataO As Date = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = DateTimePicker2.Value.Day & "/" & DateTimePicker2.Value.Month & "/" & DateTimePicker2.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)

        For tempo As Integer = 1 To tmaxx
            For cells As Integer = 1 To Total_MiniBacias
                Dim numero As Single
                numero = chuvabin(cells, tempo)
                bw.Write(numero)
                'MsgBox(cells & " " & tempo & " " & chuvabin(cells, tempo))
            Next
        Next

        bw.Close()

        'MsgBox("Arquivo salvo com sucesso!", MsgBoxStyle.Information)


        'End If
    End Sub

    Sub Carrega_Hora_Previsão() 'tenta carregar a hora inicial e final da previsão
        Try

            ofd.FileName = diretorio_previsao & "\CHUVA_OBS.mgb"
            'Declara as variáveis

            Dim diretório As String
            diretório = System.IO.Path.GetDirectoryName(ofd.FileName)

            Dim arquivo_metadados As String
            'arquivo padrão com nome dado pelo Siqueira:
            arquivo_metadados = diretório & "\CHUVA_OBS.meta"

            FileOpen(1, arquivo_metadados, OpenMode.Input)

            Dim datainiciopprev As Date
            Dim datafimpprev As Date

            Dim ll As Integer
            ll = 0
            Dim linha As String

            Do Until EOF(1)
                linha = LineInput(1)
                ll = ll + 1
                If ll = 10 Then
                    datainiciopprev = linha.Substring(0, 4) & "/" & linha.Substring(8, 2) & "/" & linha.Substring(14, 2) & " " & linha.Substring(20, 2) & ":00:00"

                    Dim addhour As Integer
                    addhour = linha.Substring(22, 8)
                    addhour = addhour - 1
                    datafimpprev = datainiciopprev.AddHours(addhour)
                End If
            Loop

            'adiciona a hora a mais, com o início da previsão
            datafimpprev = datafimpprev.AddHours(1)
            DateTimePicker1.Value = datafimpprev

            Dim hora As String
            hora = datafimpprev.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If
            tb_hora_ini.Text = hora

            DateTimePicker2.Value = datafimpprev.AddDays(10)

            FileClose()

        Catch ex As Exception

        End Try
    End Sub



    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click
        If rbGEFS.Checked = True Then
            ABRIR_GEFS_ONS()
        ElseIf rbETA40.Checked = True Then
            ABRIR_ETA40_ONS()
        ElseIf rbECMWF.Checked = True Then
            ABRIR_ECMWF_ONS()
        Else
            MsgBox("Selecione uma opção que permita a escolha de arquivos.")
        End If




    End Sub

    Sub CHUVA_ABRIR_SIMEPAR()
        ofd.Filter = "Arquivo binário com a grade de chuva prevista (.bin)|*.bin"

        If ofd.ShowDialog = Windows.Forms.DialogResult.OK Then

            tbChuvaArquivo.Text = ofd.FileName
            Dim arquivoChuva As String
            Dim arquivoCtl As String
            arquivoChuva = ofd.FileName
            arquivoCtl = System.IO.Path.GetDirectoryName(arquivoChuva) & "\" & System.IO.Path.GetFileNameWithoutExtension(arquivoChuva) & ".ctl"
            'Abre o arquivo CTL e faz a leitura
            Dim X, Y, T As Integer
            Dim dx, dy, Lon, Lat As Single
            Dim CoordX(), CoordY() As Single
            FileOpen(1, arquivoCtl, OpenMode.Input)
            Dim linha As String
            Dim ContLinhas As Integer
            ContLinhas = 0
            Do Until EOF(1)
                linha = LineInput(1)
                ContLinhas += 1
                If ContLinhas = 4 Then X = linha.Substring(9, 3)
                If ContLinhas = 4 Then dx = linha.Substring(31, 8)
                If ContLinhas = 4 Then Lon = linha.Substring(20, 10)
                If ContLinhas = 5 Then Y = linha.Substring(9, 3)
                If ContLinhas = 5 Then dy = linha.Substring(31, 8)
                If ContLinhas = 5 Then Lat = linha.Substring(20, 10)
                If ContLinhas = 7 Then T = linha.Substring(9, 3)

                If ContLinhas = 7 Then
                    Dim month As String
                    Dim dia, mes, ano, hora As Integer
                    month = linha.Substring(25, 3)
                    Select Case month
                        Case "jan"
                            mes = 1
                        Case "feb"
                            mes = 2
                        Case "mar"
                            mes = 3
                        Case "apr"
                            mes = 4
                        Case "may"
                            mes = 5
                        Case "jun"
                            mes = 6
                        Case "jul"
                            mes = 7
                        Case "aug"
                            mes = 8
                        Case "sep"
                            mes = 9
                        Case "oct"
                            mes = 10
                        Case "nov"
                            mes = 11
                        Case "dec"
                            mes = 12
                        Case Else
                            MsgBox("Erro no mês!")
                    End Select
                    dia = linha.Substring(23, 2)
                    hora = linha.Substring(20, 2)
                    ano = linha.Substring(28, 4)
                    dataIniArquivo = dia & "/" & mes & "/" & ano & " " & hora & ":00:00"
                End If
            Loop
            FileClose(1)
            MsgBox("Data da chuva prevista: " & dataIniArquivo)
            MsgBox("Número de linhas: " & Y & " Número de colunas: " & X & " Número de horas: " & T)
            MsgBox("Clique OK para iniciar a leitura do arquivo...")

            'Dimensiona a variável de chuva
            Dim PontosGrade As Integer
            PontosGrade = X * Y
            ReDim ChuvaArquivo(PontosGrade, T)
            ReDim CoordX(PontosGrade), CoordY(PontosGrade)

            'Calcula as coordenadas para cada ponto de grade
            Dim cell As Integer
            cell = 0
            For variaY As Integer = 1 To Y
                For variaX As Integer = 1 To X
                    cell = cell + 1
                    CoordX(cell) = Lon + dx * (variaX - 1)
                    'CoordY(cell) = Lat + -dy * (variaY - 1) + Y * dy 'CUIDADO! AQUI É TUDO MEIO INVERTIDO!
                    CoordY(cell) = Lat + dy * (variaY - 1) '+ Y * dy 'CUIDADO! AQUI É TUDO MEIO INVERTIDO!

                    'MsgBox("X: " & CoordX(cell) & " Y: " & CoordY(cell))
                Next
            Next

            'Faz a leitura do arquivo binário
            ' abre o arquivo e iniciao o objeto StreamReader
            Dim fs As FileStream
            fs = New FileStream(arquivoChuva, FileMode.Open)
            Dim bw As New System.IO.BinaryReader(fs)
            Dim numcells, tempobin, HoraCell As Integer
            numcells = X
            tempobin = Y
            HoraCell = T
            For Hora As Integer = 1 To HoraCell
                ' MsgBox(Hora)
                cell = 0
                For tempo As Integer = 1 To tempobin
                    For cells As Integer = 1 To numcells
                        cell = cell + 1
                        ChuvaArquivo(cell, Hora) = bw.ReadSingle
                        'If ChuvaArquivo(cell, Hora) > 0.1 Then MsgBox(ChuvaArquivo(cell, Hora))
                        'If ChuvaArquivo(cell, Hora) > 0 Then
                        '    If MsgBox(dataIniArquivo.AddHours(Hora - 1) & " " & dataIniArquivo.AddHours(Hora - 1).Hour & " " & CoordY(cell) & " " & " " & CoordX(cell) & " " & ChuvaArquivo(cell, Hora), MsgBoxStyle.YesNo) = MsgBoxResult.No Then

                        '    End If
                        'End If

                    Next
                Next
            Next
            bw.Close()

            'Verifica o ponto que tem a distância mínima a cada minibacia
            ReDim Distância_Mínima(Total_MiniBacias)
            ReDim PontoMaisPróximoda(Total_MiniBacias)
            For Mini As Integer = 1 To Total_MiniBacias
                Distância_Mínima(Mini) = 999999
                PontoMaisPróximoda(Mini) = 999999
            Next
            For Mini As Integer = 1 To Total_MiniBacias
                For p As Integer = 1 To PontosGrade
                    If distância(X_MINI(Mini), Y_MINI(Mini), CoordX(p), CoordY(p)) < Distância_Mínima(Mini) Then
                        Distância_Mínima(Mini) = distância(X_MINI(Mini), Y_MINI(Mini), CoordX(p), CoordY(p))
                        PontoMaisPróximoda(Mini) = p
                    End If
                Next
            Next

            'Transfere as variáveis mais importantes para variáveis globais
            NumPontosSIMEPAR = PontosGrade
            NumHorasSIMEPAR = T

            MsgBox("Leitura finalizada")
            Button2.Enabled = True
        End If

    End Sub

    Sub ABRIR_GEFS_ONS()

        ofd.Filter = "Arquivos dat com a grade de chuva prevista (.dat)|*.dat"
        ofd.Multiselect = True


        If ofd.ShowDialog = Windows.Forms.DialogResult.OK Then

            tbChuvaArquivo.Text = ofd.FileName
            Dim T As Integer
            Dim CoordX(), CoordY() As Single

            Dim ContLinhas As Integer
            Dim linha As String


            'Abre um arquivo e conta as linhas
            For Each S As String In ofd.FileNames
                Dim arquivo As String
                arquivo = S
                FileOpen(1, arquivo, OpenMode.Input)
                ContLinhas = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    ContLinhas += 1
                Loop
                FileClose()
                Exit For
            Next

            'dimensiona as datas
            Dim NumFiles As Integer
            NumFiles = ofd.FileNames.Length
            Dim DataIprev(NumFiles) As Date
            Dim dataFprev(NumFiles) As Date
            T = NumFiles * 24 + 1

            'Dimensiona a variável de chuva
            Dim PontosGrade As Integer
            PontosGrade = ContLinhas
            ReDim ChuvaArquivo(PontosGrade, T)
            ReDim CoordX(PontosGrade), CoordY(PontosGrade)

            MsgBox("Número de Pontos na Grade: " & PontosGrade & " Número de horas: " & T)

            'Faz a leitura dos arquivos
            Dim p As String
            p = 0
            Dim hora As Integer
            hora = 0
            Dim acumulahora As Integer
            acumulahora = 0
            For Each S As String In ofd.FileNames
                p = p + 1
                Dim file As String
                file = S
                Dim filestring As String
                filestring = System.IO.Path.GetFileNameWithoutExtension(file)

                Dim dia, mes, ano As Integer
                dia = filestring.Substring(6, 2)
                mes = filestring.Substring(8, 2)
                ano = filestring.Substring(10, 2)
                dataIniArquivo = dia & "/" & mes & "/20" & ano & " " & "00:00:00"
                DataIprev(p) = dia & "/" & mes & "/20" & ano & " " & "00:00:00"
                dia = filestring.Substring(13, 2)
                mes = filestring.Substring(15, 2)
                ano = filestring.Substring(17, 2)
                dataFprev(p) = dia & "/" & mes & "/20" & ano & " " & "00:00:00"

                'MsgBox(p & " " & filestring & " " & DataIprev(p) & " " & dataFprev(p))

                FileOpen(1, file, OpenMode.Input)
                Dim cell As Integer
                cell = 0
                acumulahora = acumulahora + 24
                Do Until EOF(1)
                    linha = LineInput(1)
                    cell = cell + 1

                    If linha.Length >= 20 Then 'para quando tem longitudes maior do que 100
                        CoordX(cell) = linha.Substring(0, 7)
                        CoordY(cell) = linha.Substring(8, 6)
                        For horario As Integer = 1 To 24
                            hora = (p - 1) * 24 + horario
                            ChuvaArquivo(cell, hora) = linha.Substring(14, 6)
                            ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                        Next
                    Else 'para quando as latitudes são menores do que 100
                        CoordX(cell) = linha.Substring(0, 6)
                        CoordY(cell) = linha.Substring(7, 6)
                        For horario As Integer = 1 To 24
                            hora = (p - 1) * 24 + horario
                            ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                            ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                        Next
                    End If

                Loop
                FileClose()
            Next

            MsgBox("Data da chuva prevista: " & dataIniArquivo)

            'MsgBox("Clique OK para iniciar a leitura do arquivo...")

            'Verifica o ponto que tem a distância mínima a cada minibacia
            ReDim Distância_Mínima(Total_MiniBacias)
            ReDim PontoMaisPróximoda(Total_MiniBacias)
            For Mini As Integer = 1 To Total_MiniBacias
                Distância_Mínima(Mini) = 999999
                PontoMaisPróximoda(Mini) = 999999
            Next
            For Mini As Integer = 1 To Total_MiniBacias
                For pf As Integer = 1 To PontosGrade
                    If distância(X_MINI(Mini), Y_MINI(Mini), CoordX(pf), CoordY(pf)) < Distância_Mínima(Mini) Then
                        Distância_Mínima(Mini) = distância(X_MINI(Mini), Y_MINI(Mini), CoordX(pf), CoordY(pf))
                        PontoMaisPróximoda(Mini) = pf
                    End If
                Next
            Next

            'Transfere as variáveis mais importantes para variáveis globais
            NumPontosSIMEPAR = PontosGrade
            NumHorasSIMEPAR = T

            MsgBox("Leitura finalizada")
            Button2.Enabled = True
        End If

    End Sub

    Sub ABRIR_ETA40_ONS()

        ofd.Filter = "Arquivos dat com a grade de chuva prevista (.dat)|*.dat"
        ofd.Multiselect = True


        If ofd.ShowDialog = Windows.Forms.DialogResult.OK Then

            tbChuvaArquivo.Text = ofd.FileName
            Dim T As Integer
            Dim CoordX(), CoordY() As Single

            Dim ContLinhas As Integer
            Dim linha As String


            'Abre um arquivo e conta as linhas
            For Each S As String In ofd.FileNames
                Dim arquivo As String
                arquivo = S
                FileOpen(1, arquivo, OpenMode.Input)
                ContLinhas = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    ContLinhas += 1
                Loop
                FileClose()
                Exit For
            Next

            'dimensiona as datas
            Dim NumFiles As Integer
            NumFiles = ofd.FileNames.Length
            Dim DataIprev(NumFiles) As Date
            Dim dataFprev(NumFiles) As Date
            T = NumFiles * 24 + 1

            'Dimensiona a variável de chuva
            Dim PontosGrade As Integer
            PontosGrade = ContLinhas
            ReDim ChuvaArquivo(PontosGrade, T)
            ReDim CoordX(PontosGrade), CoordY(PontosGrade)

            MsgBox("Número de Pontos na Grade: " & PontosGrade & " Número de horas: " & T)

            'Faz a leitura dos arquivos
            Dim p As String
            p = 0
            Dim hora As Integer
            hora = 0
            For Each S As String In ofd.FileNames
                p = p + 1
                Dim file As String
                file = S
                Dim filestring As String
                filestring = System.IO.Path.GetFileNameWithoutExtension(file)

                Dim dia, mes, ano As Integer
                dia = filestring.Substring(7, 2)
                mes = filestring.Substring(9, 2)
                ano = filestring.Substring(11, 2)
                dataIniArquivo = dia & "/" & mes & "/20" & ano & " " & "00:00:00"
                DataIprev(p) = dia & "/" & mes & "/20" & ano & " " & "00:00:00"
                dia = filestring.Substring(14, 2)
                mes = filestring.Substring(16, 2)
                ano = filestring.Substring(18, 2)
                dataFprev(p) = dia & "/" & mes & "/20" & ano & " " & "00:00:00"

                'MsgBox(p & " " & filestring & " " & DataIprev(p) & " " & dataFprev(p))

                FileOpen(1, file, OpenMode.Input)
                Dim cell As Integer
                cell = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    cell = cell + 1
                    CoordX(cell) = linha.Substring(0, 6)
                    CoordY(cell) = linha.Substring(7, 6)
                    For horario As Integer = 1 To 24
                        hora = (p - 1) * 24 + horario
                        ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                        'MsgBox(ChuvaArquivo(cell, hora) & " " & cell & " " & hora)

                        ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                    Next
                Loop
                FileClose()
            Next

            MsgBox("Data da chuva prevista: " & dataIniArquivo)

            'MsgBox("Clique OK para iniciar a leitura do arquivo...")

            'Verifica o ponto que tem a distância mínima a cada minibacia
            ReDim Distância_Mínima(Total_MiniBacias)
            ReDim PontoMaisPróximoda(Total_MiniBacias)
            For Mini As Integer = 1 To Total_MiniBacias
                Distância_Mínima(Mini) = 999999
                PontoMaisPróximoda(Mini) = 999999
            Next
            For Mini As Integer = 1 To Total_MiniBacias
                For pf As Integer = 1 To PontosGrade
                    If distância(X_MINI(Mini), Y_MINI(Mini), CoordX(pf), CoordY(pf)) < Distância_Mínima(Mini) Then
                        Distância_Mínima(Mini) = distância(X_MINI(Mini), Y_MINI(Mini), CoordX(pf), CoordY(pf))
                        PontoMaisPróximoda(Mini) = pf
                    End If
                Next
            Next

            'Transfere as variáveis mais importantes para variáveis globais
            NumPontosSIMEPAR = PontosGrade
            NumHorasSIMEPAR = T

            MsgBox("Leitura finalizada")
            Button2.Enabled = True
        End If

    End Sub

    Sub ABRIR_ECMWF_ONS()

        ofd.Filter = "Arquivos dat com a grade de chuva prevista (.dat)|*.dat"
        ofd.Multiselect = True


        If ofd.ShowDialog = Windows.Forms.DialogResult.OK Then

            tbChuvaArquivo.Text = ofd.FileName
            Dim T As Integer
            Dim CoordX(), CoordY() As Single

            Dim ContLinhas As Integer
            Dim linha As String


            'Abre um arquivo e conta as linhas
            For Each S As String In ofd.FileNames
                Dim arquivo As String
                arquivo = S
                FileOpen(1, arquivo, OpenMode.Input)
                ContLinhas = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    ContLinhas += 1
                Loop
                FileClose()
                Exit For
            Next

            'dimensiona as datas
            Dim NumFiles As Integer
            NumFiles = ofd.FileNames.Length
            Dim DataIprev(NumFiles) As Date
            Dim dataFprev(NumFiles) As Date
            T = NumFiles * 24 + 1

            'Dimensiona a variável de chuva
            Dim PontosGrade As Integer
            PontosGrade = ContLinhas
            ReDim ChuvaArquivo(PontosGrade, T)
            ReDim CoordX(PontosGrade), CoordY(PontosGrade)

            MsgBox("Número de Pontos na Grade: " & PontosGrade & " Número de horas: " & T)

            'Faz a leitura dos arquivos
            Dim p As String
            p = 0
            Dim hora As Integer
            hora = 0
            For Each S As String In ofd.FileNames
                p = p + 1
                Dim file As String
                file = S
                Dim filestring As String
                filestring = System.IO.Path.GetFileNameWithoutExtension(file)

                Dim dia, mes, ano As Integer
                dia = filestring.Substring(7, 2)
                mes = filestring.Substring(9, 2)
                ano = filestring.Substring(11, 2)
                dataIniArquivo = dia & "/" & mes & "/20" & ano & " " & "00:00:00"
                DataIprev(p) = dia & "/" & mes & "/20" & ano & " " & "00:00:00"
                dia = filestring.Substring(14, 2)
                mes = filestring.Substring(16, 2)
                ano = filestring.Substring(18, 2)
                dataFprev(p) = dia & "/" & mes & "/20" & ano & " " & "00:00:00"

                'MsgBox(p & " " & filestring & " " & DataIprev(p) & " " & dataFprev(p))

                FileOpen(1, file, OpenMode.Input)
                Dim cell As Integer
                cell = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    cell = cell + 1
                    CoordX(cell) = linha.Substring(0, 6)
                    CoordY(cell) = linha.Substring(7, 6)
                    For horario As Integer = 1 To 24
                        hora = (p - 1) * 24 + horario
                        ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                        'MsgBox(ChuvaArquivo(cell, hora) & " " & cell & " " & hora)

                        ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                    Next
                Loop
                FileClose()
            Next

            MsgBox("Data da chuva prevista: " & dataIniArquivo)

            'MsgBox("Clique OK para iniciar a leitura do arquivo...")

            'Verifica o ponto que tem a distância mínima a cada minibacia
            ReDim Distância_Mínima(Total_MiniBacias)
            ReDim PontoMaisPróximoda(Total_MiniBacias)
            For Mini As Integer = 1 To Total_MiniBacias
                Distância_Mínima(Mini) = 999999
                PontoMaisPróximoda(Mini) = 999999
            Next
            For Mini As Integer = 1 To Total_MiniBacias
                For pf As Integer = 1 To PontosGrade
                    If distância(X_MINI(Mini), Y_MINI(Mini), CoordX(pf), CoordY(pf)) < Distância_Mínima(Mini) Then
                        Distância_Mínima(Mini) = distância(X_MINI(Mini), Y_MINI(Mini), CoordX(pf), CoordY(pf))
                        PontoMaisPróximoda(Mini) = pf
                    End If
                Next
            Next

            'Transfere as variáveis mais importantes para variáveis globais
            NumPontosSIMEPAR = PontosGrade
            NumHorasSIMEPAR = T

            MsgBox("Leitura finalizada")
            Button2.Enabled = True
        End If

    End Sub

    Sub INTERPOLA_ARQUIVO()

        'Pega as datas na interface
        Dim data_ini As Date 'data inicial atualizada pela interface com base na chuva observada
        data_ini = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim data_fim As Date 'data final atualizada pela interface com base em uma previsão de X dias definida pelo usuário
        data_fim = DateTimePicker2.Value.Day & "/" & DateTimePicker2.Value.Month & "/" & DateTimePicker2.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim diff_t As Integer 'número de intervalos entre data inicial e final
        diff_t = DateDiff(DateInterval.Hour, data_ini, data_fim) + 1

        Dim diff_Prev_Final As Integer 'número de intervalos entre data do arquivo do SIMEPAR e final ajustada na interface
        diff_Prev_Final = DateDiff(DateInterval.Hour, dataIniArquivo, data_fim) + 1

        Dim data_count As Date
        data_count = dataIniArquivo
        data_count = data_count.AddHours(-1)

        If data_ini < dataIniArquivo Then
            MsgBox("Arquivo de previsão de chuva carregado inicia após o último intervalo observado", MsgBoxStyle.Critical, "Erro")
            Exit Sub
        End If

        ReDim chuvabin(Total_MiniBacias, diff_t)

        Dim ContadorIntervalo As Integer
        ContadorIntervalo = 0

        'Percorre todas as datas, começando na data inicial do arquivo de previsão de chuva e terminando na data final selecionada para a previsão
        For intervalo As Integer = 1 To diff_Prev_Final
            data_count = data_count.AddHours(+1)
            If data_count < data_ini Then
                ' MsgBox(data_count & " " & data_ini)
                'MsgBox("Pulou uma hora")
                Continue For
            End If
            ContadorIntervalo += 1
            'Se a interpolação ultrapassar o tamanho do arquivo, colocamos chuva zero
            If ContadorIntervalo > NumHorasSIMEPAR Then
                ' MsgBox("Entrou na contagem de intervalos extra: " & ContadorIntervalo & " > " & NumHorasSIMEPAR)
                For mini As Integer = 1 To Total_MiniBacias
                    chuvabin(mini, ContadorIntervalo) = 0
                Next
                Continue For
            End If
            'Faz a interpolação por vizinho mais próximo

            For mini As Integer = 1 To Total_MiniBacias
                chuvabin(mini, ContadorIntervalo) = ChuvaArquivo(PontoMaisPróximoda(mini), ContadorIntervalo)
            Next
        Next
    End Sub

    Sub INTERPOLA_ZERO()

        'Pega as datas na interface
        Dim data_ini As Date 'data inicial atualizada pela interface com base na chuva observada
        data_ini = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim data_fim As Date 'data final atualizada pela interface com base em uma previsão de X dias definida pelo usuário
        data_fim = DateTimePicker2.Value.Day & "/" & DateTimePicker2.Value.Month & "/" & DateTimePicker2.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim diff_t As Integer 'número de intervalos entre data inicial e final
        diff_t = DateDiff(DateInterval.Hour, data_ini, data_fim) + 1

        ReDim chuvabin(Total_MiniBacias, diff_t)

        Dim tmaxx As Integer
        tmaxx = diff_t

        For tempo As Integer = 1 To tmaxx
            For cells As Integer = 1 To Total_MiniBacias
                chuvabin(cells, tempo) = 0
            Next
        Next


    End Sub


End Class