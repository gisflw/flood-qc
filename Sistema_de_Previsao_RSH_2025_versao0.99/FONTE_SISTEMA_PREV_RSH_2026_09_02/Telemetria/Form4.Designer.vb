<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form4
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
        Me.SuspendLayout()
        '
        'zg1
        '
        Me.zg1.Location = New System.Drawing.Point(111, 59)
        Me.zg1.Name = "zg1"
        Me.zg1.ScrollGrace = 0
        Me.zg1.ScrollMaxX = 0
        Me.zg1.ScrollMaxY = 0
        Me.zg1.ScrollMaxY2 = 0
        Me.zg1.ScrollMinX = 0
        Me.zg1.ScrollMinY = 0
        Me.zg1.ScrollMinY2 = 0
        Me.zg1.Size = New System.Drawing.Size(417, 218)
        Me.zg1.TabIndex = 0
        '
        'Form4
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(739, 356)
        Me.Controls.Add(Me.zg1)
        Me.Name = "Form4"
        Me.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen
        Me.Text = "Visualizar Gráficos"
        Me.ResumeLayout(False)

    End Sub
    Friend WithEvents zg1 As ZedGraph.ZedGraphControl
End Class
