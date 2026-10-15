Imports System.Windows.Forms

Public Class Form18

    Private Const nMinibacias = 8649 'número de minibacias é constante
    Private NTprev As Integer  'Número de intervalos da previsao
    Private NTprevDia As Integer  'Número de intervalos da previsao (dia)
    Private Qprev(,) As Single 'Variavel das vazões previstas
    Private TRs(,) As Single   'Variavel dos TRs
    Private Usa() As Integer     'Variavel que indica se pinta ou não o shapefile
    Private Matriz(,) As Integer 'Matriz de limiares 
    Private FatorSeg As Single 'Fator de segurança

    Private Sub Form18_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load

        'Carrega os diretórios, que são padronizados

        Dim local_shape As String 'shapefile
        local_shape = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Mapa\Rios_RS_base.shp"
        txtShape.Text = local_shape

        Dim local_input As String 'shapefile
        local_input = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Mapa\Mini_TR.txt"
        txtInput.Text = local_input

        Dim local_output As String 'shapefile
        local_output = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\MGB\Output\QTUDO_PREV.MGB"
        txtPrev.Text = local_output

    End Sub

    '=========================================================================
    'Controlador dos arquivos 
#Region "Dialogos e Botoes"

    Private Sub btShapeBase_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btShapeBase.Click
        Dim op As New OpenFileDialog
        Try
            op.Filter = "Arquivos Shapefile (.shp) |*.shp"
            op.ShowDialog()
            txtShape.Text = op.FileName
        Catch

        End Try
        op = Nothing
    End Sub

    Private Sub btInputFile_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btInputFile.Click
        Dim op As New OpenFileDialog
        Try
            op.Filter = "Arquivo entrada (.txt) |*.txt"
            op.ShowDialog()
            txtInput.Text = op.FileName
        Catch

        End Try
        op = Nothing
    End Sub

    Private Sub btBinPrev_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btBinPrev.Click
        Dim op As New OpenFileDialog
        Try
            op.Filter = "Arquivo binário de previsão (.MGB) |*.MGB"
            op.ShowDialog()
            txtPrev.Text = op.FileName
        Catch

        End Try
        op = Nothing
    End Sub

    Private Sub btGeraShape_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btGeraShape.Click
        Dim mResult As MsgBoxResult = MsgBox("Por favor, verifique se os arquivos foram inseridos corretamente. Deseja continuar?", MsgBoxStyle.Question + MsgBoxStyle.YesNo)
        If mResult = MsgBoxResult.No Then Exit Sub

        Try
            LeQPrevisao()
            LeInput()
            PreencheMatriz()
            LeShapefile()

            MsgBox("Shapefile com Excedência de TRs gerado com sucesso!", MsgBoxStyle.Information + MsgBoxStyle.OkOnly)
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Exclamation)
        End Try
    End Sub


#End Region
    'Fim do controlador de dialogos e botões
    '=========================================================================

    '==============================================================================================
    'Funções começam a partir daqui
