Imports System.IO

Public Class Form2

    Private Sub Form2_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        'Verifica quantas estações existem na base de dados interna
        Dim arquivo_est As String
        Dim total_estações As Integer
        Dim linha As String
        total_estações = 0
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Estações.txt"
        FileOpen(1, arquivo_est, OpenMode.Input)
        Do Until EOF(1)
            linha = LineInput(1)
            total_estações = total_estações + 1
        Loop
        FileClose()
        '-- Fim da verificação do total de estações

        TextBox1.Text = total_estações


        Dim nome(total_estações) As String

        Dim conter4 As Integer
        'PBar1.Maximum = 24915


        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Estações.txt"
        FileOpen(2, arquivo_est, OpenMode.Input)


        For conter4 = 1 To total_estações

            linha = LineInput(2)
            nome(conter4) = linha

            ' PBar1.Increment(1)

            'Adiciona os dados à tabela

            '-----Dados carregados

            Dim nova_linha As DataRow = DataSet1.Tables(0).NewRow()

            ' Preenche os valores do registro
            nova_linha.ItemArray = New Object() {nome(conter4)}
            Try
                DataSet1.Tables(0).Rows.Add(nova_linha)
            Catch ex As Exception
                MsgBox(" erro: " & ex.Message)
            End Try

            ' FileCopy(Arqu(p), System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\" & coisa(p) & ".txt")


        Next
        'PBar1.Hide()
        FileClose()
        'Me.Text = "Fluviometria"
        'Carregou dados internos


        Me.CARREGA_DIRETÓRIOS()



    End Sub

    Private Sub DataGridView1_CellContentClick(ByVal sender As System.Object, ByVal e As System.Windows.Forms.DataGridViewCellEventArgs) Handles DataGridView1.CellContentClick

    End Sub

    Private Sub GroupBox1_Enter(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles GroupBox1.Enter

    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Try
            Dim arquivo_est As String
            arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Estações.txt"
            FileOpen(1, arquivo_est, OpenMode.Output)


            'Número de estações
            Dim tamarq As Integer
            tamarq = DataGridView1.RowCount

            'Tamanho do string na primeira célula
            Dim string_cell As Integer

            For variável As Integer = 1 To tamarq - 1

                PrintLine(1, DataGridView1.Item(0, variável - 1).Value)

            Next
            FileClose()

            TextBox1.Text = tamarq - 1

            MsgBox("Atualização da base de dados efetuada com sucesso!", MsgBoxStyle.Information, "Database")
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

    End Sub

    Sub CARREGA_DIRETÓRIOS()
        'Verifica quantas estações existem na base de dados interna
        Dim arquivo_est As String
        Dim total_estações As Integer
        Dim linha As String
        total_estações = 0
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\config.txt"
        FileOpen(1, arquivo_est, OpenMode.Input)
        Do Until EOF(1)
            linha = LineInput(1)
            total_estações = total_estações + 1
            If total_estações = 1 Then
                tb_telemetria.Text = linha
            ElseIf total_estações = 2 Then
                tb_telemetria_chuva.Text = linha
            ElseIf total_estações = 3 Then
                tb_metereológicas.Text = linha
            ElseIf total_estações = 4 Then
                tb_previsao.Text = linha
            ElseIf total_estações = 5 Then
                tb_estreito.Text = linha
            ElseIf total_estações = 6 Then
                tb_meteo_auto.Text = linha
            ElseIf total_estações = 9 Then
                'tb_previsao.Text = linha
            ElseIf total_estações = 10 Then
                'tb_previsao.Text = linha
            ElseIf total_estações = 11 Then
                tb_meteo_auto.Text = linha
            ElseIf total_estações = 12 Then
                tb_meteo_auto2.Text = linha
            ElseIf total_estações = 13 Then
                tb_meteo_auto3.Text = linha
            ElseIf total_estações = 14 Then
                tb_dir_export.Text = linha
            End If
        Loop
        FileClose()
    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_telemetria.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Sub Atualiza_config()
        Dim arquivo_est As String
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\config.txt"
        FileOpen(1, arquivo_est, OpenMode.Output)
        PrintLine(1, tb_telemetria.Text) '1
        PrintLine(1, tb_telemetria_chuva.Text) '2
        PrintLine(1, tb_metereológicas.Text) '3
        PrintLine(1, tb_previsao.Text) '4
        PrintLine(1, tb_estreito.Text) '5
        PrintLine(1, tb_meteo_auto.Text) '6
        PrintLine(1, tb_previsao.Text) '7
        PrintLine(1, tb_previsao.Text)
        PrintLine(1, tb_telemetria.Text)
        PrintLine(1, tb_telemetria.Text)
        PrintLine(1, tb_meteo_auto.Text) '11
        PrintLine(1, tb_meteo_auto2.Text) '12
        PrintLine(1, tb_meteo_auto3.Text) '13
        PrintLine(1, tb_dir_export.Text) '14
        FileClose()

    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_estreito.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_metereológicas.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button5_Click(ByVal sender As System.Object, ByVal e As System.EventArgs)
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_estreito.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs)
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            'tb_serradamesa.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button7_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button7.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_telemetria_chuva.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button8_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button8.Click
        Me.Atualiza_config()
        Me.Close()
    End Sub

    Private Sub Button9_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button9.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_previsao.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub TabPage1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles TabPage1.Click

    End Sub

    Private Sub Button10_Click(ByVal sender As System.Object, ByVal e As System.EventArgs)
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            'tb_canabrava.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button11_Click(ByVal sender As System.Object, ByVal e As System.EventArgs)
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            'tb_secQP2.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub btCompact_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btCompact.Click
        'Esse botão faz duas coisas:
        '1. Cria uma cópia de backup da pasta database
        '2. Abre todos os arquivos e reescreve eles começado somente de julho do ano passado

        '1. Faz o backup do diretório
        Dim diretorio_dtb As String
        diretorio_dtb = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        My.Computer.FileSystem.CopyDirectory(diretorio_dtb & "\Database", diretorio_dtb & "\Database_backup_" & Now.Year & Now.Month & Now.Day, True)

        'Para leitura
        Dim linha As String
        Dim datal As Date

        '2. Escreve tudo na pasta nova
        Dim f() As String = IO.Directory.GetFiles(diretorio_dtb & "\Database_backup_" & Now.Year & Now.Month & Now.Day)
        For i As Integer = 0 To UBound(f)
            ' File.Copy(f(i), Environment.GetFolderPath(Environment.SpecialFolder.Desktop) & "\Input" & "\" & System.IO.Path.GetFileName(f(i)))
            'File.Copy(f(i), "c:\mgb-hora\Input" & "\" & System.IO.Path.GetFileName(f(i)), True)

            Try

                FileOpen(2, diretorio_dtb & "\Database\" & System.IO.Path.GetFileName(f(i)), OpenMode.Output)

                FileOpen(1, f(i), OpenMode.Input)

                Dim contador As Integer
                contador = -1

                Do Until EOF(1)
                    linha = LineInput(1)
                    contador += 1

                    If contador = 0 Then
                        PrintLine(2, linha)
                    End If

                    If contador > 0 Then

                        datal = linha.Substring(4, 2) & "/" & linha.Substring(10, 2) & "/" & linha.Substring(14, 4) & " " & linha.Substring(22, 2) & ":00:00"

                        If datal >= "01/07/" & Now.Date.AddYears(-1).Year & " 00:00:00" Then
                            PrintLine(2, linha)
                        End If

                    End If

                Loop
                FileClose()

            Catch ex As Exception

                MsgBox(ex.Message)
                FileClose()

            End Try
        Next
        MsgBox("Compactação finalizada.")
    End Sub

    Private Sub Button13_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button13.Click
        If MsgBox("Deseja exportar base de dados atual do sistema?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then
            Me.EXPORTAR_DATABASE()
        End If
    End Sub

    Sub EXPORTAR_DATABASE()
        Dim fbd As New System.Windows.Forms.FolderBrowserDialog
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Database"

        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then

            ' Escreve tudo na pasta c:\mgb
            Dim f() As String = Directory.GetFiles(diretóriodoprojeto)

            pbar.Maximum = UBound(f)
            pbar.Value = 0

            'Para cada um dos arquivos na pasta    
            For i As Integer = 0 To UBound(f)

                pbar.Increment(1)

                File.Copy(f(i), fbd.SelectedPath & "\" & System.IO.Path.GetFileName(f(i)), True)
            Next

            pbar.Value = 0
            MsgBox("Base de dados exportada.", MsgBoxStyle.Information)

        End If

    End Sub


    Private Sub Button15_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button15.Click
        If MsgBox("Deseja carregar uma database e mesclar com a atual do sistema?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then
            Me.ATUALIZAR_DATABASE()
        End If
    End Sub

    Sub ATUALIZAR_DATABASE()
        Dim Arquivo(2, 24, 31, 12, Ano(AF)) As Single
        Dim linha4 As String
        Dim DataO As Date
        Dim DataE As Date
        DataO = Now.Date.Day & "/" & Now.Date.Month & "/" & Now.Date.Year & " " & Now.Date.Hour & ":00:00"
        DataE = "01/01/" & AI & " " & "00:00:00"
        '-- Fim variáveis 

        Dim fbd As New System.Windows.Forms.FolderBrowserDialog

        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then

            ' Escreve tudo na pasta c:\mgb
            Dim f() As String = Directory.GetFiles(fbd.SelectedPath)

            pbar.Maximum = UBound(f)
            pbar.Value = 0

            'Para cada um dos arquivos na pasta    
            For i As Integer = 0 To UBound(f)

                pbar.Increment(1)

                Try


                    'ZERA TUDO
                    For aaaa = AI To AF
                        For mm = 1 To 12
                            For dd = 1 To 31
                                For hh As Integer = 0 To 23
                                    Arquivo(0, hh, dd, mm, Ano(aaaa)) = -1
                                    Arquivo(1, hh, dd, mm, Ano(aaaa)) = -1
                                    Arquivo(2, hh, dd, mm, Ano(aaaa)) = -1

                                Next
                            Next
                        Next
                    Next
                    'ZERA TUDO

                    Dim diretóriodoprojeto As String
                    diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Database"

                    FileOpen(1, f(i), OpenMode.Input)
                    FileOpen(2, diretóriodoprojeto & "\" & System.IO.Path.GetFileName(f(i)), OpenMode.Input)
                    Dim c As Integer
                    c = 0

                    'Lê o arquivo 01
                    Do Until EOF(1)
                        linha4 = LineInput(1)
                        c += 1

                        If c > 1 Then
                            'Lê a chuva
                            Arquivo(1, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = AjustarDecimal(linha4.Substring(26, 10))

                            If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" < DataO Then
                                DataO = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            End If
                            If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" > DataE Then
                                DataE = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            End If
                        End If

                    Loop
                    c = 0
                    'Lê o arquivo 02
                    Do Until EOF(2)
                        linha4 = LineInput(2)
                        c += 1

                        If c > 1 Then
                            'Lê a chuva
                            Arquivo(2, linha4.Substring(22, 2), linha4.Substring(4, 2), linha4.Substring(10, 2), Ano(linha4.Substring(14, 4))) = AjustarDecimal(linha4.Substring(26, 10))

                            If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" < DataO Then
                                DataO = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            End If
                            If linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00" > DataE Then
                                DataE = linha4.Substring(4, 2) & "/" & linha4.Substring(10, 2) & "/" & linha4.Substring(14, 4) & " " & linha4.Substring(22, 2) & ":00:00"
                            End If
                        End If

                    Loop


                    FileClose(1)
                    FileClose(2)




                    Dim tmaxx As Integer
                    tmaxx = DateDiff(DateInterval.Hour, DataO, DataE) + 1
                    'tmaxx = dif. de tempo

                    Dim datacomeço As Date
                    datacomeço = DataO
                    'datacomeço.AddDays(1)

                    'define o arquivo
                    Dim arquivo_est2 As String
                    arquivo_est2 = diretóriodoprojeto & "\" & System.IO.Path.GetFileName(f(i))

                    Nome_p = "Valor"

                    FileOpen(2, arquivo_est2, OpenMode.Output)


                    Print(2, "   Dia   Mês   Ano  Hora") 'Escreve o começo do cabeçalho


                    'escreve o arquivo no cabeçalho
                    For espaço As Integer = 1 To 12 - Nome_p.Length
                        'primeiro esquece os espaços
                        Print(2, " ")
                    Next
                    'escreve o nome do arquivo
                    Print(2, Nome_p)
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

                        Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)) = Math.Max(Arquivo(1, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), Arquivo(2, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))

                        If SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                            Print(2, "       " & "-1.00")
                        ElseIf SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                            Print(2, "        " & SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                        ElseIf SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                            Print(2, "       " & SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                        ElseIf SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                            Print(2, "      " & SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                        ElseIf SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                            Print(2, "     " & SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                        ElseIf SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                            Print(2, "    " & SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                        ElseIf SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                            Print(2, "   " & SubstituirSeparador(Arquivo(0, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                            'não diminui dois espaços pois não tem o separador de milhar
                        End If

                        PrintLine(2)

                    Next

                    FileClose(2)

                Catch ex As Exception
                    MsgBox(ex.Message)
                    FileClose()
                End Try


            Next
            pbar.Value = 0


        End If
    End Sub

    Private Sub Button14_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button14.Click
        If MsgBox("Deseja carregar uma database e substituir a atual do sistema por ela?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then
            Me.COPIAR_DATABASE()
        End If
    End Sub

    Sub COPIAR_DATABASE()

        'Esta rotina copia e substitui o database da máquina
        'Variáveis
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Database"
        Dim fbd As New System.Windows.Forms.FolderBrowserDialog

        'Roda a rotina
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then

            Dim f() As String = Directory.GetFiles(fbd.SelectedPath)

            pbar.Maximum = UBound(f)
            pbar.Value = 0

            'Para cada um dos arquivos na pasta    
            For i As Integer = 0 To UBound(f)

                pbar.Increment(1)

                Try

                    'Copia
                    File.Copy(f(i), diretóriodoprojeto & "\" & System.IO.Path.GetFileName(f(i)), True)

                Catch ex As Exception

                End Try

            Next
        End If
    End Sub

    Private Sub Button6_Click_1(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_meteo_auto.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button12_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button12.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_meteo_auto2.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub Button16_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button16.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_meteo_auto3.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub

    Private Sub bt_dir_export_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles bt_dir_export.Click
        If fbd.ShowDialog = Windows.Forms.DialogResult.OK Then
            tb_dir_export.Text = fbd.SelectedPath
            Me.Atualiza_config()
        End If
    End Sub
End Class