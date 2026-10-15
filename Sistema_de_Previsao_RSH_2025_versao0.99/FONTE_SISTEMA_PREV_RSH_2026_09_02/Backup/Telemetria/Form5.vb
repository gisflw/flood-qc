Imports System.Net
Imports System.IO

Public Class Form5
    Public sf As MapWinGIS.Shapefile
    Private diretorio_previsao As String 'diretório onde deve ser salvo o arquivo de vazões observadas
    Private diretorio_arquivos As String 'diretório onde deve ser salvo o arquivo de vazões observadas

    'Variáveis para download dos dados das UHEs
    Private Tipo(), Endereço(), Usuário(), Senha(), Arquivo(), UHE() As String
    Private ColunaNoArquivo(), Ncaracteres() As Integer

    Private Sub GroupBox1_Enter(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Group.Enter

    End Sub

    Private Sub Form5_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        Me.CARREGA_TABELA_ESTAÇÕES()
        Me.ATUALIZA_DATA()
        Me.CARREGA_DIRETÓRIOS() 'carrega os diretórios onde o arquivo será salvo
        Me.Carrega_Hora_Previsão() 'Tenta carregar a hora final da previsão
        Me.ATUALIZA_VAZAO_HORIZONTE()  'Atualiza as colunas de vazão no horizonte
        Me.APERTA_BOTA_SOMENTE_DADOS_OBS()

        'Me.ATUALIZA_GARIBALDI(1) 'Atualiza Garibaldi Deflu
        'Me.ATUALIZA_GARIBALDI(2) 'Atualiza Garibaldi Aflu

        'Se o automático estiver ligado
        If TotalCommander = True Then

            Dim log_file As String
            log_file = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Log_operação_automatica.txt"
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Abriu a janela de operação das UHEs e carregou todos os arquivos de entrada.")
            FileClose(99)

            If rb_PadrãoONS.Checked = True Then
                Me.CARREGAR_DADOS_UHES_NOVO_DE_UMA_PASTA_PADRAO_ONS()
            ElseIf rb_PadrãoMGB.Checked = True Then
                Me.CARREGAR_DADOS_UHES_NOVO_DE_UMA_PASTA()
            End If

            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Carregou os arquivos das UHEs disponíveis nas pastas de previsão automática.")
            FileClose(99)

            'Me.APERTA_BOTA_SOMENTE_DADOS_OBS() 'configura todas elas para ser a fio dagua
            Me.ABERTA_BOTAO_EXPORTAR()


            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Exportou os dados das UHEs para serem usados na previsão.")
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Próxima etapa: Rodar Previsão.")
            FileClose(99)

            Me.Close()
        End If

        ''Se o automático estiver ligado
        'If TotalCommander = True Then

        '    Me.ABERTA_BOTAO_EXPORTAR()
        '    Me.Close()
        'End If
    End Sub

    Sub CARREGA_TABELA_ESTAÇÕES()
        'Try
        'Verifica quantas estações existem na base de dados interna
        Dim arquivo_est As String 'nome do arquivo que contém as estações
        Dim linha As String 'linha de leitura
        total_estações = 0 'número de estações

        'define o arquivo
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesUHEs.txt"

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

        ReDim Tipo(total_estações), Endereço(total_estações), Usuário(total_estações), Senha(total_estações), Arquivo(total_estações), ColunaNoArquivo(total_estações), Ncaracteres(total_estações), UHE(total_estações)

        ReDim Qsub(total_estações, 24, 31, 12, Ano(AF))
        ReDim Q_opera(total_estações, 24, 31, 12, Ano(AF)) 'Variável de operação 18Nov2012
        ReDim Q_inic(total_estações)
        ReDim Q_hori(total_estações)
        ReDim dataIp(total_estações)
        ReDim dataFp(total_estações)
        ReDim HoraIp(total_estações)
        ReDim HoraFp(total_estações)
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
                            Qsub(p, hh, dd, mm, Ano(aaaa)) = -1.0
                            Q_opera(p, hh, dd, mm, Ano(aaaa)) = -1.0  'Variável de operação 18Nov2012
                            Q_inic(p) = -1.0
                            Q_hori(p) = -1.0
                        Next
                    Next
                Next
            Next
        Next
        'Zerou as variáveis


        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesUHEs.txt"
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

                Tipo(est) = splt(9)
                Endereço(est) = splt(10)
                Usuário(est) = splt(11)
                Senha(est) = splt(12)
                Arquivo(est) = splt(13)
                ColunaNoArquivo(est) = splt(14)
                Ncaracteres(est) = splt(15)
                UHE(est) = splt(16)


                Me.LE_DATABASE(est, Estação(est))

                'Adiciona os dados à tabela

                Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

                ' Preenche os valores do registro
                nova_linha.ItemArray = New Object() {Estação(est), dataIp(est), dataFp(est), LON(est), LAT(est), NOME(est), "", "", Tipo(est), Endereço(est), Usuário(est), Senha(est), Arquivo(est), ColunaNoArquivo(est), Ncaracteres(est), UHE(est)}
                'Try
                DataSet1.Tables(0).Rows.Add(nova_linha)
                ' Catch ex As Exception
                'MsgBox(" erro: " & ex.Message)
                '  End Try


            End If
        Loop



        FileClose()
        'Carregou dados internos


        'Catch ex As Exception
        'MsgBox(ex.Message)
        'End Try

        Me.ATUALIZA_VAZAO_HORIZONTE()





    End Sub

    Sub LE_DATABASE(ByVal numest As Integer, ByVal codest As String)
        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\" & codest & ".txt"

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
                Qsub(numest, linha2.Substring(22, 2), linha2.Substring(4, 2), linha2.Substring(10, 2), Ano(linha2.Substring(14, 4))) = AjustarDecimal(linha2.Substring(26, 14))

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

    Sub ATUALIZA_DATA()

        dtpqob1.Value = dataIp(3)
        dtpqob2.Value = dataFp(3)
        'dtpqob3.Value = dataFp(1).AddDays(3)
        tb_hora_ini.Text = tb_hora_ini.Items(dataIp(4).Hour)
        tb_hora_fim.Text = tb_hora_fim.Items(dataFp(4).Hour)
        'tb_hora_ext.Text = tb_hora_fim.Items(dataFp(1).Hour)

        Dim datahoje As Date
        datahoje = Now.Date
        ''If dtpqob1.Value < "01/07/" & datahoje.AddYears(-1).Year Then
        dtpqob1.Value = "01/07/" & datahoje.AddYears(-1).Year
        tb_hora_ini.Text = "00"
        '' End If

        If dtpqob2.Value < dtpqob1.Value Then
            dtpqob2.Value = dtpqob1.Value
            tb_hora_fim.Text = "00"
        End If
    End Sub

    'Rotina de atualização dos postos imbutida no clique do botão 1, a seguir

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        CARREGAR_DADOS_UHES_NOVO()

    End Sub

    Sub CARREGAR_DADOS_UHES_ORIGINAL()
        'Rotina de atualizaçãp de dados das estações fluviométricas
        'Ela permite a atualização de cada posto, individualmente, muito embora ela possibilite que multiplos arquivos sejam inseridos
        'É a única rotina que deve ser atualizada em caso de mudança nos postos, pois tem coisas que devem ser modificadas no código

        'Libera a seleção de multiplos arquivos
        ofd.Multiselect = True
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"
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

                'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
                Dim CodigoLocalizado As Integer
                'Pega na tabela qual posto estamos utilizando
                'O +1 no final é utilizado pois a contagem da tabela começa em zero
                CodigoLocalizado = DataGridView1.CurrentCell.RowIndex + 1

                'Abaixo, identifica se o arquivo carregado realmente pertence à usina de interesse
                If CodigoLocalizado = 1 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "lajeado" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                ElseIf CodigoLocalizado = 2 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "lajeado" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                ElseIf CodigoLocalizado = 3 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "uhcb" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                ElseIf CodigoLocalizado = 4 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "uhcb" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                ElseIf CodigoLocalizado = 5 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "uhet" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                ElseIf CodigoLocalizado = 6 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "uhet" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                ElseIf CodigoLocalizado = 9 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "20920080" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                ElseIf CodigoLocalizado = 10 And System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) <> "20920080" Then
                    If MsgBox("Tem certeza que o dado carregado pertence à " & Estação(CodigoLocalizado) & " ?", MsgBoxStyle.YesNo, "Nome incomum de arquivo") = MsgBoxResult.Yes Then
                    Else
                        Exit Sub
                    End If
                End If
                'Passou da verificação

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

                        'Para estreito, temos a seguinte distribuição:

                        '1;UHE_Lajeado_deflu;-9.755568;-48.373168;197;UHE_Lajeado_deflu;0;0;0
                        '2;UHE_Lajeado_aflu;-9.755568;-48.373168;197;UHE_Lajeado_aflu;0;0;0
                        '3;UHE_Cana Brava_deflu;-13.399167;-48.1425;190;UHE_Cana Brava_deflu;0;0;0
                        '4;UHE_Cana Brava_aflu;-13.399167;-48.1425;190;UHE_Cana Brava_aflu;0;0;0
                        '5;UHE_Estreito_deflu;-6.99538;-47.579126;180;UHE_Estreito_deflu;0;0;0
                        '6;UHE_Estreito_aflu;-6.99538;-47.579126;180;UHE_Estreito_aflu;0;0;0
                        '7;UHE_Peixe Angical_deflu;-12.239083;-48.382369;180;UHE_Estreito_deflu;0;0;0
                        '8;UHE_Peixe Angical_aflu;-12.239083;-48.382369;180;UHE_Estreito_aflu;0;0;0
                        '9;UHE_Serra da Mesa_deflu;-13.833571;-48.304228;437;UHE_Serra da Mesa_aflu;0;0;0
                        '10;UHE_Serra da Mesa_aflu;-13.833571;-48.304228;437;UHE_Serra da Mesa_deflu;0;0;0

                        If CodigoLocalizado = 5 Then 'se for estreito defluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(52, 6)

                        ElseIf CodigoLocalizado = 6 Then 'se for estreito afluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(31, 6)

                        ElseIf CodigoLocalizado = 1 Then 'se for lajeado defluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(52, 6)

                        ElseIf CodigoLocalizado = 2 Then 'se for lajeado afluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(31, 6)

                        ElseIf CodigoLocalizado = 3 Then 'se for canabrava defluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(52, 6)

                        ElseIf CodigoLocalizado = 4 Then 'se for canabrava afluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(31, 6)

                        ElseIf CodigoLocalizado = 7 Then 'se for peixe angical defluente
                            horae = linha4.Substring(18, 6)
                            diae = linha4.Substring(0, 6)
                            Mêse = linha4.Substring(6, 6)
                            Anoe = linha4.Substring(12, 6)
                            Valore = linha4.Substring(24, 11)

                        ElseIf CodigoLocalizado = 8 Then 'se for peixe angical afluente
                            horae = linha4.Substring(18, 6)
                            diae = linha4.Substring(0, 6)
                            Mêse = linha4.Substring(6, 6)
                            Anoe = linha4.Substring(12, 6)
                            Valore = linha4.Substring(24, 11)

                        ElseIf CodigoLocalizado = 9 Then 'se for serra da mesa defluente
                            horae = linha4.Substring(11, 2)
                            diae = linha4.Substring(0, 2)
                            Mêse = linha4.Substring(3, 2)
                            Anoe = linha4.Substring(6, 4)

                            Try
                                Valore = linha4.Substring(42, 10)
                                Valore = Valore / 10
                            Catch ex As Exception
                                Valore = -1.0
                            End Try

                        ElseIf CodigoLocalizado = 10 Then 'se for serra da mesa afluente
                            horae = linha4.Substring(11, 2)
                            diae = linha4.Substring(0, 2)
                            Mêse = linha4.Substring(3, 2)
                            Anoe = linha4.Substring(6, 4)

                            Try
                                Valore = linha4.Substring(42, 10)
                                Valore = Valore / 10
                            Catch ex As Exception
                                Valore = -1.0
                            End Try

                        End If


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
                        If CodigoLocalizado <= 4 Then
                            Dim data_c As Date
                            data_c = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                            data_c = data_c.AddHours(3)
                            horae = data_c.Hour
                            diae = data_c.Day
                            Mêse = data_c.Month
                            Anoe = data_c.Year
                        End If

                        'MsgBox(diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00")
                        'Qsub(arquivo,hh,dd,mm,Ano(aaaa))

                        'Coloca o valor para a variável de vazão
                        Qsub(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore

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

            Next

            'Depois que percorreu todos os arquivos selecionados na caixa de diálogo

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            Me.ATUALIZA_ARQUIVOS_QSUBST()
            'Atualiza as datas sugeridas nos seletores de datas
            Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")

        End If
    End Sub

    Sub CARREGAR_DADOS_UHES_NOVO()
        'Rotina de atualizaçãp de dados das estações das UHES em formato tripa
        total_estações = DataGridView1.Rows.Count - 1

        'Libera a seleção de multiplos arquivos
        ofd.Multiselect = True
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "ASCII com dados horários dispostos em colunas(.uhe)|*.uhe"
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

                'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                ' cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)).Substring(0, 8)
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p))

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                'Caso o código for maior que zero, executa a rotina de ler o arquivo 
                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                    'Cria a variável linha4, que contém a leitura da linha do arquivo
                    Dim linha4 As String
                    FileOpen(4, Arqu(p), OpenMode.Input)

                    Dim contador As Integer
                    contador = -1
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        contador += 1
                        If contador = 0 Then Continue Do

                        Qsub(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = AjustarDecimal(linha4.Substring(24, 12))

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

                        'Depois de verificar as datas, vai para o próximo arquivo aberto

                    Loop

                    'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                    FileClose(4)

                End If

            Next

            'Depois que percorreu todos os arquivos selecionados na caixa de diálogo

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            Me.ATUALIZA_ARQUIVOS_QSUBST_TODOS()
            'Atualiza as datas sugeridas nos seletores de datas
            Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")

        End If
    End Sub

    Sub CARREGAR_DADOS_UHES_NOVO_FORMATOSOMAR()
        'Rotina de atualizaçãp de dados das estações das UHES em formato tripa
        total_estações = DataGridView1.Rows.Count - 1

        'Libera a seleção de multiplos arquivos
        ofd.Multiselect = True
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "ASCII com dados horários dispostos em colunas(.uhe)|*.uhe"
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

                'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                ' cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)).Substring(0, 8)
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p))

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = Estação(po) Then
                        CodigoLocalizado = po
                    End If
                Next

                'Caso o código for maior que zero, executa a rotina de ler o arquivo 
                If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                    'Cria a variável linha4, que contém a leitura da linha do arquivo
                    Dim linha4 As String
                    'Cria a variável de tabulação splt
                    Dim splt

                    FileOpen(4, Arqu(p), OpenMode.Input)

                    Dim contador As Integer
                    contador = -1
                    Do Until EOF(4)
                        linha4 = LineInput(4)
                        splt = Split(linha4, "	")
                        contador += 1
                        If contador = 0 Then Continue Do

                        Qsub(CodigoLocalizado, splt(3), splt(0), splt(1), Ano(splt(2))) = AjustarDecimal(splt(4))
                        'MsgBox(splt(3) & " " & splt(0) & " " & splt(1) & " " & splt(2) & " " & splt(4))

                        If splt(0) & "/" & splt(1) & "/" & splt(2) & " " & splt(3) & ":00:00" < dataIp(CodigoLocalizado) Then
                            dataIp(CodigoLocalizado) = splt(0) & "/" & splt(1) & "/" & splt(2) & " " & splt(3) & ":00:00"
                            If dataIp(CodigoLocalizado) < dataI Then
                                dataI = dataIp(CodigoLocalizado)
                            End If
                        End If

                        If splt(0) & "/" & splt(1) & "/" & splt(2) & " " & splt(3) & ":00:00" > dataFp(CodigoLocalizado) Then
                            dataFp(CodigoLocalizado) = splt(0) & "/" & splt(1) & "/" & splt(2) & " " & splt(3) & ":00:00"
                            If dataFp(CodigoLocalizado) > dataF Then
                                dataF = dataFp(CodigoLocalizado)
                            End If
                        End If

                        'Depois de verificar as datas, vai para o próximo arquivo aberto

                    Loop

                    'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                    FileClose(4)

                End If

            Next

            'Depois que percorreu todos os arquivos selecionados na caixa de diálogo

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            Me.ATUALIZA_ARQUIVOS_QSUBST_TODOS()
            'Atualiza as datas sugeridas nos seletores de datas
            Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")

        End If
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
            CodigoLocalizado = Codigo 'DataGridView1.CurrentCell.RowIndex + 1

            'Caso o código for maior que zero, executa a rotina de ler o arquivo 
            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo
                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha4 As String
                'Abre o arquivo
                FileOpen(4, Arqu(p), OpenMode.Input)

                Dim contador As Integer
                contador = 0

                'Começa a leitura até o final, linha por linha
                Do Until EOF(4)
                    'linha4 contém a informação da linha
                    linha4 = LineInput(4)
                    contador += 1
                    'MsgBox(linha4)
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
                        If CodigoLocalizado = 1 Then 'se for Garibaldi defluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(52, 6)
                            'MsgBox(horae & " " & diae & " " & Mêse & " " & Anoe & " " & Valore)

                        ElseIf CodigoLocalizado = 2 Then 'se for Garibaldi afluente
                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(31, 6)
                            'MsgBox(horae & " " & diae & " " & Mêse & " " & Anoe & " " & Valore)
                            'MsgBox(Valore)
                        End If

                    Catch ex As Exception
                        'FileClose(4)
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

                    'If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                    '    Dim data_c As Date
                    '    data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                    '    data_c = data_c.AddHours(1)
                    '    horae = data_c.Hour
                    '    diae = data_c.Day
                    '    Mêse = data_c.Month
                    '    Anoe = data_c.Year
                    'End If


                    ''Corrigir lajeado, cana brava e estreito com mais três horas, pois os dados são utc
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

                    'Se a vazão for zero, diz que é falha
                    If Valore = 0 Then Valore = -1.0

                    'Coloca o valor para a variável de vazão
                    Qsub(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = Valore

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

        Next

        'Depois que percorreu todos os arquivos selecionados na caixa de diálogo

        'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
        Me.AtualizaDataTabela()
        'Atualiza o arquivo de vazão substituída do database
        Me.ATUALIZA_ARQUIVOS_QSUBST()
        'Atualiza as datas sugeridas nos seletores de datas
        Me.ATUALIZA_DATA()
        'Atualiza as colunas de vazão no horizonte
        Me.ATUALIZA_VAZAO_HORIZONTE()
        'Exibe mensagem de atualização bem sucedida
        ' MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")
        stlb.Text = "Atualização automática finalizada com sucesso!"



    End Sub

    Sub AtualizaDataTabela()
        For p As Integer = 1 To total_estações
            DataGridView1.Item(1, p - 1).Value = dataIp(p)
            DataGridView1.Item(2, p - 1).Value = dataFp(p)
        Next
    End Sub

    Sub ATUALIZA_ARQUIVOS_QSUBST()
        Dim p As Integer
        p = DataGridView1.CurrentCell.RowIndex + 1

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
        'tmaxx = dif. de tempo

        Dim datacomeço As Date
        datacomeço = dataIp(p)
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\" & Estação(p) & ".txt"

        FileOpen(2, arquivo_est2, OpenMode.Output)

        Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho

        'escreve o arquivo no cabeçalho
        For espaço As Integer = 1 To 12 - Estação(p).Length
            'primeiro esquece os espaços
            Print(2, " ")
        Next
        'escreve o nome do arquivo
        Print(2, Estação(p))
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

            If FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                Print(2, "        " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                Print(2, "       " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                Print(2, "      " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                Print(2, "     " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                Print(2, "    " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                Print(2, "   " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                'não diminui dois espaços pois não tem o separador de milhar

            End If

            PrintLine(2)

        Next

        FileClose()


    End Sub

    Sub ATUALIZA_ARQUIVOS_QSUBST_TODOS()
        'Dim p As Integer
        ' p = DataGridView1.CurrentCell.RowIndex + 1

        Dim tmaxx As Integer

        For p As Integer = 1 To total_estações

            tmaxx = (DateDiff(DateInterval.Hour, dataIp(p), dataFp(p)) + 1)
            'tmaxx = dif. de tempo

            Dim datacomeço As Date
            datacomeço = dataIp(p)
            'datacomeço.AddDays(1)

            'define o arquivo
            Dim arquivo_est2 As String
            arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\" & Estação(p) & ".txt"

            FileOpen(2, arquivo_est2, OpenMode.Output)

            Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho
            'escreve o arquivo no cabeçalho
            Print(2, "      " & Estação(p))
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

                If FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                    Print(2, "       " & "-1.000000")
                ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                    Print(2, "        " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                    Print(2, "       " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                    Print(2, "      " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                    Print(2, "     " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
                ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                    Print(2, "    " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                ElseIf FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                    Print(2, "   " & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                    'não diminui dois espaços pois não tem o separador de milhar

                End If

                PrintLine(2)

            Next

            FileClose()
        Next

    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        ReDim Q(total_estações, 24, 31, 12, Ano(AF))

        'Zera as variáveis de vazão e nível
        For p As Integer = 0 To total_estações
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Q(p, hh, dd, mm, Ano(aaaa)) = Qsub(p, hh, dd, mm, Ano(aaaa))
                        Next
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        'Define que todos os outros tipos de gráfico são falsos
        Resultado = False
        VisuResultado = False
        ChuvaSUBS = False
        Pluviograma = False
        Padrão = False

        Hidrograma = True
        Cotagrama = False

        'Define qual estação vamos mostrar o gráfico
        Estação_GrafQ = DataGridView1.CurrentCell.RowIndex + 1

        'Chama o formulário com os gráficos
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
            posto(contaarq) = Estação(contaarq)

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
    End Sub 'Gera o shapefile dos postos, por procedimento padrão

    Private Sub Button5_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button5.Click
        ' ReDim Q(total_estações, 24, 31, 12, Ano(AF))
        ReDim Qdispo(total_estações, 24, 31, 12, Ano(AF))

        'Zera as variáveis de vazão e nível
        For p As Integer = 0 To total_estações
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Qdispo(p, hh, dd, mm, Ano(aaaa)) = Qsub(p, hh, dd, mm, Ano(aaaa))
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
    End Sub 'Faz o gráfico de disponibilidade temporal, rotina padrão

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        Me.ABERTA_BOTAO_EXPORTAR()
        MsgBox("Dados exportados com sucesso!", MsgBoxStyle.Information, "Previsão")
        Me.Close()
    End Sub

    Sub ABERTA_BOTAO_EXPORTAR()

        Me.ATUALIZA_EDITADOS_MANUALMENTE() 'atualiza os dados editados manualmente
        'Salva os arquivos de qsubs
        Me.SALVA_QSUBS_AUTOMATICO()
        'Me.SALVA_QSUBS_MANUALMENTE()

        Qsub = Nothing
        Erase Qsub

    End Sub

    Sub ATUALIZA_VAZAO_HORIZONTE()
        'Redimensiona as variáveis
        ReDim Q_hori(total_estações)
        ReDim Q_inic(total_estações)
        '--

        'Faz um loop que pega o valor mais recente de vazão das UHEs
        For ps As Integer = 1 To total_estações
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            If Qsub(ps, hh, dd, mm, Ano(aaaa)) <> -1.0 Then
                                Q_inic(ps) = Qsub(ps, hh, dd, mm, Ano(aaaa))
                                Q_hori(ps) = Qsub(ps, hh, dd, mm, Ano(aaaa))
                            End If
                        Next
                    Next
                Next
            Next
            DataGridView1.Item(6, ps - 1).Value = Q_inic(ps)
            DataGridView1.Item(7, ps - 1).Value = Q_hori(ps)
        Next
        'Zerou as variáveis

        'Faz um loop extra, que percorre todas as estações, e coloca -1.00 caso os dados sejam aflu
        'Como os dados aflu sempre estão em uma posição par, coloca -1 sempre na posição par
        For ps As Integer = 1 To total_estações
            If ps Mod 2 = 0 Then
                DataGridView1.Item(6, ps - 1).Value = -1.0
                DataGridView1.Item(7, ps - 1).Value = -1.0
            End If
        Next
        '-Colocou -1.0 nos dados pares

    End Sub

    Sub EXTRAPOLA_HORIZONTE()

        ReDim Q_ext(total_estações, 24, 31, 12, Ano(AF))

        'Zera as variáveis de vazão e nível
        For p As Integer = 0 To total_estações
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Q_ext(p, hh, dd, mm, Ano(aaaa)) = Qsub(p, hh, dd, mm, Ano(aaaa))
                        Next
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        Dim Q_extini(total_estações) As Double
        Dim Q_extfim(total_estações) As Double

        For variável As Integer = 1 To total_estações
            Q_extini(variável) = AjustarDecimal(DataGridView1.Item(6, variável - 1).Value)
            Q_extfim(variável) = AjustarDecimal(DataGridView1.Item(7, variável - 1).Value)
        Next

        For p As Integer = 1 To total_estações
            'Calcula datas
            Dim dataO As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"
            Dim dataE As Date = dtpqob3.Value.Day & "/" & dtpqob3.Value.Month & "/" & dtpqob3.Value.Year & " " & tb_hora_ext.Text & ":00:00"

            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
            'tmaxx = dif. de tempo

            Dim datacomeço As Date
            datacomeço = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"
            'datacomeço.AddDays(1)

            Dim k As Double

            If Me.rb_linear.Checked = True Then
                k = (Q_extfim(p) - Q_extini(p)) / tmaxx
                'Q_ext = Q_extini(p) + k*t
            ElseIf Me.rb_exponencial.Checked = True Then
                k = Math.Log((Q_extfim(p) / Q_extini(p))) / (-tmaxx)
                'Q_ext = Q_extini(p)*math.exp(-k*t)
            ElseIf Me.rb_log.Checked = True Then
                k = (Q_extfim(p) - Q_extini(p)) / Math.Log(tmaxx)
                'Q_ext = k*math.log(t) + Q_extini(p)
            End If

            For aux As Integer = 1 To tmaxx
                If aux <> 1 Then
                    datacomeço = datacomeço.AddHours(1)
                End If


                If Me.rb_linear.Checked = True Then
                    'k = (Q_extfim(p) - Q_extini(p)) / tmaxx
                    'Q_ext = 
                    Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = Q_extini(p) + k * aux
                ElseIf Me.rb_exponencial.Checked = True Then
                    'k = Math.Log(Q_extfim(p) / Q_extini(p)) / (-tmaxx)
                    'Q_ext = Q_extini(p)*math.exp(-k*t)
                    Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = Q_extini(p) * Math.Exp(-k * aux)
                ElseIf Me.rb_log.Checked = True Then
                    'k = (Q_extfim(p) - Q_extini(p)) / Math.Log(tmaxx)
                    'Q_ext = k*math.log(t) + Q_extini(p)
                    Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = k * Math.Log(aux) + Q_extini(p)
                End If

                'Novo IF para incluir arquivo de operação colocado em 18Nov2012
                'Caso o usuário possua a vazão de operação dada com entrada através de um arquivo, utiliza as vazões do arquivo
                If Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then
                    Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
                End If

                'Caso o usuário possua a vazão do MGB nesta data, ele usa a vazão real.
                If Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then
                    Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
                End If

            Next

        Next

    End Sub

    Sub ESCREVEDADOVAZÃO_HORA()

        For p As Integer = 1 To total_estações
            'Calcula datas
            Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            Dim dataE As Date = dtpqob3.Value.Day & "/" & dtpqob3.Value.Month & "/" & dtpqob3.Value.Year & " " & tb_hora_ext.Text & ":00:00"

            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
            'tmaxx = dif. de tempo

            Dim datacomeço As Date
            datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
            'datacomeço.AddDays(1)

            'define o arquivo
            Dim arquivo_est2 As String
            arquivo_est2 = fbd.SelectedPath & "\" & Estação(p) & ".txt"

            'Dim arquivo_est3 As String
            'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

            FileOpen(2, arquivo_est2, OpenMode.Output)
            'FileOpen(3, arquivo_est3, OpenMode.Output)

            Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho

            'escreve o arquivo no cabeçalho
            For espaço As Integer = 1 To 12 - Estação(p).Length
                'primeiro esquece os espaços
                Print(2, " ")
            Next
            'escreve o nome do arquivo
            Print(2, Estação(p))
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

                'If FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                '    Print(2, "       " & "-1.000000")
                'ElseIf FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                '    Print(2, "        " & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                '    Print(2, "       " & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                '    Print(2, "      " & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                'ElseIf FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                '    Print(2, "     " & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
                'ElseIf FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                '    Print(2, "    " & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                'ElseIf FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                '    Print(2, "   " & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                '    'não diminui dois espaços pois não tem o separador de milhar
                'End If


                If SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                    Print(2, "       " & "-1.000000")
                ElseIf SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                    Print(2, "        " & SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                    Print(2, "       " & SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                    Print(2, "      " & SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                    Print(2, "     " & SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                    Print(2, "    " & SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                    Print(2, "   " & SubstituirSeparador(Q_ext(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
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
        Dim dataE As Date = dtpqob3.Value.Day & "/" & dtpqob3.Value.Month & "/" & dtpqob3.Value.Year & " " & tb_hora_ext.Text & ":00:00"

        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)


        'tmaxx = dif. de tempo
        Dim datacomeço As Date
        datacomeço = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = fbd.SelectedPath & "\QSUBS.MGB"

        'Re-cria os arquivos com os mesmos limites
        FileOpen(2, arquivo_est2, OpenMode.Output)

        Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho
        For cabeçalho As Integer = 1 To total_estações Step 2 'para cada um dos arquivos de vazão selecionados
            'Verifica se é vazão observada
            'escreve o arquivo no cabeçalho
            For espaço As Integer = 1 To 28 - Estação(cabeçalho).ToString.Length
                'primeiro esquece os espaços
                Print(2, " ")

            Next
            'escreve o nome do arquivo
            Print(2, Estação(cabeçalho))
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
            For variável As Integer = 1 To total_estações Step 2

                'If FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 0 Then
                '    Print(2, "        " & "-1.0")
                'ElseIf FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 0 And FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 10 Then
                '    Print(2, "        " & "0" & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1))
                'ElseIf FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 10 And FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 100 Then
                '    Print(2, "        " & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1))
                'ElseIf FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 100 And FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 1000 Then
                '    Print(2, "       " & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1))
                'ElseIf FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 1000 And FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 10000 Then
                '    Print(2, "      " & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(0, 1) & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(2, 5))
                'ElseIf FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 10000 And FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 100000 Then
                '    Print(2, "     " & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(0, 2) & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(3, 5))
                'ElseIf FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) >= 100000 And FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1) < 1000000 Then
                '    Print(2, "    " & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(0, 3) & FormatNumber(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 1).Substring(4, 5))
                '    'não diminui dois espaços pois não tem o separador de milhar
                'End If

                If SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                    Print(2, "        " & "-1.0")
                ElseIf SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                    Print(2, "        " & "0" & SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                    Print(2, "        " & SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                    Print(2, "       " & SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                    Print(2, "      " & SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                    Print(2, "     " & SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                ElseIf SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                    Print(2, "    " & SubstituirSeparador1Dig(Q_ext(variável, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))

                    'não diminui dois espaços pois não tem o separador de milhar

                End If



            Next
            PrintLine(2)

        Next

        FileClose()


    End Sub

    Private Sub Button7_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button7.Click
        Dim arquivo_est2 As String
        Dim num As Integer
        num = DataGridView1.CurrentCell.RowIndex + 1
        arquivo_est2 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\" & Estação(num) & ".txt"
        'System.Diagnostics.Process.Start("notepad.exe", arquivo_est2)


        ReDim Tabela(24, 31, 12, Ano(AF))

        Tb_chuva = False
        Tb_vazao = True
        Nome_arq = arquivo_est2
        Nome_p = Estação(num)

        For aaaa = AI To AF
            For mm = 1 To 12
                For dd = 1 To 31
                    For hh As Integer = 0 To 23
                        Tabela(hh, dd, mm, Ano(aaaa)) = Qsub(num, hh, dd, mm, Ano(aaaa))
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        dataI = dataIp(num)
        dataF = dataFp(num)

        Dim frmedit As New Form9
        frmedit.Show()


    End Sub 'Chama o formulário de edição manual, com procedimento padrão

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
            If total_estações = 1 Then
                diretorio_arquivos = linha
            ElseIf total_estações = 4 Then
                diretorio_previsao = linha
            ElseIf total_estações = 5 Then
                DIR_ESTREITO = linha
            End If
        Loop

        stlb.Text = "Diretório arquivos de previsão: " & diretorio_previsao
        'Fecha
        FileClose()

    End Sub 'Verifica o diretório onde devem ficar salvos os arquivos de previsão

    Sub SALVA_QSUBS_MANUALMENTE()
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            statusbar.Value = 0
            statusbar.Maximum = 3

            Me.EXTRAPOLA_HORIZONTE()

            statusbar.Increment(1)

            Me.ESCREVEDADOVAZÃO_HORA()

            statusbar.Increment(1)

            Me.TRANSCREVEQOBS_HORA2()
            statusbar.Increment(1)

            MsgBox("Dados exportados com sucesso!", MsgBoxStyle.Information, "Previsão")

        End If
    End Sub

    Sub SALVA_QSUBS_AUTOMATICO()

        fbd.SelectedPath = diretorio_previsao
        statusbar.Value = 0
        statusbar.Maximum = 3

        Me.EXTRAPOLA_HORIZONTE()

        statusbar.Increment(1)

        Me.ESCREVEDADOVAZÃO_HORA()

        statusbar.Increment(1)

        Me.TRANSCREVEQOBS_HORA2()
        statusbar.Increment(1)

    End Sub

    Sub Carrega_Hora_Previsão() 'tenta carregar a hora final da previsão
        Try

            ofd.FileName = diretorio_previsao & "\PrevisaoMINI_Estreito.bin"
            'Pega o nome do arquivo com dados de chuva prevista

            'diretório do arquivo onde está salvo o arquivo de previsão
            Dim diretório As String
            diretório = System.IO.Path.GetDirectoryName(ofd.FileName)

            'nome do arquivo meta dados, que está no mesmo diretório do arquivo de previsão
            Dim arquivo_metadados As String
            'arquivo padrão com nome dado pelo Siqueira:
            arquivo_metadados = diretório & "\Previsao.meta"

            'abre o arquivo metadados
            FileOpen(1, arquivo_metadados, OpenMode.Input)

            'Declara as variáveis que correspondem às datas
            Dim datainiciopprev As Date
            Dim datafimpprev As Date

            'Declara a variável que contém a contagem de linhas e a variável que contém a linha
            Dim ll As Integer
            ll = 0
            Dim linha As String

            'Lê o arquivo linha por linha, até o final
            Do Until EOF(1)
                linha = LineInput(1)
                ll = ll + 1
                'Caso e linha for a linha 10, que contém as informações de data
                If ll = 10 Then
                    'pega a data inicial
                    datainiciopprev = linha.Substring(0, 4) & "/" & linha.Substring(8, 2) & "/" & linha.Substring(14, 2) & " " & linha.Substring(20, 2) & ":00:00"

                    'adiciona as horas correspondentes aos intervalos de tempo
                    Dim addhour As Integer
                    addhour = linha.Substring(22, 8)
                    addhour = addhour - 1
                    'a data final é a data inicial mais as horas que foram adicionadas
                    datafimpprev = datainiciopprev.AddHours(addhour)
                End If
            Loop

            'define a data final da previsão
            dtpqob3.Value = datafimpprev

            'define a hora final da previsão, acertando para que a hora tenha 2 dígitos
            Dim hora As String
            hora = datafimpprev.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If

            'define a hora na janelinha de hora
            tb_hora_ext.Text = hora

            'fecha o arquivo
            FileClose()

        Catch ex As Exception
            'caso não conseguir, simplesmente não faz nada
        End Try

    End Sub

    Sub ATUALIZA_EDITADOS_MANUALMENTE()
        Try
            'Verifica quantas estações existem na base de dados interna
            Dim arquivo_est As String 'nome do arquivo que contém as estações
            Dim linha As String 'linha de leitura
            total_estações = 0 'número de estações

            'define o arquivo
            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesUHEs.txt"

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
            'ReDim Qsub(total_estações, 24, 31, 12, Ano(AF))
            'ReDim Q_inic(total_estações)
            'ReDim Q_hori(total_estações)
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
                                'Qsub(p, hh, dd, mm, Ano(aaaa)) = -1.0
                                'Q_inic(p) = -1.0
                                'Q_hori(p) = -1.0
                            Next
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis


            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesUHEs.txt"
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
                    LAT(est) = splt(2)
                    LON(est) = splt(3)
                    ALT(est) = splt(4)
                    NOME(est) = splt(5)
                    A(est) = splt(6)
                    B(est) = splt(7)
                    C(est) = splt(8)


                    Me.LE_DATABASE(est, Estação(est))

                    'Adiciona os dados à tabela

                    'Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

                    ' Preenche os valores do registro
                    'nova_linha.ItemArray = New Object() {Estação(est), dataIp(est), dataFp(est), LON(est), LAT(est), NOME(est), "", ""}
                    'Try
                    ' DataSet1.Tables(0).Rows.Add(nova_linha)
                    ' Catch ex As Exception
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
        Me.ATUALIZA_EDITADOS_MANUALMENTE() 'Faz a leitura do database
        Me.AtualizaDataTabela() 'atualiza as datas na tabela
        Me.ATUALIZA_VAZAO_HORIZONTE() 'atualiza as vazões no horizonte de previsão
    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        Me.APERTA_BOTA_SOMENTE_DADOS_OBS()
    End Sub

    Sub APERTA_BOTA_SOMENTE_DADOS_OBS()
        'Faz um loop extra, que percorre todas as estações, e coloca -1.00 
        For ps As Integer = 1 To total_estações
            DataGridView1.Item(6, ps - 1).Value = -1.0
            DataGridView1.Item(7, ps - 1).Value = -1.0
        Next
        '-Colocou -1.0 nos dados
    End Sub

    Private Sub DataGridView1_CellContentClick(ByVal sender As System.Object, ByVal e As System.Windows.Forms.DataGridViewCellEventArgs) Handles DataGridView1.CellContentClick

    End Sub

    Private Sub Button9_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button9.Click
        DOWNLOAD_UHES()
        MsgBox("Download dos dados das UHEs finalizados. Arquivos salvos na pasta " & diretorio_arquivos)
        CONVERTE_ARQUIVOS()
        MsgBox("Arquivos das UHEs convertidos. Arquivos salvos na pasta " & diretorio_arquivos)

    End Sub

    Sub DOWNLOAD_UHES()
        'Esta rotina faz um download dos dados das UHE's nos servidores ftp

        'Verifica quantas estações existem na base de dados interna
        ' ''Dim arquivo_est As String 'nome do arquivo que contém as estações
        ' ''Dim linha As String 'linha de leitura
        ' ''total_estações = 0 'número de estações
        '' ''define o arquivo
        ' ''arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\EstacoesUHEs.txt"
        '' ''Abre e faz a leituara
        ' ''FileOpen(1, arquivo_est, OpenMode.Input)
        ' ''Do Until EOF(1)
        ' ''    linha = LineInput(1)
        ' ''    total_estações = total_estações + 1
        ' ''Loop
        ' ''FileClose()
        ' ''total_estações = total_estações - 1 'desconta a primeira linha, que é cabeçalho

        total_estações = DataGridView1.Rows.Count - 1

        'Para cada um dos arquivos na tabela, com step 2 por que os arquivos repetem
        For est As Integer = 1 To total_estações Step 2

            Try
                'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
                DownloadViaFTP(Endereço(est), Usuário(est), Senha(est))
            Catch ex As Exception
                MsgBox("Ocorreu um erro no download dos dados da usina " & " " & Estação(est), MsgBoxStyle.Critical, "Aviso de erro")
                MsgBox(ex.Message)
            End Try



        Next



    End Sub

    Private Sub DownloadViaFTP(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação
        Dim readBuffer(4096) As Byte
        Dim contador As Integer
        Dim arquivoRequisitado As FtpWebRequest
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing
        Dim pastaDestino As String

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        ' ----- Obtem local onde irá salvar o arquivo
        'pastaDestino = My.Computer.FileSystem.CombinePath(pasta_exec, My.Computer.FileSystem.GetName(arquivoFonte))
        pastaDestino = diretorio_arquivos & "\" & My.Computer.FileSystem.GetName(arquivoFonte)

        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)
            arquivoRequisitado.UsePassive = True
            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0

            'If CPTEC = False Then
            System.IO.File.Copy(pastaDestino, diretorio_previsao & "\" & System.IO.Path.GetFileName(pastaDestino), True) 'copia o arquivo para a outra pasta de arquivos também
            'MsgBox("Download finalizado! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
            'End If

        Catch ex As Exception

            MsgBox("Erro durante a realização do FTP " + vbCrLf + ex.Message)

            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()



    End Sub

    Sub CONVERTE_ARQUIVOS() 'Converte os arquivos baixados para o formato coluna

        total_estações = DataGridView1.Rows.Count - 1

        'Log dos dados das usinas
        Dim Arquivo_LOG As String
        Arquivo_LOG = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\LOG_UHEs.txt"
        FileOpen(10, Arquivo_LOG, OpenMode.Output)
        Dim ContadorErros As Integer
        ContadorErros = 0


        'Para cada um dos arquivos na tabela
        For est As Integer = 1 To total_estações


            Try

                ' ----- Obtem local onde está o arquivo
                Dim ArquivoNoPC As String
                ArquivoNoPC = diretorio_arquivos & "\" & Arquivo(est)

                ' ---- Arquivo convertido para colunas
                Dim ArquivoSalvo As String
                ArquivoSalvo = diretorio_arquivos & "\" & Estação(est) & ".uhe"

                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha4 As String

                'Abre o arquivo
                FileOpen(4, ArquivoNoPC, OpenMode.Input)

                'Abre o arquivo de saída
                FileOpen(5, ArquivoSalvo, OpenMode.Output)
                'Escreve o começo do cabeçalho
                Print(5, "   Dia   Mês   Ano  Hora")
                'escreve o arquivo no cabeçalho
                Print(5, "      " & Estação(est))
                PrintLine(5) 'finaliza a linha do cabeçalho

                'Começa a leitura até o final, linha por linha
                Do Until EOF(4)
                    'linha4 contém a informação da linha
                    linha4 = LineInput(4)

                    'Cria estas variáveis, que contém as informações de hora, dia, mes ano e vazão que estamos lendo
                    Dim horae As Integer
                    Dim diae As Integer
                    Dim Mêse As Integer
                    Dim Anoe As Integer
                    Dim Valore As Single 'Vazão Defluente, que normalmente é substituida

                    horae = linha4.Substring(9, 2)
                    diae = linha4.Substring(6, 2)
                    Mêse = linha4.Substring(4, 2)
                    Anoe = linha4.Substring(0, 4)
                    Valore = linha4.Substring(ColunaNoArquivo(est), Ncaracteres(est))

                    If Valore = 0 Then
                        Valore = -1.0
                    End If

                    If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                        Dim data_c As Date
                        data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                        data_c = data_c.AddHours(1)
                        horae = data_c.Hour
                        diae = data_c.Day
                        Mêse = data_c.Month
                        Anoe = data_c.Year
                    End If

                    'Corrigir os dados que são utc
                    ' ''If CodigoLocalizado <= 4 Then
                    ' ''    Dim data_c As Date
                    ' ''    data_c = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                    ' ''    data_c = data_c.AddHours(3)
                    ' ''    horae = data_c.Hour
                    ' ''    diae = data_c.Day
                    ' ''    Mêse = data_c.Month
                    ' ''    Anoe = data_c.Year
                    ' ''End If


                    If diae < 10 Then
                        Print(5, "    " & "0" & diae)
                    ElseIf diae >= 10 Then
                        Print(5, "    " & diae)
                    End If
                    If Mêse < 10 Then
                        Print(5, "    " & "0" & Mêse)
                    ElseIf Mêse >= 10 Then
                        Print(5, "    " & Mêse)
                    End If

                    Print(5, "  " & Anoe)

                    If horae.ToString.Length = 1 Then
                        Print(5, "    " & "0" & horae.ToString)
                    ElseIf horae.ToString.Length = 2 Then
                        Print(5, "    " & horae.ToString)
                    End If
                    'Escreveu a data

                    'Agora vai escrever os números
                    Print(5, SubstituirSeparador(FormatNumber(Valore, 6)).PadLeft(12) & "0000")
                    PrintLine(5)

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)
                FileClose(5)



            Catch ex As Exception
                FileClose(4)
                FileClose(5)
                PrintLine(10, "Ocorreu um erro na leitura dos dados da usina " & " " & Estação(est))
                PrintLine(10, ex.Message)
                ContadorErros += 1

                'MsgBox("Ocorreu um erro na leitura dos dados da usina " & " " & Estação(est), MsgBoxStyle.Critical, "Aviso de erro")
                'MsgBox(ex.Message)

            End Try



        Next


        If ContadorErros > 0 Then
            Process.Start(Arquivo_LOG)
        End If
        FileClose()

    End Sub

    Sub CONVERTE_ARQUIVOS_OLD() 'Converte os arquivos baixados para o formato coluna

        total_estações = DataGridView1.Rows.Count - 1

        'Para cada um dos arquivos na tabela
        For est As Integer = 1 To total_estações


            Try

                ' ----- Obtem local onde está o arquivo
                Dim ArquivoNoPC As String
                ArquivoNoPC = diretorio_arquivos & "\" & Arquivo(est)

                ' ---- Arquivo convertido para colunas
                Dim ArquivoSalvo As String
                ArquivoSalvo = diretorio_arquivos & "\" & Estação(est) & ".uhe"

                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha4 As String

                'Abre o arquivo
                FileOpen(4, ArquivoNoPC, OpenMode.Input)

                'Abre o arquivo de saída
                FileOpen(5, ArquivoSalvo, OpenMode.Output)
                'Escreve o começo do cabeçalho
                Print(5, "   Dia   Mês   Ano  Hora")
                'escreve o arquivo no cabeçalho
                Print(5, "      " & Estação(est))
                PrintLine(5) 'finaliza a linha do cabeçalho

                'Começa a leitura até o final, linha por linha
                Do Until EOF(4)
                    'linha4 contém a informação da linha
                    linha4 = LineInput(4)

                    'Cria estas variáveis, que contém as informações de hora, dia, mes ano e vazão que estamos lendo
                    Dim horae As Integer
                    Dim diae As Integer
                    Dim Mêse As Integer
                    Dim Anoe As Integer
                    Dim Valore As Single 'Vazão Defluente, que normalmente é substituida

                    horae = linha4.Substring(9, 2)
                    diae = linha4.Substring(6, 2)
                    Mêse = linha4.Substring(4, 2)
                    Anoe = linha4.Substring(0, 4)
                    Valore = linha4.Substring(ColunaNoArquivo(est), Ncaracteres(est))

                    If Valore = 0 Then
                        Valore = -1.0
                    End If

                    If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                        Dim data_c As Date
                        data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                        data_c = data_c.AddHours(1)
                        horae = data_c.Hour
                        diae = data_c.Day
                        Mêse = data_c.Month
                        Anoe = data_c.Year
                    End If

                    'Corrigir os dados que são utc
                    ' ''If CodigoLocalizado <= 4 Then
                    ' ''    Dim data_c As Date
                    ' ''    data_c = diae & "/" & Mêse & "/" & Anoe & " " & horae & ":00:00"
                    ' ''    data_c = data_c.AddHours(3)
                    ' ''    horae = data_c.Hour
                    ' ''    diae = data_c.Day
                    ' ''    Mêse = data_c.Month
                    ' ''    Anoe = data_c.Year
                    ' ''End If


                    If diae < 10 Then
                        Print(5, "    " & "0" & diae)
                    ElseIf diae >= 10 Then
                        Print(5, "    " & diae)
                    End If
                    If Mêse < 10 Then
                        Print(5, "    " & "0" & Mêse)
                    ElseIf Mêse >= 10 Then
                        Print(5, "    " & Mêse)
                    End If

                    Print(5, "  " & Anoe)

                    If horae.ToString.Length = 1 Then
                        Print(5, "    " & "0" & horae.ToString)
                    ElseIf horae.ToString.Length = 2 Then
                        Print(5, "    " & horae.ToString)
                    End If
                    'Escreveu a data

                    'Agora vai escrever os números
                    Print(5, SubstituirSeparador(FormatNumber(Valore, 6)).PadLeft(12) & "0000")
                    PrintLine(5)

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)
                FileClose(5)



            Catch ex As Exception

                MsgBox("Ocorreu um erro na leitura dos dados da usina " & " " & Estação(est), MsgBoxStyle.Critical, "Aviso de erro")
                MsgBox(ex.Message)

            End Try



        Next

    End Sub

    Private Sub Button10_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button10.Click
        'Este botão serve para carregar um arquivo tripa com uma coluna de dados de operação da usina

        'Posteriormente, na rotina de extrapolação de dados, os dados extrapolados são substituídos pelos dados operativos
        'Isto é feito se o dado de operação é diferente de falha

        'Tranca a seleção de multiplos arquivos
        ofd.Multiselect = False
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "Arquivo ASCII com dados horários de operação dispostos em uma única coluna(.txt;.prn)|*.txt;*.prn"
        'abre a janela de carregar arquivos, e só continua se tudo estiver OK
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then

            'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
            Dim CodigoLocalizado As Integer
            'Pega na tabela qual posto estamos utilizando
            'O +1 no final é utilizado pois a contagem da tabela começa em zero
            CodigoLocalizado = DataGridView1.CurrentCell.RowIndex + 1

            'Define que estamos atualizando o posto selecionado
            Dim p As Integer
            p = CodigoLocalizado

            'Zera a variável para ser igual ao dados de vazão da usina
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Q_opera(p, hh, dd, mm, Ano(aaaa)) = Qsub(p, hh, dd, mm, Ano(aaaa))
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis

            'Calcula datas com base nas datas informadas
            Dim dataO As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"
            Dim dataE As Date = dtpqob3.Value.Day & "/" & dtpqob3.Value.Month & "/" & dtpqob3.Value.Year & " " & tb_hora_ext.Text & ":00:00"

            'Número de intervalos de tempo
            Dim tmaxx As Integer
            tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
            'tmaxx = dif. de tempo

            'Data inicial do modelo
            Dim datacomeço As Date
            datacomeço = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

            'Abre o arquivo
            FileOpen(5, ofd.FileName, OpenMode.Input)

            'Variáveis auxiliares
            Dim aux As Integer
            aux = 0 'inicializa a variável
            Dim linha As String

            'Executa um loop que passa por todos
            Do Until (EOF(5))
                linha = LineInput(5) 'leitura
                aux = aux + 1 'contador

                ' If aux <> 1 Then
                'Adiciona uma hora
                datacomeço = datacomeço.AddHours(1)
                ' End If

                'Faz a leitura do dado de operação
                Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = linha

                'Caso o usuário possua a vazão do MGB nesta data, ele usa a vazão real.
                If Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then
                    Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
                End If

                'Atualiza a vazão no horizonte
                Q_hori(p) = Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))

            Loop

            'Atualiza o datagrid
            DataGridView1.Item(7, p - 1).Value = Q_hori(p)

            'Fecha o arquivo
            FileClose()

            'Finaliza
            MsgBox("Dados de operação da UHE carregados com sucesso.", MsgBoxStyle.Information, "MGB-IPH")

        End If
        'Fim da rotina
    End Sub

    Private Sub Button11_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button11.Click
        Me.OPERA_ARQUIVOTEXTO_DATADO()
    End Sub

    Sub OPERA_ARQUIVOTEXTO_DATADO()
        'Este botão serve para carregar um arquivo tripa com uma coluna de dados de operação da usina

        'Posteriormente, na rotina de extrapolação de dados, os dados extrapolados são substituídos pelos dados operativos
        'Isto é feito se o dado de operação é diferente de falha

        'Tranca a seleção de multiplos arquivos
        ofd.Multiselect = False
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "Arquivo ASCII com dados horários de operação separados por tabulação (.txt;.prn;.csv)|*.txt;*.prn;*.csv"
        'abre a janela de carregar arquivos, e só continua se tudo estiver OK
        If ofd.ShowDialog() = Windows.Forms.DialogResult.OK Then

            'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
            Dim CodigoLocalizado As Integer
            'Pega na tabela qual posto estamos utilizando
            'O +1 no final é utilizado pois a contagem da tabela começa em zero
            CodigoLocalizado = DataGridView1.CurrentCell.RowIndex + 1
            'Define que estamos atualizando o posto selecionado
            Dim p As Integer
            p = CodigoLocalizado

            'Zera a variável para ser igual ao dados de vazão da usina
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For hh As Integer = 0 To 23
                            Q_opera(p, hh, dd, mm, Ano(aaaa)) = Qsub(p, hh, dd, mm, Ano(aaaa))
                        Next
                    Next
                Next
            Next
            'Zerou as variáveis

            ''Data inicial do modelo
            Dim datacomeço As Date
            'datacomeço = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"

            'Abre o arquivo
            FileOpen(5, ofd.FileName, OpenMode.Input)

            'Variáveis auxiliares
            Dim aux As Integer
            aux = 0 'inicializa a variável
            Dim linha As String
            Dim splt

            'Executa um loop que passa por todos
            Do Until (EOF(5))
                linha = LineInput(5) 'leitura
                aux = aux + 1 'contador
                splt = Split(linha, ";") 'Separador


                ' If aux <> 1 Then
                'Adiciona uma hora
                'datacomeço = datacomeço.AddHours(1)
                datacomeço = linha.Substring(0, 6) & "/" & linha.Substring(6, 6) & "/" & linha.Substring(12, 6) & " " & linha.Substring(18, 6) & ":00:00"
                ''splt(0) & "/" & splt(1) & "/" & splt(2) & " " & splt(3) & ":00:00"
                ' End If

                'Faz a leitura do dado de operação
                Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = AjustarDecimal(linha.Substring(24, 12))

                'Caso o usuário possua a vazão do MGB nesta data, ele usa a vazão real.
                If Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) >= 0 Then
                    Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = Qsub(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
                End If

                'Atualiza a vazão no horizonte
                Q_hori(p) = Q_opera(p, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))

            Loop

            'Atualiza o datagrid
            DataGridView1.Item(7, p - 1).Value = Q_hori(p)

            'Finaliza
            MsgBox("Dados de operação da UHE carregados com sucesso.", MsgBoxStyle.Information, "MGB-IPH")

            'Fecha o arquivo
            FileClose(5)

        End If
        'Fim da rotina

    End Sub

    Private Sub Button12_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button12.Click


        If rb_PadrãoONS.Checked = True Then
            Me.CARREGAR_DADOS_UHES_NOVO_PADRAO_ONS()
        ElseIf rb_PadrãoMGB.Checked = True Then
            'CARREGAR_DADOS_UHES_NOVO()

            CARREGAR_DADOS_UHES_NOVO_FORMATOSOMAR()
        End If

    End Sub

    Sub CARREGAR_DADOS_UHES_NOVO_PADRAO_ONS()
        'Rotina de atualizaçãp de dados das estações das UHES em formato tripa
        total_estações = DataGridView1.Rows.Count - 1

        'Libera a seleção de multiplos arquivos
        ofd.Multiselect = True
        'coloca o filtro dos arquivos de vazão
        ofd.Filter = "Arquivo texto com dados horários dispostos em colunas(.txt)|*.txt"
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

                'Cria a variável que guarda a informação de qual posto da tabela estamos utilizando
                Dim CodigoLocalizado As Integer
                Dim cod_estação As String
                ' cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)).Substring(0, 8)
                cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) & ".txt"

                CodigoLocalizado = 0
                For po As Integer = 1 To total_estações
                    If cod_estação = Arquivo(po) Then
                        CodigoLocalizado = po

                        'Cria a variável linha4, que contém a leitura da linha do arquivo
                        Dim linha4 As String
                        FileOpen(4, Arqu(p), OpenMode.Input)

                        Dim contador As Integer
                        contador = -1
                        Do Until EOF(4)
                            linha4 = LineInput(4)

                            contador += 1
                            If contador = 0 Then Continue Do

                            'Cria estas variáveis, que contém as informações de hora, dia, mes ano e vazão que estamos lendo
                            Dim horae As Integer
                            Dim diae As Integer
                            Dim Mêse As Integer
                            Dim Anoe As Integer
                            Dim Valore As Single 'Vazão Defluente, que normalmente é substituida

                            horae = linha4.Substring(9, 2)
                            diae = linha4.Substring(6, 2)
                            Mêse = linha4.Substring(4, 2)
                            Anoe = linha4.Substring(0, 4)
                            Valore = linha4.Substring(ColunaNoArquivo(po), Ncaracteres(po))

                            If Valore = 0 Then
                                Valore = -1.0
                            End If

                            If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                                Dim data_c As Date
                                data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                                data_c = data_c.AddHours(1)
                                horae = data_c.Hour
                                diae = data_c.Day
                                Mêse = data_c.Month
                                Anoe = data_c.Year
                            End If

                            Qsub(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = AjustarDecimal(Valore)

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
                Next



            Next

            'Depois que percorreu todos os arquivos selecionados na caixa de diálogo

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            Me.ATUALIZA_ARQUIVOS_QSUBST_TODOS()
            'Atualiza as datas sugeridas nos seletores de datas
            Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            MsgBox("Atualização finalizada com sucesso!", MsgBoxStyle.Information, "Vazão UHEs")

        End If
    End Sub

    Sub CARREGAR_DADOS_UHES_NOVO_DE_UMA_PASTA_PADRAO_ONS()


        Dim f2() As String = Directory.GetFiles(DIR_ESTREITO) 'este é o diretório com os arquivos das UHEs, definido nas preferências


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

            cod_estação = System.IO.Path.GetFileNameWithoutExtension(Arqu(p)) & ".txt"

            CodigoLocalizado = 0
            For po As Integer = 1 To total_estações
                If cod_estação = Arquivo(po) Then
                    CodigoLocalizado = po
                    NumArquivosAtualizados += 1

                    'Cria a variável linha4, que contém a leitura da linha do arquivo
                    Dim linha4 As String
                    FileOpen(4, Arqu(p), OpenMode.Input)

                    Dim contador As Integer
                    contador = -1
                    Do Until EOF(4)
                        linha4 = LineInput(4)

                        contador += 1
                        If contador = 0 Then Continue Do

                        'Cria estas variáveis, que contém as informações de hora, dia, mes ano e vazão que estamos lendo
                        Dim horae As Integer
                        Dim diae As Integer
                        Dim Mêse As Integer
                        Dim Anoe As Integer
                        Dim Valore As Single 'Vazão Defluente, que normalmente é substituida

                        horae = linha4.Substring(9, 2)
                        diae = linha4.Substring(6, 2)
                        Mêse = linha4.Substring(4, 2)
                        Anoe = linha4.Substring(0, 4)
                        Valore = linha4.Substring(ColunaNoArquivo(po), Ncaracteres(po))

                        If Valore = 0 Then
                            Valore = -1.0
                        End If

                        If horae = 24 Then 'corrige o problema da hora 24, já que os arquivos da tractebel contém horas das 01 até as 24
                            Dim data_c As Date
                            data_c = diae & "/" & Mêse & "/" & Anoe & " " & 23 & ":00:00"
                            data_c = data_c.AddHours(1)
                            horae = data_c.Hour
                            diae = data_c.Day
                            Mêse = data_c.Month
                            Anoe = data_c.Year
                        End If

                        Qsub(CodigoLocalizado, horae, diae, Mêse, Ano(Anoe)) = AjustarDecimal(Valore)

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

            Next
        Next



        'atualiza os dados se atualizou pelo menos um arquivo
        If NumArquivosAtualizados > 0 Then

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            Me.ATUALIZA_ARQUIVOS_QSUBST_TODOS()
            'Atualiza as datas sugeridas nos seletores de datas
            Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            stlb.Text = "Atualização a partir de arquivos na pasta finalizada com sucesso!"

        End If
    End Sub

    Sub CARREGAR_DADOS_UHES_NOVO_DE_UMA_PASTA()


        Dim f2() As String = Directory.GetFiles(DIR_ESTREITO) 'este é o diretório com os arquivos das UHEs, definido nas preferências


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
                If cod_estação = Estação(po) Then
                    CodigoLocalizado = po
                End If
            Next

            'Caso o código for maior que zero, executa a rotina de ler o arquivo 
            If CodigoLocalizado > 0 Then 'Verifica se o é um dado de vazão mesmo

                NumArquivosAtualizados += 1

                'Cria a variável linha4, que contém a leitura da linha do arquivo
                Dim linha4 As String
                FileOpen(4, Arqu(p), OpenMode.Input)

                Dim contador As Integer
                contador = -1
                Do Until EOF(4)
                    linha4 = LineInput(4)

                    contador += 1
                    If contador = 0 Then Continue Do

                    Qsub(CodigoLocalizado, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = AjustarDecimal(linha4.Substring(24, 12))

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

                    'Depois de verificar as datas, vai para o próximo arquivo aberto

                Loop

                'Depois de passar por todas as linhas, fecha o arquivo e vai para o próximo
                FileClose(4)

            End If

        Next




        'atualiza os dados se atualizou pelo menos um arquivo
        If NumArquivosAtualizados > 0 Then

            'Atualiza as colunas da tabela principal com as datas iniciais e finais dos dados de vazão
            Me.AtualizaDataTabela()
            'Atualiza o arquivo de vazão substituída do database
            Me.ATUALIZA_ARQUIVOS_QSUBST_TODOS()
            'Atualiza as datas sugeridas nos seletores de datas
            Me.ATUALIZA_DATA()
            'Atualiza as colunas de vazão no horizonte
            Me.ATUALIZA_VAZAO_HORIZONTE()
            'Exibe mensagem de atualização bem sucedida
            stlb.Text = "Atualização a partir de arquivos na pasta finalizada com sucesso!"

        End If
    End Sub


    Private Sub Button13_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button13.Click
        MsgBox("Diretório de onde os arquivos serão buscados: " & DIR_ESTREITO, MsgBoxStyle.OkOnly, "Aviso")

        If rb_PadrãoONS.Checked = True Then
            Me.CARREGAR_DADOS_UHES_NOVO_DE_UMA_PASTA_PADRAO_ONS()
        ElseIf rb_PadrãoMGB.Checked = True Then
            Me.CARREGAR_DADOS_UHES_NOVO_DE_UMA_PASTA()
        End If


        MsgBox("Procedimento finalizado", MsgBoxStyle.OkOnly, "Aviso")
    End Sub
End Class