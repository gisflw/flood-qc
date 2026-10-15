Imports System.IO
Imports System.Security.Cryptography
Imports System.Windows.Forms
Imports ICSharpCode.SharpZipLib.Zip
Imports System.IO.Compression
Imports System.Linq
Imports System.Text.RegularExpressions
Imports System.Globalization
Imports System.Collections.Generic



Public Class Form21

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
    Private diretorio_automatico, diretorio_automatico1, diretorio_automatico2, diretorio_automatico3 As String
    Dim ZIP As Integer

    Private Class EntradaChuvaDat
        Public Entrada As ZipArchiveEntry
        Public Modelo As String
        Public NomeSemExtensao As String
        Public DataInicial As Date
        Public DataFinal As Date
    End Class


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
        Me.SALVA_DESAGREGACAO()
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
            If total_estações = 4 Then 'para estreito é 4
                diretorio_previsao = linha
            ElseIf total_estações = 6 Then 'no 6 pega o diretório das previsões automáticas
                diretorio_automatico = linha
            ElseIf total_estações = 11 Then 'no 11 pega o diretório1 das previsões automáticas
                diretorio_automatico1 = linha
            ElseIf total_estações = 12 Then 'no 12 pega o diretório2 das previsões automáticas
                diretorio_automatico2 = linha
            ElseIf total_estações = 13 Then 'no 13 pega o diretório3 das previsões automáticas
                diretorio_automatico3 = linha
            End If
        Loop

        If cb_dir.Text = "1" Then
            diretorio_automatico = diretorio_automatico1
        ElseIf cb_dir.Text = "2" Then
            diretorio_automatico = diretorio_automatico2
        ElseIf cb_dir.Text = "3" Then
            diretorio_automatico = diretorio_automatico3
        End If

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

            DateTimePicker2.Value = datafimpprev.AddDays(15)

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
        ElseIf rbECMWFnew.Checked = True Then
            ''ABRIR_ECMWF_ONS_NOVO()
            ABRIR_ECMWF_ONS_NOVO_OTIMIZADO()
        ElseIf rbChuvaArquivo.Checked = True Then
            ABRIR_ARQUIVO_TEXTO_DE_CHUVA_MANUAL()
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

                Dim dia_arquivo As Integer
                dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), dataFprev(p))

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
                            hora = (dia_arquivo - 1) * 24 + horario
                            ChuvaArquivo(cell, hora) = linha.Substring(14, 6)
                            ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)
                        Next
                    Else 'para quando as latitudes são menores do que 100
                        CoordX(cell) = linha.Substring(0, 6)
                        CoordY(cell) = linha.Substring(7, 6)
                        For horario As Integer = 1 To 24
                            hora = (dia_arquivo - 1) * 24 + horario
                            ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                            ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)
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

                Dim dia_arquivo As Integer
                dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), dataFprev(p))

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
                        hora = (dia_arquivo - 1) * 24 + horario
                        ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                        'MsgBox(ChuvaArquivo(cell, hora) & " " & cell & " " & hora)

                        ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)
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

                Dim dia_arquivo As Integer
                dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), dataFprev(p))

                FileOpen(1, file, OpenMode.Input)
                Dim cell As Integer
                cell = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    cell = cell + 1
                    CoordX(cell) = linha.Substring(0, 6)
                    CoordY(cell) = linha.Substring(7, 6)
                    For horario As Integer = 1 To 24
                        hora = (dia_arquivo - 1) * 24 + horario
                        ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                        'MsgBox(ChuvaArquivo(cell, hora) & " " & cell & " " & hora)

                        ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)
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

    Sub ABRIR_ECMWF_ONS_NOVO()

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

                Dim dia_arquivo As Integer
                dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), dataFprev(p))

                FileOpen(1, file, OpenMode.Input)
                Dim cell As Integer
                cell = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    cell = cell + 1
                    CoordX(cell) = linha.Substring(0, 6)
                    CoordY(cell) = linha.Substring(8, 6)
                    For horario As Integer = 1 To 24
                        hora = (dia_arquivo - 1) * 24 + horario
                        ChuvaArquivo(cell, hora) = linha.Substring(14, 6)
                        'MsgBox(ChuvaArquivo(cell, hora) & " " & cell & " " & hora)

                        ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)
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

    Sub ABRIR_ECMWF_ONS_NOVO_OTIMIZADO()

        ofd.Filter = "Arquivos dat com a grade de chuva prevista (.dat)|*.dat"
        ofd.Multiselect = True

        If ofd.ShowDialog() <> Windows.Forms.DialogResult.OK Then
            Exit Sub
        End If

        Try
            Me.Cursor = Cursors.WaitCursor

            tbChuvaArquivo.Text = ofd.FileName

            Dim arquivos() As String = ofd.FileNames

            ' Opcional, mas recomendado: garante ordem alfabética dos arquivos
            Array.Sort(arquivos)

            Dim T As Integer
            Dim CoordX(), CoordY() As Single
            Dim ContLinhas As Integer = 0
            Dim linha As String

            '------------------------------------------------------------
            ' Conta as linhas do primeiro arquivo
            '------------------------------------------------------------
            Using sr As New System.IO.StreamReader(arquivos(0))
                Do While Not sr.EndOfStream
                    sr.ReadLine()
                    ContLinhas += 1
                Loop
            End Using

            '------------------------------------------------------------
            ' Dimensiona variáveis
            '------------------------------------------------------------
            Dim NumFiles As Integer = arquivos.Length

            Dim DataIprev(NumFiles) As Date
            Dim DataFprev(NumFiles) As Date

            T = NumFiles * 24 + 1

            Dim PontosGrade As Integer = ContLinhas

            ReDim ChuvaArquivo(PontosGrade, T)
            ReDim CoordX(PontosGrade)
            ReDim CoordY(PontosGrade)

            MsgBox("Número de Pontos na Grade: " & PontosGrade &
               " Número de horas: " & T)

            '------------------------------------------------------------
            ' Copia os fatores horários do DataGridView uma única vez
            '------------------------------------------------------------
            Dim fatorHorario(24) As Single

            For horario As Integer = 1 To 24

                Dim valorObj As Object = DataGridView1.Item(1, horario - 1).Value

                If valorObj Is Nothing OrElse valorObj Is DBNull.Value Then
                    fatorHorario(horario) = 0.0F
                Else
                    fatorHorario(horario) = CSng(valorObj) / 100.0F
                End If

            Next

            '------------------------------------------------------------
            ' Leitura dos arquivos
            '------------------------------------------------------------
            Dim p As Integer = 0
            Dim hora As Integer

            For Each file As String In arquivos

                p += 1

                Dim filestring As String
                filestring = System.IO.Path.GetFileNameWithoutExtension(file)

                Dim dia As Integer
                Dim mes As Integer
                Dim ano As Integer

                ' Data inicial do arquivo
                dia = CInt(filestring.Substring(7, 2))
                mes = CInt(filestring.Substring(9, 2))
                ano = CInt(filestring.Substring(11, 2))

                Dim dataInicialArquivoAtual As New DateTime(2000 + ano, mes, dia, 0, 0, 0)

                DataIprev(p) = dataInicialArquivoAtual

                ' Guarda a data inicial do primeiro arquivo
                If p = 1 Then
                    dataIniArquivo = dataInicialArquivoAtual
                End If

                ' Data final do arquivo
                dia = CInt(filestring.Substring(14, 2))
                mes = CInt(filestring.Substring(16, 2))
                ano = CInt(filestring.Substring(18, 2))

                DataFprev(p) = New DateTime(2000 + ano, mes, dia, 0, 0, 0)

                Dim dia_arquivo As Integer
                dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), DataFprev(p))

                Dim cell As Integer = 0

                Using sr As New System.IO.StreamReader(file)

                    Do While Not sr.EndOfStream

                        linha = sr.ReadLine()
                        cell += 1

                        If linha.Length < 20 Then
                            Throw New Exception("Linha inválida no arquivo: " & file &
                                            vbCrLf & "Linha: " & linha)
                        End If

                        ' As coordenadas normalmente são iguais em todos os arquivos.
                        ' Por isso, basta ler no primeiro arquivo.
                        If p = 1 Then
                            CoordX(cell) = CSng(linha.Substring(0, 6))
                            CoordY(cell) = CSng(linha.Substring(8, 6))
                        End If

                        ' Valor diário ou acumulado do ponto da grade
                        Dim valorChuva As Single
                        valorChuva = CSng(linha.Substring(14, 6))

                        For horario As Integer = 1 To 24

                            hora = (dia_arquivo - 1) * 24 + horario

                            If hora >= 1 AndAlso hora <= T Then
                                ChuvaArquivo(cell, hora) = valorChuva * fatorHorario(horario)
                            End If

                        Next

                    Loop

                End Using

            Next

            MsgBox("Data da chuva prevista: " & dataIniArquivo)

            '------------------------------------------------------------
            ' Verifica o ponto que tem a distância mínima a cada minibacia
            '------------------------------------------------------------
            ReDim Distância_Mínima(Total_MiniBacias)
            ReDim PontoMaisPróximoda(Total_MiniBacias)

            For Mini As Integer = 1 To Total_MiniBacias

                Dim menorDist2 As Single = Single.MaxValue
                Dim pontoMaisProximo As Integer = -1

                Dim xMini As Single = X_MINI(Mini)
                Dim yMini As Single = Y_MINI(Mini)

                For pf As Integer = 1 To PontosGrade

                    Dim dx As Single = xMini - CoordX(pf)
                    Dim dy As Single = yMini - CoordY(pf)

                    ' Distância ao quadrado.
                    ' Para comparar qual é menor, não precisa calcular raiz quadrada.
                    Dim dist2 As Single = dx * dx + dy * dy

                    If dist2 < menorDist2 Then
                        menorDist2 = dist2
                        pontoMaisProximo = pf
                    End If

                Next

                ' Guarda a distância real, para manter compatibilidade com o restante do programa
                Distância_Mínima(Mini) = CSng(Math.Sqrt(menorDist2))
                PontoMaisPróximoda(Mini) = pontoMaisProximo

            Next

            '------------------------------------------------------------
            ' Transfere as variáveis mais importantes para variáveis globais
            '------------------------------------------------------------
            NumPontosSIMEPAR = PontosGrade
            NumHorasSIMEPAR = T

            MsgBox("Leitura finalizada")

            Button2.Enabled = True

        Catch ex As Exception

            MsgBox("Erro na leitura dos arquivos ECMWF/ONS:" & vbCrLf & ex.Message,
               MsgBoxStyle.Critical,
               "Erro")

        Finally

            Me.Cursor = Cursors.Default

        End Try

    End Sub

    Sub ABRIR_ARQUIVO_TEXTO_DE_CHUVA_MANUAL()

        ofd.Filter = "Arquivo texto com a grade de chuva prevista interpolada para o mgb (.txt)|*.txt"
        ofd.Multiselect = True

        If ofd.ShowDialog = Windows.Forms.DialogResult.OK Then

            tbChuvaArquivo.Text = ofd.FileName
            Dim T As Integer
            Dim CoordX(), CoordY() As Single

            Dim ContLinhas As Integer
            Dim linha As String
            ContLinhas = Total_MiniBacias


            'Abre um arquivo e conta as linhas para saber o tamanho do arquivo
            For Each S As String In ofd.FileNames
                Dim arquivo As String
                arquivo = S
                FileOpen(1, arquivo, OpenMode.Input)
                T = 0
                Do Until EOF(1)
                    linha = LineInput(1)
                    T += 1
                Loop
                FileClose()
                Exit For
            Next

            'dimensiona as datas
            Dim NumFiles As Integer
            NumFiles = ofd.FileNames.Length
            Dim DataIprev(T) As Date
            Dim dataFprev(T) As Date


            'Dimensiona a variável de chuva
            Dim PontosGrade As Integer
            PontosGrade = Total_MiniBacias
            ReDim ChuvaArquivo(Total_MiniBacias, T)
            ReDim CoordX(Total_MiniBacias), CoordY(Total_MiniBacias)

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

                Dim dia, mes, ano, horario As Integer

                'MsgBox(p & " " & filestring & " " & DataIprev(p) & " " & dataFprev(p))

                FileOpen(1, file, OpenMode.Input)
                Dim cell, tempo As Integer
                cell = 0
                tempo = 0

                Do Until EOF(1)
                    linha = LineInput(1)
                    tempo += 1


                    dia = linha.Substring(0, 4)
                    mes = linha.Substring(4, 4)
                    ano = linha.Substring(10, 4)
                    horario = linha.Substring(14, 4)

                    If tempo = 1 Then
                        dataIniArquivo = dia & "/" & mes & "/" & ano & " " & horario & ":00:00"
                        DataIprev(p) = dia & "/" & mes & "/" & ano & " " & horario & ":00:00"
                    End If

                    dataFprev(p) = dia & "/" & mes & "/" & ano & " " & horario & ":00:00"


                    cell = 0
                    For cell = 1 To Total_MiniBacias
                        'cell = cell + 1
                        CoordX(cell) = X_MINI(cell)
                        CoordY(cell) = Y_MINI(cell)
                        ChuvaArquivo(cell, tempo) = linha.Substring(12 + cell * 6, 6)
                        'MsgBox(tempo & " " & cell)
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

    Sub CARREGA_ARQUIVO_CHUVA_DIRETORIO_AUTOMATICO()
        'Esta subrotina carrega os arquivos de chuva do diretório automático

        'Primeiro verifica qual é o radio button que está marcado

        If rbChuvaZero.Checked = True Then

        ElseIf rbETA40.Checked = True Then
            Me.CARREGA_AUTOMATICAMENTE_ETA40_ONS()
        ElseIf rbGEFS.Checked = True Then
            Me.CARREGA_AUTOMATICAMENTE_GEFS_ONS()
        ElseIf rbECMWF.Checked = True Then
            ' Me.CARREGA_AUTOMATICAMENTE_ECMWF_ONS()
            Me.CARREGA_AUTOMATICAMENTE_ECMWF_ONS_OTIMIZADA()
        ElseIf rbECMWFnew.Checked = True Then
            ' Me.CARREGA_AUTOMATICAMENTE_ECMWF_ONS()
            Me.CARREGA_AUTOMATICAMENTE_ECMWF_ONS_OTIMIZADA()
        ElseIf rbChuvaArquivo.Checked = True Then
            CARREGA_AUTOMATICAMENTE_ARQUIVO_TEXTO_DE_CHUVA_MANUAL()
        End If

    End Sub


    Sub CARREGA_AUTOMATICAMENTE_ETA40_ONS()

        Dim f2() As String = Directory.GetFiles(diretorio_automatico) 'este é o diretório com os arquivos deixados em uma pasta

        tbChuvaArquivo.Text = diretorio_automatico
        Dim T As Integer
        Dim CoordX(), CoordY() As Single

        Dim ContLinhas As Integer
        Dim linha As String


        For i As Integer = 0 To UBound(f2)
            Dim arquivo As String
            arquivo = f2(i)
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
        NumFiles = UBound(f2) + 1
        Dim DataIprev(NumFiles) As Date
        Dim dataFprev(NumFiles) As Date
        T = NumFiles * 24 + 1

        'Dimensiona a variável de chuva
        Dim PontosGrade As Integer
        PontosGrade = ContLinhas
        ReDim ChuvaArquivo(PontosGrade, T)
        ReDim CoordX(PontosGrade), CoordY(PontosGrade)

        'MsgBox("Número de Pontos na Grade: " & PontosGrade & " Número de horas: " & T)

        'Faz a leitura dos arquivos
        Dim p As String
        p = 0
        Dim hora As Integer
        hora = 0

        For i As Integer = 0 To UBound(f2)
            p = p + 1
            Dim file As String
            file = f2(i)
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

            Dim dia_arquivo As Integer
            dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), dataFprev(p))

            FileOpen(1, file, OpenMode.Input)
            Dim cell As Integer
            cell = 0
            Do Until EOF(1)
                linha = LineInput(1)
                cell = cell + 1
                CoordX(cell) = linha.Substring(0, 6)
                CoordY(cell) = linha.Substring(7, 6)
                For horario As Integer = 1 To 24
                    hora = (dia_arquivo - 1) * 24 + horario
                    ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                    'MsgBox(ChuvaArquivo(cell, hora) & " " & cell & " " & hora)

                    'ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                    ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)

                Next
            Loop
            FileClose()
        Next

        'MsgBox("Data da chuva prevista: " & dataIniArquivo)

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

        'MsgBox("Leitura finalizada")
        Button2.Enabled = True

    End Sub

    Sub CARREGA_AUTOMATICAMENTE_GEFS_ONS()

        Dim f2() As String = Directory.GetFiles(diretorio_automatico) 'este é o diretório com os arquivos deixados em uma pasta

        tbChuvaArquivo.Text = diretorio_automatico

        Dim T As Integer
        Dim CoordX(), CoordY() As Single

        Dim ContLinhas As Integer
        Dim linha As String


        'Abre um arquivo e conta as linhas
        For i As Integer = 0 To UBound(f2)
            Dim arquivo As String
            arquivo = f2(i)
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
        NumFiles = UBound(f2) + 1
        Dim DataIprev(NumFiles) As Date
        Dim dataFprev(NumFiles) As Date
        T = NumFiles * 24 + 1

        'Dimensiona a variável de chuva
        Dim PontosGrade As Integer
        PontosGrade = ContLinhas
        ReDim ChuvaArquivo(PontosGrade, T)
        ReDim CoordX(PontosGrade), CoordY(PontosGrade)

        'MsgBox("Número de Pontos na Grade: " & PontosGrade & " Número de horas: " & T)

        'Faz a leitura dos arquivos
        Dim p As String
        p = 0
        Dim hora As Integer
        hora = 0
        Dim acumulahora As Integer
        acumulahora = 0

        For i As Integer = 0 To UBound(f2)
            p = p + 1
            Dim file As String
            file = f2(i)
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

            Dim dia_arquivo As Integer
            dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), dataFprev(p))

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
                        hora = (dia_arquivo - 1) * 24 + horario
                        ChuvaArquivo(cell, hora) = linha.Substring(14, 6)
                        'ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                        ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)

                    Next
                Else 'para quando as latitudes são menores do que 100
                    CoordX(cell) = linha.Substring(0, 6)
                    CoordY(cell) = linha.Substring(7, 6)
                    For horario As Integer = 1 To 24
                        hora = (dia_arquivo - 1) * 24 + horario
                        ChuvaArquivo(cell, hora) = linha.Substring(13, 6)
                        ' ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                        ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)

                    Next
                End If

            Loop
            FileClose()
        Next

        'MsgBox("Data da chuva prevista: " & dataIniArquivo)

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

        'MsgBox("Leitura finalizada")
        Button2.Enabled = True

    End Sub

    Sub CARREGA_AUTOMATICAMENTE_ECMWF_ONS()

        Dim f2() As String = Directory.GetFiles(diretorio_automatico) 'este é o diretório com os arquivos deixados em uma pasta

        tbChuvaArquivo.Text = diretorio_automatico
        Dim T As Integer
        Dim CoordX(), CoordY() As Single

        Dim ContLinhas As Integer
        Dim linha As String


        'Abre um arquivo e conta as linhas
        For i As Integer = 0 To UBound(f2)
            Dim arquivo As String
            arquivo = f2(i)
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
        NumFiles = UBound(f2) + 1
        Dim DataIprev(NumFiles) As Date
        Dim dataFprev(NumFiles) As Date
        T = NumFiles * 24 + 1

        'Dimensiona a variável de chuva
        Dim PontosGrade As Integer
        PontosGrade = ContLinhas
        ReDim ChuvaArquivo(PontosGrade, T)
        ReDim CoordX(PontosGrade), CoordY(PontosGrade)

        'MsgBox("Número de Pontos na Grade: " & PontosGrade & " Número de horas: " & T)

        'Faz a leitura dos arquivos
        Dim p As String
        p = 0
        Dim hora As Integer
        hora = 0
        For i As Integer = 0 To UBound(f2)
            p = p + 1
            Dim file As String
            file = f2(i)
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

            Dim dia_arquivo As Integer
            dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), dataFprev(p))

            FileOpen(1, file, OpenMode.Input)
            Dim cell As Integer
            cell = 0
            Do Until EOF(1)
                linha = LineInput(1)
                cell = cell + 1
                CoordX(cell) = linha.Substring(0, 6)
                CoordY(cell) = linha.Substring(8, 6)
                For horario As Integer = 1 To 24
                    hora = (dia_arquivo - 1) * 24 + horario
                    ChuvaArquivo(cell, hora) = linha.Substring(14, 6)
                    'MsgBox(ChuvaArquivo(cell, hora) & " " & cell & " " & hora)

                    'ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) / 24
                    ChuvaArquivo(cell, hora) = ChuvaArquivo(cell, hora) * (DataGridView1.Item(1, horario - 1).Value / 100)

                Next
            Loop
            FileClose()
        Next

        'MsgBox("Data da chuva prevista: " & dataIniArquivo)

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

        'MsgBox("Leitura finalizada")
        Button2.Enabled = True

    End Sub

    Sub CARREGA_AUTOMATICAMENTE_ECMWF_ONS_OTIMIZADA()

        Try

            Me.Cursor = Cursors.WaitCursor

            ' Busca apenas arquivos .dat na pasta automática
            Dim f2() As String = Directory.GetFiles(diretorio_automatico, "*.dat")

            ' Garante uma ordem consistente dos arquivos
            Array.Sort(f2)

            If f2.Length = 0 Then
                ' Em rotina automática, melhor sair silenciosamente ou registrar em log
                MsgBox("Nenhum arquivo .dat encontrado em: " & diretorio_automatico,
                   MsgBoxStyle.Exclamation,
                   "Arquivos não encontrados")
                Exit Sub
            End If

            tbChuvaArquivo.Text = diretorio_automatico

            Dim T As Integer
            Dim CoordX(), CoordY() As Single
            Dim ContLinhas As Integer = 0
            Dim linha As String

            '------------------------------------------------------------
            ' Conta as linhas do primeiro arquivo
            '------------------------------------------------------------
            Using sr As New System.IO.StreamReader(f2(0))

                Do While Not sr.EndOfStream
                    sr.ReadLine()
                    ContLinhas += 1
                Loop

            End Using

            '------------------------------------------------------------
            ' Dimensiona as variáveis
            '------------------------------------------------------------
            Dim NumFiles As Integer = f2.Length

            Dim DataIprev(NumFiles) As Date
            Dim DataFprev(NumFiles) As Date

            T = NumFiles * 24 + 1

            Dim PontosGrade As Integer = ContLinhas

            ReDim ChuvaArquivo(PontosGrade, T)
            ReDim CoordX(PontosGrade)
            ReDim CoordY(PontosGrade)

            '------------------------------------------------------------
            ' Copia os fatores horários do DataGridView uma única vez
            '------------------------------------------------------------
            Dim fatorHorario(24) As Single

            For horario As Integer = 1 To 24

                Dim valorObj As Object = DataGridView1.Item(1, horario - 1).Value

                If valorObj Is Nothing OrElse valorObj Is DBNull.Value Then
                    fatorHorario(horario) = 0.0F
                Else
                    fatorHorario(horario) = CSng(valorObj) / 100.0F
                End If

            Next

            '------------------------------------------------------------
            ' Faz a leitura dos arquivos
            '------------------------------------------------------------
            Dim p As Integer = 0
            Dim hora As Integer

            For Each file As String In f2

                p += 1

                Dim filestring As String
                filestring = System.IO.Path.GetFileNameWithoutExtension(file)

                Dim dia As Integer
                Dim mes As Integer
                Dim ano As Integer

                ' Data inicial do arquivo
                dia = CInt(filestring.Substring(7, 2))
                mes = CInt(filestring.Substring(9, 2))
                ano = CInt(filestring.Substring(11, 2))

                Dim dataInicialArquivoAtual As New DateTime(2000 + ano, mes, dia, 0, 0, 0)

                DataIprev(p) = dataInicialArquivoAtual

                ' Guarda a data inicial do primeiro arquivo
                If p = 1 Then
                    dataIniArquivo = dataInicialArquivoAtual
                End If

                ' Data final do arquivo
                dia = CInt(filestring.Substring(14, 2))
                mes = CInt(filestring.Substring(16, 2))
                ano = CInt(filestring.Substring(18, 2))

                DataFprev(p) = New DateTime(2000 + ano, mes, dia, 0, 0, 0)

                Dim dia_arquivo As Integer
                dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), DataFprev(p))

                Dim cell As Integer = 0

                Using sr As New System.IO.StreamReader(file)

                    Do While Not sr.EndOfStream

                        linha = sr.ReadLine()
                        cell += 1

                        If linha.Length < 20 Then
                            Throw New Exception("Linha inválida no arquivo: " & file &
                                            vbCrLf & "Linha: " & linha)
                        End If

                        ' As coordenadas da grade normalmente são iguais em todos os arquivos.
                        ' Por isso, lê apenas no primeiro arquivo.
                        If p = 1 Then
                            CoordX(cell) = CSng(linha.Substring(0, 6))
                            CoordY(cell) = CSng(linha.Substring(8, 6))
                        End If

                        Dim valorChuva As Single
                        valorChuva = CSng(linha.Substring(14, 6))

                        For horario As Integer = 1 To 24

                            hora = (dia_arquivo - 1) * 24 + horario

                            If hora >= 1 AndAlso hora <= T Then
                                ChuvaArquivo(cell, hora) = valorChuva * fatorHorario(horario)
                            End If

                        Next

                    Loop

                End Using

            Next

            '------------------------------------------------------------
            ' Verifica o ponto que tem a distância mínima a cada minibacia
            '------------------------------------------------------------
            ReDim Distância_Mínima(Total_MiniBacias)
            ReDim PontoMaisPróximoda(Total_MiniBacias)

            For Mini As Integer = 1 To Total_MiniBacias

                Dim menorDist2 As Single = Single.MaxValue
                Dim pontoMaisProximo As Integer = -1

                Dim xMini As Single = X_MINI(Mini)
                Dim yMini As Single = Y_MINI(Mini)

                For pf As Integer = 1 To PontosGrade

                    Dim dx As Single = xMini - CoordX(pf)
                    Dim dy As Single = yMini - CoordY(pf)

                    ' Distância ao quadrado.
                    ' Para comparar qual é a menor, não precisa calcular raiz quadrada.
                    Dim dist2 As Single = dx * dx + dy * dy

                    If dist2 < menorDist2 Then
                        menorDist2 = dist2
                        pontoMaisProximo = pf
                    End If

                Next

                ' Guarda a distância real, para manter compatibilidade com o resto do programa
                Distância_Mínima(Mini) = CSng(Math.Sqrt(menorDist2))
                PontoMaisPróximoda(Mini) = pontoMaisProximo

            Next

            '------------------------------------------------------------
            ' Transfere as variáveis mais importantes para variáveis globais
            '------------------------------------------------------------
            NumPontosSIMEPAR = PontosGrade
            NumHorasSIMEPAR = T

            Button2.Enabled = True

        Catch ex As Exception

            MsgBox("Erro na leitura automática dos arquivos ECMWF/ONS:" &
               vbCrLf & ex.Message,
               MsgBoxStyle.Critical,
               "Erro")

        Finally

            Me.Cursor = Cursors.Default

        End Try

    End Sub

    Sub CARREGA_AUTOMATICAMENTE_ARQUIVO_TEXTO_DE_CHUVA_MANUAL()

        Dim f2() As String = Directory.GetFiles(diretorio_automatico) 'este é o diretório com os arquivos deixados em uma pasta

        tbChuvaArquivo.Text = diretorio_automatico
        Dim T As Integer
        Dim CoordX(), CoordY() As Single

        Dim ContLinhas As Integer
        Dim linha As String
        ContLinhas = Total_MiniBacias


        'Abre um arquivo e conta as linhas para saber o tamanho do arquivo
        For i As Integer = 0 To UBound(f2)
            Dim arquivo As String
            arquivo = f2(i)
            FileOpen(1, arquivo, OpenMode.Input)
            T = 0
            Do Until EOF(1)
                linha = LineInput(1)
                T += 1
            Loop
            FileClose()
            Exit For
        Next

        'dimensiona as datas
        Dim NumFiles As Integer
        NumFiles = UBound(f2) + 1
        Dim DataIprev(T) As Date
        Dim dataFprev(T) As Date


        'Dimensiona a variável de chuva
        Dim PontosGrade As Integer
        PontosGrade = Total_MiniBacias
        ReDim ChuvaArquivo(Total_MiniBacias, T)
        ReDim CoordX(Total_MiniBacias), CoordY(Total_MiniBacias)

        'MsgBox("Número de Pontos na Grade: " & PontosGrade & " Número de horas: " & T)

        'Faz a leitura dos arquivos
        Dim p As String
        p = 0
        Dim hora As Integer
        hora = 0
        Dim acumulahora As Integer
        acumulahora = 0
        For i As Integer = 0 To UBound(f2)
            p = p + 1
            Dim file As String
            file = f2(i)

            Dim dia, mes, ano, horario As Integer

            'MsgBox(p & " " & filestring & " " & DataIprev(p) & " " & dataFprev(p))

            FileOpen(1, file, OpenMode.Input)
            Dim cell, tempo As Integer
            cell = 0
            tempo = 0

            Do Until EOF(1)
                linha = LineInput(1)
                tempo += 1


                dia = linha.Substring(0, 4)
                mes = linha.Substring(4, 4)
                ano = linha.Substring(10, 4)
                horario = linha.Substring(14, 4)

                If tempo = 1 Then
                    dataIniArquivo = dia & "/" & mes & "/" & ano & " " & horario & ":00:00"
                    DataIprev(p) = dia & "/" & mes & "/" & ano & " " & horario & ":00:00"
                End If

                dataFprev(p) = dia & "/" & mes & "/" & ano & " " & horario & ":00:00"


                cell = 0
                For cell = 1 To Total_MiniBacias
                    cell = cell + 1
                    CoordX(cell) = X_MINI(cell)
                    CoordY(cell) = Y_MINI(cell)
                    ChuvaArquivo(cell, tempo) = linha.Substring(12 + cell * 6, 6)
                Next

            Loop
            FileClose()
        Next

        'MsgBox("Data da chuva prevista: " & dataIniArquivo)

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

        'MsgBox("Leitura finalizada")
        Button2.Enabled = True


    End Sub

    Private Sub Form18_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        Me.CARREGA_MINI()
        'Me.Dimensiona_Colunas()
        ' Me.CARREGA_REGIOES()

        Me.CARREGA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()

        Me.CARREGA_DIRETÓRIOS()
        Me.Carrega_Hora_Previsão()

        Me.CARREGA_DESAGREGACAO()

        Me.CARREGA_RADIOBUTTONS_CHUVA()
        Me.CARREGA_ZIP_INFO()

        'Se o automático estiver ligado
        If TotalCommander = True Then

            Dim log_file As String
            log_file = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Log_operação_automatica.txt"
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Abriu a janela de chuva prevista.")
            FileClose(99)

            If ZIP = 0 Then
                Me.CARREGA_ARQUIVO_CHUVA_DIRETORIO_AUTOMATICO()
            End If

            If ZIP = 1 Then

                Me.CARREGA_AUTOMATICAMENTE_MODELO_CHUVA_ZIP()
            End If

            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Carregou dados de entrada da janela de chuva prevista.")
            FileClose(99)

            Me.APERTA_BOTAO_INTERPOLAR()

            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Interpolou a chuva prevista. Próxima etapa: Vazão Observada.")
            FileClose(99)

            Me.Close()
        End If

    End Sub


    Sub CARREGA_RADIOBUTTONS_CHUVA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Input)

        Dim linha_agenda As String
        linha_agenda = LineInput(1)
        If linha_agenda = "ONS_ETA" Then
            rbETA40.Checked = True
            rbGEFS.Checked = False
            rbECMWF.Checked = False
            rbChuvaZero.Checked = False
            rbChuvaArquivo.Checked = False
            rbECMWFnew.Checked = False

        ElseIf linha_agenda = "ONS_GEFS" Then
            rbETA40.Checked = False
            rbGEFS.Checked = True
            rbECMWF.Checked = False
            rbChuvaZero.Checked = False
            rbChuvaArquivo.Checked = False
            rbECMWFnew.Checked = False

        ElseIf linha_agenda = "ONS_ECMWF" Then
            rbETA40.Checked = False
            rbGEFS.Checked = False
            rbECMWF.Checked = False
            rbChuvaZero.Checked = False
            rbChuvaArquivo.Checked = False
            rbECMWFnew.Checked = True

        ElseIf linha_agenda = "Zero" Then
            rbETA40.Checked = False
            rbGEFS.Checked = False
            rbECMWF.Checked = False
            rbChuvaZero.Checked = True
            rbChuvaArquivo.Checked = False
            rbECMWFnew.Checked = False

        ElseIf linha_agenda = "Arquivo" Then
            rbETA40.Checked = False
            rbGEFS.Checked = False
            rbECMWF.Checked = False
            rbChuvaZero.Checked = False
            rbChuvaArquivo.Checked = True
            rbECMWFnew.Checked = False

        End If
        FileClose()

    End Sub


    Sub CARREGA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "diretorio_chuva_prevista.txt", OpenMode.Input)

        Dim linha_agenda As String
        linha_agenda = LineInput(1)
        cb_dir.Text = linha_agenda
        FileClose()

    End Sub

    Private Sub Button5_Click(sender As Object, e As EventArgs) Handles Button5.Click
        If rbGEFS.Checked = True Then
            'ABRIR_GEFS_ONS()
            Me.ABRIR_MODELO_CHUVA_ZIP_MANUAL()
            MsgBox("Leitura Finalizada")
        ElseIf rbETA40.Checked = True Then
            'ABRIR_ETA40_ONS()
            Me.ABRIR_MODELO_CHUVA_ZIP_MANUAL()
            MsgBox("Leitura Finalizada")
        ElseIf rbECMWF.Checked = True Then
            ' ABRIR_ECMWF_ONS()
            Me.ABRIR_MODELO_CHUVA_ZIP_MANUAL()
            MsgBox("Leitura Finalizada")
        ElseIf rbECMWFnew.Checked = True Then
            ''ABRIR_ECMWF_ONS_NOVO()
            'ABRIR_ECMWF_ONS_NOVO_OTIMIZADO()
            Me.ABRIR_MODELO_CHUVA_ZIP_MANUAL()
            MsgBox("Leitura Finalizada")
        ElseIf rbChuvaArquivo.Checked = True Then
            'ABRIR_ARQUIVO_TEXTO_DE_CHUVA_MANUAL()
        Else
            MsgBox("Selecione uma opção que permita a escolha de arquivos.")
        End If
    End Sub

    Sub CARREGA_DESAGREGACAO()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Desagregacao.csv", OpenMode.Input)

        Dim linha_desagrega As String
        Dim splt

        Do Until EOF(1)
            linha_desagrega = LineInput(1)
            splt = Split(linha_desagrega, ";")

            Dim nova_linha As DataRow = DataSet2.Tables(0).NewRow()
            ' Preenche os valores do registro
            nova_linha.ItemArray = New Object() {splt(0), splt(1)}

            Try
                DataSet2.Tables(0).Rows.Add(nova_linha)
            Catch ex As Exception
                MsgBox(" erro: " & ex.Message)
            End Try
        Loop
        FileClose()

    End Sub

    Sub SALVA_DESAGREGACAO()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo para salvar
        FileOpen(1, pasta_exec & "\" & "Desagregacao.csv", OpenMode.Output)

        For horario As Integer = 1 To 24
            PrintLine(1, (DataGridView1.Item(0, horario - 1).Value & ";" & (DataGridView1.Item(1, horario - 1).Value)))
        Next

        FileClose()

    End Sub



    Sub MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "diretorio_chuva_prevista.txt", OpenMode.Output)
        PrintLine(1, cb_dir.Text)
        FileClose()

    End Sub



    Private Sub cb_dir_SelectedIndexChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_dir.SelectedIndexChanged
        '
    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()
    End Sub

    Sub CARREGA_ECMWF_ONS_DE_ZIP_REGEX(ByVal arquivoZip As String)

        Try

            Me.Cursor = Cursors.WaitCursor

            'tbChuvaArquivo.Text = arquivoZip
            TextBox1.Text = arquivoZip


            Dim T As Integer
            Dim CoordX(), CoordY() As Single
            Dim ContLinhas As Integer = 0
            Dim linha As String

            Using fs As New FileStream(arquivoZip, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)
                Using zip As New ZipArchive(fs, ZipArchiveMode.Read)
                    '------------------------------------------------------------
                    ' Seleciona apenas arquivos .dat com o padrão correto:
                    ' ECMWF_p090726a190726.dat
                    '
                    ' Ignora arquivos como:
                    ' ECMWF_m_090726.dat
                    '------------------------------------------------------------
                    Dim entradasDat = zip.Entries.
                    Where(Function(e)

                              Dim nomeArquivo As String = e.Name
                              Dim nomeSemExtensao As String = Path.GetFileNameWithoutExtension(nomeArquivo)

                              Return nomeArquivo.ToLower().EndsWith(".dat") AndAlso
                                     Regex.IsMatch(nomeSemExtensao, "^ECMWF_p\d{6}a\d{6}$")

                          End Function).
                    OrderBy(Function(e) e.Name).
                    ToList()

                    If entradasDat.Count = 0 Then
                        MsgBox("Nenhum arquivo .dat válido foi encontrado dentro do ZIP." &
                           vbCrLf & "Padrão esperado: ECMWF_p090726a190726.dat",
                           MsgBoxStyle.Exclamation,
                           "Arquivos não encontrados")
                        Exit Sub
                    End If

                    '------------------------------------------------------------
                    ' Conta as linhas do primeiro arquivo .dat válido dentro do ZIP
                    '------------------------------------------------------------
                    Using sr As New StreamReader(entradasDat(0).Open())

                        Do While Not sr.EndOfStream
                            sr.ReadLine()
                            ContLinhas += 1
                        Loop

                    End Using

                    If ContLinhas = 0 Then
                        Throw New Exception("O primeiro arquivo .dat válido dentro do ZIP está vazio.")
                    End If

                    '------------------------------------------------------------
                    ' Dimensiona variáveis
                    '------------------------------------------------------------
                    Dim NumFiles As Integer = entradasDat.Count

                    Dim DataIprev(NumFiles) As Date
                    Dim DataFprev(NumFiles) As Date

                    T = NumFiles * 24 + 1

                    Dim PontosGrade As Integer = ContLinhas

                    ReDim ChuvaArquivo(PontosGrade, T)
                    ReDim CoordX(PontosGrade)
                    ReDim CoordY(PontosGrade)

                    '------------------------------------------------------------
                    ' Copia os fatores horários do DataGridView uma única vez
                    '------------------------------------------------------------
                    Dim fatorHorario(24) As Single

                    For horario As Integer = 1 To 24

                        Dim valorObj As Object = DataGridView1.Item(1, horario - 1).Value

                        If valorObj Is Nothing OrElse valorObj Is DBNull.Value Then
                            fatorHorario(horario) = 0.0F
                        Else
                            fatorHorario(horario) = CSng(valorObj) / 100.0F
                        End If

                    Next

                    '------------------------------------------------------------
                    ' Faz a leitura dos arquivos .dat válidos dentro do ZIP
                    '------------------------------------------------------------
                    Dim p As Integer = 0
                    Dim hora As Integer

                    For Each entrada As ZipArchiveEntry In entradasDat

                        p += 1

                        Dim filestring As String
                        filestring = Path.GetFileNameWithoutExtension(entrada.Name)

                        Dim dia As Integer
                        Dim mes As Integer
                        Dim ano As Integer

                        ' Data inicial do arquivo
                        dia = CInt(filestring.Substring(7, 2))
                        mes = CInt(filestring.Substring(9, 2))
                        ano = CInt(filestring.Substring(11, 2))

                        Dim dataInicialArquivoAtual As New DateTime(2000 + ano, mes, dia, 0, 0, 0)

                        DataIprev(p) = dataInicialArquivoAtual

                        If p = 1 Then
                            dataIniArquivo = dataInicialArquivoAtual
                        End If

                        ' Data final do arquivo
                        dia = CInt(filestring.Substring(14, 2))
                        mes = CInt(filestring.Substring(16, 2))
                        ano = CInt(filestring.Substring(18, 2))

                        DataFprev(p) = New DateTime(2000 + ano, mes, dia, 0, 0, 0)

                        Dim dia_arquivo As Integer
                        dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), DataFprev(p))

                        Dim cell As Integer = 0

                        Using sr As New StreamReader(entrada.Open())

                            Do While Not sr.EndOfStream

                                linha = sr.ReadLine()
                                cell += 1

                                If linha.Length < 20 Then
                                    Throw New Exception("Linha inválida no arquivo dentro do ZIP: " &
                                                    entrada.Name & vbCrLf &
                                                    "Linha: " & linha)
                                End If

                                ' A grade normalmente é a mesma em todos os arquivos.
                                ' Por isso, lê as coordenadas apenas no primeiro arquivo.
                                If p = 1 Then
                                    CoordX(cell) = CSng(linha.Substring(0, 6))
                                    CoordY(cell) = CSng(linha.Substring(8, 6))
                                End If

                                Dim valorChuva As Single
                                valorChuva = CSng(linha.Substring(14, 6))

                                For horario As Integer = 1 To 24

                                    hora = (dia_arquivo - 1) * 24 + horario

                                    If hora >= 1 AndAlso hora <= T Then
                                        ChuvaArquivo(cell, hora) = valorChuva * fatorHorario(horario)
                                    End If

                                Next

                            Loop

                        End Using

                    Next

                    '------------------------------------------------------------
                    ' Verifica o ponto que tem a distância mínima a cada minibacia
                    '------------------------------------------------------------
                    ReDim Distância_Mínima(Total_MiniBacias)
                    ReDim PontoMaisPróximoda(Total_MiniBacias)

                    For Mini As Integer = 1 To Total_MiniBacias

                        Dim menorDist2 As Single = Single.MaxValue
                        Dim pontoMaisProximo As Integer = -1

                        Dim xMini As Single = X_MINI(Mini)
                        Dim yMini As Single = Y_MINI(Mini)

                        For pf As Integer = 1 To PontosGrade

                            Dim dx As Single = xMini - CoordX(pf)
                            Dim dy As Single = yMini - CoordY(pf)

                            ' Distância ao quadrado.
                            ' Para comparar a menor distância, não precisa calcular raiz.
                            Dim dist2 As Single = dx * dx + dy * dy

                            If dist2 < menorDist2 Then
                                menorDist2 = dist2
                                pontoMaisProximo = pf
                            End If

                        Next

                        ' Guarda a distância real para manter compatibilidade com o restante do programa
                        Distância_Mínima(Mini) = CSng(Math.Sqrt(menorDist2))
                        PontoMaisPróximoda(Mini) = pontoMaisProximo

                    Next

                    '------------------------------------------------------------
                    ' Transfere as variáveis mais importantes para variáveis globais
                    '------------------------------------------------------------
                    NumPontosSIMEPAR = PontosGrade
                    NumHorasSIMEPAR = T

                    Button2.Enabled = True

                End Using ' zip
            End Using ' fs

        Catch ex As Exception

            MsgBox("Erro ao ler arquivos ECMWF/ONS dentro do ZIP:" &
                   vbCrLf & ex.Message,
                   MsgBoxStyle.Critical,
                   "Erro")

        Finally

            Me.Cursor = Cursors.Default

        End Try

    End Sub

    Sub ABRIR_ECMWF_ONS_ZIP_MANUAL()

        ofd.Filter = "Arquivos ZIP com previsão ECMWF/ONS (.zip)|*.zip"
        ofd.Multiselect = False

        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            Me.CARREGA_ECMWF_ONS_DE_ZIP_REGEX(ofd.FileName)
        End If

    End Sub

    Sub CARREGA_AUTOMATICAMENTE_ECMWF_ONS_ZIP()

        Try

            Dim arquivosZip() As String
            arquivosZip = Directory.GetFiles(diretorio_automatico, "*.zip")

            If arquivosZip.Length = 0 Then
                MsgBox("Nenhum arquivo ZIP encontrado em:" &
                       vbCrLf & diretorio_automatico,
                       MsgBoxStyle.Exclamation,
                       "ZIP não encontrado")
                Exit Sub
            End If

            ' Seleciona o ZIP mais recente da pasta
            Dim zipMaisRecente As String
            zipMaisRecente = arquivosZip.
                OrderByDescending(Function(f) File.GetLastWriteTime(f)).
                First()

            CARREGA_ECMWF_ONS_DE_ZIP_REGEX(zipMaisRecente)

        Catch ex As Exception

            MsgBox("Erro no carregamento automático do ZIP ECMWF/ONS:" &
                   vbCrLf & ex.Message,
                   MsgBoxStyle.Critical,
                   "Erro")

        End Try

    End Sub

    Private Function TentaExtrairInfoArquivoDat(ByVal nomeSemExtensao As String,
                                            ByRef modelo As String,
                                            ByRef dataInicial As Date,
                                            ByRef dataFinal As Date) As Boolean

        Dim m As Match = Regex.Match(nomeSemExtensao,
                                     "^(ECMWF|ETA40|GEFS)_p(\d{2})(\d{2})(\d{2})a(\d{2})(\d{2})(\d{2})$",
                                     RegexOptions.IgnoreCase)

        If Not m.Success Then
            Return False
        End If

        Dim prefixo As String = m.Groups(1).Value.ToUpperInvariant()

        Select Case prefixo
            Case "ECMWF"
                modelo = "ECMWF_ONS"

            Case "ETA40"
                modelo = "ETA40"

            Case "GEFS"
                modelo = "GEFS"

            Case Else
                Return False
        End Select

        Dim diaI As Integer = CInt(m.Groups(2).Value)
        Dim mesI As Integer = CInt(m.Groups(3).Value)
        Dim anoI As Integer = 2000 + CInt(m.Groups(4).Value)

        Dim diaF As Integer = CInt(m.Groups(5).Value)
        Dim mesF As Integer = CInt(m.Groups(6).Value)
        Dim anoF As Integer = 2000 + CInt(m.Groups(7).Value)

        dataInicial = New DateTime(anoI, mesI, diaI, 0, 0, 0)
        dataFinal = New DateTime(anoF, mesF, diaF, 0, 0, 0)

        Return True

    End Function

    Private Function TentaLerLinhaGrade(ByVal linha As String,
                                    ByRef x As Single,
                                    ByRef y As Single,
                                    ByRef chuva As Single) As Boolean

        If linha Is Nothing Then Return False

        Dim partes() As String
        partes = linha.Trim().Split(New Char() {}, StringSplitOptions.RemoveEmptyEntries)

        If partes.Length < 3 Then
            Return False
        End If

        If Not Single.TryParse(partes(0), NumberStyles.Float, CultureInfo.InvariantCulture, x) Then
            Return False
        End If

        If Not Single.TryParse(partes(1), NumberStyles.Float, CultureInfo.InvariantCulture, y) Then
            Return False
        End If

        If Not Single.TryParse(partes(2), NumberStyles.Float, CultureInfo.InvariantCulture, chuva) Then
            Return False
        End If

        Return True

    End Function

    Sub CARREGA_MODELO_CHUVA_DE_ZIP(ByVal arquivoZip As String)

        Try

            Me.Cursor = Cursors.WaitCursor

            TextBox1.Text = arquivoZip

            Dim T As Integer
            Dim CoordX(), CoordY() As Single
            Dim ContLinhas As Integer = 0
            Dim linha As String

            Using fs As New FileStream(arquivoZip, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)
                Using zip As New ZipArchive(fs, ZipArchiveMode.Read)

                    '------------------------------------------------------------
                    ' Identifica arquivos .dat válidos e o modelo
                    '------------------------------------------------------------
                    Dim entradasValidas As New List(Of EntradaChuvaDat)
                    Dim modelosEncontrados As New HashSet(Of String)(StringComparer.OrdinalIgnoreCase)

                    For Each entrada As ZipArchiveEntry In zip.Entries

                        If Not entrada.Name.ToLowerInvariant().EndsWith(".dat") Then
                            Continue For
                        End If

                        Dim nomeSemExtensao As String
                        nomeSemExtensao = Path.GetFileNameWithoutExtension(entrada.Name)

                        Dim modeloTemp As String = ""
                        Dim dataInicialTemp As Date
                        Dim dataFinalTemp As Date

                        If TentaExtrairInfoArquivoDat(nomeSemExtensao,
                                                      modeloTemp,
                                                      dataInicialTemp,
                                                      dataFinalTemp) Then

                            Dim info As New EntradaChuvaDat()
                            info.Entrada = entrada
                            info.Modelo = modeloTemp
                            info.NomeSemExtensao = nomeSemExtensao
                            info.DataInicial = dataInicialTemp
                            info.DataFinal = dataFinalTemp

                            entradasValidas.Add(info)
                            modelosEncontrados.Add(modeloTemp)

                        End If

                    Next

                    If entradasValidas.Count = 0 Then
                        MsgBox("Nenhum arquivo .dat válido foi encontrado dentro do ZIP." &
                               vbCrLf &
                               "Padrões aceitos:" & vbCrLf &
                               "ECMWF_p090726a190726.dat" & vbCrLf &
                               "ETA40_p080726a090726.dat" & vbCrLf &
                               "GEFS_p080726a120726.dat",
                               MsgBoxStyle.Exclamation,
                               "Arquivos não encontrados")
                        Exit Sub
                    End If

                    If modelosEncontrados.Count > 1 Then
                        MsgBox("O ZIP contém arquivos de mais de um modelo." &
                               vbCrLf &
                               "Separe os arquivos em ZIPs diferentes.",
                               MsgBoxStyle.Exclamation,
                               "Mais de um modelo encontrado")
                        Exit Sub
                    End If

                    Dim modeloZip As String
                    modeloZip = modelosEncontrados.First()

                    Dim entradasDat As List(Of EntradaChuvaDat)
                    entradasDat = entradasValidas.
                        Where(Function(x) x.Modelo = modeloZip).
                        OrderBy(Function(x) x.DataFinal).
                        ToList()

                    '------------------------------------------------------------
                    ' Conta as linhas do primeiro arquivo válido
                    '------------------------------------------------------------
                    Using sr As New StreamReader(entradasDat(0).Entrada.Open())

                        Do While Not sr.EndOfStream

                            linha = sr.ReadLine()

                            If linha IsNot Nothing AndAlso linha.Trim() <> "" Then
                                ContLinhas += 1
                            End If

                        Loop

                    End Using

                    If ContLinhas = 0 Then
                        Throw New Exception("O primeiro arquivo .dat válido dentro do ZIP está vazio.")
                    End If

                    '------------------------------------------------------------
                    ' Dimensiona variáveis
                    '------------------------------------------------------------
                    Dim NumFiles As Integer = entradasDat.Count

                    Dim DataIprev(NumFiles) As Date
                    Dim DataFprev(NumFiles) As Date

                    T = NumFiles * 24 + 1

                    Dim PontosGrade As Integer = ContLinhas

                    ReDim ChuvaArquivo(PontosGrade, T)
                    ReDim CoordX(PontosGrade)
                    ReDim CoordY(PontosGrade)

                    '------------------------------------------------------------
                    ' Copia os fatores horários do DataGridView uma única vez
                    '------------------------------------------------------------
                    Dim fatorHorario(24) As Single

                    For horario As Integer = 1 To 24

                        Dim valorObj As Object = DataGridView1.Item(1, horario - 1).Value

                        If valorObj Is Nothing OrElse valorObj Is DBNull.Value Then
                            fatorHorario(horario) = 0.0F
                        Else
                            fatorHorario(horario) = CSng(valorObj) / 100.0F
                        End If

                    Next

                    '------------------------------------------------------------
                    ' Leitura dos arquivos .dat
                    '------------------------------------------------------------
                    Dim p As Integer = 0
                    Dim hora As Integer

                    For Each info As EntradaChuvaDat In entradasDat

                        p += 1

                        DataIprev(p) = info.DataInicial
                        DataFprev(p) = info.DataFinal

                        If p = 1 Then
                            dataIniArquivo = info.DataInicial
                        End If

                        Dim dia_arquivo As Integer
                        dia_arquivo = DateDiff(DateInterval.Day, DataIprev(p), DataFprev(p))

                        Dim cell As Integer = 0

                        Using sr As New StreamReader(info.Entrada.Open())

                            Do While Not sr.EndOfStream

                                linha = sr.ReadLine()

                                If linha Is Nothing OrElse linha.Trim() = "" Then
                                    Continue Do
                                End If

                                Dim x As Single
                                Dim y As Single
                                Dim valorChuva As Single

                                'If Not TentaLerLinhaGrade(linha, x, y, valorChuva) Then
                                '    Throw New Exception("Linha inválida no arquivo dentro do ZIP: " &
                                '                        info.Entrada.Name & vbCrLf &
                                '                        "Linha: " & linha)
                                'End If

                                Dim leituraNormal As Boolean
                                leituraNormal = TentaLerLinhaGrade(linha, x, y, valorChuva)

                                If Not leituraNormal Then

                                    ' Segunda tentativa: trata casos em que a coordenada Y
                                    ' e a chuva ficaram coladas, por exemplo:
                                    ' -71.70  -37.00100.63
                                    If TentaLerLinhaGradeCamposColados(linha, x, y, valorChuva) Then

                                        ' A linha foi recuperada com sucesso.
                                        ' O código pode continuar normalmente.

                                    Else

                                        Throw New Exception(
            "Linha inválida no arquivo dentro do ZIP: " &
            info.Entrada.Name & vbCrLf &
            "Linha: " & linha
        )

                                    End If

                                End If


                                cell += 1

                                ' A grade normalmente é igual em todos os arquivos.
                                ' Por isso, lê coordenadas apenas no primeiro arquivo.
                                If p = 1 Then
                                    CoordX(cell) = x
                                    CoordY(cell) = y
                                End If

                                For horario As Integer = 1 To 24

                                    hora = (dia_arquivo - 1) * 24 + horario

                                    If hora >= 1 AndAlso hora <= T Then
                                        ChuvaArquivo(cell, hora) = valorChuva * fatorHorario(horario)
                                    End If

                                Next

                            Loop

                        End Using

                    Next

                    '------------------------------------------------------------
                    ' Verifica o ponto que tem a distância mínima a cada minibacia
                    '------------------------------------------------------------
                    ReDim Distância_Mínima(Total_MiniBacias)
                    ReDim PontoMaisPróximoda(Total_MiniBacias)

                    For Mini As Integer = 1 To Total_MiniBacias

                        Dim menorDist2 As Single = Single.MaxValue
                        Dim pontoMaisProximo As Integer = -1

                        Dim xMini As Single = X_MINI(Mini)
                        Dim yMini As Single = Y_MINI(Mini)

                        For pf As Integer = 1 To PontosGrade

                            Dim dx As Single = xMini - CoordX(pf)
                            Dim dy As Single = yMini - CoordY(pf)

                            Dim dist2 As Single = dx * dx + dy * dy

                            If dist2 < menorDist2 Then
                                menorDist2 = dist2
                                pontoMaisProximo = pf
                            End If

                        Next

                        Distância_Mínima(Mini) = CSng(Math.Sqrt(menorDist2))
                        PontoMaisPróximoda(Mini) = pontoMaisProximo

                    Next

                    '------------------------------------------------------------
                    ' Transfere as variáveis mais importantes para variáveis globais
                    '------------------------------------------------------------
                    NumPontosSIMEPAR = PontosGrade
                    NumHorasSIMEPAR = T

                    Button2.Enabled = True

                    If TotalCommander = False Then
                        MsgBox("Leitura finalizada." & vbCrLf &
                          "Modelo identificado: " & modeloZip & vbCrLf &
                          "Arquivos lidos: " & NumFiles & vbCrLf &
                          "Pontos na grade: " & PontosGrade,
                          MsgBoxStyle.Information,
                          "Leitura de previsão")
                    End If


                End Using
            End Using

        Catch ex As Exception

            MsgBox("Erro ao ler arquivos de previsão dentro do ZIP:" &
                   vbCrLf & ex.Message,
                   MsgBoxStyle.Critical,
                   "Erro")

        Finally

            Me.Cursor = Cursors.Default

        End Try

    End Sub

    Private Function TentaLerLinhaGradeCamposColados(
        ByVal linha As String,
        ByRef x As Single,
        ByRef y As Single,
        ByRef valorChuva As Single) As Boolean

        x = 0.0F
        y = 0.0F
        valorChuva = 0.0F

        If String.IsNullOrWhiteSpace(linha) Then
            Return False
        End If

        '------------------------------------------------------------
        ' Primeiro separa a coordenada X do restante da linha.
        '
        ' Exemplo:
        ' -71.70  -37.00100.63
        '
        ' X     = -71.70
        ' Resto = -37.00100.63
        '------------------------------------------------------------
        Dim padraoInicial As String =
        "^\s*(?<x>[+-]?\d+[.,]\d+)\s+(?<resto>.+?)\s*$"

        Dim resultadoInicial As Match
        resultadoInicial = Regex.Match(linha, padraoInicial)

        If Not resultadoInicial.Success Then
            Return False
        End If

        Dim textoX As String
        Dim textoResto As String

        textoX = resultadoInicial.Groups("x").Value
        textoResto = resultadoInicial.Groups("resto").Value.Trim()

        '------------------------------------------------------------
        ' Descobre quantas casas decimais existem na coordenada X.
        ' Normalmente X e Y possuem a mesma quantidade de casas.
        '------------------------------------------------------------
        Dim posicaoSeparador As Integer
        posicaoSeparador = Math.Max(
        textoX.LastIndexOf("."c),
        textoX.LastIndexOf(","c)
    )

        If posicaoSeparador < 0 Then
            Return False
        End If

        Dim numeroCasasCoordenada As Integer
        numeroCasasCoordenada = textoX.Length - posicaoSeparador - 1

        If numeroCasasCoordenada <= 0 Then
            Return False
        End If

        '------------------------------------------------------------
        ' Se X possui duas casas decimais, procura:
        '
        ' Y     = número com exatamente duas casas decimais
        ' Chuva = tudo o que aparece depois
        '
        ' Assim:
        ' -37.00100.63
        '
        ' torna-se:
        ' Y     = -37.00
        ' Chuva = 100.63
        '------------------------------------------------------------
        Dim padraoResto As String =
        "^(?<y>[+-]?\d+[.,]\d{" &
        numeroCasasCoordenada.ToString() &
        "})\s*(?<chuva>[+-]?\d+(?:[.,]\d+)?(?:[Ee][+-]?\d+)?)$"

        Dim resultadoResto As Match
        resultadoResto = Regex.Match(textoResto, padraoResto)

        If Not resultadoResto.Success Then
            Return False
        End If

        Dim textoY As String
        Dim textoChuva As String

        textoY = resultadoResto.Groups("y").Value
        textoChuva = resultadoResto.Groups("chuva").Value

        If Not TentaConverterSingleFlexivel(textoX, x) Then
            Return False
        End If

        If Not TentaConverterSingleFlexivel(textoY, y) Then
            Return False
        End If

        If Not TentaConverterSingleFlexivel(textoChuva, valorChuva) Then
            Return False
        End If

        Return True

    End Function

    Private Function TentaConverterSingleFlexivel(
        ByVal texto As String,
        ByRef valor As Single) As Boolean

        valor = 0.0F

        If String.IsNullOrWhiteSpace(texto) Then
            Return False
        End If

        ' Padroniza vírgula para ponto
        Dim textoPadronizado As String
        textoPadronizado = texto.Trim().Replace(","c, "."c)

        Return Single.TryParse(
        textoPadronizado,
        NumberStyles.Float,
        CultureInfo.InvariantCulture,
        valor
    )

    End Function

    Sub ABRIR_MODELO_CHUVA_ZIP_MANUAL()

        ofd.Filter = "Arquivos ZIP de previsão de chuva (.zip)|*.zip"
        ofd.Multiselect = False

        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            CARREGA_MODELO_CHUVA_DE_ZIP(ofd.FileName)
        End If

    End Sub

    Sub CARREGA_AUTOMATICAMENTE_MODELO_CHUVA_ZIP()


        If rbChuvaZero.Checked = True Then


        Else
            Try

                Dim arquivosZip() As String
                arquivosZip = Directory.GetFiles(diretorio_automatico, "*.zip")

                If arquivosZip.Length = 0 Then
                    MsgBox("Nenhum arquivo ZIP encontrado em:" &
                           vbCrLf & diretorio_automatico,
                           MsgBoxStyle.Exclamation,
                           "ZIP não encontrado")
                    Exit Sub
                End If

                Dim zipMaisRecente As String
                zipMaisRecente = arquivosZip.
                    OrderByDescending(Function(f) File.GetLastWriteTime(f)).
                    First()

                CARREGA_MODELO_CHUVA_DE_ZIP(zipMaisRecente)

            Catch ex As Exception

                MsgBox("Erro no carregamento automático do ZIP de previsão:" &
                       vbCrLf & ex.Message,
                       MsgBoxStyle.Critical,
                       "Erro")

            End Try

        End If




    End Sub

    Sub CARREGA_ZIP_INFO()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_zip.txt", OpenMode.Input)

        Dim linha_zip As String
        linha_zip = LineInput(1)
        If linha_zip = 0 Then
            ZIP = 0
        ElseIf linha_zip = 1 Then
            ZIP = 1
        Else
            ZIP = 0
        End If
        FileClose()

    End Sub


End Class