Imports System.Drawing

Public Class Form6

    Private Sub Form6_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load

        Me.LOAD_MENSAL() 'Gantt Mensal
        'Me.LOAD_ANUAL 'Gantt Anual

    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        sfd.Filter = "Imagem (.bmp)|*.bmp"
        If sfd.ShowDialog = Windows.Forms.DialogResult.OK Then
            Dim path As String
            path = sfd.FileName
            'Dim img = New Bitmap(picImagem.Image)
            bmparea2.Save(path, System.Drawing.Imaging.ImageFormat.Bmp)
            MsgBox("Imagem gerada com sucesso!")
        End If
    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Me.Close()
    End Sub

    Private Sub Form6_Paint(ByVal sender As Object, ByVal e As System.Windows.Forms.PaintEventArgs) Handles Me.Paint
        e.Graphics.DrawImage(bmparea2, 0, 0)

    End Sub

    Sub LOAD_MENSAL()
        Dim pmax As Integer
        pmax = total_estações
        'tirar 
        'dataI = "01 /01/1950"

        Try
            Esp = 0
            xi = Panel1.Location.X + Esp
            xf = Panel1.Location.X + Panel1.Width - Esp
            yi = Panel1.Location.Y + Panel1.Height - Esp
            yf = Panel1.Location.Y + Esp

            'bmparea2 = New Bitmap(Windows.Forms.Form.ActiveForm.Size.Width, Windows.Forms.Form.ActiveForm.Size.Height)
            bmparea2 = New Bitmap(852, 704)

            'dataI = Now.Date
            'dataF = "01/01/1900"
            'For p = 1 To pmax
            '    For t = 1 To tmax(p)
            '        If data(p, t) <= dataI Then
            '            dataI = data(p, t)
            '        End If
            '        If data(p, t) >= dataF Then
            '            dataF = data(p, t)
            '        End If
            '    Next
            'Next

            graficoArea2 = Graphics.FromImage(bmparea2)
            graficoArea2.Clear(Me.BackColor)

            Dim Alt_Rec As Single

            Dim dif As Double
            dif = DateDiff(DateInterval.Month, dataI, dataF)
            'desenha-se ano cheio (meramente informativo)
            Larg_Rec = ((xf - xi) / (dif + 1)) '((DateDiff(DateInterval.Year, dataI, dataF) + 1)))
            Alt_Rec = ((yi - yf) / total_estações)


            Dim p_med(total_estações, 12, Now.Year) As Single 'quantidade de dias OK no ano e posto em questão

            'inicio do desenho: grafica eixo horizontal
            graficoArea2.DrawLine(New Pen(Color.Black, 2), xi, yi + 2, xf, yi + 2)

            Dim mesinicio As Integer
            mesinicio = 1
            Dim mesfim As Integer
            mesfim = 12
            Dim contames As Integer
            contames = 0
            Dim contaano As Integer

            'RGB - Red; Green; Blue / de 0 a 255
            For p = 1 To total_estações
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                'MsgBox(dataF.Year & " - " & dataI.Year)
                contaano = -1
                contames = -1
                For aaaa = dataI.Year To dataF.Year

                    contaano = contaano + 1
                    If aaaa = dataI.Year Then
                        mesinicio = dataI.Month
                    ElseIf aaaa > dataI.Year Then
                        mesinicio = 1
                    End If

                    If aaaa = dataF.Year Then
                        mesfim = dataF.Month
                    ElseIf aaaa < dataF.Year Then
                        mesfim = 12
                    End If
                    'contames = contaano - 1


                    '-------Loop Padrao modificado INI----------------------------------------------------------------
                    For mm = mesinicio To mesfim
                        contames = contames + 1
                        p_med(p, mm, aaaa) = 0 'p, data(p, t).Year) = 0

                        For dd = 1 To System.DateTime.DaysInMonth(aaaa, mm)
                            For hh As Integer = 0 To 23
                                If Qdispo(p, hh, dd, mm, Ano(aaaa)) > -1 Then
                                    p_med(p, mm, aaaa) = p_med(p, mm, aaaa) + 1 '
                                End If 'soma quantidade de dias OK no ano e posto em questão
                            Next
                        Next

                        '-------Loop Padrao modificado FIM----------------------------------------------------------------
                        p_med(p, mm, aaaa) = p_med(p, mm, aaaa) / ((System.DateTime.DaysInMonth(aaaa, mm)) * 24)
                        'MsgBox(p_med(p, aaaa))


                        xloc = xi + ((contames) * Larg_Rec)
                        '   xloc = xi + (((aaaa - dataI.Year) + mm - 1) * Larg_Rec)
                        'MsgBox(xloc & " - " & yloc)
                        'MsgBox(p_med(p, t))
                        If p_med(p, mm, aaaa) <= 0.5 Then 'de vermelho a amarelo
                            Dim Caneta = New Pen(Color.FromArgb(0, 0, (255 / 50) * p_med(p, mm, aaaa) * 100))
                            Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, (255 / 50) * p_med(p, mm, aaaa) * 100))
                            graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                            graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                        ElseIf p_med(p, mm, aaaa) > 0.5 Then 'de amarelo ao verde
                            Dim Caneta = New Pen(Color.FromArgb(0, 0, 255 - (255 / 50) * (1 - p_med(p, mm, aaaa)) * 100))
                            Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, 255 - (255 / 50) * (1 - p_med(p, mm, aaaa)) * 100))
                            graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                            graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                        End If
                        If p = pmax Then  'somente depois do grafico pronto
                            graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xloc, yi + 15, xloc, yf)
                            graficoArea2.DrawString(CStr(mm & "/" & aaaa - (aaaa \ 100) * 100), New Font("Arial", 7), Brushes.Gray, xloc + 0.5, yi + 10)
                        End If
                    Next
                Next
            Next

            For p = 1 To pmax
                'aqui se desenha linhas e textos horizontais
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xi - 50, yloc, xf, yloc)

                'Dim loc As Integer
                'loc = 0
                'For aux = 0 To Arqu(p).Length - 4
                'MsgBox("OK0")
                'If Arqu(p).Chars(aux) = ("\"c) Then
                'MsgBox("OK")
                'Loc = aux + 1
                'End If
                'Next
                'Arq(p) = Arq(p).Substring(loc, Arq(p).Length - loc)
                'If Loc() > 0 Then graficoArea2.DrawString(Arqu(p).Substring(Loc, Arqu(p).Length - Loc()), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)
                'If Loc() = 0 Then graficoArea2.DrawString(Arqu(p), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)

                graficoArea2.DrawString(NOME(p), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)
            Next

            Invalidate()
        Catch ex As Exception
            MsgBox(ex.ToString)
        End Try



    End Sub

    Sub LOAD_MENSAL_OLD()
        Dim pmax As Integer
        pmax = total_estações
        'tirar 
        'dataI = "01 /01/1950"

        Try
            Esp = 0
            xi = Panel1.Location.X + Esp
            xf = Panel1.Location.X + Panel1.Width - Esp
            yi = Panel1.Location.Y + Panel1.Height - Esp
            yf = Panel1.Location.Y + Esp

            'bmparea2 = New Bitmap(Windows.Forms.Form.ActiveForm.Size.Width, Windows.Forms.Form.ActiveForm.Size.Height)
            bmparea2 = New Bitmap(852, 704)

            'dataI = Now.Date
            'dataF = "01/01/1900"
            'For p = 1 To pmax
            '    For t = 1 To tmax(p)
            '        If data(p, t) <= dataI Then
            '            dataI = data(p, t)
            '        End If
            '        If data(p, t) >= dataF Then
            '            dataF = data(p, t)
            '        End If
            '    Next
            'Next

            graficoArea2 = Graphics.FromImage(bmparea2)
            graficoArea2.Clear(Me.BackColor)

            Dim Alt_Rec As Single

            Dim dif As Double
            dif = DateDiff(DateInterval.Month, dataI, dataF)
            'desenha-se ano cheio (meramente informativo)
            Larg_Rec = ((xf - xi) / (dif + 1)) '((DateDiff(DateInterval.Year, dataI, dataF) + 1)))
            Alt_Rec = ((yi - yf) / total_estações)


            Dim p_med(total_estações, 12, Now.Year) As Single 'quantidade de dias OK no ano e posto em questão

            'inicio do desenho: grafica eixo horizontal
            graficoArea2.DrawLine(New Pen(Color.Black, 2), xi, yi + 2, xf, yi + 2)

            Dim mesinicio As Integer
            mesinicio = 1
            Dim mesfim As Integer
            mesfim = 12
            Dim contames As Integer
            contames = 0
            Dim contaano As Integer

            'RGB - Red; Green; Blue / de 0 a 255
            For p = 1 To total_estações
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                'MsgBox(dataF.Year & " - " & dataI.Year)
                contaano = -1
                For aaaa = dataI.Year To dataF.Year

                    contaano = contaano + 1
                    If aaaa = dataI.Year Then
                        mesinicio = dataI.Month
                    ElseIf aaaa > dataI.Year Then
                        mesinicio = 1
                    End If

                    If aaaa = dataF.Year Then
                        mesfim = dataF.Month
                    ElseIf aaaa < dataF.Year Then
                        mesfim = 12
                    End If
                    contames = contaano - 1


                    '-------Loop Padrao modificado INI----------------------------------------------------------------
                    For mm = mesinicio To mesfim
                        contames = contames + 1
                        p_med(p, mm, aaaa) = 0 'p, data(p, t).Year) = 0

                        For dd = 1 To System.DateTime.DaysInMonth(aaaa, mm)
                            For hh As Integer = 0 To 23
                                If Qdispo(p, hh, dd, mm, Ano(aaaa)) > -1 Then
                                    p_med(p, mm, aaaa) = p_med(p, mm, aaaa) + 1 '
                                End If 'soma quantidade de dias OK no ano e posto em questão
                            Next
                        Next

                        '-------Loop Padrao modificado FIM----------------------------------------------------------------
                        p_med(p, mm, aaaa) = p_med(p, mm, aaaa) / ((System.DateTime.DaysInMonth(aaaa, mm)) * 24)
                        'MsgBox(p_med(p, aaaa))


                        xloc = xi + ((contames) * Larg_Rec)
                        '   xloc = xi + (((aaaa - dataI.Year) + mm - 1) * Larg_Rec)
                        'MsgBox(xloc & " - " & yloc)
                        'MsgBox(p_med(p, t))
                        If p_med(p, mm, aaaa) <= 0.5 Then 'de vermelho a amarelo
                            Dim Caneta = New Pen(Color.FromArgb(0, 0, (255 / 50) * p_med(p, mm, aaaa) * 100))
                            Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, (255 / 50) * p_med(p, mm, aaaa) * 100))
                            graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                            graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                        ElseIf p_med(p, mm, aaaa) > 0.5 Then 'de amarelo ao verde
                            Dim Caneta = New Pen(Color.FromArgb(0, 0, 255 - (255 / 50) * (1 - p_med(p, mm, aaaa)) * 100))
                            Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, 255 - (255 / 50) * (1 - p_med(p, mm, aaaa)) * 100))
                            graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                            graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                        End If
                        If p = pmax Then  'somente depois do grafico pronto
                            graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xloc, yi + 15, xloc, yf)
                            graficoArea2.DrawString(CStr(mm & "/" & aaaa - (aaaa \ 100) * 100), New Font("Arial", 7), Brushes.Gray, xloc + 0.5, yi + 10)
                        End If
                    Next
                Next
            Next

            For p = 1 To pmax
                'aqui se desenha linhas e textos horizontais
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xi - 50, yloc, xf, yloc)

                'Dim loc As Integer
                'loc = 0
                'For aux = 0 To Arqu(p).Length - 4
                'MsgBox("OK0")
                'If Arqu(p).Chars(aux) = ("\"c) Then
                'MsgBox("OK")
                'Loc = aux + 1
                'End If
                'Next
                'Arq(p) = Arq(p).Substring(loc, Arq(p).Length - loc)
                'If Loc() > 0 Then graficoArea2.DrawString(Arqu(p).Substring(Loc, Arqu(p).Length - Loc()), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)
                'If Loc() = 0 Then graficoArea2.DrawString(Arqu(p), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)

                graficoArea2.DrawString(NOME(p), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)
            Next

            Invalidate()
        Catch ex As Exception
            MsgBox(ex.ToString)
        End Try



    End Sub

    Sub LOAD_ANUAL()

        Dim pmax As Integer
        pmax = total_estações
        'tirar 
        'dataI = "01 /01/1950"

        Try
            Esp = 0
            xi = Panel1.Location.X + Esp
            xf = Panel1.Location.X + Panel1.Width - Esp
            yi = Panel1.Location.Y + Panel1.Height - Esp
            yf = Panel1.Location.Y + Esp

            'bmparea2 = New Bitmap(Windows.Forms.Form.ActiveForm.Size.Width, Windows.Forms.Form.ActiveForm.Size.Height)
            bmparea2 = New Bitmap(852, 704)

            'dataI = Now.Date
            'dataF = "01/01/1900"
            'For p = 1 To pmax
            '    For t = 1 To tmax(p)
            '        If data(p, t) <= dataI Then
            '            dataI = data(p, t)
            '        End If
            '        If data(p, t) >= dataF Then
            '            dataF = data(p, t)
            '        End If
            '    Next
            'Next

            graficoArea2 = Graphics.FromImage(bmparea2)
            graficoArea2.Clear(Me.BackColor)

            Dim Alt_Rec As Single

            'desenha-se ano cheio (meramente informativo)
            Larg_Rec = ((xf - xi) / (dataF.Year - dataI.Year + 1)) '((DateDiff(DateInterval.Year, dataI, dataF) + 1)))
            Alt_Rec = ((yi - yf) / total_estações)


            Dim p_med(total_estações, Now.Year) As Single 'quantidade de dias OK no ano e posto em questão

            'inicio do desenho: grafica eixo horizontal
            graficoArea2.DrawLine(New Pen(Color.Black, 2), xi, yi + 2, xf, yi + 2)

            'RGB - Red; Green; Blue / de 0 a 255
            For p = 1 To total_estações
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                'MsgBox(dataF.Year & " - " & dataI.Year)
                For aaaa = dataI.Year To dataF.Year
                    p_med(p, aaaa) = 0 'p, data(p, t).Year) = 0
                    '-------Loop Padrao modificado INI----------------------------------------------------------------
                    For mm = 1 To 12
                        For dd = 1 To System.DateTime.DaysInMonth(aaaa, mm)
                            For hh As Integer = 0 To 23
                                If Qdispo(p, hh, dd, mm, Ano(aaaa)) > -1 Then
                                    p_med(p, aaaa) = p_med(p, aaaa) + 1 '
                                End If 'soma quantidade de dias OK no ano e posto em questão
                            Next
                        Next
                    Next
                    '-------Loop Padrao modificado FIM----------------------------------------------------------------
                    p_med(p, aaaa) = p_med(p, aaaa) / ((System.DateTime.DaysInMonth(aaaa, 2) + 337) * 24)
                    'MsgBox(p_med(p, aaaa))

                    xloc = xi + ((aaaa - dataI.Year) * Larg_Rec)
                    'MsgBox(xloc & " - " & yloc)
                    'MsgBox(p_med(p, t))
                    If p_med(p, aaaa) <= 0.5 Then 'de vermelho a amarelo
                        Dim Caneta = New Pen(Color.FromArgb(0, 0, (255 / 50) * p_med(p, aaaa) * 100))
                        Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, (255 / 50) * p_med(p, aaaa) * 100))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_med(p, aaaa) > 0.5 Then 'de amarelo ao verde
                        Dim Caneta = New Pen(Color.FromArgb(0, 0, 255 - (255 / 50) * (1 - p_med(p, aaaa)) * 100))
                        Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, 255 - (255 / 50) * (1 - p_med(p, aaaa)) * 100))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    End If
                    If p = pmax Then  'somente depois do grafico pronto
                        graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xloc, yi + 15, xloc, yf)
                        graficoArea2.DrawString(CStr(aaaa - (aaaa \ 100) * 100), New Font("Arial", 7), Brushes.Gray, xloc + 0.5, yi + 10)
                    End If
                Next
            Next

            For p = 1 To pmax
                'aqui se desenha linhas e textos horizontais
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xi - 50, yloc, xf, yloc)

                'Dim loc As Integer
                'loc = 0
                'For aux = 0 To Arqu(p).Length - 4
                'MsgBox("OK0")
                'If Arqu(p).Chars(aux) = ("\"c) Then
                'MsgBox("OK")
                'Loc = aux + 1
                'End If
                'Next
                'Arq(p) = Arq(p).Substring(loc, Arq(p).Length - loc)
                'If Loc() > 0 Then graficoArea2.DrawString(Arqu(p).Substring(Loc, Arqu(p).Length - Loc()), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)
                'If Loc() = 0 Then graficoArea2.DrawString(Arqu(p), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)

                graficoArea2.DrawString(NOME(p), New Font("Arial", 7), Brushes.Gray, xi - 55, yloc + 1)
            Next

            Invalidate()
        Catch ex As Exception
            MsgBox(ex.ToString)
        End Try


    End Sub
End Class