Imports System.Net.Mail
Imports System.Net
Imports System.IO

Public Class Form16

    'usado para armazenar os anexos
    Dim Attachment As System.Net.Mail.Attachment

    'cria uma mensagem a enviar
    Dim Mailmsg As New System.Net.Mail.MailMessage()

    Dim User, Senha, NomeUser As String

    Private Sub Label1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Label1.Click

    End Sub

    Private Sub Label3_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Label3.Click

    End Sub

    Private Sub Form16_Load(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles MyBase.Load
        'Adiciona a lista de anexos

        Me.ANEXAR()

        txtAssunto.Text = "Previsão RSH " & Now.Day & "/" & Now.Month & "/" & Now.Year & " " & Now.Hour & "h"

        LISTA_EMAILS()

        TEXTO_EMAILS()

        If TotalCommander = True Then
            'Try
            Me.ENVIAR02()
            
            Me.Close()
            'Catch ex As Exception
            '    ERRO_PREVISÃO = 3
            '    Me.Close()
            'End Try

        End If




    End Sub

    Sub LISTA_EMAILS()
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivo As String
        arquivo = diretóriodoprojeto & "\emails.txt"

        FileOpen(6, arquivo, OpenMode.Input)
        Dim linha6 As String

        linha6 = LineInput(6)

        txtDestino.Text = linha6

        FileClose()


    End Sub

    Sub TEXTO_EMAILS()
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivo As String
        arquivo = diretóriodoprojeto & "\email_texto.txt"

        txtMensagem.Clear()

        FileOpen(6, arquivo, OpenMode.Input)
        Dim linha6 As String
        Do Until EOF(6)
            linha6 = LineInput(6)

            txtMensagem.Text += Environment.NewLine & linha6
        Loop


        FileClose()


    End Sub

    Sub CREDENCIAIS_EMAILS()
        Dim diretóriodoprojeto As String
        diretóriodoprojeto = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        Dim arquivo As String
        arquivo = diretóriodoprojeto & "\email_credenciais.txt"

        FileOpen(6, arquivo, OpenMode.Input)
        Dim linha6 As String
        Dim cont As Integer
        cont = 0

        Do Until EOF(6)
            linha6 = LineInput(6)
            cont += 1
            If cont = 1 Then
                User = linha6
            ElseIf cont = 2 Then
                Senha = linha6
            ElseIf cont = 3 Then
                NomeUser = linha6
            End If

        Loop


        FileClose()


    End Sub

    Sub ANEXAR()
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
        'arquivo = diretóriodoprojeto & "\Previsão_" & Now.Year & mes & dia & hora & ".txt"
        'lstanexos.Items.Add(arquivo)

        ''arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"
        'arquivo = diretóriodoprojeto & "\bgr" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"
        'lstanexos.Items.Add(arquivo)

        arquivo = diretóriodoprojeto & "\Previsão_" & Now.Year & mes & dia & hora & ".bmp"
        lstanexos.Items.Add(arquivo)

    End Sub

    Sub ENVIAR()

        'endereca o recipiente

        Mailmsg.To.Add(New MailAddress(txtDestino.Text))


        Dim mSmtpCliente As New SmtpClient(txtServidorSMTP.Text)


        Mailmsg.From = New MailAddress(txtOrigem.Text)

        'define o assunto

        Mailmsg.Subject = txtAssunto.Text


        'Anexa os arquivos um por um

        For i = 0 To lstanexos.Items.Count - 1

            Mailmsg.Attachments.Add(New Attachment(lstanexos.Items(i)))

        Next


        'Corpo do EMail

        Mailmsg.Body = txtMensagem.Text


        'ENvia o email

        mSmtpCliente.Send(Mailmsg)

    End Sub

    Sub ENVIAR02()

        'Try
        'Cursor = Windows.Forms.Cursors.WaitCursor
        'Configuramos o Cliente
        Dim clienteSmtp As SmtpClient = ConfiguraCliente()
        'Configuramos a Mensagem
        Dim mensagem As MailMessage = ConfiguraMensagem()

        'Anexa os arquivos um por um
        For i = 0 To lstanexos.Items.Count - 1
            mensagem.Attachments.Add(New Attachment(lstanexos.Items(i)))
        Next

        'Envia o Email
        clienteSmtp.Send(mensagem)
        '    Windows.Forms.MessageBox.Show("Email Enviado com Sucesso!", "Sucesso", Windows.Forms.MessageBoxButtons.OK, Windows.Forms.MessageBoxIcon.Information)
        'Catch ex As Exception
        '    Windows.Forms.MessageBox.Show("Falha no Envio: " & vbNewLine & ex.Message, "Sucesso", Windows.Forms.MessageBoxButtons.OK, Windows.Forms.MessageBoxIcon.Information)
        'Finally
        '    Cursor = Windows.Forms.Cursors.Default
        'End Try

    End Sub

    Private Sub Button1_Click(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles Button1.Click
        'Me.ENVIAR()

        Me.ENVIAR02()

    End Sub
    Private Sub TextBox2_TextChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles txtOrigem.TextChanged

    End Sub
    Private Sub txtServidorSMTP_TextChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles txtServidorSMTP.TextChanged

    End Sub
    Private Sub txtMensagem_TextChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles txtMensagem.TextChanged

    End Sub


    ''' <summary>
    ''' Configura o Cliente
    ''' </summary>
    Private Function ConfiguraCliente() As SmtpClient
        Dim clienteSmtp As New SmtpClient(txtServidorSMTP.Text)
        'Para o Gmail é necessário habilitarmos o SSL
        clienteSmtp.EnableSsl = True

        clienteSmtp.Port = 587
        clienteSmtp.Host = txtServidorSMTP.Text
        'Para Servidores que exigem autenticação, informamos usuário e senha
        'Ex: usuário e senha do Gmail (smtp.gmail.com)
        clienteSmtp.Credentials = New Net.NetworkCredential("fluvialengenharia@gmail.com", "<REDACTED>")

        'clienteSmtp.Credentials = New Net.NetworkCredential("previsao@aguaesolo.com", "<REDACTED>")

        'Foi criada a conta de e-mail previsao@aguaesolo.com e a senha é "<REDACTED>".

        Return clienteSmtp


    End Function

    ''' <summary>
    ''' Configura a Mensagem
    ''' </summary>
    Private Function ConfiguraMensagem() As MailMessage
        Dim mensagem As New MailMessage
        'Indicamos se será em formato HTML
        'mensagem.IsBodyHtml = True
        'Adicionamos o Remetente
        mensagem.From = New MailAddress(txtOrigem.Text, "previsao@aguaesolo.com")
        'Adicionamos o Destinatário
        mensagem.To.Add(txtDestino.Text)
        'Adicionamos o Assunto
        mensagem.Subject = txtAssunto.Text
        'Adicionamos a Mensagem
        mensagem.Body = txtMensagem.Text
        'Indicamos a Prioridade
        mensagem.Priority = MailPriority.Normal

        Return mensagem
    End Function

    Private Sub txtAssunto_TextChanged(ByVal sender As System.Object, ByVal e As System.EventArgs) Handles txtAssunto.TextChanged

    End Sub

    Sub FTP_ONS()

        '--Pega o nome do aquivo no computador
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
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"
        arquivo = diretóriodoprojeto & "\bgr" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"

        'Arquivo que deve ir para o FTP
        Dim caminhoarquivoFTP As String
        caminhoarquivoFTP = "/bgr" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"



        'Informe o nome servidor ftp ou ip 
        'Dim ftphost As String = "201.25.31.2"
        Dim ftphost As String = "187.52.189.10"

        Dim caminhoFTP As String = "ftp://" & ftphost & caminhoArquivoFtp
        Dim ftp As FtpWebRequest = DirectCast(FtpWebRequest.Create(caminhoFTP), FtpWebRequest)

        'define as credenciais 
        ftp.Credentials = New NetworkCredential("uhbg", "<REDACTED>")
        'define o tipo de ação 
        ftp.KeepAlive = True
        ftp.UseBinary = True
        ftp.Method = WebRequestMethods.Ftp.UploadFile
        'trata o retorno
        Dim fs As FileStream = File.OpenRead(arquivo)
        Dim buffer As Byte() = New Byte(fs.Length - 1) {}
        fs.Read(buffer, 0, buffer.Length)
        fs.Close()
        Dim ftpstream As Stream = ftp.GetRequestStream()
        ftpstream.Write(buffer, 0, buffer.Length)
        ftpstream.Close()

    End Sub


    Sub FTP_IMAGEM()

        '--Pega o nome do aquivo no computador
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
        'arquivo = diretóriodoprojeto & "\Previsão_ONS_" & Now.Year & mes & dia & hora & ".txt"
        'arquivo = diretóriodoprojeto & "\bgr" & dia & mes & Now.Year.ToString.Substring(2, 2) & ".txt"
        arquivo = diretóriodoprojeto & "\Previsão_" & Now.Year & mes & dia & hora & ".bmp"

        'Arquivo que deve ir para o FTP
        Dim caminhoarquivoFTP As String
        caminhoarquivoFTP = "/Prev.bmp"

        'Informe o nome servidor ftp ou ip 
        'Dim ftphost As String = "201.25.31.2"
        Dim ftphost As String = "187.52.189.10"


        Dim caminhoFTP As String = "ftp://" & ftphost & caminhoarquivoFTP
        Dim ftp As FtpWebRequest = DirectCast(FtpWebRequest.Create(caminhoFTP), FtpWebRequest)

        'define as credenciais 
        ftp.Credentials = New NetworkCredential("uhbg", "<REDACTED>")
        'define o tipo de ação 
        ftp.KeepAlive = True
        ftp.UseBinary = True
        ftp.Method = WebRequestMethods.Ftp.UploadFile
        'trata o retorno
        Dim fs As FileStream = File.OpenRead(arquivo)
        Dim buffer As Byte() = New Byte(fs.Length - 1) {}
        fs.Read(buffer, 0, buffer.Length)
        fs.Close()
        Dim ftpstream As Stream = ftp.GetRequestStream()
        ftpstream.Write(buffer, 0, buffer.Length)
        ftpstream.Close()

    End Sub

End Class