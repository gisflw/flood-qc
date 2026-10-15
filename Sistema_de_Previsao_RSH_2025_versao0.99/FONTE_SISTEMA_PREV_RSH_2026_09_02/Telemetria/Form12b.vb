Imports ZedGraph
Imports System.Drawing

Public Class Form12b

    Private Sub Form12b_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        CreateGraph4(zg1)
        CreateGraph3(zg2)

        If TotalCommander = True Then
            Me.SAVESCREENSHOT_AUTO()
            Me.Close()
        End If

        'SetSize()
    End Sub

    Private Sub CreateGraph4(ByVal zgc As ZedGraphControl)
        Dim p As Integer

        Dim myPane As GraphPane = zgc.GraphPane

        ' Set the titles and axis labels
        myPane.Title.Text = "Hidrograma"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Vazão (m³/s)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max =
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        '        zedGraphControl1.IsEnableVZoom = false;
        'zedGraphControl1.IsEnableVPan = false;
     
        myPane.XAxis.MajorGrid.IsVisible = True
        myPane.YAxis.MajorGrid.IsVisible = True

        ' Make up some data points from the Sine function
        Dim list = New PointPairList()
        Dim list2 = New PointPairList()
        Dim list3 = New PointPairList()
        Dim list4 = New PointPairList
        Dim list5 = New PointPairList
        Dim list6 = New PointPairList

        Dim x As XDate
        Dim y As Double
        Dim x2 As XDate
        Dim y2 As Double
        Dim x3 As XDate
        Dim y3 As Double
        Dim x4 As XDate
        Dim y4 As Double
        Dim x5 As XDate
        Dim y5 As Double
        Dim x6 As XDate
        Dim y6 As Double

        ' For x = 0 To 36
        'y = Math.Sin(x * Math.PI / 15.0)
        'list.Add(x, y)
        'Next x
        'coisa do fernando
        p = 1
        x = 0



        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado

        'Trava a Janela apenas com a previsão mais recente
        'datacomeço = datacomeço.AddDays(tamdoresultado - 20)
        'numerodedias = 20
        'zgc.IsEnableVZoom = False
        'zgc.IsEnableVPan = False

        Dim maiorY As Double
        maiorY = 0
        Dim dataprevY As Date
        dataprevY = Começoresultado.AddHours(tamdoresultado - 21 * 24 - 12) '- 12 * 24)

        datacomeço = datacomeço.AddHours(-1)
        For auxilio As Integer = 1 To numerodedias
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1
            'PrintLine(2, datacomeço.Date & " " & vazaoun(contadordiamenor) & " " & bifinal(contadordiamenor))
            x = datacomeço
            x2 = datacomeço
            x3 = datacomeço
            x4 = datacomeço
            x5 = datacomeço
            'x6 = datacomeço



            y = VazaoHIG(754, contadordiamenor)
            y2 = QPROPHIG(754, contadordiamenor)
            y3 = VazaoHIG(750, contadordiamenor)
            y4 = QPROPHIG(750, contadordiamenor)
            y5 = QOBS(754, contadordiamenor)
            'y5 = VazaoHIG(331, contadordiamenor)
            'y6 = QPROPHIG(331, contadordiamenor)


            If y >= 0 Then
                list.Add(x, y)
            End If
            If y2 >= 0 Then
                list2.Add(x2, y2)
            End If
            If y3 >= 0 Then
                list3.Add(x3, y3)
            End If
            If y4 >= 0 Then
                list4.Add(x4, y4)
            End If
            If y5 >= 0 Then
                list5.Add(x5, y5)
            End If
            'If y6 >= 0 Then
            '    list6.Add(x6, y6)
            'End If

            'Verifica o maior Y dentro de todos os valores, mas só para as datas mais atuais
            If datacomeço >= dataprevY Then
                If y > maiorY Then
                    maiorY = y
                End If
                If y2 > maiorY Then
                    maiorY = y2
                End If
                If y3 > maiorY Then
                    maiorY = y3
                End If
                If y4 > maiorY Then
                    maiorY = y4
                End If
            End If

        Next

        myPane.XAxis.Scale.Min = datacomeço.AddDays(-21).ToOADate() '1.1 'myPane.XAxis.Scale.Max.ToString * 0.9
        myPane.XAxis.Scale.Max = datacomeço.AddDays(1).ToOADate() '1.1 'myPane.XAxis.Scale.Max.ToString * 0.9

        myPane.YAxis.Scale.Max = maiorY * 1.2
        myPane.YAxis.Scale.Min = 0

        'MsgBox(myPane.XAxis.Scale.Min.ToString)
        'MsgBox(myPane.XAxis.Scale.Max.ToString)


        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        'Dim myCurve2 As LineItem = myPane.AddCurve("UHE Estreito Aflunte", list2, Color.Blue, SymbolType.None)
        'Dim myCurve As LineItem = myPane.AddCurve("Previsão UHE Estreito Afluente", list, Color.SkyBlue, SymbolType.Circle)
        'Dim myCurve4 As LineItem = myPane.AddCurve("Tupiratins", list4, Color.DarkOrange, SymbolType.None)
        'Dim myCurve3 As LineItem = myPane.AddCurve("Previsão Tupiratins", list3, Color.Salmon, SymbolType.Circle)
        'Dim myCurve6 As LineItem = myPane.AddCurve("UHE Lajeado", list6, Color.Green, SymbolType.None)
        'Dim myCurve5 As LineItem = myPane.AddCurve("Operação Estimada UHE Lajeado", list5, Color.LightGreen, SymbolType.Circle)

        'myCurve.Line.Width = 1.5
        'myCurve2.Line.Width = 1.5
        'myCurve3.Line.Width = 1.5
        'myCurve4.Line.Width = 1.5
        'myCurve5.Line.Width = 1.5
        'myCurve6.Line.Width = 1.5

        'myCurve.Symbol.Size = 0.8
        'myCurve.Line.Width = 0.2
        '' myCurve.Line.IsVisible = False
        'myCurve.Symbol.Fill = New Fill(Color.LightSkyBlue)
        ''myCurve.Line.DashOn = 1
        ''myCurve.Line.Style = Drawing2D.DashStyle.DashDot

        'myCurve3.Symbol.Size = 0.8
        'myCurve3.Line.Width = 0.2
        ''myCurve3.Line.IsVisible = False
        'myCurve3.Symbol.Fill = New Fill(Color.LightSalmon)
        ''myCurve3.Line.DashOn = 3
        ''myCurve3.Line.Style = Drawing2D.DashStyle.DashDot

        'myCurve5.Symbol.Size = 0.8
        'myCurve5.Line.Width = 0.2
        ''myCurve3.Line.IsVisible = False
        'myCurve5.Symbol.Fill = New Fill(Color.LightGreen)
        '' myCurve5.Line.DashOn = 5
        ''myCurve5.Line.Style = Drawing2D.DashStyle.DashDot

        '' Fill the area under the curve with a white-red gradient at 45 degrees
        '' myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
        ''myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

        '' Make the symbols opaque by filling them with white
        ''myCurve.Symbol.Fill = New Fill(Color.White)

        '' Fill the axis background with a color gradient
        'myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

        '' Fill the pane background with a color gradient
        'myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)



        Dim myCurve2 As LineItem = myPane.AddCurve("UHE Campos Novos Aflunte", list2, Color.MediumBlue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Previsão UHE Campos Novos Afluente", list, Color.CornflowerBlue, SymbolType.None)
        Dim myCurve4 As LineItem = myPane.AddCurve("UHE Garibaldi Afluente", list4, Color.DarkOrange, SymbolType.None)
        Dim myCurve3 As LineItem = myPane.AddCurve("Previsão UHE Garibaldi Afluente", list3, Color.Salmon, SymbolType.None)
        'Dim myCurve6 As LineItem = myPane.AddCurve("UHE Lajeado", list6, Color.DarkGreen, SymbolType.None)
        Dim myCurve5 As LineItem = myPane.AddCurve("UHE Campos Novos Observada Média Móvel", list5, Color.LimeGreen, SymbolType.None)

        myCurve.Line.Width = 1.5
        myCurve2.Line.Width = 1.5
        myCurve3.Line.Width = 1.5
        myCurve4.Line.Width = 1.5
        'myCurve5.Line.Width = 1.5
        'myCurve6.Line.Width = 1.5

        'myCurve.Symbol.Size = 0.8
        myCurve.Line.Width = 1.5
        myCurve.Line.DashOn = True
        myCurve.Line.Style = Drawing2D.DashStyle.Dash
        myCurve.Line.IsAntiAlias = True
        myCurve.Line.StepType = StepType.ForwardSegment
        myCurve.Line.IsSmooth = True
        myCurve.Line.SmoothTension = 0.05

        ' myCurve.Line.IsVisible = False
        'myCurve.Symbol.Fill = New Fill(Color.Blue)
        'myCurve.Line.DashOn = 1
        'myCurve.Line.Style = Drawing2D.DashStyle.DashDot

        ' myCurve3.Symbol.Size = 0.8
        myCurve3.Line.Width = 1.5
        myCurve3.Line.DashOn = True
        myCurve3.Line.Style = Drawing2D.DashStyle.Dash
        myCurve3.Line.IsAntiAlias = True
        myCurve3.Line.StepType = StepType.ForwardSegment
        myCurve3.Line.IsSmooth = True
        myCurve3.Line.SmoothTension = 0.05

        'myCurve3.Line.IsVisible = False
        'myCurve3.Symbol.Fill = New Fill(Color.LightSalmon)
        'myCurve3.Line.DashOn = 3
        'myCurve3.Line.Style = Drawing2D.DashStyle.DashDot

        'myCurve5.Symbol.Size = 0.8
        'myCurve5.Line.Width = 1.5
        'myCurve5.Line.DashOn = True
        'myCurve5.Line.Style = Drawing2D.DashStyle.Dash
        'myCurve5.Line.IsAntiAlias = True
        'myCurve5.Line.StepType = StepType.ForwardSegment
        'myCurve5.Line.IsSmooth = True
        'myCurve5.Line.SmoothTension = 0.05
        'myCurve3.Line.IsVisible = False
        'myCurve5.Symbol.Fill = New Fill(Color.LightGreen)
        ' myCurve5.Line.DashOn = 5
        'myCurve5.Line.Style = Drawing2D.DashStyle.DashDot

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()

    End Sub

    Private Sub CreateGraph3(ByVal zgc As ZedGraphControl)
        Dim p As Integer


        Dim myPane As GraphPane = zgc.GraphPane

        ' Set the titles and axis labels
        myPane.Title.Text = "Pluviograma Incremental UHE Campos Novos - UHE Garibaldi"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Chuva (mm)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        myPane.XAxis.MajorGrid.IsVisible = True
        myPane.YAxis.MajorGrid.IsVisible = True

        ' Make up some data points from the Sine function
        Dim list = New PointPairList
        Dim list2 = New PointPairList
        'Dim list3 = New PointPairList()


        Dim x As XDate
        Dim y As Double
        Dim x2 As XDate
        Dim y2 As Double
        'Dim x3 As XDate
        'Dim y3 As Double


        ' For x = 0 To 36
        'y = Math.Sin(x * Math.PI / 15.0)
        'list.Add(x, y)
        'Next x
        'coisa do fernando
        p = 1
        x = 0



        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado

        datacomeço = datacomeço.AddHours(-1)
        For auxilio As Integer = 1 To numerodedias
            datacomeço = datacomeço.AddHours(1)
            contadordiamenor = contadordiamenor + 1
            'PrintLine(2, datacomeço.Date & " " & vazaoun(contadordiamenor) & " " & bifinal(contadordiamenor))
            x = datacomeço
            x2 = datacomeço
            '  x3 = datacomeço

            If contadordiamenor <= tchuvabin1 Then
                y = chuvasub1(18, contadordiamenor)
                y2 = 0
            ElseIf contadordiamenor > tchuvabin1 Then
                ' Dim diff As Integer
                ' diff = contadordiamenor - tchuvabin1
                y2 = chuvasub2(18, contadordiamenor)
                y = 0
            End If

            'y3 = QOBS(postoselecionado, contadordiamenor)

            list.Add(x, y)
            list2.Add(x2, y2)
            'list3.Add(x3, y3)

        Next

        myPane.XAxis.Scale.Min = datacomeço.AddDays(-21).ToOADate() '1.1 'myPane.XAxis.Scale.Max.ToString * 0.9
        myPane.XAxis.Scale.Max = datacomeço.AddDays(1).ToOADate() '1.1 'myPane.XAxis.Scale.Max.ToString * 0.9

        myPane.YAxis.Scale.Min = 0

        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        'Dim myCurve3 As LineItem = myPane.AddCurve("Vazão Observada", list3, Color.Green, SymbolType.None)
        'Dim myCurve2 As LineItem = myPane.AddCurve("Chuva Prevista", list2, Color.Red, SymbolType.None)
        'Dim myCurve As LineItem = myPane.AddCurve("Chuva Observada", list, Color.Blue, SymbolType.None)

        '' Dim myCurve2 As BarItem = myPane.AddBar("Chuva Prevista", list2, Color.Red)
        ''Dim myCurve As BarItem = myPane.AddBar("Chuva Observada", list, Color.Blue)


        '' Fill the area under the curve with a white-red gradient at 45 degrees
        'myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
        'myCurve2.Line.Fill = New Fill(Color.White, Color.Red, 45.0F)

        '' Make the symbols opaque by filling them with white
        'myCurve.Symbol.Fill = New Fill(Color.White)

        '' Fill the axis background with a color gradient
        'myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

        '' Fill the pane background with a color gradient
        'myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

        Dim myCurve2 As LineItem = myPane.AddCurve("Chuva Prevista", list2, Color.Red, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Chuva Observada", list, Color.Blue, SymbolType.None)

        myCurve.Line.Width = 1.5
        myCurve2.Line.Width = 1.5

        ' Fill the area under the curve with a white-red gradient at 45 degrees
        myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
        myCurve2.Line.Fill = New Fill(Color.White, Color.Red, 45.0F)

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()

    End Sub

    Sub SAVESCREENSHOT()
        'Dim bmp As Bitmap = New Bitmap(Me.Width, Me.Height)

        'Me.DrawToBitmap(bmp, New Rectangle(0, 0, Me.Width, Me.Height))

        'bmp.Save("C:\Users\Walter_MudClima_INPE\Desktop\teste.bmp")


        Dim bmp As New Bitmap(Panel3.Width, Panel3.Height)
        Panel3.DrawToBitmap(bmp, Panel3.ClientRectangle)

        'Abre o diálogo para selecionar o local onde vai salvar o novo shape
        sfd.Filter = "Imagem BMP (.bmp)|*.bmp"
        If sfd.ShowDialog() = Windows.Forms.DialogResult.OK Then

            bmp.Save(sfd.FileName)
        End If


    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        Me.SAVESCREENSHOT()
    End Sub

    Private Sub Button2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button2.Click
        Me.Close()
    End Sub


    Sub SAVESCREENSHOT_AUTO()
        'Dim bmp As Bitmap = New Bitmap(Me.Width, Me.Height)

        'Me.DrawToBitmap(bmp, New Rectangle(0, 0, Me.Width, Me.Height))

        'bmp.Save("C:\Users\Walter_MudClima_INPE\Desktop\teste.bmp")


        Dim bmp As New Bitmap(Panel3.Width, Panel3.Height)
        Panel3.DrawToBitmap(bmp, Panel3.ClientRectangle)

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
        arquivo = diretóriodoprojeto & "\PrevisãoCNO_" & Now.Year & mes & dia & hora & ".bmp"

        bmp.Save(arquivo)

        ''Abre o diálogo para selecionar o local onde vai salvar o novo shape
        'sfd.Filter = "Imagem BMP (.bmp)|*.bmp"
        'If sfd.ShowDialog() = Windows.Forms.DialogResult.OK Then

        '    bmp.Save(sfd.FileName)
        'End If


    End Sub

End Class