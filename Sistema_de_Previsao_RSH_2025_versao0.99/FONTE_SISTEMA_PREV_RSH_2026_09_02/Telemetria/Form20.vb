'Importação
Imports System.Data.OleDb
Imports Microsoft.VisualBasic

Public Class Form20

    'Variáveis do uso do word
    Dim objWord As Object
    Dim Incremental_CN As Double
    Dim Incremental(20) As Integer
    Dim DirPrevAuto(3), Dir_EXPORTA As String


    Private Sub Timer1_Tick(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Timer1.Tick
        'Essa subrotina é o tick do timer, que está setado a cada 1 (1000ms).
        'A cada tick ele executa essa rotina

        Label10.Text = DateTime.Now.ToLongTimeString 'A cada 1s (1000ms) o label mostra a hora atual

        If TotalCommander = True Then 'Se estiver liberado para executar


            'Limpar memória
            If Label10.Text.Substring(3, 2) = cbLimpezaMinu.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbLimpezaHora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                'Redimendiona os arquivos de vazão
                ReDim QOBS(0, 0)
                'Redimendiona os arquivos de vazão e qprop
                ReDim QPROPHIG(0, 0)
                ReDim VazaoHIG(0, 0)
                'Redimensiona variável de leitura de chuva
                ReDim chuvasub1(0, 0)
                ReDim chuvasub2(0, 0)
            End If

            'Chuva Observada
            If Label10.Text.Substring(3, 2) = cbPOBSmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbPOBShora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Dim frmPLU As New Form7
                frmPLU.Show()
            End If

            'Chuva Prevista

            If Label10.Text.Substring(3, 2) = cbPPREVmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbPPREVhora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos

                Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()
                Me.CONFIGURA_MODELO_METEO()

                Dim frmPrevisaoTXT As New Form21
                frmPrevisaoTXT.Show()

                'If rbETA05.Checked = True Then
                '    'Dim frmPrevisaoETA As New ETASismaAUTO.FormChuvaPrevisaoETA
                '    'frmPrevisaoETA.NomeProjeto = "Estreito" 'Por exemplo
                '    'frmPrevisaoETA.Show()

                '    Dim frmprevisaoEtaAUTO As New ETASismaden_BRAMSeta05AUTO.FormChuvaPrevisaoETAauto
                '    frmprevisaoEtaAUTO.NomeProjeto = "Estreito"
                '    frmprevisaoEtaAUTO.Show()
                'End If

                'If rbGEFS.Checked = True Then
                '    Dim frmPrevisaoGEFS_ONS As New ChuvaONSauto.Form_Entrada_ChuvaSIMEPAR
                '    frmPrevisaoGEFS_ONS.Show()
                'End If

                'If rbETA40.Checked = True Then
                '    Dim frmPrevisaoETA40_ONS As New ChuvaEtaONSauto.Form_Entrada_ChuvaEtaONS
                '    frmPrevisaoETA40_ONS.Show()
                'End If

            End If

            'Vazão Observada
            If Label10.Text.Substring(3, 2) = cbQOBSmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbQOBShora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Dim frmQOBS As New Form3
                frmQOBS.Show()
            End If

            'Vazão Prevista
            If Label10.Text.Substring(3, 2) = cbUHEmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbUHEhora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Dim frmUHEs As New Form5
                frmUHEs.Show()
            End If

            'Rodar Previsão MC
            If Label10.Text.Substring(3, 2) = cbRODARmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbRODARhora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Me.ESCREVE_MODELO_AUTOMATICO_MC(1, 0)
                Dim frmRODAR As New Form8
                frmRODAR.Show()
            End If

            'Grava Resultados
            If Label10.Text.Substring(3, 2) = cbGRAVAmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbGRAVAhora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos

                OpçãoGrafico = 1
                Dim frmFIG As New Form12
                frmFIG.Show()

                OpçãoGrafico = 4
                Dim frmFIG2 As New Form12
                frmFIG2.Show()

                'Dim frmPAN As New Form19
                'frmPAN.Show()

                Me.EXPORTA_RESULTADOS("MC")

                ''Dim frmFIG2 As New Form12b
                ''frmFIG2.Show()

                ''Me.SalvaHidrogramaCNO()

                ''Me.SalvaArquivoONS_CNO_v2()
                ''Me.SalvaArquivoONS_CNO2_v3()

                ' ''Me.CalculaArquivoWord_FDC15d()
                ''Me.CalculaBoletim_FDC15d()

            End If

            'Manda o e-mail
            If Label10.Text.Substring(3, 2) = cbEMAILmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbEMAILhora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Dim frmEMAIL As New Form16
                frmEMAIL.Show()
            End If


            'Rodar Previsão IN
            If Label10.Text.Substring(3, 2) = cbRodarIN_min.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbRodarIN_hora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Me.ESCREVE_MODELO_AUTOMATICO_MC(0, 1)
                Dim frmRODAR As New Form8
                frmRODAR.Show()
            End If

            'Grava Resultados
            If Label10.Text.Substring(3, 2) = cbGRAVA_inercial_min.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbGRAVA_inercial_hora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos

                OpçãoGrafico = 1
                Dim frmFIG As New Form12
                frmFIG.Show()

                OpçãoGrafico = 4
                Dim frmFIG2 As New Form12
                frmFIG2.Show()

                Me.EXPORTA_RESULTADOS("Inercial")

            End If


            '' '' ''Manda o e-mail
            ' '' ''If Label10.Text.Substring(3, 2) = cbEMAILminCNO.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbEMAILhoraCNO.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
            ' '' ''    Dim frmEMAIL As New Form17
            ' '' ''    frmEMAIL.Show()
            ' '' ''End If

            '' ''Verifica se foi tudo certo
            ' ''If Label10.Text.Substring(3, 2) = cbVERIFICARmin.Text And Label10.Text.Substring(6, 2) = "00" And Label10.Text.Substring(0, 2) = cbVERIFICARhora.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
            ' ''    If checkVerif.Checked = False Then
            ' ''        ERRO_PREVISÃO = 0
            ' ''    End If

            ' ''    If ERRO_PREVISÃO = 3 Then 'se tiver erro de enviar email
            ' ''        ERRO_PREVISÃO = 0
            ' ''        Dim novaHora As Integer
            ' ''        novaHora = cbEMAILmin.Text + 2
            ' ''        Dim novaHoraH As String
            ' ''        novaHoraH = novaHora
            ' ''        If novaHoraH.Length < 2 Then novaHoraH = "0" & novaHoraH
            ' ''        cbEMAILmin.Text = novaHoraH

            ' ''        If cbEMAILmin.Text > 57 Then
            ' ''            cbEMAILmin.Text = "00"
            ' ''            novaHora = cbEMAILhora.Text + 1
            ' ''            novaHoraH = novaHora
            ' ''            If novaHoraH.Length < 2 Then novaHoraH = "0" & novaHoraH
            ' ''            cbEMAILhora.Text = novaHoraH
            ' ''        End If


            ' ''        novaHora = cbVERIFICARmin.Text + 2
            ' ''        novaHoraH = novaHora
            ' ''        If novaHoraH.Length < 2 Then novaHoraH = "0" & novaHoraH
            ' ''        cbVERIFICARmin.Text = novaHoraH

            ' ''        If cbVERIFICARmin.Text > 57 Then
            ' ''            cbVERIFICARmin.Text = "00"
            ' ''            novaHora = cbVERIFICARhora.Text + 1
            ' ''            novaHoraH = novaHora
            ' ''            If novaHoraH.Length < 2 Then novaHoraH = "0" & novaHoraH
            ' ''            cbVERIFICARhora.Text = novaHoraH
            ' ''        End If

            ' ''    End If
            ' ''End If

        End If

    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Timer1.Enabled = True 'Dispara o timer para funcionar
        TotalCommander = True 'libera o download para o CPTEC
        Label11.ForeColor = Drawing.Color.Green 'Indica que o CPTEC está funcionando
        Label11.Text = "Ligado"

    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        TotalCommander = False
        Label11.ForeColor = Drawing.Color.Red 'Indica que o CPTEC está funcionando
        Label11.Text = "Desligado"
    End Sub

    Private Sub Form15_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        Me.CARREGA_DIRETÓRIOS()
        Me.CARREGA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()
        Me.CARREGA_MODELOMETEO_CHUVA()
        Me.AGENDA()
        Me.INCREMENTAL_CN_LOAD()
        Me.CARREGA_INFO_CHUVA_PREVISTA()
    End Sub

    Sub AGENDA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Agenda.txt", OpenMode.Input)

        Dim linha_agenda As String
        Dim splt

        Do Until EOF(1)
            linha_agenda = LineInput(1)
            splt = Split(linha_agenda, ":")
            If splt(0) = "Chuva_Observada" Then
                cbPOBShora.Text = splt(1)
                cbPOBSmin.Text = splt(2)
            ElseIf splt(0) = "Chuva_Prevista" Then
                cbPPREVhora.Text = splt(1)
                cbPPREVmin.Text = splt(2)
            ElseIf splt(0) = "Vazão_Observada" Then
                cbQOBShora.Text = splt(1)
                cbQOBSmin.Text = splt(2)
            ElseIf splt(0) = "Vazão_UHES" Then
                cbUHEhora.Text = splt(1)
                cbUHEmin.Text = splt(2)
            ElseIf splt(0) = "Rodar_Previsão" Then
                cbRodarIN_hora.Text = splt(1)
                cbRodarIN_min.Text = splt(2)
            ElseIf splt(0) = "Gravar_Resultados" Then
                cbGRAVA_inercial_hora.Text = splt(1)
                cbGRAVA_inercial_min.Text = splt(2)
                'ElseIf splt(0) = "Email" Then
                '    cbEMAILhora.Text = splt(1)
                '    cbEMAILmin.Text = splt(2)
                'ElseIf splt(0) = "Enviar_ENERCAN" Then
                '    cbEMAILhoraCNO.Text = splt(1)
                '    cbEMAILminCNO.Text = splt(2)
            End If
        Loop
        FileClose()
    End Sub

    Sub INCREMENTAL_CN_LOAD()

        Incremental_CN = 1

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Incremental_CN.txt", OpenMode.Input)
        Dim linha_agenda As String
        linha_agenda = LineInput(1)
        Incremental_CN = linha_agenda
        FileClose()
    End Sub

    Sub CARREGA_DIRETÓRIOS() 'Abre o arquivo com as informações dos diretórios e carrega eles

        'Declara as variáveis
        Dim arquivo_est As String 'Nome do arquivo de entrada
        Dim total_estações As Integer 'Número de postos que devem ser baixados
        Dim linha As String 'variável de leitura
        total_estações = 0 'seta o total de estações como zero
        'Define o arquivo de entrada que contém a localização dos postos a serem baixados
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\config.txt"
        'Abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Input)
        'Começa a leitura do arquivo até o final
        Do Until EOF(1)
            linha = LineInput(1)
            total_estações = total_estações + 1
            'Cada linha do arquivo de entrada contém um diretório
            'Lê as linhas em ordem, definindo o diretório dos arquivos
            If total_estações = 1 Then
                DIR_RHAMA_Q = linha
            ElseIf total_estações = 2 Then
                DIR_RHAMA_P = linha
            ElseIf total_estações = 3 Then
                DIR_CPTEC = linha
            ElseIf total_estações = 4 Then
                DIR_LAJEADO = linha
            ElseIf total_estações = 5 Then
                DIR_ESTREITO = linha
                'ElseIf total_estações = 6 Then
                '    DIR_SERRADAMESA = linha
                'A linha 7 é pulada, pois contém a informação de onde devem ser salvos os arquivos
                'ElseIf total_estações = 8 Then
                '    DIR_CANABRAVA = linha
            ElseIf total_estações = 11 Then
                DirPrevAuto(1) = linha
            ElseIf total_estações = 12 Then
                DirPrevAuto(2) = linha
            ElseIf total_estações = 13 Then
                DirPrevAuto(3) = linha
            ElseIf total_estações = 14 Then
                Dir_EXPORTA = linha
            End If
        Loop
        FileClose()

    End Sub

    Sub SalvaHidrograma()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        arquivo = diretóriodoprojeto & "\Previsão_" & Now.Year & mes & dia & hora & ".txt"

        postoselecionado = 739


        FileOpen(5, arquivo, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        Dim difft As Integer
        difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)


        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")


        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1

            'Dim dia, mes, hora As String
            dia = datacomeço.Day
            If dia.ToString.Length < 2 Then
                dia = "0" & dia
            End If
            mes = datacomeço.Month
            If mes.ToString.Length < 2 Then
                mes = "0" & mes
            End If
            hora = datacomeço.Hour
            If hora.ToString.Length < 2 Then
                hora = "0" & hora
            End If

            ' Preenche os valores do registro
            PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

        Next
        FileClose()

        'End If

    End Sub

    Sub SalvaArquivoONS()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados diários de previsão para a ONS(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        arquivo = diretóriodoprojeto & "\bgr" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        postoselecionado = 739

        'FileOpen(5, sfd1.FileName, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        'Dim difft As Integer
        'difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)

        Dim toto As Integer
        toto = 24 * 15
        Dim prevONS(toto) As Single 'prevONS(hora,dia,mes) = previsão

        For hh As Integer = 1 To toto
            prevONS(hh) = -1
        Next
        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")

        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1



            ' Preenche os valores do registro
            ' PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

            If datacomeço >= datahoje Then
                contadorONS += 1
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor)
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor)
            End If

        Next
        ' FileClose()
        Dim inteiro(6) As Integer
        Dim Média(6) As Single
        contadorONS = 0
        Dim contamédia As Integer
        contamédia = 0
        For dd As Integer = 1 To 6
            contamédia = 0
            For hh As Integer = 1 To 24
                contadorONS += 1
                If prevONS(contadorONS) > 0 Then
                    Média(dd) += prevONS(contadorONS)
                    contamédia += 1
                End If
            Next
            Média(dd) = Média(dd) / contamédia
            inteiro(dd) = Média(dd)
        Next

        'Remoção de viés da previsão de 1 dia
        If Média(1) <= 100 Then Média(1) += 50
        If Média(1) > 100 And Média(1) <= 200 Then Média(1) += 30

        FileOpen(5, arquivo, OpenMode.Output)
        ' Dim dia, mes, hora As String
        dia = datahoje.Day
        If dia.ToString.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datahoje.Month
        If mes.ToString.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datahoje.Hour
        If hora.ToString.Length < 2 Then
            hora = "0" & hora
        End If
        PrintLine(5, datahoje.Year & mes & dia)

        Print(5, "BGR")
        For dd As Integer = 1 To 5
            If inteiro(dd) >= 1000 Then
                Print(5, "  " & inteiro(dd))
            End If
            If inteiro(dd) < 1000 And inteiro(dd) >= 100 Then
                Print(5, "   " & inteiro(dd))
            End If
            If inteiro(dd) < 100 And inteiro(dd) >= 10 Then
                Print(5, "    " & inteiro(dd))
            End If
            If inteiro(dd) < 10 And inteiro(dd) >= 1 Then
                Print(5, "     " & inteiro(dd))
            End If
        Next

        PrintLine(5, "      :1,500")
        FileClose()

        'End If

    End Sub

    Sub CalculaArquivoWord_BGR()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 739


        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(1, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(1, contadordiamenor) 'Se for previsão, soma simulação

            End If
        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(6) As Integer 'Valor inteiro
        Dim Média(6) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To 6
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Remoção de viés da previsão de 1 dia
        If Média(1) <= 100 Then Média(1) += 50
        If Média(1) > 100 And Média(1) <= 200 Then Média(1) += 30

        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(6) As Integer 'Valor inteiro
        Dim MédiaCHU(6) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To 6
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas 
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma
        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)

        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\Previsão_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\Boletim_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloBarraGrande.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data5)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try


        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub CalculaArquivoWord_BGR6d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 739


        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(1, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(1, contadordiamenor) 'Se for previsão, soma simulação

            End If
        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 10

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Remoção de viés da previsão de 1 dia
        If Média(1) <= 100 Then Média(1) += 50
        If Média(1) > 100 And Média(1) <= 200 Then Média(1) += 30

        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas  
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas  
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma
        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)


        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\Previsão_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\Boletim_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloBarraGrande6d.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data6)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        'SubstituiVariavel("@DATA07", Data7)
        'SubstituiVariavel("@CHUVA07", Chuva07)
        'SubstituiVariavel("@VAZAO07", Vazao07)
        'SubstituiVariavel("@DATA08", Data8)
        'SubstituiVariavel("@CHUVA08", Chuva08)
        'SubstituiVariavel("@VAZAO08", Vazao08)
        'SubstituiVariavel("@DATA09", Data9)
        'SubstituiVariavel("@CHUVA09", Chuva09)
        'SubstituiVariavel("@VAZAO09", Vazao09)
        'SubstituiVariavel("@DATA10", Data10)
        'SubstituiVariavel("@CHUVA10", Chuva10)
        'SubstituiVariavel("@VAZAO10", Vazao10)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try


        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub CalculaArquivoWord_BGR7d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 739


        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora


        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(1, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(1, contadordiamenor) 'Se for previsão, soma simulação

            End If
        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 10

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Remoção de viés da previsão de 1 dia
        If Média(1) <= 100 Then Média(1) += 50
        If Média(1) > 100 And Média(1) <= 200 Then Média(1) += 30

        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas  
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas  
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma
        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)


        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\Previsão_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\Boletim_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloBarraGrande7d.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data7)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@DATA07", Data7)
        SubstituiVariavel("@CHUVA07", Chuva07)
        SubstituiVariavel("@VAZAO07", Vazao07)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try


        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub CalculaArquivoWord_BGR7d_ChuvaPto()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 739


        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora
        Dim prevCHUONS_2(toto) As Single ' = previsão de chuva de hora em hora


        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
            prevCHUONS_2(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(3, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(3, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(3, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(3, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(8, contadordiamenor) >= 0 Then prevCHUONS_2(contadorONS) = chuvasub2(8, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(8, contadordiamenor) >= 0 Then prevCHUONS_2(contadorONS) = chuvasub1(8, contadordiamenor) 'Se for previsão, soma simulação

            End If
        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 10

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Remoção de viés da previsão de 1 dia
        If Média(1) <= 100 Then Média(1) += 50
        If Média(1) > 100 And Média(1) <= 200 Then Média(1) += 30

        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        '--Agora entra no cálculo das médias diárias ds chuvas no reservtório
        'Variáveis
        Dim inteiroCHU2(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU2(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU2 As Integer 'contador de dados usados na média
        contamédiaCHU2 = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU2 = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS_2(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU2(dd) += prevCHUONS_2(contadorONS) 'parte superior da média
                    contamédiaCHU2 += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU2(dd) = MédiaCHU2(dd) 'calcula a média (somente acumulação)
            inteiroCHU2(dd) = MédiaCHU2(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas  
        Dim Chuvap01, Chuvap02, Chuvap03, Chuvap04, Chuvap05, Chuvap06, Chuvap07, Chuvap08, Chuvap09, Chuvap10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas  
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas  
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma
        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)


        Chuvap01 = inteiroCHU2(1)
        Chuvap02 = inteiroCHU2(2)
        Chuvap03 = inteiroCHU2(3)
        Chuvap04 = inteiroCHU2(4)
        Chuvap05 = inteiroCHU2(5)
        Chuvap06 = inteiroCHU2(6)
        Chuvap07 = inteiroCHU2(7)
        Chuvap08 = inteiroCHU2(8)
        Chuvap09 = inteiroCHU2(9)
        Chuvap10 = inteiroCHU2(10)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)


        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\Previsão_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\Boletim_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloBarraGrande7d.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data7)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVAP01", Chuvap01)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVAP02", Chuvap02)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVAP03", Chuvap03)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVAP04", Chuvap04)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVAP05", Chuvap05)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVAP06", Chuvap06)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@DATA07", Data7)
        SubstituiVariavel("@CHUVAP07", Chuvap07)
        SubstituiVariavel("@CHUVA07", Chuva07)
        SubstituiVariavel("@VAZAO07", Vazao07)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try


        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub CalculaArquivoWord_BGR10d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 739


        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(1, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(1, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(1, contadordiamenor) 'Se for previsão, soma simulação

            End If
        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 10

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Remoção de viés da previsão de 1 dia
        If Média(1) <= 100 Then Média(1) += 50
        If Média(1) > 100 And Média(1) <= 200 Then Média(1) += 30

        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas  
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas  
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma
        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)


        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\Previsão_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\Boletim_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloBarraGrande10d.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data5)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@DATA07", Data7)
        SubstituiVariavel("@CHUVA07", Chuva07)
        SubstituiVariavel("@VAZAO07", Vazao07)
        SubstituiVariavel("@DATA08", Data8)
        SubstituiVariavel("@CHUVA08", Chuva08)
        SubstituiVariavel("@VAZAO08", Vazao08)
        SubstituiVariavel("@DATA09", Data9)
        SubstituiVariavel("@CHUVA09", Chuva09)
        SubstituiVariavel("@VAZAO09", Vazao09)
        SubstituiVariavel("@DATA10", Data10)
        SubstituiVariavel("@CHUVA10", Chuva10)
        SubstituiVariavel("@VAZAO10", Vazao10)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try


        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub SalvaHidrogramaCNO()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        arquivo = diretóriodoprojeto & "\PrevisãoFCH_" & Now.Year & mes & dia & hora & ".txt"

        postoselecionado = 786


        FileOpen(5, arquivo, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        Dim difft As Integer
        difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)


        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")


        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1

            'Dim dia, mes, hora As String
            dia = datacomeço.Day
            If dia.ToString.Length < 2 Then
                dia = "0" & dia
            End If
            mes = datacomeço.Month
            If mes.ToString.Length < 2 Then
                mes = "0" & mes
            End If
            hora = datacomeço.Hour
            If hora.ToString.Length < 2 Then
                hora = "0" & hora
            End If

            ' Preenche os valores do registro
            PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

        Next
        FileClose()

        'End If

    End Sub

    Sub SalvaArquivoONS_CNO_v2()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados diários de previsão para a ONS(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        arquivo = diretóriodoprojeto & "\fch_completo" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        postoselecionado = 786

        'FileOpen(5, sfd1.FileName, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        'Dim difft As Integer
        'difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)

        Dim toto As Integer
        toto = 24 * numerodedias
        Dim prevONS(toto) As Single 'prevONS(hora,dia,mes) = previsão

        For hh As Integer = 1 To toto
            prevONS(hh) = -1
        Next
        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")

        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1



            ' Preenche os valores do registro
            ' PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

            If datacomeço >= datahoje Then
                contadorONS += 1
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor)
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor)
            End If

        Next
        ' FileClose()
        Dim inteiro(6) As Integer
        Dim Média(6) As Single
        contadorONS = 0
        Dim contamédia As Integer
        contamédia = 0
        For dd As Integer = 1 To 6
            contamédia = 0
            For hh As Integer = 1 To 24
                contadorONS += 1
                If prevONS(contadorONS) > 0 Then
                    Média(dd) += prevONS(contadorONS)
                    contamédia += 1
                End If
            Next
            Média(dd) = Média(dd) / contamédia
            inteiro(dd) = Média(dd)
        Next

        'Verifica se a média é menor que o maior valor de hoje. Se sim, a média vira o maior valor para hoje.
        contadorONS = 0
        For hh As Integer = 1 To 24
            contadorONS += 1
            If prevONS(contadorONS) > 0 Then
                If Média(1) < prevONS(contadorONS) Then
                    Média(1) = prevONS(contadorONS)
                End If
            End If
        Next
        inteiro(1) = Média(1)

        FileOpen(5, arquivo, OpenMode.Output)
        ' Dim dia, mes, hora As String
        dia = datahoje.Day
        If dia.ToString.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datahoje.Month
        If mes.ToString.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datahoje.Hour
        If hora.ToString.Length < 2 Then
            hora = "0" & hora
        End If
        PrintLine(5, datahoje.Year & mes & dia)

        Print(5, "FCH")
        For dd As Integer = 1 To 5
            If inteiro(dd) >= 1000 Then
                Print(5, "  " & inteiro(dd))
            End If
            If inteiro(dd) < 1000 And inteiro(dd) >= 100 Then
                Print(5, "   " & inteiro(dd))
            End If
            If inteiro(dd) < 100 And inteiro(dd) >= 10 Then
                Print(5, "    " & inteiro(dd))
            End If
            If inteiro(dd) < 10 And inteiro(dd) >= 1 Then
                Print(5, "     " & inteiro(dd))
            End If
        Next

        PrintLine(5, "  :1,600")
        FileClose()

        'End If

    End Sub

    Sub SalvaArquivoONS_CNO2_v2()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados diários de previsão para a ONS(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        'arquivo = diretóriodoprojeto & "\cn_completo" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        arquivo = diretóriodoprojeto & "\cn" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        postoselecionado = 753

        'FileOpen(5, sfd1.FileName, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        'Dim difft As Integer
        'difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)

        Dim toto As Integer
        toto = 24 * 15
        Dim prevONS(toto) As Single 'prevONS(hora,dia,mes) = previsão

        For hh As Integer = 1 To toto
            prevONS(hh) = -1
        Next
        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")

        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1



            ' Preenche os valores do registro
            ' PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

            If datacomeço >= datahoje Then
                contadorONS += 1
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor)
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor)
            End If

        Next
        ' FileClose()
        Dim inteiro(6) As Integer
        Dim Média(6) As Single
        contadorONS = 0
        Dim contamédia As Integer
        contamédia = 0
        For dd As Integer = 1 To 6
            contamédia = 0
            For hh As Integer = 1 To 24
                contadorONS += 1
                If prevONS(contadorONS) > 0 Then
                    Média(dd) += prevONS(contadorONS)
                    contamédia += 1
                End If
            Next
            Média(dd) = Média(dd) / contamédia
            Média(dd) = Média(dd) * Incremental_CN
            inteiro(dd) = Média(dd)
        Next

        'Verifica se a média é menor que o maior valor de hoje. Se sim, a média vira o maior valor para hoje.
        contadorONS = 0
        For hh As Integer = 1 To 24
            contadorONS += 1
            If prevONS(contadorONS) > 0 Then
                If Média(1) / Incremental_CN < prevONS(contadorONS) Then
                    Média(1) = prevONS(contadorONS)
                    Média(1) = Média(1) * Incremental_CN
                End If
            End If
        Next
        inteiro(1) = Média(1)

        FileOpen(5, arquivo, OpenMode.Output)
        ' Dim dia, mes, hora As String
        dia = datahoje.Day
        If dia.ToString.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datahoje.Month
        If mes.ToString.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datahoje.Hour
        If hora.ToString.Length < 2 Then
            hora = "0" & hora
        End If
        PrintLine(5, datahoje.Year & mes & dia)

        Print(5, "CNO")
        For dd As Integer = 1 To 6
            If inteiro(dd) >= 1000 Then
                Print(5, "  " & inteiro(dd))
            End If
            If inteiro(dd) < 1000 And inteiro(dd) >= 100 Then
                Print(5, "   " & inteiro(dd))
            End If
            If inteiro(dd) < 100 And inteiro(dd) >= 10 Then
                Print(5, "    " & inteiro(dd))
            End If
            If inteiro(dd) < 10 And inteiro(dd) >= 1 Then
                Print(5, "     " & inteiro(dd))
            End If
        Next

        PrintLine(5, "  :1,600")
        FileClose()

        'End If

    End Sub

    Sub SalvaArquivoONS_CNO2_v3()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados diários de previsão para a ONS(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        'arquivo = diretóriodoprojeto & "\cn_completo" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        arquivo = diretóriodoprojeto & "\fch" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        Dim NpostosInc As Integer
        NpostosInc = 12

        Dim PostosSelecionados(NpostosInc) As Integer
        PostosSelecionados(1) = 664
        PostosSelecionados(2) = 571
        PostosSelecionados(3) = 178
        PostosSelecionados(4) = 180
        PostosSelecionados(5) = 182
        PostosSelecionados(6) = 183
        PostosSelecionados(7) = 184
        PostosSelecionados(8) = 185
        PostosSelecionados(9) = 186
        PostosSelecionados(10) = 431
        PostosSelecionados(11) = 190
        PostosSelecionados(12) = 189

        'postoselecionado = 664

        'FileOpen(5, sfd1.FileName, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        'Dim difft As Integer
        'difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)

        Dim toto As Integer
        toto = 24 * numerodedias
        Dim prevONS(toto) As Single 'prevONS(hora,dia,mes) = previsão

        For hh As Integer = 1 To toto
            prevONS(hh) = -1
        Next
        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")

        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1



            ' Preenche os valores do registro
            ' PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

            If datacomeço >= datahoje Then
                contadorONS += 1

                'Faz um somatório para cada posto selecionado como incremental
                For PostoAvaliado As Integer = 1 To NpostosInc
                    If QPROPHIG(PostosSelecionados(PostoAvaliado), contadordiamenor) > 0 Then prevONS(contadorONS) += QPROPHIG(PostosSelecionados(PostoAvaliado), contadordiamenor)
                    If VazaoHIG(PostosSelecionados(PostoAvaliado), contadordiamenor) > 0 Then prevONS(contadorONS) += VazaoHIG(PostosSelecionados(PostoAvaliado), contadordiamenor)
                Next
            End If

        Next
        ' FileClose()
        Dim inteiro(6) As Integer
        Dim Média(6) As Single
        contadorONS = 0
        Dim contamédia As Integer
        contamédia = 0
        For dd As Integer = 1 To 6
            contamédia = 0
            For hh As Integer = 1 To 24
                contadorONS += 1
                If prevONS(contadorONS) > 0 Then
                    Média(dd) += prevONS(contadorONS)
                    contamédia += 1
                End If
            Next
            Média(dd) = Média(dd) / contamédia
            Média(dd) = Média(dd) * Incremental_CN
            inteiro(dd) = Média(dd)
        Next

        'Verifica se a média é menor que o maior valor de hoje. Se sim, a média vira o maior valor para hoje.
        contadorONS = 0
        For hh As Integer = 1 To 24
            contadorONS += 1
            If prevONS(contadorONS) > 0 Then
                If Média(1) / Incremental_CN < prevONS(contadorONS) Then
                    Média(1) = prevONS(contadorONS)
                    Média(1) = Média(1) * Incremental_CN
                End If
            End If
        Next
        inteiro(1) = Média(1)

        For contador As Integer = 1 To 6
            Incremental(contador) = inteiro(contador)
        Next

        FileOpen(5, arquivo, OpenMode.Output)
        ' Dim dia, mes, hora As String
        dia = datahoje.Day
        If dia.ToString.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datahoje.Month
        If mes.ToString.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datahoje.Hour
        If hora.ToString.Length < 2 Then
            hora = "0" & hora
        End If
        PrintLine(5, datahoje.Year & mes & dia)

        Print(5, "FCH")
        For dd As Integer = 1 To 5
            If inteiro(dd) >= 1000 Then
                Print(5, "  " & inteiro(dd))
            End If
            If inteiro(dd) < 1000 And inteiro(dd) >= 100 Then
                Print(5, "   " & inteiro(dd))
            End If
            If inteiro(dd) < 100 And inteiro(dd) >= 10 Then
                Print(5, "    " & inteiro(dd))
            End If
            If inteiro(dd) < 10 And inteiro(dd) >= 1 Then
                Print(5, "     " & inteiro(dd))
            End If
        Next

        PrintLine(5, "  :1,600")
        FileClose()

        'End If

    End Sub

    Sub SalvaArquivoONS_CNO()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados diários de previsão para a ONS(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        arquivo = diretóriodoprojeto & "\cn_completo" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        postoselecionado = 52

        'FileOpen(5, sfd1.FileName, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        'Dim difft As Integer
        'difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)

        Dim toto As Integer
        toto = 24 * 15
        Dim prevONS(toto) As Single 'prevONS(hora,dia,mes) = previsão

        For hh As Integer = 1 To toto
            prevONS(hh) = -1
        Next
        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")

        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1

            'Dim dia, mes, hora As String
            'dia = datacomeço.Day
            'If dia.ToString.Length < 2 Then
            '    dia = "0" & dia
            'End If
            'mes = datacomeço.Month
            'If mes.ToString.Length < 2 Then
            '    mes = "0" & mes
            'End If
            'hora = datacomeço.Hour
            'If hora.ToString.Length < 2 Then
            '    hora = "0" & hora
            'End If

            ' Preenche os valores do registro
            ' PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

            If datacomeço >= datahoje Then
                contadorONS += 1
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor)
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor)
            End If

        Next
        ' FileClose()
        Dim inteiro(6) As Integer
        Dim Média(6) As Single
        contadorONS = 0
        Dim contamédia As Integer
        contamédia = 0
        For dd As Integer = 1 To 6
            contamédia = 0
            For hh As Integer = 1 To 24
                contadorONS += 1
                If prevONS(contadorONS) > 0 Then
                    Média(dd) += prevONS(contadorONS)
                    contamédia += 1
                End If
            Next
            Média(dd) = Média(dd) / contamédia

            'Linha de Campos Novos para multiplicar a previsão na Usina
            Média(dd) = Média(dd) * (14547 / 12015)

            inteiro(dd) = Média(dd)
        Next

        'Le os dados mais recentes
        FileOpen(6, DIR_RHAMA_Q & "\" & "Dados_Hidro.txt", OpenMode.Input)
        Dim VazaoHoje As Integer
        Dim linha6 As String
        Do Until EOF(6)
            linha6 = LineInput(6)
            Try
                VazaoHoje = linha6.Substring(52, 6)
            Catch ex As Exception
                Exit Do
            End Try
        Loop
        FileClose()

        'Calcula e aplica o coeficiente de correção
        Dim CoefCor As Double
        CoefCor = VazaoHoje / Média(1)
        For dd As Integer = 1 To 6
            Média(dd) = 1.06 * Média(dd) * CoefCor
            inteiro(dd) = Média(dd)
        Next

        FileOpen(5, arquivo, OpenMode.Output)
        ' Dim dia, mes, hora As String
        dia = datahoje.Day
        If dia.ToString.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datahoje.Month
        If mes.ToString.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datahoje.Hour
        If hora.ToString.Length < 2 Then
            hora = "0" & hora
        End If
        PrintLine(5, datahoje.Year & mes & dia)

        Print(5, "CNO")
        For dd As Integer = 1 To 6
            If inteiro(dd) >= 1000 Then
                Print(5, "  " & inteiro(dd))
            End If
            If inteiro(dd) < 1000 And inteiro(dd) >= 100 Then
                Print(5, "   " & inteiro(dd))
            End If
            If inteiro(dd) < 100 And inteiro(dd) >= 10 Then
                Print(5, "    " & inteiro(dd))
            End If
            If inteiro(dd) < 10 And inteiro(dd) >= 1 Then
                Print(5, "     " & inteiro(dd))
            End If
        Next

        PrintLine(5, "  :1,600")
        FileClose()

        'End If

    End Sub

    Sub SalvaArquivoONS_CNO2()
        'Esta rotina escreve o hidrograma do local selecionado

        'Dim sfd1 As New Windows.Forms.SaveFileDialog
        'sfd1.Filter = "ASCII com dados diários de previsão para a ONS(.txt)|*.txt"

        'If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Prev"

        Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        arquivo = diretóriodoprojeto & "\cn" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        postoselecionado = 52

        'FileOpen(5, sfd1.FileName, OpenMode.Output)

        'Cria uma variável e calcula o número de intervalos de tempo que ela possui
        'Dim difft As Integer
        'difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado
        datacomeço = datacomeço.AddHours(-1)

        Dim toto As Integer
        toto = 24 * 15
        Dim prevONS(toto) As Single 'prevONS(hora,dia,mes) = previsão

        For hh As Integer = 1 To toto
            prevONS(hh) = -1
        Next
        'Dim datacomeço As Date
        'datacomeço = dataI.AddHours(-1)

        'Dim contadordiamenor As Integer
        'contadordiamenor = 0

        ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")

        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        Dim contadorONS As Integer
        contadorONS = 0

        For t As Integer = 1 To numerodedias 'difft
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1

            'Dim dia, mes, hora As String
            'dia = datacomeço.Day
            'If dia.ToString.Length < 2 Then
            '    dia = "0" & dia
            'End If
            'mes = datacomeço.Month
            'If mes.ToString.Length < 2 Then
            '    mes = "0" & mes
            'End If
            'hora = datacomeço.Hour
            'If hora.ToString.Length < 2 Then
            '    hora = "0" & hora
            'End If

            ' Preenche os valores do registro
            ' PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

            If datacomeço >= datahoje Then
                contadorONS += 1
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor)
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor)
            End If

        Next
        ' FileClose()
        Dim inteiro(6) As Integer
        Dim Média(6) As Single
        contadorONS = 0
        Dim contamédia As Integer
        contamédia = 0
        For dd As Integer = 1 To 6
            contamédia = 0
            For hh As Integer = 1 To 24
                contadorONS += 1
                If prevONS(contadorONS) > 0 Then
                    Média(dd) += prevONS(contadorONS)
                    contamédia += 1
                End If
            Next
            Média(dd) = Média(dd) / contamédia

            'Linha de Campos Novos para multiplicar a previsão na Usina
            Média(dd) = Média(dd) * (14547 / 12015)

            inteiro(dd) = Média(dd)
        Next

        'Le os dados mais recentes
        FileOpen(6, DIR_RHAMA_Q & "\" & "Dados_Hidro.txt", OpenMode.Input)
        Dim VazaoHoje As Integer
        Dim linha6 As String
        Do Until EOF(6)
            linha6 = LineInput(6)
            Try
                VazaoHoje = linha6.Substring(52, 6)
            Catch ex As Exception
                Exit Do
            End Try
        Loop
        FileClose()

        'Calcula e aplica o coeficiente de correção
        Dim CoefCor As Double
        CoefCor = VazaoHoje / Média(1)
        For dd As Integer = 1 To 6
            Média(dd) = 0.06 * 1.06 * Média(dd) * CoefCor
            inteiro(dd) = Média(dd)
        Next

        FileOpen(5, arquivo, OpenMode.Output)
        ' Dim dia, mes, hora As String
        dia = datahoje.Day
        If dia.ToString.Length < 2 Then
            dia = "0" & dia
        End If
        mes = datahoje.Month
        If mes.ToString.Length < 2 Then
            mes = "0" & mes
        End If
        hora = datahoje.Hour
        If hora.ToString.Length < 2 Then
            hora = "0" & hora
        End If
        PrintLine(5, datahoje.Year & mes & dia)

        Print(5, "CNO")
        For dd As Integer = 1 To 6
            If inteiro(dd) >= 1000 Then
                Print(5, "  " & inteiro(dd))
            End If
            If inteiro(dd) < 1000 And inteiro(dd) >= 100 Then
                Print(5, "   " & inteiro(dd))
            End If
            If inteiro(dd) < 100 And inteiro(dd) >= 10 Then
                Print(5, "    " & inteiro(dd))
            End If
            If inteiro(dd) < 10 And inteiro(dd) >= 1 Then
                Print(5, "     " & inteiro(dd))
            End If
        Next

        PrintLine(5, "  :1,600")
        FileClose()

        'End If

    End Sub

    Sub CalculaArquivoWord_CNO()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 753

        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        'Data e dados de ontem às 7h da manhã
        Dim dataOntem7 As Date
        dataOntem7 = Now.Day & "/" & Now.Month & "/" & Now.Year & " 07:00:00"
        dataOntem7 = dataOntem7.AddDays(-1)
        Dim prevCHUONS7(toto + 24) As Single
        For hh As Integer = 1 To toto + 24
            prevCHUONS7(hh) = -1
        Next
        Dim contadorONS7 As Integer
        contadorONS7 = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação

            End If


            ' Preenche os valores do registro
            If datacomeço >= dataOntem7 Then 'Se entramos no dia de hoje
                contadorONS7 += 1
                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação
            End If

        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(6) As Integer 'Valor inteiro
        Dim Média(6) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To 6
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Aproxima as incrementais
        Dim Incremental(6) As Integer
        For dd As Integer = 1 To 6
            Incremental(dd) = Média(dd) * Incremental_CN
        Next
        


        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(6) As Integer 'Valor inteiro
        Dim MédiaCHU(6) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To 6
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, P7H, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas 
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Inc01, Inc02, Inc03, Inc04, Inc05, Inc06 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma

        soma = 0
        For i As Integer = 1 To 25
            If prevCHUONS7(i) >= 0 Then
                soma += prevCHUONS7(i)
            End If
        Next
        P7H = soma

        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)

        Inc01 = Incremental(1)
        Inc02 = Incremental(2)
        Inc03 = Incremental(3)
        Inc04 = Incremental(4)
        Inc05 = Incremental(5)
        Inc06 = Incremental(6)

        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\PrevisãoCNO_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\BoletimCNO_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloCamposNovos.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@P7H", P7H)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data6)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@INC01", Inc01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@INC02", Inc02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@INC03", Inc03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@INC04", Inc04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@INC05", Inc05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@INC06", Inc06)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub CalculaArquivoWord_CNO6d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 753

        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        'Data e dados de ontem às 7h da manhã
        Dim dataOntem7 As Date
        dataOntem7 = Now.Day & "/" & Now.Month & "/" & Now.Year & " 07:00:00"
        dataOntem7 = dataOntem7.AddDays(-1)
        Dim prevCHUONS7(toto + 24) As Single
        For hh As Integer = 1 To toto + 24
            prevCHUONS7(hh) = -1
        Next
        Dim contadorONS7 As Integer
        contadorONS7 = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação

            End If


            ' Preenche os valores do registro
            If datacomeço >= dataOntem7 Then 'Se entramos no dia de hoje
                contadorONS7 += 1
                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação
            End If

        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 10

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Aproxima as incrementais
        Dim Incremental(DiasFuturo) As Integer
        For dd As Integer = 1 To DiasFuturo
            Incremental(dd) = Média(dd) * Incremental_CN
        Next



        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, P7H, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas 
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Inc01, Inc02, Inc03, Inc04, Inc05, Inc06, Inc07, Inc08, Inc09, Inc10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)


        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma

        soma = 0
        For i As Integer = 1 To 25
            If prevCHUONS7(i) >= 0 Then
                soma += prevCHUONS7(i)
            End If
        Next
        P7H = soma

        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)

        Inc01 = Incremental(1)
        Inc02 = Incremental(2)
        Inc03 = Incremental(3)
        Inc04 = Incremental(4)
        Inc05 = Incremental(5)
        Inc06 = Incremental(6)
        Inc07 = Incremental(7)
        Inc08 = Incremental(8)
        Inc09 = Incremental(9)
        Inc10 = Incremental(10)

        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\PrevisãoCNO_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\BoletimCNO_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloCamposNovos6d.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@P7H", P7H)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data6)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@INC01", Inc01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@INC02", Inc02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@INC03", Inc03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@INC04", Inc04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@INC05", Inc05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@INC06", Inc06)
        'SubstituiVariavel("@DATA07", Data7)
        'SubstituiVariavel("@CHUVA07", Chuva07)
        'SubstituiVariavel("@VAZAO07", Vazao07)
        'SubstituiVariavel("@INC07", Inc07)
        'SubstituiVariavel("@DATA08", Data8)
        'SubstituiVariavel("@CHUVA08", Chuva08)
        'SubstituiVariavel("@VAZAO08", Vazao08)
        'SubstituiVariavel("@INC08", Inc08)
        'SubstituiVariavel("@DATA09", Data9)
        'SubstituiVariavel("@CHUVA09", Chuva09)
        'SubstituiVariavel("@VAZAO09", Vazao09)
        'SubstituiVariavel("@INC09", Inc09)
        'SubstituiVariavel("@DATA10", Data10)
        'SubstituiVariavel("@CHUVA10", Chuva10)
        'SubstituiVariavel("@VAZAO10", Vazao10)
        'SubstituiVariavel("@INC10", Inc10)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub CalculaArquivoWord_CNO7d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 750 '753
        'originalmente deveria ser o 753, porém eles pediram para mudar...

        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        'Data e dados de ontem às 7h da manhã
        Dim dataOntem7 As Date
        dataOntem7 = Now.Day & "/" & Now.Month & "/" & Now.Year & " 07:00:00"
        dataOntem7 = dataOntem7.AddDays(-1)
        Dim prevCHUONS7(toto + 24) As Single
        For hh As Integer = 1 To toto + 24
            prevCHUONS7(hh) = -1
        Next
        Dim contadorONS7 As Integer
        contadorONS7 = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação

            End If


            ' Preenche os valores do registro
            If datacomeço >= dataOntem7 Then 'Se entramos no dia de hoje
                contadorONS7 += 1
                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação
            End If

        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 10

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Verifica se a média é menor que o maior valor de hoje. Se sim, a média vira o maior valor para hoje.
        contadorONS = 0
        For hh As Integer = 1 To 24
            contadorONS += 1
            If prevONS(contadorONS) > 0 Then
                If Média(1) < prevONS(contadorONS) Then
                    Média(1) = prevONS(contadorONS)
                End If
            End If
        Next
        inteiro(1) = Média(1)

        'Aproxima as incrementais
        Incremental(1) = Incremental(1)
        Incremental(2) = Incremental(2)
        Incremental(3) = Incremental(3)
        Incremental(4) = Incremental(4)
        Incremental(5) = Incremental(5)
        Incremental(6) = Incremental(6)
        Incremental(7) = Incremental(6)
        Incremental(8) = Incremental(6)
        Incremental(9) = Incremental(6)
        Incremental(10) = Incremental(6)

        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, P7H, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas 
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Inc01, Inc02, Inc03, Inc04, Inc05, Inc06, Inc07, Inc08, Inc09, Inc10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)


        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma

        soma = 0
        For i As Integer = 1 To 25
            If prevCHUONS7(i) >= 0 Then
                soma += prevCHUONS7(i)
            End If
        Next
        P7H = soma

        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)

        Inc01 = Incremental(1)
        Inc02 = Incremental(2)
        Inc03 = Incremental(3)
        Inc04 = Incremental(4)
        Inc05 = Incremental(5)
        Inc06 = Incremental(6)
        Inc07 = Incremental(7)
        Inc08 = Incremental(8)
        Inc09 = Incremental(9)
        Inc10 = Incremental(10)

        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\PrevisãoCNO_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\BoletimCNO_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloCamposNovos7d.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@P7H", P7H)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data7)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@INC01", Inc01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@INC02", Inc02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@INC03", Inc03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@INC04", Inc04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@INC05", Inc05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@INC06", Inc06)
        SubstituiVariavel("@DATA07", Data7)
        SubstituiVariavel("@CHUVA07", Chuva07)
        SubstituiVariavel("@VAZAO07", Vazao07)
        SubstituiVariavel("@INC07", Inc07)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Sub CalculaArquivoWord_CNO10d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 753

        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 15 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        'Data e dados de ontem às 7h da manhã
        Dim dataOntem7 As Date
        dataOntem7 = Now.Day & "/" & Now.Month & "/" & Now.Year & " 07:00:00"
        dataOntem7 = dataOntem7.AddDays(-1)
        Dim prevCHUONS7(toto + 24) As Single
        For hh As Integer = 1 To toto + 24
            prevCHUONS7(hh) = -1
        Next
        Dim contadorONS7 As Integer
        contadorONS7 = 0

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação

            End If


            ' Preenche os valores do registro
            If datacomeço >= dataOntem7 Then 'Se entramos no dia de hoje
                contadorONS7 += 1
                'O mesmo para chuva
                If chuvasub2(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub2(18, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(18, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub1(18, contadordiamenor) 'Se for previsão, soma simulação
            End If

        Next

        'MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 10

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        'Aproxima as incrementais
        Dim Incremental(DiasFuturo) As Integer
        For dd As Integer = 1 To DiasFuturo
            Incremental(dd) = Média(dd) * Incremental_CN
        Next



        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, P7H, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas 
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Inc01, Inc02, Inc03, Inc04, Inc05, Inc06, Inc07, Inc08, Inc09, Inc10 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)


        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma

        soma = 0
        For i As Integer = 1 To 25
            If prevCHUONS7(i) >= 0 Then
                soma += prevCHUONS7(i)
            End If
        Next
        P7H = soma

        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)

        Inc01 = Incremental(1)
        Inc02 = Incremental(2)
        Inc03 = Incremental(3)
        Inc04 = Incremental(4)
        Inc05 = Incremental(5)
        Inc06 = Incremental(6)
        Inc07 = Incremental(7)
        Inc08 = Incremental(8)
        Inc09 = Incremental(9)
        Inc10 = Incremental(10)

        Grafico = "Grafico"


        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\PrevisãoCNO_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\BoletimCNO_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloCamposNovos10d.doc", BOLETIM, True)


        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@P7H", P7H)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data10)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@INC01", Inc01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@INC02", Inc02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@INC03", Inc03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@INC04", Inc04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@INC05", Inc05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@INC06", Inc06)
        SubstituiVariavel("@DATA07", Data7)
        SubstituiVariavel("@CHUVA07", Chuva07)
        SubstituiVariavel("@VAZAO07", Vazao07)
        SubstituiVariavel("@INC07", Inc07)
        SubstituiVariavel("@DATA08", Data8)
        SubstituiVariavel("@CHUVA08", Chuva08)
        SubstituiVariavel("@VAZAO08", Vazao08)
        SubstituiVariavel("@INC08", Inc08)
        SubstituiVariavel("@DATA09", Data9)
        SubstituiVariavel("@CHUVA09", Chuva09)
        SubstituiVariavel("@VAZAO09", Vazao09)
        SubstituiVariavel("@INC09", Inc09)
        SubstituiVariavel("@DATA10", Data10)
        SubstituiVariavel("@CHUVA10", Chuva10)
        SubstituiVariavel("@VAZAO10", Vazao10)
        SubstituiVariavel("@INC10", Inc10)
        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--


    End Sub

    Private Sub SubstituiVariavel(ByVal Achar As String, ByVal Substituir As String)

        'procura a variável e substitui o valor

        With objWord.Selection.Find

            .Text = Achar.ToString

            .Replacement.Text = Substituir.ToString

            .Forward = True

            .Format = False

            .MatchCase = False

            .MatchWholeWord = False

            .MatchWildcards = False

            .MatchSoundsLike = False

            .MatchAllWordForms = False


            While .Execute = True

                objWord.Selection.Select()

                System.Windows.Forms.Clipboard.SetDataObject(Substituir)

                objWord.Selection.Paste()

            End While


        End With


    End Sub

    Private Sub Label10_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Label10.Click

    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        TotalCommander = True 'libera o download para o CPTEC
        'If ERRO_PREVISÃO < 2 Then
        'Dim frmFIG As New Form12
        'frmFIG.Show()

        Dim frmFIG2 As New Form12b
        frmFIG2.Show()

        'Me.SalvaHidrograma()
        Me.SalvaHidrogramaCNO()

        'Me.SalvaArquivoONS()
        Me.SalvaArquivoONS_CNO_v2()
        Me.SalvaArquivoONS_CNO2_v3()

        'Me.CalculaArquivoWord_BGR()
        'Me.CalculaArquivoWord_BGR10d()

        'Me.CalculaArquivoWord_CNO()
        'Me.CalculaArquivoWord_CNO10d()

        'Me.CalculaArquivoWord_BGR7d_ChuvaPto()
        'Me.CalculaArquivoWord_CNO7d()

        'Me.CalculaArquivoWord_FDC15d()
        Me.CalculaBoletim_FDC15d()


        'Else
        'ERRO_PREVISÃO = 0
        'Dim novaHora As Integer
        'novaHora = cbEMAILhora.Text + 1
        'Dim novaHoraH As String
        'novaHoraH = novaHora
        'If novaHoraH.Length < 2 Then novaHoraH = "0" & novaHoraH
        'cbEMAILhora.Text = novaHoraH
        'cbGRAVAhora.Text = novaHoraH
        'cbPOBShora.Text = novaHoraH
        'cbPPREVhora.Text = novaHoraH
        'cbQOBShora.Text = novaHoraH
        'cbUHEhora.Text = novaHoraH
        'cbRODARhora.Text = novaHoraH
        'End If
        TotalCommander = False 'libera o download para o CPTEC
        Label11.ForeColor = Drawing.Color.Red 'Indica que o CPTEC está funcionando
        Label11.Text = "Desligado"
        Timer1.Enabled = False 'Dispara o timer para funcionar

    End Sub

    Sub CalculaArquivoWord_FDC15d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 786

        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 20 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        'Data e dados de ontem às 7h da manhã
        Dim dataOntem7 As Date
        dataOntem7 = Now.Day & "/" & Now.Month & "/" & Now.Year & " 07:00:00"
        dataOntem7 = dataOntem7.AddDays(-1)
        Dim prevCHUONS7(toto + 24) As Single
        For hh As Integer = 1 To toto + 24
            prevCHUONS7(hh) = -1
        Next
        Dim contadorONS7 As Integer
        contadorONS7 = 0

        Dim SubBaciaChuvaIncremental As Integer
        SubBaciaChuvaIncremental = 30

        ' MsgBox("Vai começar a parte de integração em uma variável só")

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) 'Se for previsão, soma simulação

            End If


            ' Preenche os valores do registro
            If datacomeço >= dataOntem7 Then 'Se entramos no dia de hoje
                contadorONS7 += 1
                'O mesmo para chuva
                If chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) 'Se for previsão, soma simulação
            End If

        Next

        ' MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 15

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        ' MsgBox("Calculou as médias diárias inteiras")


        'Verifica se a média é menor que o maior valor de hoje. Se sim, a média vira o maior valor para hoje.
        contadorONS = 0
        For hh As Integer = 1 To 24
            contadorONS += 1
            If prevONS(contadorONS) > 0 Then
                If Média(1) < prevONS(contadorONS) Then
                    Média(1) = prevONS(contadorONS)
                End If
            End If
        Next
        inteiro(1) = Média(1)

        ' MsgBox("Verificou o primeiro dia")


        'Aproxima as incrementais
        Incremental(1) = Incremental(1)
        Incremental(2) = Incremental(2)
        Incremental(3) = Incremental(3)
        Incremental(4) = Incremental(4)
        Incremental(5) = Incremental(5)
        Incremental(6) = Incremental(6)
        Incremental(7) = Incremental(6)
        Incremental(8) = Incremental(6)
        Incremental(9) = Incremental(6)
        Incremental(10) = Incremental(6)
        Incremental(11) = Incremental(6)
        Incremental(12) = Incremental(6)
        Incremental(13) = Incremental(6)
        Incremental(14) = Incremental(6)
        Incremental(15) = Incremental(6)

        ' MsgBox("Calculou as incrementais")


        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        ' MsgBox("Calculou as chuvas médias")


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10, Data11, Data12, Data13, Data14, Data15 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, P7H, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10, Chuva11, Chuva12, Chuva13, Chuva14, Chuva15 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas 
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10, Vazao11, Vazao12, Vazao13, Vazao14, Vazao15 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Inc01, Inc02, Inc03, Inc04, Inc05, Inc06, Inc07, Inc08, Inc09, Inc10, Inc11, Inc12, Inc13, Inc14, Inc15 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)
        Data11 = datahoje.AddHours(+240)
        Data12 = datahoje.AddHours(+264)
        Data13 = datahoje.AddHours(+288)
        Data14 = datahoje.AddHours(+312)
        Data15 = datahoje.AddHours(+336)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma

        soma = 0
        For i As Integer = 1 To 25
            If prevCHUONS7(i) >= 0 Then
                soma += prevCHUONS7(i)
            End If
        Next
        P7H = soma

        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)
        Chuva11 = inteiroCHU(11)
        Chuva12 = inteiroCHU(12)
        Chuva13 = inteiroCHU(13)
        Chuva14 = inteiroCHU(14)
        Chuva15 = inteiroCHU(15)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)
        Vazao11 = inteiro(11)
        Vazao12 = inteiro(12)
        Vazao13 = inteiro(13)
        Vazao14 = inteiro(14)
        Vazao15 = inteiro(15)

        Inc01 = Incremental(1)
        Inc02 = Incremental(2)
        Inc03 = Incremental(3)
        Inc04 = Incremental(4)
        Inc05 = Incremental(5)
        Inc06 = Incremental(6)
        Inc07 = Incremental(7)
        Inc08 = Incremental(8)
        Inc09 = Incremental(9)
        Inc10 = Incremental(10)
        Inc11 = Incremental(11)
        Inc12 = Incremental(12)
        Inc13 = Incremental(13)
        Inc14 = Incremental(14)
        Inc15 = Incremental(15)

        Grafico = "Grafico"


        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())


        'Faz um arquivo txt
        FileOpen(2, diretóriodoprojeto & "\Prev\BoletimFCH_" & Now.Year & mes & dia & hora & ".txt", OpenMode.Output)

        PrintLine(2, "DATA PREVISÃO: " & Data1)
        PrintLine(2, "- PRECIPITAÇÃO TOTAL MÉDIA NA BACIA NAS ÚLTIMAS 8 HORAS: 	" & PH & " mm")
        PrintLine(2, "- PRECIPITAÇÃO TOTAL MÉDIA NA BACIA NAS ÚLTIMAS 24 HORAS: 	" & P7H & " mm")
        PrintLine(2, "- VAZÃO AFLUENTE NO LOCAL DA USINA 2: 	" & QH & " m³/s")
        PrintLine(2, Data1 & " " & Chuva01 & " " & Vazao01 & " " & Inc01)
        PrintLine(2, Data2 & " " & Chuva02 & " " & Vazao02 & " " & Inc02)
        PrintLine(2, Data3 & " " & Chuva03 & " " & Vazao03 & " " & Inc03)
        PrintLine(2, Data4 & " " & Chuva04 & " " & Vazao04 & " " & Inc04)
        PrintLine(2, Data5 & " " & Chuva05 & " " & Vazao05 & " " & Inc05)
        PrintLine(2, Data6 & " " & Chuva06 & " " & Vazao06 & " " & Inc06)
        PrintLine(2, Data7 & " " & Chuva07 & " " & Vazao07 & " " & Inc07)
        PrintLine(2, Data8 & " " & Chuva08 & " " & Vazao08 & " " & Inc08)
        PrintLine(2, Data9 & " " & Chuva09 & " " & Vazao09 & " " & Inc09)
        PrintLine(2, Data10 & " " & Chuva10 & " " & Vazao10 & " " & Inc10)
        PrintLine(2, Data11 & " " & Chuva11 & " " & Vazao11 & " " & Inc11)
        PrintLine(2, Data12 & " " & Chuva12 & " " & Vazao12 & " " & Inc12)
        PrintLine(2, Data13 & " " & Chuva13 & " " & Vazao13 & " " & Inc13)
        PrintLine(2, Data14 & " " & Chuva14 & " " & Vazao14 & " " & Inc14)
        PrintLine(2, Data15 & " " & Chuva15 & " " & Vazao15 & " " & Inc15)
        FileClose(2)
    

        'Faz um arquivo só com a informação de chuva nas últimas 24h
        FileOpen(3, diretóriodoprojeto & "\Prev\ChuvaAcum24hFCH_" & Now.Year & mes & dia & hora & ".txt", OpenMode.Output)
        PrintLine(3, "- PRECIPITAÇÃO TOTAL MÉDIA NA BACIA NAS ÚLTIMAS 24 HORAS: 	" & P7H & " mm")
        FileClose(3)

        '-- Copia o boletim do diretorio do projeto para o diretorio Prev

        'Dim dia, mes, hora As String
        'dia = datahoje.Day
        'If dia.ToString.Length < 2 Then
        '    dia = "0" & dia
        'End If
        'mes = datahoje.Month
        'If mes.ToString.Length < 2 Then
        '    mes = "0" & mes
        'End If
        ''hora = datacomeço.Hour
        ''If hora.ToString.Length < 2 Then
        ''    hora = "0" & hora
        ''End If
        'Dim AnoHoje As String
        'AnoHoje = datahoje.Year

        'Dim arquivo As String
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"

        
        Dim arquivoImagem As String
        arquivoImagem = diretóriodoprojeto & "\Prev\PrevisãoFDC_" & Now.Year & mes & dia & hora & ".bmp"

        Dim BOLETIM As String
        BOLETIM = diretóriodoprojeto & "\Prev\BoletimFDC_" & Now.Year & mes & dia & hora & ".doc"

        System.IO.File.Copy(diretóriodoprojeto & "\ModeloFozDoChapeco15d.doc", BOLETIM, True)

        Windows.Forms.Clipboard.Clear()

        'Preenche o Boletim

        'Instancia a Aplicação Word.

        objWord = CreateObject("Word.Application")

        'Abre o documento aviso.doc do Microsoft Word.

        objWord.Documents.Open(BOLETIM)

        'Vai preenchendo o boletim

        SubstituiVariavel("@DATASUP", Data1)
        SubstituiVariavel("@PH", PH)
        SubstituiVariavel("@P7H", P7H)
        SubstituiVariavel("@QH", QH)
        SubstituiVariavel("@DATAINI", Data1)
        SubstituiVariavel("@DATAFIM", Data10)
        SubstituiVariavel("@DATA01", Data1)
        SubstituiVariavel("@CHUVA01", Chuva01)
        SubstituiVariavel("@VAZAO01", Vazao01)
        SubstituiVariavel("@INC01", Inc01)
        SubstituiVariavel("@DATA02", Data2)
        SubstituiVariavel("@CHUVA02", Chuva02)
        SubstituiVariavel("@VAZAO02", Vazao02)
        SubstituiVariavel("@INC02", Inc02)
        SubstituiVariavel("@DATA03", Data3)
        SubstituiVariavel("@CHUVA03", Chuva03)
        SubstituiVariavel("@VAZAO03", Vazao03)
        SubstituiVariavel("@INC03", Inc03)
        SubstituiVariavel("@DATA04", Data4)
        SubstituiVariavel("@CHUVA04", Chuva04)
        SubstituiVariavel("@VAZAO04", Vazao04)
        SubstituiVariavel("@INC04", Inc04)
        SubstituiVariavel("@DATA05", Data5)
        SubstituiVariavel("@CHUVA05", Chuva05)
        SubstituiVariavel("@VAZAO05", Vazao05)
        SubstituiVariavel("@INC05", Inc05)
        SubstituiVariavel("@DATA06", Data6)
        SubstituiVariavel("@CHUVA06", Chuva06)
        SubstituiVariavel("@VAZAO06", Vazao06)
        SubstituiVariavel("@INC06", Inc06)
        SubstituiVariavel("@DATA07", Data7)
        SubstituiVariavel("@CHUVA07", Chuva07)
        SubstituiVariavel("@VAZAO07", Vazao07)
        SubstituiVariavel("@INC07", Inc07)
        SubstituiVariavel("@DATA08", Data8)
        SubstituiVariavel("@CHUVA08", Chuva08)
        SubstituiVariavel("@VAZAO08", Vazao08)
        SubstituiVariavel("@INC08", Inc08)
        SubstituiVariavel("@DATA09", Data9)
        SubstituiVariavel("@CHUVA09", Chuva09)
        SubstituiVariavel("@VAZAO09", Vazao09)
        SubstituiVariavel("@INC09", Inc09)
        SubstituiVariavel("@DATA10", Data10)
        SubstituiVariavel("@CHUVA10", Chuva10)
        SubstituiVariavel("@VAZAO10", Vazao10)
        SubstituiVariavel("@INC10", Inc10)

        SubstituiVariavel("@DATA11", Data11)
        SubstituiVariavel("@CHUVA11", Chuva11)
        SubstituiVariavel("@VAZAO11", Vazao11)
        SubstituiVariavel("@INC11", Inc11)

        SubstituiVariavel("@DATA12", Data12)
        SubstituiVariavel("@CHUVA12", Chuva12)
        SubstituiVariavel("@VAZAO12", Vazao12)
        SubstituiVariavel("@INC12", Inc12)

        SubstituiVariavel("@DATA13", Data13)
        SubstituiVariavel("@CHUVA13", Chuva13)
        SubstituiVariavel("@VAZAO13", Vazao13)
        SubstituiVariavel("@INC13", Inc13)

        SubstituiVariavel("@DATA14", Data14)
        SubstituiVariavel("@CHUVA14", Chuva14)
        SubstituiVariavel("@VAZAO14", Vazao14)
        SubstituiVariavel("@INC14", Inc14)

        SubstituiVariavel("@DATA15", Data15)
        SubstituiVariavel("@CHUVA15", Chuva15)
        SubstituiVariavel("@VAZAO15", Vazao15)
        SubstituiVariavel("@INC15", Inc15)

        SubstituiVariavel("@GRÁFICO", " ")

        Try
            'objWord.ActiveDocument.Bookmarks("Imagem").Application.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
            'Insere uma imagem de um arquivo para o documento do Word
            objWord.Selection.InlineShapes.AddPicture(arquivoImagem, LinkToFile:=False, SaveWithDocument:=True)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

        'faz esse processo para não dar erro no do salvar normal dotm 'adicionado em 16 de março de 2014
        objWord.NormalTemplate.Saved = True

        'Fecha o documento Word 
        objWord.ActiveDocument.Close(True)

        'Fecha o Word

        objWord.Quit()

        objWord = Nothing

        '--



    End Sub

    Sub CalculaBoletim_FDC15d()

        'Dim objWord As Object

        'Pega a data de hoje, incluindo a hora
        Dim dia, mes, hora As String
        dia = Now.Day
        If dia.Length < 2 Then dia = "0" & dia
        mes = Now.Month
        If mes.Length < 2 Then mes = "0" & mes
        hora = Now.Hour
        If hora.Length < 2 Then hora = "0" & hora

        '--Define o posto que devemos utilizar para calcular
        postoselecionado = 786

        'Cria as variáveis que serão utilizadas nos cálculos das médias
        Dim contadordiamenor, numerodedias As Integer 'variáveis auxiliares
        Dim datacomeço As Date 'variáveis auxiliares
        contadordiamenor = 0 'contador do número total de dias
        'para fazer o gráfico
        datacomeço = Começoresultado 'data inicial da simulação
        numerodedias = tamdoresultado 'número de horas total da simulação + previsão
        datacomeço = datacomeço.AddHours(-1) 'zera a data inicial

        'Cria esta variável de tempo para dimensionar a variábel de previsões médias
        Dim toto As Integer
        toto = 24 * 20 '24horas vezes 15 dias
        Dim prevONS(toto) As Single ' = previsão de hora em hora, mas somente a partir dos últimos dias
        Dim prevCHUONS(toto) As Single ' = previsão de chuva de hora em hora

        'Zera a variável de previsão
        For hh As Integer = 1 To toto
            prevONS(hh) = -1
            prevCHUONS(hh) = -1
        Next

        'Acerta o relógio com a data de hoje, porém com a hora ZERO!!! Para começar desde manhã cedo!!
        Dim datahoje As Date
        datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

        'Cria e zera mais um contador
        Dim contadorONS As Integer
        contadorONS = 0

        'Data e dados de ontem às 7h da manhã
        Dim dataOntem7 As Date
        dataOntem7 = Now.Day & "/" & Now.Month & "/" & Now.Year & " 07:00:00"
        dataOntem7 = dataOntem7.AddDays(-1)
        Dim prevCHUONS7(toto + 24) As Single
        For hh As Integer = 1 To toto + 24
            prevCHUONS7(hh) = -1
        Next
        Dim contadorONS7 As Integer
        contadorONS7 = 0

        Dim SubBaciaChuvaIncremental As Integer
        SubBaciaChuvaIncremental = 30

        ' MsgBox("Vai começar a parte de integração em uma variável só")

        For t As Integer = 1 To numerodedias 'para cada intervalo de tempo total na simulação + previsão
            datacomeço = datacomeço.AddHours(1) 'verifica a data
            contadordiamenor = contadordiamenor + 1 'soma um no contador

            ' Preenche os valores do registro
            If datacomeço >= datahoje Then 'Se entramos no dia de hoje
                contadorONS += 1 'soma um no contador de dados da região de interesse
                If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor) 'Se for simulação, soma previsão
                If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor) 'Se for previsão, soma simulação

                'O mesmo para chuva
                If chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS(contadorONS) = chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) 'Se for previsão, soma simulação

            End If


            ' Preenche os valores do registro
            If datacomeço >= dataOntem7 Then 'Se entramos no dia de hoje
                contadorONS7 += 1
                'O mesmo para chuva
                If chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub2(SubBaciaChuvaIncremental, contadordiamenor) 'Se for simulação, soma previsão
                If chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) >= 0 Then prevCHUONS7(contadorONS7) = chuvasub1(SubBaciaChuvaIncremental, contadordiamenor) 'Se for previsão, soma simulação
            End If

        Next

        ' MsgBox("Passou a parte de integração em uma variável só")

        Dim DiasFuturo As Integer
        DiasFuturo = 15

        '--Agora entra no cálculo das médias diárias ds vazões
        'Variáveis
        Dim inteiro(DiasFuturo) As Integer 'Valor inteiro
        Dim Média(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédia As Integer 'contador de dados usados na média
        contamédia = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédia = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    Média(dd) += prevONS(contadorONS) 'parte superior da média
                    contamédia += 1 'parte inferior da média
                End If
            Next 'loop das horas
            Média(dd) = Média(dd) / contamédia 'calcula a média
            inteiro(dd) = Média(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        ' MsgBox("Calculou as médias diárias inteiras")


        'Verifica se a média é menor que o maior valor de hoje. Se sim, a média vira o maior valor para hoje.
        contadorONS = 0
        For hh As Integer = 1 To 24
            contadorONS += 1
            If prevONS(contadorONS) > 0 Then
                If Média(1) < prevONS(contadorONS) Then
                    Média(1) = prevONS(contadorONS)
                End If
            End If
        Next
        inteiro(1) = Média(1)

        ' MsgBox("Verificou o primeiro dia")


        'Aproxima as incrementais
        Incremental(1) = Incremental(1)
        Incremental(2) = Incremental(2)
        Incremental(3) = Incremental(3)
        Incremental(4) = Incremental(4)
        Incremental(5) = Incremental(5)
        Incremental(6) = Incremental(6)
        Incremental(7) = Incremental(6)
        Incremental(8) = Incremental(6)
        Incremental(9) = Incremental(6)
        Incremental(10) = Incremental(6)
        Incremental(11) = Incremental(6)
        Incremental(12) = Incremental(6)
        Incremental(13) = Incremental(6)
        Incremental(14) = Incremental(6)
        Incremental(15) = Incremental(6)

        ' MsgBox("Calculou as incrementais")


        '--Agora entra no cálculo das médias diárias ds chuvas
        'Variáveis
        Dim inteiroCHU(DiasFuturo) As Integer 'Valor inteiro
        Dim MédiaCHU(DiasFuturo) As Single 'Valor real
        contadorONS = 0 'contador de IT's
        Dim contamédiaCHU As Integer 'contador de dados usados na média
        contamédiaCHU = 0 'zera variável acima

        'Para cada dia que vamos computar a média
        For dd As Integer = 1 To DiasFuturo
            contamédiaCHU = 0 'zeramos o contador do dia
            For hh As Integer = 1 To 24 'percorremos todas as horas
                contadorONS += 1 'adicionarmos um no contador do dia para cada hora
                If prevCHUONS(contadorONS) > 0 Then 'se temos dados, usamos ele na média
                    MédiaCHU(dd) += prevCHUONS(contadorONS) 'parte superior da média
                    contamédiaCHU += 1 'parte inferior da média
                End If
            Next 'loop das horas
            MédiaCHU(dd) = MédiaCHU(dd) 'calcula a média (somente acumulação)
            inteiroCHU(dd) = MédiaCHU(dd) 'computa ela como número inteiro
        Next 'loop dos dias

        ' MsgBox("Calculou as chuvas médias")


        'Lista de Variáveis que devem ser gravadas no arquivo word:
        Dim Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, Data10, Data11, Data12, Data13, Data14, Data15 As Date '@Data1, @Data2, @Data3, @Data4, @Data5: datas das previsões;
        Dim PH, P7H, QH As String '@PH: Previpitação total média na bacia até a hora da previsão, @QH: Vazão simulada na hora da previsão
        Dim Chuva01, Chuva02, Chuva03, Chuva04, Chuva05, Chuva06, Chuva07, Chuva08, Chuva09, Chuva10, Chuva11, Chuva12, Chuva13, Chuva14, Chuva15 As String '@Chuva01, @Chuva21, @Chuva03, @Chuva04, @Chuva05: chuvas previstas estimadas 
        Dim Vazao01, Vazao02, Vazao03, Vazao04, Vazao05, Vazao06, Vazao07, Vazao08, Vazao09, Vazao10, Vazao11, Vazao12, Vazao13, Vazao14, Vazao15 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Inc01, Inc02, Inc03, Inc04, Inc05, Inc06, Inc07, Inc08, Inc09, Inc10, Inc11, Inc12, Inc13, Inc14, Inc15 As String '@Vazao01, @Vazao21, @Vazao03, @Vazao04, @Vazao05: vazoes previstas estimadas 
        Dim Grafico As String '@Gráfico: início do gráfico

        'Preenche os dados que vão sair no boletim:

        Data1 = datahoje
        Data2 = datahoje.AddHours(+24)
        Data3 = datahoje.AddHours(+48)
        Data4 = datahoje.AddHours(+72)
        Data5 = datahoje.AddHours(+96)
        Data6 = datahoje.AddHours(+120)
        Data7 = datahoje.AddHours(+144)
        Data8 = datahoje.AddHours(+168)
        Data9 = datahoje.AddHours(+192)
        Data10 = datahoje.AddHours(+216)
        Data11 = datahoje.AddHours(+240)
        Data12 = datahoje.AddHours(+264)
        Data13 = datahoje.AddHours(+288)
        Data14 = datahoje.AddHours(+312)
        Data15 = datahoje.AddHours(+336)

        Dim soma As Integer
        soma = 0
        For i As Integer = 1 To 8
            If prevCHUONS(i) >= 0 Then
                soma += prevCHUONS(i)
            End If
        Next
        PH = soma

        soma = 0
        For i As Integer = 1 To 25
            If prevCHUONS7(i) >= 0 Then
                soma += prevCHUONS7(i)
            End If
        Next
        P7H = soma

        soma = prevONS(8)
        QH = soma

        Chuva01 = inteiroCHU(1)
        Chuva02 = inteiroCHU(2)
        Chuva03 = inteiroCHU(3)
        Chuva04 = inteiroCHU(4)
        Chuva05 = inteiroCHU(5)
        Chuva06 = inteiroCHU(6)
        Chuva07 = inteiroCHU(7)
        Chuva08 = inteiroCHU(8)
        Chuva09 = inteiroCHU(9)
        Chuva10 = inteiroCHU(10)
        Chuva11 = inteiroCHU(11)
        Chuva12 = inteiroCHU(12)
        Chuva13 = inteiroCHU(13)
        Chuva14 = inteiroCHU(14)
        Chuva15 = inteiroCHU(15)

        Vazao01 = inteiro(1)
        Vazao02 = inteiro(2)
        Vazao03 = inteiro(3)
        Vazao04 = inteiro(4)
        Vazao05 = inteiro(5)
        Vazao06 = inteiro(6)
        Vazao07 = inteiro(7)
        Vazao08 = inteiro(8)
        Vazao09 = inteiro(9)
        Vazao10 = inteiro(10)
        Vazao11 = inteiro(11)
        Vazao12 = inteiro(12)
        Vazao13 = inteiro(13)
        Vazao14 = inteiro(14)
        Vazao15 = inteiro(15)

        Inc01 = Incremental(1)
        Inc02 = Incremental(2)
        Inc03 = Incremental(3)
        Inc04 = Incremental(4)
        Inc05 = Incremental(5)
        Inc06 = Incremental(6)
        Inc07 = Incremental(7)
        Inc08 = Incremental(8)
        Inc09 = Incremental(9)
        Inc10 = Incremental(10)
        Inc11 = Incremental(11)
        Inc12 = Incremental(12)
        Inc13 = Incremental(13)
        Inc14 = Incremental(14)
        Inc15 = Incremental(15)

        Grafico = "Grafico"


        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())


        'Faz um arquivo txt
        FileOpen(2, diretóriodoprojeto & "\Prev\BoletimFCH_" & Now.Year & mes & dia & hora & ".txt", OpenMode.Output)

        PrintLine(2, "DATA PREVISÃO: " & Data1)
        PrintLine(2, "- PRECIPITAÇÃO TOTAL MÉDIA NA BACIA NAS ÚLTIMAS 8 HORAS: 	" & PH & " mm")
        PrintLine(2, "- PRECIPITAÇÃO TOTAL MÉDIA NA BACIA NAS ÚLTIMAS 24 HORAS: 	" & P7H & " mm")
        PrintLine(2, "- VAZÃO AFLUENTE NO LOCAL DA USINA 2: 	" & QH & " m³/s")
        PrintLine(2, Data1 & " " & Chuva01 & " " & Vazao01 & " " & Inc01)
        PrintLine(2, Data2 & " " & Chuva02 & " " & Vazao02 & " " & Inc02)
        PrintLine(2, Data3 & " " & Chuva03 & " " & Vazao03 & " " & Inc03)
        PrintLine(2, Data4 & " " & Chuva04 & " " & Vazao04 & " " & Inc04)
        PrintLine(2, Data5 & " " & Chuva05 & " " & Vazao05 & " " & Inc05)
        PrintLine(2, Data6 & " " & Chuva06 & " " & Vazao06 & " " & Inc06)
        PrintLine(2, Data7 & " " & Chuva07 & " " & Vazao07 & " " & Inc07)
        PrintLine(2, Data8 & " " & Chuva08 & " " & Vazao08 & " " & Inc08)
        PrintLine(2, Data9 & " " & Chuva09 & " " & Vazao09 & " " & Inc09)
        PrintLine(2, Data10 & " " & Chuva10 & " " & Vazao10 & " " & Inc10)
        PrintLine(2, Data11 & " " & Chuva11 & " " & Vazao11 & " " & Inc11)
        PrintLine(2, Data12 & " " & Chuva12 & " " & Vazao12 & " " & Inc12)
        PrintLine(2, Data13 & " " & Chuva13 & " " & Vazao13 & " " & Inc13)
        PrintLine(2, Data14 & " " & Chuva14 & " " & Vazao14 & " " & Inc14)
        PrintLine(2, Data15 & " " & Chuva15 & " " & Vazao15 & " " & Inc15)
        FileClose(2)


        'Faz um arquivo só com a informação de chuva nas últimas 24h
        FileOpen(3, diretóriodoprojeto & "\Prev\ChuvaAcum24hFCH_" & Now.Year & mes & dia & hora & ".txt", OpenMode.Output)
        PrintLine(3, "- PRECIPITAÇÃO TOTAL MÉDIA NA BACIA NAS ÚLTIMAS 24 HORAS: 	" & P7H & " mm")
        FileClose(3)


    End Sub

    Private Sub RadioButton1_CheckedChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles RadioButton1.CheckedChanged

    End Sub

    Sub CARREGA_INFO_CHUVA_PREVISTA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Input)

        Dim linha_agenda As String
        linha_agenda = LineInput(1)
        tbChuvaPrevista.Text = linha_agenda
        FileClose()

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Input)
        Dim contador As Integer
        contador = 0
        Do Until EOF(1)
            linha_agenda = LineInput(1)
            contador += 1
            If contador > 3 Then
                cbModeloMeteo.Items.Add(linha_agenda)
            End If
        Loop
        FileClose()


    End Sub

    Private Sub GroupBox2_Enter(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles GroupBox2.Enter

    End Sub

    Private Sub cbModeloMeteo_SelectedIndexChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cbModeloMeteo.SelectedIndexChanged
    End Sub

    Sub CONFIGURA_MODELO_METEO()
        tbChuvaPrevista.Text = cbModeloMeteo.Text

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo e escreve
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Output)
        PrintLine(1, cbModeloMeteo.Text)
        PrintLine(1)
        PrintLine(1, "Options:")
        PrintLine(1, "ONS_ETA")
        PrintLine(1, "ONS_GEFS")
        PrintLine(1, "ONS_ECMWF")
        PrintLine(1, "Zero")
        PrintLine(1, "Arquivo")
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
        tb_dir_prev_auto.Text = DirPrevAuto(cb_dir.Text)
        FileClose()

    End Sub

    Sub MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "diretorio_chuva_prevista.txt", OpenMode.Output)
        PrintLine(1, cb_dir.Text)
        tb_dir_prev_auto.Text = DirPrevAuto(cb_dir.Text)

        FileClose()

    End Sub

    Sub CARREGA_MODELOMETEO_CHUVA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Input)

        Dim linha_agenda As String
        linha_agenda = LineInput(1)
        cbModeloMeteo.Text = linha_agenda
        tbChuvaPrevista.Text = cbModeloMeteo.Text
        FileClose()

    End Sub




    Private Sub cb_dir_SelectedIndexChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_dir.SelectedIndexChanged
    End Sub

    Private Sub bt_dir_prev_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles bt_dir_prev.Click
        Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()

    End Sub

    Private Sub bt_chuv_prev_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles bt_chuv_prev.Click
        Me.CONFIGURA_MODELO_METEO()

    End Sub


    Sub EXPORTA_RESULTADOS(ByVal METODO As String)

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"


        Dim Diretorio_Previsão As String
        Diretorio_Previsão = Dir_EXPORTA & "\Previsão_" & METODO & "_" & Now.Year & "_" & Now.Month & "_" & Now.Day & "_" & Now.Hour & "_" & cbModeloMeteo.Text


        If My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = True Then
            IO.Directory.Delete(Diretorio_Previsão, True)
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        ElseIf My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = False Then
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        End If


        Try 'copia tudo da pasta c:\mgb



            Dim f2() As String = IO.Directory.GetFiles(diretóriodoprojeto & "\Output")
            For i As Integer = 0 To UBound(f2)
                IO.File.Copy(f2(i), Diretorio_Previsão & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next

            ' If
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta c:/mgb-hora/output para a pasta do projeto")
        End Try




    End Sub

    Private Sub Label30_Click(sender As Object, e As EventArgs)

    End Sub

    Private Sub Button4_Click(sender As Object, e As EventArgs) Handles Button4.Click
        TotalCommander = False
        Label11.ForeColor = Drawing.Color.Red 'Indica que o CPTEC está funcionando
        Label11.Text = "Desligado"
        Me.Close()
    End Sub

    Sub ESCREVE_MODELO_AUTOMATICO_MC(ByVal MC As Integer, ByVal Inercial As Integer)
        'Verifica base de dados interna
        Dim arquivo_est As String 'nome do arquivo com as configurações
        'determina o local do arquivo que contém os diretórios
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Modelos.txt"
        'abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Output)
        'faz a leitura do arquiv, linha por linha
        PrintLine(1, MC)
        PrintLine(1, Inercial)
        PrintLine(1)
        PrintLine(1, "Legenda")
        PrintLine(1, "Primeira linha, roda Fortran MC: 1-sim ou 0-não")
        PrintLine(1, "Segunda linha, roda Fortran IN: 1-sim ou 0-não")

        'Fecha
        FileClose()

    End Sub
End Class