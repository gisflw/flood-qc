Imports System.Windows.Forms
Imports System.Xml
Imports System.Net

'Atualizações para acelerar escrita
Imports System.Globalization
Imports System.IO
Imports System.Reflection
Imports System.Text

Public Class Form3
    Public sf As MapWinGIS.Shapefile
    Private diretorio_previsao As String 'diretório onde deve ser salvo o arquivo de vazões observadas
    Private diretorio_arquivos As String 'diretório onde estão salvos os arquivos novos
    Private diretorio_arquivos2 As String 'diretório onde estão salvos os arquivos novos
    Private diretorio_arquivos3 As String 'diretório onde estão salvos os arquivos novos
    Private Fonte_Dados_Automaticos As Integer 'fonte dos dados automáticos de chuva e vazão



    Private Sub Form3_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        Me.CARREGA_TABELA_ESTAÇÕES() 'carrega a tabela principal
        Me.ATUALIZA_DATA() 'atualiza as datas nos date time pickers
        Me.CARREGA_DIRETÓRIOS() 'carrega os diretórios onde o arquivo será salvo
        Me.Carrega_Hora_Previsão() 'Tenta carregar a hora final da previsão
        'Tenta atualizar os dados automaticamente
        Try
            'Me.ATUALIZA_AUTOMATICAMENTE() 'Faz duas vezes, procurando em diretórios diferentes
            'Me.ATUALIZA_AUTOMATICAMENTE2() 'Faz duas vezes, procurando em diretórios diferentes
            'Me.ATUALIZA_AUTOMATICAMENTE3() 'Faz duas vezes, procurando em diretórios diferentes




            'Me.ATUALIZA_BARRA_GRANDE(2)
            'Me.ATUALIZA_CAMPOS_NOVOS(2)
            'Me.ATUALIZA_GARIBALDI(2)

        Catch ex As Exception
            'MsgBox(ex.Message)
            ToolStripStatusLabel1.Text = "Não foi possível atualizar os dados automaticamente."
        End Try


        'Se o automático estiver ligado
        If TotalCommander = True Then

            Dim log_file As String
            log_file = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Log_operação_automatica.txt"
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Abriu a janela de vazão observada.")
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
                Me.CARREGAR_DADOS_Q_NOVO_DE_UMA_PASTA()
                Me.CARREGAR_DADOS_N_NOVO_DE_UMA_PASTA()
                FileOpen(99, log_file, OpenMode.Append)
                PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Atualizou dados de uma pasta.")
                FileClose(99)
            End If
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Exportando dados de vazão observada...")
            FileClose(99)
            Me.APERTA_BOTAO_EXPORTAR()
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Vazão observada exportada com sucesso. Próxima etapa: vazão das UHEs.")
            FileClose(99)
            Me.Close()
        End If



        ''Se o automático estiver ligado
        'If TotalCommander = True Then
        '    Me.ATUALIZAR_ONLINE()
        '    Me.APERTA_BOTAO_EXPORTAR()
        '    Me.Close()
        'End If


    End Sub


    Sub CARREGA_TABELA_ESTAÇÕES()
        Try
            'Verifica quantas estações existem na base de dados interna
            Dim arquivo_est As String 'nome do arquivo que contém as estações
            Dim linha As String 'linha de leitura
            total_estações = 0 'número de estações

            'define o arquivo
            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesNivel.txt"

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
            ReDim Q(total_estações, 24, 31, 12, Ano(AF))
            ReDim Niv(total_estações, 24, 31, 12, Ano(AF))
            ReDim dataIp(total_estações)
            ReDim dataFp(total_estações)
            ReDim HoraIp(total_estações)
            ReDim HoraFp(total_estações)
            ReDim AssimilaQ(total_estações)
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
                                Q(p, hh, dd, mm, Ano(aaaa)) = -1.0
                                Niv(p, hh, dd, mm, Ano(aaaa)) = -1.0
                            Next
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis


            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesNivel.txt"
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
                    AssimilaQ(est) = AjustarDecimal(splt(9))

                    '--desligado em 21 de out de 2025 para acelerar, em troca da de baixo Me.LE_DATABASE(est, Estação(est))
                    Me.LE_DATABASE_Fast_NQ(est, Estação(est))

                    'Adiciona os dados à tabela

                    Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

                    ' Preenche os valores do registro
                    nova_linha.ItemArray = New Object() {Estação(est), dataIp(est), dataFp(est), LON(est), LAT(est), NOME(est)}
                    Try
                        DataSet1.Tables(0).Rows.Add(nova_linha)
                    Catch ex As Exception
                        MsgBox(" erro: " & ex.Message)
                    End Try


                End If
            Loop



            FileClose()
            'Carregou dados internos


        Catch ex As Exception
            MsgBox(ex.Message)
        End Try






    End Sub

    Sub LE_DATABASE_Fast_NQ(ByVal numest As Integer, ByVal codest As String)
        Dim basePath = Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly().Location)
        Dim arqN As String = Path.Combine(basePath, $"database\N{codest}.txt")
        Dim arqQ As String = Path.Combine(basePath, $"database\Q{codest}.txt")

        ' Datas mín/max locais desta estação (evita tocar em globais a cada linha)
        Dim dataMin As Date = Date.MaxValue
        Dim dataMax As Date = Date.MinValue

        ' Reader com buffer grande (1 MB). Ajuste encoding se necessário.
        Using srN As New StreamReader(arqN, New UTF8Encoding(False), detectEncodingFromByteOrderMarks:=True, bufferSize:=1024 * 1024),
          srQ As New StreamReader(arqQ, New UTF8Encoding(False), detectEncodingFromByteOrderMarks:=True, bufferSize:=1024 * 1024)

            ' Pula cabeçalho (primeira linha) de ambos
            srN.ReadLine()
            srQ.ReadLine()

            While True
                Dim lnN As String = srN.ReadLine()
                Dim lnQ As String = srQ.ReadLine()
                If lnN Is Nothing OrElse lnQ Is Nothing Then Exit While

                ' Linhas inválidas/curtas
                If lnN.Length < 26 OrElse lnQ.Length < 26 Then Continue While

                ' -------- Parse dos campos (uma vez só) --------
                ' Dia/Mês/Hora: 2 dígitos à direita em suas colunas; Ano: 4 dígitos
                Dim dia As Integer = Integer.Parse(lnN.Substring(4, 2), CultureInfo.InvariantCulture)
                Dim mes As Integer = Integer.Parse(lnN.Substring(10, 2), CultureInfo.InvariantCulture)
                Dim ano_calend As Integer = Integer.Parse(lnN.Substring(14, 4), CultureInfo.InvariantCulture)
                Dim hora As Integer = Integer.Parse(lnN.Substring(22, 2), CultureInfo.InvariantCulture)

                ' Valor N (nível) e Q (vazão) começam na 5ª coluna (pos 24) e vão até o fim; tira espaços
                Dim sN As String = lnN.Substring(24).Trim()
                Dim sQ As String = lnQ.Substring(24).Trim()

                Dim vN As Double
                Dim vQ As Double
                If Not Double.TryParse(sN, NumberStyles.Float Or NumberStyles.AllowThousands, CultureInfo.InvariantCulture, vN) Then
                    vN = -1.0R
                End If
                If Not Double.TryParse(sQ, NumberStyles.Float Or NumberStyles.AllowThousands, CultureInfo.InvariantCulture, vQ) Then
                    vQ = -1.0R
                End If

                ' Índice do ano (calcula 1x)
                Dim idxAno As Integer = Ano(ano_calend)

                ' Armazena
                Niv(numest, hora, dia, mes, idxAno) = vN
                Q(numest, hora, dia, mes, idxAno) = vQ

                ' Data/hora para min/max
                Dim dt As New DateTime(ano_calend, mes, dia, hora, 0, 0, DateTimeKind.Unspecified)
                If dt < dataMin Then dataMin = dt
                If dt > dataMax Then dataMax = dt
            End While
        End Using

        ' Atualiza datas desta estação
        dataIp(numest) = dataMin
        dataFp(numest) = dataMax

        ' Atualiza globais
        If dataMin < dataI Then dataI = dataMin
        If dataMax > dataF Then dataF = dataMax
    End Sub

    Sub LE_DATABASE(ByVal numest As Integer, ByVal codest As String)
        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\N" & codest & ".txt"

        Dim arquivo_est3 As String
        arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & codest & ".txt"


        FileOpen(2, arquivo_est2, OpenMode.Input)
        FileOpen(3, arquivo_est3, OpenMode.Input)


        'Declara variáveis de leitura
        Dim contador As Integer
        contador = 0
        Dim linha2 As String
        Dim linha3 As String
        '--

        Do Until EOF(2)

            linha2 = LineInput(2)
            linha3 = LineInput(3)

            contador = contador + 1

            If contador > 1 Then
                Niv(numest, linha2.Substring(22, 2), linha2.Substring(4, 2), linha2.Substring(10, 2), Ano(linha2.Substring(14, 4))) = AjustarDecimal(linha2.Substring(26, 10))
                Q(numest, linha3.Substring(22, 2), linha3.Substring(4, 2), linha3.Substring(10, 2), Ano(linha3.Substring(14, 4))) = AjustarDecimal(linha3.Substring(26, 10))


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
        FileClose(3)


    End Sub

    Sub localiza_numero_estação(ByVal Cod_Estação As String)
        Dim CodigoLocalizado As Integer

        For p As Integer = 1 To total_estações
            If Cod_Estação = "N" & Estação(p) Then
                CodigoLocalizado = p
            End If
        Next




    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Me.ATUALIZAR_Q_OFFLINE_TRIPA()
    End Sub

    Sub ATUALIZAR_Q_OFFLINE_TRIPA()
        ofd.Multiselect = True
        ofd.Filter = "ASCII com dados de vazão horários dispostos em colunas(.txt)|*.txt"
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
                ' cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)).Substring(0, 8)
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p))

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    ' If cod_estação = Estação(po) Then
                    If cod_estação = "Q" & Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                    Dim linha4 As String
                    Dim h, qi As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Dim contador As Integer
                    contador = -1
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        contador += 1
                        If contador < 1 Then Continue Do

                        'MsgBox(linha4)
                        'MsgBox(linha4.Substring(26, 10))


                        qi = AjustarDecimal(linha4.Substring(26, 10))

                        If qi < 0 Then qi = -1.0

                        'MsgBox(linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00")

                        'Niv(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = -1.0
                        Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = qi

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
            'Me.ATUALIZA_ARQUIVOS_NIVEL() 'Desabilitada em 20/10/2025 por nova rotina mais rápida
            'Me.ATUALIZA_ARQUIVOS_VAZÃO() 'Desabilitada em 20/10/2025 por nova rotina mais rápida

            Me.ATUALIZA_ARQUIVOS_NIVEL_Legado() 'novas rotinas mais eficientes
            Me.ATUALIZA_ARQUIVOS_VAZÃO_Legado() 'novas rotinas mais eficientes


            Me.ATUALIZA_DATA()
            ToolStripStatusLabel1.Text = "Atualização finalizada"

        End If
    End Sub

    Sub ATUALIZAR_N_OFFLINE_TRIPA()
        ofd.Multiselect = True
        ofd.Filter = "ASCII com dados de vazão horários dispostos em colunas(.txt)|*.txt"
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
                ' cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)).Substring(0, 8)
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p))

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    ' If cod_estação = Estação(po) Then
                    If cod_estação = "N" & Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                    Dim linha4 As String
                    Dim h, qi As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Dim contador As Integer
                    contador = -1
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        contador += 1
                        If contador < 1 Then Continue Do

                        h = AjustarDecimal(linha4.Substring(26, 10))

                        Niv(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = h
                        'Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4)))

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


                        ' ''Try
                        ' ''    h = AjustarDecimal(linha4.Substring(31, 10))
                        ' ''Catch ex As Exception
                        ' ''    h = -1.0
                        ' ''End Try

                        ' ''Try
                        ' ''    qi = AjustarDecimal(linha4.Substring(42, 10))
                        ' ''Catch ex As Exception
                        ' ''    qi = -1.0
                        ' ''End Try
                        '' ''Lê o nível
                        ' ''Niv(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = h

                        '' ''h = h * 100   'na curva chave, os valores entram multiplicados por 100

                        ' ''If h > 0 Then
                        ' ''    'A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                        ' ''    If qi < 9999 And qi > 0 Then
                        ' ''        Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = qi
                        ' ''    ElseIf qi = 9999 Then
                        ' ''        Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                        ' ''    ElseIf qi <= 0 Then
                        ' ''        Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                        ' ''    End If
                        ' ''ElseIf h <= 0 Then
                        ' ''    Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                        ' ''End If


                        ' ''If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" < dataIp(CodigoLocalizado) Then
                        ' ''    dataIp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                        ' ''    If dataIp(CodigoLocalizado) < dataI Then
                        ' ''        dataI = dataIp(CodigoLocalizado)
                        ' ''    End If
                        ' ''End If

                        ' ''If linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00" > dataFp(CodigoLocalizado) Then
                        ' ''    dataFp(CodigoLocalizado) = linha4.Substring(0, 2) & "/" & linha4.Substring(3, 2) & "/" & linha4.Substring(6, 4) & " " & linha4.Substring(11, 2) & ":00:00"
                        ' ''    If dataFp(CodigoLocalizado) > dataF Then
                        ' ''        dataF = dataFp(CodigoLocalizado)
                        ' ''    End If
                        ' ''End If


                    Loop
                    FileClose(4)



                End If


                statusbar.Increment(1)
            Next

            Me.AtualizaDataTabela()
            ' Me.ATUALIZA_ARQUIVOS_NIVEL()
            ' Me.ATUALIZA_ARQUIVOS_VAZÃO()

            Me.ATUALIZA_ARQUIVOS_NIVEL_Legado() 'novas rotinas mais eficientes
            Me.ATUALIZA_ARQUIVOS_VAZÃO_Legado() 'novas rotinas mais eficientes

            Me.ATUALIZA_DATA()
            ToolStripStatusLabel1.Text = "Atualização finalizada"

        End If
    End Sub

    Sub AtualizaDataTabela()
        For p As Integer = 1 To total_estações
            DataGridView1.Item(1, p - 1).Value = dataIp(p)
            DataGridView1.Item(2, p - 1).Value = dataFp(p)
        Next
    End Sub

    Sub ATUALIZA_ARQUIVOS_NIVEL()
        For p As Integer = 1 To total_estações
            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
            'tmaxx = dif. de tempo

            Dim datacomeço As Date
            datacomeço = dataIp(p)
            'datacomeço.AddDays(1)

            'define o arquivo
            Dim arquivo_est2 As String
            arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\N" & Estação(p) & ".txt"

            'Dim arquivo_est3 As String
            'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

            FileOpen(2, arquivo_est2, OpenMode.Output)
            'FileOpen(3, arquivo_est3, OpenMode.Output)

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

                'If FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                '    Print(2, "       " & "-1.000000")
                'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                '    Print(2, "        " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                '    Print(2, "       " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                '    Print(2, "      " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                '    Print(2, "     " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
                'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                '    Print(2, "    " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                '    Print(2, "   " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                '    'não diminui dois espaços pois não tem o separador de milhar

                'End If

                'Controle de qualidade dos dados de nível antes de gravar
                If Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) > 99999 Then
                    Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = -1.0
                End If

                If SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                    Print(2, "       " & "-1.000000")
                ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                    Print(2, "        " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                    Print(2, "       " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                    Print(2, "      " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                    Print(2, "     " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                    Print(2, "    " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 Then
                    ' Print(2, "   " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
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

    '--- NOVAS ROTINAS ATUALIZACAO NIVEIS
    Private Function Alinha(valor As String, largura As Integer) As String
        Return valor.PadLeft(largura)
    End Function

    Private Function FormataNivel(valor As Double) As String
        ' Última coluna deve ter 16 caracteres
        Const LARGURA_NIVEL As Integer = 16

        If Double.IsNaN(valor) OrElse Double.IsInfinity(valor) Then
            Return Alinha("-1.000000", LARGURA_NIVEL)
        End If

        If valor > 99999 Then valor = -1.0
        If valor < 0 Then Return Alinha("-1.000000", LARGURA_NIVEL)

        Dim s = valor.ToString("0.000000", CultureInfo.InvariantCulture)
        Return Alinha(s, LARGURA_NIVEL)
    End Function

    ' ...reaproveite as funções Alinha e FormataNivel...

    Sub ATUALIZA_ARQUIVOS_NIVEL_Legado()
        Const L6 As Integer = 6
        Const L16 As Integer = 16

        For p As Integer = 1 To total_estações
            Dim tmaxx As Integer = CInt(DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
            Dim dt0 As Date = dataIp(p)
            Dim arquivo As String =
            System.IO.Path.GetDirectoryName(Reflection.Assembly.GetExecutingAssembly().Location()) &
            "\database\N" & Estação(p) & ".txt"

            FileOpen(2, arquivo, OpenMode.Output)

            Dim cabecalho As String =
            Alinha("Dia", L6) &
            Alinha("Mês", L6) &
            Alinha("Ano", L6) &
            Alinha("Hora", L6) &
            Alinha(NOME(p), L16)
            PrintLine(2, cabecalho)

            For i As Integer = 0 To tmaxx - 1
                Dim dt As Date = dt0.AddHours(i)

                Dim dia = Alinha(dt.Day.ToString("00"), L6)
                Dim mes = Alinha(dt.Month.ToString("00"), L6)
                Dim ano_calend = Alinha(dt.Year.ToString(), L6)
                Dim hora = Alinha(dt.Hour.ToString("00"), L6)

                Dim valor As Double = Niv(p, dt.Hour, dt.Day, dt.Month, Ano(dt.Year))
                Dim nivelFmt As String = FormataNivel(valor)

                PrintLine(2, dia & mes & ano_calend & hora & nivelFmt)
            Next

            FileClose(2)
        Next
    End Sub

    ' Reaproveite Alinha e FormataVazao

    Private Function FormataVazao(valor As Double) As String
        Const LARGURA As Integer = 16

        If Double.IsNaN(valor) OrElse Double.IsInfinity(valor) Then
            Return Alinha("-1.000000", LARGURA)
        End If

        If valor > 999999 Then valor = -1.0
        If valor < 0 Then Return Alinha("-1.000000", LARGURA)

        Dim s = valor.ToString("0.000000", CultureInfo.InvariantCulture)
        Return Alinha(s, LARGURA)
    End Function

    Sub ATUALIZA_ARQUIVOS_VAZÃO_Legado()
        Const L6 As Integer = 6
        Const L16 As Integer = 16

        For p As Integer = 1 To total_estações
            Dim tmaxx As Integer = CInt(DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
            Dim dt0 As Date = dataIp(p)
            Dim arquivo As String =
            System.IO.Path.GetDirectoryName(Reflection.Assembly.GetExecutingAssembly().Location()) &
            "\database\Q" & Estação(p) & ".txt"

            FileOpen(2, arquivo, OpenMode.Output)

            Dim cabecalho As String =
            Alinha("Dia", L6) &
            Alinha("Mês", L6) &
            Alinha("Ano", L6) &
            Alinha("Hora", L6) &
            Alinha(NOME(p), L16)
            PrintLine(2, cabecalho)

            For i As Integer = 0 To tmaxx - 1
                Dim dt As Date = dt0.AddHours(i)

                Dim dia = Alinha(dt.Day.ToString("00"), L6)
                Dim mes = Alinha(dt.Month.ToString("00"), L6)
                Dim ano_calend = Alinha(dt.Year.ToString(), L6)
                Dim hora = Alinha(dt.Hour.ToString("00"), L6)

                Dim q_vaz As Double = Q(p, dt.Hour, dt.Day, dt.Month, Ano(dt.Year))
                Dim qFmt As String = FormataVazao(q_vaz)

                PrintLine(2, dia & mes & ano_calend & hora & qFmt)
            Next

            FileClose(2)
        Next
    End Sub





    '---


    Sub ATUALIZA_ARQUIVOS_VAZÃO()
        For p As Integer = 1 To total_estações
            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
            'tmaxx = dif. de tempo

            Dim datacomeço As Date
            datacomeço = dataIp(p)
            'datacomeço.AddDays(1)

            'define o arquivo
            Dim arquivo_est2 As String
            arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

            'Dim arquivo_est3 As String
            'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

            FileOpen(2, arquivo_est2, OpenMode.Output)
            'FileOpen(3, arquivo_est3, OpenMode.Output)

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

                'If FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                '    Print(2, "       " & "-1.000000")
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                '    Print(2, "        " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                '    Print(2, "       " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                '    Print(2, "      " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                '    Print(2, "     " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                '    Print(2, "    " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                '    Print(2, "   " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                '    'não diminui dois espaços pois não tem o separador de milhar

                'End If

                'Controle de qualidade dos dados de Vazão antes de gravar
                If Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) > 999999 Then
                    Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = -1.0
                End If

                If SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                    Print(2, "       " & "-1.000000")
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                    Print(2, "        " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                    Print(2, "       " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                    Print(2, "      " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                    Print(2, "     " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                    Print(2, "    " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 Then
                    'Print(2, "   " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
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

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        Resultado = False
        VisuResultado = False
        ChuvaSUBS = False
        Padrão = False

        Hidrograma = True
        Cotagrama = False
        Pluviograma = False
        Estação_GrafQ = DataGridView1.CurrentCell.RowIndex + 1
        Dim frmgraf As New Form4
        frmgraf.Show()

    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        Hidrograma = False
        Cotagrama = True
        Pluviograma = False
        Padrão = False

        Estação_GrafQ = DataGridView1.CurrentCell.RowIndex + 1
        Dim frmgraf As New Form4
        frmgraf.Show()
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
        If sfd.ShowDialog() = DialogResult.OK Then

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
        ReDim Qdispo(total_estações, 24, 31, 12, Ano(AF))

        'Zera as variáveis de vazão e nível
        For p As Integer = 0 To total_estações
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Qdispo(p, hh, dd, mm, Ano(aaaa)) = Q(p, hh, dd, mm, Ano(aaaa))
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
    End Sub

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        Me.APERTA_BOTAO_EXPORTAR()
        MsgBox("Dados exportados com sucesso!", MsgBoxStyle.Information, "Previsão")
        Me.Close()
    End Sub

    Sub APERTA_BOTAO_EXPORTAR()
        Me.ATUALIZA_DADOS_EDITADOS() 'atualiza os dados, caso eles tenham sido editados manualmente
        Me.VERIFICA_QUEM_ASSIMILA()
        ToolStripStatusLabel1.Text = "Exportando dados fluviométricos"
        ' ''Me.MEDIA_MOVEL_BARRA_GRANDE()
        ' ''Me.MEDIA_MOVEL_CAMPOS_NOVOS()
        ' ''Me.MEDIA_MOVEL_GARIBALDI()
        Me.SALVA_QOBS_AUTOMÁTICO()
        'Me.SALVA_QOBS_MANUALMENTE() 'desativado em troca do automático
        'Apaga os dados da memória
        Q = Nothing
        Niv = Nothing
        Erase Q
        Erase Niv
    End Sub

    Sub MEDIA_MOVEL_BARRA_GRANDE()
        '---Média Móvel de Barra Grande
        'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
        Dim CodigoEstacao As Integer
        'Pega na tabela qual posto estamos utilizando
        'O +1 no final é utilizado pois a contagem da tabela começa em zero
        CodigoEstacao = 15
        Dim datainicial As Date
        datainicial = dataIp(CodigoEstacao)
        datainicial = datainicial.AddHours(-1)
        'MsgBox(datainicial)
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataIp(CodigoEstacao), dataFp(CodigoEstacao)) + 1)
        'tmaxx = dif. de tempo
        Dim TamSerie As Integer
        TamSerie = tmaxx
        Dim mdiamovel(TamSerie) As Single
        'MsgBox(TamSerie)


        'Calcula a média móvel olhando quatro dias para trás
        For med As Integer = 1 To TamSerie
            datainicial = datainicial.AddHours(+1)
            'MsgBox(datainicial)
            mdiamovel(med) = Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year))
            If med > 4 Then
                mdiamovel(med) = 0
                Dim cntmovel As Integer
                cntmovel = 0
                Dim datamovel As Date
                datamovel = datainicial.AddHours(+1)
                For Movel As Integer = 1 To 4
                    datamovel = datamovel.AddHours(-1)
                    If Q(CodigoEstacao, datamovel.Hour, datamovel.Day, datamovel.Month, Ano(datamovel.Year)) <> -1 Then
                        mdiamovel(med) += Q(CodigoEstacao, datamovel.Hour, datamovel.Day, datamovel.Month, Ano(datamovel.Year))
                        cntmovel += 1
                    End If
                Next
                If cntmovel > 0 Then
                    mdiamovel(med) = mdiamovel(med) / cntmovel
                ElseIf cntmovel <= 0 Then
                    mdiamovel(med) = Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year))
                End If

            End If
        Next

        'Atribui a média móvel ao nosso sistema
        datainicial = dataIp(CodigoEstacao)
        datainicial = datainicial.AddHours(-1)
        For med As Integer = 1 To TamSerie
            datainicial = datainicial.AddHours(+1)
            Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year)) = mdiamovel(med)
        Next

    End Sub

    Sub MEDIA_MOVEL_CAMPOS_NOVOS()
        '---Média Móvel de Barra Grande
        'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
        Dim CodigoEstacao As Integer
        'Pega na tabela qual posto estamos utilizando
        'O +1 no final é utilizado pois a contagem da tabela começa em zero
        CodigoEstacao = 16
        Dim datainicial As Date
        datainicial = dataIp(CodigoEstacao)
        datainicial = datainicial.AddHours(-1)
        'MsgBox(datainicial)
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataIp(CodigoEstacao), dataFp(CodigoEstacao)) + 1)
        'tmaxx = dif. de tempo
        Dim TamSerie As Integer
        TamSerie = tmaxx
        Dim mdiamovel(TamSerie) As Single
        'MsgBox(TamSerie)


        'Calcula a média móvel olhando quatro dias para trás
        For med As Integer = 1 To TamSerie
            datainicial = datainicial.AddHours(+1)
            'MsgBox(datainicial)
            mdiamovel(med) = Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year))
            If med > 4 Then
                mdiamovel(med) = 0
                Dim cntmovel As Integer
                cntmovel = 0
                Dim datamovel As Date
                datamovel = datainicial.AddHours(+1)
                For Movel As Integer = 1 To 4
                    datamovel = datamovel.AddHours(-1)
                    If Q(CodigoEstacao, datamovel.Hour, datamovel.Day, datamovel.Month, Ano(datamovel.Year)) <> -1 Then
                        mdiamovel(med) += Q(CodigoEstacao, datamovel.Hour, datamovel.Day, datamovel.Month, Ano(datamovel.Year))
                        cntmovel += 1
                    End If
                Next
                If cntmovel > 0 Then
                    mdiamovel(med) = mdiamovel(med) / cntmovel
                ElseIf cntmovel <= 0 Then
                    mdiamovel(med) = Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year))
                End If

            End If
        Next

        'Atribui a média móvel ao nosso sistema
        datainicial = dataIp(CodigoEstacao)
        datainicial = datainicial.AddHours(-1)
        For med As Integer = 1 To TamSerie
            datainicial = datainicial.AddHours(+1)
            Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year)) = mdiamovel(med)
        Next

    End Sub

    Sub MEDIA_MOVEL_GARIBALDI()
        '---Média Móvel de Barra Grande
        'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
        Dim CodigoEstacao As Integer
        'Pega na tabela qual posto estamos utilizando
        'O +1 no final é utilizado pois a contagem da tabela começa em zero
        CodigoEstacao = 17
        Dim datainicial As Date
        datainicial = dataIp(CodigoEstacao)
        datainicial = datainicial.AddHours(-1)
        'MsgBox(datainicial)
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataIp(CodigoEstacao), dataFp(CodigoEstacao)) + 1)
        'tmaxx = dif. de tempo
        Dim TamSerie As Integer
        TamSerie = tmaxx
        Dim mdiamovel(TamSerie) As Single
        'MsgBox(TamSerie)


        'Calcula a média móvel olhando quatro dias para trás
        For med As Integer = 1 To TamSerie
            datainicial = datainicial.AddHours(+1)
            'MsgBox(datainicial)
            mdiamovel(med) = Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year))
            If med > 6 Then
                mdiamovel(med) = 0
                Dim cntmovel As Integer
                cntmovel = 0
                Dim datamovel As Date
                datamovel = datainicial.AddHours(+1)
                For Movel As Integer = 1 To 6
                    datamovel = datamovel.AddHours(-1)
                    If Q(CodigoEstacao, datamovel.Hour, datamovel.Day, datamovel.Month, Ano(datamovel.Year)) <> -1 Then
                        mdiamovel(med) += Q(CodigoEstacao, datamovel.Hour, datamovel.Day, datamovel.Month, Ano(datamovel.Year))
                        cntmovel += 1
                    End If
                Next
                If cntmovel > 0 Then
                    mdiamovel(med) = mdiamovel(med) / cntmovel
                ElseIf cntmovel <= 0 Then
                    mdiamovel(med) = Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year))
                End If

            End If
        Next

        'Atribui a média móvel ao nosso sistema
        datainicial = dataIp(CodigoEstacao)
        datainicial = datainicial.AddHours(-1)
        For med As Integer = 1 To TamSerie
            datainicial = datainicial.AddHours(+1)
            Q(CodigoEstacao, datainicial.Hour, datainicial.Day, datainicial.Month, Ano(datainicial.Year)) = mdiamovel(med)
        Next

    End Sub

    Sub VERIFICA_QUEM_ASSIMILA()
        'Esta rotina verifica se algum posto não vai ser usado na interpolação e zera os calores dele

        'Zera as variáveis dque não serão usadas
        For p As Integer = 0 To total_estações

            If AssimilaQ(p) <= 0 Then
                For aaaa = AI To AF
                    For mm = 1 To 12
                        For dd = 1 To 31
                            For hh As Integer = 0 To 23
                                Q(p, hh, dd, mm, Ano(aaaa)) = -1.0
                            Next
                        Next
                    Next
                Next
            End If

        Next


    End Sub

    Sub ESCREVEDADOVAZÃO_HORA()

        For p As Integer = 1 To total_estações
            'Calcula datas
            Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"


            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
            'tmaxx = dif. de tempo

            Dim datacomeço As Date
            datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            'datacomeço.AddDays(1)

            'define o arquivo
            Dim arquivo_est2 As String
            arquivo_est2 = fbd.SelectedPath & "\" & NOME(p) & ".txt"

            'Dim arquivo_est3 As String
            'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

            FileOpen(2, arquivo_est2, OpenMode.Output)
            'FileOpen(3, arquivo_est3, OpenMode.Output)

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

                'If FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                '    Print(2, "       " & "-1.000000")
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                '    Print(2, "        " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                '    Print(2, "       " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                '    Print(2, "      " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                '    Print(2, "     " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                '    Print(2, "    " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                '    Print(2, "   " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                '    'não diminui dois espaços pois não tem o separador de milhar

                'End If


                'Agora vai escrever os números
                'SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))
                If SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                    Print(2, "       " & "-1.000000")
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                    Print(2, "        " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                    Print(2, "       " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                    Print(2, "      " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                    Print(2, "     " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                    Print(2, "    " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                    Print(2, "   " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    'não diminui dois espaços pois não tem o separador de milhar

                End If
                PrintLine(2)

            Next

            FileClose()





        Next

    End Sub


    Sub TRANSCREVEQOBS_HORA2()
        'Calcula datas
        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)


        'tmaxx = dif. de tempo
        Dim datacomeço As Date
        datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = fbd.SelectedPath & "\QOBS.MGB"

        'Re-cria os arquivos com os mesmos limites
        FileOpen(2, arquivo_est2, OpenMode.Output)

        Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho
        For cabeçalho As Integer = 1 To total_estações 'para cada um dos arquivos de vazão selecionados
            'Verifica se é vazão observada
            'escreve o arquivo no cabeçalho
            For espaço As Integer = 1 To 12 - NOME(cabeçalho).ToString.Length
                'primeiro esquece os espaços
                Print(2, " ")

            Next
            'escreve o nome do arquivo
            Print(2, NOME(cabeçalho))
        Next
        PrintLine(2) 'finaliza a linha do cabeçalho


        For aux = 1 To (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
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
            For variável As Integer = 1 To total_estações

                'If FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 0 Then
                '    Print(2, "        " & "-1.0")
                'ElseIf FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 0 And FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 10 Then
                '    Print(2, "        " & "0" & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1))
                'ElseIf FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 10 And FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 100 Then
                '    Print(2, "        " & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1))
                'ElseIf FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 100 And FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 1000 Then
                '    Print(2, "       " & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1))
                'ElseIf FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 1000 And FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 10000 Then
                '    Print(2, "      " & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(0, 1) & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(2, 5))
                'ElseIf FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 10000 And FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 100000 Then
                '    Print(2, "     " & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(0, 2) & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(3, 5))
                'ElseIf FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 100000 And FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 1000000 Then
                '    Print(2, "    " & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(0, 3) & FormatNumber(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(4, 5))
                '    'não diminui dois espaços pois não tem o separador de milhar
                'End If


                Dim separador As String = Globalization.CultureInfo.CurrentCulture.NumberFormat.NumberDecimalSeparator
                If separador = "." Then
                    'SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))
                    If SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                        Print(2, "        " & "-1.0")
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                        Print(2, "        " & "0" & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                        Print(2, "        " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                        Print(2, "       " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                        Print(2, "      " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                        Print(2, "     " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                        Print(2, "    " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                        'não diminui dois espaços pois não tem o separador de milhar
                    End If
                Else
                    'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                    If SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                        Print(2, "        " & "-1.0")
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                        Print(2, "         " & "0" & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                        Print(2, "         " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                        Print(2, "        " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                        Print(2, "       " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                        Print(2, "      " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                    ElseIf SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                        Print(2, "     " & SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                        'não diminui dois espaços pois não tem o separador de milhar
                    End If
                End If




            Next
            PrintLine(2)

        Next

        FileClose()


    End Sub

    Sub TRANSCREVENIVOBS_HORA2()
        'Calcula datas
        Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)


        'tmaxx = dif. de tempo
        Dim datacomeço As Date
        datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = fbd.SelectedPath & "\NivOBS.MGB"

        'Re-cria os arquivos com os mesmos limites
        FileOpen(2, arquivo_est2, OpenMode.Output)

        Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho
        For cabeçalho As Integer = 1 To total_estações 'para cada um dos arquivos de vazão selecionados
            'Verifica se é vazão observada
            'escreve o arquivo no cabeçalho
            For espaço As Integer = 1 To 12 - NOME(cabeçalho).ToString.Length
                'primeiro esquece os espaços
                Print(2, " ")

            Next
            'escreve o nome do arquivo
            Print(2, NOME(cabeçalho))
        Next
        PrintLine(2) 'finaliza a linha do cabeçalho


        For aux = 1 To (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
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

            Dim valor As Single


            'Agora vai escrever os números
            For variável As Integer = 1 To total_estações

                valor = Niv(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))

                Dim separador As String = Globalization.CultureInfo.CurrentCulture.NumberFormat.NumberDecimalSeparator
                If separador = "." Then
                    'SubstituirSeparador1Dig(Q(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))
                    If SubstituirSeparador1Dig(valor) < 0 Then
                        Print(2, "        " & "-1.0")
                    ElseIf SubstituirSeparador1Dig(valor) >= 0 And SubstituirSeparador1Dig(valor) < 10 Then
                        Print(2, "        " & "0" & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 10 And SubstituirSeparador1Dig(valor) < 100 Then
                        Print(2, "        " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 100 And SubstituirSeparador1Dig(valor) < 1000 Then
                        Print(2, "       " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 1000 And SubstituirSeparador1Dig(valor) < 10000 Then
                        Print(2, "      " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 10000 And SubstituirSeparador1Dig(valor) < 100000 Then
                        Print(2, "     " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 100000 And SubstituirSeparador1Dig(valor) < 1000000 Then
                        Print(2, "    " & SubstituirSeparador1Dig(valor))
                        'não diminui dois espaços pois não tem o separador de milhar
                    End If
                Else
                    'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                    If SubstituirSeparador1Dig(valor) < 0 Then
                        Print(2, "        " & "-1.0")
                    ElseIf SubstituirSeparador1Dig(valor) >= 0 And SubstituirSeparador1Dig(valor) < 10 Then
                        Print(2, "         " & "0" & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 10 And SubstituirSeparador1Dig(valor) < 100 Then
                        Print(2, "         " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 100 And SubstituirSeparador1Dig(valor) < 1000 Then
                        Print(2, "        " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 1000 And SubstituirSeparador1Dig(valor) < 10000 Then
                        Print(2, "       " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 10000 And SubstituirSeparador1Dig(valor) < 100000 Then
                        Print(2, "      " & SubstituirSeparador1Dig(valor))
                    ElseIf SubstituirSeparador1Dig(valor) >= 100000 And SubstituirSeparador1Dig(valor) < 1000000 Then
                        Print(2, "     " & SubstituirSeparador1Dig(valor))
                        'não diminui dois espaços pois não tem o separador de milhar
                    End If
                End If




            Next
            PrintLine(2)

        Next

        FileClose()


    End Sub

    Sub ATUALIZA_DATA()

        dtpqob1.Value = dataIp(1)
        'dtpqob2.Value = dataFp(1)
        tb_hora_ini.Text = tb_hora_ini.Items(dataIp(1).Hour)
        'tb_hora_fim.Text = tb_hora_fim.Items(dataFp(1).Hour)

        Dim datahoje As Date
        datahoje = Now.Date
        If dtpqob1.Value < "01/07/" & datahoje.AddYears(-1).Year Then
            dtpqob1.Value = "01/07/" & datahoje.AddYears(-1).Year
            tb_hora_ini.Text = "00"
        End If


    End Sub

    Private Sub GroupBox1_Enter(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles GroupBox1.Enter

    End Sub

    Private Sub Button7_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button7.Click
        Dim arquivo_est2 As String
        Dim num As Integer
        num = DataGridView1.CurrentCell.RowIndex + 1
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(num) & ".txt"
        ' System.Diagnostics.Process.Start("notepad.exe", arquivo_est2)

        ReDim Tabela(24, 31, 12, Ano(AF))

        Tb_chuva = False
        Tb_vazao = True
        Nome_arq = arquivo_est2
        Nome_p = NOME(num)

        For aaaa = AI To AF
            For mm = 1 To 12
                For dd = 1 To 31
                    For hh As Integer = 0 To 23
                        Tabela(hh, dd, mm, Ano(aaaa)) = Q(num, hh, dd, mm, Ano(aaaa))
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

    Sub CARREGA_DIRETÓRIOS()
        'Verifica base de dados interna
        Dim arquivo_est As String 'no do arquivo com as configurações
        Dim total_e As Integer 'variável que faz a contagem das linhas
        Dim linha As String 'variável string que contém a linha
        total_e = 0 'zera o contador de estações
        'determina o local do arquivo que contém os diretórios
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\config.txt"
        'abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Input)
        'faz a leitura do arquiv, linha por linha
        Do Until EOF(1)
            linha = LineInput(1)
            total_e = total_e + 1
            'Quando chegar na linha 7, faz a leitura do diretório onde será salvo o arquivo da previsão
            If total_e = 1 Then
                diretorio_arquivos = linha
            ElseIf total_e = 4 Then
                diretorio_previsao = linha
            ElseIf total_e = 9 Then
                diretorio_arquivos2 = linha
            ElseIf total_e = 10 Then
                diretorio_arquivos3 = linha
            End If
        Loop

        Stlb.Text = "Diretório arquivos de previsão: " & diretorio_previsao
        'Fecha
        FileClose()

    End Sub

    Sub SALVA_QOBS_MANUALMENTE() 'salva o QOBS em local determinado manualmente pelo usuário
        'Esta rotina serve para salvar os dados do QOBS em um diretório indicado pelo usuário
        'como tudo está sendo salvo no diretório determinado automaticamente no formulário de preferências
        'esta rotina não está sendo usada

        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            statusbar.Value = 0
            statusbar.Maximum = 2

            Me.ESCREVEDADOVAZÃO_HORA()

            statusbar.Increment(1)

            Me.TRANSCREVEQOBS_HORA2()
            statusbar.Increment(1)

            MsgBox("Dados exportados com sucesso!", MsgBoxStyle.Information, "Previsão")

        End If

    End Sub

    Sub SALVA_QOBS_AUTOMÁTICO()
        fbd.SelectedPath = diretorio_previsao
        statusbar.Value = 0
        statusbar.Maximum = 2

        Me.ESCREVEDADOVAZÃO_HORA()

        statusbar.Increment(1)

        Me.TRANSCREVEQOBS_HORA2()
        statusbar.Increment(1)

        Me.TRANSCREVENIVOBS_HORA2()
        statusbar.Increment(1)

    End Sub

    Sub Carrega_Hora_Previsão() 'tenta carregar a hora final da previsão
        Try

            ofd.FileName = diretorio_previsao & "\PrevisaoMINI_Estreito.bin"
            'Declara as variáveis

            Dim diretório As String
            diretório = System.IO.Path.GetDirectoryName(ofd.FileName)

            Dim arquivo_metadados As String
            'arquivo padrão com nome dado pelo Siqueira:
            arquivo_metadados = diretório & "\Previsao.meta"

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

            dtpqob2.Value = datafimpprev

            Dim hora As String
            hora = datafimpprev.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If
            tb_hora_fim.Text = hora

            FileClose()

        Catch ex As Exception

        End Try

    End Sub

    Sub ATUALIZA_DADOS_EDITADOS()
        Try
            'Verifica quantas estações existem na base de dados interna
            Dim arquivo_est As String 'nome do arquivo que contém as estações
            Dim linha As String 'linha de leitura
            total_estações = 0 'número de estações

            'define o arquivo
            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesNivel.txt"

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
            ' ReDim ID(total_estações)
            ' ReDim Estação(total_estações)
            ' ReDim LAT(total_estações), LON(total_estações), ALT(total_estações)
            ' ReDim NOME(total_estações)
            ' ReDim A(total_estações), B(total_estações), C(total_estações)
            ' ReDim Q(total_estações, 24, 31, 12, Ano(AF))
            ' ReDim Niv(total_estações, 24, 31, 12, Ano(AF))
            ' ReDim dataIp(total_estações)
            ' ReDim dataFp(total_estações)
            ' ReDim HoraIp(total_estações)
            ' ReDim HoraFp(total_estações)
            '--

            'Zera as variáveis de data
            'For p As Integer = 0 To total_estações
            'dataIp(p) = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
            'dataFp(p) = "01/01/" & AI & " " & "00:00:00"
            'HoraIp(p) = 23
            'HoraFp(p) = 0
            'Next
            '--

            'Zera as variáveis de vazão e nível
            For p As Integer = 0 To total_estações
                For aaaa = AI To AF
                    For mm = 1 To 12
                        For dd = 1 To 31
                            For hh As Integer = 0 To 23
                                'Q(p, hh, dd, mm, Ano(aaaa)) = -1.0
                                'Niv(p, hh, dd, mm, Ano(aaaa)) = -1.0
                            Next
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis


            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesNivel.txt"
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


                    '--acelerado em 2025 outubro 21 Me.LE_DATABASE(est, Estação(est))

                    Me.LE_DATABASE_Fast_NQ(est, Estação(est))

                    'Adiciona os dados à tabela

                    'Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

                    ' Preenche os valores do registro
                    'nova_linha.ItemArray = New Object() {Estação(est), dataIp(est), dataFp(est), LON(est), LAT(est), NOME(est)}
                    'Try
                    'DataSet1.Tables(0).Rows.Add(nova_linha)
                    'Catch ex As Exception
                    'MsgBox(" erro: " & ex.Message)
                    'End Try


                End If
            Loop



            FileClose()
            'Carregou dados internos


        Catch ex As Exception
            MsgBox(ex.Message)
        End Try




    End Sub

    Private Sub Button8_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button8.Click
        Me.ATUALIZA_DADOS_EDITADOS()
        Me.AtualizaDataTabela()
    End Sub

    Sub BOTÃO1ANTIGO()
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
                    If cod_estação = "N" & Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de nível mesmo
                    Dim linha4 As String
                    Dim h As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        'Lê o nível
                        Niv(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = linha4.Substring(26, 14)
                        h = linha4.Substring(26, 14)
                        h = h * 100   'na curva chave, os valores entram multiplicados por 100

                        If h > 0 Then
                            'A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                            Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = A(CodigoLocalizado) * (h ^ 2) + B(CodigoLocalizado) * h + C(CodigoLocalizado)
                        ElseIf h <= 0 Then
                            Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = -1.0
                        End If


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
            Me.ATUALIZA_ARQUIVOS_NIVEL()
            Me.ATUALIZA_ARQUIVOS_VAZÃO()
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
                    Dim h, qi As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        Try
                            h = AjustarDecimal(linha4.Substring(31, 10))
                        Catch ex As Exception
                            h = -1.0
                        End Try

                        Try
                            qi = AjustarDecimal(linha4.Substring(42, 10))
                        Catch ex As Exception
                            qi = -1.0
                        End Try
                        'Lê o nível
                        Niv(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = h



                        'h = h * 100   'na curva chave, os valores entram multiplicados por 100

                        If h > 0 Then
                            'A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                            If qi < 9999 And qi > 0 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = qi
                            ElseIf qi = 9999 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                            ElseIf qi <= 0 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                            End If
                        ElseIf h <= 0 Then
                            Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
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
            Catch ex As Exception

            End Try



            statusbar.Increment(1)


        Loop
        FileClose(15)
        '-- Fim da verificação do total de estações

        Me.AtualizaDataTabela()
        Me.ATUALIZA_ARQUIVOS_NIVEL()
        Me.ATUALIZA_ARQUIVOS_VAZÃO()
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
                    Dim h, qi As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        Try
                            h = AjustarDecimal(linha4.Substring(31, 10))
                        Catch ex As Exception
                            h = -1.0
                        End Try

                        Try
                            qi = AjustarDecimal(linha4.Substring(42, 10))
                        Catch ex As Exception
                            qi = -1.0
                        End Try
                        'Lê o nível
                        Niv(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = h



                        'h = h * 100   'na curva chave, os valores entram multiplicados por 100

                        If h > 0 Then
                            'A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                            If qi < 9999 And qi > 0 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = qi
                            ElseIf qi = 9999 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                            ElseIf qi <= 0 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                            End If
                        ElseIf h <= 0 Then
                            Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
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
            Catch ex As Exception

            End Try



            statusbar.Increment(1)


        Loop
        FileClose(15)
        '-- Fim da verificação do total de estações

        Me.AtualizaDataTabela()
        Me.ATUALIZA_ARQUIVOS_NIVEL()
        Me.ATUALIZA_ARQUIVOS_VAZÃO()
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
                    Dim h, qi As Double
                    FileOpen(4, Arqu(p), OpenMode.Input)
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        Try
                            h = AjustarDecimal(linha4.Substring(31, 10))
                        Catch ex As Exception
                            h = -1.0
                        End Try

                        Try
                            qi = AjustarDecimal(linha4.Substring(42, 10))
                        Catch ex As Exception
                            qi = -1.0
                        End Try
                        'Lê o nível
                        Niv(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = h



                        'h = h * 100   'na curva chave, os valores entram multiplicados por 100

                        If h > 0 Then
                            'A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                            If qi < 9999 And qi > 0 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = qi
                            ElseIf qi = 9999 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                            ElseIf qi <= 0 Then
                                Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
                            End If
                        ElseIf h <= 0 Then
                            Q(CodigoLocalizado, linha4.Substring(11, 2), linha4.Substring(0, 2), linha4.Substring(3, 2), Ano(linha4.Substring(6, 4))) = -1.0
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
            Catch ex As Exception

            End Try



            statusbar.Increment(1)


        Loop
        FileClose(15)
        '-- Fim da verificação do total de estações

        Me.AtualizaDataTabela()
        Me.ATUALIZA_ARQUIVOS_NIVEL()
        Me.ATUALIZA_ARQUIVOS_VAZÃO()
        Me.ATUALIZA_DATA()
        ToolStripStatusLabel1.Text = "Dados atualizados automaticamente em " & Now.Date
        statusbar.Value = 0




    End Sub

    Sub ATUALIZA_BARRA_GRANDE(ByVal Codigo As Integer)
        'Rotina de atualizaçãp de dados das estações fluviométricas
        'Ela permite a atualização de cada posto, individualmente, muito embora ela possibilite que multiplos arquivos sejam inseridos
        'É a única rotina que deve ser atualizada em caso de mudança nos postos, pois tem coisas que devem ser modificadas no código

        ''Libera a seleção de multiplos arquivos
        'ofd.Multiselect = True
        ''coloca o filtro dos arquivos de vazão
        'ofd.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"
        ''abre a janela de carregar arquivos, e só continua se tudo estiver OK
        'If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then

        ofd.FileName = diretorio_arquivos & "\" & "UHEBarraGrande.txt"

        'Cria a variável que corresponde aos strings selecionados na caixa de diálogo
        Dim s As String
        'Cria a variável que guarda o número de arquivos carregados na caixa de diálogo
        Dim pmax As Integer
        'Preenche o número de arquivos
        pmax = ofd.FileNames.Length
        'Cria a variável que contém o nome dos arquivos individualmente, já na dimensão necessária
        Dim Arqu(pmax) As String
        'Cria a varíavel p, que contém a informação de qual posto da lista de postos nós estamos utilizando
        Dim p As Integer
        'Inicialmente p = 0, pois não começamos a ler os postos ainda
        p = 0

        'Agora, rodamos este loop, em que "s" assume o valor de cada um dos arquivos carregados
        For Each s In ofd.FileNames
            'somamos 1 à variável p, indicando qual arquivo estamos lendo
            p = p + 1
            'o nome do arquivo é guardado na variável Arqu(p)
            Arqu(p) = s
            'MsgBox(Arqu(p))

            'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
            Dim CodigoLocalizado As Integer
            'Pega na tabela qual posto estamos utilizando
            'O +1 no final é utilizado pois a contagem da tabela começa em zero
            CodigoLocalizado = 15 'DataGridView1.RowCount - 1 'Codigo 'DataGridView1.CurrentCell.RowIndex + 1

            'Caso o código for maior que zero, executa a rotina de ler o arquivo 
            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha4 As String
                'Abre o arquivo
                FileOpen(4, Arqu(p), OpenMode.Input)
                'Começa a leitura até o final, linha por linha
                Do Until EOF(4)
                    'linha4 contém a informação da linha
                    linha4 = LineInput(4)

                    'Cria estas variáveis, que contém as informações de hora, dia, mes ano e vazão que estamos lendo
                    Dim horae As Integer
                    Dim diae As Integer
                    Dim Mêse As Integer
                    Dim Anoe As Integer
                    Dim Valore As Double 'Vazão Defluente, que normalmente é substituida
                    'Dim Valore_aflu As Double 'Vazão Afluente, que normalmente é assimilada pelo modelo

                    'MsgBox(Estação(CodigoLocalizado))
                    'MsgBox(linha4)
                    'Identifica com qual das hidroelétricas estamos trabalhando

                    'ID;CODIGO;LAT;LON;ALT;NOME;A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                    '1;UHE_Gaibaldi_deflu;-28.268555;-50.692835;0;UHE_Gaibaldi_deflu;0;0;0
                    '2;UHE_Gaibaldi_aflu;-28.268555;-50.692835;0;UHE_Gaibaldi_aflu;0;0;0

                    Try
                        '    If CodigoLocalizado = 1 Then 'se for Garibaldi defluente
                        '        horae = linha4.Substring(9, 2)
                        '        diae = linha4.Substring(6, 2)
                        '        Mêse = linha4.Substring(4, 2)
                        '        Anoe = linha4.Substring(0, 4)
                        '        Valore = linha4.Substring(52, 6)

                        '    ElseIf CodigoLocalizado = 2 Then 'se for Garibaldi afluente
                        horae = linha4.Substring(9, 2)
                        diae = linha4.Substring(6, 2)
                        Mêse = linha4.Substring(4, 2)
                        Anoe = linha4.Substring(0, 4)
                        Valore = linha4.Substring(31, 6)
                        'MsgBox(Valore)
                        '            End If

                    Catch ex As Exception
                        Continue Do
                    End Try

                    If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                        Dim data_c As Date
                        data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                        data_c = data_c.AddHours(1)
                        horae = data_c.Hour
                        diae = data_c.Day
                        Mêse = data_c.Month
                        Anoe = data_c.Year
                    End If

                    'Corrigir lajeado, cana brava e estreito com mais três horas, pois os dados são utc
                    'If CodigoLocalizado <= 4 Then
                    '    Dim data_c As Date
                    '    data_c = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                    '    data_c = data_c.AddHours(3)
                    '    horae = data_c.Hour
                    '    diae = data_c.Day
                    '    Mêse = data_c.Month
                    '    Anoe = data_c.Year
                    'End If

                    'MsgBox(diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00")
                    'Qsub(arquivo,hh,dd,mm,Ano(aaaa))

                    If Valore < 99999 And Valore > 0 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    ElseIf Valore = 9999 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    ElseIf Valore <= 0 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    End If

                    'MsgBox(diae & " " & horae & " " & Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)))

                    'Verifica as datas iniciais e finais dos arquivos de vazão
                    If diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00" < dataIp(CodigoLocalizado) Then
                        dataIp(CodigoLocalizado) = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                        If dataIp(CodigoLocalizado) < dataI Then
                            dataI = dataIp(CodigoLocalizado)
                        End If
                    End If

                    If diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00" > dataFp(CodigoLocalizado) Then
                        dataFp(CodigoLocalizado) = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                        If dataFp(CodigoLocalizado) > dataF Then
                            dataF = dataFp(CodigoLocalizado)
                        End If
                    End If

                    'Depois de verificar as datas, vai para o próximo arquivo aberto

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)

            End If
            FileClose()
        Next


        'MsgBox(dataFp(15) & " " & dataIp(15))


        'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
        Me.AtualizaDataTabela()
        'Atualiza o arquivo de vazão substituída do database - substituido pelas duas rotinas abaixo
        'Me.ATUALIZA_ARQUIVOS_NIVEL()
        'Me.ATUALIZA_ARQUIVOS_VAZÃO()
        'Atualiza apenas o arquivo de vazão do local dado no database
        Me.ATUALIZA_ARQUIVO_UNICO_NIVEL(15)
        Me.ATUALIZA_ARQUIVO_UNICO_VAZÃO(15)

        'Atualiza as datas sugeridas nos seletores de datas
        Me.ATUALIZA_DATA()
        'Exibe mensagem de atualização bem sucedida
        ' MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")
        Stlb.Text = "Atualização automática finalizada com sucesso!"
        FileClose()

    End Sub

    Sub ATUALIZA_CAMPOS_NOVOS(ByVal Codigo As Integer)
        'Rotina de atualizaçãp de dados das estações fluviométricas
        'Ela permite a atualização de cada posto, individualmente, muito embora ela possibilite que multiplos arquivos sejam inseridos
        'É a única rotina que deve ser atualizada em caso de mudança nos postos, pois tem coisas que devem ser modificadas no código

        ''Libera a seleção de multiplos arquivos
        'ofd.Multiselect = True
        ''coloca o filtro dos arquivos de vazão
        'ofd.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"
        ''abre a janela de carregar arquivos, e só continua se tudo estiver OK
        'If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then

        ofd.FileName = diretorio_arquivos & "\" & "uhcn.txt"

        'Cria a variável que corresponde aos strings selecionados na caixa de diálogo
        Dim s As String
        'Cria a variável que guarda o número de arquivos carregados na caixa de diálogo
        Dim pmax As Integer
        'Preenche o número de arquivos
        pmax = ofd.FileNames.Length
        'Cria a variável que contém o nome dos arquivos individualmente, já na dimensão necessária
        Dim Arqu(pmax) As String
        'Cria a varíavel p, que contém a informação de qual posto da lista de postos nós estamos utilizando
        Dim p As Integer
        'Inicialmente p = 0, pois não começamos a ler os postos ainda
        p = 0

        'Agora, rodamos este loop, em que "s" assume o valor de cada um dos arquivos carregados
        For Each s In ofd.FileNames
            'somamos 1 à variável p, indicando qual arquivo estamos lendo
            p = p + 1
            'o nome do arquivo é guardado na variável Arqu(p)
            Arqu(p) = s

            'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
            Dim CodigoLocalizado As Integer
            'Pega na tabela qual posto estamos utilizando
            'O +1 no final é utilizado pois a contagem da tabela começa em zero
            CodigoLocalizado = 16 'DataGridView1.RowCount - 1 'Codigo 'DataGridView1.CurrentCell.RowIndex + 1

            'Caso o código for maior que zero, executa a rotina de ler o arquivo 
            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha4 As String
                'Abre o arquivo
                FileOpen(4, Arqu(p), OpenMode.Input)
                'Começa a leitura até o final, linha por linha
                Do Until EOF(4)
                    'linha4 contém a informação da linha
                    linha4 = LineInput(4)

                    'Cria estas variáveis, que contém as informações de hora, dia, mes ano e vazão que estamos lendo
                    Dim horae As Integer
                    Dim diae As Integer
                    Dim Mêse As Integer
                    Dim Anoe As Integer
                    Dim Valore As Double 'Vazão Defluente, que normalmente é substituida
                    'Dim Valore_aflu As Double 'Vazão Afluente, que normalmente é assimilada pelo modelo

                    'MsgBox(Estação(CodigoLocalizado))

                    'Identifica com qual das hidroelétricas estamos trabalhando

                    'ID;CODIGO;LAT;LON;ALT;NOME;A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                    '1;UHE_Gaibaldi_deflu;-28.268555;-50.692835;0;UHE_Gaibaldi_deflu;0;0;0
                    '2;UHE_Gaibaldi_aflu;-28.268555;-50.692835;0;UHE_Gaibaldi_aflu;0;0;0

                    Try
                        '    If CodigoLocalizado = 1 Then 'se for Garibaldi defluente
                        '        horae = linha4.Substring(9, 2)
                        '        diae = linha4.Substring(6, 2)
                        '        Mêse = linha4.Substring(4, 2)
                        '        Anoe = linha4.Substring(0, 4)
                        '        Valore = linha4.Substring(52, 6)

                        '    ElseIf CodigoLocalizado = 2 Then 'se for Garibaldi afluente
                        horae = linha4.Substring(9, 2)
                        diae = linha4.Substring(6, 2)
                        Mêse = linha4.Substring(4, 2)
                        Anoe = linha4.Substring(0, 4)
                        Valore = linha4.Substring(31, 6)
                        'MsgBox(Valore)
                        '            End If

                    Catch ex As Exception
                        Continue Do
                    End Try

                    If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                        Dim data_c As Date
                        data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                        data_c = data_c.AddHours(1)
                        horae = data_c.Hour
                        diae = data_c.Day
                        Mêse = data_c.Month
                        Anoe = data_c.Year
                    End If

                    'Corrigir lajeado, cana brava e estreito com mais três horas, pois os dados são utc
                    'If CodigoLocalizado <= 4 Then
                    '    Dim data_c As Date
                    '    data_c = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                    '    data_c = data_c.AddHours(3)
                    '    horae = data_c.Hour
                    '    diae = data_c.Day
                    '    Mêse = data_c.Month
                    '    Anoe = data_c.Year
                    'End If

                    'MsgBox(diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00")
                    'Qsub(arquivo,hh,dd,mm,Ano(aaaa))

                    If Valore < 99999 And Valore > 0 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    ElseIf Valore = 9999 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    ElseIf Valore <= 0 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    End If

                    'Verifica as datas iniciais e finais dos arquivos de vazão
                    If diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00" < dataIp(CodigoLocalizado) Then
                        dataIp(CodigoLocalizado) = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                        If dataIp(CodigoLocalizado) < dataI Then
                            dataI = dataIp(CodigoLocalizado)
                        End If
                    End If

                    If diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00" > dataFp(CodigoLocalizado) Then
                        dataFp(CodigoLocalizado) = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                        If dataFp(CodigoLocalizado) > dataF Then
                            dataF = dataFp(CodigoLocalizado)
                        End If
                    End If

                    'Depois de verificar as datas, vai para o próximo arquivo aberto

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)

            End If
            FileClose()
        Next


        'MsgBox(dataFp(15) & " " & dataIp(15))


        'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
        Me.AtualizaDataTabela()
        'Atualiza o arquivo de vazão substituída do database - substituido pelas duas rotinas abaixo
        'Me.ATUALIZA_ARQUIVOS_NIVEL()
        'Me.ATUALIZA_ARQUIVOS_VAZÃO()
        'Atualiza apenas o arquivo de vazão do local dado no database
        Me.ATUALIZA_ARQUIVO_UNICO_NIVEL(16)
        Me.ATUALIZA_ARQUIVO_UNICO_VAZÃO(16)

        'Atualiza as datas sugeridas nos seletores de datas
        Me.ATUALIZA_DATA()
        'Exibe mensagem de atualização bem sucedida
        ' MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")
        Stlb.Text = "Atualização automática finalizada com sucesso!"

    End Sub

    Sub ATUALIZA_GARIBALDI(ByVal Codigo As Integer)
        'Rotina de atualizaçãp de dados das estações fluviométricas
        'Ela permite a atualização de cada posto, individualmente, muito embora ela possibilite que multiplos arquivos sejam inseridos
        'É a única rotina que deve ser atualizada em caso de mudança nos postos, pois tem coisas que devem ser modificadas no código

        ''Libera a seleção de multiplos arquivos
        'ofd.Multiselect = True
        ''coloca o filtro dos arquivos de vazão
        'ofd.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"
        ''abre a janela de carregar arquivos, e só continua se tudo estiver OK
        'If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then

        ofd.FileName = diretorio_arquivos & "\" & "Dados_Hidro.txt"

        'Cria a variável que corresponde aos strings selecionados na caixa de diálogo
        Dim s As String
        'Cria a variável que guarda o número de arquivos carregados na caixa de diálogo
        Dim pmax As Integer
        'Preenche o número de arquivos
        pmax = ofd.FileNames.Length
        'Cria a variável que contém o nome dos arquivos individualmente, já na dimensão necessária
        Dim Arqu(pmax) As String
        'Cria a varíavel p, que contém a informação de qual posto da lista de postos nós estamos utilizando
        Dim p As Integer
        'Inicialmente p = 0, pois não começamos a ler os postos ainda
        p = 0

        'Agora, rodamos este loop, em que "s" assume o valor de cada um dos arquivos carregados
        For Each s In ofd.FileNames
            'somamos 1 à variável p, indicando qual arquivo estamos lendo
            p = p + 1
            'o nome do arquivo é guardado na variável Arqu(p)
            Arqu(p) = s

            'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
            Dim CodigoLocalizado As Integer
            'Pega na tabela qual posto estamos utilizando
            'O +1 no final é utilizado pois a contagem da tabela começa em zero
            CodigoLocalizado = 17 'DataGridView1.RowCount - 1 'Codigo 'DataGridView1.CurrentCell.RowIndex + 1

            'Caso o código for maior que zero, executa a rotina de ler o arquivo 
            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha4 As String
                'Abre o arquivo
                FileOpen(4, Arqu(p), OpenMode.Input)
                'Começa a leitura até o final, linha por linha
                Do Until EOF(4)
                    'linha4 contém a informação da linha
                    linha4 = LineInput(4)

                    'Cria estas variáveis, que contém as informações de hora, dia, mes ano e vazão que estamos lendo
                    Dim horae As Integer
                    Dim diae As Integer
                    Dim Mêse As Integer
                    Dim Anoe As Integer
                    Dim Valore As Double 'Vazão Defluente, que normalmente é substituida
                    'Dim Valore_aflu As Double 'Vazão Afluente, que normalmente é assimilada pelo modelo

                    'MsgBox(Estação(CodigoLocalizado))

                    'Identifica com qual das hidroelétricas estamos trabalhando

                    'ID;CODIGO;LAT;LON;ALT;NOME;A;B;C (Variáveis das curvas chave tipo Q(h)=a*h^2 + b*h + c)
                    '1;UHE_Gaibaldi_deflu;-28.268555;-50.692835;0;UHE_Gaibaldi_deflu;0;0;0
                    '2;UHE_Gaibaldi_aflu;-28.268555;-50.692835;0;UHE_Gaibaldi_aflu;0;0;0

                    Try
                        '    If CodigoLocalizado = 1 Then 'se for Garibaldi defluente
                        '        horae = linha4.Substring(9, 2)
                        '        diae = linha4.Substring(6, 2)
                        '        Mêse = linha4.Substring(4, 2)
                        '        Anoe = linha4.Substring(0, 4)
                        '        Valore = linha4.Substring(52, 6)

                        '    ElseIf CodigoLocalizado = 2 Then 'se for Garibaldi afluente
                        horae = linha4.Substring(9, 2)
                        diae = linha4.Substring(6, 2)
                        Mêse = linha4.Substring(4, 2)
                        Anoe = linha4.Substring(0, 4)
                        Valore = linha4.Substring(31, 6)
                        'MsgBox(Valore)
                        '            End If

                    Catch ex As Exception
                        Continue Do
                    End Try

                    If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                        Dim data_c As Date
                        data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                        data_c = data_c.AddHours(1)
                        horae = data_c.Hour
                        diae = data_c.Day
                        Mêse = data_c.Month
                        Anoe = data_c.Year
                    End If

                    'Corrigir lajeado, cana brava e estreito com mais três horas, pois os dados são utc
                    'If CodigoLocalizado <= 4 Then
                    '    Dim data_c As Date
                    '    data_c = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                    '    data_c = data_c.AddHours(3)
                    '    horae = data_c.Hour
                    '    diae = data_c.Day
                    '    Mêse = data_c.Month
                    '    Anoe = data_c.Year
                    'End If

                    'MsgBox(diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00")
                    'Qsub(arquivo,hh,dd,mm,Ano(aaaa))

                    If Valore < 99999 And Valore > 0 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    ElseIf Valore = 9999 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    ElseIf Valore <= 0 Then
                        Q(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore
                        Niv(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = -1.0
                    End If

                    'Verifica as datas iniciais e finais dos arquivos de vazão
                    If diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00" < dataIp(CodigoLocalizado) Then
                        dataIp(CodigoLocalizado) = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                        If dataIp(CodigoLocalizado) < dataI Then
                            dataI = dataIp(CodigoLocalizado)
                        End If
                    End If

                    If diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00" > dataFp(CodigoLocalizado) Then
                        dataFp(CodigoLocalizado) = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                        If dataFp(CodigoLocalizado) > dataF Then
                            dataF = dataFp(CodigoLocalizado)
                        End If
                    End If

                    'Depois de verificar as datas, vai para o próximo arquivo aberto

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)

            End If
            FileClose()
        Next


        'MsgBox(dataFp(15) & " " & dataIp(15))


        'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
        Me.AtualizaDataTabela()
        'Atualiza o arquivo de vazão substituída do database - substituido pelas duas rotinas abaixo
        'Me.ATUALIZA_ARQUIVOS_NIVEL()
        'Me.ATUALIZA_ARQUIVOS_VAZÃO()
        'Atualiza apenas o arquivo de vazão do local dado no database
        Me.ATUALIZA_ARQUIVO_UNICO_NIVEL(17)
        Me.ATUALIZA_ARQUIVO_UNICO_VAZÃO(17)

        'Atualiza as datas sugeridas nos seletores de datas
        Me.ATUALIZA_DATA()
        'Exibe mensagem de atualização bem sucedida
        ' MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")
        Stlb.Text = "Atualização automática finalizada com sucesso!"

    End Sub

    Sub ATUALIZA_ARQUIVO_UNICO_NIVEL(ByVal OrdemPosto As Integer)
        ' For p As Integer = 1 To total_estações
        Dim p As Integer
        p = OrdemPosto


        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
        'tmaxx = dif. de tempo

        Dim datacomeço As Date
        datacomeço = dataIp(p)
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\N" & Estação(p) & ".txt"

        'Dim arquivo_est3 As String
        'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

        FileOpen(2, arquivo_est2, OpenMode.Output)
        'FileOpen(3, arquivo_est3, OpenMode.Output)

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

            'If FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar

            'End If

            Print(2, SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))).PadLeft(12) & "0000")

            'If SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
            '    Print(2, "        " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
            '    Print(2, "       " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
            '    Print(2, "      " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
            '    Print(2, "     " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
            '    Print(2, "    " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
            '    Print(2, "   " & SubstituirSeparador(Niv(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            '    'não diminui dois espaços pois não tem o separador de milhar
            'End If

            PrintLine(2)

        Next

        FileClose()

        'Next

    End Sub

    Sub ATUALIZA_ARQUIVO_UNICO_VAZÃO(ByVal OrdemPosto As Integer)
        ' For p As Integer = 1 To total_estações
        Dim p As Integer
        p = OrdemPosto

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
        'tmaxx = dif. de tempo

        Dim datacomeço As Date
        datacomeço = dataIp(p)
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

        'Dim arquivo_est3 As String
        'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

        FileOpen(2, arquivo_est2, OpenMode.Output)
        'FileOpen(3, arquivo_est3, OpenMode.Output)

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

            'If FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar

            'End If

            Print(2, SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))).PadLeft(12) & "0000")

            'If SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
            '    Print(2, "        " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
            '    Print(2, "       " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
            '    Print(2, "      " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
            '    Print(2, "     " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
            '    Print(2, "    " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            'ElseIf SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
            '    Print(2, "   " & SubstituirSeparador(Q(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            '    'não diminui dois espaços pois não tem o separador de milhar
            'End If

            PrintLine(2)

        Next

        FileClose()

    End Sub

    Private Sub Button9_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button9.Click
        Me.ATUALIZAR_ONLINE()


        'Me.ATUALIZA_BARRA_GRANDE(2)

        ' ''Try
        ' ''    'Me.ATUALIZA_AUTOMATICAMENTE() 'Faz duas vezes, procurando em diretórios diferentes
        ' ''    'Me.ATUALIZA_AUTOMATICAMENTE2() 'Faz duas vezes, procurando em diretórios diferentes
        ' ''    'Me.ATUALIZA_AUTOMATICAMENTE3() 'Faz duas vezes, procurando em diretórios diferentes


        ' ''    Me.ATUALIZAR_ONLINE()

        ' ''    Me.ATUALIZA_BARRA_GRANDE(2)
        ' ''    Me.ATUALIZA_CAMPOS_NOVOS(2)
        ' ''    Me.ATUALIZA_GARIBALDI(2)

        ' ''Catch ex As Exception
        ' ''    'MsgBox(ex.Message)
        ' ''    MsgBox("Não foi possível atualizar os dados automaticamente.")
        ' ''End Try

    End Sub

#Region "Atualiza Online na Telemetria ANA"

    Sub ATUALIZAR_ONLINE()
        'Avisa que está executando
        ToolStripStatusLabel1.Text = "Conectando e fazendo download..."

        statusbar.Maximum = total_estações
        statusbar.Value = 0

        'Para cada uma das estações na tabela, em ordem, executa as rotinas de download
        For DadoEstação As Integer = 1 To total_estações '-3 para não baixar as 3 últimas, que são as usinas
            'A data inicial de download é a final de dados do arquivo
            Dim data_começo As Date = dataFp(DadoEstação).Day & "/" & dataFp(DadoEstação).Month & "/" & dataFp(DadoEstação).Year & " " & dataFp(DadoEstação).Hour & ":00:00"

            'A data final de download é sempre o momento (agora)
            Dim data_final As Date = Now.Day & "/" & Now.Month & "/" & Now.Year & " " & Now.Hour & ":00:00"

            'Executa a rotina de download
            Download_WebService(Estação(DadoEstação), data_começo, data_final, DadoEstação)

            'Adiciona valor na barra de status
            statusbar.Increment(1)
        Next

        'Executa as rotinas de atualização da base de dados
        Me.AtualizaDataTabela()
        'Me.ATUALIZA_ARQUIVOS_NIVEL()   'Desativado em 20 de outubro visando acelerar a escrita
        Me.ATUALIZA_ARQUIVOS_NIVEL_Legado()
        'Me.ATUALIZA_ARQUIVOS_VAZÃO() 'Desativado em 20 de outubro visando acelerar a escrita
        Me.ATUALIZA_ARQUIVOS_VAZÃO_Legado()
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
        Requisitar(arquivoFonte, CodEstação)
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
        Dim dataStream As IO.Stream = response.GetResponseStream()
        ' Open the stream using a StreamReader for easy access.  
        Dim reader As New IO.StreamReader(dataStream)
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
        Dim data, valor, vazao, chuva, nivel As Integer

        Try
            Do While (reader.Read())
                Select Case reader.NodeType
                    Case XmlNodeType.Element 'Exibir o início do elemento.

                        'Verifica se o elemento lido é a data, dado de chuva, vazão, ou nível
                        If reader.Name = "DataHora" Then
                            data = 1
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0
                        ElseIf reader.Name = "Chuva" Then
                            data = 0
                            valor = 0
                            vazao = 0
                            chuva = 1
                            nivel = 0
                        ElseIf reader.Name = "Vazao" Then
                            data = 0
                            valor = 0
                            vazao = 1
                            chuva = 0
                            nivel = 0
                        ElseIf reader.Name = "Nivel" Then
                            data = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 1
                        End If

                    Case XmlNodeType.Text 'Exibir o texto em cada elemento.
                        If data = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
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
                            chuva = 0
                            nivel = 0
                            Q(numposição, hh, dia, mes, Ano(year)) = reader.Value
                            If Q(numposição, hh, dia, mes, Ano(year)).ToString.Length < 2 Then
                                ' MsgBox("Vazão = " & reader.Value)
                                Q(numposição, hh, dia, mes, Ano(year)) = -1.0
                            Else
                                Q(numposição, hh, dia, mes, Ano(year)) = AjustarDecimal(reader.Value)
                            End If

                            If Q(numposição, hh, dia, mes, Ano(year)) > 0 Then
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

                        ElseIf chuva = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0
                        ElseIf nivel = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0
                            Niv(numposição, hh, dia, mes, Ano(year)) = reader.Value
                            If Niv(numposição, hh, dia, mes, Ano(year)).ToString.Length < 2 Then
                                'MsgBox("Nível = " & reader.Value)
                                Niv(numposição, hh, dia, mes, Ano(year)) = -1.0
                            Else
                                Niv(numposição, hh, dia, mes, Ano(year)) = AjustarDecimal(reader.Value)
                            End If
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





    Private Sub Button10_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button10.Click
        Me.ATUALIZAR_N_OFFLINE_TRIPA()
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
                                Qdispo(p, hh, dd, mm, Ano(aaaa)) = Q(p, hh, dd, mm, Ano(aaaa))
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

            Q_VERIFICA = True 'Avisa que é vazão
            Dim frmVER As New Form13
            frmVER.Show()
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
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

    Sub CARREGAR_DADOS_Q_NOVO_DE_UMA_PASTA()


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
                ' If cod_estação = Estação(po) Then
                If cod_estação = "Q" & Estação(po) Then
                    CodigoLocalizado = po
                End If
            Next

            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo



                NumArquivosAtualizados += 1

                Dim linha4 As String
                Dim h, qi As Double
                FileOpen(4, Arqu(p), OpenMode.Input)
                Dim contador As Integer
                contador = -1
                Do Until EOF(4)
                    linha4 = LineInput(4)

                    contador += 1
                    If contador < 1 Then Continue Do

                    'MsgBox(linha4)
                    'MsgBox(linha4.Substring(26, 10))


                    qi = AjustarDecimal(linha4.Substring(26, 10))

                    If qi < 0 Then qi = -1.0

                    'MsgBox(linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00")

                    'Niv(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = -1.0
                    Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = qi

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
            ' Me.ATUALIZA_ARQUIVOS_NIVEL()
            ' Me.ATUALIZA_ARQUIVOS_VAZÃO()

            Me.ATUALIZA_ARQUIVOS_NIVEL_Legado() 'novas rotinas mais eficientes
            Me.ATUALIZA_ARQUIVOS_VAZÃO_Legado() 'novas rotinas mais eficientes
            Me.ATUALIZA_DATA()
            Stlb.Text = "Atualização a partir de arquivos na pasta finalizada com sucesso!"

        End If
    End Sub

    Sub CARREGAR_DADOS_N_NOVO_DE_UMA_PASTA()


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
                ' If cod_estação = Estação(po) Then
                If cod_estação = "N" & Estação(po) Then
                    CodigoLocalizado = po
                End If
            Next

            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo


                NumArquivosAtualizados += 1
                Dim linha4 As String
                Dim h, qi As Double
                FileOpen(4, Arqu(p), OpenMode.Input)
                Dim contador As Integer
                contador = -1
                Do Until EOF(4)
                    linha4 = LineInput(4)

                    contador += 1
                    If contador < 1 Then Continue Do

                    h = AjustarDecimal(linha4.Substring(26, 10))

                    Niv(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = h
                    'Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = Q(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4)))

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
            'Me.ATUALIZA_ARQUIVOS_NIVEL()
            'Me.ATUALIZA_ARQUIVOS_VAZÃO()

            Me.ATUALIZA_ARQUIVOS_NIVEL_Legado() 'novas rotinas mais eficientes
            Me.ATUALIZA_ARQUIVOS_VAZÃO_Legado() 'novas rotinas mais eficientes
            Me.ATUALIZA_DATA()
            ToolStripStatusLabel1.Text = "Atualização finalizada"

        End If
    End Sub

    Private Sub Button12_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button12.Click
        Me.CARREGAR_DADOS_Q_NOVO_DE_UMA_PASTA()
        Me.CARREGAR_DADOS_N_NOVO_DE_UMA_PASTA()

    End Sub

    Private Sub DataGridView1_CellContentClick(sender As Object, e As DataGridViewCellEventArgs) Handles DataGridView1.CellContentClick

    End Sub
End Class