Imports System.Data
Imports System.Drawing
Imports System.IO
Imports System.Windows.Forms
Imports GeoAPI.Geometries
Imports MapWindow.Interfaces
Imports SharpMap
Imports SharpMap.Data
Imports SharpMap.Data.Providers
Imports SharpMap.Forms
Imports SharpMap.Layers

Public Class Form1
    Private mapBox As MapBox
    Private baseMap As Map

    ' Status bar
    Private status As StatusStrip
    Private lblCoord As ToolStripStatusLabel

    ' Estado
    Private identifyMode As Boolean = False

    ' Itens de menu (guardados para podermos marcar/desmarcar)
    Private miPan As ToolStripMenuItem
    Private miZoom As ToolStripMenuItem
    Private miZoomIn As ToolStripMenuItem
    Private miZoomOut As ToolStripMenuItem
    Private miZoomWindow As ToolStripMenuItem
    Private miExtensao As ToolStripMenuItem
    Private miIdentificar As ToolStripMenuItem
    Private miAbrir As ToolStripMenuItem
    Private miLimpar As ToolStripMenuItem
    Private miSair As ToolStripMenuItem

    ' Menu "Camadas"
    Private menuCamadas As ToolStripMenuItem
    Private btnZoomFull As Button
    Private btnPanTool As Button
    Private btnZoomWindow As Button


    ' ==== Painel de Camadas (TOC) ====
    Private pnlLayers As Panel
    Private lblLayers As Label
    Private lvLayers As ListView
    Private imgLayers As ImageList
    Private btnUp As Button
    Private btnDown As Button
    Private btnZoomLayer As Button
    Private btnRemoveLayer As Button
    Private ctxLayers As ContextMenuStrip
    Private ctxLayerZoom As ToolStripMenuItem
    Private ctxLayerRemove As ToolStripMenuItem

    Private pnlBtnsLayers As FlowLayoutPanel

    ' Modo de pick para Hidrogramas (pega só a coluna "Mini")
    Private miniPickMode As Boolean = False

    Dim DirPrevAuto(3), Dir_EXPORTA, ModeloPrev As String



    Private Sub Form1_Load(sender As Object, e As EventArgs) Handles MyBase.Load
        ' === MapBox ===
        mapBox = New MapBox() With {
            .Dock = DockStyle.Fill,
            .ActiveTool = MapBox.Tools.Pan,
            .WheelZoomMagnitude = 1.2
        }
        AddHandler mapBox.MouseClick, AddressOf MapBox_MouseClick
        AddHandler mapBox.MouseMove, AddressOf MapBox_MouseMove

        ' Garante ordem de z: MenuStrip em cima, MapBox preenchendo abaixo
        If MenuStrip1 IsNot Nothing Then
            Me.Controls.Add(mapBox)
            mapBox.BringToFront()
            MenuStrip1.BringToFront()
        Else
            Me.Controls.Add(mapBox)
        End If

        ' === Map ===
        baseMap = New Map(New Size(CInt(Me.ClientSize.Width), CInt(Me.ClientSize.Height)))





        mapBox.Map = baseMap

        ' === Status bar ===
        status = New StatusStrip() With {.SizingGrip = False}
        lblCoord = New ToolStripStatusLabel("X: —    Y: —")
        status.Items.Add(lblCoord)
        Me.Controls.Add(status)
        status.BringToFront()

        ' === Menus ===
        BuildMenu()

        BuildLayersPanel()
        RefreshLayersPanel()
        ' ... no final do Form1_Load:
        AutoLoadStartupLayers()

        LOAD_AI()

        MsgBox("          Olá! Este é o sistema de Previsão do modelo MGB-RSH." & vbCrLf & "          Este programa funciona com configurações de data estilo brasileira dd/MM/aaaa hh:mm:ss." & vbCrLf & "          Este programa funciona com separador decimal ponto *.* e de milhar vírgula *,*.", MsgBoxStyle.Information, Title:="Informação")


    End Sub

    Private Sub BuildLayersPanel()
        ' Painel lateral
        pnlLayers = New Panel() With {
        .BorderStyle = BorderStyle.FixedSingle,
        .Padding = New Padding(6)
    }
        Me.Controls.Add(pnlLayers)

        ' Título (sem Dock, vamos posicionar manualmente)
        lblLayers = New Label() With {
        .Text = "Camadas",
        .AutoSize = False,
        .Height = 24,
        .TextAlign = ContentAlignment.MiddleLeft
    }
        pnlLayers.Controls.Add(lblLayers)

        ' Lista (sem Dock; vamos setar Location/Size manualmente)
        imgLayers = New ImageList() With {.ImageSize = New Size(16, 16), .ColorDepth = ColorDepth.Depth32Bit}
        lvLayers = New ListView() With {
        .View = Windows.Forms.View.List,
        .CheckBoxes = True,
        .FullRowSelect = True,
        .MultiSelect = False,
        .HideSelection = False,
        .SmallImageList = imgLayers
    }
        AddHandler lvLayers.ItemChecked, AddressOf LvLayers_ItemChecked
        AddHandler lvLayers.SelectedIndexChanged, AddressOf LvLayers_SelectedIndexChanged
        AddHandler lvLayers.MouseDoubleClick, AddressOf LvLayers_MouseDoubleClick
        pnlLayers.Controls.Add(lvLayers)

        ' Botões no rodapé (altura fixa)
        pnlBtnsLayers = New FlowLayoutPanel() With {
        .Height = 56,
        .FlowDirection = FlowDirection.LeftToRight
    }
        pnlLayers.Controls.Add(pnlBtnsLayers)

        btnUp = New Button() With {.Text = "↑", .Width = 20}
        btnDown = New Button() With {.Text = "↓", .Width = 20}
        btnZoomLayer = New Button() With {.Text = "Zoom Camada", .Width = 100}
        btnRemoveLayer = New Button() With {.Text = "Remover", .Width = 60}
        btnZoomFull = New Button() With {.Text = "Extensão", .Width = 90}
        btnPanTool = New Button() With {.Text = "Pan", .Width = 50}
        btnZoomWindow = New Button() With {.Text = "Janela", .Width = 60}


        AddHandler btnUp.Click, AddressOf BtnUp_Click
        AddHandler btnDown.Click, AddressOf BtnDown_Click
        AddHandler btnZoomLayer.Click, AddressOf BtnZoomLayer_Click
        AddHandler btnRemoveLayer.Click, AddressOf BtnRemoveLayer_Click
        AddHandler btnZoomFull.Click, AddressOf BtnZoomFullExtent_Click
        AddHandler btnPanTool.Click, AddressOf BtnPanTool_Click
        AddHandler btnZoomWindow.Click, AddressOf BtnZoomWindow_Click

        ' Ordem dos botões na barra inferior (ajuste se quiser)

        pnlBtnsLayers.Controls.AddRange(New Control() {
    btnUp, btnDown, btnRemoveLayer, btnZoomLayer, btnZoomWindow, btnZoomFull, btnPanTool
})

        ' Context menu
        ctxLayers = New ContextMenuStrip()
        ctxLayerZoom = New ToolStripMenuItem("Zoom à camada", Nothing, AddressOf BtnZoomLayer_Click)
        ctxLayerRemove = New ToolStripMenuItem("Remover", Nothing, AddressOf BtnRemoveLayer_Click)
        ctxLayers.Items.AddRange(New ToolStripItem() {ctxLayerZoom, ctxLayerRemove})
        lvLayers.ContextMenuStrip = ctxLayers

        ' Posiciona painel e controles
        PositionLayersPanel()

        ' MapBox ocupa o restante
        mapBox.Dock = DockStyle.Fill
        mapBox.SendToBack()
        If MenuStrip1 IsNot Nothing Then MenuStrip1.BringToFront()
        If status IsNot Nothing Then status.BringToFront()
    End Sub

    Private Sub BtnZoomWindow_Click(sender As Object, e As EventArgs)
        ' Desliga modos especiais e ativa o retângulo de zoom
        identifyMode = False
        miniPickMode = False
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.ZoomWindow

        If lblCoord IsNot Nothing Then
            lblCoord.Text = "Ferramenta: Zoom Janela (clique e arraste no mapa)"
        End If

        ' (se você usa marcação de menu ativo, pode atualizar aqui)
        ' UpdateCheckedTools(pan:=False, zoomIn:=False, zoomOut:=False, zoomWin:=True, identify:=False)
    End Sub


    Private Sub BtnZoomFullExtent_Click(sender As Object, e As EventArgs)
        If baseMap Is Nothing Then Return
        If baseMap.Layers IsNot Nothing AndAlso baseMap.Layers.Count > 0 Then
            baseMap.ZoomToExtents()
            mapBox.Refresh()
        End If
    End Sub

    Private Sub BtnPanTool_Click(sender As Object, e As EventArgs)
        ' Sai de quaisquer modos especiais e ativa Pan
        identifyMode = False
        miniPickMode = False
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.Pan

        ' (opcional) feedback na status bar
        If lblCoord IsNot Nothing Then
            lblCoord.Text = "Ferramenta: Pan"
        End If

        ' (opcional) se você usa marcação de menu ativo, atualize aqui:
        ' UpdateCheckedTools(pan:=True, zoomIn:=False, zoomOut:=False, zoomWin:=False, identify:=False)
    End Sub


    Private Sub PositionLayersPanel()
        ' Margens e tamanhos
        Dim topGap As Integer = If(MenuStrip1 IsNot Nothing, MenuStrip1.Bottom, 0) + 6
        Dim bottomGap As Integer = Me.ClientSize.Height - If(status IsNot Nothing, status.Height, 0) - 6
        Dim left As Integer = 0
        Dim width As Integer = 240

        ' Define bounds do painel lateral
        pnlLayers.SetBounds(left, topGap, width, Math.Max(140, bottomGap - topGap))

        ' Bounds internos (descontando Padding)
        Dim innerLeft As Integer = pnlLayers.Padding.Left
        Dim innerTop As Integer = pnlLayers.Padding.Top
        Dim innerWidth As Integer = pnlLayers.ClientSize.Width - pnlLayers.Padding.Horizontal
        Dim innerHeight As Integer = pnlLayers.ClientSize.Height - pnlLayers.Padding.Vertical

        ' Label no topo
        lblLayers.SetBounds(innerLeft, innerTop, innerWidth, 24)

        ' Botões no rodapé
        Dim btnsH As Integer = pnlBtnsLayers.Height
        pnlBtnsLayers.SetBounds(innerLeft, pnlLayers.ClientSize.Height - pnlLayers.Padding.Bottom - btnsH, innerWidth, btnsH)

        ' ListView ocupa o miolo, com um respiro abaixo do label
        Dim spacer As Integer = 6
        Dim listTop As Integer = lblLayers.Bottom + spacer
        Dim listBottom As Integer = pnlBtnsLayers.Top - spacer
        Dim listHeight As Integer = Math.Max(40, listBottom - listTop)

        lvLayers.SetBounds(innerLeft, listTop, innerWidth, listHeight)

        ' Âncoras para redimensionar bem
        pnlLayers.Anchor = AnchorStyles.Top Or AnchorStyles.Bottom Or AnchorStyles.Left
        lvLayers.Anchor = AnchorStyles.Top Or AnchorStyles.Bottom Or AnchorStyles.Left Or AnchorStyles.Right
        lblLayers.Anchor = AnchorStyles.Top Or AnchorStyles.Left Or AnchorStyles.Right
        pnlBtnsLayers.Anchor = AnchorStyles.Left Or AnchorStyles.Right Or AnchorStyles.Bottom
    End Sub


    Private Sub Form1_Resize(sender As Object, e As EventArgs) Handles MyBase.Resize
        If pnlLayers Is Nothing Then Return
        PositionLayersPanel()
    End Sub



    Private Sub RefreshLayersPanel()
        If lvLayers Is Nothing Then Return

        lvLayers.BeginUpdate()
        imgLayers.Images.Clear()
        lvLayers.Items.Clear()

        If baseMap Is Nothing OrElse baseMap.Layers Is Nothing OrElse baseMap.Layers.Count = 0 Then
            lvLayers.EndUpdate()
            Return
        End If

        ' Mostra do topo para a base (como nos GIS)
        For i As Integer = baseMap.Layers.Count - 1 To 0 Step -1
            Dim lyr = baseMap.Layers(i)
            Dim key As String = $"k{i}_{lyr.LayerName}_{Guid.NewGuid()}"
            imgLayers.Images.Add(key, CreateLayerSwatch(lyr))

            Dim item As New ListViewItem(lyr.LayerName) With {
            .Checked = lyr.Enabled,
            .ImageKey = key,
            .Tag = lyr
        }
            lvLayers.Items.Add(item)
        Next

        lvLayers.EndUpdate()
    End Sub

    Private Function UIIndexToLayerIndex(uiIndex As Integer) As Integer
        ' uiIndex 0 (topo visual) = layerIndex Count-1
        Return baseMap.Layers.Count - 1 - uiIndex
    End Function

    Private Function LayerIndexToUIIndex(layerIndex As Integer) As Integer
        Return baseMap.Layers.Count - 1 - layerIndex
    End Function

    Private Sub LvLayers_ItemChecked(sender As Object, e As ItemCheckedEventArgs)
        Dim lyr = TryCast(e.Item.Tag, SharpMap.Layers.ILayer)
        If lyr Is Nothing Then Return
        lyr.Enabled = e.Item.Checked
        mapBox.Refresh()
        RebuildLayersMenu()   ' mantém o menu SIG ▸ Camadas em sincronia (se usar)
    End Sub

    Private Sub LvLayers_SelectedIndexChanged(sender As Object, e As EventArgs)
        ' Apenas habilita/desabilita botões conforme seleção
        Dim hasSel = lvLayers.SelectedItems.Count = 1
        btnUp.Enabled = hasSel
        btnDown.Enabled = hasSel
        btnZoomLayer.Enabled = hasSel
        btnRemoveLayer.Enabled = hasSel
    End Sub

    Private Sub LvLayers_MouseDoubleClick(sender As Object, e As MouseEventArgs)
        If lvLayers.SelectedItems.Count <> 1 Then Return
        BtnZoomLayer_Click(Nothing, EventArgs.Empty)
    End Sub

    Private Sub BtnUp_Click(sender As Object, e As EventArgs)
        If lvLayers.SelectedItems.Count <> 1 Then Return
        Dim ui = lvLayers.SelectedIndices(0)
        If ui = 0 Then Return ' já está no topo visual

        ' Move visualmente para cima
        Dim item = lvLayers.Items(ui)
        lvLayers.Items.RemoveAt(ui)
        lvLayers.Items.Insert(ui - 1, item)
        lvLayers.SelectedIndices.Clear()
        item.Selected = True

        ' Reordena na coleção (converter índices)
        Dim fromLayer = UIIndexToLayerIndex(ui)
        Dim toLayer = UIIndexToLayerIndex(ui - 1)
        MoveLayer(fromLayer, toLayer)
    End Sub

    Private Sub BtnDown_Click(sender As Object, e As EventArgs)
        If lvLayers.SelectedItems.Count <> 1 Then Return
        Dim ui = lvLayers.SelectedIndices(0)
        If ui = lvLayers.Items.Count - 1 Then Return ' já está na base visual

        Dim item = lvLayers.Items(ui)
        lvLayers.Items.RemoveAt(ui)
        lvLayers.Items.Insert(ui + 1, item)
        lvLayers.SelectedIndices.Clear()
        item.Selected = True

        Dim fromLayer = UIIndexToLayerIndex(ui)
        Dim toLayer = UIIndexToLayerIndex(ui + 1)
        MoveLayer(fromLayer, toLayer)
    End Sub

    Private Sub MoveLayer(fromIndex As Integer, toIndex As Integer)
        If fromIndex = toIndex Then Return
        If fromIndex < 0 OrElse toIndex < 0 Then Return
        If fromIndex >= baseMap.Layers.Count OrElse toIndex >= baseMap.Layers.Count Then Return

        Dim lyr = baseMap.Layers(fromIndex)
        baseMap.Layers.RemoveAt(fromIndex)
        ' Ajuste porque remover desloca índices
        If toIndex > fromIndex Then toIndex -= 1
        baseMap.Layers.Insert(toIndex, lyr)

        mapBox.Refresh()
        RebuildLayersMenu()
    End Sub

    Private Sub BtnZoomLayer_Click(sender As Object, e As EventArgs)
        If lvLayers.SelectedItems.Count <> 1 Then Return
        Dim lyr = TryCast(lvLayers.SelectedItems(0).Tag, SharpMap.Layers.ILayer)
        If lyr Is Nothing Then Return
        Try
            Dim env = lyr.Envelope
            If env IsNot Nothing Then
                baseMap.ZoomToBox(env)
                mapBox.Refresh()
            End If
        Catch
        End Try
    End Sub

    Private Sub BtnRemoveLayer_Click(sender As Object, e As EventArgs)
        If lvLayers.SelectedItems.Count <> 1 Then Return
        Dim lyr = TryCast(lvLayers.SelectedItems(0).Tag, SharpMap.Layers.ILayer)
        If lyr Is Nothing Then Return

        baseMap.Layers.Remove(lyr)
        mapBox.Refresh()
        RefreshLayersPanel()
        RebuildLayersMenu()
    End Sub


    '------------------------------------------------------------
    ' MENU
    '------------------------------------------------------------
    Private Sub BuildMenu()
        ' Garante MenuStrip
        If MenuStrip1 Is Nothing Then
            MenuStrip1 = New MenuStrip()
            Me.Controls.Add(MenuStrip1)
            MenuStrip1.BringToFront()
        End If

        ' Cria/pega "SIG"
        Dim menuSIG As ToolStripMenuItem = Nothing
        For Each it As ToolStripItem In MenuStrip1.Items
            Dim m = TryCast(it, ToolStripMenuItem)
            If m IsNot Nothing AndAlso String.Equals(m.Text, "SIG", StringComparison.OrdinalIgnoreCase) Then
                menuSIG = m : Exit For
            End If
        Next
        If menuSIG Is Nothing Then
            menuSIG = New ToolStripMenuItem("SIG")
            MenuStrip1.Items.Add(menuSIG)
        End If

        ' --- garantir que "SIG" seja o 2º item do MenuStrip ---
        Dim desiredIndex As Integer = 1 ' 0 = primeiro, 1 = segundo
        Dim currentIndex As Integer = MenuStrip1.Items.IndexOf(menuSIG)

        If currentIndex = -1 Then
            ' ainda não está no MenuStrip: insere direto na posição desejada
            If desiredIndex > MenuStrip1.Items.Count Then desiredIndex = MenuStrip1.Items.Count
            MenuStrip1.Items.Insert(desiredIndex, menuSIG)
        ElseIf currentIndex <> desiredIndex Then
            ' já existe mas está na posição errada: remove e reinsere
            MenuStrip1.Items.RemoveAt(currentIndex)
            If desiredIndex > MenuStrip1.Items.Count Then desiredIndex = MenuStrip1.Items.Count
            MenuStrip1.Items.Insert(desiredIndex, menuSIG)
        End If

        MenuStrip1.PerformLayout()


        ' Reconstrói do zero os itens controlados por este código
        menuSIG.DropDownItems.Clear()

        ' ===== Submenus dentro de "SIG" =====
        Dim menuArquivo As New ToolStripMenuItem("Arquivo")
        Dim menuNavegacao As New ToolStripMenuItem("Navegação")
        Dim menuConsulta As New ToolStripMenuItem("Consulta")
        menuCamadas = New ToolStripMenuItem("Camadas")

        ' ---- Arquivo ----
        miAbrir = New ToolStripMenuItem("Abrir Shapefile", Nothing, AddressOf BtnOpen_Click)
        miLimpar = New ToolStripMenuItem("Limpar Camadas", Nothing, AddressOf BtnClear_Click)
        miSair = New ToolStripMenuItem("Sair", Nothing, Sub() Me.Close())
        menuArquivo.DropDownItems.AddRange(New ToolStripItem() {miAbrir, New ToolStripSeparator(), miLimpar, New ToolStripSeparator(), miSair})

        ' ---- Navegação ----
        miPan = New ToolStripMenuItem("Pan", Nothing, Sub() SetTool(MapBox.Tools.Pan))

        ' Submenu Zoom
        miZoom = New ToolStripMenuItem("Zoom")
        miZoomIn = New ToolStripMenuItem("Zoom +", Nothing, Sub() SetTool(MapBox.Tools.ZoomIn))
        miZoomOut = New ToolStripMenuItem("Zoom −", Nothing, Sub() SetTool(MapBox.Tools.ZoomOut))
        miZoomWindow = New ToolStripMenuItem("Zoom Janela", Nothing, Sub() SetTool(MapBox.Tools.ZoomWindow))
        miExtensao = New ToolStripMenuItem("Extensão", Nothing, AddressOf BtnZoomFull_Click)
        miZoom.DropDownItems.AddRange(New ToolStripItem() {miZoomIn, miZoomOut, miZoomWindow, New ToolStripSeparator(), miExtensao})
        menuNavegacao.DropDownItems.AddRange(New ToolStripItem() {miPan, miZoom})

        ' ---- Consulta ----
        miIdentificar = New ToolStripMenuItem("Identificar", Nothing, AddressOf BtnIdentify_Click)
        menuConsulta.DropDownItems.Add(miIdentificar)

        ' Monta dentro do "SIG"
        menuSIG.DropDownItems.Add(menuArquivo)
        menuSIG.DropDownItems.Add(menuNavegacao)
        menuSIG.DropDownItems.Add(menuConsulta)
        menuSIG.DropDownItems.Add(New ToolStripSeparator())
        menuSIG.DropDownItems.Add(menuCamadas)

        ' Começa em Pan
        UpdateCheckedTools(pan:=True, zoomIn:=False, zoomOut:=False, zoomWin:=False, identify:=False)

        ' Constrói a lista de camadas
        RebuildLayersMenu()
    End Sub

    Private Sub RebuildLayersMenu()
        If menuCamadas Is Nothing Then Return
        menuCamadas.DropDownItems.Clear()

        If baseMap Is Nothing OrElse baseMap.Layers Is Nothing OrElse baseMap.Layers.Count = 0 Then
            Dim vazio As New ToolStripMenuItem("(nenhuma camada)") With {.Enabled = False}
            menuCamadas.DropDownItems.Add(vazio)
            Return
        End If

        ' Lista da camada do topo para a base (mais útil no dia a dia)
        For i As Integer = baseMap.Layers.Count - 1 To 0 Step -1
            Dim lyr = baseMap.Layers(i)
            Dim item As New ToolStripMenuItem(lyr.LayerName) With {
            .Checked = lyr.Enabled,           ' usa Enabled para "visível"
            .CheckOnClick = True,
            .Tag = lyr
        }
            AddHandler item.CheckedChanged, AddressOf LayerVisibility_Toggled


            ' ===== Submenu Estilo (dinâmico por tipo de geometria) =====
            Dim estiloMenu As New ToolStripMenuItem("Estilo")

            ' NullShape = 0 (valor padrão seguro na 1.1.0)
            Dim shpType As ShapeType = ShapeType.Null
            Dim vlyr = TryCast(lyr, VectorLayer)
            If vlyr IsNot Nothing Then
                Dim prov = TryCast(vlyr.DataSource, ShapeFile)
                If prov IsNot Nothing Then shpType = prov.ShapeType
            End If


            Select Case shpType
                Case ShapeType.Polygon, ShapeType.PolygonM, ShapeType.PolygonZ
                    Dim miFill As New ToolStripMenuItem("Cor de preenchimento", Nothing, AddressOf ChangePolygonFill_Click) With {.Tag = vlyr}
                    Dim miOutline As New ToolStripMenuItem("Cor de contorno", Nothing, AddressOf ChangePolygonOutline_Click) With {.Tag = vlyr}
                    estiloMenu.DropDownItems.AddRange(New ToolStripItem() {miFill, miOutline})

                Case ShapeType.PolyLine, ShapeType.PolyLineM, ShapeType.PolyLineZ
                    Dim miLineColor As New ToolStripMenuItem("Cor da linha", Nothing, AddressOf ChangeLineColor_Click) With {.Tag = vlyr}
                    Dim miLineWidth As New ToolStripMenuItem("Espessura da linha…", Nothing, AddressOf ChangeLineWidth_Click) With {.Tag = vlyr}
                    estiloMenu.DropDownItems.AddRange(New ToolStripItem() {miLineColor, miLineWidth})

                Case ShapeType.Point, ShapeType.PointM, ShapeType.PointZ, ShapeType.Multipoint
                    Dim miPtColor As New ToolStripMenuItem("Cor do ponto", Nothing, AddressOf ChangePointColor_Click) With {.Tag = vlyr}
                    Dim miPtSize As New ToolStripMenuItem("Tamanho do ponto…", Nothing, AddressOf ChangePointSize_Click) With {.Tag = vlyr}
                    Dim miPtOutline As New ToolStripMenuItem("Cor do contorno", Nothing, AddressOf ChangePointOutline_Click) With {.Tag = vlyr}
                    estiloMenu.DropDownItems.AddRange(New ToolStripItem() {miPtColor, miPtSize, miPtOutline})

                Case ShapeType.Null
                    ' sem geometrias: menu genérico
                    Dim miGeneric As New ToolStripMenuItem("(sem opções específicas)") With {.Enabled = False}
                    estiloMenu.DropDownItems.Add(miGeneric)

                Case Else
                    ' Caso desconhecido: opções genéricas
                    Dim miGeneric As New ToolStripMenuItem("(sem opções específicas)") With {.Enabled = False}
                    estiloMenu.DropDownItems.Add(miGeneric)
            End Select

            ' Subopções por camada (zoom/remover)
            Dim miZoomCamada As New ToolStripMenuItem("Zoom à camada", Nothing, AddressOf ZoomToLayer_Click) With {.Tag = lyr}
            Dim miRemover As New ToolStripMenuItem("Remover", Nothing, AddressOf RemoveLayer_Click) With {.Tag = lyr}

            item.DropDownItems.Add(estiloMenu)
            item.DropDownItems.Add(New ToolStripSeparator())
            item.DropDownItems.Add(miZoomCamada)
            item.DropDownItems.Add(New ToolStripSeparator())
            item.DropDownItems.Add(miRemover)







            menuCamadas.DropDownItems.Add(item)
        Next
    End Sub

    Private Sub LayerVisibility_Toggled(sender As Object, e As EventArgs)
        Dim it = TryCast(sender, ToolStripMenuItem)
        If it Is Nothing Then Return
        Dim lyr = TryCast(it.Tag, SharpMap.Layers.ILayer)
        If lyr Is Nothing Then Return
        lyr.Enabled = it.Checked
        mapBox.Refresh()
    End Sub

    Private Sub ZoomToLayer_Click(sender As Object, e As EventArgs)
        Dim it = TryCast(sender, ToolStripMenuItem)
        If it Is Nothing Then Return
        Dim lyr = TryCast(it.Tag, SharpMap.Layers.ILayer)
        If lyr Is Nothing Then Return

        ' Envelope da camada e zoom
        Dim env As GeoAPI.Geometries.Envelope = Nothing
        Try
            env = lyr.Envelope
        Catch
            ' fallback: se não suportar Envelope, ignora silenciosamente
        End Try
        If env IsNot Nothing Then
            baseMap.ZoomToBox(env)
            mapBox.Refresh()
        End If
    End Sub

    Private Sub RemoveLayer_Click(sender As Object, e As EventArgs)
        Dim it = TryCast(sender, ToolStripMenuItem)
        If it Is Nothing Then Return
        Dim lyr = TryCast(it.Tag, SharpMap.Layers.ILayer)
        If lyr Is Nothing Then Return

        baseMap.Layers.Remove(lyr)
        mapBox.Refresh()
        RebuildLayersMenu()
    End Sub



    Private Sub UpdateCheckedTools(pan As Boolean, zoomIn As Boolean, zoomOut As Boolean, zoomWin As Boolean, identify As Boolean)
        If miPan IsNot Nothing Then miPan.Checked = pan
        If miZoomIn IsNot Nothing Then miZoomIn.Checked = zoomIn
        If miZoomOut IsNot Nothing Then miZoomOut.Checked = zoomOut
        If miZoomWindow IsNot Nothing Then miZoomWindow.Checked = zoomWin
        If miIdentificar IsNot Nothing Then miIdentificar.Checked = identify
    End Sub


    Private Function EnsureMenu(caption As String) As ToolStripMenuItem
        If MenuStrip1 Is Nothing Then
            MenuStrip1 = New MenuStrip()
            Me.Controls.Add(MenuStrip1)
            MenuStrip1.BringToFront()
        End If
        For Each top As ToolStripItem In MenuStrip1.Items
            Dim m = TryCast(top, ToolStripMenuItem)
            If m IsNot Nothing AndAlso String.Equals(m.Text, caption, StringComparison.OrdinalIgnoreCase) Then
                Return m
            End If
        Next
        Dim created As New ToolStripMenuItem(caption)
        MenuStrip1.Items.Add(created)
        Return created
    End Function

    Private Sub ClearIfExists(parent As ToolStripMenuItem, texts As IEnumerable(Of String))
        Dim toRemove As New List(Of ToolStripItem)()
        For Each it As ToolStripItem In parent.DropDownItems
            If texts.Contains(it.Text) OrElse TypeOf it Is ToolStripSeparator Then
                toRemove.Add(it)
            End If
        Next
        For Each it In toRemove
            parent.DropDownItems.Remove(it)
        Next
    End Sub

    '------------------------------------------------------------
    ' AÇÕES
    '------------------------------------------------------------
    Private Sub SetTool(tool As MapBox.Tools)
        identifyMode = False
        mapBox.ActiveTool = tool
        Select Case tool
            Case MapBox.Tools.Pan
                UpdateCheckedTools(pan:=True, zoomIn:=False, zoomOut:=False, zoomWin:=False, identify:=False)
            Case MapBox.Tools.ZoomIn
                UpdateCheckedTools(pan:=False, zoomIn:=True, zoomOut:=False, zoomWin:=False, identify:=False)
            Case MapBox.Tools.ZoomOut
                UpdateCheckedTools(pan:=False, zoomIn:=False, zoomOut:=True, zoomWin:=False, identify:=False)
            Case MapBox.Tools.ZoomWindow
                UpdateCheckedTools(pan:=False, zoomIn:=False, zoomOut:=False, zoomWin:=True, identify:=False)
            Case Else
                UpdateCheckedTools(pan:=False, zoomIn:=False, zoomOut:=False, zoomWin:=False, identify:=False)
        End Select
    End Sub

    Private Sub BtnIdentify_Click(sender As Object, e As EventArgs)
        identifyMode = True
        mapBox.ActiveTool = MapBox.Tools.QueryPoint
        UpdateCheckedTools(pan:=False, zoomIn:=False, zoomOut:=False, zoomWin:=False, identify:=True)
    End Sub

    Private Sub BtnOpen_Click(sender As Object, e As EventArgs)
        Using ofd As New OpenFileDialog()
            ofd.Filter = "Shapefile (*.shp)|*.shp"
            If ofd.ShowDialog() = DialogResult.OK Then
                LoadShapefile(ofd.FileName)
            End If
        End Using
    End Sub

    Private Sub BtnZoomFull_Click(sender As Object, e As EventArgs)
        If baseMap Is Nothing OrElse baseMap.Layers.Count = 0 Then Return
        baseMap.ZoomToExtents()
        mapBox.Refresh()
    End Sub


    Private Sub BtnClear_Click(sender As Object, e As EventArgs)
        baseMap.Layers.Clear()
        mapBox.Refresh()
        RebuildLayersMenu()
        RefreshLayersPanel()
        RebuildLayersMenu()


    End Sub



    '------------------------------------------------------------
    ' CARREGAR SHAPE + ESTILO
    '------------------------------------------------------------
    Private Sub LoadShapefile(shpPath As String)
        Dim shp As New ShapeFile(shpPath, True)
        Dim vlay As New VectorLayer(Path.GetFileNameWithoutExtension(shpPath), shp)

        Select Case shp.ShapeType
            Case ShapeType.Polygon, ShapeType.PolygonM, ShapeType.PolygonZ
                vlay.Style.Fill = New SolidBrush(Color.FromArgb(220, 236, 246))
                vlay.Style.Outline = Pens.SteelBlue
                vlay.Style.EnableOutline = True
            Case ShapeType.PolyLine, ShapeType.PolyLineM, ShapeType.PolyLineZ
                vlay.Style.Line = New Pen(Color.DarkSlateBlue, 1.8F)
            Case ShapeType.Point, ShapeType.PointM, ShapeType.PointZ, ShapeType.Multipoint
                vlay.Style.PointColor = New SolidBrush(Color.DarkRed)
                vlay.Style.PointSize = 6.0F
                vlay.Style.EnableOutline = True
                vlay.Style.Outline = Pens.White
            Case Else
                vlay.Style.EnableOutline = True
                vlay.Style.Outline = Pens.Gray
        End Select

        baseMap.Layers.Add(vlay)
        If baseMap.Layers.Count = 1 Then baseMap.ZoomToExtents()
        mapBox.Refresh()
        RebuildLayersMenu()
        RefreshLayersPanel()
        RebuildLayersMenu()


    End Sub

    '------------------------------------------------------------
    ' IDENTIFICAÇÃO POR CLIQUE
    '------------------------------------------------------------
    Private Sub MapBox_MouseClick(sender As Object, e As MouseEventArgs)

        If baseMap Is Nothing OrElse baseMap.Layers.Count = 0 Then Return
        If e.Button <> MouseButtons.Left Then Return

        ' Converte pixel -> coordenada de mundo
        Dim imgPt As New PointF(CSng(e.X), CSng(e.Y))
        Dim world As GeoAPI.Geometries.Coordinate = mapBox.Map.ImageToWorld(imgPt)
        Dim tol As Double = mapBox.Map.PixelSize * 5
        Dim env As New GeoAPI.Geometries.Envelope(world.X - tol, world.X + tol, world.Y - tol, world.Y + tol)

        ' ---------- NOVO MODO: capturar "Mini" ----------
        If miniPickMode Then
            Dim miniValue As String = Nothing
            ' antes: If TryGetMiniAtEnvelope(env, miniValue) Then
            ' agora, restringindo à camada "Rios":
            If TryGetMiniAtEnvelopeFromLayer("Rios", env, miniValue) Then
                ShowHidrogramaPorMini(miniValue)
                ' se quiser permanecer no modo, mantenha assim:
                ' miniPickMode = True
                ' se quiser sair do modo após 1 clique, descomente:
                ' miniPickMode = False : mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.Pan
                If lblCoord IsNot Nothing Then lblCoord.Text = $"Mini selecionado (Rios): {miniValue}"
            Else
                MessageBox.Show("Nenhuma feição da camada 'Rios' sob o clique (ou sem coluna 'Mini').",
                        "Hidrogramas", MessageBoxButtons.OK, MessageBoxIcon.Information)
            End If
            Return
        End If


        ' ---------- SEU IDENTIFY ANTIGO (mantido) ----------
        If identifyMode Then
            For i As Integer = baseMap.Layers.Count - 1 To 0 Step -1
                Dim v = TryCast(baseMap.Layers(i), SharpMap.Layers.VectorLayer)
                If v Is Nothing OrElse v.DataSource Is Nothing Then Continue For

                Dim fds As New SharpMap.Data.FeatureDataSet()
                v.DataSource.ExecuteIntersectionQuery(env, fds)
                If fds.Tables.Count > 0 AndAlso fds.Tables(0).Rows.Count > 0 Then
                    Dim row = fds.Tables(0).Rows(0)
                    Dim sb As New System.Text.StringBuilder()
                    For Each col As DataColumn In fds.Tables(0).Columns
                        If Not String.Equals(col.ColumnName, "geom", StringComparison.OrdinalIgnoreCase) Then
                            sb.AppendLine($"{col.ColumnName}: {row(col)}")
                        End If
                    Next
                    MessageBox.Show(sb.ToString(), $"Atributos — {v.LayerName}",
                                MessageBoxButtons.OK, MessageBoxIcon.Information)
                    Exit For
                End If
            Next
        End If
    End Sub


    ' Procura o valor da coluna "Mini" apenas na camada indicada (por nome),
    ' dentro do envelope clicado.
    Private Function TryGetMiniAtEnvelopeFromLayer(layerName As String,
                                               env As GeoAPI.Geometries.Envelope,
                                               ByRef miniValue As String) As Boolean
        miniValue = Nothing

        For i As Integer = baseMap.Layers.Count - 1 To 0 Step -1
            Dim v = TryCast(baseMap.Layers(i), SharpMap.Layers.VectorLayer)
            If v Is Nothing Then Continue For

            ' filtra pela camada desejada (case-insensitive)
            If Not String.Equals(v.LayerName, layerName, StringComparison.OrdinalIgnoreCase) Then Continue For
            If v.DataSource Is Nothing Then Return False

            Dim fds As New SharpMap.Data.FeatureDataSet()
            v.DataSource.ExecuteIntersectionQuery(env, fds)
            If fds.Tables.Count = 0 OrElse fds.Tables(0).Rows.Count = 0 Then Return False

            ' acha a coluna "Mini" (case-insensitive)
            Dim tbl = fds.Tables(0)
            Dim miniCol As DataColumn = Nothing
            For Each c As DataColumn In tbl.Columns
                If String.Equals(c.ColumnName, "Mini", StringComparison.OrdinalIgnoreCase) Then
                    miniCol = c : Exit For
                End If
            Next
            If miniCol Is Nothing Then Return False

            miniValue = Convert.ToString(tbl.Rows(0)(miniCol), Globalization.CultureInfo.InvariantCulture)
            Return True
        Next

        Return False
    End Function



    ' Procura, de cima para baixo, o primeiro feature intersectando env
    ' que contenha a coluna "Mini" (qualquer capitalização) e retorna seu valor.
    Private Function TryGetMiniAtEnvelope(env As GeoAPI.Geometries.Envelope, ByRef miniValue As String) As Boolean
        miniValue = Nothing

        For i As Integer = baseMap.Layers.Count - 1 To 0 Step -1
            Dim v = TryCast(baseMap.Layers(i), SharpMap.Layers.VectorLayer)
            If v Is Nothing OrElse v.DataSource Is Nothing Then Continue For

            Dim fds As New SharpMap.Data.FeatureDataSet()
            v.DataSource.ExecuteIntersectionQuery(env, fds)
            If fds.Tables.Count = 0 OrElse fds.Tables(0).Rows.Count = 0 Then Continue For

            Dim tbl = fds.Tables(0)
            ' descobre o nome exato da coluna "Mini" (case-insensitive)
            Dim miniCol As DataColumn = Nothing
            For Each c As DataColumn In tbl.Columns
                If String.Equals(c.ColumnName, "Mini", StringComparison.OrdinalIgnoreCase) Then
                    miniCol = c : Exit For
                End If
            Next
            If miniCol Is Nothing Then Continue For

            ' pega a primeira feição encontrada (pode adaptar para listar todas)
            Dim row = tbl.Rows(0)
            miniValue = Convert.ToString(row(miniCol), Globalization.CultureInfo.InvariantCulture)
            Return True
        Next

        Return False
    End Function

    ' Chame seus cálculos / abra gráficos etc.
    Private Sub ShowHidrogramaPorMini(miniValue As String)
        ' TODO: troque esta mensagem pelo que você precisa
        MessageBox.Show($"Mini = {miniValue}", "Hidrogramas (demo)",
                    MessageBoxButtons.OK, MessageBoxIcon.Information)


        'Esta rotina chama a janela de gráficos em razão de qual é função ativa

        Dim selectedShapeIdx As Integer
        selectedShapeIdx = miniValue

        Dim CampoTabAtributos As Integer
        CampoTabAtributos = 6

        Dim NumField As Integer
        NumField = CampoTabAtributos 'numero do field que vc quer ler a informacao - ordem na tabela (no taquari antas é 1)
        postoselecionado = miniValue 'indica a linha e coluna selecionada na tabela de atributos

        'Se for utilizada a vizualização de hidrograma
        Me.SELECIONA_FUNCAO() 'Seleciona a função de visualização a ser chamada

        'Aqui ele chama exceções
        Me.EXCECOES_RESULTADOS()

        ' exemplo: HidrogramaViewer.Mostrar(miniValue, resultadosCache)
    End Sub

    Sub SELECIONA_FUNCAO()
        If Resultado = True Then

            'Dim frmgraph2 As New Form11 'chama o formulário do gráfico
            'frmgraph2.Show() 'chama o gráfico
            Dim frmgraph2 As New Form23 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ResultadoNível = True Then

            'Dim frmgraph2 As New Form11 'chama o formulário do gráfico
            'frmgraph2.Show() 'chama o gráfico
            Dim frmgraph2 As New Form23 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ResultadoNívelpp = True Then

            'Dim frmgraph2 As New Form11 'chama o formulário do gráfico
            'frmgraph2.Show() 'chama o gráfico
            Dim frmgraph2 As New Form23 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ResultadoNívelref = True Then

            'Dim frmgraph2 As New Form11 'chama o formulário do gráfico
            'frmgraph2.Show() 'chama o gráfico
            Dim frmgraph2 As New Form23 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ResultadoVazaopp = True Then

            'Dim frmgraph2 As New Form11 'chama o formulário do gráfico
            'frmgraph2.Show() 'chama o gráfico
            Dim frmgraph2 As New Form23 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ResultadoLimiares = True Then

            Dim frmgraph2 As New Form4 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf VisuResultado = True Then

            Dim frmgraph2 As New Form4 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ChuvaSUBS = True Then

            Dim frmgraph2 As New Form4 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf Grava = True Then

            SalvaHidrograma()

        ElseIf Grava_nível = True Then

            SalvaCotagrama()

        ElseIf Grava_chuva = True Then

            SalvaPluviograma()

        ElseIf ONS = True Then

            SalvaArquivoONS()

        End If

    End Sub


    'Redundancia para chamar Usina do Gasometro junto
    Sub EXCECOES_RESULTADOS()
        If postoselecionado = 8463 Then
            postoselecionado = 8461
            Me.SELECIONA_FUNCAO()
        End If


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


    Sub SalvaPluviograma()
        'Esta rotina escreve o pluviograma do local selecionado

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

            ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "Chuva observada (pontual)" & " " & "Chuva prevista (pontual)" & " " & "Chuva observada (acumulada)" & " " & "Chuva prevista (acumulada)")
            PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "Chuva observada (pontual)" & " " & "Chuva prevista (pontual)" & " " & "Chuva observada (montante)" & " " & "Chuva prevista (montante)" & " " & "Chuva observada (montante acumulada)" & " " & "Chuva prevista (montante acumulada)")


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

                Dim y, y2, y3, y4, y5, y6 As Single
                If contadordiamenor <= tchuvabin1 Then
                    y = chuva_montante1(postoselecionado, contadordiamenor)
                    y2 = chuvabin1(postoselecionado, contadordiamenor)
                    y5 = chuva_montante1_acumulada(postoselecionado, tchuvabin1 - contadordiamenor)
                    y3 = -1
                    y4 = -1
                    y6 = -1
                ElseIf contadordiamenor > tchuvabin1 Then
                    Dim diff As Integer
                    diff = contadordiamenor - tchuvabin1
                    y3 = chuvabin2(postoselecionado, diff)
                    y4 = chuva_montante2(postoselecionado, diff)
                    y6 = chuva_montante2_acumulada(postoselecionado, diff)
                    y = -1
                    y2 = -1
                    y5 = -1
                End If


                ' Preenche os valores do registro
                'PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

                'PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(y2) & " " & SubstituirSeparador(y3) & " " & SubstituirSeparador(y) & " " & SubstituirSeparador(y4))
                PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(y2) & " " & SubstituirSeparador(y3) & " " & SubstituirSeparador(y) & " " & SubstituirSeparador(y4) & " " & SubstituirSeparador(y5) & " " & SubstituirSeparador(y6))


            Next
            FileClose()

        End If

    End Sub

    Sub SalvaArquivoONS()
        'Esta rotina escreve o hidrograma do local selecionado

        Dim sfd1 As New Windows.Forms.SaveFileDialog
        sfd1.Filter = "ASCII com dados diários de previsão para a ONS(.txt)|*.txt"

        If sfd1.ShowDialog() = Windows.Forms.DialogResult.OK Then

            'FileOpen(5, sfd1.FileName, OpenMode.Output)

            'Cria uma variável e calcula o número de intervalos de tempo que ela possui
            'Dim difft As Integer
            'difft = DateDiff(DateInterval.Hour, dataI, dataF) + 1

            Dim contadordiamenor, numerodedias As Integer
            contadordiamenor = 0
            Dim datacomeço As Date
            'para fazer o gráfico
            datacomeço = Começoresultado
            numerodedias = tamdoresultado
            datacomeço = datacomeço.AddHours(-1)

            Dim toto As Integer
            toto = 24 * 15
            Dim prevONS(toto) As Single 'prevONS(hora,dia,mes) = previsão

            For hh As Integer = 1 To toto
                prevONS(hh) = -1
            Next
            'Dim datacomeço As Date
            'datacomeço = dataI.AddHours(-1)

            'Dim contadordiamenor As Integer
            'contadordiamenor = 0

            ' PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")

            Dim datahoje As Date
            datahoje = Now.Day & "/" & Now.Month & "/" & Now.Year & " 00:00:00"

            Dim contadorONS As Integer
            contadorONS = 0

            For t As Integer = 1 To numerodedias 'difft
                datacomeço = datacomeço.AddHours(1)
                contadordiamenor = contadordiamenor + 1

                'Dim dia, mes, hora As String
                'dia = datacomeço.Day
                'If dia.ToString.Length < 2 Then
                '    dia = "0" & dia
                'End If
                'mes = datacomeço.Month
                'If mes.ToString.Length < 2 Then
                '    mes = "0" & mes
                'End If
                'hora = datacomeço.Hour
                'If hora.ToString.Length < 2 Then
                '    hora = "0" & hora
                'End If

                ' Preenche os valores do registro
                ' PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

                If datacomeço >= datahoje Then
                    contadorONS += 1
                    If QPROPHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = QPROPHIG(postoselecionado, contadordiamenor)
                    If VazaoHIG(postoselecionado, contadordiamenor) > 0 Then prevONS(contadorONS) = VazaoHIG(postoselecionado, contadordiamenor)
                End If

            Next
            ' FileClose()
            Dim inteiro(6) As Integer
            Dim Média(6) As Single
            contadorONS = 0
            Dim contamédia As Integer
            contamédia = 0
            For dd As Integer = 1 To 6
                contamédia = 0
                For hh As Integer = 1 To 24
                    contadorONS += 1
                    If prevONS(contadorONS) > 0 Then
                        Média(dd) += prevONS(contadorONS)
                        contamédia += 1
                    End If
                Next
                Média(dd) = Média(dd) / contamédia
                inteiro(dd) = Média(dd)
            Next

            FileOpen(5, sfd1.FileName, OpenMode.Output)
            Dim dia, mes, hora As String
            dia = datahoje.Day
            If dia.ToString.Length < 2 Then
                dia = "0" & dia
            End If
            mes = datahoje.Month
            If mes.ToString.Length < 2 Then
                mes = "0" & mes
            End If
            hora = datahoje.Hour
            If hora.ToString.Length < 2 Then
                hora = "0" & hora
            End If
            PrintLine(5, datahoje.Year & mes & dia)

            Print(5, "BGR")
            For dd As Integer = 1 To 5
                If inteiro(dd) >= 1000 Then
                    Print(5, "  " & inteiro(dd))
                End If
                If inteiro(dd) < 1000 And inteiro(dd) >= 100 Then
                    Print(5, "   " & inteiro(dd))
                End If
                If inteiro(dd) < 100 And inteiro(dd) >= 10 Then
                    Print(5, "    " & inteiro(dd))
                End If
                If inteiro(dd) < 10 And inteiro(dd) >= 1 Then
                    Print(5, "     " & inteiro(dd))
                End If
            Next

            PrintLine(5, "      :1,500")
            FileClose()

        End If

    End Sub




    '------------------------------------------------------------
    ' COORDENADAS NA STATUS BAR
    '------------------------------------------------------------
    Private Sub MapBox_MouseMove(sender As Object, e As MouseEventArgs)
        If mapBox Is Nothing OrElse mapBox.Map Is Nothing Then Return
        Dim imgPt As New PointF(CSng(e.X), CSng(e.Y))
        Dim world As Coordinate = mapBox.Map.ImageToWorld(imgPt)
        If world IsNot Nothing Then
            lblCoord.Text = $"X: {world.X:0.###}    Y: {world.Y:0.###}"
        Else
            lblCoord.Text = "X: —    Y: —"
        End If
    End Sub

    ' --- Helpers de UI para estilo ---
    Private Function AskColor(initial As Color) As Color?
        Using cd As New ColorDialog()
            cd.Color = initial
            If cd.ShowDialog() = DialogResult.OK Then
                Return cd.Color
            End If
        End Using
        Return Nothing
    End Function

    Private Function AskSingle(prompt As String, initial As Single, minValue As Single, maxValue As Single) As Single?
        Dim txt = Microsoft.VisualBasic.Interaction.InputBox($"{prompt} (mín {minValue}, máx {maxValue})", "Valor", initial.ToString(System.Globalization.CultureInfo.InvariantCulture))
        If String.IsNullOrWhiteSpace(txt) Then Return Nothing
        Dim v As Single
        If Single.TryParse(txt.Replace(",", "."), System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, v) Then
            v = Math.Max(minValue, Math.Min(maxValue, v))
            Return v
        End If
        Return Nothing
    End Function

    Private Sub ApplyAndRefresh()
        mapBox.Refresh()
        RebuildLayersMenu()
        RefreshLayersPanel()
    End Sub


    ' ---- Estilo: manipuladores genéricos ----
    Private Sub ChangePolygonFill_Click(sender As Object, e As EventArgs)
        Dim lyr = TryCast(DirectCast(sender, ToolStripMenuItem).Tag, VectorLayer)
        If lyr Is Nothing Then Return
        Dim current = If(TryCast(lyr.Style.Fill, SolidBrush)?.Color, Color.LightBlue)
        Dim c = AskColor(current)
        If c.HasValue Then
            lyr.Style.Fill = New SolidBrush(c.Value)
            ApplyAndRefresh()
        End If
    End Sub

    Private Sub ChangePolygonOutline_Click(sender As Object, e As EventArgs)
        Dim lyr = TryCast(DirectCast(sender, ToolStripMenuItem).Tag, VectorLayer)
        If lyr Is Nothing Then Return
        Dim currPen = If(lyr.Style.Outline, Pens.SteelBlue)
        Dim c = AskColor(currPen.Color)
        If c.HasValue Then
            Dim w = If(lyr.Style.Outline IsNot Nothing, lyr.Style.Outline.Width, 1.2F)
            lyr.Style.Outline = New Pen(c.Value, w)
            lyr.Style.EnableOutline = True
            ApplyAndRefresh()
        End If
    End Sub

    Private Sub ChangeLineColor_Click(sender As Object, e As EventArgs)
        Dim lyr = TryCast(DirectCast(sender, ToolStripMenuItem).Tag, VectorLayer)
        If lyr Is Nothing Then Return
        Dim currPen = If(lyr.Style.Line, New Pen(Color.DarkSlateBlue, 1.5F))
        Dim c = AskColor(currPen.Color)
        If c.HasValue Then
            Dim w = If(lyr.Style.Line IsNot Nothing, lyr.Style.Line.Width, 1.5F)
            lyr.Style.Line = New Pen(c.Value, w)
            ApplyAndRefresh()
        End If
    End Sub

    Private Sub ChangeLineWidth_Click(sender As Object, e As EventArgs)
        Dim lyr = TryCast(DirectCast(sender, ToolStripMenuItem).Tag, VectorLayer)
        If lyr Is Nothing Then Return
        Dim currW = If(lyr.Style.Line IsNot Nothing, lyr.Style.Line.Width, 1.5F)
        Dim v = AskSingle("Espessura da linha (px)", CSng(currW), 0.1F, 20.0F)
        If v.HasValue Then
            Dim c = If(lyr.Style.Line IsNot Nothing, lyr.Style.Line.Color, Color.DarkSlateBlue)
            lyr.Style.Line = New Pen(c, v.Value)
            ApplyAndRefresh()
        End If
    End Sub

    Private Sub ChangePointColor_Click(sender As Object, e As EventArgs)
        Dim lyr = TryCast(DirectCast(sender, ToolStripMenuItem).Tag, VectorLayer)
        If lyr Is Nothing Then Return
        Dim curr = If(TryCast(lyr.Style.PointColor, SolidBrush)?.Color, Color.DarkRed)
        Dim c = AskColor(curr)
        If c.HasValue Then
            lyr.Style.PointColor = New SolidBrush(c.Value)
            ApplyAndRefresh()
        End If
    End Sub

    Private Sub ChangePointSize_Click(sender As Object, e As EventArgs)
        Dim lyr = TryCast(DirectCast(sender, ToolStripMenuItem).Tag, VectorLayer)
        If lyr Is Nothing Then Return
        Dim curr = If(lyr.Style.PointSize > 0, lyr.Style.PointSize, 6.0F)
        Dim v = AskSingle("Tamanho do ponto (px)", curr, 1.0F, 64.0F)
        If v.HasValue Then
            lyr.Style.PointSize = v.Value
            ApplyAndRefresh()
        End If
    End Sub

    Private Sub ChangePointOutline_Click(sender As Object, e As EventArgs)
        Dim lyr = TryCast(DirectCast(sender, ToolStripMenuItem).Tag, VectorLayer)
        If lyr Is Nothing Then Return
        Dim currPen = If(lyr.Style.Outline, Pens.White)
        Dim c = AskColor(currPen.Color)
        If c.HasValue Then
            Dim w = If(lyr.Style.Outline IsNot Nothing, lyr.Style.Outline.Width, 1.0F)
            lyr.Style.Outline = New Pen(c.Value, w)
            lyr.Style.EnableOutline = True
            ApplyAndRefresh()
        End If
    End Sub

    ' === Helpers de amostra de cor (colar dentro da classe Form1) ===
    Private Function CreateLayerSwatch(lyr As SharpMap.Layers.ILayer) As Bitmap
        Dim bmp As New Bitmap(16, 16)
        Using g As Graphics = Graphics.FromImage(bmp)
            g.Clear(Color.Transparent)
            Dim c As Color = GetLayerDisplayColor(lyr)
            Using b As New SolidBrush(c)
                g.FillRectangle(b, 1, 3, 14, 10)
            End Using
            Using p As New Pen(Color.Black)
                g.DrawRectangle(p, 1, 3, 14, 10)
            End Using
        End Using
        Return bmp
    End Function

    Private Function GetLayerDisplayColor(lyr As SharpMap.Layers.ILayer) As Color
        Dim v = TryCast(lyr, VectorLayer)
        If v Is Nothing Then Return Color.Gray

        Dim prov = TryCast(v.DataSource, ShapeFile)
        Dim st As ShapeType = If(prov IsNot Nothing, prov.ShapeType, ShapeType.Null)

        Select Case st
            Case ShapeType.Polygon, ShapeType.PolygonM, ShapeType.PolygonZ
                Dim sb = TryCast(v.Style.Fill, SolidBrush)
                If sb IsNot Nothing Then Return sb.Color
                Return Color.LightSteelBlue

            Case ShapeType.PolyLine, ShapeType.PolyLineM, ShapeType.PolyLineZ
                If v.Style.Line IsNot Nothing Then Return v.Style.Line.Color
                Return Color.SteelBlue

            Case ShapeType.Point, ShapeType.PointM, ShapeType.PointZ, ShapeType.Multipoint
                Dim sbPt = TryCast(v.Style.PointColor, SolidBrush)
                If sbPt IsNot Nothing Then Return sbPt.Color
                Return Color.IndianRed

            Case Else
                Dim sb2 = TryCast(v.Style.Fill, SolidBrush)
                If sb2 IsNot Nothing Then Return sb2.Color
                If v.Style.Line IsNot Nothing Then Return v.Style.Line.Color
                Dim sbPt2 = TryCast(v.Style.PointColor, SolidBrush)
                If sbPt2 IsNot Nothing Then Return sbPt2.Color
                Return Color.Gray
        End Select
    End Function

    ' ========= Helpers de símbolo (pontos) =========
    Private Function MakeSquareSymbol(fill As Color, Optional size As Integer = 14) As Bitmap
        Dim bmp As New Bitmap(size, size)
        Using g = Graphics.FromImage(bmp)
            g.SmoothingMode = Drawing2D.SmoothingMode.AntiAlias
            g.Clear(Color.Transparent)
            Dim pad As Integer = 2
            Using b As New SolidBrush(fill)
                g.FillRectangle(b, pad, pad, size - 2 * pad, size - 2 * pad)
            End Using
            Using p As New Pen(Color.Black, 1.0F)
                g.DrawRectangle(p, pad, pad, size - 2 * pad, size - 2 * pad)
            End Using
        End Using
        Return bmp
    End Function

    Private Function MakeTriangleSymbol(fill As Color, Optional size As Integer = 16) As Bitmap
        Dim bmp As New Bitmap(size, size)
        Using g = Graphics.FromImage(bmp)
            g.SmoothingMode = Drawing2D.SmoothingMode.AntiAlias
            g.Clear(Color.Transparent)
            Dim pad As Single = 2.0F
            Dim pts As PointF() = {
            New PointF(size / 2.0F, pad),
            New PointF(size - pad, size - pad),
            New PointF(pad, size - pad)
        }
            Using b As New SolidBrush(fill)
                g.FillPolygon(b, pts)
            End Using
            Using p As New Pen(Color.Black, 1.0F)
                g.DrawPolygon(p, pts)
            End Using
        End Using
        Return bmp
    End Function

    ' ========= Loader genérico com estilo =========
    Private Function AddVectorLayer(shpPath As String, layerName As String, styleApplier As Action(Of SharpMap.Layers.VectorLayer)) As SharpMap.Layers.VectorLayer
        Dim prov As New SharpMap.Data.Providers.ShapeFile(shpPath, True)
        Dim v As New SharpMap.Layers.VectorLayer(layerName, prov)
        styleApplier?.Invoke(v)
        baseMap.Layers.Add(v)
        Return v
    End Function


    Private Sub AutoLoadStartupLayers()
        Dim shpDir As String = IO.Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Shapefiles")

        ' helper local
        Dim load As Func(Of String, String, Action(Of SharpMap.Layers.VectorLayer), SharpMap.Layers.VectorLayer) =
        Function(fileName As String, niceName As String, applier As Action(Of SharpMap.Layers.VectorLayer)) As SharpMap.Layers.VectorLayer
            Dim full As String = IO.Path.Combine(shpDir, fileName)
            If Not IO.File.Exists(full) Then Return Nothing
            Return AddVectorLayer(full, niceName, applier)
        End Function

        ' --- carregue primeiro as camadas "de fundo" (ficarão por baixo) ---
        load("areas_urbanas.shp", "Áreas Urbanas",
        Sub(v)
            v.Style.Fill = New SolidBrush(Color.FromArgb(200, 210, 210, 210))
            v.Style.Outline = New Pen(Color.DimGray, 1.0F)
            v.Style.EnableOutline = True
        End Sub)

        load("ibge_limiteRS_ajust.shp", "Limite RS (IBGE)",
        Sub(v)
            v.Style.Fill = New SolidBrush(Color.FromArgb(0, 0, 0, 0))
            v.Style.Outline = New Pen(Color.Red, 1.2F)
            v.Style.EnableOutline = True
        End Sub)

        load("Bacia2_RS.shp", "Bacias (contorno)",
        Sub(v)
            v.Style.Fill = New SolidBrush(Color.FromArgb(0, 0, 0, 0))
            v.Style.Outline = New Pen(Color.FromArgb(40, 40, 40), 2.2F)
            v.Style.EnableOutline = True
        End Sub)

        load("Rios.shp", "Rios",
        Sub(v)
            v.Style.Line = New Pen(Color.RoyalBlue, 1.4F)
        End Sub)



        ' --- guarde referências dos dois "de água" para irem POR CIMA ---
        Dim lyrReserv As SharpMap.Layers.VectorLayer = load("reservatórios.shp", "Reservatórios",
        Sub(v)
            v.Style.Fill = New SolidBrush(Color.FromArgb(200, 198, 226)) ' azul claro
            v.Style.Outline = New Pen(Color.DarkBlue, 1.2F)
            v.Style.EnableOutline = True
        End Sub)

        Dim lyrLagos As SharpMap.Layers.VectorLayer = load("lagos_lagoas.shp", "Lagos/Lagoas",
        Sub(v)
            v.Style.Fill = New SolidBrush(Color.FromArgb(200, 198, 226))
            v.Style.Outline = New Pen(Color.DarkBlue, 1.2F)
            v.Style.EnableOutline = True
        End Sub)

        '' Pontos (se quiser que fiquem abaixo dos lagos/reservatórios, carregue agora;
        '' se quiser que fiquem ACIMA da água, mova estes dois loads para DEPOIS dos lagos/reservatórios)
        'load("UHEs_RS.shp", "UHEs (RS)",
        'Sub(v)
        '    v.Style.Symbol = MakeTriangleSymbol(Color.Gold)
        '    v.Style.SymbolScale = 1.0F
        'End Sub)

        ''--'load("Postos_flu_telemetria_RSH2.shp", "Postos Flu/Telemetria",
        ''load("EstacoesNivel.shp", "Postos Flu/Telemetria",
        ''Sub(v)
        ''    v.Style.Symbol = MakeSquareSymbol(Color.Red)
        ''    v.Style.SymbolScale = 1.0F
        ''End Sub)


        ' ------------------------------------------------------------
        ' ESTAÇÕES / PONTOS
        ' ------------------------------------------------------------

        '        ' UHEs
        '        load("UHEs_RS.shp", "UHEs (RS)",
        'Sub(v)
        '    v.Style.Symbol = MakeTriangleSymbol(Color.Gold)
        '    v.Style.SymbolScale = 1.0F
        'End Sub)


        ' Estações pluviométricas
        ' Círculo azul celeste
        load("EstacoesPluv.shp", "Estações Pluviométricas",
Sub(v)
    v.Style.Symbol = MakeCircleSymbol(Color.DeepSkyBlue, Color.DodgerBlue)
    v.Style.SymbolScale = 0.5F
End Sub)

        ' Estações convencionais SGB
        ' Triângulo invertido marrom
        load("F_Convencionais_SGB.shp", "Estações Convencionais SGB",
Sub(v)
    v.Style.Symbol = MakeInvertedTriangleSymbol(Color.SaddleBrown, Color.SaddleBrown, True)
    v.Style.SymbolScale = 1.0F
End Sub)


        ' Estações telemétricas SGB
        ' Triângulo invertido verde claro
        load("F_Telemetricas_SGB.shp", "Estações Telemétricas SGB",
Sub(v)
    v.Style.Symbol = MakeInvertedTriangleSymbol(Color.LightGreen, Color.ForestGreen, True)
    v.Style.SymbolScale = 1.0F
End Sub)





        '        ' Estações de nível
        '        ' Triângulo invertido transparente, apenas contorno
        '        load("EstacoesNivel.shp", "Estações de Nível",
        'Sub(v)
        '    v.Style.Symbol = MakeInvertedTriangleSymbol(Color.Transparent, Color.Black, False)
        '    v.Style.SymbolScale = 1.0F
        'End Sub)



        '    'Dim shpDir As String = IO.Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Shapefiles")
        '    Dim estacoesPath As String = IO.Path.Combine(shpDir, "EstacoesNivel.shp")
        '    If IO.File.Exists(estacoesPath) Then
        '        Dim provEst As New SharpMap.Data.Providers.ShapeFile(estacoesPath, True)

        '        ' camada de pontos
        '        Dim lyrEst As New SharpMap.Layers.VectorLayer("Estações de Nível", provEst)
        '        lyrEst.Style.Symbol = MakeSquareSymbol(Color.Red)
        '        'lyrEst.Style.PointColor = New SolidBrush(Color.DarkRed)
        '        lyrEst.Style.PointSize = 7.0F
        '        lyrEst.Style.EnableOutline = True
        '        lyrEst.Style.Outline = Pens.Black
        '        baseMap.Layers.Add(lyrEst)

        '        ' camada de rótulos
        '        Dim labelCol As String = ResolveColumnCaseInsensitive(provEst, "CODIGO")
        '        Dim lbl As New SharpMap.Layers.LabelLayer("Rótulos Estações") With {
        '    .DataSource = provEst,
        '    .LabelColumn = labelCol
        '}
        '        lbl.Style = New SharpMap.Styles.LabelStyle() With {
        '    .ForeColor = Color.Black,
        '    .BackColor = New SolidBrush(Color.FromArgb(180, Color.White)),
        '    .Halo = New Pen(Color.White, 2.0F),
        '    .Font = New Font("Segoe UI", 9.0F, FontStyle.Bold),
        '    .Offset = New PointF(10.0F, -10.0F),
        '    .CollisionDetection = True
        '}
        '        baseMap.Layers.Add(lbl) ' adicionar por último para ficar por cima
        '    End If


        ' ------------------------------------------------------------
        ' UHEs
        ' ------------------------------------------------------------
        Dim uhesPath As String = IO.Path.Combine(shpDir, "UHEs_RS.shp")
        If IO.File.Exists(uhesPath) Then
            Dim provUHE As New SharpMap.Data.Providers.ShapeFile(uhesPath, True)
            ' Camada de pontos
            Dim lyrUHE As New SharpMap.Layers.VectorLayer("UHEs (RS)", provUHE)
            lyrUHE.Style.Symbol = MakeTriangleSymbol(Color.Gold)
            lyrUHE.Style.SymbolScale = 1.0F
            baseMap.Layers.Add(lyrUHE)
            ' ------------------------------------------------------------
            ' Rótulos das UHEs
            ' Campo: EMPREENDIM
            ' ------------------------------------------------------------
            Dim labelColUHE As String =
        ResolveColumnCaseInsensitive(provUHE, "EMPREENDIM")
            Dim lblUHE As New SharpMap.Layers.LabelLayer("Rótulos UHEs") With {
        .DataSource = provUHE,
        .LabelColumn = labelColUHE
    }
            lblUHE.Style = New SharpMap.Styles.LabelStyle() With {
        .ForeColor = Color.Black,
        .BackColor = New SolidBrush(Color.FromArgb(180, Color.White)),
        .Halo = New Pen(Color.White, 2.0F),
        .Font = New Font("Segoe UI", 9.0F, FontStyle.Bold),
        .Offset = New PointF(10.0F, -10.0F),
        .CollisionDetection = True
    }
            baseMap.Layers.Add(lblUHE)
        End If

        Dim estacoesPath As String = IO.Path.Combine(shpDir, "EstacoesNivel.shp")
        If IO.File.Exists(estacoesPath) Then
            Dim provEst As New SharpMap.Data.Providers.ShapeFile(estacoesPath, True)
            ' ------------------------------------------------------------
            ' Camada de pontos - Estações de Nível
            ' Triângulo invertido sem preenchimento
            ' ------------------------------------------------------------
            Dim lyrEst As New SharpMap.Layers.VectorLayer("Estações de Nível", provEst)
            lyrEst.Style.Symbol = MakeInvertedTriangleOutlineSymbol(Color.Black)
            lyrEst.Style.SymbolScale = 1.0F
            baseMap.Layers.Add(lyrEst)
            ' ------------------------------------------------------------
            ' Camada de rótulos
            ' ------------------------------------------------------------
            Dim labelCol As String = ResolveColumnCaseInsensitive(provEst, "CODIGO")
            Dim lbl As New SharpMap.Layers.LabelLayer("Rótulos Estações") With {
        .DataSource = provEst,
        .LabelColumn = labelCol
    }
            lbl.Style = New SharpMap.Styles.LabelStyle() With {
        .ForeColor = Color.Black,
        .BackColor = New SolidBrush(Color.FromArgb(180, Color.White)),
        .Halo = New Pen(Color.White, 2.0F),
        .Font = New Font("Segoe UI", 9.0F, FontStyle.Bold),
        .Offset = New PointF(10.0F, -10.0F),
        .CollisionDetection = True
    }
            ' adicionar por último para ficar por cima
            baseMap.Layers.Add(lbl)
        End If



        ' Reposiciona: remove e adiciona de novo para irem ao topo da pilha (desenho por cima)
        'If lyrReserv IsNot Nothing Then
        '    baseMap.Layers.Remove(lyrReserv)
        '    baseMap.Layers.Add(lyrReserv)
        'End If
        'If lyrLagos IsNot Nothing Then
        '    baseMap.Layers.Remove(lyrLagos)
        '    baseMap.Layers.Add(lyrLagos)
        'End If

        ' --- ZOOM PARA A EXTENSÃO TOTAL (ponto 1) ---
        If baseMap.Layers.Count > 0 Then
            baseMap.ZoomToExtents()
        End If

        ' Atualiza UI
        mapBox.Refresh()
        RefreshLayersPanel()
        RebuildLayersMenu()

        ApplyPerfTweaks()
        mapBox.Refresh()

    End Sub

    Private Function MakeInvertedTriangleSymbol(ByVal fillColor As Color,
                                             ByVal outlineColor As Color,
                                             ByVal preencher As Boolean) As Bitmap

        Dim tamanho As Integer = 18

        Dim bmp As New Bitmap(tamanho, tamanho)

        Using g As Graphics = Graphics.FromImage(bmp)

            g.SmoothingMode = Drawing2D.SmoothingMode.AntiAlias
            g.Clear(Color.Transparent)

            Dim pontos() As PointF = {
            New PointF(2.0F, 3.0F),
            New PointF(tamanho - 3.0F, 3.0F),
            New PointF(tamanho / 2.0F, tamanho - 2.0F)
        }

            ' Preenchimento
            If preencher Then
                Using br As New SolidBrush(fillColor)
                    g.FillPolygon(br, pontos)
                End Using
            End If

            ' Contorno
            Using pn As New Pen(outlineColor, 1.5F)
                g.DrawPolygon(pn, pontos)
            End Using

        End Using

        Return bmp

    End Function

    Private Function MakeCircleSymbol(ByVal fillColor As Color,
                                  ByVal outlineColor As Color) As Bitmap

        Dim tamanho As Integer = 16

        Dim bmp As New Bitmap(tamanho, tamanho)

        Using g As Graphics = Graphics.FromImage(bmp)

            g.SmoothingMode = Drawing2D.SmoothingMode.AntiAlias
            g.Clear(Color.Transparent)

            Using br As New SolidBrush(fillColor)
                g.FillEllipse(br, 2, 2, tamanho - 5, tamanho - 5)
            End Using

            Using pn As New Pen(outlineColor, 1.3F)
                g.DrawEllipse(pn, 2, 2, tamanho - 5, tamanho - 5)
            End Using

        End Using

        Return bmp

    End Function

    Private Function MakeInvertedTriangleOutlineSymbol(ByVal outlineColor As Color) As Bitmap

        Dim tamanho As Integer = 18

        Dim bmp As New Bitmap(tamanho, tamanho)

        Using g As Graphics = Graphics.FromImage(bmp)

            g.SmoothingMode = Drawing2D.SmoothingMode.AntiAlias
            g.Clear(Color.Transparent)

            ' Triângulo apontando para baixo
            Dim pontos() As PointF = {
            New PointF(2.0F, 3.0F),
            New PointF(tamanho - 3.0F, 3.0F),
            New PointF(tamanho / 2.0F, tamanho - 2.0F)
        }

            ' Apenas contorno, sem preenchimento
            Using pn As New Pen(outlineColor, 1.8F)
                g.DrawPolygon(pn, pontos)
            End Using

        End Using

        Return bmp

    End Function

    Private Sub ApplyPerfTweaks()
        If baseMap Is Nothing Then Return

        ' 1) Desligar antialias e ligar clipping por camada
        For Each lyr In baseMap.Layers
            Dim v = TryCast(lyr, SharpMap.Layers.VectorLayer)
            If v Is Nothing Then Continue For

            ' Desenha só o que está no viewport
            v.ClippingEnabled = True

            ' Antialias off acelera muito linhas/polígonos
            v.SmoothingMode = Drawing2D.SmoothingMode.None

            ' 2) Regras por tipo geométrico (se fonte for Shapefile)
            Dim shp = TryCast(v.DataSource, SharpMap.Data.Providers.ShapeFile)
            If shp IsNot Nothing Then
                Select Case shp.ShapeType
                    Case ShapeType.PolyLine, ShapeType.PolyLineM, ShapeType.PolyLineZ
                        ' Rios só quando der zoom (ajuste valor conforme seus SRIDs)
                        ' Se usa coordenadas geográficas (graus), use um valor menor (ex.: 0.2)
                        ' Se usa metros (projeção), use algo como 10000-50000
                        v.MinVisible = 0.2   ' em graus (WGS84). Se for projeção em metros, mude p/ ~20000

                    Case ShapeType.Polygon, ShapeType.PolygonM, ShapeType.PolygonZ
                        ' Contornos muito detalhados: deixe sem antialias e evite outline duplo grosso
                        If v.Style.Outline IsNot Nothing AndAlso v.Style.Outline.Width > 2.0F Then
                            v.Style.Outline.Width = 2.0F
                        End If

                    Case ShapeType.Point, ShapeType.PointM, ShapeType.PointZ, ShapeType.Multipoint
                        ' 3) Se há muitos pontos, prefira círculo simples ao invés de bitmap
                        Dim many As Boolean = False
                        Try
                            Dim cnt As Integer = shp.GetFeatureCount()
                            many = cnt > 5000 ' ajuste o limiar ao seu caso
                        Catch
                        End Try

                        If many Then
                            ' Trocar símbolo bitmap por ponto simples acelera MUITO
                            v.Style.Symbol = Nothing
                            If v.Style.PointColor Is Nothing Then v.Style.PointColor = New SolidBrush(Color.DarkRed)
                            If v.Style.PointSize <= 0 Then v.Style.PointSize = 5.0F
                            ' Contorno fino no ponto (opcional, tirar se quiser ainda mais velocidade)
                            v.Style.EnableOutline = False
                        End If
                End Select
            End If
        Next

        ' 4) Reduzir transparências pesadas (alpha alto em áreas grandes custa)
        '    Se tiver polígonos com Fill muito translúcido, você pode reduzir um pouco o alpha aqui:
        For Each lyr In baseMap.Layers
            Dim v = TryCast(lyr, SharpMap.Layers.VectorLayer)
            If v Is Nothing Then Continue For
            Dim sb = TryCast(v.Style.Fill, SolidBrush)
            If sb IsNot Nothing AndAlso sb.Color.A > 0 AndAlso sb.Color.A < 160 Then
                Dim c = sb.Color
                v.Style.Fill = New SolidBrush(Color.FromArgb(160, c.R, c.G, c.B)) ' limita alpha
            End If
        Next
    End Sub



    Private Sub ChuvaObservadaToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles ChuvaObservadaToolStripMenuItem.Click
        Dim Frm7 As New Form7
        Frm7.Show()

    End Sub


    Sub LOAD_AI()
        ' ''Verifica quantas estações existem na base de dados interna
        ''Dim arquivo_AI As String 'nome do arquivo que contém as estações
        ''Dim linha As String 'linha de leitura
        ''total_estações = 0 'número de estações

        ' ''define o arquivo
        ''arquivo_AI = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\AI.txt"

        ' ''Abre e faz a leituar
        ''FileOpen(1, arquivo_AI, OpenMode.Input)
        ''linha = LineInput(1)
        ''AI = linha
        ''FileClose()



        Me.FALSE_ALL()

        'Variável para rodar automaticamente
        TotalCommander = False

        AI = Now.Year - 10
        AF = Now.Year + 2

    End Sub

    Private Sub PreferênciasToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles PreferênciasToolStripMenuItem.Click
        Dim Frm2 As New Form2
        Frm2.Show()
    End Sub

    Private Sub DadosONStxtToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DadosONStxtToolStripMenuItem.Click
        Dim frm21 As New Form21
        frm21.Show()

    End Sub

    Private Sub VazãoObservadaToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles VazãoObservadaToolStripMenuItem.Click
        Dim frm3 As New Form3
        frm3.Show()
    End Sub

    Private Sub OperaçãoDeReservatóriosToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles OperaçãoDeReservatóriosToolStripMenuItem.Click
        Dim frm5 As New Form5
        frm5.Show()
    End Sub

    Private Sub RodarPrevisãoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles RodarPrevisãoToolStripMenuItem.Click
        Me.ENABLE_RESULTS_BUTTONS()

        Dim frm8 As New Form8
        frm8.Show()
    End Sub

    Sub ENABLE_RESULTS_BUTTONS()
        Label1.Enabled = True
        btCP.Enabled = True
        btCR.Enabled = True
        btHB.Enabled = True
        btHP.Enabled = True

    End Sub

    Sub FALSE_ALL()
        Resultado = False
        ResultadoLimiares = False
        VisuResultado = False
        ChuvaSUBS = False
        Padrão = False
        Grava = False
        ONS = False
        Grava_nível = False
        ResultadoNível = False
        ResultadoNívelpp = False
        ResultadoVazaopp = False
        ResultadoNívelref = False
        Grava_chuva = False

    End Sub

    Private Sub HidrogramasToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles HidrogramasToolStripMenuItem.Click

    End Sub

    ' Ativa o modo de clique para capturar o "Mini"
    Private Sub HidrogramasComLimiarToolStripMenuItem_Click(sender As Object, e As EventArgs) _
        Handles HidrogramasComLimiarToolStripMenuItem.Click

        Me.FALSE_ALL()
        ResultadoLimiares = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub PluviogramasToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles PluviogramasToolStripMenuItem.Click

        Me.FALSE_ALL()
        VisuResultado = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub GeralToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles GeralToolStripMenuItem.Click

        Me.FALSE_ALL()
        Padrão = True

        Dim frm4 As New Form4
        frm4.Show()
    End Sub

    Private Sub BaixoUruguaiToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles BaixoUruguaiToolStripMenuItem.Click
        OpçãoGrafico = 1
        Dim frm12 As New Form12
        frm12.Show()
    End Sub

    Private Sub MédioUruguaiToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles MédioUruguaiToolStripMenuItem.Click
        OpçãoGrafico = 2
        Dim frm12 As New Form12
        frm12.Show()
    End Sub

    Private Sub AltoUruguaiToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles AltoUruguaiToolStripMenuItem.Click
        OpçãoGrafico = 7
        Dim frm12 As New Form12
        frm12.Show()
    End Sub

    Private Sub IjuíIbicuíETaquariToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles IjuíIbicuíETaquariToolStripMenuItem.Click
        OpçãoGrafico = 3
        Dim frm12 As New Form12
        frm12.Show()
    End Sub

    Private Sub JacuíETaquariToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles JacuíETaquariToolStripMenuItem.Click
        OpçãoGrafico = 4
        Dim frm12 As New Form12
        frm12.Show()
    End Sub

    Private Sub SinosCaíEGravataíToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles SinosCaíEGravataíToolStripMenuItem.Click
        OpçãoGrafico = 5
        Dim frm12 As New Form12
        frm12.Show()
    End Sub

    Private Sub NegroELitoralDoceToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles NegroELitoralDoceToolStripMenuItem.Click
        OpçãoGrafico = 6
        Dim frm12 As New Form12
        frm12.Show()
    End Sub

    Private Sub ChuvaNoHorizonteDePrevisãoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles ChuvaNoHorizonteDePrevisãoToolStripMenuItem.Click
        MsgBox("A chuva média no horizonte de previsão é " & FormatNumber(Chuva_no_horizonte, 2) & "mm", MsgBoxStyle.Information, "Chuva no Horizonte de Previsão")

    End Sub

    Private Sub GravarHidrogramaToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles GravarHidrogramaToolStripMenuItem.Click

        Me.FALSE_ALL()
        Grava = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub GravarCotagramaDoTrechoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles GravarCotagramaDoTrechoToolStripMenuItem.Click

        Me.FALSE_ALL()
        Grava_nível = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub CentroDeComandoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles CentroDeComandoToolStripMenuItem.Click

    End Sub

    Private Sub CotagramaDoTrechoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles CotagramaDoTrechoToolStripMenuItem.Click

    End Sub

    Private Sub DesativarVisualizaçãoDeResultadosToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DesativarVisualizaçãoDeResultadosToolStripMenuItem.Click
        miniPickMode = False
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.Pan
    End Sub

    Private Sub DownloadANAToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DownloadANAToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "DownloadTelemetriaANA.exe"

        '' Create new process to run setup
        'Dim processRun As New Process
        '' Set process props
        'processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\DownloadTelemetriaANA\DownloadTelemetriaANA\DownloadTelemetriaANA\bin\x64\Release\" & ExecutavelFORTRAN
        '' Start the process
        'processRun.Start()
        '' Wait for the process to finish
        'processRun.WaitForExit()
        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\DownloadTelemetriaANA\DownloadTelemetriaANA\DownloadTelemetriaANA\bin\x64\Release\" & ExecutavelFORTRAN)

        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

    End Sub

    Private Sub DownloadINMETToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DownloadINMETToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "main_v3.exe"

        '' Create new process to run setup
        'Dim processRun As New Process
        '' Set process props
        'processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Executavel_baixadordadosINMET\Executavel_baixadordadosINMET\dist\" & ExecutavelFORTRAN
        '' Start the process
        'processRun.Start()
        '' Wait for the process to finish
        'processRun.WaitForExit()


        Try
            ' Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\DownloadTelemetriaANA\DownloadTelemetriaANA\DownloadTelemetriaANA\bin\x64\Release\" & ExecutavelFORTRAN)
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Executavel_baixadordadosINMET\dist\main_v3\" & ExecutavelFORTRAN)

        Catch ex As Exception
            MsgBox(ex.Message)
        End Try

    End Sub

    ' Tenta descobrir o nome exato da coluna (case-insensitive) executando uma
    ' consulta sobre o envelope total só para obter o esquema (Columns).
    Private Function ResolveColumnCaseInsensitive(prov As SharpMap.Data.Providers.ShapeFile, desired As String) As String
        If prov Is Nothing Then Return desired
        Dim fds As New SharpMap.Data.FeatureDataSet()
        Try
            prov.Open()
            ' Pega o envelope total e executa uma interseção só para obter o schema
            Dim env = prov.GetExtents()
            prov.ExecuteIntersectionQuery(env, fds)
        Catch
            ' ignora e cai no padrão
        Finally
            Try : prov.Close() : Catch : End Try
        End Try

        If fds.Tables.Count > 0 Then
            For Each col As DataColumn In fds.Tables(0).Columns
                If String.Equals(col.ColumnName, desired, StringComparison.OrdinalIgnoreCase) Then
                    Return col.ColumnName ' retorna o nome exato
                End If
            Next
        End If
        ' fallback
        Return desired
    End Function

    Private Sub VentoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles VentoToolStripMenuItem.Click
        Dim frmVento As New FormWind
        frmVento.Show()
    End Sub

    Private Sub BrutoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles BrutoToolStripMenuItem.Click
        'MsgBox("Para visualizar o hidrograma clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
        Me.FALSE_ALL()
        Resultado = True


        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub PósprocessadoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles PósprocessadoToolStripMenuItem.Click
        'MsgBox("Para visualizar o hidrograma clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
        Me.FALSE_ALL()
        ResultadoVazaopp = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub CotagramaBrutoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles CotagramaBrutoToolStripMenuItem.Click

        Me.FALSE_ALL()
        ResultadoNível = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub CotagramaDeReferênciaPósprocessadoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles CotagramaDeReferênciaPósprocessadoToolStripMenuItem.Click
        Me.FALSE_ALL()
        ResultadoNívelref = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub CotagramaPósprocessadoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles CotagramaPósprocessadoToolStripMenuItem.Click
        Me.FALSE_ALL()
        ResultadoNívelpp = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub SobreToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles SobreToolStripMenuItem.Click
        Dim Frm22 As New Form22
        Frm22.Show()
    End Sub

    Private Sub ONSToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles ONSToolStripMenuItem.Click

        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "ONS_Horarios_Process.exe"

        '' Create new process to run setup
        'Dim processRun As New Process
        '' Set process props
        'processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Executavel_baixadordadosINMET\Executavel_baixadordadosINMET\dist\" & ExecutavelFORTRAN
        '' Start the process
        'processRun.Start()
        '' Wait for the process to finish
        'processRun.WaitForExit()


        Try
            ' Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\DownloadTelemetriaANA\DownloadTelemetriaANA\DownloadTelemetriaANA\bin\x64\Release\" & ExecutavelFORTRAN)
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\ONS\" & ExecutavelFORTRAN)

        Catch ex As Exception
            MsgBox(ex.Message)
        End Try


    End Sub

    Private Sub DownloadIMERGEGSMAPToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DownloadIMERGEGSMAPToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "main_baixador_virtuais.exe"

        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Executavel_baixador_IMERG_GSMAP\main_baixador_virtuais\" & ExecutavelFORTRAN)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Private Sub VisualizaçãoDeHidrogramasToolStripMenuItem_Click(sender As Object, e As EventArgs)

    End Sub

    Private Sub DownloadAPIANAToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DownloadAPIANAToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "telemetria.exe"

        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Download_API_ANA\" & ExecutavelFORTRAN)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Private Sub btHB_Click(sender As Object, e As EventArgs) Handles btHB.Click
        'MsgBox("Para visualizar o hidrograma clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
        Me.FALSE_ALL()
        Resultado = True


        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub btHP_Click(sender As Object, e As EventArgs) Handles btHP.Click
        'MsgBox("Para visualizar o hidrograma clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
        Me.FALSE_ALL()
        ResultadoVazaopp = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub btCR_Click(sender As Object, e As EventArgs) Handles btCR.Click
        Me.FALSE_ALL()
        ResultadoNívelref = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub btCP_Click(sender As Object, e As EventArgs) Handles btCP.Click
        Me.FALSE_ALL()
        ResultadoNívelpp = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub DownloadVentoINMETToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DownloadVentoINMETToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "BaixadorVento.exe"

        '' Create new process to run setup
        'Dim processRun As New Process
        '' Set process props
        'processRun.StartInfo.FileName = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Executavel_baixadordadosINMET\Executavel_baixadordadosINMET\dist\" & ExecutavelFORTRAN
        '' Start the process
        'processRun.Start()
        '' Wait for the process to finish
        'processRun.WaitForExit()


        Try
            ' Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\DownloadTelemetriaANA\DownloadTelemetriaANA\DownloadTelemetriaANA\bin\x64\Release\" & ExecutavelFORTRAN)
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Vento\plugin\dist\" & ExecutavelFORTRAN)

        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Private Sub btLUPA_Click(sender As Object, e As EventArgs) Handles btLUPA.Click
        ' Desliga modos especiais e ativa o retângulo de zoom
        identifyMode = False
        miniPickMode = False
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.ZoomWindow

        If lblCoord IsNot Nothing Then
            lblCoord.Text = "Ferramenta: Zoom Janela (clique e arraste no mapa)"
        End If

        ' (se você usa marcação de menu ativo, pode atualizar aqui)
        ' UpdateCheckedTools(pan:=False, zoomIn:=False, zoomOut:=False, zoomWin:=True, identify:=False)

    End Sub

    Private Sub btMUNDO_Click(sender As Object, e As EventArgs) Handles btMUNDO.Click
        If baseMap Is Nothing Then Return
        If baseMap.Layers IsNot Nothing AndAlso baseMap.Layers.Count > 0 Then
            baseMap.ZoomToExtents()
            mapBox.Refresh()
        End If
    End Sub

    Private Sub btMAO_Click(sender As Object, e As EventArgs) Handles btMAO.Click
        ' Sai de quaisquer modos especiais e ativa Pan
        identifyMode = False
        miniPickMode = False
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.Pan

        ' (opcional) feedback na status bar
        If lblCoord IsNot Nothing Then
            lblCoord.Text = "Ferramenta: Pan"
        End If

        ' (opcional) se você usa marcação de menu ativo, atualize aqui:
        ' UpdateCheckedTools(pan:=True, zoomIn:=False, zoomOut:=False, zoomWin:=False, identify:=False)
    End Sub

    Private Sub btCLOSE_Click(sender As Object, e As EventArgs) Handles btCLOSE.Click
        Me.Close()

    End Sub

    Private Sub CentroDeComandoToolStripMenuItem1_Click(sender As Object, e As EventArgs) Handles CentroDeComandoToolStripMenuItem1.Click
        Dim frm15 As New Form20
        frm15.Show()
    End Sub

    Private Sub RodarEmBateladaToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles RodarEmBateladaToolStripMenuItem.Click
        Me.ENABLE_RESULTS_BUTTONS()
        Dim frmBATLADA As New Form14
        frmBATLADA.Show()
    End Sub

    Private Sub GravarPluviogramaDoTrechoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles GravarPluviogramaDoTrechoToolStripMenuItem.Click

        Me.FALSE_ALL()
        Grava_chuva = True

        miniPickMode = True
        mapBox.ActiveTool = SharpMap.Forms.MapBox.Tools.QueryPoint
        ' (opcional) avise no status bar:
        If lblCoord IsNot Nothing Then lblCoord.Text = "Clique em um shape para capturar o valor de 'Mini'…"
    End Sub

    Private Sub ToolStripMenuItem5_Click(sender As Object, e As EventArgs)

    End Sub

    Private Sub GeometriasToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles GeometriasToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "Form_Editor.exe"
        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Form_Editor\" & ExecutavelFORTRAN)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Private Sub ParâmetrosFísicosToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles ParâmetrosFísicosToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "Form_parcal.exe"
        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\Form_parcal\" & ExecutavelFORTRAN)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Private Sub VisualizaçãoDeHidrogramasToolStripMenuItem_Click_1(sender As Object, e As EventArgs)

    End Sub

    Private Sub IiToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles IiToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "HydrographViewer.exe"

        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\HydrographViewer\dist\" & ExecutavelFORTRAN)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Private Sub InícioRápidoToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles InícioRápidoToolStripMenuItem.Click
        Dim Frm24 As New Form24
        Frm24.Show()
    End Sub

    Private Sub ExportarPrevisãoRodadaManualmenteToolStripMenuItem_Click(sender As Object, e As EventArgs)

    End Sub

    Private Sub ExportarRodadaManualToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles ExportarRodadaManualToolStripMenuItem.Click
        Try
            Me.CARREGA_DIRETÓRIOS()
            Me.CARREGA_INFO_CHUVA_PREVISTA()
            Me.EXPORTA_RESULTADOS("Inercial", ModeloPrev)
            MsgBox("Resultados exportados para a pasta " & Dir_EXPORTA)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try



    End Sub

    Sub EXPORTA_RESULTADOS_OLD(ByVal METODO As String, ByVal MODELO_PREV As String)

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        Dim Diretorio_Previsão As String
        Diretorio_Previsão = Dir_EXPORTA & "\Previsão_" & METODO & "_" & Now.Year & "_" & Now.Month & "_" & Now.Day & "_" & Now.Hour & "_" & MODELO_PREV
        If My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = True Then
            IO.Directory.Delete(Diretorio_Previsão, True)
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        ElseIf My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = False Then
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        End If


        Diretorio_Previsão = Dir_EXPORTA & "\Previsão_" & METODO & "_" & Now.Year & "_" & Now.Month & "_" & Now.Day & "_" & Now.Hour & "_" & MODELO_PREV & "\Output"
        If My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = True Then
            IO.Directory.Delete(Diretorio_Previsão, True)
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        ElseIf My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = False Then
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        End If
        Try 'copia tudo da pasta c:\mgb
            Dim f2() As String = IO.Directory.GetFiles("c:\mgb-hora\Output")
            For i As Integer = 0 To UBound(f2)
                IO.File.Copy(f2(i), Diretorio_Previsão & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta c:/mgb-hora/output para a pasta do projeto")
        End Try


        Diretorio_Previsão = Dir_EXPORTA & "\Previsão_" & METODO & "_" & Now.Year & "_" & Now.Month & "_" & Now.Day & "_" & Now.Hour & "_" & MODELO_PREV & "\Input"
        If My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = True Then
            IO.Directory.Delete(Diretorio_Previsão, True)
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        ElseIf My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = False Then
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        End If
        Try 'copia tudo da pasta c:\mgb
            Dim f2() As String = IO.Directory.GetFiles("c:\mgb-hora\Input")
            For i As Integer = 0 To UBound(f2)
                IO.File.Copy(f2(i), Diretorio_Previsão & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta c:/mgb-hora/Input para a pasta do projeto")
        End Try

    End Sub

    Sub EXPORTA_RESULTADOS(ByVal METODO As String, ByVal MODELO_PREV As String)

        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB"

        Dim Diretorio_Previsão As String
        Diretorio_Previsão = Dir_EXPORTA & "\Previsão_" & METODO & "_" & Now.Year & "_" & Now.Month & "_" & Now.Day & "_" & Now.Hour & "_" & MODELO_PREV
        If My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = True Then
            IO.Directory.Delete(Diretorio_Previsão, True)
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        ElseIf My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = False Then
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        End If


        Diretorio_Previsão = Dir_EXPORTA & "\Previsão_" & METODO & "_" & Now.Year & "_" & Now.Month & "_" & Now.Day & "_" & Now.Hour & "_" & MODELO_PREV & "\Output"
        If My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = True Then
            IO.Directory.Delete(Diretorio_Previsão, True)
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        ElseIf My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = False Then
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        End If
        Try 'copia tudo da pasta .\mgb\output
            'Desligado 27 de agosto de 2026 Dim f2() As String = IO.Directory.GetFiles("c")
            Dim f2() As String = IO.Directory.GetFiles(diretóriodoprojeto & "\Output")

            For i As Integer = 0 To UBound(f2)
                IO.File.Copy(f2(i), Diretorio_Previsão & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta " & diretóriodoprojeto & "\Output" & "para a pasta do projeto")
        End Try


        Diretorio_Previsão = Dir_EXPORTA & "\Previsão_" & METODO & "_" & Now.Year & "_" & Now.Month & "_" & Now.Day & "_" & Now.Hour & "_" & MODELO_PREV & "\Input"
        If My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = True Then
            IO.Directory.Delete(Diretorio_Previsão, True)
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        ElseIf My.Computer.FileSystem.DirectoryExists(Diretorio_Previsão) = False Then
            System.IO.Directory.CreateDirectory(Diretorio_Previsão)
        End If
        Try 'copia tudo da pasta .\mgb\output
            'Desligado 27 de agosto de 2026 Dim f2() As String = IO.Directory.GetFiles("c:\mgb-hora\Input")
            Dim f2() As String = IO.Directory.GetFiles(diretóriodoprojeto & "\Input")

            For i As Integer = 0 To UBound(f2)
                IO.File.Copy(f2(i), Diretorio_Previsão & "\" & System.IO.Path.GetFileName(f2(i)), True)
            Next
        Catch ex As Exception
            MsgBox("Problema ao copiar os arquivos da pasta " & diretóriodoprojeto & "\Input" & "para a pasta do projeto")
        End Try




    End Sub

    Private Sub DeslocarChuvaToolStripMenuItem_Click(sender As Object, e As EventArgs) Handles DeslocarChuvaToolStripMenuItem.Click
        'Nome do modelo a ser rodado
        Dim ExecutavelFORTRAN As String
        ExecutavelFORTRAN = "EditorPrevChuva.exe"

        Try
            Dim pid = Shell(System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Plugins\EditorPrevChuva\" & ExecutavelFORTRAN)
        Catch ex As Exception
            MsgBox(ex.Message)
        End Try
    End Sub

    Sub CARREGA_DIRETÓRIOS() 'Abre o arquivo com as informações dos diretórios e carrega eles

        'Declara as variáveis
        Dim arquivo_est As String 'Nome do arquivo de entrada
        Dim total_estações As Integer 'Número de postos que devem ser baixados
        Dim linha As String 'variável de leitura
        total_estações = 0 'seta o total de estações como zero
        'Define o arquivo de entrada que contém a localização dos postos a serem baixados
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\config.txt"
        'Abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Input)
        'Começa a leitura do arquivo até o final
        Do Until EOF(1)
            linha = LineInput(1)
            total_estações = total_estações + 1
            'Cada linha do arquivo de entrada contém um diretório
            'Lê as linhas em ordem, definindo o diretório dos arquivos
            If total_estações = 1 Then
                DIR_RHAMA_Q = linha
            ElseIf total_estações = 2 Then
                DIR_RHAMA_P = linha
            ElseIf total_estações = 3 Then
                DIR_CPTEC = linha
            ElseIf total_estações = 4 Then
                DIR_LAJEADO = linha
            ElseIf total_estações = 5 Then
                DIR_ESTREITO = linha
                'ElseIf total_estações = 6 Then
                '    DIR_SERRADAMESA = linha
                'A linha 7 é pulada, pois contém a informação de onde devem ser salvos os arquivos
                'ElseIf total_estações = 8 Then
                '    DIR_CANABRAVA = linha
            ElseIf total_estações = 11 Then
                DirPrevAuto(1) = linha
            ElseIf total_estações = 12 Then
                DirPrevAuto(2) = linha
            ElseIf total_estações = 13 Then
                DirPrevAuto(3) = linha
            ElseIf total_estações = 14 Then
                Dir_EXPORTA = linha
            End If
        Loop
        FileClose()

    End Sub

    Sub CARREGA_INFO_CHUVA_PREVISTA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Input)

        Dim linha_agenda As String
        linha_agenda = LineInput(1)
        FileClose()

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Input)
        ModeloPrev = LineInput(1)
        FileClose()

    End Sub

End Class
