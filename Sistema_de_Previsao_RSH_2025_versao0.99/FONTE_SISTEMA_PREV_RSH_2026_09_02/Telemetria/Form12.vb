Imports ZedGraph
Imports System.Drawing


Public Class Form12

    Dim MiniSim(3), MiniPrev(3), MiniObs(3) As Integer
    Dim NomeSim(3), NomePrev(3), NomeObs(3) As String
    Dim SubIniPlu, SubFimPlu As Integer


    Private Sub Form12_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load

        Me.ESCOLHE_OPCAO() 'Define qual gráfico vai plotar

        CreateGraph4(zg1)
        CreateGraph3(zg2)


        If TotalCommander = True Then

            Dim log_file As String
            log_file = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Log_operação_automatica.txt"
            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Abriu a janela de salvar hidrograma padrão.")
            FileClose(99)

            Me.SAVESCREENSHOT_AUTO()

            FileOpen(99, log_file, OpenMode.Append)
            PrintLine(99, Now.Date & " " & Now.Hour & ":" & Now.Minute & " " & "Salvou hidrograma padrão.")
            FileClose(99)

            Me.Close()


        End If


        'If TotalCommander = True Then
        '    Me.SAVESCREENSHOT_AUTO()
        '    Me.Close()
        'End If

        'SetSize()
    End Sub

    Sub ESCOLHE_OPCAO()
        If OpçãoGrafico = 1 Then 'Baixo Uruguai
            MiniSim(1) = 8597
            MiniPrev(1) = 8597
            MiniObs(1) = 8597
            NomeSim(1) = "Uruguaiana Simulada"
            NomePrev(1) = "Uruguaiana Prevista"
            NomeObs(1) = "Uruguaiana Observada"

            MiniSim(2) = 8579
            MiniPrev(2) = 8579
            MiniObs(2) = 8579
            NomeSim(2) = "Passo São Borja Simulada"
            NomePrev(2) = "Passo São Borja Prevista"
            NomeObs(2) = "Passo São Borja Observada"

            MiniSim(3) = 8401
            MiniPrev(3) = 8401
            MiniObs(3) = 8401
            NomeSim(3) = "Passo Mariano Pinto Simulada"
            NomePrev(3) = "Passo Mariano Pinto Prevista"
            NomeObs(3) = "Passo Mariano Pinto Observada"


            SubIniPlu = 20
            SubFimPlu = 70

        ElseIf OpçãoGrafico = 3 Then 'Ijuí, Ibicuí e Quaraí

            MiniSim(1) = 8281
            MiniPrev(1) = 8281
            MiniObs(1) = 8281
            NomeSim(1) = "UHE São José Jusante Simulada"
            NomePrev(1) = "UHE São José Jusante Prevista"
            NomeObs(1) = "UHE São José Observada"

            MiniSim(2) = 8306
            MiniPrev(2) = 8306
            MiniObs(2) = 8306
            NomeSim(2) = "Manoel Viana Simulada"
            NomePrev(2) = "Manoel Viana Prevista"
            NomeObs(2) = "Manoel Viana Observada"

            MiniSim(3) = 7655
            MiniPrev(3) = 7655
            MiniObs(3) = 7655
            NomeSim(3) = "Quaraí Simulada"
            NomePrev(3) = "Quaraí Prevista"
            NomeObs(3) = "Quaraí Observada"


            SubIniPlu = 70
            SubFimPlu = 79

        ElseIf OpçãoGrafico = 2 Then 'Médio uruguai

            MiniSim(1) = 8009
            MiniPrev(1) = 8009
            MiniObs(1) = 8009
            NomeSim(1) = "UHE Monjolinho Jusante Simulada"
            NomePrev(1) = "UHE Monjolinho Jusante Prevista"
            NomeObs(1) = "UHE Monjolinho Observada"

            MiniSim(2) = 8468
            MiniPrev(2) = 8468
            MiniObs(2) = 8468
            NomeSim(2) = "UHE Itá Montante Simulada"
            NomePrev(2) = "UHE Itá Montante  Prevista"
            NomeObs(2) = "UHE Itá Montante Observada"

            MiniSim(3) = 8524
            MiniPrev(3) = 8524
            MiniObs(3) = 8524
            NomeSim(3) = "Itapiranga Simulada"
            NomePrev(3) = "Itapiranga Prevista"
            NomeObs(3) = "Itapiranga Observada"

            SubIniPlu = 14
            SubFimPlu = 50

        ElseIf OpçãoGrafico = 7 Then 'Alto Uruguai

            MiniSim(1) = 8280
            MiniPrev(1) = 8280
            MiniObs(1) = 8280
            NomeSim(1) = "UHE Barra Grande Afluente Simulada"
            NomePrev(1) = "UHE Barra Grande Afluente Prevista"
            NomeObs(1) = "UHE Barra Grande Afluente Observada"

            MiniSim(2) = 8392
            MiniPrev(2) = 8392
            MiniObs(2) = 8392
            NomeSim(2) = "UHE Campos Novos Afluente Simulada"
            NomePrev(2) = "UHE Campos Novos Afluente Prevista"
            NomeObs(2) = "UHE Campos Novos Afluente Observada"

            MiniSim(3) = 8434
            MiniPrev(3) = 8434
            MiniObs(3) = 8434
            NomeSim(3) = "UHE Machadinho Afluente Simulada"
            NomePrev(3) = "UHE Machadinho Afluente Prevista"
            NomeObs(3) = "UHE Machadinho Afluente Observada"


            SubIniPlu = 6
            SubFimPlu = 38

        ElseIf OpçãoGrafico = 4 Then 'Jacuí e Taquari

            MiniSim(1) = 8424
            MiniPrev(1) = 8424
            MiniObs(1) = 8424
            NomeSim(1) = "Rio Jacuí em Rio Pardo Simulada"
            NomePrev(1) = "Rio Jacuí em Rio Pardo Prevista"
            NomeObs(1) = "Rio Jacuí em Rio Pardo Observada"

            MiniSim(2) = 8255
            MiniPrev(2) = 8255
            MiniObs(2) = 8255
            NomeSim(2) = "UHE 14 de Julho Afluente Simulada"
            NomePrev(2) = "UHE 14 de Julho Afluente Prevista"
            NomeObs(2) = "UHE 14 de Julho Afluente Observada"

            MiniSim(3) = 8325
            MiniPrev(3) = 8325
            MiniObs(3) = 8325
            NomeSim(3) = "Encantado Simulada"
            NomePrev(3) = "Encantado Prevista"
            NomeObs(3) = "Encantado Observada"


            SubIniPlu = 80
            SubFimPlu = 118

        ElseIf OpçãoGrafico = 5 Then 'Sinos, Caí e Gravatai

            MiniSim(1) = 7897
            MiniPrev(1) = 7897
            MiniObs(1) = 7897
            NomeSim(1) = "Passo Montenegro Simulada"
            NomePrev(1) = "Passo Montenegro Prevista"
            NomeObs(1) = "Passo Montenegro Observada"

            MiniSim(2) = 7546
            MiniPrev(2) = 7546
            MiniObs(2) = 7546
            NomeSim(2) = "Campo Bom Simulada"
            NomePrev(2) = "Campo Bom Prevista"
            NomeObs(2) = "Campo Bom Observada"

            MiniSim(3) = 7167
            MiniPrev(3) = 7167
            MiniObs(3) = 7167
            NomeSim(3) = "CORSAN Alvorada Simulada"
            NomePrev(3) = "CORSAN Alvorada Prevista"
            NomeObs(3) = "CORSAN Alvorada Observada"


            SubIniPlu = 96
            SubFimPlu = 97

        ElseIf OpçãoGrafico = 6 Then 'Litoral Doce Sul

            MiniSim(1) = 8317
            MiniPrev(1) = 8317
            MiniObs(1) = 8317
            NomeSim(1) = "Passo do Mendonça Simulada"
            NomePrev(1) = "Passo do Mendonça Prevista"
            NomeObs(1) = "Passo do Mendonça Observada"

            MiniSim(2) = 7156
            MiniPrev(2) = 7156
            MiniObs(2) = 7156
            NomeSim(2) = "Pedro Osório Simulada"
            NomePrev(2) = "Pedro Osório Prevista"
            NomeObs(2) = "Pedro Osório Observada"

            MiniSim(3) = 6603
            MiniPrev(3) = 6603
            MiniObs(3) = 6603
            NomeSim(3) = "Arroio Grande Simulada"
            NomePrev(3) = "Arroio Grande Prevista"
            NomeObs(3) = "Arroio Grande Observada"


            SubIniPlu = 98
            SubFimPlu = 99

        End If


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
        Dim list7 = New PointPairList
        Dim list8 = New PointPairList
        Dim list9 = New PointPairList

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
        Dim x7 As XDate
        Dim y7 As Double
        Dim x8 As XDate
        Dim y8 As Double
        Dim x9 As XDate
        Dim y9 As Double


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
        'dataprevY = Começoresultado.AddHours(tamdoresultado - 21 * 24 - 10 * 24)
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
            x6 = datacomeço
            x7 = datacomeço
            x8 = datacomeço
            x9 = datacomeço



            y = VazaoHIG(MiniObs(1), contadordiamenor)
            y2 = QPROPHIG(MiniObs(1), contadordiamenor)
            y5 = QOBS(MiniObs(1), contadordiamenor)


            y3 = VazaoHIG(MiniObs(2), contadordiamenor)
            y4 = QPROPHIG(MiniObs(2), contadordiamenor)
            y6 = QOBS(MiniObs(2), contadordiamenor)

            y7 = VazaoHIG(MiniObs(3), contadordiamenor)
            y8 = QPROPHIG(MiniObs(3), contadordiamenor)
            y9 = QOBS(MiniObs(3), contadordiamenor)



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
            If y7 >= 0 Then
                list7.Add(x7, y7)
            End If
            If y8 >= 0 Then
                list8.Add(x8, y8)
            End If
            If y9 >= 0 Then
                list9.Add(x9, y9)
            End If




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
                If y7 > maiorY Then
                    maiorY = y7
                End If
                If y8 > maiorY Then
                    maiorY = y8
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



        Dim myCurve2 As LineItem = myPane.AddCurve(NomeSim(1), list2, Color.MediumBlue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve(NomePrev(1), list, Color.CornflowerBlue, SymbolType.None)
        'Dim myCurve5 As LineItem = myPane.AddCurve(NomeObs(1), list5, Color.LimeGreen, SymbolType.None)

        Dim myCurve4 As LineItem = myPane.AddCurve(NomeSim(2), list4, Color.DarkOrange, SymbolType.None)
        Dim myCurve3 As LineItem = myPane.AddCurve(NomePrev(2), list3, Color.Salmon, SymbolType.None)
        'Dim myCurve6 As LineItem = myPane.AddCurve(NomeObs(2), list6, Color.DarkGreen, SymbolType.None)

        Dim myCurve8 As LineItem = myPane.AddCurve(NomeSim(3), list8, Color.DarkGreen, SymbolType.None)
        Dim myCurve7 As LineItem = myPane.AddCurve(NomePrev(3), list7, Color.LimeGreen, SymbolType.None)
        'Dim myCurve9 As LineItem = myPane.AddCurve(NomeObs(1), list5, Color.LimeGreen, SymbolType.None)

        myCurve.Line.Width = 1.5
        myCurve2.Line.Width = 1.5
        myCurve3.Line.Width = 1.5
        myCurve4.Line.Width = 1.5
        'myCurve5.Line.Width = 1.5
        'myCurve6.Line.Width = 1.5
        myCurve7.Line.Width = 1.5
        myCurve8.Line.Width = 1.5
        'myCurve9.Line.Width = 1.5

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

        myCurve7.Line.Width = 1.5
        myCurve7.Line.DashOn = True
        myCurve7.Line.Style = Drawing2D.DashStyle.Dash
        myCurve7.Line.IsAntiAlias = True
        myCurve7.Line.StepType = StepType.ForwardSegment
        myCurve7.Line.IsSmooth = True
        myCurve7.Line.SmoothTension = 0.05

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
        myPane.Title.Text = "Pluviograma"
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

            Dim NPluv As Integer
            NPluv = SubFimPlu - SubIniPlu + 1

            If contadordiamenor <= tchuvabin1 Then
                y = 0
                For subbb As Integer = SubIniPlu To SubFimPlu
                    y += chuvasub1(subbb, contadordiamenor)
                Next
                y = y / NPluv 'faz uma média simples na chuva média
                y2 = 0
            ElseIf contadordiamenor > tchuvabin1 Then
                ' Dim diff As Integer
                ' diff = contadordiamenor - tchuvabin1
                y2 = 0
                For subbb As Integer = SubIniPlu To SubFimPlu
                    y2 += chuvasub2(subbb, contadordiamenor)
                Next
                y2 = y2 / NPluv 'faz uma média simples na chuva média
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
        arquivo = diretóriodoprojeto & "\Previsão_" & Now.Year & mes & dia & hora & ".bmp"

        bmp.Save(arquivo)

        ''Abre o diálogo para selecionar o local onde vai salvar o novo shape
        'sfd.Filter = "Imagem BMP (.bmp)|*.bmp"
        'If sfd.ShowDialog() = Windows.Forms.DialogResult.OK Then

        '    bmp.Save(sfd.FileName)
        'End If


    End Sub

    Private Sub Panel3_Paint(ByVal sender As System.Object, ByVal e As System.Windows.Forms.PaintEventArgs) Handles Panel3.Paint

    End Sub

    Private Sub zg1_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles zg1.Load

    End Sub
End Class