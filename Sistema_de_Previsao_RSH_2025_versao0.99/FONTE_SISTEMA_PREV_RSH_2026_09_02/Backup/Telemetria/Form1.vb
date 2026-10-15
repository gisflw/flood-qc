Imports System.Net
Imports System.IO
Imports PluginSIMEPAR
Imports System.Windows.Forms
Imports System.IO.Compression

Imports ICSharpCode.SharpZipLib.Checksums
Imports ICSharpCode.SharpZipLib.Zip
Imports ICSharpCode.SharpZipLib.GZip
Imports System.Text

Public Class Form1

    'Variável que contém o número de postos que devem ser baixados
    Dim total_postos As Integer

    'Geral

    Private Sub btnFtp_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btnFtp.Click
        Me.Download_Manual_Aquisição_intervalo_V2()
        'Me.Download_Manual_Aquisição_intervalo() 'chama a rotina que faz o download de todos os dados dentro do período definido
        ' Me.DOWNLOAD_MANUAL_RHAMA() 'esta é a rotina padrão de download, considerando apenas o primeiro date time picker

    End Sub

    Private Sub DownloadViaFTP(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing
        Dim pastaDestino As String

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        ' ----- Obtem local onde irá salvar o arquivo
        'pastaDestino = My.Computer.FileSystem.CombinePath(pasta_exec, My.Computer.FileSystem.GetName(arquivoFonte))

        pastaDestino = DIR_RHAMA_Q & "\" & My.Computer.FileSystem.GetName(arquivoFonte)

        If  CPTEC = True Then
            pastaDestino = DIR_CPTEC & "\" & My.Computer.FileSystem.GetName(arquivoFonte)
        End If

        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)
            arquivoRequisitado.UsePassive = True
            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0

            'If CPTEC = False Then
            '    System.IO.File.Copy(pastaDestino, DIR_ESTREITO & "\" & System.IO.Path.GetFileName(pastaDestino), True) 'copia o arquivo para a outra pasta de arquivos também
            '    MsgBox("Download finalizado! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
            'End If

        Catch ex As Exception

            'If CPTEC = False Then
            MsgBox("Erro durante a realização do FTP " + vbCrLf + ex.Message)
            'End If

            Return
        End Try




        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

        If System.IO.Path.GetExtension(pastaDestino) = ".gz" Then
            'Descompacta o arquivo que foi baixado
            descompactaArquivo(pastaDestino, System.IO.Path.GetDirectoryName(pastaDestino) & "\" & System.IO.Path.GetFileNameWithoutExtension(pastaDestino))
        End If

    End Sub

    Sub COPIAR_ARQUIVO(ByVal Entrada As String, ByVal Saida As String)
        FileOpen(10, Entrada, OpenMode.Input)
        FileOpen(20, Saida, OpenMode.Output)
        Dim leitor As String
        Do Until EOF(10)
            leitor = LineInput(10)
            PrintLine(20, leitor)
        Loop
        FileClose()
    End Sub

    Private Sub btiniciar_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btiniciar.Click

        Timer1.Enabled = True 'Libera o timer para começar a rodar
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp principal é o dia de hoje
        estreito = True 'como clicamos no botão de estreito, libera a variável de download de estreito
        Label23.ForeColor = Drawing.Color.Green 'indica que estreito está ligado, pintando o label de verde


    End Sub

    Private Sub Timer1_Tick(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Timer1.Tick
        'Essa subrotina é o tick do timer, que está setado a cada 1 (1000ms).
        'A cada tick ele executa essa rotina

        Label2.Text = DateTime.Now.ToLongTimeString 'A cada 1s (1000ms) o label mostra a hora atual

        If GEFS_ONS = True Then
            If Label2.Text.Substring(3, 2) = tbMinDGEFS.Text And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = tbHoraDGEFS.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Me.DOWNLOAD_GEFS_ONS_AUTO() 'Dispara o download  automático
                'Se for meio-dia
            End If
        End If

        If ETA_ONS = True Then
            If Label2.Text.Substring(3, 2) = tbMinDETA40.Text And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = tbHoraDETA40.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Me.DOWNLOAD_ETA40_ONS_AUTO() 'Dispara o download  automático
                'Se for meio-dia
            End If
        End If

        If CPTEC_SISMADEM = True Then 'Se o download do cptec estiver ativado
            'Se for meia-noite
            If Label2.Text.Substring(3, 2) = cbDown1min.Text And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = cbDownload01.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Me.BAIXARCPTEC_SISMADEM_AUTOMATICO() 'Dispara o download  automático
                'Se for meio-dia
            ElseIf Label2.Text.Substring(3, 2) = cbDown2min.Text And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = cbDownload02.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                Me.BAIXARCPTEC_SISMADEM_AUTOMATICO() 'Dispara o download  automático
            End If

            'Às 23horas ele já passa para o próximo dia
            If Label2.Text.Substring(3, 2) = "00" And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = cbVERIFICAnoite.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                dtpSISMA.Value = dtpSISMA.Value.AddDays(1)
                DateTimePicker1.Value = DateTimePicker1.Value.AddDays(1)
            End If

            'Às 01 ele verifica o dia
            If Label2.Text.Substring(3, 2) = "00" And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = cbVERIFICAmanhã.Text Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
                dtpSISMA.Value = Now.Date
                DateTimePicker1.Value = Now.Date
            End If

        End If

        'If estreito = True Then 'Se o download de estreito estiver habilitado
        '    'Se a hora do dtp for a mesma que a hora definida para download
        '    If Label2.Text.Substring(3, 2) = tb_espera.Text And Label2.Text.Substring(6, 2) = "00" Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
        '        Me.DOWNLOAD_AUTOMÁTICO_V2() 'Dispara o download  automático
        '    End If
        'End If

        'If CPTEC = True Then 'Se o download do cptec estiver ativado
        '    'Se for meia-noite
        '    If Label2.Text.Substring(3, 2) = TextBox9.Text And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = "00" Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
        '        Me.DOWNLOAD_AUTOMÁTICO_CPTEC() 'Dispara o download  automático
        '        'Se for meio-dia
        '    ElseIf Label2.Text.Substring(3, 2) = TextBox9.Text And Label2.Text.Substring(6, 2) = "00" And Label2.Text.Substring(0, 2) = "12" Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
        '        Me.DOWNLOAD_AUTOMÁTICO_CPTEC() 'Dispara o download  automático
        '    End If
        'End If

        'If Lajeado = True Then 'Se o download de lajeado estiver ativado e a hora for o horário definido
        '    If Label2.Text.Substring(3, 2) = TextBox12.Text And Label2.Text.Substring(6, 2) = "00" Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
        '        Me.DOWNLOAD_AUTOMÁTICO_LAJEADO() 'Dispara o download  automático
        '    End If
        'End If

        'If UHE_Estreito = True Then 'Se o download de estreito estiver ativado e a hora for o horário definido
        '    If Label2.Text.Substring(3, 2) = TextBox22.Text And Label2.Text.Substring(6, 2) = "00" Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
        '        Me.DOWNLOAD_AUTOMÁTICO_ESTREITO() 'Dispara o download  automático
        '    End If
        'End If


        'If UHE_SerraDaMesa = True Then 'Se o download de serra da mesa estiver ativado e a hora for o horário definido
        '    If Label2.Text.Substring(3, 2) = TextBox27.Text And Label2.Text.Substring(6, 2) = "00" Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
        '        Me.DOWNLOAD_AUTOMÁTICO_SERRADAMESA() 'Dispara o download  automático
        '    End If
        'End If

        'If UHE_CanaBrava = True Then 'Se o download de estreito estiver ativado e a hora for o horário definido
        '    If Label2.Text.Substring(3, 2) = TextBox32.Text And Label2.Text.Substring(6, 2) = "00" Then 'caso a data for um valor de hora redonda, com os minutos pré definidos
        '        Me.DOWNLOAD_AUTOMÁTICO_CANABRAVA() 'Dispara o download  automático
        '    End If
        'End If

    End Sub

    Sub BAIXARCPTEC_SISMADEM_AUTOMATICO()
        'DEFINE O DIA COM DOIS DÍGITOS
        Dim dia, mes As String
        dia = dtpSISMA.Value.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        'DEFINE O MÊS DO ANO COM DOIS DÍGITOS
        mes = dtpSISMA.Value.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If

        'Este textbox contém a parte do nome do arquivo a ser baixado que varia diariamente
        ' TextBox23.Text = dtpSISMA.Value.Year & mes & dia & cbSISMA.Text

        'Parte do nome do arquivo a ser baixado que varia diariamente
        Dim ParteArquivo As String
        ParteArquivo = dtpSISMA.Value.Year & mes & dia & cbSISMA.Text

        'ser adquiridas via FTP do CPTEC
        Me.Text = "Fazendo download das estações do SISMADEM no ftp do CPTEC..."

        'Define a pasta onde está salvando
        CPTEC = True

        'variável com o nome do arquivo
        Dim arquivoFonte As String

        'Baixa o dado de chuva de 50mb
        arquivoFonte = tbEndSISMA.Text & cbSISMAmodel.Text & ParteArquivo & ".bin.gz"

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP_automático_CPTEC(arquivoFonte, "", "")

        Try
            'Faz uma cópia do arquivo com outro nome padronizado
            System.IO.File.Copy(DIR_CPTEC & "\" & cbSISMAmodel.Text & ParteArquivo & ".bin", DIR_CPTEC & "\" & cbSISMAmodel.Text & ".bin", True)
        Catch ex As Exception

        End Try

        'Baixa o .ctl
        arquivoFonte = tbEndSISMA.Text & cbSISMAmodel.Text & ".ctl"

        'efetuar o download via FTP anônimo
        DownloadViaFTP_automático_CPTEC(arquivoFonte, "", "")

        'Faz uma cópia do arquivo ctl com outro nome
        System.IO.File.Copy(DIR_CPTEC & "\" & cbSISMAmodel.Text & ".ctl", DIR_CPTEC & "\" & ParteArquivo & "_" & cbSISMAmodel.Text & ".ctl", True)

        'Fecha o trabalho com o CPTEC
        'CPTEC_SISMADEM = False
        CPTEC = False

        'Fecha o arquivo que contém o nome
        FileClose()

        'MsgBox("Download das estações do SISMADEM no ftp do CPTEC realizado com sucesso!")
        'Me.Text = "Aquisição de dados"

    End Sub

    Private Sub btparar_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles btparar.Click
        'Para o Download e Estreito
        estreito = False
        Label23.ForeColor = Drawing.Color.Red

    End Sub

    Private Sub Form1_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        'Control.CheckForIllegalCrossThreadCalls = False

        'Quanto o formulário é carregado, ele declara todas as variáveis que controlam o download automático como falsas
        estreito = False
        CPTEC = False
        CPTEC_SISMADEM = False
        Lajeado = False
        UHE_Estreito = False
        UHE_SerraDaMesa = False
        UHE_CanaBrava = False
        GEFS_ONS = False
        ETA_ONS = False
        '-

        'Chama a rotina que carrega o arquivo texto que contém os diretórios de onde os dados devem ser salvos
        Me.CARREGA_DIRETÓRIOS()
        '-Diretórios carregados

        'Carrega os Horários
        Me.AGENDA()

        Me.ACERTA_SIMEPAR() 'acerta date time pickers simepar
    End Sub

    Sub AGENDA()
        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Agenda.txt", OpenMode.Input)

        Dim linha_agenda As String
        Dim splt

        Do Until EOF(1)
            linha_agenda = LineInput(1)
            splt = Split(linha_agenda, ":")
            If splt(0) = "Download01_ETA_SISMADEM" Then
                cbDownload01.Text = splt(1)
                cbDown1min.Text = splt(2)
                'atualiza o GEFS com o mesmo horário do ETA, por que não vai usar os dois ao mesmo tempo!
                tbHoraDGEFS.Text = splt(1)
                tbMinDGEFS.Text = splt(2)
            ElseIf splt(0) = "Download_Garibaldi" Then
                TextBox12.Text = splt(1)
            ElseIf splt(0) = "Download_BarraGrande" Then
                TextBox22.Text = splt(1)
            ElseIf splt(0) = "Download_CamposNovos" Then
                TextBox27.Text = splt(1)
            End If
        Loop
        FileClose()
    End Sub

    Sub DOWNLOAD_AUTOMÁTICO()

        Me.Text = "Fazendo download..."

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações.txt", OpenMode.Input)


        'Abre o arquivo de log
        'FileOpen(2, pasta_exec & "\" & DateTimePicker1.Value.Year & "." & DateTimePicker1.Value.Month & "." & DateTimePicker1.Value.Day & "." & Label2.Text.Substring(0, 2) & "_N" & "log.txt", OpenMode.Output)
        FileOpen(2, pasta_exec & "\" & "log.txt", OpenMode.Output)

        'Variável de leitura
        Dim linha As String

        'Acerta a barra de progresso
        PBar.Visible = True
        PBar.Maximum = 10
        PBar.Value = 0


        'Faz o download dos arquivos individualmente
        Do Until EOF(1)
            linha = LineInput(1)

            PBar.Increment(1)

            Dim arquivoFonte As String 'arquivo que será baixado

            Dim dia, mes, hora As String
            dia = DateTimePicker1.Value.Day
            If dia.Length < 2 Then
                dia = "0" & dia
            End If

            mes = DateTimePicker1.Value.Month
            If mes.Length < 2 Then
                mes = "0" & mes
            End If

            hora = Label2.Text.Substring(0, 2)
            If hora.Length < 2 Then
                hora = "0" & hora
            End If

            'informa o endereço do arquivo de cotas
            arquivoFonte = txtarq_auto.Text & DateTimePicker1.Value.Year & "." & mes & "." & dia & "." & hora & "_N" & linha & ".txt"

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP_automático(arquivoFonte, tb_usuário.Text, tb_senha.Text)

            'informa o endereço do arquivo de chuva
            arquivoFonte = txtarq_auto.Text & DateTimePicker1.Value.Year & "." & mes & "." & dia & "." & hora & "_P" & linha & ".txt"

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP_automático(arquivoFonte, tb_usuário.Text, tb_senha.Text)

        Loop

        FileClose()

        PBar.Visible = False

        Me.Text = "Aquisição de dados"


    End Sub 'Substituída pela V2, não está sendo utilizada

    Sub DOWNLOAD_MANUAL_FTP() 'Não está sendo usado, é a forma mais simples de download manual de um arquivo quando o botão é clicado
        Dim arquivoFonte As String

        arquivoFonte = txtArq.Text

        If (arquivoFonte = "") Or arquivoFonte = String.Empty Then
            MsgBox("Informe a URL e o nome do arquivo para download.")
            Return
        End If

        estreito = True

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP(arquivoFonte, "estreito", "estreito2011")

        estreito = False

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
            ElseIf total_estações = 6 Then
                DIR_SERRADAMESA = linha
                'A linha 7 é pulada, pois contém a informação de onde devem ser salvos os arquivos
            ElseIf total_estações = 8 Then
                DIR_CANABRAVA = linha
            End If
        Loop
        FileClose()

    End Sub

    '-Fim das subrotinas mais gerais

    '--Estreito RHAMA

    Sub DOWNLOAD_AUTOMÁTICO_V2()

        Me.Text = "Fazendo download..."

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações.txt", OpenMode.Input)


        'Abre o arquivo de log
        'FileOpen(2, pasta_exec & "\" & DateTimePicker1.Value.Year & "." & DateTimePicker1.Value.Month & "." & DateTimePicker1.Value.Day & "." & Label2.Text.Substring(0, 2) & "_N" & "log.txt", OpenMode.Output)
        FileOpen(2, pasta_exec & "\" & "log.txt", OpenMode.Output)

        'Variável de leitura
        Dim linha As String

        'Acerta a barra de progresso
        PBar.Visible = True
        PBar.Maximum = 20
        PBar.Value = 0

        total_postos = 0

        'Faz o download dos arquivos individualmente
        Do Until EOF(1)
            linha = LineInput(1)
            PBar.Increment(1)
            Dim arquivoFonte As String 'arquivo que será baixado
            total_postos = total_postos + 1

            'informa o endereço do arquivo de cotas
            arquivoFonte = txtarq_auto.Text & linha & ".INPE.txt"

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP_automático_V2(arquivoFonte, tb_usuário.Text, tb_senha.Text)

        Loop

        FileClose()

        'PBar.Visible = False

        Me.Text = "Aquisição de dados"

        Me.CONVERTE_DOWNLOADS()


    End Sub

    Private Sub DownloadViaFTP_automático(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Substituída pela V2
        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação

        'Declara as variáveis que são importantes para o download no FTP
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest 'Nome do arquivo que deve ser baixado
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing 'Nome do arquivo que deve ser salvo
        Dim pastaDestino As String 'pasta onde o arquivo deve ser salvo

        Dim app_path As String 'Local onde o programa está sendo executado
        app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        Dim arqfonte_name As String 'nome do arquivo que deve ser baixado
        arqfonte_name = System.IO.Path.GetFileName(arquivoFonte)

        ' ----- Obtem local onde irá salvar o arquivo
        'O local depende se estamos trabalhando com postos Plu ou Flu
        If RHAMA_Flu = True Then
            pastaDestino = DIR_RHAMA_Q & "\" & arqfonte_name
        ElseIf RHAMA_Plu = True Then
            pastaDestino = DIR_RHAMA_P & "\" & arqfonte_name
        End If

        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)

            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0
            PrintLine(2, System.IO.Path.GetFileName(arquivoFonte) & " OK")
        Catch ex As Exception
            PrintLine(2, System.IO.Path.GetFileName(arquivoFonte) & " ERRO")
            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

    End Sub 'Substituída pela V2

    Private Sub DownloadViaFTP_automático_V2(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação

        'Declara as variáveis que são importantes para o download no FTP
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest 'Nome do arquivo que deve ser baixado
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing 'Nome do arquivo que deve ser salvo
        Dim pastaDestino As String 'pasta onde o arquivo deve ser salvo

        Dim app_path As String 'Local onde o programa está sendo executado
        app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        Dim arqfonte_name As String 'nome do arquivo que deve ser baixado
        arqfonte_name = System.IO.Path.GetFileName(arquivoFonte)

        ' ----- Obtem local onde irá salvar o arquivo
        'O local depende se estamos trabalhando com postos Plu ou Flu
        pastaDestino = app_path & "\Estreito_RHAMA\" & arqfonte_name

        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)

            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0
            PrintLine(2, System.IO.Path.GetFileName(arquivoFonte) & " OK")
        Catch ex As Exception
            PrintLine(2, System.IO.Path.GetFileName(arquivoFonte) & " ERRO")
            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

    End Sub

    Sub Download_Manual_Aquisição_intervalo() 'Prepara o download de dados do FTP dentro do intervalo definido

        'define a data inicial e final de download, para a aquisição dos dados
        Dim dataO As Date = dtp_RHAMA.Value.Day & "/" & dtp_RHAMA.Value.Month & "/" & dtp_RHAMA.Value.Year & " " & cb_hora_RHAMA.Text & ":00:00"
        Dim dataE As Date = dtp_RHAMA2.Value.Day & "/" & dtp_RHAMA2.Value.Month & "/" & dtp_RHAMA2.Value.Year & " " & cb_hora_RHAMA2.Text & ":00:00"

        'número de dias dentro do intervalo
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        Dim datacomeço As Date 'define a data inicial da interpolação
        datacomeço = dtp_RHAMA.Value.Day & "/" & dtp_RHAMA.Value.Month & "/" & dtp_RHAMA.Value.Year & " " & cb_hora_RHAMA.Text & ":00:00"
        'data inicial igual ao primeiro dia do intervalo

        'diminui uma hora da data incial, pois no próximo loop ele vai adicionar uma hora a cada rodada
        'desta forma, na primeira rodada a data do começo será a data inicial
        datacomeço = datacomeço.AddHours(-1)

        'começa o loop de download
        For t As Integer = 1 To tmaxx 'Para cada intervalo de tempo

            'incrementa uma hora na data
            datacomeço = datacomeço.AddHours(1)

            'muda o nome da tela
            Me.Text = "Fazendo download..."

            ' ----- Obtem local onde irá salvar o arquivo
            Dim pasta_exec As String
            pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

            'Abre o arquivo de estações
            FileOpen(1, pasta_exec & "\" & "Estações.txt", OpenMode.Input)

            'Abre o arquivo de log
            'FileOpen(2, pasta_exec & "\" & datacomeço.Year & "." & datacomeço.Month & "." & datacomeço.Day & "." & datacomeço.Hour & "_N" & "log.txt", OpenMode.Output)
            FileOpen(2, pasta_exec & "\" & "log.txt", OpenMode.Output)


            'Variável de leitura
            Dim linha As String

            'Acerta a barra de progresso
            PBar.Visible = True
            PBar.Maximum = 10
            PBar.Value = 0


            'Faz o download dos arquivos individualmente
            Do Until EOF(1)
                linha = LineInput(1)

                PBar.Increment(1)

                Dim arquivoFonte As String 'arquivo que será baixado

                'Abaixo, acerta os valores de dia, mes e hora para eles sempre serem strings com length = 2
                Dim dia, mes, hora As String
                dia = datacomeço.Day
                If dia.Length < 2 Then
                    dia = "0" & dia
                End If

                mes = datacomeço.Month
                If mes.Length < 2 Then
                    mes = "0" & mes
                End If

                hora = datacomeço.Hour
                If hora.Length < 2 Then
                    hora = "0" & hora
                End If
                '- Acertou os strings para terem length = 2

                'Define as variáveis que diferenciam dados flu e plu para download dos dados flu
                RHAMA_Flu = True
                RHAMA_Plu = False

                'informa o endereço do arquivo de cotas
                arquivoFonte = txtarq_auto.Text & datacomeço.Year & "." & mes & "." & dia & "." & hora & "_N" & linha & ".txt"

                'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
                'efetuar o download via FTP anônimo
                DownloadViaFTP_automático(arquivoFonte, tb_usuário.Text, tb_senha.Text)

                'Define as variáveis que diferenciam dados flu e plu para download dos dados flu
                RHAMA_Flu = False
                RHAMA_Plu = True

                'informa o endereço do arquivo de chuva
                arquivoFonte = txtarq_auto.Text & datacomeço.Year & "." & mes & "." & dia & "." & hora & "_P" & linha & ".txt"

                'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
                'efetuar o download via FTP anônimo
                DownloadViaFTP_automático(arquivoFonte, tb_usuário.Text, tb_senha.Text)

                'Define as variáveis que diferenciam dados flu e plu 
                RHAMA_Flu = False
                RHAMA_Plu = False

            Loop

            FileClose()

            PBar.Visible = False

            Me.Text = "Aquisição de dados"





        Next




    End Sub 'Substituída pela V2

    Sub Download_Manual_Aquisição_intervalo_V2() 'Prepara o download de dados do FTP dentro do intervalo definido

        'define a data inicial e final de download, para a aquisição dos dados
        Dim dataO As Date = dtp_RHAMA.Value.Day & "/" & dtp_RHAMA.Value.Month & "/" & dtp_RHAMA.Value.Year & " " & cb_hora_RHAMA.Text & ":00:00"
        Dim dataE As Date = dtp_RHAMA2.Value.Day & "/" & dtp_RHAMA2.Value.Month & "/" & dtp_RHAMA2.Value.Year & " " & cb_hora_RHAMA2.Text & ":00:00"

        'número de dias dentro do intervalo
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        Dim datacomeço As Date 'define a data inicial da interpolação
        datacomeço = dtp_RHAMA.Value.Day & "/" & dtp_RHAMA.Value.Month & "/" & dtp_RHAMA.Value.Year & " " & cb_hora_RHAMA.Text & ":00:00"
        'data inicial igual ao primeiro dia do intervalo

        'diminui uma hora da data incial, pois no próximo loop ele vai adicionar uma hora a cada rodada
        'desta forma, na primeira rodada a data do começo será a data inicial
        datacomeço = datacomeço.AddHours(-1)

        'começa o loop de download


        'muda o nome da tela
        Me.Text = "Fazendo download..."

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações.txt", OpenMode.Input)

        'Abre o arquivo de log
        'FileOpen(2, pasta_exec & "\" & datacomeço.Year & "." & datacomeço.Month & "." & datacomeço.Day & "." & datacomeço.Hour & "_N" & "log.txt", OpenMode.Output)
        FileOpen(2, pasta_exec & "\" & "log.txt", OpenMode.Output)


        'Variável de leitura
        Dim linha As String

        'Acerta a barra de progresso
        PBar.Visible = True
        PBar.Maximum = 20
        PBar.Value = 0

        total_postos = 0

        'Faz o download dos arquivos individualmente
        Do Until EOF(1)
            linha = LineInput(1)
            PBar.Increment(1)
            Dim arquivoFonte As String 'arquivo que será baixado
            total_postos = total_postos + 1

            'informa o endereço do arquivo de cotas
            arquivoFonte = txtarq_auto.Text & linha & ".INPE.txt"

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP_automático_V2(arquivoFonte, tb_usuário.Text, tb_senha.Text)

        Loop

        FileClose()

        'PBar.Visible = False

        Me.Text = "Aquisição de dados"

        Me.CONVERTE_DOWNLOADS()

        MsgBox("Download dos arquivos finalizado.", MsgBoxStyle.Information, "Aquisição de dados telemétricos")

    End Sub

    Sub CONVERTE_DOWNLOADS() 'Converte os dados baixados para a pasta padrão

        'Variável que contém o número de postos a serem baixados
        Dim numpostos As Integer
        numpostos = total_postos 'define ela como sendo igual ao número de linhas do arquivo de estações

        'Variáveis necessárias para dimensionamento
        Dim hh As Integer 'hora
        Dim min As Integer 'minutos
        Dim mes As Integer 'meses
        Dim dia As Integer 'dias
        Dim year As Integer 'anos

        'Define os valores das variáveis
        hh = 24
        min = 45
        mes = 12
        dia = 31
        year = AF


        'Dimensiona as variáveis de chuva e nível, que vão conter os resultados
        'Dim chuva_min(numpostos, min, hh, dia, mes, Ano(year)) As Double
        Dim chuva_hor(numpostos, hh, dia, mes, Ano(year)) As Double
        'Dim nivel_min(numpostos, min, hh, dia, mes, Ano(year)) As Double
        Dim nivel_hor(numpostos, hh, dia, mes, Ano(year)) As Double

        'Zera as variáveis 
        For p As Integer = 0 To numpostos
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For h As Integer = 0 To 23
                            For minu As Integer = 1 To 45
                                'chuva_min(p, minu, h, dd, mm, Ano(aaaa)) = -1.0
                                chuva_hor(p, h, dd, mm, Ano(aaaa)) = -1.0
                                'nivel_min(p, minu, h, dd, mm, Ano(aaaa)) = -1.0
                                nivel_hor(p, h, dd, mm, Ano(aaaa)) = -1.0
                            Next
                        Next
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        'Dimensiona algumas variáveis de leitura que são importantes
        Dim arq(numpostos) As String 'nome dos postos que serão utilizados
        Dim posto As Integer 'número do posto que está sendo usado
        posto = 0 'o número do posto começa como zero, e vai aumentando no contador

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        'que é a pasta de execução do plugin

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações.txt", OpenMode.Input)


        'Variável de leitura
        Dim linha As String

        'Acerta a barra de progresso
        'PBar.Visible = True
        'PBar.Maximum = 20
        'PBar.Value = 0


        'Faz o download dos arquivos individualmente
        Do Until EOF(1) 'faz um loop até o final do arquivo
            'define a variável de leituda como sento o input da linha do arquivo
            linha = LineInput(1)
            'aumenta um ponto na barra de progressp
            PBar.Increment(1)
            'Dimensiona a variável que contém o nome do arquivo da estação que será feita a leitura
            Dim arquivoFonte As String 'arquivo que será transformado
            'informa o endereço do arquivo
            arquivoFonte = pasta_exec & "\Estreito_RHAMA\" & linha & ".INPE.txt"

            'Redefine as variávies que servem para dimensionar os postos
            hh = 24
            min = 45
            mes = 12
            dia = 31
            year = AF

            'Aumenta 1 na variável de contagem dos postos
            'cada linha do arquivo de estações tem a informação de um posto
            posto = posto + 1
            'coloca o nome do posto que está na posição 1 como sendo o nome do arquivo que será aberto
            arq(posto) = arquivoFonte

            'Abre o arquivo de interesse
            FileOpen(4, arq(posto), OpenMode.Input)

            'Cria outra variável de leitura
            Dim linha4 As String
            'Cria um contador de linhas do arquivo de interesse
            Dim cont_linha As Integer
            'Define que o valor inicial do contador é -1
            cont_linha = -1

            'Esta variável simboliza a chuva acumulada em cada mes, ela é definida neste momento
            Dim plu_acum_mes(mes) As Double


            'Carrega os valores da variável de chuva acumulada, inicialmente ela é zero para todos os meses
            For m As Integer = 0 To 12
                plu_acum_mes(m) = 0
            Next

            'Essa variável indica o nome do arquivo que será gerado após a conversão
            Dim data_fim_nome As String

            'Variáveis utilizadas no cálculo da média dos dados de vazão
            Dim acumflu As Double = 0 'soma dos níveis
            Dim flumed As Double = 0 'nível médio
            Dim cont_acum As Integer = 0 'contador de quantos minutos temos dados


            'Começa a leitura completa do arquivo que contém os dados da estação
            Do Until EOF(4)
                'Lê linha por linha, guardando os resultados na variável linha4
                linha4 = LineInput(4)
                'adiciona 1 ao contador a cada linha
                cont_linha = cont_linha + 1
                'A primeira linha é o cabeçalho, então começa a leitura a partir do contador = 1
                If cont_linha >= 1 Then
                    'lê as variáveis hora, minuto, mes, dia e ano da linha do arquivo
                    hh = linha4.Substring(11, 2)
                    min = linha4.Substring(14, 2)
                    mes = linha4.Substring(3, 2)
                    dia = linha4.Substring(0, 2)
                    year = linha4.Substring(6, 4)

                    'cria estas daus variáveis que correspondem aos valores de chuva e vazão, contidos no arquivo
                    Dim plu As Double
                    Dim flu As Double

                    'Cria esta variável que é o delimitador da linha
                    Dim splt
                    'a linha é delimitada por uma vírgula
                    splt = Split(linha4, ",")
                    ' e a chuva é o segundo valor no delimitador (o primeiro é o zero)
                    Try 'Verifica se existe algum valor escrito
                        plu = splt(1) 'Se existe, pega o valor
                    Catch ex As Exception 'Se não existe
                        plu = -1.0 'coloca -1.00
                    End Try

                    'O nível é o terceiro valor no delimitador, mas os dados de nível nem sempre existem
                    'Então usamos um Try para tentar ler eles
                    'Caso eles não existem, colocamos o valor -1.00
                    Try
                        flu = splt(2)
                    Catch ex As Exception
                        flu = -1.0
                    End Try

                    'Calculamos os dados de nível na forma de uma média dos quatro dados de 15 minutos
                    'A seguinte sequência vai somando os dados, para calcular a média na hora final
                    If min = 15 Then 'soma no minuto 15
                        If flu > 0 Then 'somente soma se existir dado de vazão
                            acumflu = acumflu + flu
                            cont_acum = cont_acum + 1
                        End If
                    ElseIf min = 30 Then 'soma no minuto 30
                        If flu > 0 Then 'somente soma se existir dado de vazão
                            acumflu = acumflu + flu
                            cont_acum = cont_acum + 1
                        End If
                    ElseIf min = 45 Then 'soma no minuto 45
                        If flu > 0 Then 'somente soma se existir dado de vazão
                            acumflu = acumflu + flu
                            cont_acum = cont_acum + 1
                        End If
                    ElseIf min = 0 Then 'soma no minuto zero
                        If flu > 0 Then 'somente soma se existir dado de vazão
                            acumflu = acumflu + flu
                            cont_acum = cont_acum + 1
                        End If
                        'Faz a conta do nível médio
                        If cont_acum > 0 Then 'só calcula o nível caso existam dados em pelo menos um dos 15min
                            flumed = acumflu / cont_acum
                        ElseIf cont_acum = 0 Then 'caso não existam dados, colocamos falha
                            flumed = -1.0
                        End If
                        acumflu = 0 'como a chuva média foi calculada, zeramos as variáveis de contagem
                        cont_acum = 0 'como a chuva média foi calculada, zeramos as variáveis de contagem
                    End If




                    'chuva_min(posto, min, hh, dia, mes, Ano(year)) = plu
                    'nivel_min(posto, min, hh, dia, mes, Ano(year)) = flu

                    'Caso a linha que estamos lendo seja a primeira linha do arquivo
                    'Já definimos a nossa chuva acumulada como sendo esta primeira linha
                    If cont_linha = 1 Then
                        If plu < 0 Then 'mas somente fazemos isso caso o primeiro dado não seja falha
                            plu = 0 'caso ele seja falha, assumimos como sendo igual a zero
                        End If
                        plu_acum_mes(mes) = plu
                    End If

                    'Caso a linha que está sendo lida seja a primeira linha do mês, ou seja, caso o dia seja = 1
                    'Definimos que a chuva acumulada do mes é igual a zero
                    If dia = 1 Then
                        If plu < 0 Then 'Mas, se o dado for falha, sabemos que a chuva é zero no começo do mes
                            plu = 0
                        End If
                        plu_acum_mes(mes) = plu
                    End If

                    'Agora o programa vai ler a linha, e só vai gravar os valores de chuva caso estes sejam
                    'os valores do minuto 00, ou seja, de hora cheia
                    If min = 0 Then
                        'O valor de chuva é o valor medido, menos o acumulado na última hora
                        If plu >= 0 Then 'contudo, somente podemos fazer isso se o dado não for falha
                            chuva_hor(posto, hh, dia, mes, Ano(year)) = plu - plu_acum_mes(mes)
                        ElseIf plu < 0 Then 'caso o dado for falha, a chuva terá que ser falha
                            chuva_hor(posto, hh, dia, mes, Ano(year)) = -1.0
                        End If
                        'O valor de nível é o valor médio calculado
                        nivel_hor(posto, hh, dia, mes, Ano(year)) = flumed
                        flumed = 0 'Zeramos o valor médio depois de preenchermos a variável de nível

                        'Caso o nosso valor for maior que o acumulado (ou seja, a chuva não foi zero)
                        'definimos o novo acumulado
                        If plu > plu_acum_mes(mes) Then
                            plu_acum_mes(mes) = plu
                        End If
                    End If



                    'Define o nome do arquivo gerado como o último dia com dados
                    Dim ddd, hhh, mmm As String
                    ddd = dia
                    If ddd.ToString.Length < 2 Then
                        ddd = "0" & ddd
                    End If

                    mmm = mes
                    If mmm.ToString.Length < 2 Then
                        mmm = "0" & mmm
                    End If

                    hhh = hh
                    If hhh.ToString.Length < 2 Then
                        hhh = "0" & hhh
                    End If

                    data_fim_nome = year & "." & mmm & "." & ddd & "." & hhh & "_"
                    'Notar que este valor vai variando até o final do arquivo

                End If

            Loop

            'Após o término do loop de leitura do arquivo da estação, fecha o arquivo
            FileClose(4)

            If cont_linha >= 1 Then 'só executa as rotinas de gravar os arquivos, caso tenha lido algo

                'Define uma variável que corresponde à data final do arquivo que foi lido
                Dim data As Date
                data = dia & "/" & mes & "/" & year & " " & hh & ":00:00"

                'e uma variável que corresponde ao começo do arquivo que foi lido
                Dim datacomeço As Date
                'sempre vamos gravar um arquivo com antecedência de 4 dias
                datacomeço = data.AddDays(-7)

                'descobre o código doposto, para salvar os arquivos nas pastas corretas
                Dim cod_pos As String
                'O código do posto é igual aos primeiros 8 dígitos do posto
                cod_pos = System.IO.Path.GetFileNameWithoutExtension(arq(posto)).Substring(0, 8)

                'Abre o arquivo novo no local que corresponde ao lugar onde devem ser salvos os dados de nível
                FileOpen(2, DIR_RHAMA_Q & "\" & data_fim_nome & "N" & cod_pos & ".txt", OpenMode.Output)

                'O tempo máximo que devemos salvar os dados é 4 dias para trás
                Dim tmaxx As Integer
                tmaxx = DateDiff(DateInterval.Hour, datacomeço, data) + 1

                'escreve o arquivo de níveis, começando do tempo 1 até o tempo final
                For aux = 1 To tmaxx
                    'Adiciona uma hora no contador somente se tivermos passado do tempo 1
                    If aux <> 1 Then
                        datacomeço = datacomeço.AddHours(1)
                    End If

                    'Acerta o arquivo para escrever conforme os espaçamentos
                    If datacomeço.Day < 10 Then
                        Print(2, "    " & "0" & datacomeço.Day)
                    ElseIf datacomeço.Day >= 10 Then
                        Print(2, "    " & datacomeço.Day)
                    End If
                    If datacomeço.Month < 10 Then
                        Print(2, "    " & "0" & datacomeço.Month)
                    ElseIf datacomeço.Month >= 10 Then
                        Print(2, "    " & datacomeço.Month)

                    End If

                    Print(2, "  " & datacomeço.Year)

                    If datacomeço.Hour.ToString.Length = 1 Then
                        Print(2, "    " & "0" & datacomeço.Hour.ToString)
                    ElseIf datacomeço.Hour.ToString.Length = 2 Then
                        Print(2, "    " & datacomeço.Hour.ToString)
                    End If

                    'Escreveu a data

                    'Agora vai escrever os números

                    If FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                        Print(2, "       " & "-1.000000")
                    ElseIf FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                        Print(2, "        " & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                    ElseIf FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                        Print(2, "       " & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                    ElseIf FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                        Print(2, "      " & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                    ElseIf FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                        Print(2, "     " & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
                    ElseIf FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                        Print(2, "    " & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                    ElseIf FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                        Print(2, "   " & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(nivel_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                        'não diminui dois espaços pois não tem o separador de milhar

                    End If

                    PrintLine(2)

                Next

                FileClose(2)
                '--Terminou de gerar arquivo de níveis

                'Grava arquivo de precipitação

                'Acerta a data final do arquivo
                'Dim data As Date
                data = dia & "/" & mes & "/" & year & " " & hh & ":00:00"

                'Acerta a data inicial do arquivo
                'Dim datacomeço As Date
                datacomeço = data.AddDays(-7)

                'pega o código do posto novamente
                'Dim cod_pos As String
                cod_pos = System.IO.Path.GetFileNameWithoutExtension(arq(posto)).Substring(0, 8)

                'Abre o arquivo que deve ser gravado como dado de vazão
                FileOpen(3, DIR_RHAMA_P & "\" & data_fim_nome & "P" & cod_pos & ".txt", OpenMode.Output)

                'calcula as 4 horas como tempo máximo
                'Dim tmaxx As Integer
                tmaxx = DateDiff(DateInterval.Hour, datacomeço, data) + 1

                'Começando do tempo 1, vamos seguindo a diante
                For aux = 1 To tmaxx

                    'Só adicionamos valores no contador de datas depois do tempo 1
                    If aux <> 1 Then
                        datacomeço = datacomeço.AddHours(1)
                    End If

                    'Começa a escrever as datas
                    If datacomeço.Day < 10 Then
                        Print(3, "    " & "0" & datacomeço.Day)
                    ElseIf datacomeço.Day >= 10 Then
                        Print(3, "    " & datacomeço.Day)
                    End If
                    If datacomeço.Month < 10 Then
                        Print(3, "    " & "0" & datacomeço.Month)
                    ElseIf datacomeço.Month >= 10 Then
                        Print(3, "    " & datacomeço.Month)

                    End If

                    Print(3, "  " & datacomeço.Year)

                    If datacomeço.Hour.ToString.Length = 1 Then
                        Print(3, "    " & "0" & datacomeço.Hour.ToString)
                    ElseIf datacomeço.Hour.ToString.Length = 2 Then
                        Print(3, "    " & datacomeço.Hour.ToString)
                    End If

                    'Escreveu a data

                    'Agora vai escrever os números

                    If FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                        Print(3, "       " & "-1.000000")
                    ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                        Print(3, "        " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                    ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                        Print(3, "       " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                    ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                        Print(3, "      " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                    ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                        Print(3, "     " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                    ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                        Print(3, "    " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                    ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                        Print(3, "   " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                        'não diminui dois espaços pois não tem o separador de milhar

                    End If

                    PrintLine(3)

                Next

                FileClose(3)
                '--Terminou de gerar arquivo de precipitações





            End If


            FileClose(4)
            'Fecha o arquivo 4
        Loop

        'Depois que o loop externo termina, ele fecha o arquivo 1, que continha a lista de estações
        FileClose()

        'Acerta a barra de progresso
        PBar.Visible = False



    End Sub

    Sub DOWNLOAD_MANUAL_RHAMA() 'Não está sendo usada pois considera apenas a dtp'
        'mas é completamente funcional

        Me.Text = "Fazendo download..."

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações.txt", OpenMode.Input)

        'Abre o arquivo de log
        FileOpen(2, pasta_exec & "\" & dtp_RHAMA.Value.Year & "." & dtp_RHAMA.Value.Month & "." & dtp_RHAMA.Value.Day & "." & cb_hora_RHAMA.Text & "_N" & "log.txt", OpenMode.Output)


        'Variável de leitura
        Dim linha As String

        'Acerta a barra de progresso
        PBar.Visible = True
        PBar.Maximum = 10
        PBar.Value = 0


        'Faz o download dos arquivos individualmente
        Do Until EOF(1)
            linha = LineInput(1)

            PBar.Increment(1)

            Dim arquivoFonte As String 'arquivo que será baixado

            Dim dia, mes, hora As String
            dia = dtp_RHAMA.Value.Day
            If dia.Length < 2 Then
                dia = "0" & dia
            End If

            mes = dtp_RHAMA.Value.Month
            If mes.Length < 2 Then
                mes = "0" & mes
            End If

            hora = cb_hora_RHAMA.Text.Substring(0, 2)
            If hora.Length < 2 Then
                hora = "0" & hora
            End If

            'Define as variáveis que diferenciam dados flu e plu para download dos dados flu
            RHAMA_Flu = True
            RHAMA_Plu = False

            'informa o endereço do arquivo de cotas
            arquivoFonte = txtarq_auto.Text & dtp_RHAMA.Value.Year & "." & mes & "." & dia & "." & hora & "_N" & linha & ".txt"

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP_automático(arquivoFonte, tb_usuário.Text, tb_senha.Text)

            'Define as variáveis que diferenciam dados flu e plu para download dos dados flu
            RHAMA_Flu = False
            RHAMA_Plu = True

            'informa o endereço do arquivo de chuva
            arquivoFonte = txtarq_auto.Text & dtp_RHAMA.Value.Year & "." & mes & "." & dia & "." & hora & "_P" & linha & ".txt"

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP_automático(arquivoFonte, tb_usuário.Text, tb_senha.Text)

            'Define as variáveis que diferenciam dados flu e plu 
            RHAMA_Flu = False
            RHAMA_Plu = False

        Loop

        FileClose()

        PBar.Visible = False

        Me.Text = "Aquisição de dados"

    End Sub 'Não está sendo usada

    '------ Fim Estreito RHAMA

    '--CPTEC

    Private Sub Button6_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button6.Click
        Me.BAIXARCPTEC()
        'Me.BackgroundWorker1.RunWorkerAsync()
    End Sub

    Private Sub Button5_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button5.Click

        Timer1.Enabled = True 'Dispara o timer para funcionar
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        CPTEC = True 'libera o download para o CPTEC
        Label24.ForeColor = Drawing.Color.Green 'Indica que o CPTEC está funcionando

    End Sub

    Sub DOWNLOAD_AUTOMÁTICO_CPTEC()

        Me.PREENCHE_ESTAÇÕES_CPTEC_Automático()

        Me.Text = "Fazendo download..."

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações_CPTEC.txt", OpenMode.Input)

        'Variável de leitura
        Dim linha As String

        'Acerta a barra de progresso
        PBar.Visible = True
        PBar.Maximum = tb_horiz.Text
        PBar.Value = 0

        'Faz o download dos arquivos individualmente
        Do Until EOF(1)
            linha = LineInput(1)

            Dim arquivoFonte As String 'arquivo que será baixado

            PBar.Increment(1)

            Dim data_aq As Date
            data_aq = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & Label2.Text.Substring(0, 2) & ":00:00"

            Dim dia, mes, hora As String
            dia = DateTimePicker1.Value.Day

            'adição especial para o CPTEC, baixa os dados do dia anterior
            Dim diaanterior As Date
            diaanterior = data_aq.AddHours(-12)
            dia = diaanterior.Day

            If dia.Length < 2 Then
                dia = "0" & dia
            End If

            mes = diaanterior.Month
            If mes.Length < 2 Then
                mes = "0" & mes
            End If

            hora = diaanterior.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If

            'informa o endereço do arquivo
            arquivoFonte = TextBox6.Text & diaanterior.Year & mes & dia & hora & "/" & cb_modelo.Text & diaanterior.Year & mes & dia & hora & "+" & linha & ".grb"

            If cb_modelo.Text = "eta_20km_" Then
                arquivoFonte = TextBox6.Text & diaanterior.Year & mes & dia & hora & "/" & "ll20gfsa" & diaanterior.Year & mes & dia & hora & "+" & linha & ".grb"
            End If

            'MsgBox(arquivoFonte)

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP_automático_CPTEC(arquivoFonte, "", "")


        Loop

        FileClose()

        PBar.Visible = False
        Me.Text = "Aquisição de dados"


    End Sub

    Private Sub Button4_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button4.Click
        CPTEC = False
        Label24.ForeColor = Drawing.Color.Red

    End Sub

    Private Sub DownloadViaFTP_automático_CPTEC(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing
        Dim pastaDestino As String

        Dim app_path As String
        app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        Dim arqfonte_name As String
        arqfonte_name = System.IO.Path.GetFileName(arquivoFonte)

        ' ----- Obtem local onde irá salvar o arquivo
        ' pastaDestino = System.IO.Path.Combine(app_path, arqfonte_name)
        'pastaDestino = app_path & "\CPTEC-ETA_15KM\" & arqfonte_name
        pastaDestino = DIR_CPTEC & "\" & arqfonte_name


        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)

            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0

            '  PrintLine(2, "Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
            'MsgBox("Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
        Catch ex As Exception
            ' MsgBox("Erro durante a realização do FTP " + vbCrLf + ex.Message)
            ' PrintLine(2, "Erro durante a realização do FTP " + vbCrLf + ex.Message)
            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

        If System.IO.Path.GetExtension(pastaDestino) = ".gz" Then
            'Descompacta o arquivo que foi baixado
            descompactaArquivo(pastaDestino, System.IO.Path.GetDirectoryName(pastaDestino) & "\" & System.IO.Path.GetFileNameWithoutExtension(pastaDestino))

            Try 'Tenta fazer uma cópia
                File.Copy(System.IO.Path.GetDirectoryName(pastaDestino) & "\" & System.IO.Path.GetFileNameWithoutExtension(pastaDestino), System.IO.Path.GetDirectoryName(pastaDestino) & "\" & cbSISMAmodel.Text & ".bin")
            Catch ex As Exception

            End Try


        End If

    End Sub

    Sub PREENCHE_ESTAÇÕES_CPTEC()


        Dim dia, mes, hora As String
        dia = dtpCPTEC.Value.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        mes = dtpCPTEC.Value.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If


        TextBox10.Text = dtpCPTEC.Value.Year & mes & dia & ComboBox1.Text

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(3, pasta_exec & "\" & "Estações_CPTEC.txt", OpenMode.Output)

        Dim data_d As Date
        data_d = TextBox10.Text.Substring(6, 2) & "/" & TextBox10.Text.Substring(4, 2) & "/" & TextBox10.Text.Substring(0, 4) & " " & TextBox10.Text.Substring(8, 2) & ":00:00"

        For adddata As Integer = 0 To tb_horiz.Text
            Dim data_aq As Date
            data_aq = data_d.AddHours(adddata)

            dia = data_aq.Day
            If dia.Length < 2 Then
                dia = "0" & dia
            End If

            mes = data_aq.Month
            If mes.Length < 2 Then
                mes = "0" & mes
            End If

            hora = data_aq.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If


            PrintLine(3, data_aq.Year & mes & dia & hora)
        Next
        FileClose()



    End Sub

    Sub PREENCHE_ESTAÇÕES_CPTEC_Automático()
        ' TextBox10.Text = dtpCPTEC.Value.Year & dtpCPTEC.Value.Month & dtpCPTEC.Value.Day & ComboBox1.Text


        Dim data_a As Date
        data_a = DateTimePicker1.Value.Day & "/" & DateTimePicker1.Value.Month & "/" & DateTimePicker1.Value.Year & " " & Label2.Text.Substring(0, 2) & ":00:00"

        Dim dia, mes, hora As String

        'adição especial para o CPTEC, baixa os dados do dia anterior
        Dim diaanterior As Date
        diaanterior = data_a.AddHours(-12)

        dia = diaanterior.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        mes = diaanterior.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If

        hora = diaanterior.Hour
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(3, pasta_exec & "\" & "Estações_CPTEC.txt", OpenMode.Output)

        Dim data_d As Date
        data_d = dia & "/" & mes & "/" & data_a.Year & " " & hora & ":00:00"

        For adddata As Integer = 0 To tb_horiz.Text
            Dim data_aq As Date
            data_aq = data_d.AddHours(adddata)

            dia = data_aq.Day
            If dia.Length < 2 Then
                dia = "0" & dia
            End If

            mes = data_aq.Month
            If mes.Length < 2 Then
                mes = "0" & mes
            End If

            hora = data_aq.Hour
            If hora.Length < 2 Then
                hora = "0" & hora
            End If


            PrintLine(3, data_aq.Year & mes & dia & hora)
        Next
        FileClose()



    End Sub

    Private Sub cb_modelo_SelectedIndexChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_modelo.SelectedIndexChanged
        If cb_modelo.SelectedItem = "eta_05km_" Then
            TextBox6.Text = "ftp://ftp1.cptec.inpe.br/modelos/io/tempo/regional/Eta05km/grib/"
        ElseIf cb_modelo.SelectedItem = "eta_10km_" Then
            TextBox6.Text = "ftp://ftp1.cptec.inpe.br/modelos/io/tempo/regional/Eta10km/grib/"
        ElseIf cb_modelo.SelectedItem = "rec_5L_eta_15km" Then
            'TextBox6.Text = "ftp://ftp1.cptec.inpe.br/modelos/io/tempo/regional/Eta15km/grib/"
            TextBox6.Text = "ftp://ftp1.cptec.inpe.br/modelos/io/tempo/regional/Eta15km/rec_grib/"
        ElseIf cb_modelo.SelectedItem = "eta_20km_" Then
            TextBox6.Text = "ftp://ftp1.cptec.inpe.br/modelos/io/tempo/regional/Eta20km/grib/"
        ElseIf cb_modelo.SelectedItem = "eta_40km_" Then
            TextBox6.Text = "ftp://ftp1.cptec.inpe.br/modelos/io/tempo/regional/Eta40km/grib/"
        End If




    End Sub

    '------Fim CPTEC

    '--UHE Garibaldi

    Private Sub Button9_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button9.Click
        Dim arquivoFonte As String

        arquivoFonte = TextBox13.Text

        If (arquivoFonte = "") Or arquivoFonte = String.Empty Then
            MsgBox("Informe a URL e o nome do arquivo para download.")
            Return
        End If

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        Lajeado = True

        DownloadViaFTP(arquivoFonte, TextBox11.Text, TextBox8.Text)

        Lajeado = False

    End Sub

    Private Sub Button7_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button7.Click
        Lajeado = False
        Label25.ForeColor = Drawing.Color.Red

    End Sub

    Private Sub Button8_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button8.Click
        Timer1.Enabled = True
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        Lajeado = True
        Label25.ForeColor = Drawing.Color.Green

    End Sub

    Sub DOWNLOAD_AUTOMÁTICO_LAJEADO()

        Me.Text = "Fazendo download..."


        Dim arquivoFonte As String 'arquivo que será baixado



        'informa o endereço do arquivo
        arquivoFonte = TextBox7.Text

        'MsgBox(arquivoFonte)

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP_automático_LAJEADO(arquivoFonte, TextBox11.Text, TextBox8.Text)

        FileClose()

        Me.Text = "Aquisição de dados"


    End Sub

    Private Sub DownloadViaFTP_automático_LAJEADO(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing
        Dim pastaDestino As String

        Dim app_path As String
        app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        Dim arqfonte_name As String
        arqfonte_name = System.IO.Path.GetFileName(arquivoFonte)

        Dim dia, mes, hora As String
        dia = DateTimePicker1.Value.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        mes = DateTimePicker1.Value.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If

        hora = Label2.Text.Substring(0, 2)
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        ' ----- Obtem local onde irá salvar o arquivo
        'pastaDestino = app_path & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        'pastaDestino = app_path & "\UHE_Lajeado\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        'pastaDestino = DIR_LAJEADO & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name

        pastaDestino = DIR_RHAMA_Q & "\" & arqfonte_name


        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)
            arquivoRequisitado.UsePassive = True
            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0
            '  PrintLine(2, "Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
            'MsgBox("Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
        Catch ex As Exception
            ' MsgBox("Erro durante a realização do FTP " + vbCrLf + ex.Message)
            ' PrintLine(2, "Erro durante a realização do FTP " + vbCrLf + ex.Message)
            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

    End Sub

    '-------Fim LAJEADO


    '--COMEÇO UHE ESTREITO

    Private Sub Button14_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button14.Click
        Timer1.Enabled = True 'Libera o funcionamento do contador (timer)
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        UHE_Estreito = True 'libera o funcionamento do download para a UHE Estreito
        Label36.ForeColor = Drawing.Color.Green 'pinta de verde o label que simboliza a UHE-Estreito
    End Sub

    Private Sub Button13_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button13.Click
        UHE_Estreito = False
        Label36.ForeColor = Drawing.Color.Red

    End Sub

    Private Sub Button15_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button15.Click
        Dim arquivoFonte As String

        arquivoFonte = TextBox23.Text

        If (arquivoFonte = "") Or arquivoFonte = String.Empty Then
            MsgBox("Informe a URL e o nome do arquivo para download.")
            Return
        End If

        UHE_Estreito = True

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP(arquivoFonte, TextBox21.Text, TextBox20.Text)

        UHE_Estreito = False

    End Sub

    Sub DOWNLOAD_AUTOMÁTICO_ESTREITO()

        Me.Text = "Fazendo download..."


        Dim arquivoFonte As String 'arquivo que será baixado



        'informa o endereço do arquivo
        arquivoFonte = TextBox19.Text '& "uhet.txt"

        'MsgBox(arquivoFonte)

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP_automático_ESTREITO(arquivoFonte, TextBox21.Text, TextBox20.Text)

        FileClose()

        Me.Text = "Aquisição de dados"


    End Sub

    Private Sub DownloadViaFTP_automático_ESTREITO(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing
        Dim pastaDestino As String

        Dim app_path As String
        app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        Dim arqfonte_name As String
        arqfonte_name = System.IO.Path.GetFileName(arquivoFonte)

        Dim dia, mes, hora As String
        dia = DateTimePicker1.Value.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        mes = DateTimePicker1.Value.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If

        hora = Label2.Text.Substring(0, 2)
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        ' ----- Obtem local onde irá salvar o arquivo
        ' pastaDestino = app_path & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        ' pastaDestino = app_path & "\UHE_Estreito\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        ' pastaDestino = DIR_ESTREITO & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        pastaDestino = DIR_RHAMA_Q & "\" & arqfonte_name


        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)

            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0
            '  PrintLine(2, "Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
            'MsgBox("Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
        Catch ex As Exception
            ' MsgBox("Erro durante a realização do FTP " + vbCrLf + ex.Message)
            ' PrintLine(2, "Erro durante a realização do FTP " + vbCrLf + ex.Message)
            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

    End Sub

    '-------Fim UHE ESTREITO

    '--UHE Serra da Mesa

    Private Sub Button17_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button17.Click
        Timer1.Enabled = True
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        UHE_SerraDaMesa = True
        Label42.ForeColor = Drawing.Color.Green
    End Sub

    Private Sub Button16_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button16.Click
        UHE_SerraDaMesa = False
        Label42.ForeColor = Drawing.Color.Red

    End Sub

    Private Sub Button18_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button18.Click
        Dim arquivoFonte As String

        arquivoFonte = TextBox28.Text

        If (arquivoFonte = "") Or arquivoFonte = String.Empty Then
            MsgBox("Informe a URL e o nome do arquivo para download.")
            Return
        End If

        UHE_SerraDaMesa = True
        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP(arquivoFonte, TextBox26.Text, TextBox25.Text)

        UHE_SerraDaMesa = False



    End Sub

    Sub DOWNLOAD_AUTOMÁTICO_SERRADAMESA()

        Me.Text = "Fazendo download..."


        Dim arquivoFonte As String 'arquivo que será baixado



        'informa o endereço do arquivo
        arquivoFonte = TextBox24.Text

        'MsgBox(arquivoFonte)

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP_automático_SERRADAMESA(arquivoFonte, TextBox26.Text, TextBox25.Text)

        FileClose()

        Me.Text = "Telemetria"


    End Sub

    Private Sub DownloadViaFTP_automático_SERRADAMESA(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing
        Dim pastaDestino As String

        Dim app_path As String
        app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        Dim arqfonte_name As String
        arqfonte_name = System.IO.Path.GetFileName(arquivoFonte)

        Dim dia, mes, hora As String
        dia = DateTimePicker1.Value.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        mes = DateTimePicker1.Value.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If

        hora = Label2.Text.Substring(0, 2)
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        ' ----- Obtem local onde irá salvar o arquivo
        ' pastaDestino = app_path & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        ' pastaDestino = app_path & "\UHE_Estreito\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        ' pastaDestino = DIR_ESTREITO & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        pastaDestino = DIR_RHAMA_Q & "\" & arqfonte_name


        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)

            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0
            '  PrintLine(2, "Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
            'MsgBox("Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
        Catch ex As Exception
            ' MsgBox("Erro durante a realização do FTP " + vbCrLf + ex.Message)
            ' PrintLine(2, "Erro durante a realização do FTP " + vbCrLf + ex.Message)
            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

    End Sub


    '-------Fim UHE Serra da Mesa

    Private Sub dtp_RHAMA_ValueChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles dtp_RHAMA.ValueChanged
        dtp_RHAMA2.Value = dtp_RHAMA.Value 'equaliza os dois date picker quando o primeiro é mudado
    End Sub

    Private Sub cb_hora_RHAMA_SelectedIndexChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles cb_hora_RHAMA.SelectedIndexChanged
        cb_hora_RHAMA2.Text = cb_hora_RHAMA.Text 'equaliza os dois date picker quando o primeiro é mudado
    End Sub

    '-- UHE Cana Brava

    Private Sub Label53_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Label53.Click

    End Sub

    Private Sub Button20_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button20.Click
        Timer1.Enabled = True 'Libera o funcionamento do contador (timer)
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        UHE_CanaBrava = True 'libera o funcionamento do download para a UHE Estreito
        Label53.ForeColor = Drawing.Color.Green 'pinta de verde o label que simboliza a UHE Cana Brava
    End Sub

    Private Sub Button19_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button19.Click
        UHE_CanaBrava = False 'desliga o download de cana brava
        Label53.ForeColor = Drawing.Color.Red 'pinta o label de cana brava na cor vermelha

    End Sub

    Private Sub Button21_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button21.Click
        Dim arquivoFonte As String 'nome do arquivo que será baixado

        arquivoFonte = TextBox33.Text() 'o nome do arquivo que será baixado está escrito na caixa de texto de download manual

        'Verifica se o arquivo fonte não está vazio
        If (arquivoFonte = "") Or arquivoFonte = String.Empty Then
            MsgBox("Informe a URL e o nome do arquivo para download.")
            Return
        End If

        'Liga o indicador de canabrava
        UHE_CanaBrava = True

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        '                 Nome do arquivo, usuário, senha
        DownloadViaFTP(arquivoFonte, TextBox31.Text, TextBox30.Text)

        'Desliga o indicador de canabrava
        UHE_CanaBrava = False

    End Sub

    Sub DOWNLOAD_AUTOMÁTICO_CANABRAVA()

        Me.Text = "Fazendo download..."

        Dim arquivoFonte As String 'arquivo que será baixado

        'informa o endereço do arquivo
        arquivoFonte = TextBox29.Text & "uhcb.txt"

        'MsgBox(arquivoFonte)

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP_automático_CANABRAVA(arquivoFonte, TextBox31.Text, TextBox30.Text)

        FileClose()

        Me.Text = "Aquisição de dados"


    End Sub

    Private Sub DownloadViaFTP_automático_CANABRAVA(ByVal arquivoFonte As String, ByVal userName As String, ByVal password As String)

        'Faz o Download do arquivo definido via FTP e 
        'salva em uma pasta da aplicação
        Dim readBuffer(4096) As Byte
        Dim contador As Integer

        Dim arquivoRequisitado As FtpWebRequest
        Dim respostaFTP As FtpWebResponse = Nothing
        Dim respostaStream As Stream = Nothing
        Dim arquivoSaida As FileStream = Nothing
        Dim pastaDestino As String

        Dim app_path As String
        app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        Dim arqfonte_name As String
        arqfonte_name = System.IO.Path.GetFileName(arquivoFonte)

        Dim dia, mes, hora As String
        dia = DateTimePicker1.Value.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        mes = DateTimePicker1.Value.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If

        hora = Label2.Text.Substring(0, 2)
        If hora.Length < 2 Then
            hora = "0" & hora
        End If

        ' ----- Obtem local onde irá salvar o arquivo
        ' pastaDestino = app_path & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        ' pastaDestino = app_path & "\UHE_Estreito\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name
        pastaDestino = DIR_CANABRAVA & "\" & DateTimePicker1.Value.Year & mes & dia & hora & "_" & arqfonte_name


        Try
            ' ----- Faz a conexao com o arquivo no site FTP
            arquivoRequisitado = CType(FtpWebRequest.Create(New Uri(arquivoFonte)), FtpWebRequest)
            arquivoRequisitado.Credentials = New NetworkCredential(userName, password)

            arquivoRequisitado.KeepAlive = False
            arquivoRequisitado.UseBinary = True

            arquivoRequisitado.Method = WebRequestMethods.Ftp.DownloadFile

            ' ----- Abre um canal de transmissão para o arquivo
            respostaFTP = CType(arquivoRequisitado.GetResponse, FtpWebResponse)
            respostaStream = respostaFTP.GetResponseStream
            arquivoSaida = New FileStream(pastaDestino, FileMode.Create)

            ' ----- Salva o conteúdo do arquivo de saida bloco a bloco
            Do
                contador = respostaStream.Read(readBuffer, 0, readBuffer.Length)
                arquivoSaida.Write(readBuffer, 0, contador)
            Loop Until contador = 0
            '  PrintLine(2, "Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
            'MsgBox("Download completo! " & vbNewLine & arquivoFonte & vbNewLine & " em " & pastaDestino)
        Catch ex As Exception
            ' MsgBox("Erro durante a realização do FTP " + vbCrLf + ex.Message)
            ' PrintLine(2, "Erro durante a realização do FTP " + vbCrLf + ex.Message)
            Return
        End Try

        ' ----- libera recursos.
        respostaStream.Close()
        arquivoSaida.Flush()
        arquivoSaida.Close()
        respostaFTP.Close()

    End Sub

    '--------------- Fim UHE Cana Brava

    'SIMEPAR

    Private Sub Button22_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button22.Click
        Dim dataIn As New DateTime(dtsm_ini.Value.Year, dtsm_ini.Value.Month, dtsm_ini.Value.Day, tbsm_ini.Text, 0, 0)
        Dim dataFim As DateTime = dtsm_fim.Value.Day & "/" & dtsm_fim.Value.Month & "/" & dtsm_fim.Value.Year & " " & tbsm_fim.Text & ":00:00"

        ' 37;SMAreias;-10.89600;-48.33890;0;Areias;0;0;-1.0
        '38;SMFazJur;-11.25690;-48.51470;0;FazJurupari;0;0;-1.0
        '39;SMIpueir;-11.24690;-47.83630;0;Ipueiras;0;0;-1.0
        '40;SMJacint;-11.98175;-48.65719;0;Jacinto;0;0;-1.0
        '41;SMLucena;-9.69000;-48.37280;0;Lucena;0;0;-1.0
        '42;SMMangue;-10.34470;-48.63470;0;Mangues;0;0;-1.0
        '43;SMPorJer;-11.75803;-47.83630;0;PortoJeronimo;0;0;-1.0
        '44;SMTocant;-9.56690;-48.37280;0;Tocantinia;0;0;-1.0

        PBar.Show()
        PBar.Value = 0
        PBar.Maximum = 8

        'Tocantinia
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.Tocantinia, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMTocant" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMTocant" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)

        'PortoJeronimo
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.PortoJeronimo, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMPorJer" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMPorJer" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)

        'Mangues
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.Mangues, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMMangue" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMMangue" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)

        'Lucena
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.Lucena, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMLucena" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMLucena" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)

        'Jacinto
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.Jacinto, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMJacint" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMJacint" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)

        'Ipueiras
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.Ipueiras, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMIpueir" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMIpueir" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)

        'FazJarupari
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.FazJurupari, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMFazJur" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMFazJur" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)

        'Areias
        Try
            Dim simepar As New HttpSimepar(HttpSimepar.Estacao.Areias, HttpSimepar.TipoEstacao.Hidrologica, HttpSimepar.TipoInformacao.Precipitacao, dataIn, dataFim)
            simepar.Requsitar()
            Dim app_path As String 'Local onde o programa está sendo executado
            app_path = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
            simepar.PlotarDados(app_path & "\Estreito_RHAMA\" & "SMAreias" & ".txt")
            Me.CONVERTE_SIMEPAR(app_path & "\Estreito_RHAMA\" & "SMAreias" & ".txt")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical)
        End Try

        PBar.Increment(1)
        PBar.Hide()
        MsgBox("Download dos dados finalizado", MsgBoxStyle.Information, "Aquisição de dados")

    End Sub

    Sub CONVERTE_SIMEPAR(ByVal nome_arquivo As String)

        'Dim nome_arquivo As String = app_path & "\Estreito_RHAMA\" & "Tocantinia" & ".txt"

        'Variável que contém o número de postos a serem baixados
        Dim numpostos As Integer
        numpostos = 1 'define ela como sendo igual ao número de linhas do arquivo de estações

        'Variáveis necessárias para dimensionamento
        Dim hh As Integer 'hora
        Dim min As Integer 'minutos
        Dim mes As Integer 'meses
        Dim dia As Integer 'dias
        Dim year As Integer 'anos

        'Define os valores das variáveis
        hh = 24
        min = 45
        mes = 12
        dia = 31
        year = AF


        'Dimensiona as variáveis de chuva e nível, que vão conter os resultados
        'Dim chuva_min(numpostos, min, hh, dia, mes, Ano(year)) As Double
        Dim chuva_hor(numpostos, hh, dia, mes, Ano(year)) As Double

        'Zera as variáveis 
        For p As Integer = 0 To numpostos
            For aaaa = AI To AF
                For mm = 1 To 12
                    For dd = 1 To 31
                        For h As Integer = 0 To 23
                            For minu As Integer = 1 To 45
                                'chuva_min(p, minu, h, dd, mm, Ano(aaaa)) = -1.0
                                chuva_hor(p, h, dd, mm, Ano(aaaa)) = -1.0
                            Next
                        Next
                    Next
                Next
            Next
        Next
        'Zerou as variáveis

        'Dimensiona algumas variáveis de leitura que são importantes
        Dim arq(numpostos) As String 'nome dos postos que serão utilizados
        Dim posto As Integer 'número do posto que está sendo usado
        posto = 0 'o número do posto começa como zero, e vai aumentando no contador

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())
        'que é a pasta de execução do plugin

        Dim arquivoFonte As String 'arquivo que será transformado
        'informa o endereço do arquivo
        arquivoFonte = nome_arquivo

        'Redefine as variávies que servem para dimensionar os postos
        hh = 24
        min = 45
        mes = 12
        dia = 31
        year = AF

        'Aumenta 1 na variável de contagem dos postos
        'cada linha do arquivo de estações tem a informação de um posto
        posto = posto + 1
        'coloca o nome do posto que está na posição 1 como sendo o nome do arquivo que será aberto
        arq(posto) = arquivoFonte

        'Abre o arquivo de interesse
        FileOpen(4, arq(posto), OpenMode.Input)

        'Cria outra variável de leitura
        Dim linha4 As String
        'Cria um contador de linhas do arquivo de interesse
        Dim cont_linha As Integer
        'Define que o valor inicial do contador é -1
        cont_linha = -1

        'Esta variável simboliza a chuva acumulada em cada mes, ela é definida neste momento
        Dim plu_acum_mes(mes) As Double


        'Carrega os valores da variável de chuva acumulada, inicialmente ela é zero para todos os meses
        For m As Integer = 0 To 12
            plu_acum_mes(m) = 0
        Next

        'Essa variável indica o nome do arquivo que será gerado após a conversão
        Dim data_fim_nome As String

        'Variáveis utilizadas no cálculo da média dos dados de vazão
        Dim acumflu As Double = 0 'soma dos níveis
        Dim flumed As Double = 0 'nível médio
        Dim cont_acum As Integer = 0 'contador de quantos minutos temos dados


        'Começa a leitura completa do arquivo que contém os dados da estação
        Do Until EOF(4)
            'Lê linha por linha, guardando os resultados na variável linha4
            linha4 = LineInput(4)
            'adiciona 1 ao contador a cada linha
            cont_linha = cont_linha + 1
            'A primeira linha é o cabeçalho, então começa a leitura a partir do contador = 1
            If cont_linha >= 1 Then
                'lê as variáveis hora, minuto, mes, dia e ano da linha do arquivo
                hh = linha4.Substring(11, 2)
                min = linha4.Substring(14, 2)
                mes = linha4.Substring(3, 2)
                dia = linha4.Substring(0, 2)
                year = linha4.Substring(6, 4)

                'cria estas daus variáveis que correspondem aos valores de chuva e vazão, contidos no arquivo
                Dim plu As Double
                Dim flu As Double

                Dim splt
                splt = Split(linha4, ";")

                Try
                    plu = splt(1)
                Catch ex As Exception
                    plu = -1.0
                End Try

                'plu = splt(1)

                'O nível é o terceiro valor no delimitador, mas os dados de nível nem sempre existem
                'Então usamos um Try para tentar ler eles
                'Caso eles não existem, colocamos o valor -1.00
                Try
                    flu = splt(2)
                Catch ex As Exception
                    flu = -1.0
                End Try

                If min = 15 Then
                    If plu >= 0 Then
                        acumflu = acumflu + plu
                        cont_acum = cont_acum + 1
                    End If
                ElseIf min = 30 Then
                    If plu >= 0 Then
                        acumflu = acumflu + plu
                        cont_acum = cont_acum + 1
                    End If
                ElseIf min = 45 Then
                    If plu >= 0 Then
                        acumflu = acumflu + plu
                        cont_acum = cont_acum + 1
                    End If
                ElseIf min = 0 Then
                    If plu >= 0 Then
                        acumflu = acumflu + plu
                        cont_acum = cont_acum + 1
                    End If
                    'Faz a conta do nível médio
                    If cont_acum > 0 Then 'só calcula o nível caso existam dados em pelo menos um dos 15min
                        flumed = acumflu
                    ElseIf cont_acum = 0 Then 'caso não existam dados, colocamos falha
                        flumed = -1.0
                    End If
                    acumflu = 0 'como a chuva média foi calculada, zeramos as variáveis de contagem
                    cont_acum = 0 'como a chuva média foi calculada, zeramos as variáveis de contagem
                End If


                'If cont_linha = 1 Then
                ' If plu < 0 Then
                ' plu = 0
                'End If
                'plu_acum_mes(mes) = plu
                'End If

                '   If dia = 1 Then
                'If plu < 0 Then
                'plu = 0
                'End If
                'plu_acum_mes(mes) = plu
                'End If

                If min = 0 Then
                    'O valor de chuva é o valor medido, menos o acumulado na última hora
                    'If plu >= 0 Then
                    chuva_hor(posto, hh, dia, mes, Ano(year)) = flumed
                    ' ElseIf plu < 0 Then
                    '   chuva_hor(posto, hh, dia, mes, Ano(year)) = -1.0
                    'End If
                    'O valor de nível é o valor na hora cheia
                    'nivel_hor(posto, hh, dia, mes, Ano(year)) = flumed
                    flumed = 0

                    ' If chuva_hor(posto, hh, dia, mes, Ano(year)) < 0 Then
                    'MsgBox(arq(posto) & " " & hh & " " & dia & " " & mes & " " & year & " " & chuva_hor(posto, hh, dia, mes, Ano(year)))
                    ' End If

                    'Caso o nosso valor for maior que o acumulado (ou seja, a chuva não foi zero)
                    'definimos o novo acumulado
                    'If plu > plu_acum_mes(mes) Then
                    'plu_acum_mes(mes) = plu
                    'End If
                End If

                'Define o nome do arquivo gerado como o último dia com dados
                Dim ddd, hhh, mmm As String
                ddd = dia
                If ddd.ToString.Length < 2 Then
                    ddd = "0" & ddd
                End If

                mmm = mes
                If mmm.ToString.Length < 2 Then
                    mmm = "0" & mmm
                End If

                hhh = hh
                If hhh.ToString.Length < 2 Then
                    hhh = "0" & hhh
                End If

                data_fim_nome = year & "." & mmm & "." & ddd & "." & hhh & "_"
                'Notar que este valor vai variando até o final do arquivo

            End If

        Loop

        'Após o término do loop de leitura do arquivo da estação, fecha o arquivo
        FileClose(4)

        If cont_linha >= 1 Then 'só executa as rotinas de gravar os arquivos, caso tenha lido algo

            'Define uma variável que corresponde à data final do arquivo que foi lido
            Dim data As Date
            data = dia & "/" & mes & "/" & year & " " & hh & ":00:00"

            'e uma variável que corresponde ao começo do arquivo que foi lido
            Dim datacomeço As Date

            'Grava arquivo de precipitação

            'Acerta a data final do arquivo
            'Dim data As Date
            data = dia & "/" & mes & "/" & year & " " & hh & ":00:00"

            'Acerta a data inicial do arquivo
            'Dim datacomeço As Date
            datacomeço = data.AddDays(-7)

            'pega o código do posto novamente
            Dim cod_pos As String
            cod_pos = System.IO.Path.GetFileNameWithoutExtension(arq(posto))

            'Abre o arquivo que deve ser gravado como dado de vazão
            FileOpen(3, DIR_RHAMA_P & "\" & data_fim_nome & "P" & cod_pos & ".txt", OpenMode.Output)

            'calcula as 4 horas como tempo máximo
            Dim tmaxx As Integer
            tmaxx = DateDiff(DateInterval.Hour, datacomeço, data) + 1

            'Começando do tempo 1, vamos seguindo a diante
            For aux = 1 To tmaxx

                'Só adicionamos valores no contador de datas depois do tempo 1
                If aux <> 1 Then
                    datacomeço = datacomeço.AddHours(1)
                End If

                'Começa a escrever as datas
                If datacomeço.Day < 10 Then
                    Print(3, "    " & "0" & datacomeço.Day)
                ElseIf datacomeço.Day >= 10 Then
                    Print(3, "    " & datacomeço.Day)
                End If
                If datacomeço.Month < 10 Then
                    Print(3, "    " & "0" & datacomeço.Month)
                ElseIf datacomeço.Month >= 10 Then
                    Print(3, "    " & datacomeço.Month)

                End If

                Print(3, "  " & datacomeço.Year)

                If datacomeço.Hour.ToString.Length = 1 Then
                    Print(3, "    " & "0" & datacomeço.Hour.ToString)
                ElseIf datacomeço.Hour.ToString.Length = 2 Then
                    Print(3, "    " & datacomeço.Hour.ToString)
                End If

                'Escreveu a data

                'Agora vai escrever os números

                If FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
                    Print(3, "       " & "-1.000000")
                ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
                    Print(3, "        " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
                    Print(3, "       " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
                    Print(3, "      " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
                ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
                    Print(3, "     " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
                    Print(3, "    " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
                ElseIf FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
                    Print(3, "   " & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(chuva_hor(posto, datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
                    'não diminui dois espaços pois não tem o separador de milhar

                End If

                PrintLine(3)

            Next

            FileClose(3)
            '--Terminou de gerar arquivo de precipitações





        End If


        FileClose(4)
        'Fecha o arquivo 4

        'Depois que o loop externo termina, ele fecha o arquivo 1, que continha a lista de estações
        FileClose()

        'Acerta a barra de progresso
        ' PBar.Visible = False


    End Sub

    Sub ACERTA_SIMEPAR()
        dtsm_ini.Value = Now.Date.AddDays(-7)
        dtsm_fim.Value = Now.Date
    End Sub

    '----------Fim SIMEPAR

    Sub BAIXARCPTEC()
        Me.PREENCHE_ESTAÇÕES_CPTEC() 'Esta rotina preenche um arquivo texto com a lista de estações que devem
        'ser adquiridas via FTP do CPTEC
        Me.Text = "Fazendo download das estações do CPTEC..."

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações_CPTEC.txt", OpenMode.Input)

        'Variável de leitura
        Dim linha As String

        'Acerta a barra de progresso
        PBar.Visible = True
        PBar.Maximum = tb_horiz.Text
        PBar.Value = 0

        'Faz o download dos arquivos individualmente
        Do Until EOF(1)
            linha = LineInput(1)

            Dim arquivoFonte As String

            PBar.Increment(1)

            arquivoFonte = TextBox6.Text & "/" & TextBox10.Text & "/" & cb_modelo.Text & TextBox10.Text & "+" & linha & ".grb"

            If cb_modelo.Text = "eta_20km_" Then
                arquivoFonte = TextBox6.Text & "/" & TextBox10.Text & "/" & "ll20gfsa" & TextBox10.Text & "+" & linha & ".grb"
            End If

            If (arquivoFonte = "") Or arquivoFonte = String.Empty Then
                MsgBox("Informe a URL e o nome do arquivo para download.")
                Return
            End If


            CPTEC = True
            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            DownloadViaFTP(arquivoFonte, "", "")

            CPTEC = False


        Loop

        FileClose()

        MsgBox("Download das estações do CPTEC realizado com sucesso!")

        PBar.Visible = False
        Me.Text = "Aquisição de dados"



    End Sub



    Private Sub BackgroundWorker1_DoWork(ByVal sender As System.Object, ByVal e As System.ComponentModel.DoWorkEventArgs) Handles BackgroundWorker1.DoWork
        BAIXARCPTEC()
    End Sub

    Private Sub Button23_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button23.Click
        Me.Close()
    End Sub

    Private Sub Button26_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button26.Click
        Me.BAIXARCPTEC_SISMADEM()
    End Sub

    Sub BAIXARCPTEC_SISMADEM()
        'DEFINE O DIA COM DOIS DÍGITOS
        Dim dia, mes As String
        dia = dtpSISMA.Value.Day
        If dia.Length < 2 Then
            dia = "0" & dia
        End If

        'DEFINE O MÊS DO ANO COM DOIS DÍGITOS
        mes = dtpSISMA.Value.Month
        If mes.Length < 2 Then
            mes = "0" & mes
        End If

        'Parte do nome do arquivo a ser baixado que varia diariamente
        Dim ParteArquivo As String
        ParteArquivo = dtpSISMA.Value.Year & mes & dia & cbSISMA.Text

        'ser adquiridas via FTP do CPTEC
        Me.Text = "Fazendo download " & ParteArquivo & " ..."

        'Define a pasta onde está salvando
        CPTEC = True

        'variável com o nome do arquivo
        Dim arquivoFonte As String

        'Baixa o dado de chuva de 50mb
        arquivoFonte = tbEndSISMA.Text & cbSISMAmodel.Text & ParteArquivo & ".bin.gz"

        'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
        'efetuar o download via FTP anônimo
        DownloadViaFTP(arquivoFonte, "", "")

        'Baixa o .ctl
        arquivoFonte = tbEndSISMA.Text & cbSISMAmodel.Text & ".ctl"

        'efetuar o download via FTP anônimo
        DownloadViaFTP(arquivoFonte, "", "")

        'Faz uma cópia do arquivo ctl com outro nome
        System.IO.File.Copy(DIR_CPTEC & "\" & cbSISMAmodel.Text & ".ctl", DIR_CPTEC & "\" & ParteArquivo & "_" & cbSISMAmodel.Text & ".ctl", True)

        'Fecha o trabalho com o CPTEC
        'CPTEC_SISMADEM = False
        CPTEC = False

        'Fecha o arquivo que contém o nome
        FileClose()

        MsgBox("Download das estações do SISMADEM no ftp do CPTEC realizado com sucesso!")
        Me.Text = "Aquisição de dados"

    End Sub

    Public Sub descompactaArquivo(ByVal arquivoOrigem As String, ByVal arquivoDestino As String)
        Try
            ' ----- Compacata o contéudo do arquivo e ' guarda o resultado em um novo arquivo
            Dim streamFonte As New FileStream(arquivoOrigem, FileMode.Open, FileAccess.Read)

            Dim streamDestino As New FileStream(arquivoDestino, FileMode.Create, FileAccess.Write)

            ' ----- Os bytes serão processados através de um decompressor de stream
            Dim streamDescompactado As New GZipStream(streamFonte, CompressionMode.Decompress, True)

            ' ----- Processa os bytes de um arquivo para outro
            Const tamanhoBloco As Integer = 4096
            Dim buffer(tamanhoBloco) As Byte
            Dim bytesLidos As Integer

            Do
                bytesLidos = streamDescompactado.Read(buffer, 0, tamanhoBloco)

                If (bytesLidos = 0) Then Exit Do

                streamDestino.Write(buffer, 0, bytesLidos)


            Loop

            ' ----- Fecha todos os arquivos
            streamFonte.Close()
            streamDescompactado.Close()
            streamDestino.Close()

        Catch ex As Exception
            MsgBox(ex.Message)
        Finally

        End Try
    End Sub

    Private Sub Button27_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button27.Click
        Timer1.Enabled = True 'Dispara o timer para funcionar
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        CPTEC_SISMADEM = True 'libera o download para o CPTEC
        Label65.ForeColor = Drawing.Color.Green 'Indica que o CPTEC está funcionando
    End Sub

    Private Sub Button25_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button25.Click
        CPTEC_SISMADEM = False
        Label65.ForeColor = Drawing.Color.Red
    End Sub

    Private Sub tbEndSISMA_TextChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles tbEndSISMA.TextChanged

    End Sub

    Private Sub Label2_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Label2.Click

    End Sub

    Private Sub GroupBox21_Enter(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles GroupBox21.Enter

    End Sub

    Private Sub Label25_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Label25.Click

    End Sub

    Private Sub TextBox12_TextChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles TextBox12.TextChanged

    End Sub

    Private Sub Button30_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button30.Click

        Dim arquivofonte As String
        Dim pastaDestino As String
        arquivofonte = tbONS_GEFS.Text
        pastaDestino = DIR_CPTEC & "\" & My.Computer.FileSystem.GetName(arquivoFonte)

        'criando uma instância da classe WebClient
        Dim cliente As WebClient = New WebClient

        'usando o método DownloadFile para efetuar o download
        Try
            cliente.DownloadFile(arquivofonte, pastaDestino)
            MsgBox("Arquivo recebido com sucesso.", MsgBoxStyle.Information)
        Catch ex As Exception
            MsgBox(ex.Message & " - " & ex.Source)
        End Try

        'limpa a memoria
        cliente.Dispose()
        FileClose()


        If System.IO.Path.GetExtension(pastaDestino) = ".zip" Then
            'Descompacta o arquivo que foi baixado
            '  descompactaArquivo(pastaDestino, System.IO.Path.GetDirectoryName(pastaDestino) & "\" & System.IO.Path.GetFileNameWithoutExtension(pastaDestino))
            DESCOMPACTAR_GEFS(pastaDestino)
           

        End If

    End Sub

    Sub DESCOMPACTAR_GEFS(ByVal fileC As String)

        Dim Copia As String
        Copia = System.IO.Path.GetDirectoryName(fileC) & "\" & "PrevisaoGEFS.zip"
        FileCopy(fileC, Copia)
        Dim Saida As String
        Saida = System.IO.Path.GetDirectoryName(Copia) & "\" & System.IO.Path.GetFileNameWithoutExtension(Copia)

        If Directory.Exists(Saida) Then

            My.Computer.FileSystem.DeleteDirectory(Saida, FileIO.UIOption.OnlyErrorDialogs, FileIO.RecycleOption.SendToRecycleBin)

        End If

        'Dim arquivoOriginal As FileStream = File.OpenRead(Copia)
        'Dim arquivoDestino As FileStream = File.Create(Copia.Replace(".ZIP", ""))
        'Dim zip As New GZipStream(arquivoOriginal, CompressionMode.Decompress, False)
        'Dim arquivoEmBytes((Convert.ToInt32(arquivoOriginal.Length))) As Byte
        'Dim qtdBytes As Int32 = zip.Read(arquivoEmBytes, 0, arquivoEmBytes.Length)
        'arquivoDestino.Write(arquivoEmBytes, 0, qtdBytes)
        'zip.Close()
        'arquivoDestino.Close()

        Dim unzipFile As New ICSharpCode.SharpZipLib.Zip.FastZip

        unzipFile.ExtractZip(Copia, Saida, ".*")

    End Sub

    Sub DOWNLOAD_GEFS_ONS_AUTO()
        Dim arquivofonte As String
        Dim pastaDestino As String
        arquivofonte = tbONS_GEFS.Text
        pastaDestino = DIR_CPTEC & "\" & My.Computer.FileSystem.GetName(arquivofonte)

        'criando uma instância da classe WebClient
        Dim cliente As WebClient = New WebClient

        'usando o método DownloadFile para efetuar o download
        Try
            cliente.DownloadFile(arquivofonte, pastaDestino)
            ' MsgBox("Arquivo recebido com sucesso.", MsgBoxStyle.Information)
        Catch ex As Exception
            ' MsgBox(ex.Message & " - " & ex.Source)
        End Try

        'limpa a memoria
        cliente.Dispose()
        FileClose()

        If System.IO.Path.GetExtension(pastaDestino) = ".zip" Then
            'Descompacta o arquivo que foi baixado
            '  descompactaArquivo(pastaDestino, System.IO.Path.GetDirectoryName(pastaDestino) & "\" & System.IO.Path.GetFileNameWithoutExtension(pastaDestino))
            DESCOMPACTAR_GEFS(pastaDestino)
        End If
    End Sub

    Private Sub Button29_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button29.Click
        Timer1.Enabled = True 'Dispara o timer para funcionar
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        GEFS_ONS = True 'libera o download para o CPTEC
        lbGEFS_ONS.ForeColor = Drawing.Color.Green 'Indica que o CPTEC está funcionando
    End Sub

    Private Sub Button28_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button28.Click
        GEFS_ONS = False 'libera o download para o CPTEC
        lbGEFS_ONS.ForeColor = Drawing.Color.Red 'Indica que o CPTEC está funcionando
    End Sub

    Private Sub Button36_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button36.Click

        Dim arquivofonte As String
        Dim pastaDestino As String
        arquivofonte = tbONS_ETA.Text()
        pastaDestino = DIR_CPTEC & "\" & My.Computer.FileSystem.GetName(arquivofonte)

        'criando uma instância da classe WebClient
        Dim cliente As WebClient = New WebClient

        'usando o método DownloadFile para efetuar o download
        Try
            cliente.DownloadFile(arquivofonte, pastaDestino)
            MsgBox("Arquivo recebido com sucesso.", MsgBoxStyle.Information)
        Catch ex As Exception
            MsgBox(ex.Message & " - " & ex.Source)
        End Try

        'limpa a memoria
        cliente.Dispose()
        FileClose()

        If System.IO.Path.GetExtension(pastaDestino) = ".zip" Then
            'Descompacta o arquivo que foi baixado
            '  descompactaArquivo(pastaDestino, System.IO.Path.GetDirectoryName(pastaDestino) & "\" & System.IO.Path.GetFileNameWithoutExtension(pastaDestino))
            DESCOMPACTAR_ETA40(pastaDestino)
        End If

    End Sub

    Sub DESCOMPACTAR_ETA40(ByVal fileC As String)
        Dim Copia As String
        Copia = System.IO.Path.GetDirectoryName(fileC) & "\" & "PrevisaoETA40.zip"
        FileCopy(fileC, Copia)
        Dim Saida As String
        Saida = System.IO.Path.GetDirectoryName(Copia) & "\" & System.IO.Path.GetFileNameWithoutExtension(Copia)
        If Directory.Exists(Saida) Then
            My.Computer.FileSystem.DeleteDirectory(Saida, FileIO.UIOption.OnlyErrorDialogs, FileIO.RecycleOption.SendToRecycleBin)
        End If
        Dim unzipFile As New ICSharpCode.SharpZipLib.Zip.FastZip
        unzipFile.ExtractZip(Copia, Saida, ".*")
    End Sub


    Private Sub Button38_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button38.Click
        Timer1.Enabled = True 'Dispara o timer para funcionar
        DateTimePicker1.Text = Now.Date 'a cada um segundo, a data do dtp é o dia de hoje
        ETA_ONS = True 'libera o download para o CPTEC
        lbETA_ONS.ForeColor = Drawing.Color.Green 'Indica que o CPTEC está funcionando
    End Sub

    Private Sub Button37_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button37.Click
        ETA_ONS = False 'libera o download para o CPTEC
        lbETA_ONS.ForeColor = Drawing.Color.Red 'Indica que o CPTEC está funcionando
    End Sub

    Sub DOWNLOAD_ETA40_ONS_AUTO()
        Dim arquivofonte As String
        Dim pastaDestino As String
        arquivofonte = tbONS_ETA.Text
        pastaDestino = DIR_CPTEC & "\" & My.Computer.FileSystem.GetName(arquivofonte)

        'criando uma instância da classe WebClient
        Dim cliente As WebClient = New WebClient

        'usando o método DownloadFile para efetuar o download
        Try
            cliente.DownloadFile(arquivofonte, pastaDestino)
            ' MsgBox("Arquivo recebido com sucesso.", MsgBoxStyle.Information)
        Catch ex As Exception
            ' MsgBox(ex.Message & " - " & ex.Source)
        End Try

        'limpa a memoria
        cliente.Dispose()
        FileClose()

        If System.IO.Path.GetExtension(pastaDestino) = ".zip" Then
            'Descompacta o arquivo que foi baixado
            '  descompactaArquivo(pastaDestino, System.IO.Path.GetDirectoryName(pastaDestino) & "\" & System.IO.Path.GetFileNameWithoutExtension(pastaDestino))
            DESCOMPACTAR_ETA40(pastaDestino)
        End If
    End Sub

End Class