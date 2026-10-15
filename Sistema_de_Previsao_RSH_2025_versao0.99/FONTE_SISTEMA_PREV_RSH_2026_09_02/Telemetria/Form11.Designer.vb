<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form11
    Inherits System.Windows.Forms.Form

    'Descartar substituições de formulário para limpar a lista de componentes.
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

    'Exigido pelo Windows Form Designer
    Private components As System.ComponentModel.IContainer

    'OBSERVAÇÃO: o procedimento a seguir é exigido pelo Windows Form Designer
    'Pode ser modificado usando o Windows Form Designer.  
    'Não o modifique usando o editor de códigos.
    <System.Diagnostics.DebuggerStepThrough()> _
    Private Sub InitializeComponent()
        Me.components = New System.ComponentModel.Container()
        Me.zg1 = New ZedGraph.ZedGraphControl()
        Me.GroupBox1 = New System.Windows.Forms.GroupBox()
        Me.bt_dud_minus = New System.Windows.Forms.Button()
        Me.bt_dud_plus = New System.Windows.Forms.Button()
        Me.dud_alfa2 = New System.Windows.Forms.TextBox()
        Me.btMINUS = New System.Windows.Forms.Button()
        Me.btPLUS = New System.Windows.Forms.Button()
        Me.TextBox1 = New System.Windows.Forms.TextBox()
        Me.chkBandas = New System.Windows.Forms.CheckBox()
        Me.Button5 = New System.Windows.Forms.Button()
        Me.btnZoomUltimos21d = New System.Windows.Forms.Button()
        Me.Button4 = New System.Windows.Forms.Button()
        Me.Button3 = New System.Windows.Forms.Button()
        Me.Button2 = New System.Windows.Forms.Button()
        Me.tbVIES = New System.Windows.Forms.TextBox()
        Me.tbMini = New System.Windows.Forms.TextBox()
        Me.Label3 = New System.Windows.Forms.Label()
        Me.Button1 = New System.Windows.Forms.Button()
        Me.dud_alfa = New System.Windows.Forms.DomainUpDown()
        Me.Label2 = New System.Windows.Forms.Label()
        Me.Label1 = New System.Windows.Forms.Label()
        Me.Button6 = New System.Windows.Forms.Button()
        Me.GroupBox1.SuspendLayout()
        Me.SuspendLayout()
        '
        'zg1
        '
        Me.zg1.Location = New System.Drawing.Point(12, 12)
        Me.zg1.Name = "zg1"
        Me.zg1.ScrollGrace = 0R
        Me.zg1.ScrollMaxX = 0R
        Me.zg1.ScrollMaxY = 0R
        Me.zg1.ScrollMaxY2 = 0R
        Me.zg1.ScrollMinX = 0R
        Me.zg1.ScrollMinY = 0R
        Me.zg1.ScrollMinY2 = 0R
        Me.zg1.Size = New System.Drawing.Size(804, 489)
        Me.zg1.TabIndex = 1
        '
        'GroupBox1
        '
        Me.GroupBox1.Controls.Add(Me.Button6)
        Me.GroupBox1.Controls.Add(Me.bt_dud_minus)
        Me.GroupBox1.Controls.Add(Me.bt_dud_plus)
        Me.GroupBox1.Controls.Add(Me.dud_alfa2)
        Me.GroupBox1.Controls.Add(Me.btMINUS)
        Me.GroupBox1.Controls.Add(Me.btPLUS)
        Me.GroupBox1.Controls.Add(Me.TextBox1)
        Me.GroupBox1.Controls.Add(Me.chkBandas)
        Me.GroupBox1.Controls.Add(Me.Button5)
        Me.GroupBox1.Controls.Add(Me.btnZoomUltimos21d)
        Me.GroupBox1.Controls.Add(Me.Button4)
        Me.GroupBox1.Controls.Add(Me.Button3)
        Me.GroupBox1.Controls.Add(Me.Button2)
        Me.GroupBox1.Controls.Add(Me.tbVIES)
        Me.GroupBox1.Controls.Add(Me.tbMini)
        Me.GroupBox1.Controls.Add(Me.Label3)
        Me.GroupBox1.Controls.Add(Me.Button1)
        Me.GroupBox1.Controls.Add(Me.dud_alfa)
        Me.GroupBox1.Controls.Add(Me.Label2)
        Me.GroupBox1.Controls.Add(Me.Label1)
        Me.GroupBox1.Dock = System.Windows.Forms.DockStyle.Right
        Me.GroupBox1.Location = New System.Drawing.Point(834, 0)
        Me.GroupBox1.Name = "GroupBox1"
        Me.GroupBox1.Size = New System.Drawing.Size(200, 513)
        Me.GroupBox1.TabIndex = 2
        Me.GroupBox1.TabStop = False
        Me.GroupBox1.Text = "Parâmetros"
        '
        'bt_dud_minus
        '
        Me.bt_dud_minus.Location = New System.Drawing.Point(137, 54)
        Me.bt_dud_minus.Name = "bt_dud_minus"
        Me.bt_dud_minus.Size = New System.Drawing.Size(19, 20)
        Me.bt_dud_minus.TabIndex = 19
        Me.bt_dud_minus.Text = "-"
        Me.bt_dud_minus.UseVisualStyleBackColor = True
        '
        'bt_dud_plus
        '
        Me.bt_dud_plus.Location = New System.Drawing.Point(116, 59)
        Me.bt_dud_plus.Name = "bt_dud_plus"
        Me.bt_dud_plus.Size = New System.Drawing.Size(19, 20)
        Me.bt_dud_plus.TabIndex = 18
        Me.bt_dud_plus.Text = "+"
        Me.bt_dud_plus.UseVisualStyleBackColor = True
        '
        'dud_alfa2
        '
        Me.dud_alfa2.Location = New System.Drawing.Point(60, 59)
        Me.dud_alfa2.Name = "dud_alfa2"
        Me.dud_alfa2.Size = New System.Drawing.Size(52, 20)
        Me.dud_alfa2.TabIndex = 17
        Me.dud_alfa2.Text = "0.97"
        '
        'btMINUS
        '
        Me.btMINUS.Location = New System.Drawing.Point(137, 81)
        Me.btMINUS.Name = "btMINUS"
        Me.btMINUS.Size = New System.Drawing.Size(19, 20)
        Me.btMINUS.TabIndex = 16
        Me.btMINUS.Text = "-"
        Me.btMINUS.UseVisualStyleBackColor = True
        '
        'btPLUS
        '
        Me.btPLUS.Location = New System.Drawing.Point(116, 89)
        Me.btPLUS.Name = "btPLUS"
        Me.btPLUS.Size = New System.Drawing.Size(19, 20)
        Me.btPLUS.TabIndex = 15
        Me.btPLUS.Text = "+"
        Me.btPLUS.UseVisualStyleBackColor = True
        '
        'TextBox1
        '
        Me.TextBox1.Location = New System.Drawing.Point(151, 121)
        Me.TextBox1.Name = "TextBox1"
        Me.TextBox1.Size = New System.Drawing.Size(27, 20)
        Me.TextBox1.TabIndex = 14
        Me.TextBox1.Text = "10"
        '
        'chkBandas
        '
        Me.chkBandas.AutoSize = True
        Me.chkBandas.Location = New System.Drawing.Point(16, 123)
        Me.chkBandas.Name = "chkBandas"
        Me.chkBandas.Size = New System.Drawing.Size(179, 17)
        Me.chkBandas.TabIndex = 13
        Me.chkBandas.Text = "Bandas de incerteza de           %"
        Me.chkBandas.UseVisualStyleBackColor = True
        '
        'Button5
        '
        Me.Button5.Location = New System.Drawing.Point(113, 230)
        Me.Button5.Name = "Button5"
        Me.Button5.Size = New System.Drawing.Size(75, 46)
        Me.Button5.TabIndex = 12
        Me.Button5.Text = "Zoom extend"
        Me.Button5.UseVisualStyleBackColor = True
        '
        'btnZoomUltimos21d
        '
        Me.btnZoomUltimos21d.Location = New System.Drawing.Point(19, 230)
        Me.btnZoomUltimos21d.Name = "btnZoomUltimos21d"
        Me.btnZoomUltimos21d.Size = New System.Drawing.Size(88, 46)
        Me.btnZoomUltimos21d.TabIndex = 11
        Me.btnZoomUltimos21d.Text = "Centrar gráfico na previsão"
        Me.btnZoomUltimos21d.UseVisualStyleBackColor = True
        '
        'Button4
        '
        Me.Button4.Dock = System.Windows.Forms.DockStyle.Bottom
        Me.Button4.Location = New System.Drawing.Point(3, 452)
        Me.Button4.Name = "Button4"
        Me.Button4.Size = New System.Drawing.Size(194, 58)
        Me.Button4.TabIndex = 10
        Me.Button4.Text = "Fechar"
        Me.Button4.UseVisualStyleBackColor = True
        '
        'Button3
        '
        Me.Button3.Location = New System.Drawing.Point(16, 349)
        Me.Button3.Name = "Button3"
        Me.Button3.Size = New System.Drawing.Size(172, 43)
        Me.Button3.TabIndex = 9
        Me.Button3.Text = "Copiar Y previsão para área de transferência"
        Me.Button3.UseVisualStyleBackColor = True
        '
        'Button2
        '
        Me.Button2.Location = New System.Drawing.Point(16, 297)
        Me.Button2.Name = "Button2"
        Me.Button2.Size = New System.Drawing.Size(172, 46)
        Me.Button2.TabIndex = 8
        Me.Button2.Text = "Exportar dados para texto"
        Me.Button2.UseVisualStyleBackColor = True
        '
        'tbVIES
        '
        Me.tbVIES.Location = New System.Drawing.Point(60, 85)
        Me.tbVIES.Name = "tbVIES"
        Me.tbVIES.Size = New System.Drawing.Size(52, 20)
        Me.tbVIES.TabIndex = 7
        Me.tbVIES.Text = "0.00"
        '
        'tbMini
        '
        Me.tbMini.Location = New System.Drawing.Point(110, 27)
        Me.tbMini.Name = "tbMini"
        Me.tbMini.Size = New System.Drawing.Size(50, 20)
        Me.tbMini.TabIndex = 6
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(13, 30)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(91, 13)
        Me.Label3.TabIndex = 5
        Me.Label3.Text = "Minibacia (trecho)"
        '
        'Button1
        '
        Me.Button1.Location = New System.Drawing.Point(16, 164)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(172, 46)
        Me.Button1.TabIndex = 4
        Me.Button1.Text = "Atualizar Gráfico"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'dud_alfa
        '
        Me.dud_alfa.Location = New System.Drawing.Point(60, 59)
        Me.dud_alfa.Name = "dud_alfa"
        Me.dud_alfa.Size = New System.Drawing.Size(52, 20)
        Me.dud_alfa.TabIndex = 2
        Me.dud_alfa.Text = "0.95"
        '
        'Label2
        '
        Me.Label2.AutoSize = True
        Me.Label2.Location = New System.Drawing.Point(13, 89)
        Me.Label2.Name = "Label2"
        Me.Label2.Size = New System.Drawing.Size(44, 13)
        Me.Label2.TabIndex = 1
        Me.Label2.Text = "Viés (m)"
        '
        'Label1
        '
        Me.Label1.AutoSize = True
        Me.Label1.Location = New System.Drawing.Point(13, 61)
        Me.Label1.Name = "Label1"
        Me.Label1.Size = New System.Drawing.Size(25, 13)
        Me.Label1.TabIndex = 0
        Me.Label1.Text = "Alfa"
        '
        'Button6
        '
        Me.Button6.Location = New System.Drawing.Point(16, 398)
        Me.Button6.Name = "Button6"
        Me.Button6.Size = New System.Drawing.Size(172, 43)
        Me.Button6.TabIndex = 20
        Me.Button6.Text = "Copiar X,Y previsão para área de transferência"
        Me.Button6.UseVisualStyleBackColor = True
        '
        'Form11
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(1034, 513)
        Me.Controls.Add(Me.GroupBox1)
        Me.Controls.Add(Me.zg1)
        Me.Name = "Form11"
        Me.Text = "Visualizar gráfico"
        Me.GroupBox1.ResumeLayout(False)
        Me.GroupBox1.PerformLayout()
        Me.ResumeLayout(False)

    End Sub

    Friend WithEvents zg1 As ZedGraph.ZedGraphControl
    Friend WithEvents GroupBox1 As Windows.Forms.GroupBox
    Friend WithEvents Button1 As Windows.Forms.Button
    Friend WithEvents dud_alfa As Windows.Forms.DomainUpDown
    Friend WithEvents Label2 As Windows.Forms.Label
    Friend WithEvents Label1 As Windows.Forms.Label
    Friend WithEvents tbMini As Windows.Forms.TextBox
    Friend WithEvents Label3 As Windows.Forms.Label
    Friend WithEvents tbVIES As Windows.Forms.TextBox
    Friend WithEvents Button2 As Windows.Forms.Button
    Friend WithEvents Button3 As Windows.Forms.Button
    Friend WithEvents Button4 As Windows.Forms.Button
    Friend WithEvents btnZoomUltimos21d As Windows.Forms.Button
    Friend WithEvents Button5 As Windows.Forms.Button
    Friend WithEvents TextBox1 As Windows.Forms.TextBox
    Friend WithEvents chkBandas As Windows.Forms.CheckBox
    Friend WithEvents btMINUS As Windows.Forms.Button
    Friend WithEvents btPLUS As Windows.Forms.Button
    Friend WithEvents bt_dud_minus As Windows.Forms.Button
    Friend WithEvents bt_dud_plus As Windows.Forms.Button
    Friend WithEvents dud_alfa2 As Windows.Forms.TextBox
    Friend WithEvents Button6 As Windows.Forms.Button
End Class
