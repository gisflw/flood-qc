Imports ZedGraph
Imports System.Drawing

Public Class Form4

    Public espessura_linha As Single
    Private Sub Form4_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load

        espessura_linha = 2

        If Resultado = True Then
            CreateGraph2(zg1)
            SetSize()

        ElseIf ResultadoNível = True Then
            CreateGraph2(zg1)
            SetSize()

        ElseIf ResultadoVazaopp = True Then
            CreateGraph_HidrogramaPosProcessado(zg1)
            SetSize()
        ElseIf ResultadoNívelpp = True Then
            CreateGraph_CotagramaPosProcessado(zg1)
            SetSize()

        ElseIf ResultadoLimiares = True Then
            CreateGraph2(zg1)
            SetSize()

        ElseIf VisuResultado = True Then
            CreateGraph2(zg1)
            SetSize()
        ElseIf ChuvaSUBS = True Then
            CreateGraph3(zg1)
            SetSize()
        ElseIf Padrão = True Then
            CreateGraph4(zg1)
            SetSize()
        Else
            CreateGraph(zg1)
            SetSize()
        End If

        
    End Sub

    Private Sub SetSize()
        zg1.Location = New Point(10, 10)
        ' Leave a small margin around the outside of the control
        zg1.Size = New Size(ClientRectangle.Width - 20, ClientRectangle.Height - 20)
    End Sub

    Private Sub CreateGraph(ByVal zgc As ZedGraphControl)
       

        Dim myPane As GraphPane = zgc.GraphPane

        ' Set the titles and axis labels
        If Hidrograma = True Then
            myPane.Title.Text = "Hidrograma"
            myPane.XAxis.Title.Text = "Data"
            myPane.YAxis.Title.Text = "Vazão (m³/s)"

            'myPane.AddY2Axis("Teste")
            'myPane.Y2Axis.Title.Text = "Vazão (m³/s)"
            'myPane.Y2Axis.IsVisible = True

        ElseIf Cotagrama = True Then
            myPane.Title.Text = "Cotagrama"
            myPane.XAxis.Title.Text = "Data"
            myPane.YAxis.Title.Text = "Nível (m)"


        ElseIf Pluviograma = True Then
            myPane.Title.Text = "Pluviograma"
            myPane.XAxis.Title.Text = "Data"
            myPane.YAxis.Title.Text = "Precipitação (mm)"
        End If
        
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        ' Make up some data points from the Sine function
        Dim list = New PointPairList()
        Dim list2 = New PointPairList()
        Dim list3 = New PointPairList()
        Dim list4 = New PointPairList()
        Dim list5 = New PointPairList()


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

        x = 0
        x2 = 0
        x3 = 0
        x4 = 0
        x5 = 0


        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = dataIp(Estação_GrafQ)
        numerodedias = DateDiff(DateInterval.Hour, dataIp(Estação_GrafQ), dataFp(Estação_GrafQ)) + 1

        ' MsgBox(datacomeço.Date

        datacomeço = datacomeço.AddHours(-1)
        For auxilio As Integer = 1 To numerodedias
            datacomeço = datacomeço.AddHours(1)


            x = datacomeço

            If Hidrograma = True Then
                y = Q(Estação_GrafQ, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
            ElseIf Cotagrama = True Then
                y = Niv(Estação_GrafQ, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
            ElseIf Pluviograma = True Then
                y = Chuva(Estação_GrafQ, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))
            End If

            ' If Plotar_falhas = 1 Then
            ' If y <> -1.0 Then
            ' list.Add(x, y)
            ' End If
            ' ElseIf Plotar_falhas = 0 Then
            list.Add(x, y)
            'End If





        Next







        If Hidrograma = True Then
            ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
            Dim myCurve As LineItem = myPane.AddCurve(NOME(Estação_GrafQ), list, Color.Red, SymbolType.None)
            'Dim myCurve2 As BarItem = myPane.AddBar(NOME(Estação_GrafQ), list, Color.Blue)
            'myCurve2.IsY2Axis = True
            ' Fill the area under the curve with a white-red gradient at 45 degrees
            'myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
            'myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

            ' Make the symbols opaque by filling them with white
            myCurve.Symbol.Fill = New Fill(Color.White)

            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()

        ElseIf Cotagrama = True Then
            ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
            Dim myCurve As LineItem = myPane.AddCurve(NOME(Estação_GrafQ), list, Color.LimeGreen, SymbolType.None)

            ' Fill the area under the curve with a white-red gradient at 45 degrees
            'myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
            'myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

            ' Make the symbols opaque by filling them with white
            myCurve.Symbol.Fill = New Fill(Color.White)

            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()

        ElseIf Pluviograma = True Then
            ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
            Dim myCurve As LineItem = myPane.AddCurve(NOME(Estação_GrafQ), list, Color.Blue, SymbolType.None)

            ' Fill the area under the curve with a white-red gradient at 45 degrees
            'myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
            'myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

            ' Make the symbols opaque by filling them with white
            myCurve.Symbol.Fill = New Fill(Color.White)

            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()

        End If

        




    End Sub

    Private Sub Form4_Resize(ByVal sender As Object, ByVal e As System.EventArgs) Handles Me.Resize
        SetSize()
    End Sub

    Private Sub CreateGraph2(ByVal zgc As ZedGraphControl)
        Dim p As Integer


        If Resultado = True Then
            Dim myPane As GraphPane = zgc.GraphPane

            ' Set the titles and axis labels
            myPane.Title.Text = "Hidrograma"
            myPane.XAxis.Title.Text = "Data"
            myPane.YAxis.Title.Text = "Vazão (m³/s)"
            myPane.XAxis.Type = AxisType.Date

            myPane.XAxis.MajorGrid.IsVisible = True
            myPane.YAxis.MajorGrid.IsVisible = True
            myPane.YAxis.Scale.Min = 0

            'myPane.XAxis.Scale.Min = MinTime
            'myPane.XAxis.Scale.Max = MaxTime
            'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
            'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
            'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

            ' Make up some data points from the Sine function
            Dim list = New PointPairList()
            Dim list2 = New PointPairList()
            Dim list3 = New PointPairList()
            'Dim list4 = New PointPairList()
            'Dim list5 = New PointPairList()
            'Dim list6 = New PointPairList()
            'Dim list7 = New PointPairList()
            'Dim list8 = New PointPairList()


            Dim x As XDate
            Dim y As Double
            Dim x2 As XDate
            Dim y2 As Double
            Dim x3 As XDate
            Dim y3 As Double

            'Limiares
            'Dim x4 As XDate
            'Dim y4 As Double
            'Dim x5 As XDate
            'Dim y5 As Double
            'Dim x6 As XDate
            'Dim y6 As Double
            'Dim x7 As XDate
            'Dim y7 As Double
            'Dim x8 As XDate
            'Dim y8 As Double



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
                x3 = datacomeço

                'Limiares
                'x4 = datacomeço
                'x5 = datacomeço
                'x6 = datacomeço
                'x7 = datacomeço
                'x8 = datacomeço


                y = VazaoHIG(postoselecionado, contadordiamenor)
                y2 = QPROPHIG(postoselecionado, contadordiamenor)
                y3 = QOBS(postoselecionado, contadordiamenor)

                'limiares
                'y4 = VazaoHIG(postoselecionado, contadordiamenor)
                'y5 = QPROPHIG(postoselecionado, contadordiamenor)
                'y6 = Mini_TRs(postoselecionado, 3)
                'y7 = Mini_TRs(postoselecionado, 4)
                'y8 = Mini_TRs(postoselecionado, 5)

                If y >= 0 Then
                    list.Add(x, y)
                End If
                If y2 >= 0 Then
                    list2.Add(x2, y2)
                End If
                If y3 >= 0 Then
                    list3.Add(x3, y3)
                End If

            Next




            ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
            'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
            'com posto selecionado no nome da linha: (o certo da aki em cima)

            Dim myCurve2 As LineItem = myPane.AddCurve("Vazão Calculada", list2, Color.Blue, SymbolType.None)
            Dim myCurve As LineItem = myPane.AddCurve("Vazão Prevista", list, Color.Red, SymbolType.None)

            Dim myCurve3 As LineItem = myPane.AddCurve("Vazão Observada", list3, Color.Black, SymbolType.Square)

            myCurve3.Line.Width = espessura_linha
            myCurve2.Line.Width = espessura_linha
            myCurve.Line.Width = espessura_linha

            With myCurve3
                .Line.IsVisible = False          ' <- remove a linha conectando os pontos
                .Symbol.Type = SymbolType.Square ' Circle, Square, Diamond, Triangle, etc.
                .Symbol.Size = 1.5
                .Symbol.Fill = New Fill(Color.Black)      ' cor de preenchimento do símbolo
                .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
            End With


            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()


        ElseIf ResultadoNível = True Then
            Dim myPane As GraphPane = zgc.GraphPane

            ' Set the titles and axis labels
            myPane.Title.Text = "Cotagrama"
            myPane.XAxis.Title.Text = "Data"
            myPane.YAxis.Title.Text = "Nível (metros)"
            myPane.XAxis.Type = AxisType.Date

            myPane.XAxis.MajorGrid.IsVisible = True
            myPane.YAxis.MajorGrid.IsVisible = True
            myPane.YAxis.Scale.Min = 0

            'myPane.XAxis.Scale.Min = MinTime
            'myPane.XAxis.Scale.Max = MaxTime
            'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
            'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
            'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

            ' Make up some data points from the Sine function
            Dim list = New PointPairList()
            Dim list2 = New PointPairList()
            Dim list3 = New PointPairList()
            'Dim list4 = New PointPairList()
            'Dim list5 = New PointPairList()
            'Dim list6 = New PointPairList()
            'Dim list7 = New PointPairList()
            'Dim list8 = New PointPairList()


            Dim x As XDate
            Dim y As Double
            Dim x2 As XDate
            Dim y2 As Double
            Dim x3 As XDate
            Dim y3 As Double

            'Limiares
            'Dim x4 As XDate
            'Dim y4 As Double
            'Dim x5 As XDate
            'Dim y5 As Double
            'Dim x6 As XDate
            'Dim y6 As Double
            'Dim x7 As XDate
            'Dim y7 As Double
            'Dim x8 As XDate
            'Dim y8 As Double



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
                x3 = datacomeço

                'Limiares
                'x4 = datacomeço
                'x5 = datacomeço
                'x6 = datacomeço
                'x7 = datacomeço
                'x8 = datacomeço


                y = CotaHIG(postoselecionado, contadordiamenor)
                y2 = CotaPROPHIG(postoselecionado, contadordiamenor)
                y3 = NivOBS_ref(postoselecionado, contadordiamenor)

                If y >= 0 Then
                    list.Add(x, y)
                End If
                If y2 >= 0 Then
                    list2.Add(x2, y2)
                End If
                If y3 >= 0 Then
                    list3.Add(x3, y3)
                End If

            Next


            ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
            'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
            'com posto selecionado no nome da linha: (o certo da aki em cima)
            Dim myCurve2 As LineItem = myPane.AddCurve("Nível Calculado", list2, Color.Blue, SymbolType.None)
            Dim myCurve As LineItem = myPane.AddCurve("Nível Previsto", list, Color.Red, SymbolType.None)
            Dim myCurve3 As LineItem = myPane.AddCurve("Nível Observado", list3, Color.Black, SymbolType.Square)

            myCurve3.Line.Width = espessura_linha
            myCurve2.Line.Width = espessura_linha
            myCurve.Line.Width = espessura_linha

            With myCurve3
                .Line.IsVisible = False          ' <- remove a linha conectando os pontos
                .Symbol.Type = SymbolType.Square ' Circle, Square, Diamond, Triangle, etc.
                .Symbol.Size = 1.5
                .Symbol.Fill = New Fill(Color.Black)      ' cor de preenchimento do símbolo
                .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
            End With


            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()

        ElseIf ResultadoLimiares = True Then
            Dim myPane As GraphPane = zgc.GraphPane

            ' Set the titles and axis labels
            myPane.Title.Text = "Hidrograma"
            myPane.XAxis.Title.Text = "Data"
            myPane.YAxis.Title.Text = "Vazão (m³/s)"
            myPane.XAxis.Type = AxisType.Date
            'myPane.XAxis.Scale.Min = MinTime
            'myPane.XAxis.Scale.Max = MaxTime
            'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
            'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
            'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

            ' Make up some data points from the Sine function
            Dim list = New PointPairList()
            Dim list2 = New PointPairList()
            Dim list3 = New PointPairList()
            Dim list4 = New PointPairList()
            Dim list5 = New PointPairList()
            Dim list6 = New PointPairList()
            Dim list7 = New PointPairList()
            Dim list8 = New PointPairList()


            Dim x As XDate
            Dim y As Double
            Dim x2 As XDate
            Dim y2 As Double
            Dim x3 As XDate
            Dim y3 As Double

            'Limiares
            Dim x4 As XDate
            Dim y4 As Double
            Dim x5 As XDate
            Dim y5 As Double
            Dim x6 As XDate
            Dim y6 As Double
            Dim x7 As XDate
            Dim y7 As Double
            Dim x8 As XDate
            Dim y8 As Double



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
                x3 = datacomeço

                'Limiares
                x4 = datacomeço
                x5 = datacomeço
                x6 = datacomeço
                x7 = datacomeço
                x8 = datacomeço


                y = VazaoHIG(postoselecionado, contadordiamenor)
                y2 = QPROPHIG(postoselecionado, contadordiamenor)
                y3 = QOBS(postoselecionado, contadordiamenor)

                'limiares
                y4 = Mini_TRs(postoselecionado, 1)
                y5 = Mini_TRs(postoselecionado, 2)
                y6 = Mini_TRs(postoselecionado, 3)
                y7 = Mini_TRs(postoselecionado, 4)
                y8 = Mini_TRs(postoselecionado, 5)

                If y >= 0 Then
                    list.Add(x, y)
                End If
                If y2 >= 0 Then
                    list2.Add(x2, y2)
                End If
                list3.Add(x3, y3)

                list4.Add(x4, y4)
                list5.Add(x5, y5)
                list6.Add(x6, y6)
                list7.Add(x7, y7)
                list8.Add(x8, y8)

            Next




            ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
            'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
            'com posto selecionado no nome da linha: (o certo da aki em cima)
            Dim myCurve3 As LineItem = myPane.AddCurve("Vazão Observada", list3, Color.Green, SymbolType.None)
            Dim myCurve2 As LineItem = myPane.AddCurve("Vazão Calculada", list2, Color.Blue, SymbolType.None)
            Dim myCurve As LineItem = myPane.AddCurve("Vazão Prevista", list, Color.DarkBlue, SymbolType.Square)


            If Mini_Usa(postoselecionado) > 0 Then
                Dim myCurve8 As LineItem = myPane.AddCurve("Limiar 05", list8, Color.DarkRed, SymbolType.None)
                Dim myCurve7 As LineItem = myPane.AddCurve("Limiar 04", list7, Color.Red, SymbolType.None)
                Dim myCurve6 As LineItem = myPane.AddCurve("Limiar 03", list6, Color.Orange, SymbolType.None)
                Dim myCurve5 As LineItem = myPane.AddCurve("Limiar 02", list5, Color.Yellow, SymbolType.None)
                Dim myCurve4 As LineItem = myPane.AddCurve("Limiar 01", list4, Color.LightGreen, SymbolType.None)
            End If

            ' Fill the area under the curve with a white-red gradient at 45 degrees
            ' myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
            'myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

            ' Make the symbols opaque by filling them with white
            'myCurve.Symbol.Fill = New Fill(Color.White)

            myCurve.Symbol.Size = 0.8
            myCurve.Line.Width = 0.2
            ' myCurve.Line.IsVisible = False
            myCurve.Symbol.Fill = New Fill(Color.DarkBlue)

            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()



        ElseIf VisuResultado = True Then
            Dim myPane As GraphPane = zgc.GraphPane

            'myPane.AddY2Axis("Chuva (mm)")

            ' Set the titles and axis labels
            myPane.Title.Text = "Pluviograma"
            myPane.XAxis.Title.Text = "Data"
            myPane.YAxis.Title.Text = "Chuva (mm)"
            myPane.XAxis.Type = AxisType.Date
            'myPane.XAxis.Scale.Min = MinTime
            'myPane.XAxis.Scale.Max = MaxTime
            'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
            'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
            'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

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
                    y = chuva_montante1(postoselecionado, contadordiamenor)
                    y2 = 0
                ElseIf contadordiamenor > tchuvabin1 Then
                    Dim diff As Integer
                    diff = contadordiamenor - tchuvabin1
                    y2 = chuva_montante2(postoselecionado, diff)
                    y = 0
                End If
              
                'y3 = QOBS(postoselecionado, contadordiamenor)

                list.Add(x, y)
                list2.Add(x2, y2)
                'list3.Add(x3, y3)

            Next




            ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
            'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
            'com posto selecionado no nome da linha: (o certo da aki em cima)
            'Dim myCurve3 As LineItem = myPane.AddCurve("Vazão Observada", list3, Color.Green, SymbolType.None)
            Dim myCurve2 As LineItem = myPane.AddCurve("Chuva Prevista", list2, Color.Red, SymbolType.None)
            Dim myCurve As LineItem = myPane.AddCurve("Chuva Observada", list, Color.Blue, SymbolType.None)


            ' Dim myCurve3 As BarItem = myPane.AddBar("Chuva Prevista", list2, Color.Red)
            ' myCurve3.IsX2Axis = True
            'Dim myCurve As BarItem = myPane.AddBar("Chuva Observada", list, Color.Blue)

            ' Fill the area under the curve with a white-red gradient at 45 degrees
            myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
            myCurve2.Line.Fill = New Fill(Color.White, Color.Red, 45.0F)

            ' Make the symbols opaque by filling them with white
            myCurve.Symbol.Fill = New Fill(Color.White)

            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()

        End If


    End Sub

    Private Sub CreateGraph3(ByVal zgc As ZedGraphControl)
        Dim p As Integer


        Dim myPane As GraphPane = zgc.GraphPane

        ' Set the titles and axis labels
        myPane.Title.Text = "Pluviograma"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Chuva (mm)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

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
                y = chuvasub1(postoselecionado, contadordiamenor)
                y2 = 0
            ElseIf contadordiamenor > tchuvabin1 Then
                ' Dim diff As Integer
                ' diff = contadordiamenor - tchuvabin1
                y2 = chuvasub2(postoselecionado, contadordiamenor)
                y = 0
            End If

            'y3 = QOBS(postoselecionado, contadordiamenor)

            list.Add(x, y)
            list2.Add(x2, y2)
            'list3.Add(x3, y3)

        Next




        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        'Dim myCurve3 As LineItem = myPane.AddCurve("Vazão Observada", list3, Color.Green, SymbolType.None)
        Dim myCurve2 As LineItem = myPane.AddCurve("Chuva Prevista", list2, Color.Red, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Chuva Observada", list, Color.Blue, SymbolType.None)

        ' Dim myCurve2 As BarItem = myPane.AddBar("Chuva Prevista", list2, Color.Red)
        'Dim myCurve As BarItem = myPane.AddBar("Chuva Observada", list, Color.Blue)


        ' Fill the area under the curve with a white-red gradient at 45 degrees
        myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
        myCurve2.Line.Fill = New Fill(Color.White, Color.Red, 45.0F)

        ' Make the symbols opaque by filling them with white
        myCurve.Symbol.Fill = New Fill(Color.White)

        ' Fill the axis background with a color gradient
        myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

        ' Fill the pane background with a color gradient
        myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()



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
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

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
            x6 = datacomeço



            y = VazaoHIG(8580, contadordiamenor)
            y2 = QPROPHIG(8580, contadordiamenor)
            y3 = VazaoHIG(8334, contadordiamenor)
            y4 = QPROPHIG(8334, contadordiamenor)
            y5 = VazaoHIG(7684, contadordiamenor)
            y6 = QPROPHIG(7684, contadordiamenor)

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
            If y6 >= 0 Then
                list6.Add(x6, y6)
            End If

        Next




        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        Dim myCurve2 As LineItem = myPane.AddCurve("Rio Uruguai (São Borja)", list2, Color.Blue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Previsão Rio Uruguai (São Borja)", list, Color.SkyBlue, SymbolType.Circle)
        Dim myCurve4 As LineItem = myPane.AddCurve("Rio Taquari-Antas (Encantado)", list4, Color.DarkOrange, SymbolType.None)
        Dim myCurve3 As LineItem = myPane.AddCurve("Previsão Rio Taquari-Antas (Encantado)", list3, Color.Salmon, SymbolType.Circle)
        Dim myCurve6 As LineItem = myPane.AddCurve("Rio dos Sinos (São Leopoldo)", list6, Color.Green, SymbolType.None)
        Dim myCurve5 As LineItem = myPane.AddCurve("Previsão Rio dos Sinos (São Leopoldo)", list5, Color.LightGreen, SymbolType.Circle)

        myCurve.Line.Width = 1.5
        myCurve2.Line.Width = 1.5
        myCurve3.Line.Width = 1.5
        myCurve4.Line.Width = 1.5
        myCurve5.Line.Width = 1.5
        myCurve6.Line.Width = 1.5

        myCurve.Symbol.Size = 0.8
        myCurve.Line.Width = 0.2
        ' myCurve.Line.IsVisible = False
        myCurve.Symbol.Fill = New Fill(Color.LightSkyBlue)
        'myCurve.Line.DashOn = 1
        'myCurve.Line.Style = Drawing2D.DashStyle.DashDot

        myCurve3.Symbol.Size = 0.8
        myCurve3.Line.Width = 0.2
        'myCurve3.Line.IsVisible = False
        myCurve3.Symbol.Fill = New Fill(Color.LightSalmon)
        'myCurve3.Line.DashOn = 3
        'myCurve3.Line.Style = Drawing2D.DashStyle.DashDot

        myCurve5.Symbol.Size = 0.8
        myCurve5.Line.Width = 0.2
        'myCurve3.Line.IsVisible = False
        myCurve5.Symbol.Fill = New Fill(Color.LightGreen)
        ' myCurve5.Line.DashOn = 5
        'myCurve5.Line.Style = Drawing2D.DashStyle.DashDot

        ' Fill the area under the curve with a white-red gradient at 45 degrees
        ' myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
        'myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

        ' Make the symbols opaque by filling them with white
        'myCurve.Symbol.Fill = New Fill(Color.White)

        ' Fill the axis background with a color gradient
        myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

        ' Fill the pane background with a color gradient
        myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()


    End Sub

    Private Sub CreateGraph_HidrogramaPosProcessado(ByVal zgc As ZedGraphControl)
        Dim p As Integer



        Dim myPane As GraphPane = zgc.GraphPane

        ' Set the titles and axis labels
        myPane.Title.Text = "Hidrograma Pós-Processado"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Vazão (m³/s)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        ' Make up some data points from the Sine function
        Dim list = New PointPairList()
        Dim list2 = New PointPairList()
        Dim list3 = New PointPairList()
        'Dim list4 = New PointPairList()
        'Dim list5 = New PointPairList()
        'Dim list6 = New PointPairList()
        'Dim list7 = New PointPairList()
        'Dim list8 = New PointPairList()


        Dim x As XDate
        Dim y As Double
        Dim x2 As XDate
        Dim y2 As Double
        Dim x3 As XDate
        Dim y3 As Double

        'Limiares
        'Dim x4 As XDate
        'Dim y4 As Double
        'Dim x5 As XDate
        'Dim y5 As Double
        'Dim x6 As XDate
        'Dim y6 As Double
        'Dim x7 As XDate
        'Dim y7 As Double
        'Dim x8 As XDate
        'Dim y8 As Double



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
            x3 = datacomeço

            'Limiares
            ' x4 = datacomeço
            'x5 = datacomeço
            'x6 = datacomeço
            'x7 = datacomeço
            'x8 = datacomeço


            y = VazaoHIG_CORR(postoselecionado, contadordiamenor)
            y2 = QPROPHIG_CORR(postoselecionado, contadordiamenor)
            y3 = QOBS(postoselecionado, contadordiamenor)

            'limiares
            'y4 = VazaoHIG(postoselecionado, contadordiamenor)
            ' y5 = QPROPHIG(postoselecionado, contadordiamenor)
            'y6 = Mini_TRs(postoselecionado, 3)
            'y7 = Mini_TRs(postoselecionado, 4)
            'y8 = Mini_TRs(postoselecionado, 5)

            If y >= 0 Then
                list.Add(x, y)
            End If
            If y2 >= 0 Then
                list2.Add(x2, y2)
            End If
            list3.Add(x3, y3)

            'If y4 >= 0 Then
            '    list4.Add(x4, y4)
            'End If
            'If y5 >= 0 Then
            '    list5.Add(x5, y5)
            'End If

            'list4.Add(x4, y4)
            'list5.Add(x5, y5)
            'list6.Add(x6, y6)
            'list7.Add(x7, y7)
            'list8.Add(x8, y8)

        Next




        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        Dim myCurve3 As LineItem = myPane.AddCurve("Vazão Observada", list3, Color.Green, SymbolType.None)
        Dim myCurve2 As LineItem = myPane.AddCurve("Vazão Calculada Pós-Processada", list2, Color.Blue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Vazão Prevista Pós-Processada", list, Color.Red, SymbolType.None)


        'If Mini_Usa(postoselecionado) > 0 Then
        '    Dim myCurve8 As LineItem = myPane.AddCurve("Limiar 05", list8, Color.DarkRed, SymbolType.None)
        '    Dim myCurve7 As LineItem = myPane.AddCurve("Limiar 04", list7, Color.Red, SymbolType.None)
        '    Dim myCurve6 As LineItem = myPane.AddCurve("Limiar 03", list6, Color.Orange, SymbolType.None)
        'Dim myCurve5 As LineItem = myPane.AddCurve("Limiar 02", list5, Color.Yellow, SymbolType.None)
        ' Dim myCurve4 As LineItem = myPane.AddCurve("Limiar 01", list4, Color.DarkGreen, SymbolType.None)
        'End If

        ' Fill the area under the curve with a white-red gradient at 45 degrees
        ' myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
        'myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

        ' Make the symbols opaque by filling them with white
        myCurve.Symbol.Fill = New Fill(Color.White)

        ' Fill the axis background with a color gradient
        myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

        ' Fill the pane background with a color gradient
        myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()

    End Sub

    Private Sub CreateGraph_CotagramaPosProcessado(ByVal zgc As ZedGraphControl)
        Dim p As Integer



        Dim myPane As GraphPane = zgc.GraphPane

        ' Set the titles and axis labels
        myPane.Title.Text = "Cotagrama Pós-Processado"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Nível (m)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        ' Make up some data points from the Sine function
        Dim list = New PointPairList()
        Dim list2 = New PointPairList()
        Dim list3 = New PointPairList()
        'Dim list4 = New PointPairList()
        'Dim list5 = New PointPairList()
        'Dim list6 = New PointPairList()
        'Dim list7 = New PointPairList()
        'Dim list8 = New PointPairList()


        Dim x As XDate
        Dim y As Double
        Dim x2 As XDate
        Dim y2 As Double
        Dim x3 As XDate
        Dim y3 As Double

        'Limiares
        'Dim x4 As XDate
        'Dim y4 As Double
        'Dim x5 As XDate
        'Dim y5 As Double
        'Dim x6 As XDate
        'Dim y6 As Double
        'Dim x7 As XDate
        'Dim y7 As Double
        'Dim x8 As XDate
        'Dim y8 As Double



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
            x3 = datacomeço

            'Limiares
            ' x4 = datacomeço
            'x5 = datacomeço
            'x6 = datacomeço
            'x7 = datacomeço
            'x8 = datacomeço


            y = CotaHIG_ref_CORR(postoselecionado, contadordiamenor)
            y2 = CotaPROPHIG_ref_CORR(postoselecionado, contadordiamenor)
            y3 = NivOBS_ref(postoselecionado, contadordiamenor)

            'limiares
            'y4 = VazaoHIG(postoselecionado, contadordiamenor)
            ' y5 = QPROPHIG(postoselecionado, contadordiamenor)
            'y6 = Mini_TRs(postoselecionado, 3)
            'y7 = Mini_TRs(postoselecionado, 4)
            'y8 = Mini_TRs(postoselecionado, 5)

            If y >= 0 Then
                list.Add(x, y)
            End If
            If y2 >= 0 Then
                list2.Add(x2, y2)
            End If
            list3.Add(x3, y3)

            'If y4 >= 0 Then
            '    list4.Add(x4, y4)
            'End If
            'If y5 >= 0 Then
            '    list5.Add(x5, y5)
            'End If

            'list4.Add(x4, y4)
            'list5.Add(x5, y5)
            'list6.Add(x6, y6)
            'list7.Add(x7, y7)
            'list8.Add(x8, y8)

        Next




        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        Dim myCurve3 As LineItem = myPane.AddCurve("Nível observado de referência", list3, Color.Green, SymbolType.None)
        Dim myCurve2 As LineItem = myPane.AddCurve("Nível calculado pós-processado", list2, Color.Blue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Nível previsto pós-processado", list, Color.Red, SymbolType.None)


        'If Mini_Usa(postoselecionado) > 0 Then
        '    Dim myCurve8 As LineItem = myPane.AddCurve("Limiar 05", list8, Color.DarkRed, SymbolType.None)
        '    Dim myCurve7 As LineItem = myPane.AddCurve("Limiar 04", list7, Color.Red, SymbolType.None)
        '    Dim myCurve6 As LineItem = myPane.AddCurve("Limiar 03", list6, Color.Orange, SymbolType.None)
        'Dim myCurve5 As LineItem = myPane.AddCurve("Limiar 02", list5, Color.Yellow, SymbolType.None)
        ' Dim myCurve4 As LineItem = myPane.AddCurve("Limiar 01", list4, Color.DarkGreen, SymbolType.None)
        'End If

        ' Fill the area under the curve with a white-red gradient at 45 degrees
        ' myCurve.Line.Fill = New Fill(Color.White, Color.Blue, 45.0F)
        'myCurve2.Line.Fill = New Fill(Color.White, Color.Yellow, 45.0F)

        ' Make the symbols opaque by filling them with white
        myCurve.Symbol.Fill = New Fill(Color.White)

        ' Fill the axis background with a color gradient
        myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)

        ' Fill the pane background with a color gradient
        myPane.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()

    End Sub

End Class