Imports System.IO

Public Class Form10

    Private região(,) As Integer 'Arquivo de regiões, a primeira coluna é o número de regiões e a segunda é o número máximo de mini-bacias em cada região
    Private chuvabin(,) As Single 'Double
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

    'Variáveis de leitura dos dados da SOMAR
    Private opendiagSOMAR As New Windows.Forms.OpenFileDialog
    Private DataIsomar, DataFsomar As Date
    Private NT_SOMAR, NC As Integer
    Private ChuvaMatrizSOMAR(,) As Single 'ChuvaMatriz(NC, NT)

    'Variáveis do novo CHUVABIN
    Public NOVO_NT As Integer 'número de intervalos de tempo



    Private Sub Form10_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        Me.CARREGA_MINI()
        'Me.Dimensiona_Colunas()
        ' Me.CARREGA_REGIOES()

        Me.CARREGA_DIRETÓRIOS()
        Me.Carrega_Hora_Previsão()
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

        NC = Total_MiniBacias
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
            If total_estações = 4 Then
                diretorio_previsao = linha
            End If
        Loop

        Stlb.Text = "Diretório arquivos de previsão: " & diretorio_previsao
        'Fecha
        FileClose()

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

    Sub LOAD_SOMAR_FILES()
        'opendiag.Filter = "Arquivo com chuva interpolada (.mgb;.hig)|*.mgb;*.hig"
        opendiagSOMAR.Reset()
        lbSOMARfiles.Items.Clear()

        opendiagSOMAR.Filter = "Arquivo com chuva em uma hora da SOMAR (.txt;.plu;.asc)|*.txt;*.plu;*.asc"
        opendiagSOMAR.Multiselect = True
        Dim ArquivoSolo As String
        If opendiagSOMAR.ShowDialog = Windows.Forms.DialogResult.OK Then
            For Each s As String In opendiagSOMAR.FileNames
                ArquivoSolo = System.IO.Path.GetFileName(s)
                lbSOMARfiles.Items.Add(ArquivoSolo)
            Next
            tbChuvaArquivo.Text = System.IO.Path.GetDirectoryName(opendiagSOMAR.FileNames(0))

        End If
    End Sub

    Sub ProcessSOMARFiles()
        DataIsomar = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
        DataFsomar = "01/01/" & AI & " " & "00:00:00"

        Dim FileNameDate As String
        Dim DateFileName As Date

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

        tbHSOMARi.Text = DataIsomar.Hour
        If tbHSOMARi.Text.Length = 1 Then tbHSOMARi.Text = "0" & tbHSOMARi.Text 'Caso os tamanhos não fecharem, coloca um zero na frente
        tbHSOMARf.Text = DataFsomar.Hour
        If tbHSOMARf.Text.Length = 1 Then tbHSOMARf.Text = "0" & tbHSOMARf.Text 'Caso os tamanhos não fecharem, coloca um zero na frente
        dtpSOMARi.Value = DataIsomar
        dtpSOMARf.Value = DataFsomar

        NT_SOMAR = (DateDiff(DateInterval.Hour, DataIsomar, DataFsomar) + 1)
        NC = Total_MiniBacias
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

        If rbGFS.Checked = True Then
            iDia = DataString.Substring(23, 2)
            iMes = DataString.Substring(21, 2)
            iAno = DataString.Substring(17, 4)
            iHora = DataString.Substring(25, 2)
        ElseIf rbWRF.Checked = True Then
            iDia = DataString.Substring(19, 2)
            iMes = DataString.Substring(17, 2)
            iAno = DataString.Substring(13, 4)
            iHora = DataString.Substring(21, 2)
        End If

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

    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click
        LOAD_SOMAR_FILES()
        ProcessSOMARFiles()
        MsgBox("Leitura finalizada")
        Button2.Enabled = True
    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        Me.INTERPOLA_ARQUIVO_SOMAR()
        Me.SALVAR_CHUVABIN_AUTOMATICO()
        'Me.SALVAR_CHUVABIN() 'desligado, para usar o diretório automaticamente
        If CheckBox1.Checked Then
            Me.SALVA_CHUVATEXTO()
        End If
        Me.ESCREVER_METADADOS()
        MsgBox("Interpolação finalizada com sucesso!", MsgBoxStyle.Information, "MGB-IPH")
        Me.Close()
    End Sub

    Sub INTERPOLA_ARQUIVO_SOMAR()

        'Pega as datas na interface
        Dim data_ini As Date 'data inicial atualizada pela interface com base na chuva observada
        data_ini = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim data_fim As Date 'data final atualizada pela interface com base em uma previsão de X dias definida pelo usuário
        data_fim = DateTimePicker2.Value.Day & "/" & DateTimePicker2.Value.Month & "/" & DateTimePicker2.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim diff_t As Integer 'número de intervalos entre data inicial e final
        diff_t = DateDiff(DateInterval.Hour, data_ini, data_fim) + 1

        Dim diff_Prev_Final As Integer 'número de intervalos entre data do arquivo do SIMEPAR e final ajustada na interface
        diff_Prev_Final = DateDiff(DateInterval.Hour, DataIsomar, data_fim) + 1

        Dim data_count As Date
        data_count = DataIsomar
        data_count = data_count.AddHours(-1)

        If data_ini < DataIsomar Then
            MsgBox("Arquivo de previsão de chuva carregado inicia após o último intervalo observado", MsgBoxStyle.Critical, "Erro")
            Exit Sub
        End If

        ReDim chuvabin(Total_MiniBacias, diff_t)
        ' MsgBox("Entrou na contagem de intervalos extra: " & ContadorIntervalo & " > " & NumHorasSIMEPAR)
        For mini As Integer = 0 To Total_MiniBacias
            For tempo As Integer = 0 To diff_t
                chuvabin(mini, tempo) = 0
            Next
        Next

        'Contador de intervalos do chuvabin
        Dim ContadorIntervalo As Integer
        ContadorIntervalo = 0

        'Percorre todas as datas, começando na data inicial do arquivo de previsão de chuva e terminando na data final selecionada para a previsão
        For intervalo As Integer = 1 To diff_Prev_Final

            'Verifica a data, e se não for igual a data inicial, ele volta ao começo do loop
            data_count = data_count.AddHours(+1)
            If data_count < data_ini Then
                Continue For
            End If

            'Se é uma data que vamos utilizar, então adicionamos um valor ao contador
            ContadorIntervalo += 1

            'Se a interpolação ultrapassar o tamanho do arquivo de chuva prevista disponível, mantemos os zeros da variável inicializada
            'E saímos do For aqui mesmo
            If intervalo > NT_SOMAR Then
                Exit For
            End If

            'Finalmente, se vamos usar a chuva deste intervalo, atribui ao binário final de chuva
            For mini As Integer = 1 To Total_MiniBacias
                chuvabin(mini, ContadorIntervalo) = ChuvaMatrizSOMAR(mini, intervalo)
            Next
        Next

        '' ''Percorre todas as datas, começando na data inicial do arquivo de previsão de chuva e terminando na data final selecionada para a previsão
        ' ''For intervalo As Integer = 1 To diff_Prev_Final
        ' ''    data_count = data_count.AddHours(+1)
        ' ''    If data_count < data_ini Then
        ' ''        ' MsgBox(data_count & " " & data_ini)
        ' ''        'MsgBox("Pulou uma hora")
        ' ''        'ContadorIntervalo += 1
        ' ''        Continue For
        ' ''    End If
        ' ''    ContadorIntervalo += 1
        ' ''    'Se a interpolação ultrapassar o tamanho do arquivo, colocamos chuva zero
        ' ''    If ContadorIntervalo > NT_SOMAR Then
        ' ''        ' MsgBox("Entrou na contagem de intervalos extra: " & ContadorIntervalo & " > " & NumHorasSIMEPAR)
        ' ''        For mini As Integer = 1 To Total_MiniBacias
        ' ''            chuvabin(mini, ContadorIntervalo) = 0
        ' ''        Next
        ' ''        Continue For
        ' ''    End If
        ' ''    'Faz a interpolação por vizinho mais próximo

        ' ''    For mini As Integer = 1 To Total_MiniBacias
        ' ''        chuvabin(mini, ContadorIntervalo) = ChuvaMatrizSOMAR(mini, ContadorIntervalo)
        ' ''    Next
        ' ''Next
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


End Class