Imports System.Drawing
Imports System.Drawing.Drawing2D
Imports System.Globalization
Imports System.Text
Imports System.Windows.Forms
Imports ZedGraph

Public Class Form11

    Public espessura_linha As Single

    ' Referência da curva base (prevista) e das bandas
    Private _curveBase As LineItem = Nothing
    Private _curveBandSup As LineItem = Nothing
    Private _curveBandInf As LineItem = Nothing


    Private Sub Form11_Load(sender As Object, e As EventArgs) Handles MyBase.Load
        Me.CARREGA_DADOS_ENTRADA()
        Me.Init_dud_alfa()
        If ResultadoVazaopp = True Then
            Corrigir_AR1_Simples_VAZOES_GRAFICO()
            CreateGraph_HidrogramaPosProcessado(zg1)
            SetSize()
            tbVIES.Enabled = False
            btPLUS.Enabled = False
            btMINUS.Enabled = False
        ElseIf ResultadoNívelpp = True Then
            Corrigir_AR1_Simples_NIVEIS_GRAFICO()
            CreateGraph_CotagramaPosProcessado(zg1)
            SetSize()
        ElseIf ResultadoNívelref = True Then
            Corrigir_VIES_Simples_NIVEIS_GRAFICO()
            CreateGraph_CotagramaReferenciado(zg1)
            SetSize()
            dud_alfa.Enabled = False
            dud_alfa2.Enabled = False
            bt_dud_minus.Enabled = False
            bt_dud_plus.Enabled = False
        ElseIf Resultado = True Then
            CreateGraph2(zg1)
            SetSize()
            tbVIES.Enabled = False
            dud_alfa.Enabled = False
            Button1.Enabled = False
            dud_alfa2.Enabled = False
            bt_dud_minus.Enabled = False
            bt_dud_plus.Enabled = False
        ElseIf ResultadoNível = True Then
            CreateGraph2(zg1)
            SetSize()
            tbVIES.Enabled = False
            dud_alfa.Enabled = False
            Button1.Enabled = False
            dud_alfa2.Enabled = False
            bt_dud_minus.Enabled = False
            bt_dud_plus.Enabled = False
        End If
        Me.CENTRAR_NA_PREVISAO()
    End Sub
    Private Sub SetSize()
        zg1.Location = New Point(10, 10)
        ' Leave a small margin around the outside of the control
        zg1.Size = New Size(ClientRectangle.Width - 220, ClientRectangle.Height - 20)
    End Sub
    Private Sub Form11_Resize(ByVal sender As Object, ByVal e As System.EventArgs) Handles Me.Resize
        SetSize()
    End Sub

    Sub CARREGA_DADOS_ENTRADA()
        tbMini.Text = postoselecionado
        dud_alfa2.Text = ALFA_VAZOES(postoselecionado)
        'MsgBox(BIAS_NIVEL(postoselecionado))
        tbVIES.Text = BIAS_NIVEL(postoselecionado)
        'MsgBox(tbVIES.Text)

        espessura_linha = 2.0
    End Sub

    Private Sub CreateGraph_HidrogramaPosProcessado(ByVal zgc As ZedGraphControl)

        ' --- LIMPAR ZEDGRAPH ANTES DE PLOTAR ---
        Dim myPane As GraphPane = zgc.GraphPane

        myPane.CurveList.Clear()          ' remove todas as curvas anteriores
        myPane.GraphObjList.Clear()       ' remove linhas/caixas/textos desenhados

        ' Reseta autoscale (evita herdar limites antigos)
        myPane.XAxis.Scale.MinAuto = True
        myPane.XAxis.Scale.MaxAuto = True
        myPane.YAxis.Scale.MinAuto = True
        myPane.YAxis.Scale.MaxAuto = True

        ' (Opcional) limpar títulos anteriores
        myPane.Title.Text = String.Empty
        myPane.XAxis.Title.Text = String.Empty
        myPane.YAxis.Title.Text = String.Empty

        ' (Opcional) limpar preenchimentos antigos
        myPane.Chart.Fill = Nothing
        myPane.Fill = Nothing

        ' Dá um refresh visual do controle (sem recomputar escalas ainda)
        zgc.Invalidate()
        ' --- FIM LIMPEZA ---




        Dim p As Integer

        ' Set the titles and axis labels
        myPane.Title.Text = "Hidrograma Pós-Processado (" & Cod_graf(postoselecionado) & " - " & Nome_graf(postoselecionado) & ")"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Vazão (m³/s)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        myPane.XAxis.MajorGrid.IsVisible = True
        myPane.YAxis.MajorGrid.IsVisible = True

        myPane.YAxis.Scale.Min = 0

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

            If y3 >= 0 Then
                list3.Add(x3, y3)
            End If

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

        Dim myCurve2 As LineItem = myPane.AddCurve("Vazão Calculada Pós-Processada", list2, Color.Blue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Vazão Prevista Pós-Processada", list, Color.Red, SymbolType.None)
        Dim myCurve3 As LineItem = myPane.AddCurve("Vazão Observada", list3, Color.Black, SymbolType.Square)

        myCurve3.Line.Width = espessura_linha
        myCurve2.Line.Width = espessura_linha
        myCurve.Line.Width = espessura_linha

        With myCurve3
            .Line.IsVisible = False          ' <- remove a linha conectando os pontos
            .Symbol.Type = SymbolType.Square ' Circle, Square, Diamond, Triangle, etc.
            .Symbol.Size = 2.0
            .Symbol.Fill = New Fill(Color.Black)      ' cor de preenchimento do símbolo
            .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
        End With

        ' Guarde a referência para uso nas bandas:
        _curveBase = myCurve

        ' Como você limpa/repinta às vezes, também zere as bandas antigas:
        _curveBandSup = Nothing
        _curveBandInf = Nothing


        'With myCurve
        '    .Line.IsVisible = False          ' <- remove a linha conectando os pontos
        '    .Symbol.Type = SymbolType.Circle ' Circle, Square, Diamond, Triangle, etc.
        '    .Symbol.Size = 2
        '    .Symbol.Fill = New Fill(Color.Red)      ' cor de preenchimento do símbolo
        '    .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
        'End With


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
        'myCurve.Symbol.Fill = New Fill(Color.White)

        ' Fill the axis background with a color gradient

        'myPane.Chart.Fill = New Fill(Color.White, Color.LightGoldenrodYellow, 45.0F)
        myPane.Chart.Fill = New Fill(Color.White)

        ' Fill the pane background with a color gradient
        '.Fill = New Fill(Color.White, Color.FromArgb(220, 220, 255), 45.0F)
        myPane.Fill = New Fill(Color.White)


        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()

        zgc.Invalidate()   ' repinta


    End Sub

    Private Sub CreateGraph_CotagramaPosProcessado(ByVal zgc As ZedGraphControl)

        ' --- LIMPAR ZEDGRAPH ANTES DE PLOTAR ---
        Dim myPane As GraphPane = zgc.GraphPane

        myPane.CurveList.Clear()          ' remove todas as curvas anteriores
        myPane.GraphObjList.Clear()       ' remove linhas/caixas/textos desenhados

        ' Reseta autoscale (evita herdar limites antigos)
        myPane.XAxis.Scale.MinAuto = True
        myPane.XAxis.Scale.MaxAuto = True
        myPane.YAxis.Scale.MinAuto = True
        myPane.YAxis.Scale.MaxAuto = True

        ' (Opcional) limpar títulos anteriores
        myPane.Title.Text = String.Empty
        myPane.XAxis.Title.Text = String.Empty
        myPane.YAxis.Title.Text = String.Empty

        ' (Opcional) limpar preenchimentos antigos
        myPane.Chart.Fill = Nothing
        myPane.Fill = Nothing

        ' Dá um refresh visual do controle (sem recomputar escalas ainda)
        zgc.Invalidate()
        ' --- FIM LIMPEZA ---

        Dim p As Integer

        ' Set the titles and axis labels
        myPane.Title.Text = "Cotagrama Pós-Processado (" & Cod_graf(postoselecionado) & " - " & Nome_graf(postoselecionado) & ")"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Nível (m)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        myPane.XAxis.MajorGrid.IsVisible = True
        myPane.YAxis.MajorGrid.IsVisible = True

        myPane.YAxis.Scale.Min = 0

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

            If y3 >= 0 Then
                list3.Add(x3, y3)
            End If

        Next




        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        Dim myCurve2 As LineItem = myPane.AddCurve("Nível calculado pós-processado", list2, Color.Blue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Nível previsto pós-processado", list, Color.Red, SymbolType.None)

        Dim myCurve3 As LineItem = myPane.AddCurve("Nível observado de referência", list3, Color.Black, SymbolType.Square)

        myCurve3.Line.Width = espessura_linha
        myCurve2.Line.Width = espessura_linha
        myCurve.Line.Width = espessura_linha

        With myCurve3
            .Line.IsVisible = False          ' <- remove a linha conectando os pontos
            .Symbol.Type = SymbolType.Square ' Circle, Square, Diamond, Triangle, etc.
            .Symbol.Size = 2.0
            .Symbol.Fill = New Fill(Color.Black)      ' cor de preenchimento do símbolo
            .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
        End With

        ' Guarde a referência para uso nas bandas:
        _curveBase = myCurve

        ' Como você limpa/repinta às vezes, também zere as bandas antigas:
        _curveBandSup = Nothing
        _curveBandInf = Nothing

        ' Fill the axis background with a color gradient
        myPane.Chart.Fill = New Fill(Color.White)

        ' Fill the pane background with a color gradient
        myPane.Fill = New Fill(Color.White)

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()

        zgc.Invalidate()   ' repinta


    End Sub



    Private Sub CreateGraph_CotagramaReferenciado(ByVal zgc As ZedGraphControl)

        ' --- LIMPAR ZEDGRAPH ANTES DE PLOTAR ---
        Dim myPane As GraphPane = zgc.GraphPane

        myPane.CurveList.Clear()          ' remove todas as curvas anteriores
        myPane.GraphObjList.Clear()       ' remove linhas/caixas/textos desenhados

        ' Reseta autoscale (evita herdar limites antigos)
        myPane.XAxis.Scale.MinAuto = True
        myPane.XAxis.Scale.MaxAuto = True
        myPane.YAxis.Scale.MinAuto = True
        myPane.YAxis.Scale.MaxAuto = True

        ' (Opcional) limpar títulos anteriores
        myPane.Title.Text = String.Empty
        myPane.XAxis.Title.Text = String.Empty
        myPane.YAxis.Title.Text = String.Empty

        ' (Opcional) limpar preenchimentos antigos
        myPane.Chart.Fill = Nothing
        myPane.Fill = Nothing

        ' Dá um refresh visual do controle (sem recomputar escalas ainda)
        zgc.Invalidate()
        ' --- FIM LIMPEZA ---

        Dim p As Integer

        ' Set the titles and axis labels
        myPane.Title.Text = "Cotagrama Referenciado (" & Cod_graf(postoselecionado) & " - " & Nome_graf(postoselecionado) & ")"
        myPane.XAxis.Title.Text = "Data"
        myPane.YAxis.Title.Text = "Nível (m)"
        myPane.XAxis.Type = AxisType.Date
        'myPane.XAxis.Scale.Min = MinTime
        'myPane.XAxis.Scale.Max = MaxTime
        'myPane.XAxis.Scale.MinorStep = RangeAsseX / 30
        'myPane.XAxis.Scale.MajorStep = RangeAsseX / 6
        'myPane.XAxis.Scale.MajorUnit = DateUnit.Second

        myPane.XAxis.MajorGrid.IsVisible = True
        myPane.YAxis.MajorGrid.IsVisible = True

        myPane.YAxis.Scale.Min = 0

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


            y = CotaHIG_ref(postoselecionado, contadordiamenor)
            y2 = CotaPROPHIG_ref(postoselecionado, contadordiamenor)
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

            If y3 >= 0 Then
                list3.Add(x3, y3)
            End If

        Next




        ' Generate a blue curve with circle symbols, and "My Curve 2" in the legend
        'Dim myCurve As LineItem = myPane.AddCurve("Vazão", list, Color.Blue, SymbolType.None)
        'com posto selecionado no nome da linha: (o certo da aki em cima)
        Dim myCurve2 As LineItem = myPane.AddCurve("Nível calculado referenciado", list2, Color.Blue, SymbolType.None)
        Dim myCurve As LineItem = myPane.AddCurve("Nível previsto referenciado", list, Color.Red, SymbolType.None)
        Dim myCurve3 As LineItem = myPane.AddCurve("Nível observado de referência", list3, Color.Black, SymbolType.Square)

        myCurve3.Line.Width = espessura_linha
        myCurve2.Line.Width = espessura_linha
        myCurve.Line.Width = espessura_linha

        With myCurve3
            .Line.IsVisible = False          ' <- remove a linha conectando os pontos
            .Symbol.Type = SymbolType.Square ' Circle, Square, Diamond, Triangle, etc.
            .Symbol.Size = 2
            .Symbol.Fill = New Fill(Color.Black)      ' cor de preenchimento do símbolo
            .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
        End With

        ' Guarde a referência para uso nas bandas:
        _curveBase = myCurve

        ' Como você limpa/repinta às vezes, também zere as bandas antigas:
        _curveBandSup = Nothing
        _curveBandInf = Nothing

        ' Fill the axis background with a color gradient
        myPane.Chart.Fill = New Fill(Color.White)

        ' Fill the pane background with a color gradient
        myPane.Fill = New Fill(Color.White)

        ' Calculate the Axis Scale Ranges
        zgc.AxisChange()

        zgc.Invalidate()   ' repinta


    End Sub


    ' Chame isto no Form_Load
    Private Sub Init_dud_alfa()
        dud_alfa.Items.Clear()
        dud_alfa.Sorted = False
        dud_alfa.Wrap = False
        dud_alfa.ReadOnly = False

        Dim fmt = Globalization.CultureInfo.CurrentCulture
        For i As Integer = 100 To 0 Step -1
            dud_alfa.Items.Add((i / 100.0).ToString("0.00", fmt))
        Next

        Set_dud_alfa_Value(ALFA_VAZOES(postoselecionado)) ' sua função de setar valor inicial
    End Sub


    ' Seta o valor garantindo que existe na lista (arredonda e limita)
    Private Sub Set_dud_alfa_Value(v As Double)
        If v < 0 Then v = 0
        If v > 1 Then v = 1
        v = Math.Round(v, 2, MidpointRounding.AwayFromZero)

        Dim txt As String = v.ToString("0.00", CultureInfo.CurrentCulture)
        ' procura item igual; se não achar por formatação, posiciona via índice
        Dim idx As Integer = CInt(Math.Round(v * 100.0, MidpointRounding.AwayFromZero))
        If idx < 0 Then idx = 0
        If idx > 100 Then idx = 100

        dud_alfa.SelectedItem = txt   ' tenta por texto (ok na maioria dos casos)
        If dud_alfa.SelectedItem Is Nothing Then
            ' fallback: “navega” até o índice via UpButton/DownButton
            ' DomainUpDown não expõe SelectedIndex, mas SelectedItem null aqui é raro.
            dud_alfa.Text = txt
        End If
    End Sub

    ' Pega o valor como Double (0.00 a 1.00)
    Private Function Get_dud_alfa_Value() As Double
        Dim v As Double
        If Double.TryParse(dud_alfa.Text, NumberStyles.Float, CultureInfo.CurrentCulture, v) Then
            If v < 0 Then v = 0
            If v > 1 Then v = 1
            Return Math.Round(v, 2, MidpointRounding.AwayFromZero)
        End If
        Return 0.0
    End Function

    ' Quando o usuário clicar nas setas (ou girar a rodinha / pressionar setas),
    ' o DomainUpDown avança/retrocede 1 item, o que equivale a 0,01.
    ' Este evento é chamado após a troca do item:
    Private Sub dud_alfa_SelectedItemChanged(sender As Object, e As EventArgs) Handles dud_alfa.SelectedItemChanged
        ' Se quiser reagir ao novo valor:
        Dim alfa As Double = Get_dud_alfa_Value()
        ' Ex.: atualizar algum label:
        ' lblAlpha.Text = $"α = {alfa:0.00}"

        'Me.ATUALIZAR_GRAFICO()
    End Sub

    ' Se permitir digitação manual, normaliza na saída do foco:
    Private Sub dud_alfa_Leave(sender As Object, e As EventArgs) Handles dud_alfa.Leave
        Dim v As Double = Get_dud_alfa_Value()
        Set_dud_alfa_Value(v) ' reescreve arredondado e dentro de [0,1]
    End Sub


    Private Sub Corrigir_AR1_Simples_VAZOES_GRAFICO()

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado

        Dim tMax As Integer = tamdoresultado  ' use a sua variável de tamanho temporal

        ' (opcional) truncar negativos na correção final?
        Dim truncarNaoNegativo As Boolean = False ' ajuste se quiser


        Dim m As Integer
        m = postoselecionado

        Dim ehat As Double = 0.0
        Dim temEstado As Boolean = False

        ' --- parâmetros ---
        Dim alpha As Double = dud_alfa2.Text
        Dim okA As Boolean = Double.TryParse(dud_alfa2.Text, NumberStyles.Float, CultureInfo.InvariantCulture, alpha)
        If Not okA Then okA = Double.TryParse(dud_alfa2.Text, NumberStyles.Float, CultureInfo.CurrentCulture, alpha)
        If Not okA OrElse alpha < 0 OrElse alpha > 1 Then alpha = 0.97


        For t As Integer = 1 To tMax
            Dim qobservada As Double = QOBS(m, t)
            Dim qhind As Double = QPROPHIG(m, t)

            Dim temObs As Boolean = (qobservada > 0)
            Dim temHind As Boolean = (qhind > 0)

            ' Atualiza/propaga estado do erro
            If temObs AndAlso temHind Then
                Dim r As Double = qobservada - qhind
                If Not temEstado Then
                    ' primeira observação: inicializa pelo residual atual
                    ehat = r
                    temEstado = True
                Else
                    ' filtro AR(1) do erro
                    ehat = alpha * ehat + (1.0 - alpha) * r
                End If
            Else
                ' sem observação útil: só propaga (se já tiver estado)
                If temEstado Then
                    ehat = alpha * ehat
                End If
            End If

            ' Corrige QPROPHIG (se houver modelado no passado)
            If temHind Then
                Dim qc As Double = qhind + If(temEstado, ehat, 0.0)
                If truncarNaoNegativo AndAlso qc < 0 Then qc = 0
                QPROPHIG_CORR(m, t) = qc
            End If

            ' Corrige VazaoHIG (previsão), usando o estado corrente (último erro conhecido/propagado)
            Dim qfc As Double = VazaoHIG(m, t)
            If qfc >= 0 Then
                Dim qfCorr As Double = qfc + If(temEstado, ehat, 0.0)
                If truncarNaoNegativo AndAlso qfCorr < 0 Then qfCorr = 0
                VazaoHIG_CORR(m, t) = qfCorr
            End If
        Next

    End Sub

    Private Sub Corrigir_AR1_Simples_NIVEIS_GRAFICO()

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado

        Dim tMax As Integer = tamdoresultado  ' use a sua variável de tamanho temporal

        ' (opcional) truncar negativos na correção final?
        Dim truncarNaoNegativo As Boolean = False ' ajuste se quiser


        'BIAS_NIVEL(postoselecionado) = tbVIES.Text

        Dim m As Integer
        m = postoselecionado

        Dim ehat As Double = 0.0
        Dim temEstado As Boolean = False

        ' --- parâmetros ---
        Dim alpha As Double = dud_alfa2.Text
        Dim okA As Boolean = Double.TryParse(dud_alfa2.Text, NumberStyles.Float, CultureInfo.InvariantCulture, alpha)
        If Not okA Then okA = Double.TryParse(dud_alfa2.Text, NumberStyles.Float, CultureInfo.CurrentCulture, alpha)
        If Not okA OrElse alpha < 0 OrElse alpha > 1 Then alpha = 0.97


        For t As Integer = 1 To tMax
            Dim COTAobservada As Double = NivOBS_ref(m, t)

            'CotaPROPHIG_ref(m, t) = CotaPROPHIG(m, t) - BIAS_NIVEL(m)
            'CotaHIG_ref(m, t) = CotaHIG(m, t) - BIAS_NIVEL(m)

            Dim COTAhind As Double = CotaPROPHIG_ref(m, t)
            Dim temObs As Boolean = (COTAobservada > 0)
            Dim temHind As Boolean = (COTAhind > 0)

            ' Atualiza/propaga estado do erro
            If temObs AndAlso temHind Then
                Dim r As Double = COTAobservada - COTAhind
                If Not temEstado Then
                    ' primeira observação: inicializa pelo residual atual
                    ehat = r
                    temEstado = True
                Else
                    ' filtro AR(1) do erro
                    ehat = alpha * ehat + (1.0 - alpha) * r
                End If
            Else
                ' sem observação útil: só propaga (se já tiver estado)
                If temEstado Then
                    ehat = alpha * ehat
                End If
            End If

            ' Corrige QPROPHIG (se houver modelado no passado)
            If temHind Then
                Dim qc As Double = COTAhind + If(temEstado, ehat, 0.0)
                If truncarNaoNegativo AndAlso qc < 0 Then qc = 0
                CotaPROPHIG_ref_CORR(m, t) = qc
            End If

            ' Corrige VazaoHIG (previsão), usando o estado corrente (último erro conhecido/propagado)
            Dim qfc As Double = CotaHIG_ref(m, t)
            If qfc >= 0 Then
                Dim qfCorr As Double = qfc + If(temEstado, ehat, 0.0)
                If truncarNaoNegativo AndAlso qfCorr < 0 Then qfCorr = 0
                CotaHIG_ref_CORR(m, t) = qfCorr
            End If
        Next

    End Sub

    Private Sub Corrigir_VIES_Simples_NIVEIS_GRAFICO()

        Dim contadordiamenor, numerodedias As Integer
        contadordiamenor = 0
        Dim datacomeço As Date
        'para fazer o gráfico
        datacomeço = Começoresultado
        numerodedias = tamdoresultado

        Dim tMax As Integer = tamdoresultado  ' use a sua variável de tamanho temporal

        ' (opcional) truncar negativos na correção final?
        Dim truncarNaoNegativo As Boolean = False ' ajuste se quiser


        BIAS_NIVEL(postoselecionado) = tbVIES.Text

        Dim m As Integer
        m = postoselecionado

        Dim ehat As Double = 0.0
        Dim temEstado As Boolean = False

        ' --- parâmetros ---
        Dim alpha As Double = dud_alfa.Text
        Dim okA As Boolean = Double.TryParse(dud_alfa.Text, NumberStyles.Float, CultureInfo.InvariantCulture, alpha)
        If Not okA Then okA = Double.TryParse(dud_alfa.Text, NumberStyles.Float, CultureInfo.CurrentCulture, alpha)
        If Not okA OrElse alpha < 0 OrElse alpha > 1 Then alpha = 0.97


        For t As Integer = 1 To tMax

            If CotaPROPHIG(m, t) > 0 Then
                CotaPROPHIG_ref(m, t) = CotaPROPHIG(m, t) - BIAS_NIVEL(m)

            End If
            If CotaHIG(m, t) > 0 Then
                CotaHIG_ref(m, t) = CotaHIG(m, t) - BIAS_NIVEL(m)
            End If


        Next

    End Sub

    Private Sub Button1_Click(sender As Object, e As EventArgs) Handles Button1.Click
        Me.ATUALIZAR_GRAFICO()
    End Sub

    Sub ATUALIZAR_GRAFICO()
        If ResultadoVazaopp = True Then
            Me.Corrigir_AR1_Simples_VAZOES_GRAFICO()
            CreateGraph_HidrogramaPosProcessado(zg1)
            SetSize()
        ElseIf ResultadoNívelpp = True Then
            Me.Corrigir_AR1_Simples_NIVEIS_GRAFICO()
            CreateGraph_CotagramaPosProcessado(zg1)
            SetSize()
        ElseIf ResultadoNívelref = True Then
            Me.Corrigir_VIES_Simples_NIVEIS_GRAFICO()
            CreateGraph_CotagramaReferenciado(zg1)
            SetSize()
        End If

        Me.CENTRAR_NA_PREVISAO()

    End Sub


    '-------------------------------------------------------------
    'ROTINAS DE SALVAMENTO DE HIDROGRAMAS E COTAGRAMAS
    '-------------------------------------------------------------
    Sub SalvaHidrograma()
        'Esta rotina escreve o hidrograma do local selecionado

        Dim sfd1 As New Windows.Forms.SaveFileDialog
        sfd1.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"

        If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

            FileOpen(5, sfd1.FileName, OpenMode.Output)

            'Cria uma variável e calcula o número de intervalos de tempo que ela possui
            Dim difft As Integer
            difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

            Dim contadordiamenor, numerodedias As Integer
            contadordiamenor = 0
            Dim datacomeço As Date
            'para fazer o gráfico
            datacomeço = Começoresultado
            numerodedias = tamdoresultado
            datacomeço = datacomeço.AddHours(-1)


            'Dim datacomeço As Date
            'datacomeço = dataI.AddHours(-1)

            'Dim contadordiamenor As Integer
            'contadordiamenor = 0

            PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada" & " " & "VazãoCalculada(pós-processada)" & " " & "VazãoPrevista(pós-processada)")


            For t As Integer = 1 To numerodedias 'difft
                datacomeço = datacomeço.AddHours(1)
                contadordiamenor = contadordiamenor + 1

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

                ' Preenche os valores do registro
                PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QPROPHIG_CORR(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG_CORR(postoselecionado, contadordiamenor)))

            Next
            FileClose()

        End If

    End Sub

    Sub SalvaCotagrama()
        'Esta rotina escreve o hidrograma do local selecionado

        Dim sfd1 As New Windows.Forms.SaveFileDialog
        sfd1.Filter = "ASCII com dados horários dispostos em colunas(.txt)|*.txt"

        If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

            FileOpen(5, sfd1.FileName, OpenMode.Output)

            'Cria uma variável e calcula o número de intervalos de tempo que ela possui
            Dim difft As Integer
            difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

            Dim contadordiamenor, numerodedias As Integer
            contadordiamenor = 0
            Dim datacomeço As Date
            'para fazer o gráfico
            datacomeço = Começoresultado
            numerodedias = tamdoresultado
            datacomeço = datacomeço.AddHours(-1)


            'Dim datacomeço As Date
            'datacomeço = dataI.AddHours(-1)

            'Dim contadordiamenor As Integer
            'contadordiamenor = 0

            PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "CotaCalculada(bruto)" & " " & "CotaPrevista(bruto)" & " " & "CotaObservada(bruto)" & " " & "CotaCalculada(referência)" & " " & "CotaPrevista(referência)" & " " & "CotaObservada(referência)" & " " & "CotaCalculada(referência pós-processada)" & " " & "CotaPrevista(referência pós-processada)")


            For t As Integer = 1 To numerodedias 'difft
                datacomeço = datacomeço.AddHours(1)
                contadordiamenor = contadordiamenor + 1

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

                ' Preenche os valores do registro
                'PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

                PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(CotaPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(CotaHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(NivOBS(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(CotaPROPHIG_ref(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(CotaHIG_ref(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(NivOBS_ref(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(CotaPROPHIG_ref_CORR(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(CotaHIG_ref_CORR(postoselecionado, contadordiamenor)))


            Next
            FileClose()

        End If

    End Sub

    Private Sub Button2_Click(sender As Object, e As EventArgs) Handles Button2.Click
        If ResultadoVazaopp = True Then
            Me.SalvaHidrograma()
        ElseIf ResultadoNívelpp = True Then
            Me.SalvaCotagrama()
        End If
    End Sub

    Private Sub CopiarYsDaCurva(zgc As ZedGraphControl, tituloCurva As String,
                            Optional formato As String = "0.######",
                            Optional cultura As CultureInfo = Nothing)

        If cultura Is Nothing Then cultura = CultureInfo.CurrentCulture

        Dim pane As GraphPane = zgc.GraphPane
        If pane Is Nothing OrElse pane.CurveList Is Nothing OrElse pane.CurveList.Count = 0 Then
            MessageBox.Show("Não há curvas para copiar.", "Aviso", MessageBoxButtons.OK, MessageBoxIcon.Information)
            Exit Sub
        End If

        ' Procura a curva pelo rótulo
        Dim alvo As LineItem = Nothing
        For Each ci As CurveItem In pane.CurveList
            Dim li As LineItem = TryCast(ci, LineItem)
            If li IsNot Nothing AndAlso String.Equals(li.Label.Text, tituloCurva, StringComparison.OrdinalIgnoreCase) Then
                alvo = li
                Exit For
            End If
        Next

        If alvo Is Nothing Then
            MessageBox.Show($"Curva '{tituloCurva}' não encontrada.", "Aviso", MessageBoxButtons.OK, MessageBoxIcon.Information)
            Exit Sub
        End If

        Dim sb As New StringBuilder()
        Dim nIncluidos As Integer = 0

        For i As Integer = 0 To alvo.NPts - 1
            Dim pp As PointPair = alvo.Points(i)
            Dim y As Double = pp.Y
            If Not Double.IsNaN(y) AndAlso Not Double.IsInfinity(y) AndAlso y >= 0 Then
                sb.AppendLine(y.ToString(formato, cultura))
                nIncluidos += 1
            End If
        Next

        If nIncluidos = 0 Then
            MessageBox.Show("Nenhum valor Y positivo encontrado para copiar.", "Aviso", MessageBoxButtons.OK, MessageBoxIcon.Information)
            Exit Sub
        End If

        Clipboard.SetText(sb.ToString())
        MessageBox.Show($"{nIncluidos} valores copiados para a área de transferência.", "OK", MessageBoxButtons.OK, MessageBoxIcon.Information)
    End Sub

    Private Sub Button3_Click(sender As Object, e As EventArgs) Handles Button3.Click
        If ResultadoVazaopp = True Then
            CopiarYsDaCurva(zg1, "Vazão Prevista Pós-Processada")
        ElseIf ResultadoNívelpp = True Then
            CopiarYsDaCurva(zg1, "Nível previsto pós-processado")
        ElseIf ResultadoNívelref = True Then
            CopiarYsDaCurva(zg1, "Nível previsto referenciado")
        ElseIf Resultado = True Then
            CopiarYsDaCurva(zg1, "Vazão Prevista")
        ElseIf ResultadoNível = True Then
            CopiarYsDaCurva(zg1, "Nível Previsto")
        End If
    End Sub

    Private Sub CopiarXYsDaCurva(zgc As ZedGraphControl, tituloCurva As String,
                            Optional formatoY As String = "0.######",
                            Optional cultura As CultureInfo = Nothing,
                            Optional separador As String = vbTab)

        If cultura Is Nothing Then cultura = CultureInfo.CurrentCulture

        Dim pane As GraphPane = zgc.GraphPane
        If pane Is Nothing OrElse pane.CurveList Is Nothing OrElse pane.CurveList.Count = 0 Then
            MessageBox.Show("Não há curvas para copiar.", "Aviso", MessageBoxButtons.OK, MessageBoxIcon.Information)
            Exit Sub
        End If

        ' Procura a curva pelo rótulo
        Dim alvo As LineItem = Nothing
        For Each ci As CurveItem In pane.CurveList
            Dim li As LineItem = TryCast(ci, LineItem)
            If li IsNot Nothing AndAlso String.Equals(li.Label.Text, tituloCurva, StringComparison.OrdinalIgnoreCase) Then
                alvo = li
                Exit For
            End If
        Next

        If alvo Is Nothing Then
            MessageBox.Show(String.Format("Curva '{0}' não encontrada.", tituloCurva),
                        "Aviso", MessageBoxButtons.OK, MessageBoxIcon.Information)
            Exit Sub
        End If

        Dim sb As New StringBuilder()
        Dim nIncluidos As Integer = 0

        For i As Integer = 0 To alvo.NPts - 1
            Dim pp As PointPair = alvo.Points(i)

            Dim x As Double = pp.X
            Dim y As Double = pp.Y

            If Double.IsNaN(y) OrElse Double.IsInfinity(y) OrElse y < 0 Then
                Continue For
            End If

            ' Converte X (double) para DateTime via XDate
            Dim dt As DateTime
            Try
                dt = New XDate(x).DateTime
            Catch
                ' Se não for data válida, ignora o ponto
                Continue For
            End Try

            Dim xStr As String = dt.ToString("dd/MM/yyyy HH':00:00'", cultura)
            Dim yStr As String = y.ToString(formatoY, cultura)

            sb.Append(xStr)
            sb.Append(separador)
            sb.AppendLine(yStr)

            nIncluidos += 1
        Next

        If nIncluidos = 0 Then
            MessageBox.Show("Nenhum par X/Y positivo encontrado para copiar.", "Aviso",
                        MessageBoxButtons.OK, MessageBoxIcon.Information)
            Exit Sub
        End If

        Clipboard.SetText(sb.ToString())
        MessageBox.Show(String.Format("{0} linhas (X e Y) copiadas para a área de transferência.", nIncluidos),
                    "OK", MessageBoxButtons.OK, MessageBoxIcon.Information)
    End Sub

    Private Sub CreateGraph2(ByVal zgc As ZedGraphControl)
        Dim p As Integer


        If Resultado = True Then
            Dim myPane As GraphPane = zgc.GraphPane

            ' Set the titles and axis labels
            myPane.Title.Text = "Hidrograma (" & Cod_graf(postoselecionado) & " - " & Nome_graf(postoselecionado) & ")"
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
                .Symbol.Size = 2.0
                .Symbol.Fill = New Fill(Color.Black)      ' cor de preenchimento do símbolo
                .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
            End With

            ' Guarde a referência para uso nas bandas:
            _curveBase = myCurve

            ' Como você limpa/repinta às vezes, também zere as bandas antigas:
            _curveBandSup = Nothing
            _curveBandInf = Nothing

            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()


        ElseIf ResultadoNível = True Then
            Dim myPane As GraphPane = zgc.GraphPane

            ' Set the titles and axis labels
            myPane.Title.Text = "Cotagrama (" & Cod_graf(postoselecionado) & " - " & Nome_graf(postoselecionado) & ")"
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
                .Symbol.Size = 2.0
                .Symbol.Fill = New Fill(Color.Black)      ' cor de preenchimento do símbolo
                .Symbol.Border.IsVisible = False        ' opcional: sem contorno do símbolo
            End With

            ' Guarde a referência para uso nas bandas:
            _curveBase = myCurve

            ' Como você limpa/repinta às vezes, também zere as bandas antigas:
            _curveBandSup = Nothing
            _curveBandInf = Nothing

            ' Fill the axis background with a color gradient
            myPane.Chart.Fill = New Fill(Color.White)

            ' Fill the pane background with a color gradient
            myPane.Fill = New Fill(Color.White)

            ' Calculate the Axis Scale Ranges
            zgc.AxisChange()

        End If



    End Sub

    Private Sub Button4_Click(sender As Object, e As EventArgs) Handles Button4.Click
        Me.Close()

    End Sub

    Private Sub Button5_Click(sender As Object, e As EventArgs) Handles btnZoomUltimos21d.Click
        Me.CENTRAR_NA_PREVISAO()
    End Sub

    Sub CENTRAR_NA_PREVISAO()

        Dim pane = zg1.GraphPane

        ' fim da série (dados horários)
        Dim dtFim As Date = Começoresultado.AddHours(tamdoresultado - 1)

        ' janela alvo no X
        Dim xMin As Double = New XDate(dtFim.AddDays(-28)) ' 28 dias para trás
        Dim xMax As Double = New XDate(dtFim.AddDays(+1))  ' 1 dia depois (sobra margem)

        ' desligar autoscale do X e aplicar janela
        pane.XAxis.Scale.MinAuto = False
        pane.XAxis.Scale.MaxAuto = False
        pane.XAxis.Scale.Min = xMin
        pane.XAxis.Scale.Max = xMax

        ' Y em auto (ajusta ao que estiver visível)
        pane.YAxis.Scale.MinAuto = True
        pane.YAxis.Scale.MaxAuto = True

        ' recalcula e repinta
        zg1.AxisChange()
        zg1.Invalidate()
    End Sub

    Private Sub Button7_Click_1(sender As Object, e As EventArgs) Handles Button5.Click
        Dim pane = zg1.GraphPane
        pane.XAxis.Scale.MinAuto = True
        pane.XAxis.Scale.MaxAuto = True
        pane.YAxis.Scale.MinAuto = True
        pane.YAxis.Scale.MaxAuto = True
        zg1.AxisChange()
        zg1.Invalidate()
    End Sub

    Private Function FindCurve(pane As GraphPane, label As String) As LineItem
        For Each c As CurveItem In pane.CurveList
            If String.Equals(c.Label.Text, label, StringComparison.OrdinalIgnoreCase) Then
                Return TryCast(c, LineItem)
            End If
        Next
        Return Nothing
    End Function

    Private Sub ToggleBandasIncerteza(zgc As ZedGraphControl, baseCurve As LineItem, ligado As Boolean, Optional frac As Double = 0.05)
        If baseCurve Is Nothing Then Exit Sub

        Dim pane As GraphPane = zgc.GraphPane

        If ligado Then
            ' já existem? não recria
            If _curveBandSup IsNot Nothing OrElse _curveBandInf IsNot Nothing Then
                Exit Sub
            End If

            If baseCurve.Points Is Nothing OrElse baseCurve.Points.Count = 0 Then Exit Sub

            Dim src As IPointList = baseCurve.Points
            Dim sup As New PointPairList()
            Dim inf As New PointPairList()

            For i As Integer = 0 To src.Count - 1
                Dim xx As Double = src(i).X
                Dim yy As Double = src(i).Y
                If Double.IsNaN(yy) Then
                    sup.Add(xx, Double.NaN)
                    inf.Add(xx, Double.NaN)
                Else
                    sup.Add(xx, yy * (1.0 + frac))
                    inf.Add(xx, yy * (1.0 - frac))
                End If
            Next

            _curveBandSup = pane.AddCurve(Nothing, sup, Color.Red, SymbolType.None)
            _curveBandSup.Line.Style = DashStyle.Dash
            _curveBandSup.Line.Width = 1.5F

            _curveBandInf = pane.AddCurve(Nothing, inf, Color.Red, SymbolType.None)
            _curveBandInf.Line.Style = DashStyle.Dash
            _curveBandInf.Line.Width = 1.5F

            ' opcional: mandar as bandas pro “fundo” do desenho
            pane.CurveList.Move(pane.CurveList.IndexOf(_curveBandSup), 0)
            pane.CurveList.Move(pane.CurveList.IndexOf(_curveBandInf), 0)

        Else
            If _curveBandSup IsNot Nothing Then
                pane.CurveList.Remove(_curveBandSup)
                _curveBandSup = Nothing
            End If
            If _curveBandInf IsNot Nothing Then
                pane.CurveList.Remove(_curveBandInf)
                _curveBandInf = Nothing
            End If
        End If

        zgc.AxisChange()
        zgc.Invalidate()
    End Sub


    Private Sub chkBandas_CheckedChanged(sender As Object, e As EventArgs) Handles chkBandas.CheckedChanged
        ' 5% de banda (ajuste se quiser outra porcentagem)
        Dim Porcentagem As Single
        Porcentagem = TextBox1.Text / 100

        ToggleBandasIncerteza(zg1, _curveBase, chkBandas.Checked, Porcentagem)
    End Sub

    Private Sub btPLUS_Click(sender As Object, e As EventArgs) Handles btPLUS.Click
        tbVIES.Text = tbVIES.Text + 0.1
        Me.ATUALIZAR_GRAFICO()
    End Sub

    Private Sub btMINUS_Click(sender As Object, e As EventArgs) Handles btMINUS.Click
        tbVIES.Text = tbVIES.Text - 0.1
        Me.ATUALIZAR_GRAFICO()
    End Sub

    Private Sub bt_dud_plus_Click(sender As Object, e As EventArgs) Handles bt_dud_plus.Click
        dud_alfa2.Text = dud_alfa2.Text + 0.01
        Me.ATUALIZAR_GRAFICO()
    End Sub

    Private Sub bt_dud_minus_Click(sender As Object, e As EventArgs) Handles bt_dud_minus.Click
        dud_alfa2.Text = dud_alfa2.Text - 0.01
        Me.ATUALIZAR_GRAFICO()
    End Sub

    Private Sub Button6_Click(sender As Object, e As EventArgs) Handles Button6.Click
        If ResultadoVazaopp = True Then
            CopiarXYsDaCurva(zg1, "Vazão Prevista Pós-Processada")
        ElseIf ResultadoNívelpp = True Then
            CopiarXYsDaCurva(zg1, "Nível previsto pós-processado")
        ElseIf ResultadoNívelref = True Then
            CopiarXYsDaCurva(zg1, "Nível previsto referenciado")
        ElseIf Resultado = True Then
            CopiarXYsDaCurva(zg1, "Vazão Prevista")
        ElseIf ResultadoNível = True Then
            CopiarXYsDaCurva(zg1, "Nível Previsto")
        End If
    End Sub
End Class