#Region "Funcoes"

    Private Sub LeShapefile_OLD()

        Dim sfdOut As New System.Windows.Forms.SaveFileDialog
        sfdOut.Filter = "Shapefile (.shp) |*.shp"
        Dim ArqOut As String
        If sfdOut.ShowDialog = Windows.Forms.DialogResult.OK Then
            ArqOut = sfdOut.FileName
        Else
            Exit Sub
        End If
        Dim NomeArqOut As String
        NomeArqOut = IO.Path.GetFileNameWithoutExtension(ArqOut)
        Dim DirArqOut As String
        DirArqOut = IO.Path.GetDirectoryName(ArqOut)

        Dim dir, NomeArq As String
        NomeArq = IO.Path.GetFileNameWithoutExtension(txtShape.Text)
        dir = IO.Path.GetDirectoryName(txtShape.Text)

        'Copia os arquivos base e sobreescreve o arquivo gerado
        IO.File.Copy(dir & "\" & NomeArq & ".dbf", DirArqOut & "\" & NomeArqOut & ".dbf", True)
        IO.File.Copy(dir & "\" & NomeArq & ".shx", DirArqOut & "\" & NomeArqOut & ".shx", True)
        IO.File.Copy(dir & "\" & NomeArq & ".shp", DirArqOut & "\" & NomeArqOut & ".shp", True)
        IO.File.Copy(dir & "\" & NomeArq & ".prj", DirArqOut & "\" & NomeArqOut & ".prj", True)

        Dim result As Boolean
        Dim sfile = New MapWinGIS.Shapefile
        Dim f As New MapWinGIS.Field
        'result = sfile.Open(tbDrLine.Text) 'abre o shape
        result = sfile.Open(DirArqOut & "\" & NomeArqOut & ".shp") 'abre o shape
        result = sfile.StartEditingShapes(True) 'começa a editar o shape
        f = New MapWinGIS.Field 'cria uma coluna de atributos
        f.Type = MapWinGIS.FieldType.DOUBLE_FIELD 'tipo de dado do atributo
        f.Name = "Flag" 'nome da coluna
        f.Width = 10 'largura

        'Popula tabela de atributos
        Dim nomes_coluna(NTprev) As String
        For i = 1 To NTprev
            nomes_coluna(i) = "horiz" & CStr(i)
        Next

        f = New MapWinGIS.Field 'cria uma coluna de atributos
        f.Type = MapWinGIS.FieldType.DOUBLE_FIELD 'tipo de dado do atributo
        f.Name = "FlagUsa" 'nome da coluna
        f.Width = 3 'largura
        result = sfile.EditInsertField(f, 1)

        For j = 1 To NTprevDia
            f = New MapWinGIS.Field 'cria uma coluna de atributos
            f.Type = MapWinGIS.FieldType.DOUBLE_FIELD 'tipo de dado do atributo
            f.Name = nomes_coluna(j) 'nome da coluna
            f.Width = 10 'largura
            result = sfile.EditInsertField(f, j + 1)
        Next

        For i = 0 To sfile.NumShapes
            result = sfile.EditCellValue(1, i, Usa(sfile.Table.CellValue(0, i))) 'acrescenta o atributo na coluna de atributos
            For j = 1 To NTprevDia
                result = sfile.EditCellValue(j + 1, i, Matriz(sfile.Table.CellValue(0, i), j)) 'acrescenta o atributo na coluna de atributos

            Next
        Next
        result = sfile.StopEditingShapes(True, True) 'para de editar o shape

    End Sub

    Private Sub LeShapefile()

        Dim sfdOut As New System.Windows.Forms.SaveFileDialog
        sfdOut.Filter = "Shapefile (.shp) |*.shp"
        Dim ArqOut As String
        If sfdOut.ShowDialog = Windows.Forms.DialogResult.OK Then
            ArqOut = sfdOut.FileName
        Else
            Exit Sub
        End If
        Dim NomeArqOut As String
        NomeArqOut = IO.Path.GetFileNameWithoutExtension(ArqOut)
        Dim DirArqOut As String
        DirArqOut = IO.Path.GetDirectoryName(ArqOut)

        Dim dir, NomeArq As String
        NomeArq = IO.Path.GetFileNameWithoutExtension(txtShape.Text)
        dir = IO.Path.GetDirectoryName(txtShape.Text)

        'Copia os arquivos base e sobreescreve o arquivo gerado
        IO.File.Copy(dir & "\" & NomeArq & ".dbf", DirArqOut & "\" & NomeArqOut & ".dbf", True)
        IO.File.Copy(dir & "\" & NomeArq & ".shx", DirArqOut & "\" & NomeArqOut & ".shx", True)
        IO.File.Copy(dir & "\" & NomeArq & ".shp", DirArqOut & "\" & NomeArqOut & ".shp", True)
        IO.File.Copy(dir & "\" & NomeArq & ".prj", DirArqOut & "\" & NomeArqOut & ".prj", True)

        Dim result As Boolean
        Dim sfile = New MapWinGIS.Shapefile
        Dim f As New MapWinGIS.Field
        'result = sfile.Open(tbDrLine.Text) 'abre o shape
        result = sfile.Open(DirArqOut & "\" & NomeArqOut & ".shp") 'abre o shape
        result = sfile.StartEditingShapes(True) 'começa a editar o shape
        f = New MapWinGIS.Field 'cria uma coluna de atributos
        f.Type = MapWinGIS.FieldType.DOUBLE_FIELD 'tipo de dado do atributo
        f.Name = "Flag" 'nome da coluna
        f.Width = 10 'largura

        'Popula tabela de atributos
        Dim nomes_coluna(NTprev) As String
        For i = 1 To NTprev
            nomes_coluna(i) = "horiz" & CStr(i)
        Next

        f = New MapWinGIS.Field 'cria uma coluna de atributos
        f.Type = MapWinGIS.FieldType.DOUBLE_FIELD 'tipo de dado do atributo
        f.Name = "FlagUsa" 'nome da coluna
        f.Width = 3 'largura
        result = sfile.EditInsertField(f, 1)

        f = New MapWinGIS.Field 'cria uma coluna de atributos
        f.Type = MapWinGIS.FieldType.DOUBLE_FIELD 'tipo de dado do atributo
        f.Name = "MaxHoriz" 'nome da coluna
        f.Width = 10 'largura
        result = sfile.EditInsertField(f, 2)

        For j = 1 To NTprevDia
            f = New MapWinGIS.Field 'cria uma coluna de atributos
            f.Type = MapWinGIS.FieldType.DOUBLE_FIELD 'tipo de dado do atributo
            f.Name = nomes_coluna(j) 'nome da coluna
            f.Width = 10 'largura
            result = sfile.EditInsertField(f, j + 2)
        Next

        For i = 0 To sfile.NumShapes
            result = sfile.EditCellValue(1, i, Usa(sfile.Table.CellValue(0, i))) 'acrescenta o atributo FlagUsa na coluna de atributos
            result = sfile.EditCellValue(2, i, Matriz(sfile.Table.CellValue(0, i), 0)) 'acrescenta o atributo MaxHoriz na coluna de atributos
            For j = 1 To NTprevDia
                result = sfile.EditCellValue(j + 2, i, Matriz(sfile.Table.CellValue(0, i), j)) 'acrescenta o atributo horiz(j) na coluna de atributos
            Next
        Next
        result = sfile.StopEditingShapes(True, True) 'para de editar o shape

    End Sub


    Private Sub LeInput()

        Using Text As New IO.StreamReader(txtInput.Text)
            'Descarta o cabeçalho
            Text.ReadLine()
            'Usa/ñ usa;TR 2;TR 5;TR 10;TR 25;TR 50            
            ReDim Usa(nMinibacias)
            ReDim TRs(nMinibacias, 5)

            Dim strLine() As String

            For j = 1 To nMinibacias
                strLine = Split(Text.ReadLine, ";")
                Usa(j) = CInt(strLine(0))
                TRs(j, 1) = CSng(strLine(1))
                TRs(j, 2) = CSng(strLine(2))
                TRs(j, 3) = CSng(strLine(3))
                TRs(j, 4) = CSng(strLine(4))
                TRs(j, 5) = CSng(strLine(5))
            Next
            'Termina a leitura do input
        End Using

    End Sub

    Private Sub LeQPrevisao()

        Dim fs As New IO.FileStream(txtPrev.Text, IO.FileMode.Open, IO.FileAccess.Read)
        'Calcula número de intervalos de tempo com base no tamanho do arquivo e minibacias
        Dim nBytes As Long = fs.Length

        NTprev = nBytes / (4 * nMinibacias) '4 pois são números reais no arquivo de previsao        
        NTprevDia = CInt(NTprev / 24)

        'Fator de segurança
        FatorSeg = tbFatorSeg.Text

        ReDim Qprev(NTprev, nMinibacias) 'Redimensiona a variável
        Using bs As New IO.BinaryReader(fs)

            For i = 1 To NTprev
                For j = 1 To nMinibacias
                    Qprev(i, j) = bs.ReadSingle()
                    Qprev(i, j) = Qprev(i, j) * FatorSeg
                Next
            Next
        End Using

        'Termina de ler o arquivo
    End Sub

    Private Sub PreencheMatriz()
        ReDim Matriz(nMinibacias, NTprevDia)

        Dim MaxQ As Single
        Dim maxTR As Integer

        For j = 1 To nMinibacias
            maxTR = 0

            For i = 1 To NTprevDia

                'Pula a minibacia caso ela não esteja marcada para plotar o resultado
                If Usa(j) = 0 Then
                    Matriz(j, i) = 0
                    Matriz(j, 0) = 0
                    Continue For
                End If

                MaxQ = IdentificaMaxQ(j, i) 'Pega a vazão máxima prevista para 'i' dias à frente, para aquela minibacia

                If MaxQ < TRs(j, 1) Then 'TR 2
                    Matriz(j, i) = 0

                ElseIf MaxQ < TRs(j, 2) Then 'TR 5
                    Matriz(j, i) = 1
                    If maxTR < 1 Then maxTR = 1

                ElseIf MaxQ < TRs(j, 3) Then 'TR 10
                    Matriz(j, i) = 2
                    If maxTR < 2 Then maxTR = 2

                ElseIf MaxQ < TRs(j, 4) Then 'TR 25
                    Matriz(j, i) = 3
                    If maxTR < 3 Then maxTR = 3

                ElseIf MaxQ < TRs(j, 5) Then 'TR 50
                    Matriz(j, i) = 4
                    If maxTR < 4 Then maxTR = 4

                Else
                    Matriz(j, i) = 5
                    If maxTR < 5 Then maxTR = 5
                End If

            Next

            Matriz(j, 0) = maxTR 'Atualiza a coluna do maxTR para todo o horizonte de previsão
        Next

    End Sub

    'Acha a vazão máxima para o dia desejado, para uma dada minibacia
    Private Function IdentificaMaxQ(ByVal mini As Integer, ByVal dia As Integer) As Single

        Dim flowMaxDia As Single

        For k = ((dia - 1) * 24) To (dia * 24) - 1

            If flowMaxDia < Qprev(k + 1, mini) Then flowMaxDia = Qprev(k + 1, mini)

        Next
        Return flowMaxDia

    End Function

#End Region
End Class

