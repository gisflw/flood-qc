<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form16
    Inherits System.Windows.Forms.Form

    'Form overrides dispose to clean up the component list.
    <System.Diagnostics.DebuggerNonUserCode()> _
    Protected Overrides Sub Dispose(ByVal disposing As Boolean)
        Try
            If disposing AndAlso components IsNot Nothing Then
                components.Dispose()
            End If
        Finally
            MyBase.Dispose(disposing)
        End Try
    End Sub

    'Required by the Windows Form Designer
    Private components As System.ComponentModel.IContainer

    'NOTE: The following procedure is required by the Windows Form Designer
    'It can be modified using the Windows Form Designer.  
    'Do not modify it using the code editor.
    <System.Diagnostics.DebuggerStepThrough()> _
    Private Sub InitializeComponent()
        Dim resources As System.ComponentModel.ComponentResourceManager = New System.ComponentModel.ComponentResourceManager(GetType(Form16))
        Me.Label1 = New System.Windows.Forms.Label
        Me.Label2 = New System.Windows.Forms.Label
        Me.Label3 = New System.Windows.Forms.Label
        Me.Label4 = New System.Windows.Forms.Label
        Me.Label5 = New System.Windows.Forms.Label
        Me.Label6 = New System.Windows.Forms.Label
        Me.Button1 = New System.Windows.Forms.Button
        Me.txtServidorSMTP = New System.Windows.Forms.TextBox
        Me.txtOrigem = New System.Windows.Forms.TextBox
        Me.txtDestino = New System.Windows.Forms.TextBox
        Me.txtAssunto = New System.Windows.Forms.TextBox
        Me.txtMensagem = New System.Windows.Forms.TextBox
        Me.lstanexos = New System.Windows.Forms.ListBox
        Me.SuspendLayout()
        '
        'Label1
        '
        Me.Label1.AutoSize = True
        Me.Label1.Location = New System.Drawing.Point(24, 38)
        Me.Label1.Name = "Label1"
        Me.Label1.Size = New System.Drawing.Size(79, 13)
        Me.Label1.TabIndex = 0
        Me.Label1.Text = "Servidor SMTP"
        '
        'Label2
        '
        Me.Label2.AutoSize = True
        Me.Label2.Location = New System.Drawing.Point(24, 81)
        Me.Label2.Name = "Label2"
        Me.Label2.Size = New System.Drawing.Size(21, 13)
        Me.Label2.TabIndex = 1
        Me.Label2.Text = "De"
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(24, 117)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(29, 13)
        Me.Label3.TabIndex = 2
        Me.Label3.Text = "Para"
        '
        'Label4
        '
        Me.Label4.AutoSize = True
        Me.Label4.Location = New System.Drawing.Point(24, 157)
        Me.Label4.Name = "Label4"
        Me.Label4.Size = New System.Drawing.Size(42, 13)
        Me.Label4.TabIndex = 3
        Me.Label4.Text = "Anexos"
        '
        'Label5
        '
        Me.Label5.AutoSize = True
        Me.Label5.Location = New System.Drawing.Point(24, 223)
        Me.Label5.Name = "Label5"
        Me.Label5.Size = New System.Drawing.Size(45, 13)
        Me.Label5.TabIndex = 4
        Me.Label5.Text = "Assunto"
        '
        'Label6
        '
        Me.Label6.AutoSize = True
        Me.Label6.Location = New System.Drawing.Point(24, 266)
        Me.Label6.Name = "Label6"
        Me.Label6.Size = New System.Drawing.Size(59, 13)
        Me.Label6.TabIndex = 5
        Me.Label6.Text = "Mensagem"
        '
        'Button1
        '
        Me.Button1.Location = New System.Drawing.Point(517, 509)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(75, 41)
        Me.Button1.TabIndex = 6
        Me.Button1.Text = "Enviar"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'txtServidorSMTP
        '
        Me.txtServidorSMTP.Location = New System.Drawing.Point(109, 35)
        Me.txtServidorSMTP.Name = "txtServidorSMTP"
        Me.txtServidorSMTP.Size = New System.Drawing.Size(483, 20)
        Me.txtServidorSMTP.TabIndex = 7
        Me.txtServidorSMTP.Text = "smtp.gmail.com"
        '
        'txtOrigem
        '
        Me.txtOrigem.Location = New System.Drawing.Point(109, 78)
        Me.txtOrigem.Name = "txtOrigem"
        Me.txtOrigem.Size = New System.Drawing.Size(483, 20)
        Me.txtOrigem.TabIndex = 8
        Me.txtOrigem.Text = "fluvialengenharia@gmail.com"
        '
        'txtDestino
        '
        Me.txtDestino.Location = New System.Drawing.Point(109, 117)
        Me.txtDestino.Name = "txtDestino"
        Me.txtDestino.Size = New System.Drawing.Size(483, 20)
        Me.txtDestino.TabIndex = 9
        Me.txtDestino.Text = "fernando.fan@bol.com.br,  p.rogenes@gmail.com, diogo.buarque@gmail.com"
        '
        'txtAssunto
        '
        Me.txtAssunto.Location = New System.Drawing.Point(109, 220)
        Me.txtAssunto.Name = "txtAssunto"
        Me.txtAssunto.Size = New System.Drawing.Size(483, 20)
        Me.txtAssunto.TabIndex = 10
        Me.txtAssunto.Text = "Previsão Barra Grande"
        '
        'txtMensagem
        '
        Me.txtMensagem.Location = New System.Drawing.Point(109, 263)
        Me.txtMensagem.Multiline = True
        Me.txtMensagem.Name = "txtMensagem"
        Me.txtMensagem.Size = New System.Drawing.Size(483, 240)
        Me.txtMensagem.TabIndex = 11
        Me.txtMensagem.Text = resources.GetString("txtMensagem.Text")
        '
        'lstanexos
        '
        Me.lstanexos.FormattingEnabled = True
        Me.lstanexos.Location = New System.Drawing.Point(109, 151)
        Me.lstanexos.Name = "lstanexos"
        Me.lstanexos.Size = New System.Drawing.Size(483, 56)
        Me.lstanexos.TabIndex = 12
        '
        'Form16
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(627, 562)
        Me.Controls.Add(Me.lstanexos)
        Me.Controls.Add(Me.txtMensagem)
        Me.Controls.Add(Me.txtAssunto)
        Me.Controls.Add(Me.txtDestino)
        Me.Controls.Add(Me.txtOrigem)
        Me.Controls.Add(Me.txtServidorSMTP)
        Me.Controls.Add(Me.Button1)
        Me.Controls.Add(Me.Label6)
        Me.Controls.Add(Me.Label5)
        Me.Controls.Add(Me.Label4)
        Me.Controls.Add(Me.Label3)
        Me.Controls.Add(Me.Label2)
        Me.Controls.Add(Me.Label1)
        Me.Name = "Form16"
        Me.Text = "Email"
        Me.ResumeLayout(False)
        Me.PerformLayout()

    End Sub
    Friend WithEvents Label1 As System.Windows.Forms.Label
    Friend WithEvents Label2 As System.Windows.Forms.Label
    Friend WithEvents Label3 As System.Windows.Forms.Label
    Friend WithEvents Label4 As System.Windows.Forms.Label
    Friend WithEvents Label5 As System.Windows.Forms.Label
    Friend WithEvents Label6 As System.Windows.Forms.Label
    Friend WithEvents Button1 As System.Windows.Forms.Button
    Friend WithEvents txtServidorSMTP As System.Windows.Forms.TextBox
    Friend WithEvents txtOrigem As System.Windows.Forms.TextBox
    Friend WithEvents txtDestino As System.Windows.Forms.TextBox
    Friend WithEvents txtAssunto As System.Windows.Forms.TextBox
    Friend WithEvents txtMensagem As System.Windows.Forms.TextBox
    Friend WithEvents lstanexos As System.Windows.Forms.ListBox
End Class
