<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form12b
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
        Me.components = New System.ComponentModel.Container
        Me.zg1 = New ZedGraph.ZedGraphControl
        Me.zg2 = New ZedGraph.ZedGraphControl
        Me.Panel1 = New System.Windows.Forms.Panel
        Me.Panel2 = New System.Windows.Forms.Panel
        Me.Button1 = New System.Windows.Forms.Button
        Me.Button2 = New System.Windows.Forms.Button
        Me.Panel3 = New System.Windows.Forms.Panel
        Me.sfd = New System.Windows.Forms.SaveFileDialog
        Me.Panel1.SuspendLayout()
        Me.Panel2.SuspendLayout()
        Me.Panel3.SuspendLayout()
        Me.SuspendLayout()
        '
        'zg1
        '
        Me.zg1.Dock = System.Windows.Forms.DockStyle.Fill
        Me.zg1.Location = New System.Drawing.Point(0, 0)
        Me.zg1.Name = "zg1"
        Me.zg1.ScrollGrace = 0
        Me.zg1.ScrollMaxX = 0
        Me.zg1.ScrollMaxY = 0
        Me.zg1.ScrollMaxY2 = 0
        Me.zg1.ScrollMinX = 0
        Me.zg1.ScrollMinY = 0
        Me.zg1.ScrollMinY2 = 0
        Me.zg1.Size = New System.Drawing.Size(1132, 355)
        Me.zg1.TabIndex = 1
        '
        'zg2
        '
        Me.zg2.Dock = System.Windows.Forms.DockStyle.Fill
        Me.zg2.Location = New System.Drawing.Point(0, 0)
        Me.zg2.Name = "zg2"
        Me.zg2.ScrollGrace = 0
        Me.zg2.ScrollMaxX = 0
        Me.zg2.ScrollMaxY = 0
        Me.zg2.ScrollMaxY2 = 0
        Me.zg2.ScrollMinX = 0
        Me.zg2.ScrollMinY = 0
        Me.zg2.ScrollMinY2 = 0
        Me.zg2.Size = New System.Drawing.Size(1132, 348)
        Me.zg2.TabIndex = 2
        '
        'Panel1
        '
        Me.Panel1.Controls.Add(Me.zg1)
        Me.Panel1.Location = New System.Drawing.Point(7, 357)
        Me.Panel1.Name = "Panel1"
        Me.Panel1.Size = New System.Drawing.Size(1132, 355)
        Me.Panel1.TabIndex = 3
        '
        'Panel2
        '
        Me.Panel2.Controls.Add(Me.zg2)
        Me.Panel2.Location = New System.Drawing.Point(7, 3)
        Me.Panel2.Name = "Panel2"
        Me.Panel2.Size = New System.Drawing.Size(1132, 348)
        Me.Panel2.TabIndex = 4
        '
        'Button1
        '
        Me.Button1.Location = New System.Drawing.Point(957, 733)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(94, 37)
        Me.Button1.TabIndex = 5
        Me.Button1.Text = "Salvar"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'Button2
        '
        Me.Button2.Location = New System.Drawing.Point(1057, 733)
        Me.Button2.Name = "Button2"
        Me.Button2.Size = New System.Drawing.Size(94, 37)
        Me.Button2.TabIndex = 6
        Me.Button2.Text = "Fechar"
        Me.Button2.UseVisualStyleBackColor = True
        '
        'Panel3
        '
        Me.Panel3.Controls.Add(Me.Panel2)
        Me.Panel3.Controls.Add(Me.Panel1)
        Me.Panel3.Location = New System.Drawing.Point(12, 12)
        Me.Panel3.Name = "Panel3"
        Me.Panel3.Size = New System.Drawing.Size(1147, 715)
        Me.Panel3.TabIndex = 7
        '
        'Form12
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.BackColor = System.Drawing.Color.White
        Me.ClientSize = New System.Drawing.Size(1175, 782)
        Me.Controls.Add(Me.Panel3)
        Me.Controls.Add(Me.Button2)
        Me.Controls.Add(Me.Button1)
        Me.FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedSingle
        Me.MaximizeBox = False
        Me.Name = "Form12b"
        Me.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen
        Me.Text = "Hidrograma Padrão 02"
        Me.Panel1.ResumeLayout(False)
        Me.Panel2.ResumeLayout(False)
        Me.Panel3.ResumeLayout(False)
        Me.ResumeLayout(False)

    End Sub
    Friend WithEvents zg1 As ZedGraph.ZedGraphControl
    Friend WithEvents zg2 As ZedGraph.ZedGraphControl
    Friend WithEvents Panel1 As System.Windows.Forms.Panel
    Friend WithEvents Panel2 As System.Windows.Forms.Panel
    Friend WithEvents Button1 As System.Windows.Forms.Button
    Friend WithEvents Button2 As System.Windows.Forms.Button
    Friend WithEvents Panel3 As System.Windows.Forms.Panel
    Friend WithEvents sfd As System.Windows.Forms.SaveFileDialog
End Class
