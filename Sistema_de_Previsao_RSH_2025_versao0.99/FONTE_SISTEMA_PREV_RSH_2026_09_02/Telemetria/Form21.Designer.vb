<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form21
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
        Me.GroupBox2 = New System.Windows.Forms.GroupBox()
        Me.tbChuvaArquivo = New System.Windows.Forms.TextBox()
        Me.Button4 = New System.Windows.Forms.Button()
        Me.StatusStrip1 = New System.Windows.Forms.StatusStrip()
        Me.stlb = New System.Windows.Forms.ToolStripStatusLabel()
        Me.GroupBox6 = New System.Windows.Forms.GroupBox()
        Me.rbECMWFnew = New System.Windows.Forms.RadioButton()
        Me.rbChuvaArquivo = New System.Windows.Forms.RadioButton()
        Me.rbChuvaZero = New System.Windows.Forms.RadioButton()
        Me.rbECMWF = New System.Windows.Forms.RadioButton()
        Me.rbGEFS = New System.Windows.Forms.RadioButton()
        Me.rbETA40 = New System.Windows.Forms.RadioButton()
        Me.DataSet1 = New System.Data.DataSet()
        Me.DataTable1 = New System.Data.DataTable()
        Me.DataColumn6 = New System.Data.DataColumn()
        Me.DataColumn7 = New System.Data.DataColumn()
        Me.DataColumn1 = New System.Data.DataColumn()
        Me.DataColumn2 = New System.Data.DataColumn()
        Me.DataColumn3 = New System.Data.DataColumn()
        Me.DataColumn4 = New System.Data.DataColumn()
        Me.DataColumn5 = New System.Data.DataColumn()
        Me.GroupBox5 = New System.Windows.Forms.GroupBox()
        Me.tb_mini = New System.Windows.Forms.TextBox()
        Me.Button3 = New System.Windows.Forms.Button()
        Me.ofd = New System.Windows.Forms.OpenFileDialog()
        Me.GroupBox4 = New System.Windows.Forms.GroupBox()
        Me.Button1 = New System.Windows.Forms.Button()
        Me.cb_dir = New System.Windows.Forms.ComboBox()
        Me.Label3 = New System.Windows.Forms.Label()
        Me.CheckBox1 = New System.Windows.Forms.CheckBox()
        Me.Button2 = New System.Windows.Forms.Button()
        Me.GroupBox3 = New System.Windows.Forms.GroupBox()
        Me.tb_hora_ini = New System.Windows.Forms.ComboBox()
        Me.Label6 = New System.Windows.Forms.Label()
        Me.DateTimePicker2 = New System.Windows.Forms.DateTimePicker()
        Me.Label2 = New System.Windows.Forms.Label()
        Me.Label1 = New System.Windows.Forms.Label()
        Me.DateTimePicker1 = New System.Windows.Forms.DateTimePicker()
        Me.sfd = New System.Windows.Forms.SaveFileDialog()
        Me.GroupBox1 = New System.Windows.Forms.GroupBox()
        Me.DataGridView1 = New System.Windows.Forms.DataGridView()
        Me.HoraDataGridViewTextBoxColumn = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.PorcentagemDaChuvaDataGridViewTextBoxColumn = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.DataSet2 = New System.Data.DataSet()
        Me.DataTable2 = New System.Data.DataTable()
        Me.DataColumn8 = New System.Data.DataColumn()
        Me.DataColumn9 = New System.Data.DataColumn()
        Me.GroupBox7 = New System.Windows.Forms.GroupBox()
        Me.TextBox1 = New System.Windows.Forms.TextBox()
        Me.Button5 = New System.Windows.Forms.Button()
        Me.GroupBox2.SuspendLayout()
        Me.StatusStrip1.SuspendLayout()
        Me.GroupBox6.SuspendLayout()
        CType(Me.DataSet1, System.ComponentModel.ISupportInitialize).BeginInit()
        CType(Me.DataTable1, System.ComponentModel.ISupportInitialize).BeginInit()
        Me.GroupBox5.SuspendLayout()
        Me.GroupBox4.SuspendLayout()
        Me.GroupBox3.SuspendLayout()
        Me.GroupBox1.SuspendLayout()
        CType(Me.DataGridView1, System.ComponentModel.ISupportInitialize).BeginInit()
        CType(Me.DataSet2, System.ComponentModel.ISupportInitialize).BeginInit()
        CType(Me.DataTable2, System.ComponentModel.ISupportInitialize).BeginInit()
        Me.GroupBox7.SuspendLayout()
        Me.SuspendLayout()
        '
        'GroupBox2
        '
        Me.GroupBox2.Controls.Add(Me.tbChuvaArquivo)
        Me.GroupBox2.Controls.Add(Me.Button4)
        Me.GroupBox2.Location = New System.Drawing.Point(8, 67)
        Me.GroupBox2.Name = "GroupBox2"
        Me.GroupBox2.Size = New System.Drawing.Size(944, 54)
        Me.GroupBox2.TabIndex = 31
        Me.GroupBox2.TabStop = False
        Me.GroupBox2.Text = "Arquivo de chuva (.dat)"
        '
        'tbChuvaArquivo
        '
        Me.tbChuvaArquivo.Location = New System.Drawing.Point(6, 22)
        Me.tbChuvaArquivo.Name = "tbChuvaArquivo"
        Me.tbChuvaArquivo.Size = New System.Drawing.Size(869, 20)
        Me.tbChuvaArquivo.TabIndex = 1
        '
        'Button4
        '
        Me.Button4.Location = New System.Drawing.Point(881, 19)
        Me.Button4.Name = "Button4"
        Me.Button4.Size = New System.Drawing.Size(53, 23)
        Me.Button4.TabIndex = 0
        Me.Button4.Text = "..."
        Me.Button4.UseVisualStyleBackColor = True
        '
        'StatusStrip1
        '
        Me.StatusStrip1.Items.AddRange(New System.Windows.Forms.ToolStripItem() {Me.stlb})
        Me.StatusStrip1.Location = New System.Drawing.Point(0, 470)
        Me.StatusStrip1.Name = "StatusStrip1"
        Me.StatusStrip1.Size = New System.Drawing.Size(966, 22)
        Me.StatusStrip1.TabIndex = 30
        Me.StatusStrip1.Text = "StatusStrip1"
        '
        'stlb
        '
        Me.stlb.Name = "stlb"
        Me.stlb.Size = New System.Drawing.Size(91, 17)
        Me.stlb.Text = "Chuva SIMEPAR"
        '
        'GroupBox6
        '
        Me.GroupBox6.Controls.Add(Me.rbECMWFnew)
        Me.GroupBox6.Controls.Add(Me.rbChuvaArquivo)
        Me.GroupBox6.Controls.Add(Me.rbChuvaZero)
        Me.GroupBox6.Controls.Add(Me.rbECMWF)
        Me.GroupBox6.Controls.Add(Me.rbGEFS)
        Me.GroupBox6.Controls.Add(Me.rbETA40)
        Me.GroupBox6.Location = New System.Drawing.Point(8, 205)
        Me.GroupBox6.Name = "GroupBox6"
        Me.GroupBox6.Size = New System.Drawing.Size(434, 94)
        Me.GroupBox6.TabIndex = 32
        Me.GroupBox6.TabStop = False
        Me.GroupBox6.Text = "Modelo"
        '
        'rbECMWFnew
        '
        Me.rbECMWFnew.AutoSize = True
        Me.rbECMWFnew.Location = New System.Drawing.Point(6, 68)
        Me.rbECMWFnew.Name = "rbECMWFnew"
        Me.rbECMWFnew.Size = New System.Drawing.Size(162, 17)
        Me.rbECMWFnew.TabIndex = 35
        Me.rbECMWFnew.Text = "ONS ECMWF (formato novo)"
        Me.rbECMWFnew.UseVisualStyleBackColor = True
        '
        'rbChuvaArquivo
        '
        Me.rbChuvaArquivo.AutoSize = True
        Me.rbChuvaArquivo.Location = New System.Drawing.Point(206, 45)
        Me.rbChuvaArquivo.Name = "rbChuvaArquivo"
        Me.rbChuvaArquivo.Size = New System.Drawing.Size(119, 17)
        Me.rbChuvaArquivo.TabIndex = 34
        Me.rbChuvaArquivo.Text = "Chuva Arquivo TXT"
        Me.rbChuvaArquivo.UseVisualStyleBackColor = True
        '
        'rbChuvaZero
        '
        Me.rbChuvaZero.AutoSize = True
        Me.rbChuvaZero.Checked = True
        Me.rbChuvaZero.Location = New System.Drawing.Point(206, 23)
        Me.rbChuvaZero.Name = "rbChuvaZero"
        Me.rbChuvaZero.Size = New System.Drawing.Size(81, 17)
        Me.rbChuvaZero.TabIndex = 33
        Me.rbChuvaZero.TabStop = True
        Me.rbChuvaZero.Text = "Chuva Zero"
        Me.rbChuvaZero.UseVisualStyleBackColor = True
        '
        'rbECMWF
        '
        Me.rbECMWF.AutoSize = True
        Me.rbECMWF.Location = New System.Drawing.Point(206, 68)
        Me.rbECMWF.Name = "rbECMWF"
        Me.rbECMWF.Size = New System.Drawing.Size(167, 17)
        Me.rbECMWF.TabIndex = 3
        Me.rbECMWF.Text = "ONS ECMWF (formato antigo)"
        Me.rbECMWF.UseVisualStyleBackColor = True
        '
        'rbGEFS
        '
        Me.rbGEFS.AutoSize = True
        Me.rbGEFS.Location = New System.Drawing.Point(6, 45)
        Me.rbGEFS.Name = "rbGEFS"
        Me.rbGEFS.Size = New System.Drawing.Size(79, 17)
        Me.rbGEFS.TabIndex = 2
        Me.rbGEFS.Text = "ONS GEFS"
        Me.rbGEFS.UseVisualStyleBackColor = True
        '
        'rbETA40
        '
        Me.rbETA40.AutoSize = True
        Me.rbETA40.Location = New System.Drawing.Point(6, 23)
        Me.rbETA40.Name = "rbETA40"
        Me.rbETA40.Size = New System.Drawing.Size(84, 17)
        Me.rbETA40.TabIndex = 1
        Me.rbETA40.Text = "ONS ETA40"
        Me.rbETA40.UseVisualStyleBackColor = True
        '
        'DataSet1
        '
        Me.DataSet1.DataSetName = "NewDataSet"
        Me.DataSet1.Tables.AddRange(New System.Data.DataTable() {Me.DataTable1})
        '
        'DataTable1
        '
        Me.DataTable1.Columns.AddRange(New System.Data.DataColumn() {Me.DataColumn6, Me.DataColumn7, Me.DataColumn1, Me.DataColumn2, Me.DataColumn3, Me.DataColumn4, Me.DataColumn5})
        Me.DataTable1.TableName = "Table1"
        '
        'DataColumn6
        '
        Me.DataColumn6.ColumnName = "Data Inicial"
        '
        'DataColumn7
        '
        Me.DataColumn7.ColumnName = "Data Final"
        '
        'DataColumn1
        '
        Me.DataColumn1.ColumnName = "Região A"
        '
        'DataColumn2
        '
        Me.DataColumn2.ColumnName = "Região B"
        '
        'DataColumn3
        '
        Me.DataColumn3.ColumnName = "Região C"
        '
        'DataColumn4
        '
        Me.DataColumn4.ColumnName = "Região D"
        '
        'DataColumn5
        '
        Me.DataColumn5.ColumnName = "Região E"
        '
        'GroupBox5
        '
        Me.GroupBox5.Controls.Add(Me.tb_mini)
        Me.GroupBox5.Controls.Add(Me.Button3)
        Me.GroupBox5.Location = New System.Drawing.Point(8, 6)
        Me.GroupBox5.Name = "GroupBox5"
        Me.GroupBox5.Size = New System.Drawing.Size(944, 54)
        Me.GroupBox5.TabIndex = 29
        Me.GroupBox5.TabStop = False
        Me.GroupBox5.Text = "MINI.MGB"
        '
        'tb_mini
        '
        Me.tb_mini.Location = New System.Drawing.Point(6, 22)
        Me.tb_mini.Name = "tb_mini"
        Me.tb_mini.ReadOnly = True
        Me.tb_mini.Size = New System.Drawing.Size(869, 20)
        Me.tb_mini.TabIndex = 1
        '
        'Button3
        '
        Me.Button3.Enabled = False
        Me.Button3.Location = New System.Drawing.Point(881, 19)
        Me.Button3.Name = "Button3"
        Me.Button3.Size = New System.Drawing.Size(53, 23)
        Me.Button3.TabIndex = 0
        Me.Button3.Text = "..."
        Me.Button3.UseVisualStyleBackColor = True
        '
        'ofd
        '
        Me.ofd.FileName = "OpenFileDialog1"
        '
        'GroupBox4
        '
        Me.GroupBox4.Controls.Add(Me.Button1)
        Me.GroupBox4.Controls.Add(Me.cb_dir)
        Me.GroupBox4.Controls.Add(Me.Label3)
        Me.GroupBox4.Controls.Add(Me.CheckBox1)
        Me.GroupBox4.Location = New System.Drawing.Point(8, 305)
        Me.GroupBox4.Name = "GroupBox4"
        Me.GroupBox4.Size = New System.Drawing.Size(434, 47)
        Me.GroupBox4.TabIndex = 28
        Me.GroupBox4.TabStop = False
        Me.GroupBox4.Text = "Opções"
        '
        'Button1
        '
        Me.Button1.Location = New System.Drawing.Point(307, 15)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(43, 23)
        Me.Button1.TabIndex = 3
        Me.Button1.Text = "ok"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'cb_dir
        '
        Me.cb_dir.FormattingEnabled = True
        Me.cb_dir.Items.AddRange(New Object() {"1", "2", "3"})
        Me.cb_dir.Location = New System.Drawing.Point(262, 17)
        Me.cb_dir.Name = "cb_dir"
        Me.cb_dir.Size = New System.Drawing.Size(42, 21)
        Me.cb_dir.TabIndex = 2
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(207, 22)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(49, 13)
        Me.Label3.TabIndex = 1
        Me.Label3.Text = "Diretório:"
        '
        'CheckBox1
        '
        Me.CheckBox1.AutoSize = True
        Me.CheckBox1.Enabled = False
        Me.CheckBox1.Location = New System.Drawing.Point(6, 19)
        Me.CheckBox1.Name = "CheckBox1"
        Me.CheckBox1.Size = New System.Drawing.Size(115, 17)
        Me.CheckBox1.TabIndex = 0
        Me.CheckBox1.Text = "Salvar chuva texto"
        Me.CheckBox1.UseVisualStyleBackColor = True
        '
        'Button2
        '
        Me.Button2.Location = New System.Drawing.Point(788, 358)
        Me.Button2.Name = "Button2"
        Me.Button2.Size = New System.Drawing.Size(164, 89)
        Me.Button2.TabIndex = 27
        Me.Button2.Text = "Interpolar"
        Me.Button2.UseVisualStyleBackColor = True
        '
        'GroupBox3
        '
        Me.GroupBox3.Controls.Add(Me.tb_hora_ini)
        Me.GroupBox3.Controls.Add(Me.Label6)
        Me.GroupBox3.Controls.Add(Me.DateTimePicker2)
        Me.GroupBox3.Controls.Add(Me.Label2)
        Me.GroupBox3.Controls.Add(Me.Label1)
        Me.GroupBox3.Controls.Add(Me.DateTimePicker1)
        Me.GroupBox3.Location = New System.Drawing.Point(8, 358)
        Me.GroupBox3.Name = "GroupBox3"
        Me.GroupBox3.Size = New System.Drawing.Size(434, 94)
        Me.GroupBox3.TabIndex = 26
        Me.GroupBox3.TabStop = False
        Me.GroupBox3.Text = "Intervalo de tempo"
        '
        'tb_hora_ini
        '
        Me.tb_hora_ini.FormattingEnabled = True
        Me.tb_hora_ini.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.tb_hora_ini.Location = New System.Drawing.Point(173, 23)
        Me.tb_hora_ini.Name = "tb_hora_ini"
        Me.tb_hora_ini.Size = New System.Drawing.Size(44, 21)
        Me.tb_hora_ini.TabIndex = 18
        Me.tb_hora_ini.Text = "00"
        '
        'Label6
        '
        Me.Label6.AutoSize = True
        Me.Label6.Location = New System.Drawing.Point(223, 26)
        Me.Label6.Name = "Label6"
        Me.Label6.Size = New System.Drawing.Size(13, 13)
        Me.Label6.TabIndex = 16
        Me.Label6.Text = "h"
        '
        'DateTimePicker2
        '
        Me.DateTimePicker2.Format = System.Windows.Forms.DateTimePickerFormat.[Short]
        Me.DateTimePicker2.Location = New System.Drawing.Point(54, 48)
        Me.DateTimePicker2.Name = "DateTimePicker2"
        Me.DateTimePicker2.Size = New System.Drawing.Size(113, 20)
        Me.DateTimePicker2.TabIndex = 5
        '
        'Label2
        '
        Me.Label2.AutoSize = True
        Me.Label2.Location = New System.Drawing.Point(16, 50)
        Me.Label2.Name = "Label2"
        Me.Label2.Size = New System.Drawing.Size(23, 13)
        Me.Label2.TabIndex = 4
        Me.Label2.Text = "Fim"
        '
        'Label1
        '
        Me.Label1.AutoSize = True
        Me.Label1.Location = New System.Drawing.Point(16, 26)
        Me.Label1.Name = "Label1"
        Me.Label1.Size = New System.Drawing.Size(32, 13)
        Me.Label1.TabIndex = 3
        Me.Label1.Text = "Inicio"
        '
        'DateTimePicker1
        '
        Me.DateTimePicker1.Format = System.Windows.Forms.DateTimePickerFormat.[Short]
        Me.DateTimePicker1.Location = New System.Drawing.Point(54, 22)
        Me.DateTimePicker1.Name = "DateTimePicker1"
        Me.DateTimePicker1.Size = New System.Drawing.Size(113, 20)
        Me.DateTimePicker1.TabIndex = 2
        '
        'GroupBox1
        '
        Me.GroupBox1.Controls.Add(Me.DataGridView1)
        Me.GroupBox1.Location = New System.Drawing.Point(448, 205)
        Me.GroupBox1.Name = "GroupBox1"
        Me.GroupBox1.Size = New System.Drawing.Size(334, 242)
        Me.GroupBox1.TabIndex = 33
        Me.GroupBox1.TabStop = False
        Me.GroupBox1.Text = "Desagregação dos dados diários"
        '
        'DataGridView1
        '
        Me.DataGridView1.AutoGenerateColumns = False
        Me.DataGridView1.ColumnHeadersHeightSizeMode = System.Windows.Forms.DataGridViewColumnHeadersHeightSizeMode.AutoSize
        Me.DataGridView1.Columns.AddRange(New System.Windows.Forms.DataGridViewColumn() {Me.HoraDataGridViewTextBoxColumn, Me.PorcentagemDaChuvaDataGridViewTextBoxColumn})
        Me.DataGridView1.DataMember = "Table1"
        Me.DataGridView1.DataSource = Me.DataSet2
        Me.DataGridView1.Dock = System.Windows.Forms.DockStyle.Fill
        Me.DataGridView1.Location = New System.Drawing.Point(3, 16)
        Me.DataGridView1.Name = "DataGridView1"
        Me.DataGridView1.Size = New System.Drawing.Size(328, 223)
        Me.DataGridView1.TabIndex = 0
        '
        'HoraDataGridViewTextBoxColumn
        '
        Me.HoraDataGridViewTextBoxColumn.DataPropertyName = "Hora"
        Me.HoraDataGridViewTextBoxColumn.HeaderText = "Hora"
        Me.HoraDataGridViewTextBoxColumn.Name = "HoraDataGridViewTextBoxColumn"
        '
        'PorcentagemDaChuvaDataGridViewTextBoxColumn
        '
        Me.PorcentagemDaChuvaDataGridViewTextBoxColumn.DataPropertyName = "Porcentagem da chuva (%)"
        Me.PorcentagemDaChuvaDataGridViewTextBoxColumn.HeaderText = "Porcentagem da chuva (%)"
        Me.PorcentagemDaChuvaDataGridViewTextBoxColumn.Name = "PorcentagemDaChuvaDataGridViewTextBoxColumn"
        '
        'DataSet2
        '
        Me.DataSet2.DataSetName = "NewDataSet"
        Me.DataSet2.Tables.AddRange(New System.Data.DataTable() {Me.DataTable2})
        '
        'DataTable2
        '
        Me.DataTable2.Columns.AddRange(New System.Data.DataColumn() {Me.DataColumn8, Me.DataColumn9})
        Me.DataTable2.TableName = "Table1"
        '
        'DataColumn8
        '
        Me.DataColumn8.ColumnName = "Hora"
        '
        'DataColumn9
        '
        Me.DataColumn9.ColumnName = "Porcentagem da chuva (%)"
        '
        'GroupBox7
        '
        Me.GroupBox7.Controls.Add(Me.TextBox1)
        Me.GroupBox7.Controls.Add(Me.Button5)
        Me.GroupBox7.Location = New System.Drawing.Point(8, 127)
        Me.GroupBox7.Name = "GroupBox7"
        Me.GroupBox7.Size = New System.Drawing.Size(944, 54)
        Me.GroupBox7.TabIndex = 34
        Me.GroupBox7.TabStop = False
        Me.GroupBox7.Text = "Arquivo de chuva (.zip)"
        '
        'TextBox1
        '
        Me.TextBox1.Location = New System.Drawing.Point(6, 22)
        Me.TextBox1.Name = "TextBox1"
        Me.TextBox1.Size = New System.Drawing.Size(869, 20)
        Me.TextBox1.TabIndex = 1
        '
        'Button5
        '
        Me.Button5.Location = New System.Drawing.Point(881, 19)
        Me.Button5.Name = "Button5"
        Me.Button5.Size = New System.Drawing.Size(53, 23)
        Me.Button5.TabIndex = 0
        Me.Button5.Text = "..."
        Me.Button5.UseVisualStyleBackColor = True
        '
        'Form21
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(966, 492)
        Me.Controls.Add(Me.GroupBox7)
        Me.Controls.Add(Me.GroupBox1)
        Me.Controls.Add(Me.GroupBox2)
        Me.Controls.Add(Me.StatusStrip1)
        Me.Controls.Add(Me.GroupBox6)
        Me.Controls.Add(Me.GroupBox5)
        Me.Controls.Add(Me.GroupBox4)
        Me.Controls.Add(Me.Button2)
        Me.Controls.Add(Me.GroupBox3)
        Me.Name = "Form21"
        Me.Text = "Previsão de Chuva Arquivo .dat e .txt"
        Me.GroupBox2.ResumeLayout(False)
        Me.GroupBox2.PerformLayout()
        Me.StatusStrip1.ResumeLayout(False)
        Me.StatusStrip1.PerformLayout()
        Me.GroupBox6.ResumeLayout(False)
        Me.GroupBox6.PerformLayout()
        CType(Me.DataSet1, System.ComponentModel.ISupportInitialize).EndInit()
        CType(Me.DataTable1, System.ComponentModel.ISupportInitialize).EndInit()
        Me.GroupBox5.ResumeLayout(False)
        Me.GroupBox5.PerformLayout()
        Me.GroupBox4.ResumeLayout(False)
        Me.GroupBox4.PerformLayout()
        Me.GroupBox3.ResumeLayout(False)
        Me.GroupBox3.PerformLayout()
        Me.GroupBox1.ResumeLayout(False)
        CType(Me.DataGridView1, System.ComponentModel.ISupportInitialize).EndInit()
        CType(Me.DataSet2, System.ComponentModel.ISupportInitialize).EndInit()
        CType(Me.DataTable2, System.ComponentModel.ISupportInitialize).EndInit()
        Me.GroupBox7.ResumeLayout(False)
        Me.GroupBox7.PerformLayout()
        Me.ResumeLayout(False)
        Me.PerformLayout()

    End Sub
    Friend WithEvents GroupBox2 As System.Windows.Forms.GroupBox
    Friend WithEvents tbChuvaArquivo As System.Windows.Forms.TextBox
    Friend WithEvents Button4 As System.Windows.Forms.Button
    Friend WithEvents StatusStrip1 As System.Windows.Forms.StatusStrip
    Friend WithEvents stlb As System.Windows.Forms.ToolStripStatusLabel
    Friend WithEvents GroupBox6 As System.Windows.Forms.GroupBox
    Friend WithEvents rbECMWF As System.Windows.Forms.RadioButton
    Friend WithEvents rbGEFS As System.Windows.Forms.RadioButton
    Friend WithEvents rbETA40 As System.Windows.Forms.RadioButton
    Friend WithEvents DataSet1 As System.Data.DataSet
    Friend WithEvents DataTable1 As System.Data.DataTable
    Friend WithEvents DataColumn6 As System.Data.DataColumn
    Friend WithEvents DataColumn7 As System.Data.DataColumn
    Friend WithEvents DataColumn1 As System.Data.DataColumn
    Friend WithEvents DataColumn2 As System.Data.DataColumn
    Friend WithEvents DataColumn3 As System.Data.DataColumn
    Friend WithEvents DataColumn4 As System.Data.DataColumn
    Friend WithEvents DataColumn5 As System.Data.DataColumn
    Friend WithEvents GroupBox5 As System.Windows.Forms.GroupBox
    Friend WithEvents tb_mini As System.Windows.Forms.TextBox
    Friend WithEvents Button3 As System.Windows.Forms.Button
    Friend WithEvents ofd As System.Windows.Forms.OpenFileDialog
    Friend WithEvents GroupBox4 As System.Windows.Forms.GroupBox
    Friend WithEvents CheckBox1 As System.Windows.Forms.CheckBox
    Friend WithEvents Button2 As System.Windows.Forms.Button
    Friend WithEvents GroupBox3 As System.Windows.Forms.GroupBox
    Friend WithEvents tb_hora_ini As System.Windows.Forms.ComboBox
    Friend WithEvents Label6 As System.Windows.Forms.Label
    Friend WithEvents DateTimePicker2 As System.Windows.Forms.DateTimePicker
    Friend WithEvents Label2 As System.Windows.Forms.Label
    Friend WithEvents Label1 As System.Windows.Forms.Label
    Friend WithEvents DateTimePicker1 As System.Windows.Forms.DateTimePicker
    Friend WithEvents sfd As System.Windows.Forms.SaveFileDialog
    Friend WithEvents rbChuvaZero As System.Windows.Forms.RadioButton
    Friend WithEvents rbChuvaArquivo As System.Windows.Forms.RadioButton
    Friend WithEvents cb_dir As System.Windows.Forms.ComboBox
    Friend WithEvents Label3 As System.Windows.Forms.Label
    Friend WithEvents Button1 As System.Windows.Forms.Button
    Friend WithEvents GroupBox1 As Windows.Forms.GroupBox
    Friend WithEvents DataGridView1 As Windows.Forms.DataGridView
    Friend WithEvents DataSet2 As DataSet
    Friend WithEvents DataTable2 As DataTable
    Friend WithEvents DataColumn8 As DataColumn
    Friend WithEvents DataColumn9 As DataColumn
    Friend WithEvents HoraDataGridViewTextBoxColumn As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents PorcentagemDaChuvaDataGridViewTextBoxColumn As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents rbECMWFnew As Windows.Forms.RadioButton
    Friend WithEvents GroupBox7 As Windows.Forms.GroupBox
    Friend WithEvents TextBox1 As Windows.Forms.TextBox
    Friend WithEvents Button5 As Windows.Forms.Button
End Class
