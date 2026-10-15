Imports System.IO

Public Class Form8
    Private datainicio As Date
    Private dataFim As Date

    Private datainiciosub As Date
    Private dataFimsub As Date

    Private datainiciopob As Date
    Private dataFimpob As Date

    Private datainiciopprev As Date
    Private dataFimpprev As Date

    Private total_mini As Integer

    Private numcels As Integer 'número de células
    Private A_MINI() As Single 'área da minibacia
    Private A_SUB() As Integer 'área da subbacia
    Private Total_MiniBacias As Integer
    Private total_subbacias As Integer

    Dim RodaMC, RodaIN As Integer

    Private diretorio_previsao As String 'diretório onde deve ser salvo o arquivo de chuvas observadas

    Private Sub tb_ini_qob_TextChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles tb_ini_qob.TextChanged

    End Sub

    Private Sub Form8_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        'Cria os diretórios toda a vez que o formulário de simulação é aberto
        If My.Computer.FileSystem.DirectoryExists("c:\mgb-hora") = True Then
            Directory.Delete("c:\mgb-hora", True)
            System.IO.Directory.CreateDirectory("c:\mgb-hora")
            System.IO.Directory.CreateDirectory("c:\mgb-hora\Input")
            System.IO.Directory.CreateDirectory("c:\mgb-hora\Output")
        ElseIf My.Computer.FileSystem.DirectoryExists("c:\mgb-hora") = False Then
            System.IO.Directory.CreateDirectory("c:\mgb-hora")
            System.IO.Directory.CreateDirectory("c:\mgb-hora\Input")
            System.IO.Directory.CreateDirectory("c:\mgb-hora\Output")
        End If
        Try
            Me.CARREGA_MULTIPLICADORES()
            Me.CARREGA_DIRETÓRIOS()
            Me.CARREGAENTRADAS_AUTOMATICO()
            Me.CARREGA_dtp_Prev_hidro()
            Me.CARREGA_MODELO_AUTOMATICO()
        Catch ex As Exception
            MsgBox(ex.Message)
            MsgBox("Verifique os arquivos de entrada", MsgBoxStyle.Information, "MGB-IPH")
        End Try



        Try
            If TotalCommander = True Then

                Dim log_file As String
                log_file = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Log_operação_automatica.txt"
                FileOpen(99, log_file, OpenMode.Append)
                PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Abriu a janela de Rodar Previsão e carregou todos os arquivos de entrada.")
                FileClose(99)


                If RodaMC = 1 Then
                    If lb_status.ForeColor = Drawing.Color.Green Then
                        FileOpen(99, log_file, OpenMode.Append)
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Arquivos de entrada foram verificados com sucesso. Rodada vai iniciar.")
                        FileClose(99)
                        Me.RODAR_PREVISAO_AUTOMATICO()
                        FileOpen(99, log_file, OpenMode.Append)
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Rodada da previsão finalizada.")
                        FileClose(99)

                    Else
                        FileOpen(99, log_file, OpenMode.Append)
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Arquivos de entrada incompatíveis.")
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Previsão abortada. Verifique se todas as etapas foram executadas com sucesso.")
                        FileClose(99)
                        ERRO_PREVISÃO = 2
                        Me.Close()
                    End If
                End If

                If RodaIN = 1 Then
                    If lb_status.ForeColor = Drawing.Color.Green Then
                        FileOpen(99, log_file, OpenMode.Append)
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Arquivos de entrada foram verificados com sucesso. Rodada vai iniciar.")
                        FileClose(99)
                        Me.RODAR_PREVISAO_AUTOMATICO_INERCIAL()
                        FileOpen(99, log_file, OpenMode.Append)
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Rodada da previsão finalizada.")
                        FileClose(99)

                    Else
                        FileOpen(99, log_file, OpenMode.Append)
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Arquivos de entrada incompatíveis.")
                        PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Previsão abortada. Verifique se todas as etapas foram executadas com sucesso.")
                        FileClose(99)
                        ERRO_PREVISÃO = 2
                        Me.Close()
                    End If
                End If


                Me.Close()
            End If
        Catch ex As Exception
            FileClose()
            MsgBox("Erro na previsão!!!")
        End Try

        ''Try
        ''    If TotalCommander = True Then
        ''        If lb_status.ForeColor = Drawing.Color.Green Then
        ''            Me.RODAR_PREVISAO()
        ''            Me.Close()
        ''        Else
        ''            ERRO_PREVISÃO = 2
        ''            Me.Close()
        ''        End If

        ''    End If
        ''Catch ex As Exception
        ''    MsgBox("Erro na previsão!!!")
        ''End Try

    End Sub

    Sub CARREGA_dtp_Prev_hidro()
        dtpPrevHidro.Value = datainiciopprev.Date
        Dim hora As String
        hora = datainiciopprev.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If
        tb_PrevHidro.Text = hora
    End Sub

    Sub verificadataqobs_hora()

        'MsgBox("Vai começar a verificar")
        datainicio = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
        dataFim = "01/01/" & AI & " " & "00:00:00"
        'MsgBox("Arquivo: " & arqvazao)

        FileOpen(3, tb_qob.Text, OpenMode.Input)
        Dim linha As String

        'MsgBox("Abriu!")
        Dim contador As Integer
        contador = 0

        Do Until EOF(3)
            linha = LineInput(3)
            '  MsgBox(linha)

            contador = contador + 1
            If contador > 1 Then
                '     MsgBox(linha)
                If linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00" < datainicio Then
                    datainicio = linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00"
                End If
                If linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00" > dataFim Then
                    dataFim = linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00"
                End If
            End If
        Loop


        Dim dia, mes, hora As String
        dia = datainicio.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datainicio.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datainicio.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        tb_ini_qob.Text = dia & "/" & mes & "/" & datainicio.Year & " " & hora & ":00"

        dia = dataFim.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = dataFim.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = dataFim.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        tb_fim_qob.Text = dia & "/" & mes & "/" & dataFim.Year & " " & hora & ":00"

        FileClose(3)

    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        '  ofd.Multiselect = True
        ofd.Filter = "Arquivo de vazões observadas do MGB-IPH(.MGB)|*.MGB"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            tb_qob.Text = ofd.FileName
            Me.verificadataqobs_hora()
            Me.ATUALIZA_STATUS()
        End If
    End Sub

    Sub ATUALIZA_STATUS()

        Dim arquivo_verifica As String 'no do arquivo com as configurações
        arquivo_verifica = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\verifica.txt"

        If tb_ini_qob.Text = tb_ini_qsubs.Text Then
            If tb_ini_qob.Text = tb_ini_pobs.Text Then
                If tb_fim_qob.Text = tb_fim_qsubs.Text Then
                    If tb_fim_qob.Text = tb_fim_pprev.Text Then

                        Dim data1, data2 As Date
                        data1 = tb_fim_pobs.Text
                        data2 = tb_ini_pprev.Text
                        If data2 = data1.AddHours(1) Then


                            dtpqob1.Value = datainicio
                            dtpqob2.Value = dataFim

                            Dim hora As String
                            hora = datainicio.Hour
                            If hora.Length < 2 Then
                                hora = "0" & hora
                            End If

                            tb_hora_ini.Text = hora

                            hora = dataFim.Hour
                            If hora.Length < 2 Then
                                hora = "0" & hora
                            End If
                            tb_hora_fim.Text = hora


                            lb_status.Text = "Modelo pronto para rodar previsão"
                            lb_status.ForeColor = Drawing.Color.Green

                            Button5.Enabled = True
                            Button6.Enabled = True
                        Else
                            lb_status.Text = "Arquivos de precipitação não são compatíveis."
                            lb_status.ForeColor = Drawing.Color.Red
                        End If
                    Else

                        lb_status.Text = "Arquivos de vazão e chuva prevista não são compatíveis."
                        lb_status.ForeColor = Drawing.Color.Red

                    End If
                Else
                    lb_status.Text = "Arquivos de vazão observada e substituída não são compatíveis."
                    lb_status.ForeColor = Drawing.Color.Red

                End If
            Else
                lb_status.Text = "Arquivos de vazão observada e chuva observada não são compatíveis."
                lb_status.ForeColor = Drawing.Color.Red

            End If
        Else
            lb_status.Text = "Arquivos de vazão observada e substituída não são compatíveis."
            lb_status.ForeColor = Drawing.Color.Red

        End If



    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        '  ofd.Multiselect = True
        ofd.Filter = "Arquivo de vazões substituídas do MGB-IPH(.MGB)|*.MGB"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            tb_qsub.Text = ofd.FileName
            Me.verificadataqsubs_hora()
            Me.ATUALIZA_STATUS()

        End If
    End Sub

    Sub verificadataqsubs_hora()

        'MsgBox("Vai começar a verificar")
        datainiciosub = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
        dataFimsub = "01/01/" & AI & " " & "00:00:00"
        'MsgBox("Arquivo: " & arqvazao)

        FileOpen(3, tb_qsub.Text, OpenMode.Input)
        Dim linha As String

        'MsgBox("Abriu!")
        Dim contador As Integer
        contador = 0

        Do Until EOF(3)
            linha = LineInput(3)
            '  MsgBox(linha)

            contador = contador + 1
            If contador > 1 Then
                '     MsgBox(linha)
                If linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00" < datainiciosub Then
                    datainiciosub = linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00"
                End If
                If linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00" > dataFimsub Then
                    dataFimsub = linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00"
                End If
            End If
        Loop


        Dim dia, mes, hora As String
        dia = datainiciosub.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datainiciosub.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datainiciosub.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        tb_ini_qsubs.Text = dia & "/" & mes & "/" & datainiciosub.Year & " " & hora & ":00"

        dia = dataFimsub.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = dataFimsub.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = dataFimsub.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        tb_fim_qsubs.Text = dia & "/" & mes & "/" & dataFimsub.Year & " " & hora & ":00"

        FileClose(3)

    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        '  ofd.Multiselect = True
        ofd.Filter = "Arquivo de chuva observada interpolada do MGB-IPH(.MGB)|*.MGB"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            tb_pob.Text = ofd.FileName
            'Declara as variáveis
            Dim nome_metadados As String
            nome_metadados = System.IO.Path.GetFileNameWithoutExtension(ofd.FileName)
            Dim diretório As String
            diretório = System.IO.Path.GetDirectoryName(ofd.FileName)

            Dim arquivo_metadados As String
            arquivo_metadados = diretório & "\" & nome_metadados & ".meta"

            FileOpen(1, arquivo_metadados, OpenMode.Input)

            Dim ll As Integer
            ll = 0
            Dim linha As String
            total_mini = 0

            Do Until EOF(1)
                linha = LineInput(1)
                ll = ll + 1
                If ll = 10 Then
                    datainiciopob = linha.Substring(0, 4) & "/" & linha.Substring(8, 2) & "/" & linha.Substring(14, 2) & " " & linha.Substring(20, 2) & ":00:00"
                    Dim addhour As Integer
                    addhour = linha.Substring(22, 8)
                    addhour = addhour - 1
                    dataFimpob = datainiciopob.AddHours(addhour)
                ElseIf ll = 13 Then
                    total_mini = linha
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

            tb_ini_pobs.Text = dia & "/" & mes & "/" & datainiciopob.Year & " " & hora & ":00"

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

            tb_fim_pobs.Text = dia & "/" & mes & "/" & dataFimpob.Year & " " & hora & ":00"
            FileClose()


            Me.ATUALIZA_STATUS()


        End If




    End Sub

    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click
        ofd.Filter = "Arquivo de chuva prevista interpolada do MGB-IPH(.mgb;.bin)|*.mgb;*.bin"
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
            tb_pprev.Text = ofd.FileName
            'Declara as variáveis
            Dim nome_metadados As String
            nome_metadados = System.IO.Path.GetFileNameWithoutExtension(ofd.FileName)
            Dim diretório As String
            diretório = System.IO.Path.GetDirectoryName(ofd.FileName)

            Dim arquivo_metadados As String
            arquivo_metadados = diretório & "\" & nome_metadados & ".meta"

            'arquivo padrão com nome dado pelo Siqueira:
            arquivo_metadados = diretório & "\Previsao.meta"
            Dim diretóriodoprojeto As String
            diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input"
            File.Copy(arquivo_metadados, diretóriodoprojeto & "\Previsao.meta", True)
            '-- Copiou o arquivo com o nome dado pelo Siqueira

            FileOpen(1, arquivo_metadados, OpenMode.Input)

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
                    dataFimpprev = datainiciopprev.AddHours(addhour)
                End If
            Loop

            Dim dia, mes, hora As String
            dia = datainiciopprev.Day
            If dia.Length < 2 Then
                dia = "0" & dia
            End If
            mes = datainiciopprev.Month
            If mes.Length < 2 Then
                mes = "0" & mes
            End If
            hora = datainiciopprev.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If

            tb_ini_pprev.Text = dia & "/" & mes & "/" & datainiciopprev.Year & " " & hora & ":00"

            dia = dataFimpprev.Day
            If dia.Length < 2 Then
                dia = "0" & dia
            End If
            mes = dataFimpprev.Month
            If mes.Length < 2 Then
                mes = "0" & mes
            End If
            hora = dataFimpprev.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If

            tb_fim_pprev.Text = dia & "/" & mes & "/" & dataFimpprev.Year & " " & hora & ":00"

            FileClose()

            Me.ATUALIZA_STATUS()


        End If

    End Sub

    Sub COMPLETA_CHUVABIN()
        Dim data1, data2 As Date
        data1 = tb_ini_pobs.Text
        data2 = tb_fim_pobs.Text

        Dim tmaxx1 As Integer
        tmaxx1 = (DateDiff(DateInterval.Hour, data1, data2) + 1)

        tchuvabin1 = tmaxx1

        ReDim chuvabin1(total_mini, tmaxx1)
        'Dim chuvabin1(total_mini, tmaxx1) As Double

        Dim data3, data4 As Date
        data3 = tb_ini_pprev.Text
        data4 = tb_fim_pprev.Text

        Dim tmaxx2 As Integer
        tmaxx2 = (DateDiff(DateInterval.Hour, data3, data4) + 1)

        ReDim chuvabin2(total_mini, tmaxx2)
        ' Dim chuvabin2(total_mini, tmaxx2) As Double

        'Abre o arquivo com a chuva 1 (observada)
        Dim nomeArquivo As String
        nomeArquivo = tb_pob.Text

        ' Retorna um texto do arquivo definido
        Dim linhaTexto As String
        linhaTexto = ""

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Open)
        Dim bw As New System.IO.BinaryReader(fs)

        '  FileOpen(2, "teste.txt", OpenMode.Output, OpenAccess.Write)

        Dim numcells, cell, tempobin As Integer
        numcells = total_mini
        cell = 0
        tempobin = tmaxx1

        For tempo As Integer = 1 To tempobin
            For cells As Integer = 1 To numcells

                chuvabin1(cells, tempo) = bw.ReadSingle

            Next
        Next
        bw.Close()

        'MsgBox("Chuva1 leu!")

        'Abre o arquivo com a chuva 2 (prevista)
        Dim nomeArquivo2 As String
        nomeArquivo2 = tb_pprev.Text

        ' Retorna um texto do arquivo definido
        'Dim linhaTexto As String
        linhaTexto = ""

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs2 As FileStream
        fs2 = New FileStream(nomeArquivo2, FileMode.Open)
        Dim bw2 As New System.IO.BinaryReader(fs2)

        'Variável com a chuva no horizonte de previsão
        Chuva_no_horizonte = 0
        '  FileOpen(2, "teste.txt", OpenMode.Output, OpenAccess.Write)

        numcells = total_mini
        cell = 0
        tempobin = tmaxx2

        'For tempo As Integer = 1 To tempobin
        '    For cells As Integer = 1 To numcells

        '        chuvabin2(cells, tempo) = bw2.ReadSingle

        '        Chuva_no_horizonte = Chuva_no_horizonte + chuvabin2(cells, tempo)
        '    Next
        'Next
        'bw2.Close()

        'Fator de correção para a chuva prevista
        Dim FC, FC2, FC3 As Double
        FC = tbFATOR.Text
        FC2 = tbFator2.Text
        FC3 = tbFATOR3.Text

        For tempo As Integer = 1 To tempobin
            For cells As Integer = 1 To numcells

                'Faz a leitura
                chuvabin2(cells, tempo) = bw2.ReadSingle

                'Aplica o fator de correção
                If tempo <= 24 Then ' Primeiras 24h
                    chuvabin2(cells, tempo) = FC * chuvabin2(cells, tempo)
                ElseIf tempo > 24 And tempo <= 48 Then 'Das 24 até as 48
                    chuvabin2(cells, tempo) = FC2 * chuvabin2(cells, tempo)
                ElseIf tempo > 48 Then 'Demais horas
                    chuvabin2(cells, tempo) = FC3 * chuvabin2(cells, tempo)
                End If

                'Calcula chuva no horizonte de previsão
                Chuva_no_horizonte = Chuva_no_horizonte + chuvabin2(cells, tempo)
            Next
        Next
        bw2.Close()


        'MsgBox("Chuva2 leu!")
        'Calcula a chuva média no horizonte de previsão
        Chuva_no_horizonte = Chuva_no_horizonte / numcells

        'Escreve o novo arquivo binário

        Dim nomeArquivo3 As String
        nomeArquivo3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\CHUVABIN.hig"

        If My.Computer.FileSystem.FileExists(nomeArquivo3) = True Then
            File.Delete(nomeArquivo3)
        End If

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs3 As FileStream
        fs3 = New FileStream(nomeArquivo3, FileMode.Create, FileAccess.ReadWrite)
        Dim bw3 As New System.IO.BinaryWriter(fs3)

        'MsgBox("Abriu o Arquivo")

        Dim tmaxx3 As Integer
        tmaxx3 = (DateDiff(DateInterval.Hour, data1, data4) + 1)

        For tempo As Integer = 1 To tmaxx3
            For cells As Integer = 1 To total_mini
                Dim numero As Single

                If tempo <= tmaxx1 Then
                    numero = chuvabin1(cells, tempo)

                ElseIf tempo > tmaxx1 Then
                    numero = chuvabin2(cells, tempo - tmaxx1)

                End If

                bw3.Write(numero)
            Next
        Next

        bw3.Close()

        'MsgBox("Arquivo salvo com sucesso!", MsgBoxStyle.Information)

    End Sub

    Private Sub Button5_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button5.Click
        Me.RODAR_PREVISAO()
        MsgBox("Previsão realizada com sucesso!", MsgBoxStyle.Exclamation, "MGB")
        Me.Close()
    End Sub

    Sub RODAR_PREVISAO()
        Me.COMPLETA_CHUVABIN() 'cola os dois CHUVABIN, copia na pasta input do plugin
        Me.COPIA_VAZOES() 'copia o QSUBS e o QOBS do MGB para a pasta de input do plugin
        Me.ESCRVE_PARHIG_HORA_V2() 'escreve o parhig e coloca na pasta input do plugin
        Me.ESCREVE_uhe_tocantins() 'escreve o arquivo Uhe_tocantins.MGB e coloca na pasta input do plugin
        Me.COPIA_DADOS_ESTAÇÕES_RESERV() 'copia os arquivos dos reservatórios para a pasta c:/mgb-hora/input

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        ' Escreve tudo na pasta c:\mgb
        Dim f() As String = Directory.GetFiles(diretóriodoprojeto & "\Input")
        For i As Integer = 0 To UBound(f)
            ' File.Copy(f(i), Environment.GetFolderPath(Environment.SpecialFolder.Desktop) & "\Input" & "\" & System.IO.Path.GetFileName(f(i)))
            File.Copy(f(i), "c:\mgb-hora\Input" & "\" & System.IO.Path.GetFileName(f(i)), True)

        Next

        Me.RODAFORTRAN() 'Chama o MGB Fortran

        Try 'copia tudo da pasta c:\mgb

            'Antes de copiar, deleta a pasta output original
            Me.RESET_OUPUT()

            Dim f2() As String = Directory.GetFiles("c:\mgb-hora\Output")
            For i As Integer = 0 To UBound(f2)
                File.Copy(f2(i), diretóriodoprojeto & "\Output" & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
            ' If
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta c:/mgb-hora/output para a pasta do projeto")
        End Try

        ' Try
        'Salva os arquivos de saída na memória
        'Me.SalvaresultadoNovo_hora()

        If CheckBox1.Checked = True Then
            Me.SalvaresultadoQobs()

            Me.SalvaHidrogramas_Binários()

            ' Me.REMOVE_VIES()

            Me.COPIA_QSUBST_OUTPUT()

            Me.CARREGA_MINI()
            Me.INTERPOLA_CHUVA_SUB()

            Me.LeLimiares() 'Faz a leitura dos limiares - FMF em 01 de Set de 2018
        End If

       
        'Catch ex As Exception
        '    MsgBox("Problema ao salvar os resultados na memória", MsgBoxStyle.Critical, "Erro")
        'End Try

    End Sub

    Sub COPIA_VAZOES()
        Dim arq_qob, arq_qsub As String
        arq_qob = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\QOBS_hora.PRN"
        arq_qsub = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\QSUBST_hora.PRN"

        File.Copy(tb_qob.Text, arq_qob, True)
        File.Copy(tb_qsub.Text, arq_qsub, True)
    End Sub


    Sub ESCRVE_PARHIG_HORA() ' escreve o PARHIG
        'Declara as variáveis deste projeto de previsão

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        Dim NC As Integer
        NC = total_mini

        Dim NU, NB, NCLI As Integer
        NU = 6
        NB = 40
        NCLI = 23

        Dim Nqobsf As Integer
        Nqobsf = 10

        Dim celpostoscomdados As String
        celpostoscomdados = "163  201  341  256  355  172  257  238  349  285"

        Dim numphd As Integer
        numphd = 10

        Dim Nqsubstf As Integer
        Nqsubstf = 3


        'Começa a escrever o Parhig


        FileOpen(1, diretóriodoprojeto & "\Input\PARHIG.hig", OpenMode.Output) 'cria e abre o parhig para edição
        PrintLine(1, "ARQUIVO DE INFORMACOES GERAIS PARA O MODELO DE GRANDES BACIAS") 'vai escrevendo...
        PrintLine(1, "!")
        PrintLine(1, "Projeto " & "Estreito")
        PrintLine(1, "!")
        PrintLine(1, "       DIA       MES       ANO      HORA          !INÍCIO DA SIMULAÇÃO")
        Print(1, "        ")
        'Escreve a data inicial...
        If dtpqob1.Value.Day.ToString.Length = 1 Then
            Print(1, "0")
        End If
        Print(1, dtpqob1.Value.Day.ToString)
        Print(1, "       ")
        If dtpqob1.Value.Month.ToString.Length = 1 Then
            Print(1, "0")
        End If
        Print(1, dtpqob1.Value.Month.ToString)
        Print(1, "       ")
        Print(1, dtpqob1.Value.Year.ToString)
        Print(1, "        ")
        Print(1, tb_hora_ini.Text)
        PrintLine(1)
        PrintLine(1)
        PrintLine(1, "        NT        DT       !NUMERO DE INTERVALOS DE TEMPO E TAMANHO DO INTERVALO EM SEGUNDOS")
        'Calcula o tempo da simulação...

        'Calcula datas
        Dim dataO As Date
        Dim dataE As Date
        dataO = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        dataE = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tamanhodoint As Integer
        tamanhodoint = DateDiff(DateInterval.Hour, dataO, dataE) + 1

        If tamanhodoint >= 10000 And tamanhodoint < 100000 Then 'dependendo do tamanho do intervalo de tempo, ajusta os espaços
            Print(1, "     " & tamanhodoint)
        ElseIf tamanhodoint >= 1000 And tamanhodoint < 10000 Then
            Print(1, "      " & tamanhodoint)
        ElseIf tamanhodoint >= 100 And tamanhodoint < 1000 Then
            Print(1, "       " & tamanhodoint)
        ElseIf tamanhodoint >= 10 And tamanhodoint < 100 Then
            Print(1, "        " & tamanhodoint)
        ElseIf tamanhodoint >= 100000 And tamanhodoint < 1000000 Then
            Print(1, "    " & tamanhodoint)
        ElseIf tamanhodoint >= 1000000 And tamanhodoint < 10000000 Then
            Print(1, "   " & tamanhodoint)
        End If

        PrintLine(1, "     3600.") 'uma hora tem 3600 segundos
        PrintLine(1)
        PrintLine(1, "        NC        NU        NB      NCLI     !NUMERO DE CÉLULAS, USOS, BACIAS E POSTOS CLIMA")


        'Escreve os N's, respeitanod os espaços...
        If NC <= 1 Then 'dependendo do tamanho, ele escreve o espaço...
            Print(1, "         " & NC)
        ElseIf NC >= 10 And NC < 100 Then
            Print(1, "        " & NC)
        ElseIf NC >= 100 And NC < 1000 Then
            Print(1, "       " & NC)
        ElseIf NC >= 1000 And NC < 10000 Then
            Print(1, "      " & NC)
        ElseIf NC >= 10000 Then
            Print(1, "     " & NC)
        End If

        If NU < 10 Then 'dependendo do tamanho, ele escreve o espaço...
            Print(1, "         " & NU)
        ElseIf NU >= 10 Then
            Print(1, "        " & NU)
        End If
        If NB < 10 Then 'dependendo do tamanho, ele escreve o espaço...
            Print(1, "         " & NB)
        ElseIf NB >= 10 And NB < 100 Then
            Print(1, "        " & NB)
        ElseIf NB >= 100 Then
            Print(1, "       " & NB)
        End If
        If NCLI < 10 Then 'dependendo do tamanho, ele escreve o espaço...
            Print(1, "        -" & NCLI)
        ElseIf NCLI >= 10 And NCLI < 100 Then
            Print(1, "       -" & NCLI)
        ElseIf NCLI >= 100 And NCLI < 1000 Then
            Print(1, "      -" & NCLI)
        ElseIf NCLI >= 1000 And NCLI < 10000 Then
            Print(1, "     -" & NCLI)
        ElseIf NCLI >= 10000 And NCLI < 100000 Then
            Print(1, "    -" & NCLI)
        ElseIf NCLI >= 100000 And NCLI < 1000000 Then
            Print(1, "   -" & NCLI)
        ElseIf NCLI >= 1000000 And NCLI < 10000000 Then
            Print(1, "  -" & NCLI)
        End If
        PrintLine(1) 'Fecha a linha
        PrintLine(1) 'Espaço em branco
        'Escreve se vai ser calibração automática ou manual
        PrintLine(1, "    ICALIB							!INDICA SE VAI USAR CALIBRAÇÃO AUTOMÁTICA (1) OU NÃO (0) ")
        'If rbauto.Checked Then
        'PrintLine(1, "         1	                        !OU SE VAI FAZER PREVISAO (2)")
        'ElseIf rbmanu.Checked Then
        'PrintLine(1, "         0	                        !OU SE VAI FAZER PREVISAO (2)")
        'End If

        PrintLine(1, "         0	                        !OU SE VAI FAZER PREVISAO (2)")

        'CLIMA DIÁRIO NÃO ESTÁ SENDO USADO!!!

        PrintLine(1)
        PrintLine(1, "NOME DOS ARQUIVOS QUE CONTEM OS DADOS METEOROLÓGICOS DIÁRIOS ")

        '  PrintLine(1)
        ' PrintLine(1, "NOME DOS ARQUIVOS QUE CONTEM OS DADOS METEOROLÓGICOS DIÁRIOS ")
        'FileOpen(2, diretóriodoprojeto & "\clidia_" & nomedoprojeto & ".mgb", OpenMode.Input)
        'Dim dadocli, arqcli As String
        'Do Until EOF(2)
        'dadocli = LineInput(2)
        'arqcli = System.IO.Path.GetFileName(dadocli)
        'PrintLine(1, "p" & arqcli)

        'Loop
        'FileClose(2)

        'E segue escrevendo...
        PrintLine(1)

        PrintLine(1, "!NOME DO ARQUIVO QUE CONTEM OS DADOS METEOROLÓGICOS MÉDIOS MENSAIS")
        PrintLine(1, "medias.cli ") 'média climáticas
        PrintLine(1)
        PrintLine(1, "!NUMERO DE POSTOS COM VAZÃO OBSERVADA, NOME DO ARQUIVO EM QUE ESTÃO OS DADOS ")
        PrintLine(1, Nqobsf.ToString & "  QOBS_hora.PRN") 'número de Qobs e o nome do arquivo
        PrintLine(1)
        PrintLine(1, "!NUMEROS DAS CÉLULAS QUE CORRESPONDEM AOS POSTOS FLU COM DADOS                          ")
        PrintLine(1, celpostoscomdados) 'células que corrempondem às Qobs
        PrintLine(1)
        PrintLine(1, "!NUMERO DE PONTOS EM QUE SE DESEJA GRAVAR HIDROGRAMAS")
        PrintLine(1, numphd.ToString) 'pontos onde vão ser gravados os hidrogramas
        PrintLine(1)
        PrintLine(1, "!CÉLULAS QUE CORRESPONDEM A ESTES PONTOS ") 'células que correpondem aos pontos vêm do datagrid

        PrintLine(1, "163  163_Farinha.txt")
        PrintLine(1, "201  201_Itacaja.txt")
        PrintLine(1, "341  341_Tupiratins.txt")
        PrintLine(1, "256  256_Goiatins.txt")
        PrintLine(1, "355  355_Tocantinopolis.txt")
        PrintLine(1, "172  172_PonteAlta.txt")
        PrintLine(1, "257  257_Jatoba.txt")
        PrintLine(1, "238  238_RioDasBalsas.txt")
        PrintLine(1, "349  349_Carolina.txt")
        PrintLine(1, "285  285_PortoReal.txt")
        PrintLine(1)
        'Escreve as coisas da Qsubst
        PrintLine(1, "!Número de células onde a vazão calc deve ser subst. pela vazão lida de arquivo e nome do arquivo")
        PrintLine(1, Nqsubstf & "  QSUBST_hora.PRN")
        PrintLine(1)
        PrintLine(1, "!Células cuja vazão será subsituída")
        PrintLine(1, "353")
        PrintLine(1, "329")
        PrintLine(1, "300")
        FileClose(1)


    End Sub


    Sub RODAFORTRAN()
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "MGB_Hora.exe"
        If cbAssimila.Checked = True Then
            ExecutavelFORTRAN = "MGB_Hora_Assimila.exe"
        End If

        ' Create new process to run setup
        Dim processRun As New Process
        ' Set process props
        processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\" & ExecutavelFORTRAN
        ' Start the process
        processRun.Start()
        ' Wait for the process to finish
        processRun.WaitForExit()

    End Sub

    Sub RESET_OUPUT()
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        If My.Computer.FileSystem.DirectoryExists(diretóriodoprojeto & "\Output") = True Then
            Directory.Delete(diretóriodoprojeto & "\Output", True)
            System.IO.Directory.CreateDirectory(diretóriodoprojeto & "\Output")
        ElseIf My.Computer.FileSystem.DirectoryExists(diretóriodoprojeto & "\Output") = False Then
            System.IO.Directory.CreateDirectory(diretóriodoprojeto & "\Output")
        End If

    End Sub


    Sub SalvaresultadoNovo_hora()
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"
        'Dim linha As String

        'Declara as variáveis para salvar os dados
        Dim Nqobsf As Integer
        Nqobsf = 10

        Dim celpostoscomdados As String
        celpostoscomdados = "258 238 286 342 201 172 257 163 349 361"

        'Lê as variáveis dos postos com dados
        FileOpen(15, diretóriodoprojeto & "\Input\Parhig.hig", OpenMode.Input)
        Dim linha As String
        Dim ll As Integer
        ll = 0
        Do Until EOF(15)
            linha = LineInput(15)
            ll = ll + 1
            If ll = 23 Then
                Nqobsf = linha.Substring(0, 2)
            ElseIf ll = 26 Then
                celpostoscomdados = linha
            End If
        Loop
        FileClose(15)


        'celpostoscomdados
        'Nqobsf

        'Variável para saber quantas mini bacias temos
        Dim QuantMini As Integer
        QuantMini = total_mini


        Dim data1, data2 As Date
        data1 = tb_ini_qob.Text
        data2 = tb_fim_qob.Text

        Dim tamanhodoint As Integer
        tamanhodoint = (DateDiff(DateInterval.Hour, data1, data2) + 1)

        'MsgBox(QuantMini & "  " & tamanhodoint)
        'Redimendiona os arquivos de vazão e qprop
        ReDim QPROPHIG(QuantMini, tamanhodoint + 1), VazaoHIG(QuantMini, tamanhodoint + 1)

        'Escreve código de falha nos dois arquivos

        For mbac As Integer = 1 To QuantMini
            For tint As Integer = 1 To tamanhodoint + 1
                QPROPHIG(mbac, tint) = -1
                VazaoHIG(mbac, tint) = -1
            Next
        Next

        ' MsgBox("Zerou as variáveis")

        'MsgBox("Mbac = " & QuantMini)


        'Variável que contém quais arquivos de vazão temos
        Dim Temosdados(Nqobsf)

        'MsgBox(celpostoscomdados)

        'Preenche a variável acima
        Dim splt
        For nqo As Integer = 1 To Nqobsf
            splt = Split(celpostoscomdados, " ")
            Temosdados(nqo) = splt(nqo - 1)
        Next
        'MsgBox("Preencheu a variável que temos dados, temos essa quantia: " & Nqobsf)


        'Agora temos dois arrays prontos para serem preenchidos e aonde eles devem ser preenchidos

        'VAriáveis para leitura
        Dim linhavazao, linhaqprop As String
        Dim diadavazao As Integer
        Dim startpoint As Integer 'Variável para saber onde começa a ler
        startpoint = 2

        ' MsgBox("Criou os startpoints")

        'Para cada minibacia que temos dados abrimos os arquivos e preenchemos os arrays
        For nqo As Integer = 1 To Nqobsf
            startpoint = startpoint + 15 ' O espaço entre as vazãoes no arquivo de saída é de 15espaços
            FileOpen(11, diretóriodoprojeto & "\Output\VAZAO.HIG", OpenMode.Input)
            'FileOpen(12, "QPROP.HIG", OpenMode.Input)
            diadavazao = 0
            '          MsgBox("Abriu os arquivos vazao")
            '         MsgBox(Temosdados(nqo))
            Do Until EOF(11)
                'If nqo > 1 Then
                'MsgBox(linhavazao)
                'MsgBox(linhavazao.Substring(startpoint, 7))
                'End If
                linhavazao = LineInput(11)
                'linhaqprop = LineInput(12)
                diadavazao = diadavazao + 1
                VazaoHIG(Temosdados(nqo), diadavazao) = linhavazao.Substring(startpoint - 2, 9)
                'QPROPHIG(Temosdados(nqo), diadavazao) = linhaqprop.Substring(startpoint, 7)
            Loop
            FileClose()
            '        MsgBox("Leu a " & Temosdados(nqo) & "ª")
        Next

        ' MsgBox("Começou a ler o Qobs")

        'Fazer o mesmo para o Qobs.PRN agora
        startpoint = 12
        For nqo As Integer = 1 To Nqobsf
            startpoint = startpoint + 12 ' O espaço entre as vazãoes no arquivo de saída é de 15espaços
            'FileOpen(11, "VAZAO.HIG", OpenMode.Input)
            FileOpen(12, diretóriodoprojeto & "\Input\QOBS_hora.PRN", OpenMode.Input)
            diadavazao = -1
            'MsgBox("Abriu os arquivos vazao e q prop")
            'MsgBox(Temosdados(nqo))
            Do Until EOF(12)
                'If nqo > 1 Then
                'MsgBox(linhavazao)
                'MsgBox(linhavazao.Substring(startpoint, 7))
                'End If
                'linhavazao = LineInput(11)
                linhaqprop = LineInput(12)
                diadavazao = diadavazao + 1
                If diadavazao > 0 Then
                    QPROPHIG(Temosdados(nqo), diadavazao) = linhaqprop.Substring(startpoint + 3, 9)

                End If
                'VazaoHIG(Temosdados(nqo), diadavazao) = linhavazao.Substring(startpoint, 7)
            Loop
            FileClose()
        Next

        Começoresultado = tb_ini_pobs.Text
        tamdoresultado = tamanhodoint

    End Sub

    Sub SalvaHidrogramasSaídaNovo_hora() 'Essa rotina faz a leitura do Qtudo.bin

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"
        'Dim linha As String

        'Variável para saber quantas mini bacias temos
        Dim QuantMini As Integer
        QuantMini = total_mini

        Dim data1, data2 As Date
        data1 = tb_ini_qob.Text
        data2 = tb_fim_qob.Text

        Dim tamanhodoint As Integer
        tamanhodoint = (DateDiff(DateInterval.Hour, data1, data2) + 1)
        'MsgBox("Calculou o tempo")

        'Redimendiona os arquivos de vazão e qprop
        ReDim VazaoMini(QuantMini, tamanhodoint + 1)

        'Escreve código de falha nos dois arquivos

        For mbac As Integer = 1 To QuantMini
            For tint As Integer = 1 To tamanhodoint + 1
                VazaoMini(mbac, tint) = -1
            Next
        Next


        'Abre o arquivo com a vazão
        Dim nomeArquivo As String
        nomeArquivo = diretóriodoprojeto & "\Output\QTUDO_bin.MGB"

        ' Retorna um texto do arquivo definido
        Dim linhaTexto As String
        linhaTexto = ""

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Open)
        Dim bw As New System.IO.BinaryReader(fs)

        Dim numcells, cell, tempobin As Integer
        numcells = total_mini
        cell = 0
        tempobin = tamanhodoint

        For tempo As Integer = 1 To tempobin
            For cells As Integer = 1 To numcells

                VazaoMini(cells, tempo) = bw.ReadSingle

            Next
        Next
        bw.Close()

        Começoresultado = tb_ini_qob.Text
        tamdoresultado = tamanhodoint

    End Sub

    Sub ESCRVE_PARHIG_HORA_V2() ' escreve o PARHIG
        'Declara as variáveis deste projeto de previsão

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        FileOpen(1, diretóriodoprojeto & "\PARHIG.hig", OpenMode.Input)
        FileOpen(2, diretóriodoprojeto & "\MGB\Input\PARHIG.hig", OpenMode.Output)

        Dim linha As String
        Dim ll As Integer
        ll = 0

        Do Until EOF(1)
            linha = LineInput(1)
            ll = ll + 1
            If ll <= 5 Then
                PrintLine(2, linha)
            ElseIf ll = 6 Then
                'Escreve a data inicial...

                Print(2, "        ")
                If dtpqob1.Value.Day.ToString.Length = 1 Then
                    Print(2, "0")
                End If
                Print(2, dtpqob1.Value.Day.ToString)
                Print(2, "       ")
                If dtpqob1.Value.Month.ToString.Length = 1 Then
                    Print(2, "0")
                End If
                Print(2, dtpqob1.Value.Month.ToString)
                Print(2, "       ")
                Print(2, dtpqob1.Value.Year.ToString)
                Print(2, "        ")
                Print(2, tb_hora_ini.Text)
                PrintLine(2)
            ElseIf ll = 7 Then
                PrintLine(2)
            ElseIf ll = 8 Then
                PrintLine(2, linha)
            ElseIf ll = 9 Then
                'Calcula o tempo da simulação...

                'Calcula datas
                Dim dataO As Date
                Dim dataE As Date
                dataO = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
                dataE = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

                Dim tamanhodoint As Integer
                tamanhodoint = DateDiff(DateInterval.Hour, dataO, dataE) + 1

                If tamanhodoint >= 10000 And tamanhodoint < 100000 Then 'dependendo do tamanho do intervalo de tempo, ajusta os espaços
                    Print(2, "     " & tamanhodoint)
                ElseIf tamanhodoint >= 1000 And tamanhodoint < 10000 Then
                    Print(2, "      " & tamanhodoint)
                ElseIf tamanhodoint >= 100 And tamanhodoint < 1000 Then
                    Print(2, "       " & tamanhodoint)
                ElseIf tamanhodoint >= 10 And tamanhodoint < 100 Then
                    Print(2, "        " & tamanhodoint)
                ElseIf tamanhodoint >= 100000 And tamanhodoint < 1000000 Then
                    Print(2, "    " & tamanhodoint)
                ElseIf tamanhodoint >= 1000000 And tamanhodoint < 10000000 Then
                    Print(2, "   " & tamanhodoint)
                End If

                PrintLine(2, "     3600.") 'uma hora tem 3600 segundos

            ElseIf ll = 10 Then
                PrintLine(2)
            ElseIf ll = 11 Then
                PrintLine(2, linha)
            ElseIf ll >= 12 Then
                PrintLine(2, linha)
            End If

           

        Loop

        FileClose(1)
        FileClose(2)



    End Sub


    Private Sub GroupBox5_Enter(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles GroupBox5.Enter

    End Sub

    Sub SalvaHidrogramas_Binários() 'Essa rotina faz a leitura dos arquivos Qtudo.bin e Qprev, binários

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"
        'Dim linha As String

        'Variável para saber quantas mini bacias temos
        Dim QuantMini As Integer
        QuantMini = total_mini

        Dim data1, data2, data3, data4 As Date
        data1 = tb_ini_pobs.Text
        data2 = tb_fim_pobs.Text
        data3 = tb_ini_pprev.Text
        data4 = tb_fim_pprev.Text

        Dim tamanhodoint As Integer
        tamanhodoint = (DateDiff(DateInterval.Hour, data1, data4) + 1)

        Dim tamanhoOBS As Integer
        tamanhoOBS = (DateDiff(DateInterval.Hour, data1, data2) + 1)

        Dim tamanhoPREV As Integer
        tamanhoPREV = (DateDiff(DateInterval.Hour, data3, data4) + 1)

        'MsgBox(QuantMini & "  " & tamanhodoint)
        'Redimendiona os arquivos de vazão e qprop
        ReDim QPROPHIG(QuantMini, tamanhodoint + 1)
        ReDim VazaoHIG(QuantMini, tamanhodoint + 1)
        'Qprop = vazão observada e assimilada
        'VazãoHIG = vazão prevista e calculada

        'Escreve código de falha nos dois arquivos

        For mbac As Integer = 1 To QuantMini
            For tint As Integer = 1 To tamanhodoint + 1
                QPROPHIG(mbac, tint) = -1
                VazaoHIG(mbac, tint) = -1
            Next
        Next

        'Abre o arquivo com a vazão observada
        Dim nomeArquivo As String
        nomeArquivo = diretóriodoprojeto & "\Output\QTUDO_ATUAL.MGB"

        ' Retorna um texto do arquivo definido
        Dim linhaTexto As String
        linhaTexto = ""

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Open)
        Dim bw As New System.IO.BinaryReader(fs)

        Dim numcells, cell, tempobin As Integer
        numcells = total_mini
        cell = 0
        tempobin = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            For cells As Integer = 1 To numcells
                QPROPHIG(cells, tempo) = bw.ReadSingle
            Next
        Next
        bw.Close()

        'Abre o arquivo com a vazão substituída
        Dim nomeArquivo2 As String
        nomeArquivo2 = diretóriodoprojeto & "\Output\QTUDO_PREV.MGB"

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs2 As FileStream
        fs2 = New FileStream(nomeArquivo2, FileMode.Open)
        Dim bw2 As New System.IO.BinaryReader(fs2)

        numcells = total_mini
        cell = 0
        tempobin = tamanhoPREV

        Dim t As Integer
        t = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            t = t + 1
            For cells As Integer = 1 To numcells
                VazaoHIG(cells, t) = bw2.ReadSingle
            Next
        Next
        bw2.Close()


        Começoresultado = tb_ini_qob.Text
        tamdoresultado = tamanhodoint

    End Sub


    Sub SalvaresultadoQobs()
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"
        'Dim linha As String

        'Declara as variáveis para salvar os dados
        Dim Nqobsf As Integer
        'Nqobsf = 6

        Dim celpostoscomdados As String
        'celpostoscomdados = "41  14  32  31  13  52"

        'Lê as variáveis dos postos com dados
        FileOpen(15, diretóriodoprojeto & "\Input\Parhig.hig", OpenMode.Input)
        Dim linha As String
        Dim ll As Integer
        ll = 0
        Do Until EOF(15)
            linha = LineInput(15)
            ll = ll + 1
            If ll = 23 Then
                Nqobsf = linha.Substring(0, 2)
            ElseIf ll = 26 Then
                celpostoscomdados = linha
            End If
        Loop
        FileClose(15)

        'Variável para saber quantas mini bacias temos
        Dim QuantMini As Integer
        QuantMini = total_mini


        Dim data1, data2 As Date
        data1 = tb_ini_qob.Text
        data2 = tb_fim_qob.Text

        Dim tamanhodoint As Integer
        tamanhodoint = (DateDiff(DateInterval.Hour, data1, data2) + 1)

        'Redimendiona os arquivos de vazão
        ReDim QOBS(QuantMini, tamanhodoint + 1)
        'Escreve código de falha no arquivo

        For mbac As Integer = 1 To QuantMini
            For tint As Integer = 1 To tamanhodoint + 1
                QOBS(mbac, tint) = -1
            Next
        Next

        'Variável que contém quais arquivos de vazão temos
        Dim Temosdados(Nqobsf)

        'Preenche a variável acima
        Dim splt
        For nqo As Integer = 1 To Nqobsf
            splt = Split(celpostoscomdados, "  ")
            Temosdados(nqo) = splt(nqo - 1)
        Next
        
        'Agora temos dois arrays prontos para serem preenchidos e aonde eles devem ser preenchidos

        'VAriáveis para leitura
        Dim linhaqprop As String
        Dim diadavazao As Integer
        Dim startpoint As Integer 'Variável para saber onde começa a ler

        'Fazer o mesmo para o Qobs.PRN agora
        startpoint = 12
        For nqo As Integer = 1 To Nqobsf
            startpoint = startpoint + 12 ' O espaço entre as vazãoes no arquivo de saída é de 15espaços
            FileOpen(12, diretóriodoprojeto & "\Input\QOBS_hora.PRN", OpenMode.Input)
            diadavazao = -1
            Do Until EOF(12)
                linhaqprop = LineInput(12)
                diadavazao = diadavazao + 1
                If diadavazao > 0 Then
                    QOBS(Temosdados(nqo), diadavazao) = AjustarDecimal(linhaqprop.Substring(startpoint + 3, 9))
                End If
            Loop
            FileClose()
        Next

    End Sub


    Sub CARREGA_MINI()
        Dim arquivo_MINI As String 'localização do arquivo de mini bacias
        arquivo_MINI = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\MINI.MGB"

        Dim linha As String 'Arquivo de leitura

            numcels = -1
        FileOpen(1, arquivo_MINI, OpenMode.Input)
            Do Until EOF(1)
                linha = LineInput(1)
                numcels = numcels + 1
                If numcels >= 1 Then
                End If
            Loop
            FileClose()

            Total_MiniBacias = numcels
        ReDim A_MINI(Total_MiniBacias)
        ReDim Sub_MINI(Total_MiniBacias)

        total_subbacias = 0

        'Carrega a área de cada mini-bacia, a sub-bacia de cada mini-bacia e o número de subbacias
            numcels = -1
        FileOpen(1, arquivo_MINI, OpenMode.Input)
        Do Until EOF(1)
            linha = LineInput(1)
            numcels = numcels + 1
            If numcels >= 1 Then
                A_MINI(numcels) = AjustarDecimal(linha.Substring(54, 15))
                Sub_MINI(numcels) = linha.Substring(47, 7)

                If Sub_MINI(numcels) > total_subbacias Then
                    total_subbacias = Sub_MINI(numcels)
                End If

            End If
            

        Loop
        FileClose()

        ReDim A_SUB(total_subbacias)

        'Calcula a área das subbacias
        For subbac As Integer = 1 To total_subbacias
            For minibac As Integer = 1 To Total_MiniBacias
                If Sub_MINI(minibac) = subbac Then
                    A_SUB(subbac) = A_SUB(subbac) + A_MINI(minibac)
                End If
            Next
        Next


        FileClose()

    End Sub

    Sub INTERPOLA_CHUVA_SUB()

        Try
            Dim data1, data2 As Date
            data1 = tb_ini_pobs.Text
            data2 = tb_fim_pobs.Text

            Dim tmaxx1 As Integer
            tmaxx1 = (DateDiff(DateInterval.Hour, data1, data2) + 1)

            Dim data3, data4 As Date
            data3 = tb_ini_pprev.Text
            data4 = tb_fim_pprev.Text

            Dim tmaxx2 As Integer
            tmaxx2 = (DateDiff(DateInterval.Hour, data3, data4) + 1)


            Dim tmaxx3 As Integer
            tmaxx3 = (DateDiff(DateInterval.Hour, data1, data4) + 1)


            ReDim chuvasub1(total_subbacias, tmaxx3)
            ReDim chuvasub2(total_subbacias, tmaxx3)

            For tempo As Integer = 1 To tmaxx3
                For subb As Integer = 1 To total_subbacias
                    chuvasub1(subb, tempo) = -1
                    chuvasub2(subb, tempo) = -1
                Next
            Next


            For tempo As Integer = 1 To tmaxx3
                For subb As Integer = 1 To total_subbacias
                    For cells As Integer = 1 To Total_MiniBacias
                        If subb = Sub_MINI(cells) Then
                            If tempo <= tmaxx1 Then
                                chuvasub1(subb, tempo) = chuvasub1(subb, tempo) + chuvabin1(cells, tempo) * A_MINI(cells)
                            ElseIf tempo > tmaxx1 Then
                                Dim t As Integer
                                t = tempo - tmaxx1
                                chuvasub2(subb, tempo) = chuvasub2(subb, tempo) + chuvabin2(cells, t) * A_MINI(cells)
                            End If
                        End If
                    Next
                Next
            Next

            For tempo As Integer = 1 To tmaxx3
                For subb As Integer = 1 To total_subbacias
                    chuvasub1(subb, tempo) = chuvasub1(subb, tempo) / A_SUB(subb)
                    chuvasub2(subb, tempo) = chuvasub2(subb, tempo) / A_SUB(subb)
                Next
            Next

            'Grava o arquivo saída
            Try
                Dim arquivoCHUVASUB As String
                arquivoCHUVASUB = "c:\mgb-hora\Output\" & "CHUVABACIAS.txt"
                FileOpen(15, arquivoCHUVASUB, OpenMode.Output)
                For tempo As Integer = 1 To tmaxx3
                    For subb As Integer = 1 To total_subbacias
                        If tempo <= tmaxx1 Then
                            Print(15, " " & chuvasub1(subb, tempo))
                        ElseIf tempo > tmaxx1 Then
                            ' Dim t As Integer
                            't = tempo - tmaxx1
                            Print(15, " " & chuvasub2(subb, tempo))
                        End If
                    Next
                    PrintLine(15)
                Next
                FileClose(15)
            Catch ex As Exception

            End Try



        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
        

    End Sub

    Sub COPIA_QSUBST_OUTPUT()

        Try
            File.Copy("c:\mgb-hora\Input\Qsubst_hora.prn", "c:\mgb-hora\Output\Qsubst_hora.prn", True)

        Catch ex As Exception

        End Try


    End Sub


    Sub CARREGAENTRADAS_AUTOMATICO()
        'Carrega o arquivo de vazões observadas
        ofd.FileName = diretorio_previsao & "\QOBS.MGB"
        tb_qob.Text = ofd.FileName
        Me.verificadataqobs_hora()
        Me.ATUALIZA_STATUS()

        'Carrega o arquivo de vazões observadas
        ofd.FileName = diretorio_previsao & "\QSUBS.MGB"
        tb_qsub.Text = ofd.FileName
        Me.verificadataqsubs_hora()
        Me.ATUALIZA_STATUS()

        'Carrega o arquivo de chuvas observadas
        Me.CARREGA_CHUVAOBS_AUTO()

        'Carrega o arquivo de chuvas previstas
        Me.CARREGA_CHUVAPREV_AUTO()

    End Sub

    Sub CARREGA_MULTIPLICADORES()
        'Verifica base de dados interna
        Dim arquivo_est As String 'nome do arquivo com as configurações
        Dim linha As String 'variável string que contém a linha
        'determina o local do arquivo que contém os diretórios
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Multiplicadores.txt"
        'abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Input)
        'faz a leitura do arquiv, linha por linha
        linha = LineInput(1)
        Dim splt
        splt = Split(linha, ";")
        tbFATOR.Text = splt(0)
        tbFator2.Text = splt(1)
        tbFATOR3.Text = splt(2)
        'Fecha
        FileClose()

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
            If total_estações = 7 Then
                diretorio_previsao = linha
            End If
        Loop

        Stlb.Text = "Diretório arquivos de previsão: " & diretorio_previsao
        'Fecha
        FileClose()

    End Sub

    Sub CARREGA_CHUVAOBS_AUTO()
        '  ofd.Multiselect = True
        'ofd.Filter = "Arquivo de chuva observada interpolada do MGB-IPH(.MGB)|*.MGB"
        'If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
        ofd.FileName = diretorio_previsao & "\CHUVA_OBS.MGB"
        tb_pob.Text = ofd.FileName
        'Declara as variáveis
        Dim nome_metadados As String
        nome_metadados = System.IO.Path.GetFileNameWithoutExtension(ofd.FileName)
        Dim diretório As String
        diretório = System.IO.Path.GetDirectoryName(ofd.FileName)

        Dim arquivo_metadados As String
        arquivo_metadados = diretório & "\" & nome_metadados & ".meta"

        FileOpen(1, arquivo_metadados, OpenMode.Input)

        Dim ll As Integer
        ll = 0
        Dim linha As String
        total_mini = 0

        Do Until EOF(1)
            linha = LineInput(1)
            ll = ll + 1
            If ll = 10 Then
                datainiciopob = linha.Substring(0, 4) & "/" & linha.Substring(8, 2) & "/" & linha.Substring(14, 2) & " " & linha.Substring(20, 2) & ":00:00"
                Dim addhour As Integer
                addhour = linha.Substring(22, 8)
                addhour = addhour - 1
                dataFimpob = datainiciopob.AddHours(addhour)
            ElseIf ll = 13 Then
                total_mini = linha
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

        tb_ini_pobs.Text = dia & "/" & mes & "/" & datainiciopob.Year & " " & hora & ":00"

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

        tb_fim_pobs.Text = dia & "/" & mes & "/" & dataFimpob.Year & " " & hora & ":00"
        FileClose()


        Me.ATUALIZA_STATUS()


    End Sub

    Sub CARREGA_CHUVAPREV_AUTO()

        'ofd.Filter = "Arquivo de chuva prevista interpolada do MGB-IPH(.mgb;.bin)|*.mgb;*.bin"
        'If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then

        ofd.FileName = diretorio_previsao & "\PrevisaoMINI_Estreito.bin"
        tb_pprev.Text = ofd.FileName
        'Declara as variáveis
        Dim nome_metadados As String
        nome_metadados = System.IO.Path.GetFileNameWithoutExtension(ofd.FileName)
        Dim diretório As String
        diretório = System.IO.Path.GetDirectoryName(ofd.FileName)

        Dim arquivo_metadados As String
        arquivo_metadados = diretório & "\" & nome_metadados & ".meta"

        'arquivo padrão com nome dado pelo Siqueira:
        arquivo_metadados = diretório & "\Previsao.meta"
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input"
        File.Copy(arquivo_metadados, diretóriodoprojeto & "\Previsao.meta", True)
        '-- Copiou o arquivo com o nome dado pelo Siqueira

        FileOpen(1, arquivo_metadados, OpenMode.Input)

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
                dataFimpprev = datainiciopprev.AddHours(addhour)
            End If
        Loop

        Dim dia, mes, hora As String
        dia = datainiciopprev.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datainiciopprev.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datainiciopprev.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        tb_ini_pprev.Text = dia & "/" & mes & "/" & datainiciopprev.Year & " " & hora & ":00"

        dia = dataFimpprev.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If
        mes = dataFimpprev.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If
        hora = dataFimpprev.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        tb_fim_pprev.Text = dia & "/" & mes & "/" & dataFimpprev.Year & " " & hora & ":00"

        FileClose()

        Me.ATUALIZA_STATUS()

        'End If

    End Sub

    Sub COPIA_DADOS_ESTAÇÕES_RESERV()
        'Esta rotina copia os arquivos texto, gerados pela interface, com os dados dos reservatórios, para a pasta input do MGB
        Try
            'Verifica quantas estações existem na base de dados interna
            Dim arquivo_est As String 'nome do arquivo que contém as estações
            Dim linha As String 'linha de leitura
            Dim total_est_res As Integer
            total_est_res = 0 'número de estações

            'define o arquivo com a lista de estações
            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesUHEs.txt"

            'Abre e faz a leitura do arquivo com a lista de estações
            FileOpen(1, arquivo_est, OpenMode.Input)
            'começa a leitura, linha por linha, até o final
            Do Until EOF(1)
                linha = LineInput(1)
                'contador
                total_est_res = total_est_res + 1

                'para pular o cabeçalho, a rotina só funciona quando o contador é maior que 1
                If total_est_res > 1 Then
                    'crie um delimitador, para abrir o nome da estação
                    Dim splt
                    'o delimitador é ";"
                    splt = Split(linha, ";")
                    'carrega o nome da estação em uma variável chamada estação
                    Dim estação As String
                    estação = splt(1)

                    'carrega o local onde a estação está atualmente
                    Dim arquivo_est2 As String
                    arquivo_est2 = diretorio_previsao & "\" & estação & ".txt"

                    File.Copy(arquivo_est2, "c:\mgb-hora\Input" & "\" & estação & ".txt", True)

                End If
            Loop
            FileClose()
            total_est_res = total_est_res - 1 'desconta a primeira linha, que é cabeçalho
            '-- Fim da cópia das estações
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
        
    End Sub

    Sub ESCREVE_uhe_tocantins() ' escreve o PARHIG
        'Declara as variáveis deste projeto de previsão

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        FileOpen(2, diretóriodoprojeto & "\MGB\Input\uhe_tocantins.mgb", OpenMode.Output)

        If cb_1.Checked = True Then
            PrintLine(2, "1    UHE_ESTREITO")
        ElseIf cb_1.Checked = False Then
            PrintLine(2, "0    UHE_ESTREITO")
        End If
        If cb_2.Checked = True Then
            PrintLine(2, "1    UHE_LAJEADO")
        ElseIf cb_2.Checked = False Then
            PrintLine(2, "0    UHE_LAJEADO")
        End If
        If cb_3.Checked = True Then
            PrintLine(2, "1    UHE_PEIXEANGICAL")
        ElseIf cb_3.Checked = False Then
            PrintLine(2, "0    UHE_PEIXEANGICAL")
        End If
        If cb_4.Checked = True Then
            PrintLine(2, "1    UHE_CANABRAVA")
        ElseIf cb_4.Checked = False Then
            PrintLine(2, "0    UHE_CANABRAVA")
        End If
        If cb_5.Checked = True Then
            PrintLine(2, "1    UHE_SERRADAMESA")
        ElseIf cb_5.Checked = False Then
            PrintLine(2, "0    UHE_SERRADAMESA")
        End If

                   FileClose(2)



    End Sub



    Private Sub cb_2_CheckedChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_2.CheckedChanged
        If cb_2.Checked = True Then
            cb_2.ForeColor = Drawing.Color.Green
        ElseIf cb_2.Checked = False Then
            cb_2.ForeColor = Drawing.Color.Red
        End If
    End Sub

    Private Sub cb_1_CheckedChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_1.CheckedChanged
        If cb_1.Checked = True Then
            cb_1.ForeColor = Drawing.Color.Green
        ElseIf cb_1.Checked = False Then
            cb_1.ForeColor = Drawing.Color.Red
        End If
    End Sub

    Private Sub cb_3_CheckedChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_3.CheckedChanged
        If cb_3.Checked = True Then
            cb_3.ForeColor = Drawing.Color.Green
        ElseIf cb_3.Checked = False Then
            cb_3.ForeColor = Drawing.Color.Red
        End If
    End Sub

    Private Sub cb_4_CheckedChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_4.CheckedChanged
        If cb_4.Checked = True Then
            cb_4.ForeColor = Drawing.Color.Green
        ElseIf cb_4.Checked = False Then
            cb_4.ForeColor = Drawing.Color.Red
        End If
    End Sub

    Private Sub cb_5_CheckedChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_5.CheckedChanged
        If cb_5.Checked = True Then
            cb_5.ForeColor = Drawing.Color.Green
        ElseIf cb_5.Checked = False Then
            cb_5.ForeColor = Drawing.Color.Red
        End If
    End Sub

    Sub REMOVE_VIES()

        Dim data1, data2, data3, data4 As Date
        data1 = tb_ini_pobs.Text
        data2 = tb_fim_pobs.Text
        data3 = tb_ini_pprev.Text
        data4 = tb_fim_pprev.Text

        Dim tamanhodoint As Integer
        tamanhodoint = (DateDiff(DateInterval.Hour, data1, data4) + 1)

        Dim tamanhoOBS As Integer
        tamanhoOBS = (DateDiff(DateInterval.Hour, data1, data2) + 1)

        Dim tamanhoPREV As Integer
        tamanhoPREV = (DateDiff(DateInterval.Hour, data3, data4) + 1)

        Dim tempobin As Integer


        tempobin = tamanhoPREV

        Dim t As Integer
        Dim coef As Double


        'Correção em Garibaldi
        t = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            t = t + 1


            If VazaoHIG(750, t) < 500 Then
                VazaoHIG(750, t) = coef * VazaoHIG(750, t)
            End If

        Next

        'Correção em Campos Novos
        t = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            t = t + 1

            coef = -0.0021 * VazaoHIG(754, t) + 2.01
            If VazaoHIG(754, t) < 500 Then
                VazaoHIG(754, t) = coef * VazaoHIG(750, t)
            End If

        Next


  



    End Sub

    Sub LeLimiares()

        Dim local_input As String 'shapefile
        local_input = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Mapa\Mini_TR.txt"

        Dim nMinibacias As Integer
        nMinibacias = Total_MiniBacias

        ReDim Mini_Usa(nMinibacias)
        ReDim Mini_TRs(nMinibacias, 5)

        Using Text As New IO.StreamReader(local_input)
            'Descarta o cabeçalho
            Text.ReadLine()
            'Usa/ñ usa;TR 2;TR 5;TR 10;TR 25;TR 50            

            Dim strLine() As String
            For j = 1 To nMinibacias
                strLine = Split(Text.ReadLine, ";")
                Mini_Usa(j) = CInt(strLine(0))
                Mini_TRs(j, 1) = CSng(strLine(1))
                Mini_TRs(j, 2) = CSng(strLine(2))
                Mini_TRs(j, 3) = CSng(strLine(3))
                Mini_TRs(j, 4) = CSng(strLine(4))
                Mini_TRs(j, 5) = CSng(strLine(5))
            Next
            'Termina a leitura do input
        End Using



    End Sub

    Sub RODAR_PREVISAO_AUTOMATICO()
        Me.COMPLETA_CHUVABIN() 'cola os dois CHUVABIN, copia na pasta input do plugin
        Me.COPIA_VAZOES() 'copia o QSUBS e o QOBS do MGB para a pasta de input do plugin
        Me.ESCRVE_PARHIG_HORA_V2() 'escreve o parhig e coloca na pasta input do plugin
        Me.ESCREVE_uhe_tocantins() 'escreve o arquivo Uhe_tocantins.MGB e coloca na pasta input do plugin
        Me.COPIA_DADOS_ESTAÇÕES_RESERV() 'copia os arquivos dos reservatórios para a pasta c:/mgb-hora/input

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        ' Escreve tudo na pasta c:\mgb
        Dim f() As String = Directory.GetFiles(diretóriodoprojeto & "\Input")
        For i As Integer = 0 To UBound(f)
            ' File.Copy(f(i), Environment.GetFolderPath(Environment.SpecialFolder.Desktop) & "\Input" & "\" & System.IO.Path.GetFileName(f(i)))
            File.Copy(f(i), "c:\mgb-hora\Input" & "\" & System.IO.Path.GetFileName(f(i)), True)

        Next

        Me.RODAFORTRAN_AUTOMATICO() 'Chama o MGB Fortran

        Try 'copia tudo da pasta c:\mgb

            'Antes de copiar, deleta a pasta output original
            Me.RESET_OUPUT()

            'Antes de copiar, deleta a pasta output original
            Me.RESET_OUPUT()

            Dim f2() As String = Directory.GetFiles("c:\mgb-hora\Output")
            For i As Integer = 0 To UBound(f2)
                File.Copy(f2(i), diretóriodoprojeto & "\Output" & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
            ' If
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta c:/mgb-hora/output para a pasta do projeto")
        End Try

        ' Try
        'Salva os arquivos de saída na memória
        'Me.SalvaresultadoNovo_hora()

        If CheckBox1.Checked = True Then
            Me.SalvaresultadoQobs()

            Me.SalvaHidrogramas_Binários()

            ' Me.REMOVE_VIES()

            Me.COPIA_QSUBST_OUTPUT()

            Me.CARREGA_MINI()
            Me.INTERPOLA_CHUVA_SUB()

            Me.LeLimiares() 'Faz a leitura dos limiares - FMF em 01 de Set de 2018
        End If


        'Catch ex As Exception
        '    MsgBox("Problema ao salvar os resultados na memória", MsgBoxStyle.Critical, "Erro")
        'End Try

    End Sub

    Sub RODAFORTRAN_AUTOMATICO()
        ' Create new process to run setup
        Dim processRun As New Process
        ' Set process props
        processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\" & "MGB_hora_auto.exe"
        processRun.StartInfo.UseShellExecute = False
        ' Start the process
        processRun.Start()
        ' Wait for the process to finish
        processRun.WaitForExit()

    End Sub

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        Me.RODAR_PREVISAO_INERCIAL()
        MsgBox("Previsão realizada com sucesso!", MsgBoxStyle.Exclamation, "MGB")
        Me.Close()
    End Sub

    Sub RODAR_PREVISAO_INERCIAL()
        Me.COMPLETA_CHUVABIN() 'cola os dois CHUVABIN, copia na pasta input do plugin
        Me.COPIA_VAZOES() 'copia o QSUBS e o QOBS do MGB para a pasta de input do plugin
        Me.ESCRVE_PARHIG_HORA_V2() 'escreve o parhig e coloca na pasta input do plugin
        Me.ESCREVE_uhe_tocantins() 'escreve o arquivo Uhe_tocantins.MGB e coloca na pasta input do plugin
        Me.COPIA_DADOS_ESTAÇÕES_RESERV() 'copia os arquivos dos reservatórios para a pasta c:/mgb-hora/input

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        ' Escreve tudo na pasta c:\mgb
        Dim f() As String = Directory.GetFiles(diretóriodoprojeto & "\Input")
        For i As Integer = 0 To UBound(f)
            ' File.Copy(f(i), Environment.GetFolderPath(Environment.SpecialFolder.Desktop) & "\Input" & "\" & System.IO.Path.GetFileName(f(i)))
            File.Copy(f(i), "c:\mgb-hora\Input" & "\" & System.IO.Path.GetFileName(f(i)), True)

        Next

        Me.RODAFORTRAN_INERCIAL() 'Chama o MGB Fortran

        Try 'copia tudo da pasta c:\mgb

            'Antes de copiar, deleta a pasta output original
            Me.RESET_OUPUT()

            Dim f2() As String = Directory.GetFiles("c:\mgb-hora\Output")
            For i As Integer = 0 To UBound(f2)
                File.Copy(f2(i), diretóriodoprojeto & "\Output" & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
            ' If
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta c:/mgb-hora/output para a pasta do projeto")
        End Try

        ' Try
        'Salva os arquivos de saída na memória
        'Me.SalvaresultadoNovo_hora()

        If CheckBox1.Checked = True Then
            Me.SalvaresultadoQobs()

            Me.SalvaHidrogramas_Binários_Inercial()
            Me.SalvaCotagramas_Binários_Inercial()

            ' Me.REMOVE_VIES()

            Me.COPIA_QSUBST_OUTPUT()

            Me.CARREGA_MINI()
            Me.INTERPOLA_CHUVA_SUB()

            Me.LeLimiares() 'Faz a leitura dos limiares - FMF em 01 de Set de 2018
        End If


        'Catch ex As Exception
        '    MsgBox("Problema ao salvar os resultados na memória", MsgBoxStyle.Critical, "Erro")
        'End Try

    End Sub

    Sub RODAFORTRAN_INERCIAL()
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "MGB_Inercial_PrevRS_FORTRAN.exe"

        ' Create new process to run setup
        Dim processRun As New Process
        ' Set process props
        processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\" & ExecutavelFORTRAN
        ' Start the process
        processRun.Start()
        ' Wait for the process to finish
        processRun.WaitForExit()

    End Sub

    Sub RODAR_PREVISAO_AUTOMATICO_INERCIAL()
        Me.COMPLETA_CHUVABIN() 'cola os dois CHUVABIN, copia na pasta input do plugin
        Me.COPIA_VAZOES() 'copia o QSUBS e o QOBS do MGB para a pasta de input do plugin
        Me.ESCRVE_PARHIG_HORA_V2() 'escreve o parhig e coloca na pasta input do plugin
        Me.ESCREVE_uhe_tocantins() 'escreve o arquivo Uhe_tocantins.MGB e coloca na pasta input do plugin
        Me.COPIA_DADOS_ESTAÇÕES_RESERV() 'copia os arquivos dos reservatórios para a pasta c:/mgb-hora/input

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        ' Escreve tudo na pasta c:\mgb
        Dim f() As String = Directory.GetFiles(diretóriodoprojeto & "\Input")
        For i As Integer = 0 To UBound(f)
            ' File.Copy(f(i), Environment.GetFolderPath(Environment.SpecialFolder.Desktop) & "\Input" & "\" & System.IO.Path.GetFileName(f(i)))
            File.Copy(f(i), "c:\mgb-hora\Input" & "\" & System.IO.Path.GetFileName(f(i)), True)

        Next

        Me.RODAFORTRAN_AUTOMATICO_INERCIAL() 'Chama o MGB Fortran

        Try 'copia tudo da pasta c:\mgb

            'Antes de copiar, deleta a pasta output original
            Me.RESET_OUPUT()

            'Antes de copiar, deleta a pasta output original
            Me.RESET_OUPUT()

            Dim f2() As String = Directory.GetFiles("c:\mgb-hora\Output")
            For i As Integer = 0 To UBound(f2)
                File.Copy(f2(i), diretóriodoprojeto & "\Output" & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
            ' If
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta c:/mgb-hora/output para a pasta do projeto")
        End Try

        ' Try
        'Salva os arquivos de saída na memória
        'Me.SalvaresultadoNovo_hora()

        If CheckBox1.Checked = True Then
            Me.SalvaresultadoQobs()

            Me.SalvaHidrogramas_Binários_Inercial()
            Me.SalvaCotagramas_Binários_Inercial()


            ' Me.REMOVE_VIES()

            Me.COPIA_QSUBST_OUTPUT()

            Me.CARREGA_MINI()
            Me.INTERPOLA_CHUVA_SUB()

            Me.LeLimiares() 'Faz a leitura dos limiares - FMF em 01 de Set de 2018
        End If


        'Catch ex As Exception
        '    MsgBox("Problema ao salvar os resultados na memória", MsgBoxStyle.Critical, "Erro")
        'End Try

    End Sub

    Sub RODAFORTRAN_AUTOMATICO_INERCIAL()
        ' Create new process to run setup
        Dim processRun As New Process
        ' Set process props
        processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\" & "MGB_Inercial_PrevRS_FORTRAN.exe"
        processRun.StartInfo.UseShellExecute = False
        ' Start the process
        processRun.Start()
        ' Wait for the process to finish
        processRun.WaitForExit()

    End Sub

    Sub CARREGA_MODELO_AUTOMATICO()
        'Verifica base de dados interna
        Dim arquivo_est As String 'nome do arquivo com as configurações
        Dim linha As String 'variável string que contém a linha
        'determina o local do arquivo que contém os diretórios
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Modelos.txt"
        'abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Input)
        'faz a leitura do arquiv, linha por linha
        linha = LineInput(1)
        RodaMC = linha

        linha = LineInput(1)
        RodaIN = linha

        'Fecha
        FileClose()

    End Sub

    Sub SalvaHidrogramas_Binários_Inercial() 'Essa rotina faz a leitura dos arquivos Qtudo.bin e Qprev, binários

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"
        'Dim linha As String

        'Variável para saber quantas mini bacias temos
        Dim QuantMini As Integer
        QuantMini = total_mini

        Dim data1, data2, data3, data4 As Date
        data1 = tb_ini_pobs.Text
        data2 = tb_fim_pobs.Text
        data3 = tb_ini_pprev.Text
        data4 = tb_fim_pprev.Text

        Dim tamanhodoint As Integer
        tamanhodoint = (DateDiff(DateInterval.Hour, data1, data4) + 1)

        Dim tamanhoOBS As Integer
        tamanhoOBS = (DateDiff(DateInterval.Hour, data1, data2) + 1)

        Dim tamanhoPREV As Integer
        tamanhoPREV = (DateDiff(DateInterval.Hour, data3, data4) + 1)

        'MsgBox(QuantMini & "  " & tamanhodoint)
        'Redimendiona os arquivos de vazão e qprop
        ReDim QPROPHIG(QuantMini, tamanhodoint + 1)
        ReDim VazaoHIG(QuantMini, tamanhodoint + 1)
        'Qprop = vazão observada e assimilada
        'VazãoHIG = vazão prevista e calculada

        'Escreve código de falha nos dois arquivos

        For mbac As Integer = 1 To QuantMini
            For tint As Integer = 1 To tamanhodoint + 1
                QPROPHIG(mbac, tint) = -1
                VazaoHIG(mbac, tint) = -1
            Next
        Next

        'Abre o arquivo com a vazão observada
        Dim nomeArquivo As String
        nomeArquivo = diretóriodoprojeto & "\Output\QTUDO_Inercial_Atual.MGB"

        ' Retorna um texto do arquivo definido
        Dim linhaTexto As String
        linhaTexto = ""

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Open)
        Dim bw As New System.IO.BinaryReader(fs)

        Dim numcells, cell, tempobin As Integer
        numcells = total_mini
        cell = 0
        tempobin = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            For cells As Integer = 1 To numcells
                QPROPHIG(cells, tempo) = bw.ReadSingle
            Next
        Next
        bw.Close()

        'Abre o arquivo com a vazão substituída
        Dim nomeArquivo2 As String
        nomeArquivo2 = diretóriodoprojeto & "\Output\QTUDO_Inercial_Prev.MGB"

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs2 As FileStream
        fs2 = New FileStream(nomeArquivo2, FileMode.Open)
        Dim bw2 As New System.IO.BinaryReader(fs2)

        numcells = total_mini
        cell = 0
        tempobin = tamanhoPREV

        Dim t As Integer
        t = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            t = t + 1
            For cells As Integer = 1 To numcells
                VazaoHIG(cells, t) = bw2.ReadSingle
            Next
        Next
        bw2.Close()


        Começoresultado = tb_ini_qob.Text
        tamdoresultado = tamanhodoint

    End Sub

    Sub SalvaCotagramas_Binários_Inercial() 'Essa rotina faz a leitura dos arquivos Qtudo.bin e Qprev, binários

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"
        'Dim linha As String

        'Variável para saber quantas mini bacias temos
        Dim QuantMini As Integer
        QuantMini = total_mini

        Dim data1, data2, data3, data4 As Date
        data1 = tb_ini_pobs.Text
        data2 = tb_fim_pobs.Text
        data3 = tb_ini_pprev.Text
        data4 = tb_fim_pprev.Text

        Dim tamanhodoint As Integer
        tamanhodoint = (DateDiff(DateInterval.Hour, data1, data4) + 1)

        Dim tamanhoOBS As Integer
        tamanhoOBS = (DateDiff(DateInterval.Hour, data1, data2) + 1)

        Dim tamanhoPREV As Integer
        tamanhoPREV = (DateDiff(DateInterval.Hour, data3, data4) + 1)

        'MsgBox(QuantMini & "  " & tamanhodoint)
        'Redimendiona os arquivos de vazão e qprop
        ReDim CotaPROPHIG(QuantMini, tamanhodoint + 1)
        ReDim CotaHIG(QuantMini, tamanhodoint + 1)
        'Qprop = vazão observada e assimilada
        'VazãoHIG = vazão prevista e calculada

        'Escreve código de falha nos dois arquivos

        For mbac As Integer = 1 To QuantMini
            For tint As Integer = 1 To tamanhodoint + 1
                CotaPROPHIG(mbac, tint) = -1
                CotaHIG(mbac, tint) = -1
            Next
        Next

        'Abre o arquivo com a vazão observada
        Dim nomeArquivo As String
        nomeArquivo = diretóriodoprojeto & "\Output\YTUDO.MGB"

        ' Retorna um texto do arquivo definido
        Dim linhaTexto As String
        linhaTexto = ""

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs As FileStream
        fs = New FileStream(nomeArquivo, FileMode.Open)
        Dim bw As New System.IO.BinaryReader(fs)

        Dim numcells, cell, tempobin As Integer
        numcells = total_mini
        cell = 0
        tempobin = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            For cells As Integer = 1 To numcells
                CotaPROPHIG(cells, tempo) = bw.ReadSingle
            Next
        Next
        bw.Close()

        'Abre o arquivo com a vazão substituída
        Dim nomeArquivo2 As String
        nomeArquivo2 = diretóriodoprojeto & "\Output\YTUDO_Prev.MGB"

        ' abre o arquivo e iniciao o objeto StreamReader
        Dim fs2 As FileStream
        fs2 = New FileStream(nomeArquivo2, FileMode.Open)
        Dim bw2 As New System.IO.BinaryReader(fs2)

        numcells = total_mini
        cell = 0
        tempobin = tamanhoPREV

        Dim t As Integer
        t = tamanhoOBS

        For tempo As Integer = 1 To tempobin
            t = t + 1
            For cells As Integer = 1 To numcells
                CotaHIG(cells, t) = bw2.ReadSingle
            Next
        Next
        bw2.Close()


        Começoresultado = tb_ini_qob.Text
        tamdoresultado = tamanhodoint

    End Sub

End Class