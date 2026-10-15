Public Class Form_parcal

    Private Sub Form_parcal_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        jatemtabelauso = False
    End Sub

    Private Sub Button7_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button7.Click
        Me.Close()

    End Sub

    Private Sub Button5_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button5.Click
        ofd1.Filter = "MGB HRCs file (.HRC)|*.HRC"
        If ofd1.ShowDialog = Windows.Forms.DialogResult.OK Then
            FileOpen(1, ofd1.FileName, OpenMode.Input)
            Dim leitura As String

            numerodeblocoscal = 0
            Do Until EOF(1)
                leitura = LineInput(1)
                numerodeblocoscal = numerodeblocoscal + 1
            Loop

            tbofdbloc.Text = ofd1.FileName
            FileClose()
            Button6.Enabled = True

            tbblocos.Text = numerodeblocoscal
            'pega os nomes do blocos!
            ReDim nomeblocos(tbblocos.Text)
            Dim auxiliar As Integer
            Dim splt
            auxiliar = 0
            FileOpen(1, ofd1.FileName, OpenMode.Input)
            Do Until EOF(1)
                leitura = LineInput(1)
                auxiliar = auxiliar + 1
                splt = Split(leitura, " // ")
                nomeblocos(auxiliar) = splt(0)
            Loop
            FileClose()
            '-nomes pegos
        End If




    End Sub

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        Dim tamarq As Integer
        Dim leitura As String
        'Abre os arquivo, lê, verifica o número de bacias
        ofd1.Filter = "Soil parameters datafile(.CAL)|*.CAL"
        If ofd1.ShowDialog = Windows.Forms.DialogResult.OK Then
            FileOpen(1, ofd1.FileName, OpenMode.Input)
            numpar = 7
            numbacia = 0
            tamarq = 0
            Do Until EOF(1)
                leitura = LineInput(1)
                tamarq = tamarq + 1
                If leitura = "       use      Wm       b    Kbas    Kint      XL     CAP      Wc" Then
                    numbacia = numbacia + 1
                ElseIf leitura = "      use      Wm       b    Kbas    Kint      XL     CAP      Wc" Then 'Caso o cabeçalho esteja deslocado
                    numbacia = numbacia + 1
                End If



            Loop
            'Calculando o número de bacias a partir do número de blocos, poderia ser feito como acima
            'numbacia = tamarq / (6 + tbblocos.Text) 'removido temporatiramente

            'tbbac.Text = numbacia
            FileClose()
            TextBox2.Text = ofd1.FileName

            'Teste para ver se o número de blocos é consistente
            Dim testador As Double
            testador = (tamarq - numbacia * 6) / numbacia

            If FormatNumber(testador, 0) <> tbblocos.Text Then 'se estiver errado não execua mais
                TextBox2.Clear()
                MsgBox("The number of HRCs on the soil paramenters file does not match with the number on HRCs file.", MsgBoxStyle.Critical)
            Else 'se estiver certo, executa

                TextBox2.Enabled = True
                'Carrega os dados no array
                Dim auxbac, auxini, bacia, bloco, par As Integer
                ReDim Parametro(numSubWatersheds, tbblocos.Text + 4, 7)
                FileOpen(1, ofd1.FileName, OpenMode.Input)
                auxbac = 0
                bacia = 1
                bloco = 0
                auxini = 2
                Do Until EOF(1)
                    leitura = LineInput(1)
                    '  MsgBox(leitura)
                    auxbac = auxbac + 1
                    If auxbac > auxini And auxbac < auxini + tbblocos.Text + 1 Then
                        bloco = bloco + 1

                        Parametro(bacia, bloco, 1) = leitura.Substring(12, 6)
                        Parametro(bacia, bloco, 2) = leitura.Substring(21, 5)
                        Parametro(bacia, bloco, 3) = leitura.Substring(27, 7)
                        Parametro(bacia, bloco, 4) = leitura.Substring(35, 7)
                        Parametro(bacia, bloco, 5) = leitura.Substring(45, 5)
                        Parametro(bacia, bloco, 6) = leitura.Substring(53, 5)
                        Parametro(bacia, bloco, 7) = leitura.Substring(61, 5)

                        'MsgBox(leitura.Substring(13, 5) & leitura.Substring(0, 2))
                    ElseIf auxbac = auxini + tbblocos.Text + 1 Then
                        bloco = bloco + 1
                        Parametro(bacia, bloco, 1) = leitura.Substring(13, 5)
                    ElseIf auxbac = auxini + tbblocos.Text + 2 Then
                        bloco = bloco + 1
                        Parametro(bacia, bloco, 1) = leitura.Substring(12, 6)
                    ElseIf auxbac = auxini + tbblocos.Text + 3 Then
                        bloco = bloco + 1
                        'MsgBox(leitura.Substring(10, 8))
                        Parametro(bacia, bloco, 1) = leitura.Substring(10, 8)
                    ElseIf auxbac = auxini + tbblocos.Text + 4 Then
                        bloco = bloco + 1
                        Try
                            Parametro(bacia, bloco, 1) = leitura.Substring(12, 8)
                        Catch ex As Exception
                            Parametro(bacia, bloco, 1) = leitura.Substring(12, 6)
                        End Try
                        auxini = auxini + tbblocos.Text + 6
                        bloco = 0
                        bacia = bacia + 1
                    End If
                Loop
                FileClose()
                '- Dados carregados no array
                'Hora de passar tudo isso para o datagrid
                tbnumbac.Text = 1
                Me.Criatabela()
                Me.carregatabela()
                Button1.Enabled = True
                Button2.Enabled = True
                Button3.Enabled = True
                Button4.Enabled = True
                Button9.Enabled = True
                Button10.Enabled = True
                Button11.Enabled = True
                Button12.Enabled = True
                Combo.Items.Clear()
                Me.carregacombobox() 'carrega os dados na combobox
            End If
        End If

    End Sub
    Sub carregatabela()
        Dim bacia As Integer
        bacia = Tbnumbac.Text

        ''Limpa o Datagrid
        'T_iniuso.Clear()
        'T_iniuso.Columns.Remove(Blocouso)
        'T_iniuso.Columns.Remove(Wm)
        'T_iniuso.Columns.Remove(b)
        'T_iniuso.Columns.Remove(Kbas)
        'T_iniuso.Columns.Remove(Kint)
        'T_iniuso.Columns.Remove(XL)
        'T_iniuso.Columns.Remove(CAP)
        'T_iniuso.Columns.Remove(Wc)
        'datasetuso.Tables.Remove(T_iniuso)
        'T_iniuso.Columns.Add(Blocouso)
        'T_iniuso.Columns.Add(Wm)
        'T_iniuso.Columns.Add(b)
        'T_iniuso.Columns.Add(Kbas)
        'T_iniuso.Columns.Add(Kint)
        'T_iniuso.Columns.Add(XL)
        'T_iniuso.Columns.Add(CAP)
        'T_iniuso.Columns.Add(Wc)
        'jatemtabelauso = True  '-para não dar problemas
        'datasetuso.Tables.Add(T_iniuso)
        'DataGriduso.DataSource = datasetuso
        'DataGriduso.SetDataBinding(datasetuso, "Parâmetros Calibráveis")
        'row_compluso = False
        '- Datagrid limpo

        ReDim Linhasuso(tbblocos.Text + 6)
        Dim auxiliar As Integer
        For auxiliar = 1 To tbblocos.Text

            DataGriduso.Item(0, auxiliar - 1).Value = nomeblocos(auxiliar)
            DataGriduso.Item(1, auxiliar - 1).Value = Parametro(bacia, auxiliar, 1)
            DataGriduso.Item(2, auxiliar - 1).Value = Parametro(bacia, auxiliar, 2)
            DataGriduso.Item(3, auxiliar - 1).Value = Parametro(bacia, auxiliar, 3)
            DataGriduso.Item(4, auxiliar - 1).Value = Parametro(bacia, auxiliar, 4)
            DataGriduso.Item(5, auxiliar - 1).Value = Parametro(bacia, auxiliar, 5)
            DataGriduso.Item(6, auxiliar - 1).Value = Parametro(bacia, auxiliar, 6)
            DataGriduso.Item(7, auxiliar - 1).Value = Parametro(bacia, auxiliar, 7)

        Next
        tbCS.Text = Parametro(bacia, tbblocos.Text + 1, 1)
        tbCI.Text = Parametro(bacia, tbblocos.Text + 2, 1)
        tbCB.Text = Parametro(bacia, tbblocos.Text + 3, 1)
        tbQB.Text = Parametro(bacia, tbblocos.Text + 4, 1)


    End Sub
    Sub salvadados() 'salva as modificações feitas pelo usuário
        Dim auxiliar As Integer
        For auxiliar = 1 To tbblocos.Text

            Parametro(tbnumbac.Text, auxiliar, 1) = DataSetuso.Tables(0).Rows(auxiliar - 1).Item(1)
            Parametro(tbnumbac.Text, auxiliar, 2) = DataSetuso.Tables(0).Rows(auxiliar - 1).Item(2)
            Parametro(tbnumbac.Text, auxiliar, 3) = DataSetuso.Tables(0).Rows(auxiliar - 1).Item(3)
            Parametro(tbnumbac.Text, auxiliar, 4) = DataSetuso.Tables(0).Rows(auxiliar - 1).Item(4)
            Parametro(tbnumbac.Text, auxiliar, 5) = DataSetuso.Tables(0).Rows(auxiliar - 1).Item(5)
            Parametro(tbnumbac.Text, auxiliar, 6) = DataSetuso.Tables(0).Rows(auxiliar - 1).Item(6)
            Parametro(tbnumbac.Text, auxiliar, 7) = DataSetuso.Tables(0).Rows(auxiliar - 1).Item(7)

            Parametro(Tbnumbac.Text, tbblocos.Text + 1, 1) = tbCS.Text
            Parametro(Tbnumbac.Text, tbblocos.Text + 2, 1) = tbCI.Text
            Parametro(Tbnumbac.Text, tbblocos.Text + 3, 1) = tbCB.Text
            Parametro(Tbnumbac.Text, tbblocos.Text + 4, 1) = tbQB.Text
        Next
        ' 
    End Sub
    Sub carregacombobox()
        For itens As Integer = 1 To numSubWatersheds
            Combo.Items.Add(itens)
        Next
    End Sub

    Private Sub Button8_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button8.Click

        'If MsgBox("This is going to delete all data, do you want to proceed?", MsgBoxStyle.YesNo, "Soil Parameters") = MsgBoxResult.Yes Then

        ' tbbac.Text = InputBox("You choosed to create a new calibration parameters file. How many subwatersheds are going to be used on the project?", "Number of subwatersheds", 5)
        If jatemtabelauso = False Then
            Me.Criatabela()
        End If

        Dim auxiliar As Integer
        For auxiliar = 1 To tbblocos.Text

            DataGriduso.Item(0, auxiliar - 1).Value = ""
            DataGriduso.Item(1, auxiliar - 1).Value = "" 'Parametro(bacia, auxiliar, 1)
            DataGriduso.Item(2, auxiliar - 1).Value = "" 'Parametro(bacia, auxiliar, 2)
            DataGriduso.Item(3, auxiliar - 1).Value = "" 'Parametro(bacia, auxiliar, 3)
            DataGriduso.Item(4, auxiliar - 1).Value = "" 'Parametro(bacia, auxiliar, 4)
            DataGriduso.Item(5, auxiliar - 1).Value = "" 'Parametro(bacia, auxiliar, 5)
            DataGriduso.Item(6, auxiliar - 1).Value = "" 'Parametro(bacia, auxiliar, 6)
            DataGriduso.Item(7, auxiliar - 1).Value = "" 'Parametro(bacia, auxiliar, 7)

        Next

        tbnumbac.Clear()
        tbCS.Clear()
        tbCI.Clear()
        tbCB.Clear()
        tbQB.Clear()
        ' tbofdbloc.Clear()
        TextBox2.Clear()
        'Button6.Enabled = False
        'tbblocos.Clear()
        'Tbbac.Clear()
        Button1.Enabled = False
        Button2.Enabled = False
        Button3.Enabled = False
        Button4.Enabled = False
        Button9.Enabled = False
        'Cria um novo array com zero em todos os termos
        ReDim Parametro(numSubWatersheds, tbblocos.Text + 4, 7)
        For totbac As Integer = 1 To numSubWatersheds
            For totbloc As Integer = 1 To tbblocos.Text + 4
                For totpar As Integer = 1 To 7
                    Parametro(totbac, totbloc, totpar) = 0
                Next
            Next
        Next
        'Passa tudo pra tabela
        tbnumbac.Text = 1
        Me.carregatabela()
        Button1.Enabled = True
        Button2.Enabled = True
        Button3.Enabled = True
        Button4.Enabled = True
        Button9.Enabled = True
        Button10.Enabled = True
        Button11.Enabled = True
        Button12.Enabled = True

        Combo.Items.Clear()

        Me.carregacombobox()

        'End If

    End Sub

    Private Sub Button9_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button9.Click
        sfd1.Filter = "Soil Parameters File(.CAL)|*.CAL"
        Dim nomebloco, describloco As String
        Dim auxiliar, blocoutilizado As Integer
        Me.salvadados() 'chama a sub que salva os parametros no array
        If sfd1.ShowDialog = Windows.Forms.DialogResult.OK Then
            FileOpen(1, sfd1.FileName, OpenMode.Output) 'abre o arquivo
            For auxiliar = 1 To numSubWatersheds ' começa o array, desde 1 até o número de sub-bacias
                PrintLine(1, "Watershed " & auxiliar) 'para cada bacia, ele coloca o nome dela e o cabeçalho
                PrintLine(1, "       use      Wm       b    Kbas    Kint      XL     CAP      Wc")
                'depois ele faz um loop para escrever os blocos, desde 1 aé o númeor de blocos
                For blocoutilizado = 1 To tbblocos.Text
                    nomebloco = DataSetuso.Tables(0).Rows(blocoutilizado - 1).Item(0) 'ele vai ler o nome de cada bloco e vai ajustar para que ele tenha tamanho = a "9"
                    If nomebloco.Length < 9 Then 'verifica se o tamanho é menor que nove
                        For aumn As Integer = 1 To 9 - nomebloco.Length 'aumenta um " " no final do nome de acordo com o tamanho
                            nomebloco = nomebloco & " "
                        Next
                        'Obs.: os nomes não passam de 9 pois a form "Blocos" não permite
                    End If

                    'Começa a escrita dos parâmetros
                    Print(1, nomebloco)
                    'Wm
                    For espaço As Integer = 1 To 9 - FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1).Length
                        Print(1, " ")
                    Next
                    If Parametro(auxiliar, blocoutilizado, 1) >= 1000 Then
                        Print(1, " " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1).Substring(0, 1) & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1).Substring(2, 5))
                    ElseIf Parametro(auxiliar, blocoutilizado, 1) < 1000 Then
                        Print(1, FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1))
                    End If
                    'b
                    For espaço As Integer = 1 To 8 - FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2).Length
                        Print(1, " ")
                    Next
                    Print(1, FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2))
                    'kbas
                    For espaço As Integer = 1 To 8 - FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2).Length
                        Print(1, " ")
                    Next
                    If Parametro(auxiliar, blocoutilizado, 3) >= 1000 Then
                        Print(1, " " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2).Substring(0, 1) & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2).Substring(2, 6))
                    ElseIf Parametro(auxiliar, blocoutilizado, 3) < 1000 Then
                        Print(1, FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2))
                    End If
                    'kint
                    For espaço As Integer = 1 To 8 - FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2).Length
                        Print(1, " ")
                    Next
                    If Parametro(auxiliar, blocoutilizado, 4) >= 1000 Then
                        Print(1, " " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2).Substring(0, 1) & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2).Substring(2, 6))
                    ElseIf Parametro(auxiliar, blocoutilizado, 4) < 1000 Then
                        Print(1, FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2))
                    End If
                    'XL
                    For espaço As Integer = 1 To 8 - FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2).Length
                        Print(1, " ")
                    Next
                    Print(1, FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2))
                    'CAP
                    For espaço As Integer = 1 To 8 - FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2).Length
                        Print(1, " ")
                    Next
                    Print(1, FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2))
                    'Wc
                    For espaço As Integer = 1 To 8 - FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2).Length
                        Print(1, " ")
                    Next
                    Print(1, FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))

                    PrintLine(1)


                    'If Parametro(auxiliar, blocoutilizado, 4) < 10 Then 'verifica o kint, que a água é menor que 10 e nos demais é diferente

                    '                    If Parametro(auxiliar, blocoutilizado, 1) < 10 Then 'Agora ele vai lendo a tabela e escrevendo, ele faz a verificação do campo "wm"
                    'PrintLine(1, nomebloco & "      " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))
                    'ElseIf Parametro(auxiliar, blocoutilizado, 1) >= 10 And Parametro(auxiliar, blocoutilizado, 1) < 100 Then
                    'PrintLine(1, nomebloco & "     " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))
                    'ElseIf Parametro(auxiliar, blocoutilizado, 1) >= 100 And Parametro(auxiliar, blocoutilizado, 1) < 1000 Then
                    'PrintLine(1, nomebloco & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))
                    'ElseIf Parametro(auxiliar, blocoutilizado, 1) >= 1000 Then
                    'PrintLine(1, nomebloco & "   " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1).Substring(0, 1) & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1).Substring(2, 5) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))

                    'End If
                    'ElseIf Parametro(auxiliar, blocoutilizado, 4) >= 10 Then

                    '                    If Parametro(auxiliar, blocoutilizado, 1) < 10 Then 'Agora ele vai lendo a tabela e escrevendo, ele faz a verificação do campo "wm"
                    'PrintLine(1, nomebloco & "      " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "   " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))
                    'ElseIf Parametro(auxiliar, blocoutilizado, 1) >= 10 And Parametro(auxiliar, blocoutilizado, 1) < 100 Then
                    'PrintLine(1, nomebloco & "     " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "   " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))
                    'ElseIf Parametro(auxiliar, blocoutilizado, 1) >= 100 And Parametro(auxiliar, blocoutilizado, 1) < 1000 Then
                    'PrintLine(1, nomebloco & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "   " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))
                    'ElseIf Parametro(auxiliar, blocoutilizado, 1) >= 1000 Then
                    'PrintLine(1, nomebloco & "   " & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1).Substring(0, 1) & FormatNumber(Parametro(auxiliar, blocoutilizado, 1), 1).Substring(2, 5) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 2), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 3), 2) & "   " & FormatNumber(Parametro(auxiliar, blocoutilizado, 4), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 5), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 6), 2) & "    " & FormatNumber(Parametro(auxiliar, blocoutilizado, 7), 2))

                    'End If
                    'End If


                Next
                'Agora ele vai escrever os 4 campos que vão abaixo dos blocos (CS,CI,CB e QB), e vai permear eles pelos seus tamanhos.
                Print(1, "CS")
                For cont As Integer = 1 To 16 - FormatNumber(Parametro(auxiliar, tbblocos.Text + 1, 1), 2).ToString.Length
                    Print(1, " ")
                Next
                If Parametro(auxiliar, tbblocos.Text + 1, 1) = 0 Then
                    MsgBox("Please provide Cs Parameter value for all sub-basins. The Parameters file couldn't be created successfully.")
                    FileClose()
                    Exit Sub
                End If
                PrintLine(1, FormatNumber(Parametro(auxiliar, tbblocos.Text + 1, 1), 2))
                Print(1, "CI")
                For cont As Integer = 1 To 16 - FormatNumber(Parametro(auxiliar, tbblocos.Text + 2, 1), 2).ToString.Length
                    Print(1, " ")
                Next
                If Parametro(auxiliar, tbblocos.Text + 2, 1) = 0 Then
                    MsgBox("Please provide Ci Parameter value for all sub-basins. The Parameters file couldn't be created successfully.")
                    FileClose()
                    Exit Sub
                End If
                PrintLine(1, FormatNumber(Parametro(auxiliar, tbblocos.Text + 2, 1), 2))
                Print(1, "CB")
                If Parametro(auxiliar, tbblocos.Text + 3, 1) >= 1000 And Parametro(auxiliar, tbblocos.Text + 3, 1) < 10000 Then
                    Print(1, "         ")
                    If Parametro(auxiliar, tbblocos.Text + 3, 1) = 0 Then
                        MsgBox("Please provide Cb Parameter value for all sub-basins. The Parameters file couldn't be created successfully.")
                        FileClose()
                        Exit Sub
                    End If
                    PrintLine(1, FormatNumber(Parametro(auxiliar, tbblocos.Text + 3, 1), 2).Substring(0, 1) & FormatNumber(Parametro(auxiliar, tbblocos.Text + 3, 1), 2).Substring(2, 6))
                ElseIf Parametro(auxiliar, tbblocos.Text + 3, 1) < 1000 And Parametro(auxiliar, tbblocos.Text + 3, 1) >= 100 Then
                    Print(1, "          ")
                    If Parametro(auxiliar, tbblocos.Text + 3, 1) = 0 Then
                        MsgBox("Please provide Cb Parameter value for all sub-basins. The Parameters file couldn't be created successfully.")
                        FileClose()
                        Exit Sub
                    End If
                    PrintLine(1, FormatNumber(Parametro(auxiliar, tbblocos.Text + 3, 1), 2))
                ElseIf Parametro(auxiliar, tbblocos.Text + 3, 1) < 100 Then
                    Print(1, "           ")
                    If Parametro(auxiliar, tbblocos.Text + 3, 1) = 0 Then
                        MsgBox("Please provide Cb Parameter value for all sub-basins. The Parameters file couldn't be created successfully.")
                        FileClose()
                        Exit Sub
                    End If
                    PrintLine(1, FormatNumber(Parametro(auxiliar, tbblocos.Text + 3, 1), 2))
                ElseIf Parametro(auxiliar, tbblocos.Text + 3, 1) >= 10000 Then
                    Print(1, "        ")
                    If Parametro(auxiliar, tbblocos.Text + 3, 1) = 0 Then
                        MsgBox("Please provide Cb Parameter value for all sub-basins. The Parameters file couldn't be created successfully.")
                        FileClose()
                        Exit Sub
                    End If
                    PrintLine(1, FormatNumber(Parametro(auxiliar, tbblocos.Text + 3, 1), 2).Substring(0, 2) & FormatNumber(Parametro(auxiliar, tbblocos.Text + 3, 1), 2).Substring(3, 6))
                End If
                Print(1, "QB_M3/SKM2")
                For cont As Integer = 1 To 8 - FormatNumber(Parametro(auxiliar, tbblocos.Text + 4, 1), 2).ToString.Length
                    Print(1, " ")
                Next
                PrintLine(1, FormatNumber(Parametro(auxiliar, tbblocos.Text + 4, 1), 4))
                'Recomeça tudo para a próxima bacia
            Next

            FileClose()
            Label14.Text = "Soil Parameters successfully created."

        End If
    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        If tbnumbac.Text + 1 <= numSubWatersheds Then
            Me.salvadados()
            tbnumbac.Text = tbnumbac.Text + 1
            Me.carregatabela()
        Else
        End If
    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        Me.salvadados()
        tbnumbac.Text = numSubWatersheds
        Me.carregatabela()

    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        If tbnumbac.Text - 1 >= 1 Then
            Me.salvadados()
            tbnumbac.Text = tbnumbac.Text - 1
            Me.carregatabela()
        Else
        End If
    End Sub

    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click
        Me.salvadados()
        tbnumbac.Text = 1
        Me.carregatabela()
    End Sub

    Private Sub Button10_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button10.Click
        For totbloc As Integer = 1 To tbblocos.Text
            For totpar As Integer = 1 To 7
                Parametro(tbnumbac.Text, totbloc, totpar) = Parametro(Combo.SelectedItem, totbloc, totpar)
            Next
        Next
        Parametro(tbnumbac.Text, tbblocos.Text + 1, 1) = Parametro(Combo.SelectedItem, tbblocos.Text + 1, 1)
        Parametro(tbnumbac.Text, tbblocos.Text + 2, 1) = Parametro(Combo.SelectedItem, tbblocos.Text + 2, 1)
        Parametro(tbnumbac.Text, tbblocos.Text + 3, 1) = Parametro(Combo.SelectedItem, tbblocos.Text + 3, 1)
        Parametro(tbnumbac.Text, tbblocos.Text + 4, 1) = Parametro(Combo.SelectedItem, tbblocos.Text + 4, 1)

        Me.carregatabela()
    End Sub

    Private Sub Button11_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button11.Click

        If MsgBox("Extrapolates data for all subwatersheds. Are you sure?", MsgBoxStyle.YesNo, "Extrapolate Data") = MsgBoxResult.Yes Then
            Me.salvadados()
            For totbac As Integer = 1 To numSubWatersheds
                For totbloc As Integer = 1 To tbblocos.Text + 4
                    For totpar As Integer = 1 To 7
                        Parametro(totbac, totbloc, totpar) = Parametro(tbnumbac.Text, totbloc, totpar)
                    Next
                Next
            Next

            Me.carregatabela()

        End If

    End Sub

    Private Sub Button12_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button12.Click
        If MsgBox("Copy data from the first line to all others, are you sure?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then

            For totbloc As Integer = 1 To tbblocos.Text
                For totpar As Integer = 1 To 7
                    Parametro(tbnumbac.Text, totbloc, totpar) = DataSetuso.Tables(0).Rows(0).Item(totpar)
                Next
            Next
            Me.carregatabela()
        End If


    End Sub
    Sub Criatabela()

        For totbloc As Integer = 1 To tbblocos.Text
            'Escreve os valores no data grid!!!
            Dim nova_linha As DataRow = DataSetuso.Tables(0).NewRow()

            ' Preenche os valores do registro
            nova_linha.ItemArray = New Object() {"", "", "", "", "", "", "", ""}
            Try
                DataSetuso.Tables(0).Rows.Add(nova_linha)
            Catch ex As Exception
                MsgBox(" error: " & ex.Message)
            End Try
        Next
        jatemtabelauso = True
    End Sub

    Private Sub Button13_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btMINI.Click
        ofd1.Filter = "Mini.GTP (.GTP)|*.GTP"
        If ofd1.ShowDialog = Windows.Forms.DialogResult.OK Then
            tbMINI.Text = ofd1.FileName
        End If

        Button8.Enabled = True

        TotalMINI = 0
        Using fs As New IO.StreamReader(ofd1.FileName)
            linha = fs.ReadLine
            Do Until fs.EndOfStream
                linha = fs.ReadLine
                TotalMINI += 1
            Loop
            fs.Close()
        End Using

        ReDim Subwatershed(TotalMINI)

        MINIBACIA = 0

        Using fs As New IO.StreamReader(ofd1.FileName)
            linha = fs.ReadLine
            Do Until fs.EndOfStream
                linha = fs.ReadLine
                MINIBACIA = MINIBACIA + 1
                If MINIBACIA > 0 Then
                    Subwatershed(MINIBACIA) = linha.Substring(46, 9)
                End If
            Loop
            fs.Close()
        End Using
        'tbbac.Text = Subwatershed.Max()
        numSubWatersheds = Subwatershed.Max()
    End Sub
End Class