<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class FormWind
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
        Dim resources As System.ComponentModel.ComponentResourceManager = New System.ComponentModel.ComponentResourceManager(GetType(FormWind))
        Me.GroupBox1 = New System.Windows.Forms.GroupBox()
        Me.DataGridView1 = New System.Windows.Forms.DataGridView()
        Me.DiaDataGridViewTextBoxColumn1 = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.MesDataGridViewTextBoxColumn1 = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.AnoDataGridViewTextBoxColumn1 = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.HoraDataGridViewTextBoxColumn1 = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.Velocidadems1DataGridViewTextBoxColumn = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.DireçãoazimuteDataGridViewTextBoxColumn = New System.Windows.Forms.DataGridViewTextBoxColumn()
        Me.DataSet1 = New System.Data.DataSet()
        Me.DataTable1 = New System.Data.DataTable()
        Me.DataColumn1 = New System.Data.DataColumn()
        Me.DataColumn2 = New System.Data.DataColumn()
        Me.DataColumn3 = New System.Data.DataColumn()
        Me.DataColumn4 = New System.Data.DataColumn()
        Me.DataColumn5 = New System.Data.DataColumn()
        Me.DataColumn6 = New System.Data.DataColumn()
        Me.StatusStrip1 = New System.Windows.Forms.StatusStrip()
        Me.stlb = New System.Windows.Forms.ToolStripStatusLabel()
        Me.GroupBox3 = New System.Windows.Forms.GroupBox()
        Me.Button6 = New System.Windows.Forms.Button()
        Me.tb_hora_fim = New System.Windows.Forms.ComboBox()
        Me.tb_hora_ini = New System.Windows.Forms.ComboBox()
        Me.Label7 = New System.Windows.Forms.Label()
        Me.Label6 = New System.Windows.Forms.Label()
        Me.dtpqob2 = New System.Windows.Forms.DateTimePicker()
        Me.dtpqob1 = New System.Windows.Forms.DateTimePicker()
        Me.Label3 = New System.Windows.Forms.Label()
        Me.Label4 = New System.Windows.Forms.Label()
        Me.ofd = New System.Windows.Forms.OpenFileDialog()
        Me.PictureBox1 = New System.Windows.Forms.PictureBox()
        Me.Button1 = New System.Windows.Forms.Button()
        Me.Button2 = New System.Windows.Forms.Button()
        Me.Button3 = New System.Windows.Forms.Button()
        Me.GroupBox1.SuspendLayout()
        CType(Me.DataGridView1, System.ComponentModel.ISupportInitialize).BeginInit()
        CType(Me.DataSet1, System.ComponentModel.ISupportInitialize).BeginInit()
        CType(Me.DataTable1, System.ComponentModel.ISupportInitialize).BeginInit()
        Me.StatusStrip1.SuspendLayout()
        Me.GroupBox3.SuspendLayout()
        CType(Me.PictureBox1, System.ComponentModel.ISupportInitialize).BeginInit()
        Me.SuspendLayout()
        '
        'GroupBox1
        '
        Me.GroupBox1.Controls.Add(Me.DataGridView1)
        Me.GroupBox1.Location = New System.Drawing.Point(264, 6)
        Me.GroupBox1.Name = "GroupBox1"
        Me.GroupBox1.Size = New System.Drawing.Size(524, 526)
        Me.GroupBox1.TabIndex = 13
        Me.GroupBox1.TabStop = False
        Me.GroupBox1.Text = "Editar Vento sobre o Guaíba e Laguna dos Patos"
        '
        'DataGridView1
        '
        Me.DataGridView1.AutoGenerateColumns = False
        Me.DataGridView1.ColumnHeadersHeightSizeMode = System.Windows.Forms.DataGridViewColumnHeadersHeightSizeMode.AutoSize
        Me.DataGridView1.Columns.AddRange(New System.Windows.Forms.DataGridViewColumn() {Me.DiaDataGridViewTextBoxColumn1, Me.MesDataGridViewTextBoxColumn1, Me.AnoDataGridViewTextBoxColumn1, Me.HoraDataGridViewTextBoxColumn1, Me.Velocidadems1DataGridViewTextBoxColumn, Me.DireçãoazimuteDataGridViewTextBoxColumn})
        Me.DataGridView1.DataMember = "Table1"
        Me.DataGridView1.DataSource = Me.DataSet1
        Me.DataGridView1.Dock = System.Windows.Forms.DockStyle.Fill
        Me.DataGridView1.Location = New System.Drawing.Point(3, 16)
        Me.DataGridView1.Name = "DataGridView1"
        Me.DataGridView1.Size = New System.Drawing.Size(518, 507)
        Me.DataGridView1.TabIndex = 0
        '
        'DiaDataGridViewTextBoxColumn1
        '
        Me.DiaDataGridViewTextBoxColumn1.DataPropertyName = "Dia"
        Me.DiaDataGridViewTextBoxColumn1.HeaderText = "Dia"
        Me.DiaDataGridViewTextBoxColumn1.Name = "DiaDataGridViewTextBoxColumn1"
        Me.DiaDataGridViewTextBoxColumn1.Width = 50
        '
        'MesDataGridViewTextBoxColumn1
        '
        Me.MesDataGridViewTextBoxColumn1.DataPropertyName = "Mes"
        Me.MesDataGridViewTextBoxColumn1.HeaderText = "Mes"
        Me.MesDataGridViewTextBoxColumn1.Name = "MesDataGridViewTextBoxColumn1"
        Me.MesDataGridViewTextBoxColumn1.Width = 50
        '
        'AnoDataGridViewTextBoxColumn1
        '
        Me.AnoDataGridViewTextBoxColumn1.DataPropertyName = "Ano"
        Me.AnoDataGridViewTextBoxColumn1.HeaderText = "Ano"
        Me.AnoDataGridViewTextBoxColumn1.Name = "AnoDataGridViewTextBoxColumn1"
        Me.AnoDataGridViewTextBoxColumn1.Width = 50
        '
        'HoraDataGridViewTextBoxColumn1
        '
        Me.HoraDataGridViewTextBoxColumn1.DataPropertyName = "Hora"
        Me.HoraDataGridViewTextBoxColumn1.HeaderText = "Hora"
        Me.HoraDataGridViewTextBoxColumn1.Name = "HoraDataGridViewTextBoxColumn1"
        Me.HoraDataGridViewTextBoxColumn1.Width = 50
        '
        'Velocidadems1DataGridViewTextBoxColumn
        '
        Me.Velocidadems1DataGridViewTextBoxColumn.DataPropertyName = "Velocidade (m.s-1)"
        Me.Velocidadems1DataGridViewTextBoxColumn.HeaderText = "Velocidade (m.s-1)"
        Me.Velocidadems1DataGridViewTextBoxColumn.Name = "Velocidadems1DataGridViewTextBoxColumn"
        '
        'DireçãoazimuteDataGridViewTextBoxColumn
        '
        Me.DireçãoazimuteDataGridViewTextBoxColumn.DataPropertyName = "Direção (azimute)"
        Me.DireçãoazimuteDataGridViewTextBoxColumn.HeaderText = "Direção (azimute)"
        Me.DireçãoazimuteDataGridViewTextBoxColumn.Name = "DireçãoazimuteDataGridViewTextBoxColumn"
        '
        'DataSet1
        '
        Me.DataSet1.DataSetName = "NewDataSet"
        Me.DataSet1.Tables.AddRange(New System.Data.DataTable() {Me.DataTable1})
        '
        'DataTable1
        '
        Me.DataTable1.Columns.AddRange(New System.Data.DataColumn() {Me.DataColumn1, Me.DataColumn2, Me.DataColumn3, Me.DataColumn4, Me.DataColumn5, Me.DataColumn6})
        Me.DataTable1.TableName = "Table1"
        '
        'DataColumn1
        '
        Me.DataColumn1.ColumnName = "Dia"
        '
        'DataColumn2
        '
        Me.DataColumn2.ColumnName = "Mes"
        '
        'DataColumn3
        '
        Me.DataColumn3.ColumnName = "Ano"
        '
        'DataColumn4
        '
        Me.DataColumn4.ColumnName = "Hora"
        '
        'DataColumn5
        '
        Me.DataColumn5.ColumnName = "Velocidade (m.s-1)"
        '
        'DataColumn6
        '
        Me.DataColumn6.ColumnName = "Direção (azimute)"
        '
        'StatusStrip1
        '
        Me.StatusStrip1.Items.AddRange(New System.Windows.Forms.ToolStripItem() {Me.stlb})
        Me.StatusStrip1.Location = New System.Drawing.Point(0, 539)
        Me.StatusStrip1.Name = "StatusStrip1"
        Me.StatusStrip1.Size = New System.Drawing.Size(796, 22)
        Me.StatusStrip1.TabIndex = 12
        Me.StatusStrip1.Text = "StatusStrip1"
        '
        'stlb
        '
        Me.stlb.Name = "stlb"
        Me.stlb.Size = New System.Drawing.Size(26, 17)
        Me.stlb.Text = "text"
        '
        'GroupBox3
        '
        Me.GroupBox3.Controls.Add(Me.Button6)
        Me.GroupBox3.Controls.Add(Me.tb_hora_fim)
        Me.GroupBox3.Controls.Add(Me.tb_hora_ini)
        Me.GroupBox3.Controls.Add(Me.Label7)
        Me.GroupBox3.Controls.Add(Me.Label6)
        Me.GroupBox3.Controls.Add(Me.dtpqob2)
        Me.GroupBox3.Controls.Add(Me.dtpqob1)
        Me.GroupBox3.Controls.Add(Me.Label3)
        Me.GroupBox3.Controls.Add(Me.Label4)
        Me.GroupBox3.Location = New System.Drawing.Point(12, 375)
        Me.GroupBox3.Name = "GroupBox3"
        Me.GroupBox3.Size = New System.Drawing.Size(249, 154)
        Me.GroupBox3.TabIndex = 11
        Me.GroupBox3.TabStop = False
        Me.GroupBox3.Text = "Dados de vento"
        '
        'Button6
        '
        Me.Button6.Location = New System.Drawing.Point(24, 83)
        Me.Button6.Name = "Button6"
        Me.Button6.Size = New System.Drawing.Size(175, 58)
        Me.Button6.TabIndex = 16
        Me.Button6.Text = "Exportar Dados de Vento"
        Me.Button6.UseVisualStyleBackColor = True
        '
        'tb_hora_fim
        '
        Me.tb_hora_fim.FormattingEnabled = True
        Me.tb_hora_fim.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.tb_hora_fim.Location = New System.Drawing.Point(155, 55)
        Me.tb_hora_fim.Name = "tb_hora_fim"
        Me.tb_hora_fim.Size = New System.Drawing.Size(44, 21)
        Me.tb_hora_fim.TabIndex = 15
        Me.tb_hora_fim.Text = "23"
        '
        'tb_hora_ini
        '
        Me.tb_hora_ini.FormattingEnabled = True
        Me.tb_hora_ini.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.tb_hora_ini.Location = New System.Drawing.Point(155, 31)
        Me.tb_hora_ini.Name = "tb_hora_ini"
        Me.tb_hora_ini.Size = New System.Drawing.Size(44, 21)
        Me.tb_hora_ini.TabIndex = 14
        Me.tb_hora_ini.Text = "00"
        '
        'Label7
        '
        Me.Label7.AutoSize = True
        Me.Label7.Location = New System.Drawing.Point(205, 60)
        Me.Label7.Name = "Label7"
        Me.Label7.Size = New System.Drawing.Size(13, 13)
        Me.Label7.TabIndex = 13
        Me.Label7.Text = "h"
        '
        'Label6
        '
        Me.Label6.AutoSize = True
        Me.Label6.Location = New System.Drawing.Point(205, 34)
        Me.Label6.Name = "Label6"
        Me.Label6.Size = New System.Drawing.Size(13, 13)
        Me.Label6.TabIndex = 11
        Me.Label6.Text = "h"
        '
        'dtpqob2
        '
        Me.dtpqob2.Format = System.Windows.Forms.DateTimePickerFormat.[Short]
        Me.dtpqob2.Location = New System.Drawing.Point(49, 57)
        Me.dtpqob2.Name = "dtpqob2"
        Me.dtpqob2.Size = New System.Drawing.Size(93, 20)
        Me.dtpqob2.TabIndex = 9
        '
        'dtpqob1
        '
        Me.dtpqob1.Format = System.Windows.Forms.DateTimePickerFormat.[Short]
        Me.dtpqob1.Location = New System.Drawing.Point(49, 31)
        Me.dtpqob1.Name = "dtpqob1"
        Me.dtpqob1.Size = New System.Drawing.Size(93, 20)
        Me.dtpqob1.TabIndex = 8
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(6, 63)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(26, 13)
        Me.Label3.TabIndex = 1
        Me.Label3.Text = "Fim:"
        '
        'Label4
        '
        Me.Label4.AutoSize = True
        Me.Label4.Location = New System.Drawing.Point(6, 33)
        Me.Label4.Name = "Label4"
        Me.Label4.Size = New System.Drawing.Size(37, 13)
        Me.Label4.TabIndex = 0
        Me.Label4.Text = "Início:"
        '
        'ofd
        '
        Me.ofd.FileName = "ofd"
        '
        'PictureBox1
        '
        Me.PictureBox1.Image = CType(resources.GetObject("PictureBox1.Image"), System.Drawing.Image)
        Me.PictureBox1.Location = New System.Drawing.Point(12, 12)
        Me.PictureBox1.Name = "PictureBox1"
        Me.PictureBox1.Size = New System.Drawing.Size(246, 128)
        Me.PictureBox1.SizeMode = System.Windows.Forms.PictureBoxSizeMode.StretchImage
        Me.PictureBox1.TabIndex = 14
        Me.PictureBox1.TabStop = False
        '
        'Button1
        '
        Me.Button1.Location = New System.Drawing.Point(12, 221)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(120, 51)
        Me.Button1.TabIndex = 15
        Me.Button1.Text = "Atualizar dados de velocidade"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'Button2
        '
        Me.Button2.Location = New System.Drawing.Point(138, 221)
        Me.Button2.Name = "Button2"
        Me.Button2.Size = New System.Drawing.Size(120, 51)
        Me.Button2.TabIndex = 16
        Me.Button2.Text = "Atualizar dados de direção"
        Me.Button2.UseVisualStyleBackColor = True
        '
        'Button3
        '
        Me.Button3.Location = New System.Drawing.Point(12, 174)
        Me.Button3.Name = "Button3"
        Me.Button3.Size = New System.Drawing.Size(246, 41)
        Me.Button3.TabIndex = 17
        Me.Button3.Text = "Abrir janela de download de dados de vento"
        Me.Button3.UseVisualStyleBackColor = True
        '
        'FormWind
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(796, 561)
        Me.Controls.Add(Me.Button3)
        Me.Controls.Add(Me.Button2)
        Me.Controls.Add(Me.Button1)
        Me.Controls.Add(Me.PictureBox1)
        Me.Controls.Add(Me.GroupBox1)
        Me.Controls.Add(Me.StatusStrip1)
        Me.Controls.Add(Me.GroupBox3)
        Me.Name = "FormWind"
        Me.Text = "Vento"
        Me.GroupBox1.ResumeLayout(False)
        CType(Me.DataGridView1, System.ComponentModel.ISupportInitialize).EndInit()
        CType(Me.DataSet1, System.ComponentModel.ISupportInitialize).EndInit()
        CType(Me.DataTable1, System.ComponentModel.ISupportInitialize).EndInit()
        Me.StatusStrip1.ResumeLayout(False)
        Me.StatusStrip1.PerformLayout()
        Me.GroupBox3.ResumeLayout(False)
        Me.GroupBox3.PerformLayout()
        CType(Me.PictureBox1, System.ComponentModel.ISupportInitialize).EndInit()
        Me.ResumeLayout(False)
        Me.PerformLayout()

    End Sub

    Friend WithEvents GroupBox1 As Windows.Forms.GroupBox
    Friend WithEvents DataGridView1 As Windows.Forms.DataGridView
    Friend WithEvents DiaDataGridViewTextBoxColumn1 As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents MesDataGridViewTextBoxColumn1 As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents AnoDataGridViewTextBoxColumn1 As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents HoraDataGridViewTextBoxColumn1 As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents Velocidadems1DataGridViewTextBoxColumn As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents DireçãoazimuteDataGridViewTextBoxColumn As Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents DataSet1 As DataSet
    Friend WithEvents DataTable1 As DataTable
    Friend WithEvents DataColumn1 As DataColumn
    Friend WithEvents DataColumn2 As DataColumn
    Friend WithEvents DataColumn3 As DataColumn
    Friend WithEvents DataColumn4 As DataColumn
    Friend WithEvents DataColumn5 As DataColumn
    Friend WithEvents DataColumn6 As DataColumn
    Friend WithEvents StatusStrip1 As Windows.Forms.StatusStrip
    Friend WithEvents stlb As Windows.Forms.ToolStripStatusLabel
    Friend WithEvents GroupBox3 As Windows.Forms.GroupBox
    Friend WithEvents Button6 As Windows.Forms.Button
    Friend WithEvents tb_hora_fim As Windows.Forms.ComboBox
    Friend WithEvents tb_hora_ini As Windows.Forms.ComboBox
    Friend WithEvents Label7 As Windows.Forms.Label
    Friend WithEvents Label6 As Windows.Forms.Label
    Friend WithEvents dtpqob2 As Windows.Forms.DateTimePicker
    Friend WithEvents dtpqob1 As Windows.Forms.DateTimePicker
    Friend WithEvents Label3 As Windows.Forms.Label
    Friend WithEvents Label4 As Windows.Forms.Label
    Friend WithEvents ofd As Windows.Forms.OpenFileDialog
    Friend WithEvents PictureBox1 As Windows.Forms.PictureBox
    Friend WithEvents Button1 As Windows.Forms.Button
    Friend WithEvents Button2 As Windows.Forms.Button
    Friend WithEvents Button3 As Windows.Forms.Button
End Class
