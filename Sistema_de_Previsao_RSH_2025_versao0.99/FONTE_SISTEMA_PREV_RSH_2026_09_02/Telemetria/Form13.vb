Imports System.Drawing
Imports System.Drawing.Imaging
Imports System.IO
Imports System.Windows.Forms

Public Class Form13



    'Dimensiona as variáveis para cálculo de estatísticas
    Dim Media(total_estações) As Single 'Média
    Dim Desv_pad(total_estações) As Single 'Desio Padrão
    Dim Max(total_estações) As Single 'Máxima da série
    Dim Min(total_estações) As Single 'Mínima da série
    Dim MaxAnual(total_estações, Ano(Now.Year)) As Single 'Máxima anual
    Dim MinAnual(total_estações, Ano(Now.Year)) As Single 'Mínima anual

    'Private Sub Form13_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load

    '    ' Me.LOAD_MENSAL() 'Gantt Mensal
    '    'Me.LOAD_ANUAL 'Gantt Anual

    '    'Verifica se é chuva ou vazão
    '    If Q_VERIFICA = True Then ' Vazão
    '        Me.CALCULAR_Estatísticas()
    '        Me.LOAD_HORÁRIO_Q()
    '        Label4.Visible = True
    '    Else
    '        Me.LOAD_HORÁRIO()
    '    End If



    'End Sub

    Private Sub Form13_Load(sender As Object, e As EventArgs) Handles MyBase.Load
        ' 1) Desliga eventuais Paints legados que desenham bmparea2 no Form/Panel
        'Try : RemoveHandler Me.Paint, AddressOf Form13_Paint : Catch : End Try
        'Try : RemoveHandler Panel1.Paint, AddressOf Panel1_Paint : Catch : End Try

        ' 2) Double-buffering (Form e Panel) para reduzir flicker
        Me.DoubleBuffered = True
        Try
            Dim pi = GetType(Panel).GetProperty("DoubleBuffered",
            Reflection.BindingFlags.Instance Or Reflection.BindingFlags.NonPublic)
            If pi IsNot Nothing Then pi.SetValue(Panel1, True, Nothing)
        Catch : End Try

        ' 3) Preparação do painel (scroll fica por conta do método)
        Panel1.AutoScroll = True
        If Q_VERIFICA Then
            Me.CALCULAR_Estatísticas()
            Me.LOAD_HORÁRIO_Q_SCROLL()
            Label4.Visible = True
        Else
            Me.LOAD_HORÁRIO_Scroll() ' sua rotina de chuva
        End If

        ' Me.DEBUG_GANTT_VIEW()
    End Sub


   

    Private Sub GenerateGanttBmp(filePath As String)
        ' ---- parâmetros visuais ----
        Const ROW_H As Integer = 18
        Const COL_W As Integer = 10
        Const HORAS As Integer = 72
        Const LEFT_LABEL As Integer = 140
        Const TOP_M As Integer = 20
        Const RIGHT_M As Integer = 20
        Const BOTTOM_M As Integer = 30

        Dim pmax As Integer = Math.Max(1, total_estações)

        ' dimensões do canvas
        Dim contentW As Integer = LEFT_LABEL + HORAS * COL_W + RIGHT_M
        Dim contentH As Integer = TOP_M + pmax * ROW_H + BOTTOM_M
        If contentW < 50 Then contentW = 50
        If contentH < 50 Then contentH = 50

        Dim bmp As New Bitmap(contentW, contentH)
        Using g As Graphics = Graphics.FromImage(bmp)
            g.Clear(Color.White)
            g.TextRenderingHint = Drawing.Text.TextRenderingHint.ClearTypeGridFit

            Using penAxis As New Pen(Color.Black, 1),
              penGrid As New Pen(Color.Gainsboro, 1),
              fntLab As New Font("Segoe UI", 8, FontStyle.Regular),
              fntTick As New Font("Segoe UI", 7, FontStyle.Regular),
              brBlack As New SolidBrush(Color.Black),
              brGreen As New SolidBrush(Color.FromArgb(51, 204, 0)),
              brYellow As New SolidBrush(Color.FromArgb(255, 255, 0)),
              brOrange As New SolidBrush(Color.FromArgb(255, 165, 0)),
              brRed As New SolidBrush(Color.FromArgb(225, 0, 0)),
              brGrayText As New SolidBrush(Color.DimGray)

                Dim brMap As SolidBrush() = {brGreen, brYellow, brOrange, brRed}

                ' borda do canvas (diagnóstico visual)
                Using penBorder As New Pen(Color.Silver, 1)
                    g.DrawRectangle(penBorder, 0, 0, bmp.Width - 1, bmp.Height - 1)
                End Using

                Dim baseY As Integer = TOP_M + pmax * ROW_H
                g.DrawLine(penAxis, LEFT_LABEL, baseY, contentW - RIGHT_M, baseY)

                ' data base
                Dim fim As Date = If(dataF = Date.MinValue, Now, dataF)
                Dim dt0 As Date = fim.AddHours(-HORAS)

                ' grid vertical + rótulos
                For h As Integer = 1 To HORAS
                    Dim x As Integer = LEFT_LABEL + (h - 1) * COL_W
                    If h = 1 OrElse h Mod 6 = 0 Then
                        g.DrawLine(penGrid, x, TOP_M, x, baseY)
                        g.DrawString(h.ToString(), fntTick, brGrayText, x + 1, baseY + 4)
                    End If
                Next

                ' linhas/estações
                For p As Integer = 1 To pmax
                    Dim y As Integer = TOP_M + (p - 1) * ROW_H
                    g.DrawLine(penGrid, LEFT_LABEL, y, contentW - RIGHT_M, y)

                    ' nome da estação
                    Dim nome As String = $"Estação {p}"
                    Try
                        If Not String.IsNullOrWhiteSpace(nome(p)) Then nome = nome(p)
                    Catch
                    End Try
                    Dim nameY As Integer = y + (ROW_H - fntLab.Height) \ 2
                    g.DrawString(nome, fntLab, brGrayText, 4, nameY)

                    ' 72h de blocos
                    Dim dt As Date = dt0
                    For h As Integer = 1 To HORAS
                        dt = dt.AddHours(1)
                        Dim cls As Integer
                        Try
                            Dim qv As Double = Qdispo(p, dt.Hour, dt.Day, dt.Month, Ano(dt.Year))
                            If qv < 0 Then
                                cls = -1
                            Else
                                Dim m As Double = Media(p)
                                Dim d As Double = Desv_pad(p)
                                If qv < m + d Then cls = 0 _
                            Else If qv < m + 2 * d Then cls = 1 _
                            Else If qv < m + 3 * d Then cls = 2 _
                            Else If qv >= m + 4 * d Then cls = 3 _
                            Else cls = 2
                            End If
                        Catch
                            cls = -2 ' erro -> não pinta
                        End Try

                        Dim x As Integer = LEFT_LABEL + (h - 1) * COL_W
                        Dim rect As New Rectangle(x, y + 1, COL_W - 1, ROW_H - 2)

                        If cls = -1 Then
                            g.FillRectangle(brBlack, rect)
                        ElseIf cls >= 0 Then
                            g.FillRectangle(brMap(Math.Min(cls, brMap.Length - 1)), rect)
                        End If
                    Next
                Next
            End Using
        End Using

        ' salva (PNG evita perdas)
        Dim dir = Path.GetDirectoryName(filePath)
        If Not Directory.Exists(dir) Then Directory.CreateDirectory(dir)
        bmp.Save(filePath, ImageFormat.Png)
        bmp.Dispose()
    End Sub

    Private Sub ShowGanttFromFile(filePath As String)
        If Not File.Exists(filePath) Then
            MessageBox.Show("Arquivo do Gantt não encontrado: " & filePath, "Gantt", MessageBoxButtons.OK, MessageBoxIcon.Warning)
            Exit Sub
        End If

        ' garante PictureBox dentro do Panel1
        Dim pic As PictureBox = EnsurePbInPanel()

        ' carregue sem travar o arquivo (usar FileStream + Image.FromStream)
        Dim img As Image
        Using fs As New FileStream(filePath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)
            img = Image.FromStream(fs)
        End Using

        Dim old As Image = pic.Image
        pic.Image = img
        pic.Size = img.Size

        ' ativa rolagem no Panel
        Panel1.AutoScroll = True
        Panel1.AutoScrollMinSize = img.Size

        If old IsNot Nothing AndAlso Not Object.ReferenceEquals(old, pic.Image) Then old.Dispose()
    End Sub


    ' Chame este método para diagnosticar a cadeia visual
    Private Sub DEBUG_GANTT_VIEW()
        ' 0) Confirma que foi chamado
        Debug.WriteLine("DEBUG_GANTT_VIEW: entrou")

        ' 1) Confirma Panel1
        If Panel1 Is Nothing Then
            MessageBox.Show("Panel1 = Nothing (o formulário não tem um Panel1).", "Diag", MessageBoxButtons.OK, MessageBoxIcon.Error)
            Exit Sub
        End If

        ' 2) Cria/garante o PictureBox
        If pbGantt Is Nothing Then
            pbGantt = New PictureBox() With {
            .Name = "pbGantt",
            .SizeMode = PictureBoxSizeMode.Normal,
            .BackColor = Color.AliceBlue,         ' cor chamativa p/ ver
            .BorderStyle = BorderStyle.Fixed3D,
            .Location = New Point(0, 0),
            .Visible = True
        }
        End If

        ' 3) Garante parent e visibilidade
        If pbGantt.Parent IsNot Panel1 Then
            ' NÃO limpa o Panel; só garante que pbGantt está dentro dele
            Try
                Panel1.Controls.Add(pbGantt)
            Catch
                ' Se já estiver em outro parent, remova de lá e adicione aqui
                If pbGantt.Parent IsNot Nothing Then pbGantt.Parent.Controls.Remove(pbGantt)
                Panel1.Controls.Add(pbGantt)
            End Try
        End If

        ' 4) Força layout do Panel e do PictureBox
        Panel1.AutoScroll = True
        Panel1.Dock = DockStyle.Fill       ' o panel ocupa a área central do form
        Panel1.BackColor = Color.WhiteSmoke

        pbGantt.Location = New Point(0, 0)
        pbGantt.Dock = DockStyle.None
        pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
        pbGantt.Visible = True
        pbGantt.BringToFront()

        ' 5) Faz um bitmap de teste e pinta um retângulo vermelho
        Dim W As Integer = Math.Max(Panel1.ClientSize.Width - 40, 400)
        Dim H As Integer = Math.Max(Panel1.ClientSize.Height - 40, 300)
        Dim bmp As New Bitmap(W, H)
        Using g As Graphics = Graphics.FromImage(bmp)
            g.Clear(Color.White)
            Using p As New Pen(Color.Blue, 2)
                g.DrawRectangle(p, 0, 0, W - 1, H - 1) ' borda azul do canvas
            End Using
            Using br As New SolidBrush(Color.Red)
                g.FillRectangle(br, 20, 20, 200, 120)   ' BLOCO VERMELHO
            End Using
            Using f As New Font("Segoe UI", 14, FontStyle.Bold)
                g.DrawString("TESTE", f, Brushes.DarkSlateGray, 30, 30)
            End Using
        End Using

        ' 6) Swap seguro + rolagem
        Dim oldImg As Image = pbGantt.Image
        pbGantt.Image = bmp
        pbGantt.Size = New Size(W, H)
        Panel1.AutoScrollMinSize = New Size(W, H)
        If oldImg IsNot Nothing Then oldImg.Dispose()

        ' 7) Log do estado atual (aparece na janela Output do VS)
        Debug.WriteLine($"Panel1: {Panel1.Name}  Size={Panel1.Size}  Client={Panel1.ClientSize}  Controls={Panel1.Controls.Count}")
        For Each c As Control In Panel1.Controls
            Debug.WriteLine($"  - Child: {c.Name}  Type={c.GetType().Name}  Bounds={c.Bounds}  Visible={c.Visible}")
        Next
        Debug.WriteLine($"pbGantt.Parent={If(pbGantt.Parent?.Name, "N/D")}  Bounds={pbGantt.Bounds}  Visible={pbGantt.Visible}")

        ' 8) Mensagem rápida se nada aparece
        If pbGantt.Parent IsNot Panel1 OrElse Not pbGantt.Visible Then
            MessageBox.Show("pbGantt não está visível dentro do Panel1.", "Diag", MessageBoxButtons.OK, MessageBoxIcon.Warning)
        End If
    End Sub


    ' === Gantt com linhas de altura fixa + scroll no Panel1 ===
    Private Sub LOAD_HORÁRIO_Q_SCROLL()
        ' ===== Parâmetros visuais =====
        Const ROW_H As Integer = 18
        Const COL_W As Integer = 10
        Const HORAS As Integer = 72
        Const LEFT_LABEL As Integer = 140
        Const TOP_M As Integer = 20
        Const RIGHT_M As Integer = 20
        Const BOTTOM_M As Integer = 30

        ' ===== Checagens básicas do layout =====
        If Panel1 Is Nothing Then
            MessageBox.Show("Panel1 não existe neste formulário.", "Gantt", MessageBoxButtons.OK, MessageBoxIcon.Error)
            Exit Sub
        End If
        If pbGantt Is Nothing Then
            MessageBox.Show("Coloque um PictureBox chamado 'pbGantt' DENTRO do Panel1.", "Gantt", MessageBoxButtons.OK, MessageBoxIcon.Error)
            Exit Sub
        End If
        If pbGantt.Parent IsNot Panel1 Then
            MessageBox.Show($"O 'pbGantt' não está dentro do Panel1 (parent atual: {If(pbGantt.Parent?.Name, "N/D")}).",
                        "Gantt", MessageBoxButtons.OK, MessageBoxIcon.Error)
            Exit Sub
        End If

        Panel1.AutoScroll = True
        pbGantt.Location = New Point(0, 0)
        pbGantt.Dock = DockStyle.None
        pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
        pbGantt.Visible = True
        pbGantt.BringToFront()

        Dim pmax As Integer = total_estações
        If pmax <= 0 Then pmax = 1

        ' ===== Dimensões do conteúdo =====
        Dim contentW As Integer = LEFT_LABEL + HORAS * COL_W + RIGHT_M
        Dim contentH As Integer = TOP_M + pmax * ROW_H + BOTTOM_M
        If contentW < 50 Then contentW = 50
        If contentH < 50 Then contentH = 50

        ' ===== Bitmap =====
        Dim bmp As New Bitmap(contentW, contentH)
        Dim desenhou As Boolean = False

        Using g As Graphics = Graphics.FromImage(bmp)
            g.Clear(Panel1.BackColor)
            g.TextRenderingHint = Drawing.Text.TextRenderingHint.ClearTypeGridFit

            ' Borda para diagnosticar
            Using penBorder As New Pen(Color.Silver, 1)
                g.DrawRectangle(penBorder, 0, 0, bmp.Width - 1, bmp.Height - 1)
            End Using

            Using penAxis As New Pen(Color.Black, 1),
              penGrid As New Pen(Color.Gainsboro, 1),
              fntLab As New Font("Segoe UI", 8, FontStyle.Regular),
              fntTick As New Font("Segoe UI", 7, FontStyle.Regular),
              brBlack As New SolidBrush(Color.Black),
              brGreen As New SolidBrush(Color.FromArgb(51, 204, 0)),
              brYellow As New SolidBrush(Color.FromArgb(255, 255, 0)),
              brOrange As New SolidBrush(Color.FromArgb(255, 165, 0)),
              brRed As New SolidBrush(Color.FromArgb(225, 0, 0)),
              brGrayText As New SolidBrush(Color.DimGray)

                Dim brMap As SolidBrush() = {brGreen, brYellow, brOrange, brRed}
                Dim baseY As Integer = TOP_M + pmax * ROW_H
                g.DrawLine(penAxis, LEFT_LABEL, baseY, contentW - RIGHT_M, baseY)

                ' Datas
                Dim fim As Date = If(dataF = Date.MinValue, Now, dataF)
                Dim dt0 As Date = fim.AddHours(-HORAS)

                ' Grid + rótulos
                For h As Integer = 1 To HORAS
                    Dim x As Integer = LEFT_LABEL + (h - 1) * COL_W
                    If h = 1 OrElse h Mod 6 = 0 Then
                        g.DrawLine(penGrid, x, TOP_M, x, baseY)
                        g.DrawString(h.ToString(), fntTick, brGrayText, x + 1, baseY + 4)
                    End If
                Next

                ' Linhas por estação
                For p As Integer = 1 To pmax
                    Dim y As Integer = TOP_M + (p - 1) * ROW_H
                    g.DrawLine(penGrid, LEFT_LABEL, y, contentW - RIGHT_M, y)

                    ' Nome
                    Dim nomeEst As String
                    Try
                        nomeEst = If(String.IsNullOrWhiteSpace(NOME(p)), $"Estação {p}", NOME(p))
                    Catch
                        nomeEst = $"Estação {p}"
                    End Try
                    Dim nameY As Integer = y + (ROW_H - fntLab.Height) \ 2
                    g.DrawString(nomeEst, fntLab, brGrayText, 4, nameY)

                    ' Blocos 72h
                    Dim dt As Date = dt0
                    For h As Integer = 1 To HORAS
                        dt = dt.AddHours(1)
                        Dim cls As Integer
                        Try
                            Dim qv As Double = Qdispo(p, dt.Hour, dt.Day, dt.Month, Ano(dt.Year))
                            If qv < 0 Then
                                cls = -1
                            Else
                                Dim m As Double = Media(p)
                                Dim d As Double = Desv_pad(p)
                                If qv < m + d Then cls = 0 _
                            Else If qv < m + 2 * d Then cls = 1 _
                            Else If qv < m + 3 * d Then cls = 2 _
                            Else If qv >= m + 4 * d Then cls = 3 _
                            Else cls = 2
                            End If
                        Catch
                            cls = -2 ' erro -> não pinta
                        End Try

                        Dim x As Integer = LEFT_LABEL + (h - 1) * COL_W
                        Dim rect As New Rectangle(x, y + 1, COL_W - 1, ROW_H - 2)

                        If cls = -1 Then
                            g.FillRectangle(brBlack, rect) : desenhou = True
                        ElseIf cls >= 0 Then
                            g.FillRectangle(brMap(Math.Min(cls, brMap.Length - 1)), rect) : desenhou = True
                        End If
                    Next
                Next
            End Using

            ' Fallback visual se nada foi pintado
            If Not desenhou Then
                Using f As New Font("Segoe UI", 12, FontStyle.Bold)
                    g.DrawString("TESTE: canvas OK, mas nenhum bloco foi pintado.", f, Brushes.Firebrick, 10, 10)
                End Using
                Using br As New SolidBrush(Color.LightGray)
                    g.FillRectangle(br, LEFT_LABEL + 10, TOP_M + 10, 200, 60)
                End Using
            End If
        End Using

        ' ===== Swap + scroll =====
        Dim oldImg As Image = pbGantt.Image
        pbGantt.Image = bmp
        pbGantt.Size = New Size(contentW, contentH)
        Panel1.AutoScrollMinSize = pbGantt.Size
        If oldImg IsNot Nothing AndAlso Not Object.ReferenceEquals(oldImg, pbGantt.Image) Then oldImg.Dispose()

        ' Mantém compat com código legado (se existir)
        bmparea2 = pbGantt.Image

        pbGantt.Invalidate()
    End Sub



    ' Pega nome da estação sem estourar
    Private Function TryGetStationName(p As Integer) As String
        Try
            Dim s = NOME(p)
            If String.IsNullOrWhiteSpace(s) Then Return $"Estação {p}"
            Return s
        Catch
            Return $"Estação {p}"
        End Try
    End Function

    ' Calcula a classe da vazão (retorna:
    '  -2 = erro/sem dado (não pinta),
    '  -1 = preto,
    '  0..3 = verde/amarelo/laranja/vermelho)
    'Private Function ClassificaQ(p As Integer, hh As Integer, dd As Integer, mm As Integer, aaaa As Integer) As Integer
    '    Try
    '        Dim qv As Double = Qdispo(p, hh, dd, mm, Ano(aaaa))
    '        If qv < 0 Then Return -1
    '        Dim m As Double = Media(p)
    '        Dim d As Double = Desv_pad(p)
    '        If qv < m + d Then Return 0
    '        If qv < m + 2 * d Then Return 1
    '        If qv < m + 3 * d Then Return 2
    '        If qv >= m + 4 * d Then Return 3
    '        Return 2
    '    Catch
    '        Return -2
    '    End Try
    'End Function





    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        ' escolhe a melhor fonte da imagem (pbGantt ou bmparea2)
        Dim src As Image = Nothing
        If pbGantt IsNot Nothing AndAlso pbGantt.Image IsNot Nothing Then
            src = pbGantt.Image
        ElseIf bmparea2 IsNot Nothing Then
            src = bmparea2
        End If

        If src Is Nothing Then
            MessageBox.Show("Não há imagem para salvar.", "Exportar", MessageBoxButtons.OK, MessageBoxIcon.Warning)
            Exit Sub
        End If

        Using sfd As New SaveFileDialog()
            sfd.Title = "Salvar Gantt"
            sfd.Filter = "Bitmap (*.bmp)|*.bmp"
            sfd.AddExtension = True
            sfd.DefaultExt = "bmp"
            sfd.FileName = $"gantt_q_{DateTime.Now:yyyyMMdd_HHmm}.bmp"

            If sfd.ShowDialog(Me) = DialogResult.OK Then
                ' faz uma cópia para não salvar o objeto “ao vivo” do PictureBox
                Using bmp As New Bitmap(src)
                    bmp.Save(sfd.FileName, ImageFormat.Bmp)
                End Using
                MessageBox.Show("Imagem salva com sucesso!", "Exportar", MessageBoxButtons.OK, MessageBoxIcon.Information)
            End If
        End Using
    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Me.Close()
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

    Sub LOAD_HORÁRIO()
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
            dif = 72 'DateDiff(DateInterval.Month, dataI, dataF)
            'desenha-se ano cheio (meramente informativo)
            Larg_Rec = ((xf - xi) / (dif + 1)) '((DateDiff(DateInterval.Year, dataI, dataF) + 1)))
            Alt_Rec = ((yi - yf) / total_estações)


            'Dim p_med(total_estações, 12, Now.Year) As Single 'quantidade de dias OK no ano e posto em questão

            Dim p_hor(total_estações, 72) As Single 'Dados dos últimos 3 dias

            'inicio do desenho: grafica eixo horizontal
            graficoArea2.DrawLine(New Pen(Color.Black, 2), xi, yi + 2, xf, yi + 2)

            Dim mesinicio As Integer
            mesinicio = 1
            Dim mesfim As Integer
            mesfim = 12
            Dim contames As Integer
            contames = 0
            Dim contaano As Integer
            Dim dd, hh, mm, aaaa As Integer

            'RGB - Red; Green; Blue / de 0 a 255
            For p = 1 To total_estações
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                'MsgBox(dataF.Year & " - " & dataI.Year)

                Dim datacomeço As Date
                datacomeço = dataF.AddHours(-72)

                For horaAnalisada As Integer = 1 To 72
                    datacomeço = datacomeço.AddHours(+1)
                    hh = datacomeço.Hour
                    dd = datacomeço.Day
                    mm = datacomeço.Month
                    aaaa = datacomeço.Year

                    If Qdispo(p, hh, dd, mm, Ano(aaaa)) < 0 Then
                        p_hor(p, horaAnalisada) = -1
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= 0 And Qdispo(p, hh, dd, mm, Ano(aaaa)) < 0.001 Then
                        p_hor(p, horaAnalisada) = 3
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= 0.001 And Qdispo(p, hh, dd, mm, Ano(aaaa)) < 50 Then
                        p_hor(p, horaAnalisada) = 0
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= 50 And Qdispo(p, hh, dd, mm, Ano(aaaa)) < 100 Then
                        p_hor(p, horaAnalisada) = 1
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= 100 Then
                        p_hor(p, horaAnalisada) = 2
                    End If 'soma quantidade de dias OK no ano e posto em questão

                    xloc = xi + ((horaAnalisada) * Larg_Rec)


                    If p_hor(p, horaAnalisada) = -1 Then 'preto
                        Dim Caneta = New Pen(Color.FromArgb(0, 0, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 0 Then 'verde
                        Dim Caneta = New Pen(Color.FromArgb(51, 204, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(51, 204, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 1 Then 'amarelo
                        Dim Caneta = New Pen(Color.FromArgb(255, 255, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(255, 255, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 2 Then 'vermelho
                        Dim Caneta = New Pen(Color.FromArgb(255, 0, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(255, 0, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 3 Then 'ciano
                        Dim Caneta = New Pen(Color.FromArgb(224, 255, 255))
                        Dim Pintar = New SolidBrush(Color.FromArgb(224, 255, 255))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    End If

                    If p = pmax Then  'somente depois do grafico pronto
                        graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xloc, yi + 15, xloc, yf)
                        graficoArea2.DrawString(CStr(horaAnalisada), New Font("Arial", 7), Brushes.Gray, xloc + 0.5, yi + 10)
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

    Sub CALCULAR_Estatísticas()
        'Variáveis de auxílio
        Dim dd, hh, mm, aaaa As Integer


        'Inicializa as variáveis
        For p = 1 To total_estações
            Media(p) = 0
            Desv_pad(p) = 0
            Max(p) = 0
            Min(p) = 0
            For anod As Integer = dataI.Year To dataF.Year
                MaxAnual(p, Ano(anod)) = 0
                MinAnual(p, Ano(anod)) = 999999
            Next
        Next


        'Percorre todos os postos
        For p = 1 To total_estações

            'Datainicial da análise
            Dim datacomeço As Date
            datacomeço = dataIp(p)

            'Calcula o tempo máximo da análise
            Dim tmaxx As Integer
            tmaxx = DateDiff(DateInterval.Hour, dataIp(p), dataFp(p))

            'Desconta as últimas 72h
            tmaxx = tmaxx - 72 'somente para a análise de TR via verificação

            'Desconta uma hora para começar a análise
            datacomeço = datacomeço.AddHours(-1)

            'Contador de intervalos com dado
            Dim cm As Integer
            cm = 0

            'Começa o loop de cálculo
            For t As Integer = 1 To tmaxx
                datacomeço = datacomeço.AddHours(1)
                hh = datacomeço.Hour
                dd = datacomeço.Day
                mm = datacomeço.Month
                aaaa = datacomeço.Year

                If Qdispo(p, hh, dd, mm, Ano(aaaa)) >= 0 Then
                    Media(p) += Qdispo(p, hh, dd, mm, Ano(aaaa))
                    cm += 1

                    If Qdispo(p, hh, dd, mm, Ano(aaaa)) > Max(p) Then
                        Max(p) = Qdispo(p, hh, dd, mm, Ano(aaaa))
                    End If
                    If Qdispo(p, hh, dd, mm, Ano(aaaa)) < Min(p) Then
                        Min(p) = Qdispo(p, hh, dd, mm, Ano(aaaa))
                    End If
                End If

            Next

            'Calcula a média
            If cm = 0 Then
                Media(p) = 0
            Else
                Media(p) = Media(p) / cm
            End If


            'Agora vai calcular o desvio padrão

            'Desconta uma hora para começar a análise
            datacomeço = datacomeço.AddHours(-1)

            'Contador de intervalos com dado
            cm = 0

            'Começa o loop de cálculo
            For t As Integer = 1 To tmaxx
                datacomeço = datacomeço.AddHours(1)
                hh = datacomeço.Hour
                dd = datacomeço.Day
                mm = datacomeço.Month
                aaaa = datacomeço.Year

                If Qdispo(p, hh, dd, mm, Ano(aaaa)) >= 0 And Media(p) > 0 Then
                    Desv_pad(p) += (Qdispo(p, hh, dd, mm, Ano(aaaa)) - Media(p)) ^ 2
                    cm += 1

                End If

            Next

            'Calcula o desvio
            If cm < 2 Then
                Desv_pad(p) = 0
            Else
                Desv_pad(p) = (Desv_pad(p) / (cm - 1)) ^ 0.5
            End If



        Next


    End Sub

    Sub LOAD_HORÁRIO_Q()
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
            dif = 72 'DateDiff(DateInterval.Month, dataI, dataF)
            'desenha-se ano cheio (meramente informativo)
            Larg_Rec = ((xf - xi) / (dif + 1)) '((DateDiff(DateInterval.Year, dataI, dataF) + 1)))
            Alt_Rec = ((yi - yf) / total_estações)


            'Dim p_med(total_estações, 12, Now.Year) As Single 'quantidade de dias OK no ano e posto em questão

            Dim p_hor(total_estações, 72) As Single 'Dados dos últimos 3 dias

            'inicio do desenho: grafica eixo horizontal
            graficoArea2.DrawLine(New Pen(Color.Black, 2), xi, yi + 2, xf, yi + 2)

            Dim mesinicio As Integer
            mesinicio = 1
            Dim mesfim As Integer
            mesfim = 12
            Dim contames As Integer
            contames = 0
            Dim contaano As Integer
            Dim dd, hh, mm, aaaa As Integer

            'RGB - Red; Green; Blue / de 0 a 255
            For p = 1 To total_estações
                yloc = (yi - Alt_Rec) - ((p - 1) * Alt_Rec)
                'MsgBox(dataF.Year & " - " & dataI.Year)

                Dim datacomeço As Date
                datacomeço = dataF.AddHours(-72)

                For horaAnalisada As Integer = 1 To 72
                    datacomeço = datacomeço.AddHours(+1)
                    hh = datacomeço.Hour
                    dd = datacomeço.Day
                    mm = datacomeço.Month
                    aaaa = datacomeço.Year

                    If Qdispo(p, hh, dd, mm, Ano(aaaa)) < 0 Then
                        p_hor(p, horaAnalisada) = -1
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= 0 And Qdispo(p, hh, dd, mm, Ano(aaaa)) < (Media(p) + Desv_pad(p)) Then
                        p_hor(p, horaAnalisada) = 0
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= (Media(p) + Desv_pad(p)) And Qdispo(p, hh, dd, mm, Ano(aaaa)) < (Media(p) + 2 * Desv_pad(p)) Then
                        p_hor(p, horaAnalisada) = 1
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= (Media(p) + 2 * Desv_pad(p)) And Qdispo(p, hh, dd, mm, Ano(aaaa)) < (Media(p) + 3 * Desv_pad(p)) Then
                        p_hor(p, horaAnalisada) = 2
                    ElseIf Qdispo(p, hh, dd, mm, Ano(aaaa)) >= (Media(p) + 4 * Desv_pad(p)) Then
                        p_hor(p, horaAnalisada) = 3
                    End If 'soma quantidade de dias OK no ano e posto em questão

                    xloc = xi + ((horaAnalisada) * Larg_Rec)


                    If p_hor(p, horaAnalisada) = -1 Then 'preto
                        Dim Caneta = New Pen(Color.FromArgb(0, 0, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(0, 0, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 0 Then 'verde
                        Dim Caneta = New Pen(Color.FromArgb(51, 204, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(51, 204, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 1 Then 'amarelo
                        Dim Caneta = New Pen(Color.FromArgb(255, 255, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(255, 255, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 2 Then 'laranja
                        Dim Caneta = New Pen(Color.FromArgb(255, 165, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(255, 165, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    ElseIf p_hor(p, horaAnalisada) = 3 Then 'vermelho
                        Dim Caneta = New Pen(Color.FromArgb(225, 0, 0))
                        Dim Pintar = New SolidBrush(Color.FromArgb(225, 0, 0))
                        graficoArea2.DrawRectangle(Caneta, xloc, yloc, Larg_Rec, Alt_Rec)
                        graficoArea2.FillRectangle(Pintar, xloc, yloc, Larg_Rec, Alt_Rec)
                    End If

                    If p = pmax Then  'somente depois do grafico pronto
                        graficoArea2.DrawLine(New Pen(Color.Gray, 0.01), xloc, yi + 15, xloc, yf)
                        graficoArea2.DrawString(CStr(horaAnalisada), New Font("Arial", 7), Brushes.Gray, xloc + 0.5, yi + 10)
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



    ' Garante Panel1/PictureBox prontos e dentro da hierarquia correta
    Private Sub PreparePanelAndPictureBox()
        Me.AutoScroll = False
        Panel1.AutoScroll = True
        Panel1.BackColor = Color.White
        Panel1.Dock = DockStyle.Fill

        If pbGantt Is Nothing Then
            pbGantt = New PictureBox() With {
            .SizeMode = PictureBoxSizeMode.Normal,
            .Location = New Point(0, 0),
            .BackColor = Color.White
        }
        End If

        If pbGantt.Parent IsNot Panel1 Then
            ' CUIDADO: isso remove todos os filhos do Panel1.
            ' Use somente se seus botões/legendas NÃO estiverem dentro do Panel1.
            Panel1.Controls.Clear()
            Panel1.Controls.Add(pbGantt)
        End If

        pbGantt.Location = New Point(0, 0)
        pbGantt.Dock = DockStyle.None
        pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
        pbGantt.BringToFront()
    End Sub

    ' Faz o swap da imagem de forma segura e sincroniza bmparea2 (legado)
    Private Sub SafeSwapImage(newBmp As Bitmap)
        Dim contentW As Integer = newBmp.Width
        Dim contentH As Integer = newBmp.Height

        Dim oldImg As Image = pbGantt.Image
        pbGantt.Image = newBmp
        pbGantt.Size = New Size(contentW, contentH)
        Panel1.AutoScrollMinSize = New Size(contentW, contentH)

        ' Mantém variável legado sincronizada (para qualquer Paint antigo que ainda use)
        bmparea2 = pbGantt.Image

        If oldImg IsNot Nothing AndAlso Not Object.ReferenceEquals(oldImg, pbGantt.Image) Then
            oldImg.Dispose()
        End If
    End Sub

    ' Garante que apenas o Panel rola e o pbGantt fica contido
    Private Sub EnsureGanttContained()
        Me.AutoScroll = False
        Panel1.AutoScroll = True
        Panel1.Dock = DockStyle.Fill
        If pbGantt IsNot Nothing Then
            pbGantt.Location = New Point(0, 0)
            pbGantt.Dock = DockStyle.None
            pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
            pbGantt.BringToFront()
        End If
    End Sub

    ' Procura um painel para hospedar o Gantt:
    '  1) tenta achar um Panel chamado "Panel1" (em toda a hierarquia)
    '  2) senão, pega o MAIOR Panel visível do formulário
    '  3) senão, cria um "pnlGanttAuto" dockado em Fill
    Private Function ResolveHostPanel() As Panel
        ' 1) procura por nome
        Dim found() As Control = Me.Controls.Find("Panel1", searchAllChildren:=True)
        If found IsNot Nothing AndAlso found.Length > 0 AndAlso TypeOf found(0) Is Panel Then
            Return DirectCast(found(0), Panel)
        End If

        ' 2) maior Panel visível
        Dim best As Panel = Nothing
        Dim bestArea As Integer = -1
        Dim stack As New Stack(Of Control)()
        stack.Push(Me)
        While stack.Count > 0
            Dim c = stack.Pop()
            For Each child As Control In c.Controls
                stack.Push(child)
                If TypeOf child Is Panel AndAlso child.Visible Then
                    Dim p = DirectCast(child, Panel)
                    Dim area = p.ClientSize.Width * p.ClientSize.Height
                    If area > bestArea Then
                        bestArea = area
                        best = p
                    End If
                End If
            Next
        End While
        If best IsNot Nothing Then Return best

        ' 3) cria um novo painel só pro Gantt
        Dim autoP As New Panel With {
        .Name = "pnlGanttAuto",
        .AutoScroll = True,
        .Dock = DockStyle.Fill,
        .BackColor = Color.White
    }
        Me.Controls.Add(autoP)
        autoP.BringToFront()
        Return autoP
    End Function

    ' Garante um PictureBox hosteado no painel
    Private Function EnsurePictureBox(host As Panel) As PictureBox
        If pbGantt Is Nothing Then
            pbGantt = New PictureBox With {
            .Name = "pbGantt",
            .SizeMode = PictureBoxSizeMode.Normal,
            .BackColor = Color.White,
            .BorderStyle = BorderStyle.FixedSingle
        }
        End If
        If pbGantt.Parent IsNot host Then
            If pbGantt.Parent IsNot Nothing Then pbGantt.Parent.Controls.Remove(pbGantt)
            host.Controls.Add(pbGantt)
        End If
        pbGantt.Location = New Point(0, 0)
        pbGantt.Dock = DockStyle.None
        pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
        pbGantt.Visible = True
        pbGantt.BringToFront()
        host.AutoScroll = True
        Return pbGantt
    End Function

    ' Swap seguro de imagem + ajuste de scroll
    Private Sub SwapImage(host As Panel, pic As PictureBox, bmp As Bitmap)
        Dim oldImg As Image = pic.Image
        pic.Image = bmp
        pic.Size = New Size(bmp.Width, bmp.Height)
        host.AutoScrollMinSize = pic.Size
        bmparea2 = pic.Image ' compat. com código legado, se existir
        If oldImg IsNot Nothing AndAlso Not Object.ReferenceEquals(oldImg, pic.Image) Then
            oldImg.Dispose()
        End If
    End Sub

    ' Classificação segura (igual à anterior)
    Private Function ClassificaQ(p As Integer, hh As Integer, dd As Integer, mm As Integer, aaaa As Integer) As Integer
        Try
            Dim qv As Double = Qdispo(p, hh, dd, mm, Ano(aaaa))
            If qv < 0 Then Return -1
            Dim m As Double = Media(p)
            Dim d As Double = Desv_pad(p)
            If qv < m + d Then Return 0
            If qv < m + 2 * d Then Return 1
            If qv < m + 3 * d Then Return 2
            If qv >= m + 4 * d Then Return 3
            Return 2
        Catch
            Return -2
        End Try
    End Function

    Private Function NomeEstacao(p As Integer) As String
        Try
            Dim s = NOME(p)
            If String.IsNullOrWhiteSpace(s) Then Return $"Estação {p}"
            Return s
        Catch
            Return $"Estação {p}"
        End Try
    End Function

    Private Function EnsurePbInPanel() As PictureBox
        ' se já existir, retorna
        If pbGantt IsNot Nothing AndAlso pbGantt.Parent Is Panel1 Then
            pbGantt.Visible = True
            pbGantt.Location = New Point(0, 0)
            pbGantt.Dock = DockStyle.None
            pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
            pbGantt.BringToFront()
            Return pbGantt
        End If

        ' senão, cria/realoca
        If pbGantt Is Nothing Then
            pbGantt = New PictureBox With {
            .Name = "pbGantt",
            .BackColor = Color.White,
            .BorderStyle = BorderStyle.FixedSingle,
            .SizeMode = PictureBoxSizeMode.Normal,
            .Location = New Point(0, 0)
        }
        Else
            If pbGantt.Parent IsNot Nothing Then pbGantt.Parent.Controls.Remove(pbGantt)
        End If

        Panel1.Controls.Add(pbGantt)
        pbGantt.Dock = DockStyle.None
        pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
        pbGantt.Visible = True
        pbGantt.BringToFront()

        Return pbGantt
    End Function


    ' === Gantt por faixas fixas (sem médias/desvios) ===
    Private Sub LOAD_HORÁRIO_Scroll()
        ' ---- layout ----
        Const ROW_H As Integer = 18
        Const COL_W As Integer = 10
        Const HORAS As Integer = 72
        Const LEFT_LABEL As Integer = 140
        Const TOP_M As Integer = 20
        Const RIGHT_M As Integer = 20
        Const BOTTOM_M As Integer = 30

        ' ---- garante Panel1/pbGantt prontos ----
        If Panel1 Is Nothing OrElse Not Panel1.Visible Then Panel1.Visible = True
        Panel1.AutoScroll = True

        If pbGantt Is Nothing Then
            pbGantt = New PictureBox() With {
                .Name = "pbGantt",
                .SizeMode = PictureBoxSizeMode.Normal,
                .BackColor = Color.White,
                .BorderStyle = BorderStyle.FixedSingle
            }
            Panel1.Controls.Add(pbGantt)
        ElseIf pbGantt.Parent IsNot Panel1 Then
            If pbGantt.Parent IsNot Nothing Then pbGantt.Parent.Controls.Remove(pbGantt)
            Panel1.Controls.Add(pbGantt)
        End If

        pbGantt.Location = New Point(0, 0)
        pbGantt.Dock = DockStyle.None
        pbGantt.Anchor = AnchorStyles.Top Or AnchorStyles.Left
        pbGantt.Visible = True
        pbGantt.BringToFront()

        ' ---- dados básicos ----
        Dim pmax As Integer = Math.Max(1, total_estações)
        Dim fim As Date = If(dataF = Date.MinValue, Now, dataF)
        Dim dt0 As Date = fim.AddHours(-HORAS)

        ' ---- dimensões do canvas ----
        Dim contentW As Integer = LEFT_LABEL + HORAS * COL_W + RIGHT_M
        Dim contentH As Integer = TOP_M + pmax * ROW_H + BOTTOM_M
        If contentW < 50 Then contentW = 50
        If contentH < 50 Then contentH = 50

        Dim bmp As New Bitmap(contentW, contentH)

        Using g As Graphics = Graphics.FromImage(bmp)
            g.Clear(Panel1.BackColor)
            g.TextRenderingHint = Drawing.Text.TextRenderingHint.ClearTypeGridFit

            Using penAxis As New Pen(Color.Black, 1)
                Using penGrid As New Pen(Color.Gainsboro, 1)
                    Using fntLab As New Font("Segoe UI", 8, FontStyle.Regular)
                        Using fntTick As New Font("Segoe UI", 7, FontStyle.Regular)

                            ' --- novas cores ---
                            Using brFalha As New SolidBrush(Color.Black)                         ' v < 0  → preto
                                Using brZero As New SolidBrush(Color.White)                          ' v = 0  → branco
                                    Using brAzulClaro As New SolidBrush(Color.FromArgb(198, 224, 255))   ' 0 ≤ v < 10 → azul claro
                                        Using brVerde As New SolidBrush(Color.FromArgb(0, 176, 80))          ' 10 ≤ v < 30 → verde
                                            Using brAmarelo As New SolidBrush(Color.Yellow)                      ' 30 ≤ v < 50 → amarelo
                                                Using brVermelho As New SolidBrush(Color.FromArgb(225, 0, 0))        ' v ≥ 50 → vermelho
                                                    Using brGrayText As New SolidBrush(Color.DimGray)

                                                        Dim baseY As Integer = TOP_M + pmax * ROW_H

                                                        ' eixo X
                                                        g.DrawLine(penAxis, LEFT_LABEL, baseY, contentW - RIGHT_M, baseY)

                                                        ' grade vertical + rótulos (a cada 6h)
                                                        For h As Integer = 1 To HORAS
                                                            Dim x As Integer = LEFT_LABEL + (h - 1) * COL_W
                                                            If h = 1 OrElse h Mod 6 = 0 Then
                                                                g.DrawLine(penGrid, x, TOP_M, x, baseY)
                                                                g.DrawString(h.ToString(), fntTick, brGrayText, x + 1, baseY + 4)
                                                            End If
                                                        Next

                                                        ' linhas/nomes por estação
                                                        For p As Integer = 1 To pmax
                                                            Dim y As Integer = TOP_M + (p - 1) * ROW_H
                                                            g.DrawLine(penGrid, LEFT_LABEL, y, contentW - RIGHT_M, y)

                                                            Dim nomeEst As String
                                                            Try
                                                                nomeEst = If(String.IsNullOrWhiteSpace(NOME(p)), $"Estação {p}", NOME(p))
                                                            Catch
                                                                nomeEst = $"Estação {p}"
                                                            End Try
                                                            Dim nameY As Integer = y + (ROW_H - fntLab.Height) \ 2
                                                            g.DrawString(nomeEst, fntLab, brGrayText, 4, nameY)

                                                            ' blocos por hora
                                                            Dim dt As Date = dt0
                                                            For h As Integer = 1 To HORAS
                                                                dt = dt.AddHours(1)
                                                                Dim x As Integer = LEFT_LABEL + (h - 1) * COL_W
                                                                Dim rect As New Rectangle(x, y + 1, COL_W - 1, ROW_H - 2)

                                                                Dim v As Double
                                                                Dim ok As Boolean = True
                                                                Try
                                                                    v = Qdispo(p, dt.Hour, dt.Day, dt.Month, Ano(dt.Year))
                                                                Catch
                                                                    ok = False
                                                                End Try

                                                                If Not ok OrElse v < 0 Then
                                                                    g.FillRectangle(brFalha, rect)                ' preto
                                                                ElseIf v = 0 Then
                                                                    g.FillRectangle(brZero, rect)                 ' branco
                                                                ElseIf v < 10 Then
                                                                    g.FillRectangle(brAzulClaro, rect)            ' azul claro
                                                                ElseIf v < 30 Then
                                                                    g.FillRectangle(brVerde, rect)                ' verde
                                                                ElseIf v < 50 Then
                                                                    g.FillRectangle(brAmarelo, rect)              ' amarelo
                                                                Else
                                                                    g.FillRectangle(brVermelho, rect)             ' vermelho
                                                                End If
                                                            Next
                                                        Next

                                                    End Using : End Using : End Using : End Using : End Using : End Using : End Using : End Using
                    End Using : End Using : End Using : End Using


        ' ---- swap + scroll + export compat ----
        Dim oldImg As Image = pbGantt.Image
        pbGantt.Image = bmp
        pbGantt.Size = New Size(contentW, contentH)
        Panel1.AutoScrollMinSize = pbGantt.Size
        If oldImg IsNot Nothing AndAlso Not Object.ReferenceEquals(oldImg, pbGantt.Image) Then oldImg.Dispose()

        ' manter compatibilidade com seu botão Exportar BMP
        bmparea2 = pbGantt.Image
        pbGantt.Invalidate()
    End Sub

    Private Sub Label3_Click(sender As Object, e As EventArgs) Handles Label3.Click

    End Sub
End Class