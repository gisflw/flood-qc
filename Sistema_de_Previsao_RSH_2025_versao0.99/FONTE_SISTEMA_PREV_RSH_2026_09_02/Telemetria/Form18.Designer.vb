<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form18
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
        Me.GroupBox1 = New System.Windows.Forms.GroupBox
        Me.btBinPrev = New System.Windows.Forms.Button
        Me.btInputFile = New System.Windows.Forms.Button
        Me.btShapeBase = New System.Windows.Forms.Button
        Me.txtPrev = New System.Windows.Forms.TextBox
        Me.txtInput = New System.Windows.Forms.TextBox
        Me.txtShape = New System.Windows.Forms.TextBox
        Me.Label3 = New System.Windows.Forms.Label
        Me.Label2 = New System.Windows.Forms.Label
        Me.Label1 = New System.Windows.Forms.Label
        Me.btGeraShape = New System.Windows.Forms.Button
        Me.Label4 = New System.Windows.Forms.Label
        Me.tbFatorSeg = New System.Windows.Forms.TextBox
        Me.GroupBox1.SuspendLayout()
        Me.SuspendLayout()
        '
        'GroupBox1
        '
        Me.GroupBox1.Controls.Add(Me.btBinPrev)
        Me.GroupBox1.Controls.Add(Me.btInputFile)
        Me.GroupBox1.Controls.Add(Me.btShapeBase)
        Me.GroupBox1.Controls.Add(Me.txtPrev)
        Me.GroupBox1.Controls.Add(Me.txtInput)
        Me.GroupBox1.Controls.Add(Me.txtShape)
        Me.GroupBox1.Controls.Add(Me.Label3)
        Me.GroupBox1.Controls.Add(Me.Label2)
        Me.GroupBox1.Controls.Add(Me.Label1)
        Me.GroupBox1.Location = New System.Drawing.Point(12, 12)
        Me.GroupBox1.Name = "GroupBox1"
        Me.GroupBox1.Size = New System.Drawing.Size(503, 112)
        Me.GroupBox1.TabIndex = 0
        Me.GroupBox1.TabStop = False
        Me.GroupBox1.Text = "Arquivos para gerar shapefile"
        '
        'btBinPrev
        '
        Me.btBinPrev.Location = New System.Drawing.Point(448, 74)
        Me.btBinPrev.Name = "btBinPrev"
        Me.btBinPrev.Size = New System.Drawing.Size(42, 23)
        Me.btBinPrev.TabIndex = 8
        Me.btBinPrev.Text = "..."
        Me.btBinPrev.UseVisualStyleBackColor = True
        '
        'btInputFile
        '
        Me.btInputFile.Location = New System.Drawing.Point(448, 48)
        Me.btInputFile.Name = "btInputFile"
        Me.btInputFile.Size = New System.Drawing.Size(42, 23)
        Me.btInputFile.TabIndex = 7
        Me.btInputFile.Text = "..."
        Me.btInputFile.UseVisualStyleBackColor = True
        '
        'btShapeBase
        '
        Me.btShapeBase.Location = New System.Drawing.Point(448, 22)
        Me.btShapeBase.Name = "btShapeBase"
        Me.btShapeBase.Size = New System.Drawing.Size(42, 23)
        Me.btShapeBase.TabIndex = 6
        Me.btShapeBase.Text = "..."
        Me.btShapeBase.UseVisualStyleBackColor = True
        '
        'txtPrev
        '
        Me.txtPrev.Location = New System.Drawing.Point(93, 76)
        Me.txtPrev.Name = "txtPrev"
        Me.txtPrev.Size = New System.Drawing.Size(349, 20)
        Me.txtPrev.TabIndex = 5
        '
        'txtInput
        '
        Me.txtInput.Location = New System.Drawing.Point(93, 50)
        Me.txtInput.Name = "txtInput"
        Me.txtInput.Size = New System.Drawing.Size(349, 20)
        Me.txtInput.TabIndex = 4
        '
        'txtShape
        '
        Me.txtShape.Location = New System.Drawing.Point(93, 24)
        Me.txtShape.Name = "txtShape"
        Me.txtShape.Size = New System.Drawing.Size(349, 20)
        Me.txtShape.TabIndex = 3
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(6, 79)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(86, 13)
        Me.Label3.TabIndex = 2
        Me.Label3.Text = "Binário Previsão:"
        '
        'Label2
        '
        Me.Label2.AutoSize = True
        Me.Label2.Location = New System.Drawing.Point(6, 53)
        Me.Label2.Name = "Label2"
        Me.Label2.Size = New System.Drawing.Size(73, 13)
        Me.Label2.TabIndex = 1
        Me.Label2.Text = "Arquivo Input:"
        '
        'Label1
        '
        Me.Label1.AutoSize = True
        Me.Label1.Location = New System.Drawing.Point(6, 27)
        Me.Label1.Name = "Label1"
        Me.Label1.Size = New System.Drawing.Size(81, 13)
        Me.Label1.TabIndex = 0
        Me.Label1.Text = "Shapefile Base:"
        '
        'btGeraShape
        '
        Me.btGeraShape.Location = New System.Drawing.Point(375, 130)
        Me.btGeraShape.Name = "btGeraShape"
        Me.btGeraShape.Size = New System.Drawing.Size(140, 36)
        Me.btGeraShape.TabIndex = 1
        Me.btGeraShape.Text = "Gerar shapefile"
        Me.btGeraShape.UseVisualStyleBackColor = True
        '
        'Label4
        '
        Me.Label4.AutoSize = True
        Me.Label4.Location = New System.Drawing.Point(12, 153)
        Me.Label4.Name = "Label4"
        Me.Label4.Size = New System.Drawing.Size(104, 13)
        Me.Label4.TabIndex = 2
        Me.Label4.Text = "Fator de Segurança:"
        '
        'tbFatorSeg
        '
        Me.tbFatorSeg.Location = New System.Drawing.Point(117, 150)
        Me.tbFatorSeg.Name = "tbFatorSeg"
        Me.tbFatorSeg.Size = New System.Drawing.Size(29, 20)
        Me.tbFatorSeg.TabIndex = 3
        Me.tbFatorSeg.Text = "1"
        '
        'Form18
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(527, 175)
        Me.Controls.Add(Me.tbFatorSeg)
        Me.Controls.Add(Me.Label4)
        Me.Controls.Add(Me.btGeraShape)
        Me.Controls.Add(Me.GroupBox1)
        Me.Name = "Form18"
        Me.Text = "Mapa de limiares de inundação"
        Me.GroupBox1.ResumeLayout(False)
        Me.GroupBox1.PerformLayout()
        Me.ResumeLayout(False)
        Me.PerformLayout()

    End Sub
    Friend WithEvents GroupBox1 As System.Windows.Forms.GroupBox
    Friend WithEvents Label3 As System.Windows.Forms.Label
    Friend WithEvents Label2 As System.Windows.Forms.Label
    Friend WithEvents Label1 As System.Windows.Forms.Label
    Friend WithEvents btBinPrev As System.Windows.Forms.Button
    Friend WithEvents btInputFile As System.Windows.Forms.Button
    Friend WithEvents btShapeBase As System.Windows.Forms.Button
    Friend WithEvents txtPrev As System.Windows.Forms.TextBox
    Friend WithEvents txtInput As System.Windows.Forms.TextBox
    Friend WithEvents txtShape As System.Windows.Forms.TextBox
    Friend WithEvents btGeraShape As System.Windows.Forms.Button
    Friend WithEvents Label4 As System.Windows.Forms.Label
    Friend WithEvents tbFatorSeg As System.Windows.Forms.TextBox
End Class
