Imports System.Globalization
Imports System.IO
Imports System.Text

Public Class Form9

    Private Sub DataGridView1_CellContentClick(ByVal sender As System.Object, ByVal e As System.Windows.Forms.DataGridViewCellEventArgs) Handles DataGridView1.CellContentClick

    End Sub

    Private Sub Form9_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        Dim difft As Integer
        difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

        Dim datacomeço As Date
        datacomeço = dataI.AddHours(-1)


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
            nova_linha.ItemArray = New Object() {dia, mes, datacomeço.Year, hora, Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))}

            Try
                DataSet1.Tables(0).Rows.Add(nova_linha)
            Catch ex As Exception
                MsgBox(" erro: " & ex.Message)
            End Try


        Next




    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Me.ATUALIZA_DADOS()
        If Tb_vazao = True Then
            'ESCREVEDADOVAZÃO_HORA()
            ESCREVEDADOVAZÃO_HORA_Legado()
        ElseIf Tb_chuva Then
            'ESCREVEDADOCHUVA_HORA()
            ESCREVEDADOCHUVA_HORA_Legado()

        End If

        Me.Close()
    End Sub

    '--NOVAS ROTINAS DE ESCRITA OTIMIZADAS

    Private Function Alinha(valor As String, largura As Integer) As String
        Return valor.PadLeft(largura)
    End Function

    Private Function FormataValor(valor As Double) As String
        Const LARGURA As Integer = 16
        If Double.IsNaN(valor) OrElse Double.IsInfinity(valor) Then
            Return Alinha("-1.000000", LARGURA)
        End If
        If valor < 0 Then
            Return Alinha("-1.000000", LARGURA)
        End If
        Dim s = valor.ToString("0.000000", CultureInfo.InvariantCulture)
        Return Alinha(s, LARGURA)
    End Function

    ' Reaproveite Alinha e FormataValor acima

    Sub ESCREVEDADOVAZÃO_HORA_Legado()
        Const L6 As Integer = 6

        Dim tmaxx As Integer = DataGridView1.RowCount - 1
        Dim datacomeco As Date = dataI
        Dim arquivo_est2 As String = Nome_arq

        FileOpen(2, arquivo_est2, OpenMode.Output)

        PrintLine(2,
        Alinha("Dia", L6) &
        Alinha("Mês", L6) &
        Alinha("Ano", L6) &
        Alinha("Hora", L6) &
        Alinha(Nome_p, 16)
    )

        For i As Integer = 1 To tmaxx
            If i <> 1 Then datacomeco = datacomeco.AddHours(1)

            Dim dia = Alinha(datacomeco.Day.ToString("00"), L6)
            Dim mes = Alinha(datacomeco.Month.ToString("00"), L6)
            Dim ano_calend = Alinha(datacomeco.Year.ToString(), L6)
            Dim hora = Alinha(datacomeco.Hour.ToString("00"), L6)

            Dim v As Double = Tabela(datacomeco.Hour, datacomeco.Day, datacomeco.Month, Ano(datacomeco.Year))
            Dim vFmt As String = FormataValor(v)

            PrintLine(2, dia & mes & ano_calend & hora & vFmt)
        Next

        FileClose(2)
    End Sub

    ' Reaproveite Alinha e FormataValor acima

    Sub ESCREVEDADOCHUVA_HORA_Legado()
        Const L6 As Integer = 6

        Dim tmaxx As Integer = DataGridView1.RowCount - 1
        Dim datacomeco As Date = dataI
        Dim arquivo As String = Nome_arq

        FileOpen(2, arquivo, OpenMode.Output)

        PrintLine(2,
        Alinha("Dia", L6) &
        Alinha("Mês", L6) &
        Alinha("Ano", L6) &
        Alinha("Hora", L6) &
        Alinha(Nome_p, 16)
    )

        For i As Integer = 1 To tmaxx
            If i <> 1 Then datacomeco = datacomeco.AddHours(1)

            Dim dia = Alinha(datacomeco.Day.ToString("00"), L6)
            Dim mes = Alinha(datacomeco.Month.ToString("00"), L6)
            Dim ano_calend = Alinha(datacomeco.Year.ToString(), L6)
            Dim hora = Alinha(datacomeco.Hour.ToString("00"), L6)

            Dim v As Double = Tabela(datacomeco.Hour, datacomeco.Day, datacomeco.Month, Ano(datacomeco.Year))
            Dim vFmt As String = FormataValor(v)

            PrintLine(2, dia & mes & ano_calend & hora & vFmt)
        Next

        FileClose(2)
    End Sub



    Sub ESCREVEDADOVAZÃO_HORA()

        'Calcula datas
        ' Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        ' Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"


        Dim tmaxx As Integer
        tmaxx = DataGridView1.RowCount - 1
        'tmaxx = dif. de tempo

        Dim datacomeço As Date
        datacomeço = dataI
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = Nome_arq

        'Dim arquivo_est3 As String
        'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

        FileOpen(2, arquivo_est2, OpenMode.Output)
        'FileOpen(3, arquivo_est3, OpenMode.Output)

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

            'If FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar
            'End If

            If SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                'não diminui dois espaços pois não tem o separador de milhar
            End If


            PrintLine(2)

        Next

        FileClose()


    End Sub

    Sub ESCREVEDADOCHUVA_HORA()

        'Calcula datas
        ' Dim dataO As Date = dtpqob1.Value.Day & "/" & dtpqob1.Value.Month & "/" & dtpqob1.Value.Year & " " & tb_hora_ini.Text & ":00:00"
        ' Dim dataE As Date = dtpqob2.Value.Day & "/" & dtpqob2.Value.Month & "/" & dtpqob2.Value.Year & " " & tb_hora_fim.Text & ":00:00"


        Dim tmaxx As Integer
        tmaxx = DataGridView1.RowCount - 1
        'tmaxx = dif. de tempo

        Dim datacomeço As Date
        datacomeço = dataI
        'datacomeço.AddDays(1)

        'define o arquivo
        Dim arquivo_est2 As String
        arquivo_est2 = Nome_arq

        'Dim arquivo_est3 As String
        'arquivo_est3 = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\database\Q" & Estação(p) & ".txt"

        FileOpen(2, arquivo_est2, OpenMode.Output)
        'FileOpen(3, arquivo_est3, OpenMode.Output)

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

            'If FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 0 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 10 Then
            '    Print(2, "        " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 10 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 100 Then
            '    Print(2, "       " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 100 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 1000 Then
            '    Print(2, "      " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 1000 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 10000 Then
            '    Print(2, "     " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(0, 1) & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(2, 10))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 10000 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 100000 Then
            '    Print(2, "    " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(0, 2) & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(3, 10))
            'ElseIf FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) >= 100000 And FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2) < 1000000 Then
            '    Print(2, "   " & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(0, 3) & FormatNumber(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 2).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar
            'End If

            If SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
            ElseIf SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(Tabela(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))))
                'não diminui dois espaços pois não tem o separador de milhar
            End If

            PrintLine(2)

        Next

        FileClose()


    End Sub

    Sub ATUALIZA_DADOS()

        Dim tmaxx As Integer
        tmaxx = DataGridView1.RowCount - 1
        'tmaxx = dif. de tempo

        Dim dia As Integer
        Dim mes As Integer
        Dim anos As Integer
        Dim hora As Integer
        Dim valor As Double

        For aux = 1 To tmaxx
            dia = DataGridView1.Item(0, aux - 1).Value
            mes = DataGridView1.Item(1, aux - 1).Value
            anos = DataGridView1.Item(2, aux - 1).Value
            hora = DataGridView1.Item(3, aux - 1).Value
            valor = DataGridView1.Item(4, aux - 1).Value
            Tabela(hora, dia, mes, Ano(anos)) = AjustarDecimal(valor)
        Next
    End Sub
End Class