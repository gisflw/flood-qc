Public Class Editor

    Private Area_MINI_TAB(), Ltr_MINI_TAB(), Str_MINI_TAB(), AreaM_TAB(), Ycen_TAB(), Xcen_TAB(), Ltl_MINI_TAB(), Stl_MINI_TAB(), Width(), Depth(), Manning(), PBlocos(,) As Double
    Private MiniJus_TAB(), CatID_MINI_TAB(), MINI_CatID_TAB(), Ordem_MINI_TAB(), Sub_TAB() As Integer
    Private intTOTAL_MINIBACIAS As Integer
    Private numerodeblocoscal As Integer
    Private nomeblocos() As String

    Private Sub btABRIRMINI_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btABRIRMINI.Click
        'Rotina que carrega o MINI
        Me.CARREGACELLHIG_TABELA()


        'Rotina que escreve o mini na tabela
        Me.CARREGA_TABELA()
    End Sub

    Sub CARREGACELLHIG_TABELA()

        'Declara as variáveis utilizadas
        Dim MINIBACIA As Integer 'Identificador da Minibacia
        Dim opendiag As New Windows.Forms.OpenFileDialog
        opendiag.Filter = "MGB geometry file (.mgb;.txt;.gtp)|*.mgb;*.txt;*.gtp"
        Dim linha As String 'String de leitura
        intTOTAL_MINIBACIAS = 0

        If opendiag.ShowDialog = Windows.Forms.DialogResult.OK Then

            tbcell.Text = opendiag.FileName

            'Primeiro uma leitura para saber quantas minibacias temos
            FileOpen(1, opendiag.FileName, OpenMode.Input)
            Do Until EOF(1)
                linha = LineInput(1)
                intTOTAL_MINIBACIAS = intTOTAL_MINIBACIAS + 1
            Loop
            intTOTAL_MINIBACIAS = intTOTAL_MINIBACIAS - 1
            FileClose(1)

            'Redimensiona as variáveis utilizadas para ler o CELL.MGB
            ReDim Area_MINI_TAB(intTOTAL_MINIBACIAS) 'area em km² de cada minibacia
            ReDim Ltr_MINI_TAB(intTOTAL_MINIBACIAS) 'comprimento do trecho de rio na minibacia
            ReDim Str_MINI_TAB(intTOTAL_MINIBACIAS) 'declividade do trecho de rio na minibacia
            ReDim Ltl_MINI_TAB(intTOTAL_MINIBACIAS) 'comprimento do trecho de rio na minibacia
            ReDim Stl_MINI_TAB(intTOTAL_MINIBACIAS) 'declividade do trecho de rio na minibacia
            ReDim PBlocos(intTOTAL_MINIBACIAS, numerodeblocoscal) 'declividade do trecho de rio na minibacia
            ReDim MiniJus_TAB(intTOTAL_MINIBACIAS) 'minibacia a jusante da bacia selecionada
            ReDim MINI_CatID_TAB(intTOTAL_MINIBACIAS) 'entra com o cat ID e sai com a MINIBACIA
            ReDim CatID_MINI_TAB(intTOTAL_MINIBACIAS) 'entra com a mini e tira o CATID
            ReDim Ordem_MINI_TAB(intTOTAL_MINIBACIAS) 'Ordem da bacia (cabeceira, secundária...)
            ReDim AreaM_TAB(intTOTAL_MINIBACIAS) 'Área de montante da mini bacia
            ReDim Ycen_TAB(intTOTAL_MINIBACIAS) 'Latitude em graus
            ReDim Xcen_TAB(intTOTAL_MINIBACIAS) 'Longitude em graus
            ReDim Sub_TAB(intTOTAL_MINIBACIAS) 'Subbacia
            ReDim Width(intTOTAL_MINIBACIAS) 'Subbacia
            ReDim Depth(intTOTAL_MINIBACIAS) 'Subbacia
            ReDim Manning(intTOTAL_MINIBACIAS) 'Subbacia



            'Agora faz uma leitura para identificar cada uma das características da minibacia
            MINIBACIA = -1
            FileOpen(1, opendiag.FileName, OpenMode.Input)
            Do Until EOF(1)
                linha = LineInput(1)
                MINIBACIA = MINIBACIA + 1
                If MINIBACIA > 0 Then
                    CatID_MINI_TAB(MINIBACIA) = linha.Substring(0, 8)
                    Xcen_TAB(MINIBACIA) = linha.Substring(16, 15)
                    Ycen_TAB(MINIBACIA) = linha.Substring(31, 15)
                    Sub_TAB(MINIBACIA) = linha.Substring(46, 8)
                    Area_MINI_TAB(MINIBACIA) = linha.Substring(54, 15)
                    AreaM_TAB(MINIBACIA) = linha.Substring(69, 15)
                    Ltr_MINI_TAB(MINIBACIA) = linha.Substring(84, 15)
                    Str_MINI_TAB(MINIBACIA) = linha.Substring(99, 15)
                    Ltl_MINI_TAB(MINIBACIA) = linha.Substring(114, 15)
                    Stl_MINI_TAB(MINIBACIA) = linha.Substring(129, 15)
                    MiniJus_TAB(MINIBACIA) = linha.Substring(144, 8)
                    Ordem_MINI_TAB(MINIBACIA) = linha.Substring(152, 8)
                    Width(MINIBACIA) = linha.Substring(165, 10)
                    Depth(MINIBACIA) = linha.Substring(175, 10)
                    Manning(MINIBACIA) = linha.Substring(185, 10)

                    For nbloc As Integer = 1 To numerodeblocoscal
                        PBlocos(MINIBACIA, nbloc) = linha.Substring(195 + (nbloc - 1) * 10, 10)
                    Next
                End If
            Loop
            FileClose(1)

        End If

    End Sub

    Sub CARREGA_TABELA()

        ' Escrve tudo no DataGridView
        '   CatID    Mini           Xcen           Ycen     Sub     Area_(km2)    AreaM_(km2)       Ltr_(km)     Str_(m/km)       Lrl_(km)     Srl_(m/km) MiniJus   Ordem  Hdr    BLC_01    BLC_02    BLC_03    BLC_04    BLC_05    BLC_06    BLC_07    BLC_08    BLC_09

        dtgvMINI.Columns.Add("   CatID", "   CatID")
        dtgvMINI.Columns.Add("    Mini", "    Mini")
        dtgvMINI.Columns.Add("           Xcen", "           Xcen")
        dtgvMINI.Columns.Add("           Ycen", "           Ycen")
        dtgvMINI.Columns.Add("     Sub", "     Sub")
        dtgvMINI.Columns.Add("     Area_(km2)", "     Area_(km2)")
        dtgvMINI.Columns.Add("    AreaM_(km2)", "    AreaM_(km2)")
        dtgvMINI.Columns.Add("       Ltr_(km)", "       Ltr_(km)")
        dtgvMINI.Columns.Add("     Str_(m/km)", "     Str_(m/km)")
        dtgvMINI.Columns.Add("       Lrl_(km)", "       Lrl_(km)")
        dtgvMINI.Columns.Add("     Srl_(m/km)", "     Srl_(m/km)")
        dtgvMINI.Columns.Add(" MiniJus", " MiniJus")
        dtgvMINI.Columns.Add("   Ordem", "   Ordem")
        dtgvMINI.Columns.Add("  Hdr", "  Hdr")
        dtgvMINI.Columns.Add("     Width", "     Width")
        dtgvMINI.Columns.Add("     Depth", "     Depth")
        dtgvMINI.Columns.Add("      NMAN", "      NMAN")

        For coldataset As Integer = 1 To numerodeblocoscal

            dtgvMINI.Columns.Add("BLC_" & coldataset, "BLC_" & coldataset)
        Next

        For geracoes As Integer = 1 To intTOTAL_MINIBACIAS
            dtgvMINI.Rows.Add(1)
        Next

        btSALVARMini.Enabled = True

        'Escreve no datagrid as informações do MINI.MGB
        For MINIBACIA As Integer = 1 To intTOTAL_MINIBACIAS

            dtgvMINI.Item(0, MINIBACIA - 1).Value = CatID_MINI_TAB(MINIBACIA)
            dtgvMINI.Item(0, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(1, MINIBACIA - 1).Value = MINIBACIA
            dtgvMINI.Item(1, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(2, MINIBACIA - 1).Value = Xcen_TAB(MINIBACIA)
            dtgvMINI.Item(2, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(3, MINIBACIA - 1).Value = Ycen_TAB(MINIBACIA)
            dtgvMINI.Item(3, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(4, MINIBACIA - 1).Value = Sub_TAB(MINIBACIA)
            dtgvMINI.Item(4, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(5, MINIBACIA - 1).Value = Area_MINI_TAB(MINIBACIA)
            dtgvMINI.Item(5, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(6, MINIBACIA - 1).Value = AreaM_TAB(MINIBACIA)
            dtgvMINI.Item(6, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(7, MINIBACIA - 1).Value = Ltr_MINI_TAB(MINIBACIA)
            dtgvMINI.Item(7, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(8, MINIBACIA - 1).Value = Str_MINI_TAB(MINIBACIA)
            dtgvMINI.Item(8, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(9, MINIBACIA - 1).Value = Ltl_MINI_TAB(MINIBACIA)
            dtgvMINI.Item(9, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(10, MINIBACIA - 1).Value = Stl_MINI_TAB(MINIBACIA)
            dtgvMINI.Item(10, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(11, MINIBACIA - 1).Value = MiniJus_TAB(MINIBACIA)
            'dtgvMINI.Item(11, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(12, MINIBACIA - 1).Value = Ordem_MINI_TAB(MINIBACIA)
            dtgvMINI.Item(12, MINIBACIA - 1).ReadOnly = True
            dtgvMINI.Item(13, MINIBACIA - 1).Value = 0
            dtgvMINI.Item(14, MINIBACIA - 1).Value = Width(MINIBACIA)
            dtgvMINI.Item(15, MINIBACIA - 1).Value = Depth(MINIBACIA)
            dtgvMINI.Item(16, MINIBACIA - 1).Value = Manning(MINIBACIA)

            For coldataset As Integer = 1 To numerodeblocoscal
                dtgvMINI.Item(16 + coldataset, MINIBACIA - 1).Value = PBlocos(MINIBACIA, coldataset)
                'dtgvMINI.Item(16 + coldataset, MINIBACIA - 1).ReadOnly = True
            Next



        Next
    End Sub

    Private Sub Button5_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button5.Click
        ofd1.Filter = "MGB Hydrological Response Classes file (HRC)|*.HRC"
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
            btABRIRMINI.Enabled = True

            'pega os nomes do blocos!
            ReDim nomeblocos(numerodeblocoscal)
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

    Private Sub btSALVARMini_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btSALVARMini.Click
        'Rotina que atualiza os dados conforme a tabela
        Me.ATUALIZA_MINI()
        'Rotina que escreve os dados do MINI
        Me.EscreverMINI()
    End Sub

    Sub EscreverMINI()

        'Define o status da caixa de diálogo
        sfd1.Filter = "MGB geometry file (.gtp)|*.gtp"

        If sfd1.ShowDialog = Windows.Forms.DialogResult.OK Then

            'Variável auxiliar
            Dim Numero As String

            'Abre o arquivo
            FileOpen(1, sfd1.FileName, OpenMode.Output)

            'Escreve o cabeçalho
            PrintLine(1, "   CatID    Mini           Xcen           Ycen     Sub     Area_(km2)    AreaM_(km2)       Ltr_(km)     Str_(m/km)       Lrl_(km)     Srl_(m/km) MiniJus   Ordem  Hdr     Width     Depth      NMAN    BLC_01    BLC_02    BLC_03    BLC_04    BLC_05    BLC_06    BLC_07    BLC_08    BLC_09    BLC_10    BLC_11    BLC_12")

            'Escreve sequencialmente todo o arquivo de mini-bacias
            For MINIBACIA As Integer = 1 To intTOTAL_MINIBACIAS

                'Agora vai escrever cada uma das variáveis, para compor o MINI.MGB

                'CatID_MINI_TAB(MINIBACIA) = linha.Substring(0, 8)
                'Xcen_TAB(MINIBACIA) = linha.Substring(16, 15)
                'Ycen_TAB(MINIBACIA) = linha.Substring(31, 15)
                'Sub_TAB(MINIBACIA) = linha.Substring(46, 8)
                'Area_MINI_TAB(MINIBACIA) = linha.Substring(54, 15)
                'AreaM_TAB(MINIBACIA) = linha.Substring(69, 15)
                'Ltr_MINI_TAB(MINIBACIA) = linha.Substring(84, 15)
                'Str_MINI_TAB(MINIBACIA) = linha.Substring(99, 15)
                'Ltl_MINI_TAB(MINIBACIA) = linha.Substring(114, 15)
                'Stl_MINI_TAB(MINIBACIA) = linha.Substring(129, 15)
                'MiniJus_TAB(MINIBACIA) = linha.Substring(144, 8)
                'Ordem_MINI_TAB(MINIBACIA) = linha.Substring(152, 8)
                'For nbloc As Integer = 1 To numerodeblocoscal
                '    PBlocos(MINIBACIA, nbloc) = linha.Substring(165 + (nbloc - 1) * 10, 10)


                'Escreve o CatID (ÍNDICE)
                Print(1, CatID_MINI_TAB(MINIBACIA).ToString.PadLeft(8))

                'Escreve a mini-bacia (trecho de rio para o SIAQUA)
                Print(1, MINIBACIA.ToString.PadLeft(8))

                'Longitude
                Print(1, FormatNumber(Xcen_TAB(MINIBACIA), 5).PadLeft(15))

                'Latitude
                Print(1, FormatNumber(Ycen_TAB(MINIBACIA), 5).PadLeft(15))

                'Sub-bacia
                Print(1, Sub_TAB(MINIBACIA).ToString.PadLeft(8))

                'Área de Mini-bacia
                Numero = FormatNumber(Area_MINI_TAB(MINIBACIA), 5)
                If Numero >= 1000 And Numero < 10000 Then
                    Numero = Numero.Substring(0, 1) & Numero.Substring(2, 6)
                ElseIf Numero >= 10000 And Numero < 100000 Then
                    Numero = Numero.Substring(0, 2) & Numero.Substring(3, 6)
                ElseIf Numero >= 100000 And Numero < 1000000 Then
                    Numero = Numero.Substring(0, 3) & Numero.Substring(4, 6)
                ElseIf Numero >= 1000000 Then
                    Numero = Numero.Substring(0, 1) & Numero.Substring(2, 3) & Numero.Substring(6, 6)
                Else
                    Numero = Numero
                End If
                Print(1, Numero.PadLeft(15))
                ' Print(1, FormatNumber(Area_MINI_TAB(MINIBACIA), 5).PadLeft(15))

                'Área a montante da mini-bacia
                Numero = FormatNumber(AreaM_TAB(MINIBACIA), 5)
                If Numero >= 1000 And Numero < 10000 Then
                    Numero = Numero.Substring(0, 1) & Numero.Substring(2, 6)
                ElseIf Numero >= 10000 And Numero < 100000 Then
                    Numero = Numero.Substring(0, 2) & Numero.Substring(3, 6)
                ElseIf Numero >= 100000 And Numero < 1000000 Then
                    Numero = Numero.Substring(0, 3) & Numero.Substring(4, 6)
                ElseIf Numero >= 1000000 Then
                    Numero = Numero.Substring(0, 1) & Numero.Substring(2, 3) & Numero.Substring(6, 6)
                Else
                    Numero = Numero
                End If
                Print(1, Numero.PadLeft(15))
                'Print(1, FormatNumber(AreaM_TAB(MINIBACIA), 5).PadLeft(15))

                'Comprimento do trecho de rio
                Print(1, FormatNumber(Ltr_MINI_TAB(MINIBACIA), 5).PadLeft(15))

                'Declividade do trecho de rio
                Print(1, FormatNumber(Str_MINI_TAB(MINIBACIA), 5).PadLeft(15))

                'R
                Print(1, FormatNumber(Ltl_MINI_TAB(MINIBACIA), 5).PadLeft(15))

                'R
                Print(1, FormatNumber(Stl_MINI_TAB(MINIBACIA), 5).PadLeft(15))

                'Mini-bacia à jusante
                Print(1, MiniJus_TAB(MINIBACIA).ToString.PadLeft(8))

                'Ordem da mini-bacia
                Print(1, Ordem_MINI_TAB(MINIBACIA).ToString.PadLeft(8))

                'hdflag
                Print(1, "    0")

                'Largura do trecho da mini-bacia
                Print(1, Width(MINIBACIA).ToString.PadLeft(10))

                'Profundidade do trecho da mini-bacia
                Print(1, Depth(MINIBACIA).ToString.PadLeft(10))

                'Manning do trecho da mini-bacia
                Print(1, Manning(MINIBACIA).ToString.PadLeft(10))

                'Blocos
                For coldataset As Integer = 1 To numerodeblocoscal
                    Print(1, FormatNumber(PBlocos(MINIBACIA, coldataset), 5).PadLeft(10))
                Next

                'Completa o arquivo de geometria
                PrintLine(1)

            Next
            FileClose()

            MsgBox("File saved!", MsgBoxStyle.Information, "MGB-IPH")

        End If



    End Sub

    Sub ATUALIZA_MINI()

        'Essa rotina atualiza os arrays que contém as informações de cada trecho de rio
        Dim MINIBACIA As Integer

        'Copia os valores do DATAGRID
        For MINI As Integer = 1 To intTOTAL_MINIBACIAS
            MINIBACIA = dtgvMINI.Item(1, MINI - 1).Value
            CatID_MINI_TAB(MINIBACIA) = dtgvMINI.Item(0, MINI - 1).Value
            Xcen_TAB(MINIBACIA) = dtgvMINI.Item(2, MINI - 1).Value
            Ycen_TAB(MINIBACIA) = dtgvMINI.Item(3, MINI - 1).Value
            Sub_TAB(MINIBACIA) = dtgvMINI.Item(4, MINI - 1).Value
            Area_MINI_TAB(MINIBACIA) = dtgvMINI.Item(5, MINI - 1).Value
            AreaM_TAB(MINIBACIA) = dtgvMINI.Item(6, MINI - 1).Value
            Ltr_MINI_TAB(MINIBACIA) = dtgvMINI.Item(7, MINI - 1).Value
            Str_MINI_TAB(MINIBACIA) = dtgvMINI.Item(8, MINI - 1).Value
            Ltl_MINI_TAB(MINIBACIA) = dtgvMINI.Item(9, MINI - 1).Value
            Stl_MINI_TAB(MINIBACIA) = dtgvMINI.Item(10, MINI - 1).Value
            MiniJus_TAB(MINIBACIA) = dtgvMINI.Item(11, MINI - 1).Value
            Ordem_MINI_TAB(MINIBACIA) = dtgvMINI.Item(12, MINI - 1).Value
            'hdflag is alwais zero
            Width(MINIBACIA) = dtgvMINI.Item(14, MINI - 1).Value
            Depth(MINIBACIA) = dtgvMINI.Item(15, MINI - 1).Value
            Manning(MINIBACIA) = dtgvMINI.Item(16, MINI - 1).Value

            For coldataset As Integer = 1 To numerodeblocoscal
                PBlocos(MINIBACIA, coldataset) = dtgvMINI.Item(16 + coldataset, MINI - 1).Value
            Next

        Next

    End Sub

    Private Sub Editor_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load

    End Sub

    Private Sub btFECHAR_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btFECHAR.Click
        If MsgBox("Are you sure?", MsgBoxStyle.YesNo, "Close") = MsgBoxResult.Yes Then
            Me.Close()
        End If
    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click

        If MsgBox("Are you sure?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then

            'Escreve no datagrid as informações do MINI.MGB
            For MINIBACIA As Integer = 1 To intTOTAL_MINIBACIAS

                dtgvMINI.Item(16, MINIBACIA - 1).Value = tbMANNING.Text

            Next

        End If

    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        If MsgBox("Are you sure?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then

            'Escreve no datagrid as informações do MINI.MGB
            For MINIBACIA As Integer = 1 To intTOTAL_MINIBACIAS

                dtgvMINI.Item(14, MINIBACIA - 1).Value = FormatNumber(tbW_a.Text * (dtgvMINI.Item(6, MINIBACIA - 1).Value) ^ tbW_b.Text, 2)

            Next

        End If
    End Sub

    Private Sub Button3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button3.Click
        If MsgBox("Are you sure?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then

            'Escreve no datagrid as informações do MINI.MGB
            For MINIBACIA As Integer = 1 To intTOTAL_MINIBACIAS

                dtgvMINI.Item(15, MINIBACIA - 1).Value = FormatNumber(tbD_a.Text * (dtgvMINI.Item(6, MINIBACIA - 1).Value) ^ tbD_b.Text, 2)

            Next

        End If
    End Sub

    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click

        If MsgBox("Are you sure?", MsgBoxStyle.YesNo) = MsgBoxResult.Yes Then

            'Escreve no datagrid as informações do MINI.MGB
            For MINIBACIA As Integer = 1 To intTOTAL_MINIBACIAS

                If dtgvMINI.Item(4, MINIBACIA - 1).Value = tb_RUG_SelectSubb.Text Then
                    dtgvMINI.Item(16, MINIBACIA - 1).Value = tbRUGMAN_subb.Text
                End If

            Next

        End If
    End Sub

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        Dim opendiag2 As New Windows.Forms.OpenFileDialog
        opendiag2.Filter = "CSV ; separated table (.csv)|*.csv"
        Dim linha As String 'String de leitura
        Dim splt
        If opendiag2.ShowDialog = Windows.Forms.DialogResult.OK Then
            FileOpen(1, opendiag2.FileName, OpenMode.Input)
            Do Until EOF(1)
                linha = LineInput(1)
                splt = Split(linha, ";")
                Width(splt(0)) = splt(1)
                Depth(splt(0)) = splt(2)
            Loop
            FileClose()

            'Escreve no datagrid as informações do MINI.MGB
            For MINIBACIA As Integer = 1 To intTOTAL_MINIBACIAS
                dtgvMINI.Item(14, MINIBACIA - 1).Value = Width(dtgvMINI.Item(1, MINIBACIA - 1).Value)
                dtgvMINI.Item(15, MINIBACIA - 1).Value = Depth(dtgvMINI.Item(1, MINIBACIA - 1).Value)
            Next

            MsgBox("File loaded")

        End If
    End Sub
End Class