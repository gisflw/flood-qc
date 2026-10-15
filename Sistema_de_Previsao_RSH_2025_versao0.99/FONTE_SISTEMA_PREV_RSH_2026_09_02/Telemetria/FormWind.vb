Public Class FormWind

    Private diretorio_previsao As String 'diretório onde deve ser salvo o arquivo de chuvas observadas
    Private diretorio_automatico, diretorio_automatico1, diretorio_automatico2, diretorio_automatico3 As String

    Public Vento_vel(,,,)
    Public Vento_dir(,,,)

    Private Sub FormWind_Load(sender As Object, e As EventArgs) Handles MyBase.Load
        'Me.CARREGA_MINI()
        'Me.Dimensiona_Colunas()
        ' Me.CARREGA_REGIOES()

        'Me.CARREGA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()

        Me.CARREGA_DIRETÓRIOS()
        Me.ATUALIZA_DATA()
        Me.Carrega_Hora_Previsão()
        Me.CRIA_CARREGA_ARQUIVO_VENTO()
        Me.CARREGA_TABELA_PRINCIPAL()

        'Me.CARREGA_RADIOBUTTONS_CHUVA()

        ''Se o automático estiver ligado
        'If TotalCommander = True Then

        '    Dim log_file As String
        '    log_file = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Log_operação_automatica.txt"
        '    FileOpen(99, log_file, OpenMode.Append)
        '    PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Abriu a janela de chuva prevista.")
        '    FileClose(99)

        '    Me.CARREGA_ARQUIVO_CHUVA_DIRETORIO_AUTOMATICO()

        '    FileOpen(99, log_file, OpenMode.Append)
        '    PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Carregou dados de entrada da janela de chuva prevista.")
        '    FileClose(99)

        '    Me.APERTA_BOTAO_INTERPOLAR()

        '    FileOpen(99, log_file, OpenMode.Append)
        '    PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Interpolou a chuva prevista. Próxima etapa: Vazão Observada.")
        '    FileClose(99)

        '    Me.Close()
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

        'If cb_dir.Text = "1" Then
        '    diretorio_automatico = diretorio_automatico1
        'ElseIf cb_dir.Text = "2" Then
        '    diretorio_automatico = diretorio_automatico2
        'ElseIf cb_dir.Text = "3" Then
        '    diretorio_automatico = diretorio_automatico3
        'End If

        stlb.Text = "Diretório arquivos de previsão: " & diretorio_previsao
        'Fecha
        FileClose()

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

    Sub ATUALIZA_DATA()


        Dim datahoje As Date
        datahoje = Now.Date

        dtpqob1.Value = "01/07/" & datahoje.AddYears(-1).Year
            tb_hora_ini.Text = "00"


    End Sub

    Sub CRIA_CARREGA_ARQUIVO_VENTO()
        Try

            ReDim Vento_vel(24, 31, 12, Ano(AF))
            ReDim Vento_dir(24, 31, 12, Ano(AF))

            '--
            dataI = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
            dataF = "01/01/" & AI & " " & "00:00:00"
            HoraI = 23
            HoraF = 0
            '--

            'Zera as variáveis de velocidade e direção do vento

            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Vento_vel(hh, dd, mm, Ano(aaaa)) = 0.0
                            Vento_dir(hh, dd, mm, Ano(aaaa)) = 90
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis

            'define o arquivo
            Dim arquivo_est2 As String
            arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Vento_Vel.txt"

            Dim arquivo_est3 As String
            arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Vento_Dir.txt"


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
                    Vento_vel(linha2.Substring(22, 2), linha2.Substring(4, 2), linha2.Substring(10, 2), Ano(linha2.Substring(14, 4))) = AjustarDecimal(linha2.Substring(26, 10))
                    Vento_dir(linha3.Substring(22, 2), linha3.Substring(4, 2), linha3.Substring(10, 2), Ano(linha3.Substring(14, 4))) = AjustarDecimal(linha3.Substring(26, 10))

                    If linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00" < dataI Then
                        dataI = linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00"
                    End If

                    If linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00" > dataF Then
                        dataF = linha2.Substring(4, 2) & "/" & linha2.Substring(10, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(22, 2) & ":00:00"
                    End If
                End If
            Loop

            FileClose()
            'Carregou dados internos

        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

    End Sub

    Private Sub Button6_Click(sender As Object, e As EventArgs) Handles Button6.Click
        Me.ATUALIZA_DADOS()
        Me.ESCREVE_DATABASE_VEL_HORA()
        Me.ESCREVE_DATABASE_DIR_HORA()
        Me.EXPORTA_DADO_VENTO()

        MsgBox("Dados exportados com sucesso!", MsgBoxStyle.Information, "Previsão")
        Me.Close()
    End Sub

    Sub CARREGA_TABELA_PRINCIPAL()


        Dim datacomeço As Date
        datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim datafinal As Date
        datafinal = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"


        Dim difft As Integer
        difft = DateDiff(DateInterval.Hour, datacomeço, datafinal) + 1

        datacomeço = datacomeço.AddHours(-1)

        For t As Integer = 1 To difft
            datacomeço = datacomeço.AddHours(1)

            Dim dia, mes, hora As String
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

            'Adiciona os dados à tabela

            Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

            ' Preenche os valores do registro
            nova_linha.ItemArray = New Object() {dia, mes, datacomeço.Year, hora, Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))}

            Try
                DataSet1.Tables(0).Rows.Add(nova_linha)
            Catch ex As Exception
                MsgBox(" erro: " & ex.Message)
            End Try


        Next

    End Sub

    Sub ATUALIZA_DADOS()

        Dim tmaxx As Integer
        tmaxx = DataGridView1.RowCount - 1
        'tmaxx = dif. de tempo

        Dim dia As Integer
        Dim mes As Integer
        Dim anos As Integer
        Dim hora As Integer
        Dim valor_v As Single
        Dim valor_d As Single

        For aux = 1 To tmaxx
            dia = DataGridView1.Item(0, aux - 1).Value
            mes = DataGridView1.Item(1, aux - 1).Value
            anos = DataGridView1.Item(2, aux - 1).Value
            hora = DataGridView1.Item(3, aux - 1).Value
            valor_v = DataGridView1.Item(4, aux - 1).Value
            valor_d = DataGridView1.Item(5, aux - 1).Value

            Vento_vel(hora, dia, mes, Ano(anos)) = AjustarDecimal(valor_v)
            Vento_dir(hora, dia, mes, Ano(anos)) = AjustarDecimal(valor_d)

        Next
    End Sub

    Private Sub Button1_Click(sender As Object, e As EventArgs) Handles Button1.Click

        Me.CARREGAR_DADOS_VEL_PONTUAL()
        Me.LIMPA_TABELA_PRINCIPAL()
        Me.CARREGA_TABELA_PRINCIPAL()

    End Sub

    Sub CARREGAR_DADOS_VEL_PONTUAL()

        'Libera a seleção de multiplos arquivos
        ofd.Multiselect = False
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "ASCII com dados horários dispostos em colunas (.txt)|*.txt"
        ofd.InitialDirectory = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Vento\plugin\dist\result"

        'abre a janela de carregar arquivos, e só continua se tudo estiver OK
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
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

                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha2 As String

                FileOpen(4, Arqu(p), OpenMode.Input)

                Dim contador As Integer
                contador = -1
                Do Until EOF(4)
                    linha2 = LineInput(4)

                    contador += 1
                    If contador = 0 Then Continue Do

                    If contador > 1 Then
                        Vento_vel(linha2.Substring(18, 2), linha2.Substring(0, 2), linha2.Substring(6, 2), Ano(linha2.Substring(12, 4))) = AjustarDecimal(linha2.Substring(30, 6))

                        If linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(12, 4) & " " & linha2.Substring(18, 2) & ":00:00" < dataI Then
                            dataI = linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(18, 2) & ":00:00"
                        End If

                        If linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(12, 4) & " " & linha2.Substring(18, 2) & ":00:00" > dataF Then
                            dataF = linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(12, 4) & " " & linha2.Substring(18, 2) & ":00:00"
                        End If
                    End If


                    'Depois de verificar as datas, vai para o próximo arquivo aberto

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)



            Next

            'Depois que percorreu todos os arquivos selecionados na caixa de diálogo

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            'Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            ' Me.ATUALIZA_ARQUIVOS_QSUBST_TODOS()
            'Atualiza as datas sugeridas nos seletores de datas
            'Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            'Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")

        End If
    End Sub

    Sub CARREGAR_DADOS_DIR_PONTUAL()

        'Libera a seleção de multiplos arquivos
        ofd.Multiselect = False
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "ASCII com dados horários dispostos em colunas (.txt)|*.txt"

        'abre a janela de carregar arquivos, e só continua se tudo estiver OK
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then
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

                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha2 As String

                FileOpen(4, Arqu(p), OpenMode.Input)

                Dim contador As Integer
                contador = -1
                Do Until EOF(4)
                    linha2 = LineInput(4)

                    contador += 1
                    If contador = 0 Then Continue Do

                    If contador > 1 Then
                        Vento_dir(linha2.Substring(18, 2), linha2.Substring(0, 2), linha2.Substring(6, 2), Ano(linha2.Substring(12, 4))) = AjustarDecimal(linha2.Substring(30, 6))

                        If linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(12, 4) & " " & linha2.Substring(18, 2) & ":00:00" < dataI Then
                            dataI = linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(14, 4) & " " & linha2.Substring(18, 2) & ":00:00"
                        End If

                        If linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(12, 4) & " " & linha2.Substring(18, 2) & ":00:00" > dataF Then
                            dataF = linha2.Substring(0, 2) & "/" & linha2.Substring(6, 2) & "/" & linha2.Substring(12, 4) & " " & linha2.Substring(18, 2) & ":00:00"
                        End If
                    End If


                    'Depois de verificar as datas, vai para o próximo arquivo aberto

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)



            Next

            'Depois que percorreu todos os arquivos selecionados na caixa de diálogo

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            'Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            ' Me.ATUALIZA_ARQUIVOS_QSUBST_TODOS()
            'Atualiza as datas sugeridas nos seletores de datas
            'Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            'Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")

        End If
    End Sub

    Sub LIMPA_TABELA_PRINCIPAL()

        Try
            DataSet1.Tables(0).Rows.Clear()
            DataSet1.Tables(0).AcceptChanges()

        Catch ex As Exception
            MsgBox("Erro ao limpar a tabela: " & ex.Message)
        End Try

    End Sub

    Private Sub Button2_Click(sender As Object, e As EventArgs) Handles Button2.Click
        Me.CARREGAR_DADOS_DIR_PONTUAL()
        Me.LIMPA_TABELA_PRINCIPAL()
        Me.CARREGA_TABELA_PRINCIPAL()
    End Sub

    Private Sub Button3_Click(sender As Object, e As EventArgs) Handles Button3.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "BaixadorVento.exe"

        Try
            ' Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\DownloadTelemetriaANA\DownloadTelemetriaANA\DownloadTelemetriaANA\bin\x64\Release\" & ExecutavelFORTRAN)
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Vento\plugin\dist\" & ExecutavelFORTRAN)

        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Sub ESCREVE_DATABASE_VEL_HORA()

        Dim datacomeço As Date
        datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim datafinal As Date
        datafinal = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"


        Dim difft As Integer
        difft = DateDiff(DateInterval.Hour, datacomeço, datafinal) + 1

        datacomeço = datacomeço.AddHours(-1)


        Dim tmaxx As Integer
        tmaxx = difft
        'tmaxx = dif. de tempo

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Vento_Vel.txt"

        'Dim arquivo_est3 As String
        'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

        FileOpen(2, arquivo_est2, OpenMode.Output)
        'FileOpen(3, arquivo_est3, OpenMode.Output)

        PrintLine(2, "   Dia   Mês   Ano  Hora   Velocidade(m.s-1)") 'Escreve o começo do cabeçalho


        For aux = 1 To tmaxx

            datacomeço = datacomeço.AddHours(1)

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

            If SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                'não diminui dois espaços pois não tem o separador de milhar
            End If
            PrintLine(2)

        Next

        FileClose()


    End Sub

    Sub ESCREVE_DATABASE_DIR_HORA()

        Dim datacomeço As Date
        datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim datafinal As Date
        datafinal = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"


        Dim difft As Integer
        difft = DateDiff(DateInterval.Hour, datacomeço, datafinal) + 1

        datacomeço = datacomeço.AddHours(-1)


        Dim tmaxx As Integer
        tmaxx = difft
        'tmaxx = dif. de tempo

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Vento_Dir.txt"


        FileOpen(2, arquivo_est2, OpenMode.Output)

        PrintLine(2, "   Dia   Mês   Ano  Hora   Direção (azimute)") 'Escreve o começo do cabeçalho

        For aux = 1 To tmaxx

            datacomeço = datacomeço.AddHours(1)

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

            If SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                'não diminui dois espaços pois não tem o separador de milhar
            End If
            PrintLine(2)

        Next

        FileClose()


    End Sub

    Sub EXPORTA_DADO_VENTO()

        Dim datacomeço As Date
        datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"

        Dim datafinal As Date
        datafinal = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"


        Dim difft As Integer
        difft = DateDiff(DateInterval.Hour, datacomeço, datafinal) + 1

        Dim tmaxx As Integer
        tmaxx = difft
        'tmaxx = dif. de tempo

        'define o arquivo
        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Input\vento.txt"

        FileOpen(2, arquivo_est2, OpenMode.Output)

        datacomeço = datacomeço.AddHours(-1)

        For aux = 1 To tmaxx

            datacomeço = datacomeço.AddHours(+1)

            Dim int1, int2 As Single
            int1 = SubstituirSeparador(Vento_vel(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))
            int2 = SubstituirSeparador(Vento_dir(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))

            PrintLine(2, int1 & " " & int2)
        Next


        For aux = 1 To tmaxx
            PrintLine(2, "0 90")
        Next

        FileClose()


    End Sub
End Class