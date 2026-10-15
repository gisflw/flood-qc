<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Editor
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
        Me.TabControl1 = New System.Windows.Forms.TabControl()
        Me.TabPage1 = New System.Windows.Forms.TabPage()
        Me.GroupBox1 = New System.Windows.Forms.GroupBox()
        Me.dtgvMINI = New System.Windows.Forms.DataGridView()
        Me.TabPage2 = New System.Windows.Forms.TabPage()
        Me.GroupBox6 = New System.Windows.Forms.GroupBox()
        Me.Button6 = New System.Windows.Forms.Button()
        Me.Label15 = New System.Windows.Forms.Label()
        Me.GroupBox5 = New System.Windows.Forms.GroupBox()
        Me.Label12 = New System.Windows.Forms.Label()
        Me.tb_RUG_SelectSubb = New System.Windows.Forms.TextBox()
        Me.Label11 = New System.Windows.Forms.Label()
        Me.Button4 = New System.Windows.Forms.Button()
        Me.tbRUGMAN_subb = New System.Windows.Forms.TextBox()
        Me.Label10 = New System.Windows.Forms.Label()
        Me.GroupBox4 = New System.Windows.Forms.GroupBox()
        Me.tbD_b = New System.Windows.Forms.TextBox()
        Me.Label7 = New System.Windows.Forms.Label()
        Me.Label8 = New System.Windows.Forms.Label()
        Me.Button3 = New System.Windows.Forms.Button()
        Me.tbD_a = New System.Windows.Forms.TextBox()
        Me.Label9 = New System.Windows.Forms.Label()
        Me.GroupBox3 = New System.Windows.Forms.GroupBox()
        Me.tbW_b = New System.Windows.Forms.TextBox()
        Me.Label6 = New System.Windows.Forms.Label()
        Me.Label5 = New System.Windows.Forms.Label()
        Me.Button2 = New System.Windows.Forms.Button()
        Me.tbW_a = New System.Windows.Forms.TextBox()
        Me.Label2 = New System.Windows.Forms.Label()
        Me.GroupBox2 = New System.Windows.Forms.GroupBox()
        Me.Button1 = New System.Windows.Forms.Button()
        Me.tbMANNING = New System.Windows.Forms.TextBox()
        Me.Label1 = New System.Windows.Forms.Label()
        Me.tbcell = New System.Windows.Forms.TextBox()
        Me.Label4 = New System.Windows.Forms.Label()
        Me.ofd1 = New System.Windows.Forms.OpenFileDialog()
        Me.Button5 = New System.Windows.Forms.Button()
        Me.tbofdbloc = New System.Windows.Forms.TextBox()
        Me.Label3 = New System.Windows.Forms.Label()
        Me.btFECHAR = New System.Windows.Forms.Button()
        Me.btSALVARMini = New System.Windows.Forms.Button()
        Me.btABRIRMINI = New System.Windows.Forms.Button()
        Me.sfd1 = New System.Windows.Forms.SaveFileDialog()
        Me.TabControl1.SuspendLayout()
        Me.TabPage1.SuspendLayout()
        Me.GroupBox1.SuspendLayout()
        CType(Me.dtgvMINI, System.ComponentModel.ISupportInitialize).BeginInit()
        Me.TabPage2.SuspendLayout()
        Me.GroupBox6.SuspendLayout()
        Me.GroupBox5.SuspendLayout()
        Me.GroupBox4.SuspendLayout()
        Me.GroupBox3.SuspendLayout()
        Me.GroupBox2.SuspendLayout()
        Me.SuspendLayout()
        '
        'TabControl1
        '
        Me.TabControl1.Controls.Add(Me.TabPage1)
        Me.TabControl1.Controls.Add(Me.TabPage2)
        Me.TabControl1.Location = New System.Drawing.Point(12, 75)
        Me.TabControl1.Name = "TabControl1"
        Me.TabControl1.SelectedIndex = 0
        Me.TabControl1.Size = New System.Drawing.Size(1175, 377)
        Me.TabControl1.TabIndex = 40
        '
        'TabPage1
        '
        Me.TabPage1.Controls.Add(Me.GroupBox1)
        Me.TabPage1.Location = New System.Drawing.Point(4, 22)
        Me.TabPage1.Name = "TabPage1"
        Me.TabPage1.Padding = New System.Windows.Forms.Padding(3)
        Me.TabPage1.Size = New System.Drawing.Size(1167, 351)
        Me.TabPage1.TabIndex = 0
        Me.TabPage1.Text = "Editor"
        Me.TabPage1.UseVisualStyleBackColor = True
        '
        'GroupBox1
        '
        Me.GroupBox1.Controls.Add(Me.dtgvMINI)
        Me.GroupBox1.Location = New System.Drawing.Point(6, 6)
        Me.GroupBox1.Name = "GroupBox1"
        Me.GroupBox1.Size = New System.Drawing.Size(1153, 336)
        Me.GroupBox1.TabIndex = 10
        Me.GroupBox1.TabStop = False
        Me.GroupBox1.Text = "Geometry file (MINI.gtp) for edition:"
        '
        'dtgvMINI
        '
        Me.dtgvMINI.ColumnHeadersHeightSizeMode = System.Windows.Forms.DataGridViewColumnHeadersHeightSizeMode.AutoSize
        Me.dtgvMINI.DataMember = "Table1"
        Me.dtgvMINI.Dock = System.Windows.Forms.DockStyle.Fill
        Me.dtgvMINI.Location = New System.Drawing.Point(3, 16)
        Me.dtgvMINI.Name = "dtgvMINI"
        Me.dtgvMINI.Size = New System.Drawing.Size(1147, 317)
        Me.dtgvMINI.TabIndex = 0
        '
        'TabPage2
        '
        Me.TabPage2.Controls.Add(Me.GroupBox6)
        Me.TabPage2.Controls.Add(Me.GroupBox5)
        Me.TabPage2.Controls.Add(Me.GroupBox4)
        Me.TabPage2.Controls.Add(Me.GroupBox3)
        Me.TabPage2.Controls.Add(Me.GroupBox2)
        Me.TabPage2.Location = New System.Drawing.Point(4, 22)
        Me.TabPage2.Name = "TabPage2"
        Me.TabPage2.Padding = New System.Windows.Forms.Padding(3)
        Me.TabPage2.Size = New System.Drawing.Size(1167, 351)
        Me.TabPage2.TabIndex = 1
        Me.TabPage2.Text = "Shortcuts"
        Me.TabPage2.UseVisualStyleBackColor = True
        '
        'GroupBox6
        '
        Me.GroupBox6.Controls.Add(Me.Button6)
        Me.GroupBox6.Controls.Add(Me.Label15)
        Me.GroupBox6.Location = New System.Drawing.Point(381, 118)
        Me.GroupBox6.Name = "GroupBox6"
        Me.GroupBox6.Size = New System.Drawing.Size(388, 208)
        Me.GroupBox6.TabIndex = 7
        Me.GroupBox6.TabStop = False
        Me.GroupBox6.Text = "River Channel Widths and Depths by table ("".csv"" "";"")"
        '
        'Button6
        '
        Me.Button6.Location = New System.Drawing.Point(127, 119)
        Me.Button6.Name = "Button6"
        Me.Button6.Size = New System.Drawing.Size(114, 38)
        Me.Button6.TabIndex = 2
        Me.Button6.Text = "Load and Apply"
        Me.Button6.UseVisualStyleBackColor = True
        '
        'Label15
        '
        Me.Label15.AutoSize = True
        Me.Label15.Location = New System.Drawing.Point(6, 25)
        Me.Label15.Name = "Label15"
        Me.Label15.Size = New System.Drawing.Size(353, 65)
        Me.Label15.TabIndex = 0
        Me.Label15.Text = "Load a River Channel Widths and Depths table "".csv"" and "";"" separated." & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "- The t" & _
            "able format must be the following:" & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "- Mini;Width(m);Depth(m)"
        '
        'GroupBox5
        '
        Me.GroupBox5.Controls.Add(Me.Label12)
        Me.GroupBox5.Controls.Add(Me.tb_RUG_SelectSubb)
        Me.GroupBox5.Controls.Add(Me.Label11)
        Me.GroupBox5.Controls.Add(Me.Button4)
        Me.GroupBox5.Controls.Add(Me.tbRUGMAN_subb)
        Me.GroupBox5.Controls.Add(Me.Label10)
        Me.GroupBox5.Location = New System.Drawing.Point(381, 6)
        Me.GroupBox5.Name = "GroupBox5"
        Me.GroupBox5.Size = New System.Drawing.Size(388, 106)
        Me.GroupBox5.TabIndex = 3
        Me.GroupBox5.TabStop = False
        Me.GroupBox5.Text = "Manning roughness coefficient by sub-basin"
        '
        'Label12
        '
        Me.Label12.AutoSize = True
        Me.Label12.Location = New System.Drawing.Point(142, 51)
        Me.Label12.Name = "Label12"
        Me.Label12.Size = New System.Drawing.Size(57, 13)
        Me.Label12.TabIndex = 5
        Me.Label12.Text = "Sub-basin:"
        '
        'tb_RUG_SelectSubb
        '
        Me.tb_RUG_SelectSubb.Location = New System.Drawing.Point(199, 48)
        Me.tb_RUG_SelectSubb.Name = "tb_RUG_SelectSubb"
        Me.tb_RUG_SelectSubb.Size = New System.Drawing.Size(42, 20)
        Me.tb_RUG_SelectSubb.TabIndex = 4
        Me.tb_RUG_SelectSubb.Text = "1"
        '
        'Label11
        '
        Me.Label11.AutoSize = True
        Me.Label11.Location = New System.Drawing.Point(13, 53)
        Me.Label11.Name = "Label11"
        Me.Label11.Size = New System.Drawing.Size(51, 13)
        Me.Label11.TabIndex = 3
        Me.Label11.Text = "Manning:"
        '
        'Button4
        '
        Me.Button4.Location = New System.Drawing.Point(288, 48)
        Me.Button4.Name = "Button4"
        Me.Button4.Size = New System.Drawing.Size(64, 23)
        Me.Button4.TabIndex = 2
        Me.Button4.Text = "Apply"
        Me.Button4.UseVisualStyleBackColor = True
        '
        'tbRUGMAN_subb
        '
        Me.tbRUGMAN_subb.Location = New System.Drawing.Point(70, 50)
        Me.tbRUGMAN_subb.Name = "tbRUGMAN_subb"
        Me.tbRUGMAN_subb.Size = New System.Drawing.Size(42, 20)
        Me.tbRUGMAN_subb.TabIndex = 1
        Me.tbRUGMAN_subb.Text = "0.035"
        '
        'Label10
        '
        Me.Label10.AutoSize = True
        Me.Label10.Location = New System.Drawing.Point(6, 25)
        Me.Label10.Name = "Label10"
        Me.Label10.Size = New System.Drawing.Size(323, 13)
        Me.Label10.TabIndex = 0
        Me.Label10.Text = "Apply the Manning roughness coefficient to the selected sub-basin:"
        '
        'GroupBox4
        '
        Me.GroupBox4.Controls.Add(Me.tbD_b)
        Me.GroupBox4.Controls.Add(Me.Label7)
        Me.GroupBox4.Controls.Add(Me.Label8)
        Me.GroupBox4.Controls.Add(Me.Button3)
        Me.GroupBox4.Controls.Add(Me.tbD_a)
        Me.GroupBox4.Controls.Add(Me.Label9)
        Me.GroupBox4.Location = New System.Drawing.Point(6, 225)
        Me.GroupBox4.Name = "GroupBox4"
        Me.GroupBox4.Size = New System.Drawing.Size(369, 101)
        Me.GroupBox4.TabIndex = 6
        Me.GroupBox4.TabStop = False
        Me.GroupBox4.Text = "River Channel Depths  (exponencial equation)"
        '
        'tbD_b
        '
        Me.tbD_b.Location = New System.Drawing.Point(214, 56)
        Me.tbD_b.Name = "tbD_b"
        Me.tbD_b.Size = New System.Drawing.Size(42, 20)
        Me.tbD_b.TabIndex = 5
        Me.tbD_b.Text = "0.3"
        '
        'Label7
        '
        Me.Label7.AutoSize = True
        Me.Label7.Location = New System.Drawing.Point(127, 59)
        Me.Label7.Name = "Label7"
        Me.Label7.Size = New System.Drawing.Size(86, 13)
        Me.Label7.TabIndex = 4
        Me.Label7.Text = "* AreaM_(km2) ^"
        '
        'Label8
        '
        Me.Label8.AutoSize = True
        Me.Label8.Location = New System.Drawing.Point(6, 59)
        Me.Label8.Name = "Label8"
        Me.Label8.Size = New System.Drawing.Size(62, 13)
        Me.Label8.TabIndex = 3
        Me.Label8.Text = "Depth (m) ="
        '
        'Button3
        '
        Me.Button3.Location = New System.Drawing.Point(294, 54)
        Me.Button3.Name = "Button3"
        Me.Button3.Size = New System.Drawing.Size(64, 23)
        Me.Button3.TabIndex = 2
        Me.Button3.Text = "Apply"
        Me.Button3.UseVisualStyleBackColor = True
        '
        'tbD_a
        '
        Me.tbD_a.Location = New System.Drawing.Point(79, 56)
        Me.tbD_a.Name = "tbD_a"
        Me.tbD_a.Size = New System.Drawing.Size(42, 20)
        Me.tbD_a.TabIndex = 1
        Me.tbD_a.Text = "1.03"
        '
        'Label9
        '
        Me.Label9.AutoSize = True
        Me.Label9.Location = New System.Drawing.Point(6, 25)
        Me.Label9.Name = "Label9"
        Me.Label9.Size = New System.Drawing.Size(253, 13)
        Me.Label9.TabIndex = 0
        Me.Label9.Text = "Apply the following width equation to all catchments:"
        '
        'GroupBox3
        '
        Me.GroupBox3.Controls.Add(Me.tbW_b)
        Me.GroupBox3.Controls.Add(Me.Label6)
        Me.GroupBox3.Controls.Add(Me.Label5)
        Me.GroupBox3.Controls.Add(Me.Button2)
        Me.GroupBox3.Controls.Add(Me.tbW_a)
        Me.GroupBox3.Controls.Add(Me.Label2)
        Me.GroupBox3.Location = New System.Drawing.Point(6, 118)
        Me.GroupBox3.Name = "GroupBox3"
        Me.GroupBox3.Size = New System.Drawing.Size(369, 101)
        Me.GroupBox3.TabIndex = 1
        Me.GroupBox3.TabStop = False
        Me.GroupBox3.Text = "River Channel Widths (exponencial equation)"
        '
        'tbW_b
        '
        Me.tbW_b.Location = New System.Drawing.Point(214, 56)
        Me.tbW_b.Name = "tbW_b"
        Me.tbW_b.Size = New System.Drawing.Size(42, 20)
        Me.tbW_b.TabIndex = 5
        Me.tbW_b.Text = "0.5"
        '
        'Label6
        '
        Me.Label6.AutoSize = True
        Me.Label6.Location = New System.Drawing.Point(127, 59)
        Me.Label6.Name = "Label6"
        Me.Label6.Size = New System.Drawing.Size(86, 13)
        Me.Label6.TabIndex = 4
        Me.Label6.Text = "* AreaM_(km2) ^"
        '
        'Label5
        '
        Me.Label5.AutoSize = True
        Me.Label5.Location = New System.Drawing.Point(6, 59)
        Me.Label5.Name = "Label5"
        Me.Label5.Size = New System.Drawing.Size(61, 13)
        Me.Label5.TabIndex = 3
        Me.Label5.Text = "Width (m) ="
        '
        'Button2
        '
        Me.Button2.Location = New System.Drawing.Point(294, 54)
        Me.Button2.Name = "Button2"
        Me.Button2.Size = New System.Drawing.Size(64, 23)
        Me.Button2.TabIndex = 2
        Me.Button2.Text = "Apply"
        Me.Button2.UseVisualStyleBackColor = True
        '
        'tbW_a
        '
        Me.tbW_a.Location = New System.Drawing.Point(79, 56)
        Me.tbW_a.Name = "tbW_a"
        Me.tbW_a.Size = New System.Drawing.Size(42, 20)
        Me.tbW_a.TabIndex = 1
        Me.tbW_a.Text = "0.95"
        '
        'Label2
        '
        Me.Label2.AutoSize = True
        Me.Label2.Location = New System.Drawing.Point(6, 25)
        Me.Label2.Name = "Label2"
        Me.Label2.Size = New System.Drawing.Size(253, 13)
        Me.Label2.TabIndex = 0
        Me.Label2.Text = "Apply the following width equation to all catchments:"
        '
        'GroupBox2
        '
        Me.GroupBox2.Controls.Add(Me.Button1)
        Me.GroupBox2.Controls.Add(Me.tbMANNING)
        Me.GroupBox2.Controls.Add(Me.Label1)
        Me.GroupBox2.Location = New System.Drawing.Point(6, 6)
        Me.GroupBox2.Name = "GroupBox2"
        Me.GroupBox2.Size = New System.Drawing.Size(369, 106)
        Me.GroupBox2.TabIndex = 0
        Me.GroupBox2.TabStop = False
        Me.GroupBox2.Text = "Manning roughness coefficient (all catchments)"
        '
        'Button1
        '
        Me.Button1.Location = New System.Drawing.Point(291, 48)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(64, 23)
        Me.Button1.TabIndex = 2
        Me.Button1.Text = "Apply"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'tbMANNING
        '
        Me.tbMANNING.Location = New System.Drawing.Point(238, 50)
        Me.tbMANNING.Name = "tbMANNING"
        Me.tbMANNING.Size = New System.Drawing.Size(42, 20)
        Me.tbMANNING.TabIndex = 1
        Me.tbMANNING.Text = "0.035"
        '
        'Label1
        '
        Me.Label1.AutoSize = True
        Me.Label1.Location = New System.Drawing.Point(3, 53)
        Me.Label1.Name = "Label1"
        Me.Label1.Size = New System.Drawing.Size(225, 13)
        Me.Label1.TabIndex = 0
        Me.Label1.Text = "Apply the following Manning to all catchments:"
        '
        'tbcell
        '
        Me.tbcell.Location = New System.Drawing.Point(136, 45)
        Me.tbcell.Name = "tbcell"
        Me.tbcell.Size = New System.Drawing.Size(815, 20)
        Me.tbcell.TabIndex = 39
        '
        'Label4
        '
        Me.Label4.AutoSize = True
        Me.Label4.Location = New System.Drawing.Point(9, 48)
        Me.Label4.Name = "Label4"
        Me.Label4.Size = New System.Drawing.Size(121, 13)
        Me.Label4.TabIndex = 38
        Me.Label4.Text = "Geometry file (MINI.gtp):"
        '
        'ofd1
        '
        Me.ofd1.FileName = "OpenFileDialog1"
        '
        'Button5
        '
        Me.Button5.Location = New System.Drawing.Point(957, 7)
        Me.Button5.Name = "Button5"
        Me.Button5.Size = New System.Drawing.Size(47, 23)
        Me.Button5.TabIndex = 37
        Me.Button5.Text = "..."
        Me.Button5.UseVisualStyleBackColor = True
        '
        'tbofdbloc
        '
        Me.tbofdbloc.Location = New System.Drawing.Point(136, 9)
        Me.tbofdbloc.Name = "tbofdbloc"
        Me.tbofdbloc.Size = New System.Drawing.Size(815, 20)
        Me.tbofdbloc.TabIndex = 36
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(9, 14)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(109, 13)
        Me.Label3.TabIndex = 35
        Me.Label3.Text = "HRUs description file:"
        '
        'btFECHAR
        '
        Me.btFECHAR.Location = New System.Drawing.Point(1094, 458)
        Me.btFECHAR.Name = "btFECHAR"
        Me.btFECHAR.Size = New System.Drawing.Size(93, 67)
        Me.btFECHAR.TabIndex = 34
        Me.btFECHAR.Text = "Close"
        Me.btFECHAR.UseVisualStyleBackColor = True
        '
        'btSALVARMini
        '
        Me.btSALVARMini.Enabled = False
        Me.btSALVARMini.Location = New System.Drawing.Point(946, 458)
        Me.btSALVARMini.Name = "btSALVARMini"
        Me.btSALVARMini.Size = New System.Drawing.Size(142, 67)
        Me.btSALVARMini.TabIndex = 33
        Me.btSALVARMini.Text = "Save edited geometry file"
        Me.btSALVARMini.UseVisualStyleBackColor = True
        '
        'btABRIRMINI
        '
        Me.btABRIRMINI.Enabled = False
        Me.btABRIRMINI.Location = New System.Drawing.Point(957, 42)
        Me.btABRIRMINI.Name = "btABRIRMINI"
        Me.btABRIRMINI.Size = New System.Drawing.Size(47, 23)
        Me.btABRIRMINI.TabIndex = 32
        Me.btABRIRMINI.Text = "..."
        Me.btABRIRMINI.UseVisualStyleBackColor = True
        '
        'Form_Editor
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(1196, 533)
        Me.Controls.Add(Me.TabControl1)
        Me.Controls.Add(Me.tbcell)
        Me.Controls.Add(Me.Label4)
        Me.Controls.Add(Me.Button5)
        Me.Controls.Add(Me.tbofdbloc)
        Me.Controls.Add(Me.Label3)
        Me.Controls.Add(Me.btFECHAR)
        Me.Controls.Add(Me.btSALVARMini)
        Me.Controls.Add(Me.btABRIRMINI)
        Me.Name = "Form_Editor"
        Me.Text = "Geometry File Editor"
        Me.TabControl1.ResumeLayout(False)
        Me.TabPage1.ResumeLayout(False)
        Me.GroupBox1.ResumeLayout(False)
        CType(Me.dtgvMINI, System.ComponentModel.ISupportInitialize).EndInit()
        Me.TabPage2.ResumeLayout(False)
        Me.GroupBox6.ResumeLayout(False)
        Me.GroupBox6.PerformLayout()
        Me.GroupBox5.ResumeLayout(False)
        Me.GroupBox5.PerformLayout()
        Me.GroupBox4.ResumeLayout(False)
        Me.GroupBox4.PerformLayout()
        Me.GroupBox3.ResumeLayout(False)
        Me.GroupBox3.PerformLayout()
        Me.GroupBox2.ResumeLayout(False)
        Me.GroupBox2.PerformLayout()
        Me.ResumeLayout(False)
        Me.PerformLayout()

    End Sub
    Friend WithEvents TabControl1 As System.Windows.Forms.TabControl
    Friend WithEvents TabPage1 As System.Windows.Forms.TabPage
    Friend WithEvents GroupBox1 As System.Windows.Forms.GroupBox
    Friend WithEvents dtgvMINI As System.Windows.Forms.DataGridView
    Friend WithEvents TabPage2 As System.Windows.Forms.TabPage
    Friend WithEvents GroupBox6 As System.Windows.Forms.GroupBox
    Friend WithEvents Button6 As System.Windows.Forms.Button
    Friend WithEvents Label15 As System.Windows.Forms.Label
    Friend WithEvents GroupBox5 As System.Windows.Forms.GroupBox
    Friend WithEvents Label12 As System.Windows.Forms.Label
    Friend WithEvents tb_RUG_SelectSubb As System.Windows.Forms.TextBox
    Friend WithEvents Label11 As System.Windows.Forms.Label
    Friend WithEvents Button4 As System.Windows.Forms.Button
    Friend WithEvents tbRUGMAN_subb As System.Windows.Forms.TextBox
    Friend WithEvents Label10 As System.Windows.Forms.Label
    Friend WithEvents GroupBox4 As System.Windows.Forms.GroupBox
    Friend WithEvents tbD_b As System.Windows.Forms.TextBox
    Friend WithEvents Label7 As System.Windows.Forms.Label
    Friend WithEvents Label8 As System.Windows.Forms.Label
    Friend WithEvents Button3 As System.Windows.Forms.Button
    Friend WithEvents tbD_a As System.Windows.Forms.TextBox
    Friend WithEvents Label9 As System.Windows.Forms.Label
    Friend WithEvents GroupBox3 As System.Windows.Forms.GroupBox
    Friend WithEvents tbW_b As System.Windows.Forms.TextBox
    Friend WithEvents Label6 As System.Windows.Forms.Label
    Friend WithEvents Label5 As System.Windows.Forms.Label
    Friend WithEvents Button2 As System.Windows.Forms.Button
    Friend WithEvents tbW_a As System.Windows.Forms.TextBox
    Friend WithEvents Label2 As System.Windows.Forms.Label
    Friend WithEvents GroupBox2 As System.Windows.Forms.GroupBox
    Friend WithEvents Button1 As System.Windows.Forms.Button
    Friend WithEvents tbMANNING As System.Windows.Forms.TextBox
    Friend WithEvents Label1 As System.Windows.Forms.Label
    Friend WithEvents tbcell As System.Windows.Forms.TextBox
    Friend WithEvents Label4 As System.Windows.Forms.Label
    Friend WithEvents ofd1 As System.Windows.Forms.OpenFileDialog
    Friend WithEvents Button5 As System.Windows.Forms.Button
    Friend WithEvents tbofdbloc As System.Windows.Forms.TextBox
    Friend WithEvents Label3 As System.Windows.Forms.Label
    Friend WithEvents btFECHAR As System.Windows.Forms.Button
    Friend WithEvents btSALVARMini As System.Windows.Forms.Button
    Friend WithEvents btABRIRMINI As System.Windows.Forms.Button
    Friend WithEvents sfd1 As System.Windows.Forms.SaveFileDialog

End Class
