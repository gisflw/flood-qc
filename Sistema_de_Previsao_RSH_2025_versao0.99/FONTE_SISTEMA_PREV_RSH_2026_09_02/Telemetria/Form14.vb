Imports ICSharpCode.SharpZipLib

Public Class Form14

    Dim DirPrevAuto(3), Dir_EXPORTA As String

    Private Sub CheckBox1_CheckedChanged(sender As Object, e As EventArgs) Handles cbChuvaObservada.CheckedChanged

    End Sub

    Private Sub btRodar_Click(sender As Object, e As EventArgs) Handles btRodar.Click

        TotalCommander = True

        Me.ESCREVE_ZIP_INFO()

        If cbChuvaObservada.Checked = True Then
            Dim frmPLU As New Form7
            frmPLU.ShowDialog()
        End If

        If cbChuPrev0.Checked = True Then
            Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA(cb_dir.Text)
            Me.CONFIGURA_MODELO_METEO(cbModeloMeteo0.Text)
            Dim frmPREV As New Form21
            frmPREV.ShowDialog()
        End If

        If cbQobs.Checked = True Then
            Dim frmFLU As New Form3
            frmFLU.ShowDialog()
        End If

        If cbUHEs.Checked = True Then
            Dim frmUHE As New Form5
            frmUHE.ShowDialog()
        End If

        If cbRodar.Checked = True Then
            If cbChuPrev0.Checked = True Then
                Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA(cb_dir.Text)
                Me.CONFIGURA_MODELO_METEO(cbModeloMeteo0.Text)
                Dim frmPREV As New Form21
                frmPREV.ShowDialog()

                Me.ESCREVE_MODELO_AUTOMATICO_MC(0, 1)
                Dim frmRODAR As New Form8
                frmRODAR.Show()
                Me.EXPORTA_RESULTADOS("Inercial", cbModeloMeteo0.Text)
            End If
        End If

        If cbRodar.Checked = True Then
            If cbChuPrev1.Checked = True Then
                Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA(cb_dir.Text)
                Me.CONFIGURA_MODELO_METEO(cbModeloMeteo.Text)
                Dim frmPREV As New Form21
                frmPREV.ShowDialog()

                Me.ESCREVE_MODELO_AUTOMATICO_MC(0, 1)
                Dim frmRODAR As New Form8
                frmRODAR.Show()
                Me.EXPORTA_RESULTADOS("Inercial", cbModeloMeteo.Text)
            End If
        End If

        If cbRodar.Checked = True Then
            If cbChuPrev2.Checked = True Then

                Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA(cb_dir2.Text)
                Me.CONFIGURA_MODELO_METEO(cbModeloMeteo2.Text)
                Dim frmPREV As New Form21
                frmPREV.ShowDialog()

                Me.ESCREVE_MODELO_AUTOMATICO_MC(0, 1)
                Dim frmRODAR As New Form8
                frmRODAR.Show()
                Me.EXPORTA_RESULTADOS("Inercial", cbModeloMeteo2.Text)
            End If
        End If

        If cbRodar.Checked = True Then
            If cbChuPrev3.Checked = True Then

                Me.MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA(cb_dir3.Text)
                Me.CONFIGURA_MODELO_METEO(cbModeloMeteo3.Text)
                Dim frmPREV As New Form21
                frmPREV.ShowDialog()

                Me.ESCREVE_MODELO_AUTOMATICO_MC(0, 1)
                Dim frmRODAR As New Form8
                frmRODAR.Show()
                Me.EXPORTA_RESULTADOS("Inercial", cbModeloMeteo3.Text)
            End If
        End If

        MsgBox("Rodada em batelada finalizada.", MsgBoxStyle.Information)

        TotalCommander = False


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
            'Desligado 27 de agosto de 2026 Dim f2() As String = IO.Directory.GetFiles("c:\mgb-hora\Output")
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

    Private Sub Form14_Load(sender As Object, e As EventArgs) Handles MyBase.Load


        Me.CARREGA_DIRETÓRIOS()
        Me.CARREGA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()
        Me.CARREGA_INFO_CHUVA_PREVISTA()
        Me.CARREGA_ZIP_INFO()

    End Sub

    Sub CARREGA_ZIP_INFO()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_zip.txt", OpenMode.Input)

        Dim linha_zip As String
        linha_zip = LineInput(1)
        If linha_zip = 0 Then
            RadioButton1.Checked = True
            RadioButton2.Checked = False
        ElseIf linha_zip = 1 Then
            RadioButton1.Checked = False
            RadioButton2.Checked = True
        Else
            RadioButton1.Checked = True
            RadioButton2.Checked = False
        End If
        FileClose()

    End Sub

    Sub ESCREVE_ZIP_INFO()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "formato_zip.txt", OpenMode.Output)


        If RadioButton1.Checked = True Then
            PrintLine(1, "0")

        ElseIf RadioButton2.Checked = True Then
            PrintLine(1, "1")

        End If

        PrintLine(1, "")
        PrintLine(1, "Options:")
        PrintLine(1, "0 - .dat")
        PrintLine(1, "1 - zipados")

        FileClose()

    End Sub

    Sub CARREGA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA()

        tb_dir_prev_auto.Text = DirPrevAuto(1)
        tb_dir_prev_auto2.Text = DirPrevAuto(2)
        tb_dir_prev_auto3.Text = DirPrevAuto(3)
        tbPastaOutput.Text = Dir_EXPORTA

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
        Dim contador As Integer
        contador = 0
        Do Until EOF(1)
            linha_agenda = LineInput(1)
            contador += 1
            If contador > 3 Then
                cbModeloMeteo.Items.Add(linha_agenda)
                cbModeloMeteo2.Items.Add(linha_agenda)
                cbModeloMeteo3.Items.Add(linha_agenda)
            End If
        Loop

        cbModeloMeteo.Text = cbModeloMeteo.Items(0)
        cbModeloMeteo2.Text = cbModeloMeteo2.Items(1)
        cbModeloMeteo3.Text = cbModeloMeteo3.Items(2)

        FileClose()


    End Sub


    Sub MODIFICA_DIRETORIO_CHUVA_PREVISTA_AUTOMATICA(DiretorioPrev As String)
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "diretorio_chuva_prevista.txt", OpenMode.Output)
        PrintLine(1, DiretorioPrev)
        tb_dir_prev_auto.Text = DirPrevAuto(DiretorioPrev)

        FileClose()

    End Sub


    Sub CONFIGURA_MODELO_METEO(ModeloPrev As String)


        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo e escreve
        FileOpen(1, pasta_exec & "\" & "formato_chuva_prevista.txt", OpenMode.Output)
        PrintLine(1, ModeloPrev)
        PrintLine(1)
        PrintLine(1, "Options:")
        PrintLine(1, "ONS_ETA")
        PrintLine(1, "ONS_GEFS")
        PrintLine(1, "ONS_ECMWF")
        PrintLine(1, "Zero")
        PrintLine(1, "Arquivo")
        FileClose()
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

    Sub ESCREVE_MODELO_AUTOMATICO_MC(ByVal MC As Integer, ByVal Inercial As Integer)
        'Verifica base de dados interna
        Dim arquivo_est As String 'nome do arquivo com as configurações
        'determina o local do arquivo que contém os diretórios
        arquivo_est = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\Modelos.txt"
        'abre o arquivo
        FileOpen(1, arquivo_est, OpenMode.Output)
        'faz a leitura do arquiv, linha por linha
        PrintLine(1, MC)
        PrintLine(1, Inercial)
        PrintLine(1)
        PrintLine(1, "Legenda")
        PrintLine(1, "Primeira linha, roda Fortran MC: 1-sim ou 0-não")
        PrintLine(1, "Segunda linha, roda Fortran IN: 1-sim ou 0-não")

        'Fecha
        FileClose()

    End Sub
End Class