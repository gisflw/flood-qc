Imports MapWindow.Interfaces
Imports MapWinGIS
Imports System.Windows.Forms
Imports Microsoft.VisualBasic
Imports System

Imports PrevisaoChuva 'Programa de chuva prevista
'Imports ChuvaManual_BAESA

' You must change the name of this class to something unique!
Public Class Telemetria
    Implements MapWindow.Interfaces.IPlugin

    Public g_MapWin As MapWindow.Interfaces.IMapWin
    Private g_MenuStack As New Stack(Of String)

    Public ReadOnly Property Name() As String Implements MapWindow.Interfaces.IPlugin.Name
        'This is one of the more important plug-in properties because if it is not set to something then
        'your plug-in will not load at all. This is the name that appears in the Plug-ins menu to identify
        'this plug-in.
        Get
            Return "Sistema MGB-IPH_RSH"
        End Get
    End Property

    Public ReadOnly Property Author() As String Implements MapWindow.Interfaces.IPlugin.Author
        'This is the author of the plug-in.  Licensees recieve a Serial Number that corresponds with
        'this Author name.  It can be a company name, individual, or organization name.
        Get
            Return ""
        End Get
    End Property

    Public ReadOnly Property SerialNumber() As String Implements MapWindow.Interfaces.IPlugin.SerialNumber
        'This is the plug-in serial number. 
        'This is no longer needed, but the property must remain for backward compatibility.
        Get
            Return ""
        End Get
    End Property

    Public ReadOnly Property Description() As String Implements MapWindow.Interfaces.IPlugin.Description
        'This is a description of the plug-in.  It appears in the plug-ins dialog box when a user selects
        'your plug-in.  
        Get
            Return "Plugin de previsão do MGB-IPH. O uso dos resultados gerados com este sistema é de responsabilidade do usuário."
        End Get
    End Property

    Public ReadOnly Property BuildDate() As String Implements MapWindow.Interfaces.IPlugin.BuildDate
        'This is the Build Date for the plug-in.  You can either return a string of a hard-coded date
        'such as "January 1, 2003" or you can use the .NET function below to dynamically obtain the build
        'date of the assembly.
        Get
            Return System.IO.File.GetLastWriteTime(Me.GetType().Assembly.Location)
        End Get
    End Property

    Public ReadOnly Property Version() As String Implements MapWindow.Interfaces.IPlugin.Version
        'This is the version number of the plug-in.  You can either return a hard-coded string
        'such as "1.0.0.1" or you can use the .NET function shown below to dynamically return 
        'the version number from the assembly itself.
        Get
            Return System.Diagnostics.FileVersionInfo.GetVersionInfo(Me.GetType().Assembly.Location).FileVersion
        End Get
    End Property

    Public Sub Initialize(ByVal MapWin As MapWindow.Interfaces.IMapWin, ByVal ParentHandle As Integer) Implements MapWindow.Interfaces.IPlugin.Initialize
        'This event is fired when the user loads your plug-in either through the plug-in dialog 
        'box, or by checkmarking it in the plug-ins menu.  This is where you would add buttons to the
        'tool bar or menu items to the menu.  
        '
        'It is also standard to set a global reference to the IMapWin that is passed through here so that
        'you can access it elsewhere in your project to act on MapWindow.
        g_MapWin = MapWin

        Dim nil As Object
        nil = Nothing

        g_MapWin.ApplicationInfo.ApplicationName = "MGB-IPH"
        g_MapWin.ApplicationInfo.FormIcon = My.Resources.MGB2

        'Adiciona os menus no MapWindow menus

        'Primeiro adiciona um menu e depois os botões
        g_MapWin.Menus.AddMenu("Telemetria", nil, "Sistema MGB-IPH_RSH")
        'g_MapWin.Menus.AddMenu("Download", "Telemetria", nil, "Aquisição de dados")
        g_MapWin.Menus.AddMenu("ChuvaObservada", "Telemetria", nil, "Chuva Observada")
        g_MapWin.Menus.AddMenu("ChuvaPrevista_menu", "Telemetria", nil, "Chuva Prevista")

        'g_MapWin.Menus.AddMenu("ChuvaManual", "ChuvaPrevista_menu", nil, "Manual")
        g_MapWin.Menus.AddMenu("ChuvaSOMAR", "ChuvaPrevista_menu", nil, "Modelos Interpolada txt")
        'g_MapWin.Menus.AddMenu("ChuvaSISMADEM", "ChuvaPrevista_menu", nil, "CPTEC Eta/BRAMS bin")
        'g_MapWin.Menus.AddMenu("ChuvaPrevista", "ChuvaPrevista_menu", nil, "ETA-CPTEC")
        'g_MapWin.Menus.AddMenu("ChuvaBRAMS", "ChuvaPrevista_menu", nil, "BRAMS")
        'g_MapWin.Menus.AddMenu("ChuvaGFS", "ChuvaPrevista_menu", nil, "GFS")
        g_MapWin.Menus.AddMenu("ChuvaTXT", "ChuvaPrevista_menu", nil, "Chuva TXT")
        'g_MapWin.Menus.AddMenu("ChuvaONS", "ChuvaPrevista_menu", nil, "Modelos ONS txt")
        g_MapWin.Menus.AddMenu("VazãoObservada", "Telemetria", nil, "Vazão Observada")
        g_MapWin.Menus.AddMenu("VazãoSubstituída", "Telemetria", nil, "Operação de Reservatórios")
        g_MapWin.Menus.AddMenu("RodarPrevisão", "Telemetria", nil, "Rodar Previsão")
        g_MapWin.Menus.AddMenu("-", "Telemetria", nil, "-")
        g_MapWin.Menus.AddMenu("ÚltimaPrevisão", "Telemetria", nil, "Última Previsão")
        g_MapWin.Menus.AddMenu("Comparar", "ÚltimaPrevisão", nil, "Visualizar Resultados")
        g_MapWin.Menus.AddMenu("CompararLimiar", "ÚltimaPrevisão", nil, "Visualizar Resultados com limiar")
        g_MapWin.Menus.AddMenu("Visualizar", "ÚltimaPrevisão", nil, "Visualizar Pluviogramas")
        'g_MapWin.Menus.AddMenu("Subbacia", "ÚltimaPrevisão", nil, "Pluviograma por Subbacia")
        g_MapWin.Menus.AddMenu("Padrão", "ÚltimaPrevisão", nil, "Hidrograma Padrão Geral")
        g_MapWin.Menus.AddMenu("Padrão01", "ÚltimaPrevisão", nil, "Hidrograma Baixo Uruguai")
        g_MapWin.Menus.AddMenu("Padrão02", "ÚltimaPrevisão", nil, "Hidrograma Médio Uruguai")
        g_MapWin.Menus.AddMenu("Padrão07", "ÚltimaPrevisão", nil, "Hidrograma Alto Uruguai")
        g_MapWin.Menus.AddMenu("Padrão03", "ÚltimaPrevisão", nil, "Hidrograma Ijuí, Ibicuí e Quaraí")
        g_MapWin.Menus.AddMenu("Padrão04", "ÚltimaPrevisão", nil, "Hidrograma Jacuí e Taquari")
        g_MapWin.Menus.AddMenu("Padrão05", "ÚltimaPrevisão", nil, "Hidrograma Sinos, Caí e Gravatai")
        g_MapWin.Menus.AddMenu("Padrão06", "ÚltimaPrevisão", nil, "Hidrograma Negro e Litoral Doce")

        'g_MapWin.Menus.AddMenu("Email", "ÚltimaPrevisão", nil, "Email")
        g_MapWin.Menus.AddMenu("PluHorizonte", "ÚltimaPrevisão", nil, "Chuva no Horizonte de Previsão")
        g_MapWin.Menus.AddMenu("Grava", "ÚltimaPrevisão", nil, "Gravar Hidrograma")
        g_MapWin.Menus.AddMenu("GravaCota", "ÚltimaPrevisão", nil, "Gravar Cotagrama")
        g_MapWin.Menus.AddMenu("Mapa", "ÚltimaPrevisão", nil, "Mapa de limiares de inundação")
        'g_MapWin.Menus.AddMenu("ONS", "ÚltimaPrevisão", nil, "ONS")
        g_MapWin.Menus.AddMenu("--", "Telemetria", nil, "-")
        g_MapWin.Menus.AddMenu("TotalCommander", "Telemetria", nil, "Centro de Comando")
        g_MapWin.Menus.AddMenu("---", "Telemetria", nil, "-")
        g_MapWin.Menus.AddMenu("Dados", "Telemetria", nil, "Preferências...")

        'g_MapWin.Menus.AddMenu("TotalCommander", "Telemetria", nil, "Total Commander")

        Try 'Tenta adicionar os botões como ícones
            ' g_MapWin.Toolbar.AddButton("Comparar", My.Resources.graph)
            'g_MapWin.Toolbar.ButtonItem("Comparar").Tooltip = "Última Previsão: Visualizar Resultado"

            'g_MapWin.Toolbar.AddButton("Visualizar", My.Resources.cloud_rain)
            'g_MapWin.Toolbar.ButtonItem("Visualizar").Tooltip = "Última Previsão: Visualizar Pluviograma"

            'g_MapWin.Toolbar.AddButton("Subbacia", My.Resources.rain)
            'g_MapWin.Toolbar.ButtonItem("Subbacia").Tooltip = "Última Previsão: Pluviograma por Subbacia"

            'g_MapWin.Toolbar.AddButton("Padrão", My.Resources.graph2)
            'g_MapWin.Toolbar.ButtonItem("Padrão").Tooltip = "Última Previsão: Hidrograma Padrão"
        Catch ex As Exception
            'MsgBox("Plugin de previsão carregado com sucesso! Ignore qualquer mensagem acusando erro  durante a inicialização do MapWindow GIS ou do plugin.", MsgBoxStyle.Information, "MGB-IPH")
        End Try


        Try
            'Se não der, tenta adicionar os botões como PNGs
            g_MapWin.Toolbar.AddButton("Comparar", My.Resources.stocks, "Hidrograma")
            g_MapWin.Toolbar.ButtonItem("Comparar").Tooltip = "Última Previsão: Visualizar Resultado"

            g_MapWin.Toolbar.AddButton("CompararLimiar", My.Resources.stocks, "Hidrograma c/ limiar")
            g_MapWin.Toolbar.ButtonItem("CompararLimiar").Tooltip = "Última Previsão: Visualizar Resultado comparando com limiar"

            g_MapWin.Toolbar.AddButton("Visualizar", My.Resources.cloud_rain_icon2, "Pluviograma")
            g_MapWin.Toolbar.ButtonItem("Visualizar").Tooltip = "Última Previsão: Visualizar Pluviograma"

            'g_MapWin.Toolbar.AddButton("Subbacia", My.Resources.rain_icon2, "Chuva Subbacia")
            'g_MapWin.Toolbar.ButtonItem("Subbacia").Tooltip = "Última Previsão: Pluviograma Subbacia"

            g_MapWin.Toolbar.AddButton("Padrão", My.Resources.grafico, "Hidrograma Padrão")
            g_MapWin.Toolbar.ButtonItem("Padrão").Tooltip = "Última Previsão: Hidrograma Padrão"

        Catch ex As Exception

        End Try



        'Hidrogramas dos Resultados
        Resultado = False
        ResultadoLimiares = False
        VisuResultado = False
        ChuvaSUBS = False
        Padrão = False
        Grava = False
        Grava_nível = False
        ONS = False

        'Variável para rodar automaticamente
        TotalCommander = False

        Me.LOAD_AI()

    End Sub

    Public Sub Terminate() Implements MapWindow.Interfaces.IPlugin.Terminate
        'This event is fired when the user unloads your plug-in either through the plug-in dialog 
        'box, or by un-checkmarking it in the plug-ins menu.  This is where you would remove any
        'buttons from the tool bar tool bar or menu items from the menu that you may have added.
        'If you don't do this, then you will leave dangling menus and buttons that don't do anything.
    End Sub

    Public Sub ItemClicked(ByVal ItemName As String, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.ItemClicked
        'This event fires when a menu item or toolbar button is clicked.  So if you added a button or menu
        'on the Initialize event, then this is where you would handle it.

        If ItemName = "Download" Then 'Caso o botão de download for clicado, chama o formuládio de aquisição de dados
            Dim frm1 As New Form1
            frm1.Show()
        ElseIf ItemName = "Dados" Then 'Botão de preferências clicado, chama o formuládio de preferências
            Dim frm2 As New Form2
            frm2.Show()
        ElseIf ItemName = "TotalCommander" Then 'Botão de preferências clicado, chama o formuládio de preferências
            Dim frm15 As New Form20
            frm15.Show()
        ElseIf ItemName = "Email" Then 'Botão de preferências clicado, chama o formuládio de preferências
            Dim frm16 As New Form16
            frm16.Show()
        ElseIf ItemName = "VazãoObservada" Then 'Botão de vazões observadas, chama o formulário de QOBS
            Dim frmqobs As New Form3
            frmqobs.Show()
        ElseIf ItemName = "VazãoSubstituída" Then 'Botão de vazões substituídas, chama o formulário de QSUBS
            Dim frmqsub As New Form5
            frmqsub.Show()
        ElseIf ItemName = "ChuvaObservada" Then 'Botão de chuvas observadas, chama o formulário de Chuva OBS
            Dim frmpluobs As New Form7
            frmpluobs.Show()
        ElseIf ItemName = "RodarPrevisão" Then 'Botão de previsão, chama o formulário de Rodar Previsão
            Dim frmrodprev As New Form8
            frmrodprev.Show()
        ElseIf ItemName = "ChuvaPrevista" Then 'Botão de chuva prevista, chama a DLL de chuva prevista
            Dim frmPrevisao As New FormChuvaPrevisao
            frmPrevisao.NomeProjeto = "Estreito" ' Por enquanto vamos deixar assim por padrão, depois podemos deixar pro próprio usuario definir
            frmPrevisao.Show()
            'ElseIf ItemName = "ChuvaManual" Then 'Botão de chuva prevista, chama a DLL de chuva prevista
            'Dim frmManual As New Form_Entrada_Chuva
            'frmManual.Show()
        ElseIf ItemName = "ChuvaSISMADEM" Then 'Botão de chuva prevista, chama a DLL de chuva prevista do SISMADEM
            Dim frmPrevisaoETA As New ETASismaden.FormChuvaPrevisaoETA
            frmPrevisaoETA.NomeProjeto = "Estreito" 'Por exemplo
            frmPrevisaoETA.Show()
        ElseIf ItemName = "ChuvaBRAMS" Then 'Botão de chuva prevista, chama a DLL de chuva prevista do SISMADEM
            Dim frmPrevisaoETA As New ETASismaden.FormChuvaPrevisaoETA
            frmPrevisaoETA.NomeProjeto = "Estreito" 'Por exemplo
            frmPrevisaoETA.Show()
        ElseIf ItemName = "ChuvaSOMAR" Then 'Botão de chuva prevista, chama a DLL de chuva prevista do SISMADEM
            Dim frmPrevSOMAR As New Form10
            frmPrevSOMAR.Show()
        ElseIf ItemName = "ChuvaGFS" Then 'Botão de chuva prevista, chama a DLL de chuva prevista do SISMADEM
            Dim frmPrevisaoETA As New ETASismaden.FormChuvaPrevisaoETA
            frmPrevisaoETA.NomeProjeto = "Estreito" 'Por exemplo
            frmPrevisaoETA.Show()
        ElseIf ItemName = "ChuvaTXT" Then 'Botão de chuva prevista, chama a DLL de chuva prevista do SISMADEM
            Dim frmPrevisaoTXT As New Form21
            frmPrevisaoTXT.Show()
        ElseIf ItemName = "ChuvaONS" Then 'Botão de chuva prevista, chama a DLL de chuva prevista do SISMADEM
            Dim frmPrevisaoONS As New ChuvaONS.Form_Entrada_ChuvaONS
            frmPrevisaoONS.Show()
        ElseIf ItemName = "Comparar" Then 'Se a intenção por comparar hidrogramas, ele habilita o cursor para tanto
            g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            'MsgBox("Para visualizar o hidrograma clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
            Resultado = True
            ResultadoLimiares = False
            VisuResultado = False
            ChuvaSUBS = False
            Padrão = False
            Grava = False
            ONS = False

        ElseIf ItemName = "Visualizar" Then 'Se a intenção for visualizar hidrogramas, ele habiltia o cursor para tanto
            g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            'MsgBox("Para visualizar o pluviograma clique na minibacia desejada.", MsgBoxStyle.Information, "MGB")
            Resultado = False
            ResultadoLimiares = False
            VisuResultado = True
            ChuvaSUBS = False
            Padrão = False
            Grava = False
            ONS = False
            Grava_nível = False


        ElseIf ItemName = "Subbacia" Then 'Se a intenção for visualizar hidrogramas, ele habiltia o cursor para tanto
            g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            'MsgBox("Para visualizar o pluviograma clique na sub-bacia desejada.", MsgBoxStyle.Information, "MGB")
            Resultado = False
            ResultadoLimiares = False
            VisuResultado = False
            ChuvaSUBS = True
            Padrão = False
            Grava = False
            Grava_nível = False

        ElseIf ItemName = "Padrão" Then 'Se a intenção for visualizar hidrogramas, ele habiltia o cursor para tanto
            ' g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            'MsgBox("Para visualizar o pluviograma, clique na sub-bacia desejada.", MsgBoxStyle.Information, "MGB")
            Resultado = False
            ResultadoLimiares = False
            VisuResultado = False
            ChuvaSUBS = False
            Padrão = True
            Grava_nível = False

            Dim frm4 As New Form4
            frm4.Show()

        ElseIf ItemName = "Padrão01" Then
            OpçãoGrafico = 1
            Dim frm12 As New Form12
            frm12.Show()
        ElseIf ItemName = "Padrão02" Then
            OpçãoGrafico = 2
            Dim frm12 As New Form12
            frm12.Show()
        ElseIf ItemName = "Padrão03" Then
            OpçãoGrafico = 3
            Dim frm12 As New Form12
            frm12.Show()
        ElseIf ItemName = "Padrão04" Then
            OpçãoGrafico = 4
            Dim frm12 As New Form12
            frm12.Show()
        ElseIf ItemName = "Padrão05" Then
            OpçãoGrafico = 5
            Dim frm12 As New Form12
            frm12.Show()
        ElseIf ItemName = "Padrão06" Then
            OpçãoGrafico = 6
            Dim frm12 As New Form12
            frm12.Show()
        ElseIf ItemName = "Padrão07" Then
            OpçãoGrafico = 7
            Dim frm12 As New Form12
            frm12.Show()

        ElseIf ItemName = "PluHorizonte" Then 'Se a intenção for visualizar hidrogramas, ele habiltia o cursor para tanto
            ' g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            'MsgBox("Para visualizar o pluviograma, clique na sub-bacia desejada.", MsgBoxStyle.Information, "MGB")
            MsgBox("A chuva média no horizonte de previsão é " & FormatNumber(Chuva_no_horizonte, 2) & "mm", MsgBoxStyle.Information, "Chuva no Horizonte de Previsão")

        ElseIf ItemName = "Grava" Then 'Se a intenção for gravar hidrogramas, ele habiltia o cursor para tanto
            g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            MsgBox("Para gravar o hidrograma clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
            Resultado = False
            VisuResultado = False
            ResultadoLimiares = False
            ChuvaSUBS = False
            Padrão = False
            Grava = True
            ONS = False
            Grava_nível = False

        ElseIf ItemName = "GravaCota" Then 'Se a intenção for gravar hidrogramas, ele habiltia o cursor para tanto
            g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            MsgBox("Para gravar o cotagrama clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
            Resultado = False
            VisuResultado = False
            ResultadoLimiares = False
            ChuvaSUBS = False
            Padrão = False
            Grava = False
            ONS = False
            Grava_nível = True

        ElseIf ItemName = "ONS" Then 'Se a intenção for gravar hidrogramas, ele habiltia o cursor para tanto
            g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            MsgBox("Para gravar o hidrograma, clique no centróide da minibacia desejada.", MsgBoxStyle.Information, "MGB")
            Resultado = False
            ResultadoLimiares = False
            VisuResultado = False
            ChuvaSUBS = False
            Padrão = False
            Grava = False
            ONS = True
            Grava_nível = False


        ElseIf ItemName = "CompararLimiar" Then 'Se a intenção por comparar hidrogramas, ele habilita o cursor para tanto
            g_MapWin.View.CursorMode = MapWinGIS.tkCursorMode.cmSelection 'coloca o cursor do mouse para modo de seleção
            'MsgBox("Para visualizar o hidrograma clique no trecho de rio desejado.", MsgBoxStyle.Information, "MGB")
            Resultado = False
            ResultadoLimiares = True
            VisuResultado = False
            ChuvaSUBS = False
            Padrão = False
            Grava = False
            ONS = False
            Grava_nível = False



        ElseIf ItemName = "Mapa" Then
            Dim frm18 As New Form18
            frm18.Show()
        End If


    End Sub

    Public Sub LayerRemoved(ByVal Handle As Integer) Implements MapWindow.Interfaces.IPlugin.LayerRemoved
        'This event fires when the user removes a layer from MapWindow.  This is useful to know if your
        'plug-in depends on a particular layer being present. 
    End Sub

    Public Sub LayersAdded(ByVal Layers() As MapWindow.Interfaces.Layer) Implements MapWindow.Interfaces.IPlugin.LayersAdded
        'This event fires when the user adds a layer to MapWindow.  This is useful to know if your
        'plug-in depends on a particular layer being present. Also, if you keep an internal list of 
        'available layers, for example you may be keeping a list of all "point" shapefiles, then you
        'would use this event to know when layers have been added or removed.
    End Sub

    Public Sub LayersCleared() Implements MapWindow.Interfaces.IPlugin.LayersCleared
        'This event fires when the user clears all of the layers from MapWindow.  As with LayersAdded 
        'and LayersRemoved, this is useful to know if your plug-in depends on a particular layer being 
        'present or if you are maintaining your own list of layers.
    End Sub

    Public Sub LayerSelected(ByVal Handle As Integer) Implements MapWindow.Interfaces.IPlugin.LayerSelected
        'This event fires when a user selects a layer in the legend. 
    End Sub

    Public Sub LegendDoubleClick(ByVal Handle As Integer, ByVal Location As MapWindow.Interfaces.ClickLocation, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.LegendDoubleClick
        'This event fires when a user double-clicks a layer in the legend.
    End Sub

    Public Sub LegendMouseDown(ByVal Handle As Integer, ByVal Button As Integer, ByVal Location As MapWindow.Interfaces.ClickLocation, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.LegendMouseDown
        'This event fires when a user holds a mouse button down in the legend.
    End Sub

    Public Sub LegendMouseUp(ByVal Handle As Integer, ByVal Button As Integer, ByVal Location As MapWindow.Interfaces.ClickLocation, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.LegendMouseUp
        'This event fires when a user releases a mouse button in the legend.
    End Sub

    Public Sub MapDragFinished(ByVal Bounds As System.Drawing.Rectangle, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.MapDragFinished
        'If a user drags (ie draws a box) with the mouse on the map, this event fires at completion of the drag
        'and returns a system.drawing.rectangle that has the bounds of the box that was "drawn"
    End Sub

    Public Sub MapExtentsChanged() Implements MapWindow.Interfaces.IPlugin.MapExtentsChanged
        'This event fires any time there is a zoom or pan that changes the extents of the map.
    End Sub

    Public Sub MapMouseDown(ByVal Button As Integer, ByVal Shift As Integer, ByVal x As Integer, ByVal y As Integer, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.MapMouseDown
        'This event fires when the user holds a mouse button down on the map. Note that x and y are returned
        'as screen coordinates (in pixels), not map coordinates.  So if you really need the map coordinates
        'then you need to use g_MapWin.View.PixelToProj()
    End Sub

    Public Sub MapMouseMove(ByVal ScreenX As Integer, ByVal ScreenY As Integer, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.MapMouseMove
        'This event fires when the user moves the mouse over the map. Note that x and y are returned
        'as screen coordinates (in pixels), not map coordinates.  So if you really need the map coordinates
        'then you need to use g_MapWin.View.PixelToProj()
    End Sub

    Public Sub MapMouseUp(ByVal Button As Integer, ByVal Shift As Integer, ByVal x As Integer, ByVal y As Integer, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.MapMouseUp
        'This event fires when the user releases a mouse button down on the map. Note that x and y are returned
        'as screen coordinates (in pixels), not map coordinates.  So if you really need the map coordinates
        'then you need to use g_MapWin.View.PixelToProj()
    End Sub

    Public Sub Message(ByVal msg As String, ByRef Handled As Boolean) Implements MapWindow.Interfaces.IPlugin.Message
        'Plug-ins can communicate with eachother using Messages.  If a message is sent then this event fires.
        'If you know the message is "for you" then you can set Handled=True and then it will not be sent to any
        'other plug-ins.
    End Sub

    Public Sub ProjectLoading(ByVal ProjectFile As String, ByVal SettingsString As String) Implements MapWindow.Interfaces.IPlugin.ProjectLoading
        'When the user opens a project in MapWindow, this event fires.  The ProjectFile is the file name of the
        'project that the user opened (including its path in case that is important for this this plug-in to know).
        'The SettingsString variable contains any string of data that is connected to this plug-in but is stored 
        'on a project level. For example, a plug-in that shows streamflow data might allow the user to set a 
        'separate database for each project (i.e. one database for the upper Missouri River Basin, a different 
        'one for the Lower Colorado Basin.) In this case, the plug-in would store the database name in the 
        'SettingsString of the project. 
    End Sub

    Public Sub ProjectSaving(ByVal ProjectFile As String, ByRef SettingsString As String) Implements MapWindow.Interfaces.IPlugin.ProjectSaving
        'When the user saves a project in MapWindow, this event fires.  The ProjectFile is the file name of the
        'project that the user is saving (including its path in case that is important for this this plug-in to know).
        'The SettingsString variable contains any string of data that is connected to this plug-in but is stored 
        'on a project level. For example, a plug-in that shows streamflow data might allow the user to set a 
        'separate database for each project (i.e. one database for the upper Missouri River Basin, a different 
        'one for the Lower Colorado Basin.) In this case, the plug-in would store the database name in the 
        'SettingsString of the project. 
    End Sub

    Public Sub ShapesSelected(ByVal Handle As Integer, ByVal SelectInfo As MapWindow.Interfaces.SelectInfo) Implements MapWindow.Interfaces.IPlugin.ShapesSelected
        'This event fires when the user selects one or more shapes using the select tool in MapWindow. Handle is the 
        'Layer handle for the shapefile on which shapes were selected. SelectInfo holds information abou the 
        'shapes that were selected. 
        Dim sf As MapWinGIS.Shapefile
        Dim selectedShapeIdx As Integer
        If SelectInfo Is Nothing Then Exit Sub

        'Grab the index of the first of selected shapes (in case multiple shapes selected)
        selectedShapeIdx = SelectInfo(0).ShapeIndex
        'Get the shapefile associated with current layer handle being selected from
        sf = g_MapWin.Layers(Handle).GetObject


        Dim CampoTabAtributos As Integer
        CampoTabAtributos = 6


        'Se for utilizada a vizualização de hidrograma

        If Resultado = True Then

            Dim NumField As Integer
            NumField = CampoTabAtributos 'numero do field que vc quer ler a informacao - ordem na tabela (no taquari antas é 1)
            postoselecionado = sf.CellValue(NumField, selectedShapeIdx) 'indica a linha e coluna selecionada na tabela de atributos
            Dim frmgraph2 As New Form4 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ResultadoLimiares = True Then

            Dim NumField As Integer
            NumField = CampoTabAtributos 'numero do field que vc quer ler a informacao - ordem na tabela (no taquari antas é 1)
            postoselecionado = sf.CellValue(NumField, selectedShapeIdx) 'indica a linha e coluna selecionada na tabela de atributos
            Dim frmgraph2 As New Form4 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf VisuResultado = True Then

            Dim NumField As Integer
            NumField = CampoTabAtributos  'numero do field que vc quer ler a informacao - ordem na tabela
            postoselecionado = sf.CellValue(NumField, selectedShapeIdx) 'indica a linha e coluna selecionada na tabela de atributos
            Dim frmgraph2 As New Form4 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf ChuvaSUBS = True Then

            Dim NumField As Integer
            NumField = CampoTabAtributos  'numero do field que vc quer ler a informacao - ordem na tabela
            postoselecionado = sf.CellValue(NumField, selectedShapeIdx) 'indica a linha e coluna selecionada na tabela de atributos
            Dim frmgraph2 As New Form4 'chama o formulário do gráfico
            frmgraph2.Show() 'chama o gráfico

        ElseIf Grava = True Then

            Dim NumField As Integer
            NumField = CampoTabAtributos  'numero do field que vc quer ler a informacao - ordem na tabela
            postoselecionado = sf.CellValue(NumField, selectedShapeIdx) 'indica a linha e coluna selecionada na tabela de atributos
            SalvaHidrograma()

        ElseIf Grava_nível = True Then

            Dim NumField As Integer
            NumField = CampoTabAtributos  'numero do field que vc quer ler a informacao - ordem na tabela
            postoselecionado = sf.CellValue(NumField, selectedShapeIdx) 'indica a linha e coluna selecionada na tabela de atributos
            SalvaCotagrama()

        ElseIf ONS = True Then

            Dim NumField As Integer
            NumField = CampoTabAtributos  'numero do field que vc quer ler a informacao - ordem na tabela
            postoselecionado = sf.CellValue(NumField, selectedShapeIdx) 'indica a linha e coluna selecionada na tabela de atributos
            SalvaArquivoONS()

        End If

    End Sub

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

            PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "VazãoCalculada" & " " & "VazãoPrevista" & " " & "VazãoObservada")


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
                PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(QPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(VazaoHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))

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

            PrintLine(5, "Dia" & " " & "Mês" & " " & "Ano" & " " & "Hora" & " " & "CotaCalculada" & " " & "CotaPrevista" & " " & "CotaObservada")


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

                PrintLine(5, dia & " " & mes & " " & datacomeço.Year & " " & hora & " " & SubstituirSeparador(CotaPROPHIG(postoselecionado, contadordiamenor)) & " " & SubstituirSeparador(CotaHIG(postoselecionado, contadordiamenor)))  ' & " " & SubstituirSeparador(QOBS(postoselecionado, contadordiamenor)))


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

        AI = Now.Year - 10
        AF = Now.Year + 2

    End Sub

End Class