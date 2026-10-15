<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form2
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
        Me.DataGridView1 = New System.Windows.Forms.DataGridView
        Me.EstaçãoDataGridViewTextBoxColumn = New System.Windows.Forms.DataGridViewTextBoxColumn
        Me.DataSet1 = New System.Data.DataSet
        Me.DataTable1 = New System.Data.DataTable
        Me.DataColumn1 = New System.Data.DataColumn
        Me.Button1 = New System.Windows.Forms.Button
        Me.Label1 = New System.Windows.Forms.Label
        Me.TextBox1 = New System.Windows.Forms.TextBox
        Me.Label2 = New System.Windows.Forms.Label
        Me.TabControl1 = New System.Windows.Forms.TabControl
        Me.TabPage1 = New System.Windows.Forms.TabPage
        Me.GroupBox7 = New System.Windows.Forms.GroupBox
        Me.Button9 = New System.Windows.Forms.Button
        Me.tb_previsao = New System.Windows.Forms.TextBox
        Me.Button8 = New System.Windows.Forms.Button
        Me.GroupBox4 = New System.Windows.Forms.GroupBox
        Me.Button4 = New System.Windows.Forms.Button
        Me.tb_estreito = New System.Windows.Forms.TextBox
        Me.GroupBox3 = New System.Windows.Forms.GroupBox
        Me.Button3 = New System.Windows.Forms.Button
        Me.tb_metereológicas = New System.Windows.Forms.TextBox
        Me.GroupBox2 = New System.Windows.Forms.GroupBox
        Me.Label4 = New System.Windows.Forms.Label
        Me.Label3 = New System.Windows.Forms.Label
        Me.Button7 = New System.Windows.Forms.Button
        Me.tb_telemetria_chuva = New System.Windows.Forms.TextBox
        Me.Button2 = New System.Windows.Forms.Button
        Me.tb_telemetria = New System.Windows.Forms.TextBox
        Me.TabPage2 = New System.Windows.Forms.TabPage
        Me.Button14 = New System.Windows.Forms.Button
        Me.Label6 = New System.Windows.Forms.Label
        Me.Button13 = New System.Windows.Forms.Button
        Me.GroupBox10 = New System.Windows.Forms.GroupBox
        Me.pbar = New System.Windows.Forms.ProgressBar
        Me.Label7 = New System.Windows.Forms.Label
        Me.Button15 = New System.Windows.Forms.Button
        Me.Label5 = New System.Windows.Forms.Label
        Me.btCompact = New System.Windows.Forms.Button
        Me.fbd = New System.Windows.Forms.FolderBrowserDialog
        Me.GroupBox11 = New System.Windows.Forms.GroupBox
        Me.bt_dir_export = New System.Windows.Forms.Button
        Me.tb_dir_export = New System.Windows.Forms.TextBox
        Me.GroupBox6 = New System.Windows.Forms.GroupBox
        Me.Label10 = New System.Windows.Forms.Label
        Me.Button16 = New System.Windows.Forms.Button
        Me.tb_meteo_auto3 = New System.Windows.Forms.TextBox
        Me.Label9 = New System.Windows.Forms.Label
        Me.Button12 = New System.Windows.Forms.Button
        Me.tb_meteo_auto2 = New System.Windows.Forms.TextBox
        Me.Label8 = New System.Windows.Forms.Label
        Me.Button6 = New System.Windows.Forms.Button
        Me.tb_meteo_auto = New System.Windows.Forms.TextBox
        Me.GroupBox1.SuspendLayout()
        CType(Me.DataGridView1, System.ComponentModel.ISupportInitialize).BeginInit()
        CType(Me.DataSet1, System.ComponentModel.ISupportInitialize).BeginInit()
        CType(Me.DataTable1, System.ComponentModel.ISupportInitialize).BeginInit()
        Me.TabControl1.SuspendLayout()
        Me.TabPage1.SuspendLayout()
        Me.GroupBox7.SuspendLayout()
        Me.GroupBox4.SuspendLayout()
        Me.GroupBox3.SuspendLayout()
        Me.GroupBox2.SuspendLayout()
        Me.TabPage2.SuspendLayout()
        Me.GroupBox10.SuspendLayout()
        Me.GroupBox11.SuspendLayout()
        Me.GroupBox6.SuspendLayout()
        Me.SuspendLayout()
        '
        'GroupBox1
        '
        Me.GroupBox1.Controls.Add(Me.DataGridView1)
        Me.GroupBox1.Location = New System.Drawing.Point(8, 6)
        Me.GroupBox1.Name = "GroupBox1"
        Me.GroupBox1.Size = New System.Drawing.Size(522, 239)
        Me.GroupBox1.TabIndex = 0
        Me.GroupBox1.TabStop = False
        Me.GroupBox1.Text = "Estações telemétricas"
        '
        'DataGridView1
        '
        Me.DataGridView1.AutoGenerateColumns = False
        Me.DataGridView1.ColumnHeadersHeightSizeMode = System.Windows.Forms.DataGridViewColumnHeadersHeightSizeMode.AutoSize
        Me.DataGridView1.Columns.AddRange(New System.Windows.Forms.DataGridViewColumn() {Me.EstaçãoDataGridViewTextBoxColumn})
        Me.DataGridView1.DataMember = "Table1"
        Me.DataGridView1.DataSource = Me.DataSet1
        Me.DataGridView1.Location = New System.Drawing.Point(6, 19)
        Me.DataGridView1.Name = "DataGridView1"
        Me.DataGridView1.Size = New System.Drawing.Size(510, 214)
        Me.DataGridView1.TabIndex = 0
        '
        'EstaçãoDataGridViewTextBoxColumn
        '
        Me.EstaçãoDataGridViewTextBoxColumn.DataPropertyName = "Estação"
        Me.EstaçãoDataGridViewTextBoxColumn.HeaderText = "Estação"
        Me.EstaçãoDataGridViewTextBoxColumn.Name = "EstaçãoDataGridViewTextBoxColumn"
        Me.EstaçãoDataGridViewTextBoxColumn.Width = 460
        '
        'DataSet1
        '
        Me.DataSet1.DataSetName = "NewDataSet"
        Me.DataSet1.Tables.AddRange(New System.Data.DataTable() {Me.DataTable1})
        '
        'DataTable1
        '
        Me.DataTable1.Columns.AddRange(New System.Data.DataColumn() {Me.DataColumn1})
        Me.DataTable1.TableName = "Table1"
        '
        'DataColumn1
        '
        Me.DataColumn1.Caption = "Estação"
        Me.DataColumn1.ColumnName = "Estação"
        '
        'Button1
        '
        Me.Button1.Enabled = False
        Me.Button1.Location = New System.Drawing.Point(48, 291)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(162, 65)
        Me.Button1.TabIndex = 1
        Me.Button1.Text = "Atualizar lista de estações"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'Label1
        '
        Me.Label1.AutoSize = True
        Me.Label1.Location = New System.Drawing.Point(228, 299)
        Me.Label1.Name = "Label1"
        Me.Label1.Size = New System.Drawing.Size(213, 52)
        Me.Label1.TabIndex = 2
        Me.Label1.Text = "  Insira ou remova as estações telemétricas " & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "que  desejar  e  depois  atualize  " & _
            "a  lista  de" & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "estações para download clicando no botão" & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "ao lado."
        '
        'TextBox1
        '
        Me.TextBox1.Location = New System.Drawing.Point(48, 249)
        Me.TextBox1.Name = "TextBox1"
        Me.TextBox1.ReadOnly = True
        Me.TextBox1.Size = New System.Drawing.Size(52, 20)
        Me.TextBox1.TabIndex = 3
        '
        'Label2
        '
        Me.Label2.AutoSize = True
        Me.Label2.Location = New System.Drawing.Point(11, 252)
        Me.Label2.Name = "Label2"
        Me.Label2.Size = New System.Drawing.Size(34, 13)
        Me.Label2.TabIndex = 4
        Me.Label2.Text = "Total:"
        '
        'TabControl1
        '
        Me.TabControl1.Controls.Add(Me.TabPage1)
        Me.TabControl1.Controls.Add(Me.TabPage2)
        Me.TabControl1.Dock = System.Windows.Forms.DockStyle.Fill
        Me.TabControl1.Location = New System.Drawing.Point(0, 0)
        Me.TabControl1.Name = "TabControl1"
        Me.TabControl1.SelectedIndex = 0
        Me.TabControl1.Size = New System.Drawing.Size(570, 720)
        Me.TabControl1.TabIndex = 5
        '
        'TabPage1
        '
        Me.TabPage1.Controls.Add(Me.GroupBox11)
        Me.TabPage1.Controls.Add(Me.GroupBox6)
        Me.TabPage1.Controls.Add(Me.GroupBox7)
        Me.TabPage1.Controls.Add(Me.Button8)
        Me.TabPage1.Controls.Add(Me.GroupBox4)
        Me.TabPage1.Controls.Add(Me.GroupBox3)
        Me.TabPage1.Controls.Add(Me.GroupBox2)
        Me.TabPage1.Location = New System.Drawing.Point(4, 22)
        Me.TabPage1.Name = "TabPage1"
        Me.TabPage1.Padding = New System.Windows.Forms.Padding(3)
        Me.TabPage1.Size = New System.Drawing.Size(562, 694)
        Me.TabPage1.TabIndex = 0
        Me.TabPage1.Text = "Diretórios"
        Me.TabPage1.UseVisualStyleBackColor = True
        '
        'GroupBox7
        '
        Me.GroupBox7.Controls.Add(Me.Button9)
        Me.GroupBox7.Controls.Add(Me.tb_previsao)
        Me.GroupBox7.Location = New System.Drawing.Point(8, 214)
        Me.GroupBox7.Name = "GroupBox7"
        Me.GroupBox7.Size = New System.Drawing.Size(546, 68)
        Me.GroupBox7.TabIndex = 6
        Me.GroupBox7.TabStop = False
        Me.GroupBox7.Text = "Diretório destino dos arquivos temporários de previsão"
        '
        'Button9
        '
        Me.Button9.Location = New System.Drawing.Point(477, 28)
        Me.Button9.Name = "Button9"
        Me.Button9.Size = New System.Drawing.Size(40, 23)
        Me.Button9.TabIndex = 1
        Me.Button9.Text = "..."
        Me.Button9.UseVisualStyleBackColor = True
        '
        'tb_previsao
        '
        Me.tb_previsao.Location = New System.Drawing.Point(6, 28)
        Me.tb_previsao.Name = "tb_previsao"
        Me.tb_previsao.Size = New System.Drawing.Size(465, 20)
        Me.tb_previsao.TabIndex = 0
        '
        'Button8
        '
        Me.Button8.Location = New System.Drawing.Point(409, 650)
        Me.Button8.Name = "Button8"
        Me.Button8.Size = New System.Drawing.Size(145, 36)
        Me.Button8.TabIndex = 5
        Me.Button8.Text = "Salvar e fechar"
        Me.Button8.UseVisualStyleBackColor = True
        '
        'GroupBox4
        '
        Me.GroupBox4.Controls.Add(Me.Button4)
        Me.GroupBox4.Controls.Add(Me.tb_metereológicas)
        Me.GroupBox4.Location = New System.Drawing.Point(8, 140)
        Me.GroupBox4.Name = "GroupBox4"
        Me.GroupBox4.Size = New System.Drawing.Size(546, 68)
        Me.GroupBox4.TabIndex = 2
        Me.GroupBox4.TabStop = False
        Me.GroupBox4.Text = "Diretório destino de download dos arquivos de previsão meteorológica"
        '
        'Button4
        '
        Me.Button4.Location = New System.Drawing.Point(477, 28)
        Me.Button4.Name = "Button4"
        Me.Button4.Size = New System.Drawing.Size(40, 23)
        Me.Button4.TabIndex = 1
        Me.Button4.Text = "..."
        Me.Button4.UseVisualStyleBackColor = True
        '
        'tb_estreito
        '
        Me.tb_estreito.Location = New System.Drawing.Point(6, 31)
        Me.tb_estreito.Name = "tb_estreito"
        Me.tb_estreito.Size = New System.Drawing.Size(465, 20)
        Me.tb_estreito.TabIndex = 0
        '
        'GroupBox3
        '
        Me.GroupBox3.Controls.Add(Me.Button3)
        Me.GroupBox3.Controls.Add(Me.tb_estreito)
        Me.GroupBox3.Location = New System.Drawing.Point(8, 284)
        Me.GroupBox3.Name = "GroupBox3"
        Me.GroupBox3.Size = New System.Drawing.Size(546, 68)
        Me.GroupBox3.TabIndex = 1
        Me.GroupBox3.TabStop = False
        Me.GroupBox3.Text = "Diretório dos arquivos com dados observados de operação das UHEs"
        '
        'Button3
        '
        Me.Button3.Location = New System.Drawing.Point(477, 28)
        Me.Button3.Name = "Button3"
        Me.Button3.Size = New System.Drawing.Size(40, 23)
        Me.Button3.TabIndex = 1
        Me.Button3.Text = "..."
        Me.Button3.UseVisualStyleBackColor = True
        '
        'tb_metereológicas
        '
        Me.tb_metereológicas.Location = New System.Drawing.Point(9, 30)
        Me.tb_metereológicas.Name = "tb_metereológicas"
        Me.tb_metereológicas.Size = New System.Drawing.Size(465, 20)
        Me.tb_metereológicas.TabIndex = 0
        '
        'GroupBox2
        '
        Me.GroupBox2.Controls.Add(Me.Label4)
        Me.GroupBox2.Controls.Add(Me.Label3)
        Me.GroupBox2.Controls.Add(Me.Button7)
        Me.GroupBox2.Controls.Add(Me.tb_telemetria_chuva)
        Me.GroupBox2.Controls.Add(Me.Button2)
        Me.GroupBox2.Controls.Add(Me.tb_telemetria)
        Me.GroupBox2.Location = New System.Drawing.Point(8, 6)
        Me.GroupBox2.Name = "GroupBox2"
        Me.GroupBox2.Size = New System.Drawing.Size(546, 128)
        Me.GroupBox2.TabIndex = 0
        Me.GroupBox2.TabStop = False
        Me.GroupBox2.Text = "Diretório destino dos arquivos de telemetria"
        '
        'Label4
        '
        Me.Label4.AutoSize = True
        Me.Label4.Location = New System.Drawing.Point(6, 78)
        Me.Label4.Name = "Label4"
        Me.Label4.Size = New System.Drawing.Size(66, 13)
        Me.Label4.TabIndex = 5
        Me.Label4.Text = "Precipitação"
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(6, 23)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(37, 13)
        Me.Label3.TabIndex = 4
        Me.Label3.Text = "Vazão"
        '
        'Button7
        '
        Me.Button7.Location = New System.Drawing.Point(477, 94)
        Me.Button7.Name = "Button7"
        Me.Button7.Size = New System.Drawing.Size(40, 23)
        Me.Button7.TabIndex = 3
        Me.Button7.Text = "..."
        Me.Button7.UseVisualStyleBackColor = True
        '
        'tb_telemetria_chuva
        '
        Me.tb_telemetria_chuva.Location = New System.Drawing.Point(6, 94)
        Me.tb_telemetria_chuva.Name = "tb_telemetria_chuva"
        Me.tb_telemetria_chuva.Size = New System.Drawing.Size(465, 20)
        Me.tb_telemetria_chuva.TabIndex = 2
        '
        'Button2
        '
        Me.Button2.Location = New System.Drawing.Point(477, 39)
        Me.Button2.Name = "Button2"
        Me.Button2.Size = New System.Drawing.Size(40, 23)
        Me.Button2.TabIndex = 1
        Me.Button2.Text = "..."
        Me.Button2.UseVisualStyleBackColor = True
        '
        'tb_telemetria
        '
        Me.tb_telemetria.Location = New System.Drawing.Point(6, 39)
        Me.tb_telemetria.Name = "tb_telemetria"
        Me.tb_telemetria.Size = New System.Drawing.Size(465, 20)
        Me.tb_telemetria.TabIndex = 0
        '
        'TabPage2
        '
        Me.TabPage2.Controls.Add(Me.Button14)
        Me.TabPage2.Controls.Add(Me.Label6)
        Me.TabPage2.Controls.Add(Me.Button13)
        Me.TabPage2.Controls.Add(Me.GroupBox10)
        Me.TabPage2.Controls.Add(Me.Label7)
        Me.TabPage2.Controls.Add(Me.Button15)
        Me.TabPage2.Controls.Add(Me.Label5)
        Me.TabPage2.Controls.Add(Me.btCompact)
        Me.TabPage2.Controls.Add(Me.GroupBox1)
        Me.TabPage2.Controls.Add(Me.Label2)
        Me.TabPage2.Controls.Add(Me.Button1)
        Me.TabPage2.Controls.Add(Me.TextBox1)
        Me.TabPage2.Controls.Add(Me.Label1)
        Me.TabPage2.Location = New System.Drawing.Point(4, 22)
        Me.TabPage2.Name = "TabPage2"
        Me.TabPage2.Padding = New System.Windows.Forms.Padding(3)
        Me.TabPage2.Size = New System.Drawing.Size(562, 694)
        Me.TabPage2.TabIndex = 1
        Me.TabPage2.Text = "Database"
        Me.TabPage2.UseVisualStyleBackColor = True
        '
        'Button14
        '
        Me.Button14.Location = New System.Drawing.Point(48, 574)
        Me.Button14.Name = "Button14"
        Me.Button14.Size = New System.Drawing.Size(162, 65)
        Me.Button14.TabIndex = 20
        Me.Button14.Text = "Carregar e Substituir Base de Dados Externa"
        Me.Button14.UseVisualStyleBackColor = True
        '
        'Label6
        '
        Me.Label6.AutoSize = True
        Me.Label6.Location = New System.Drawing.Point(228, 442)
        Me.Label6.Name = "Label6"
        Me.Label6.Size = New System.Drawing.Size(225, 39)
        Me.Label6.TabIndex = 19
        Me.Label6.Text = "Clique no botão ao lado para exportar a pasta " & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & """database"" deste computador para " & _
            "um local" & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "predefinido."
        '
        'Button13
        '
        Me.Button13.Location = New System.Drawing.Point(48, 432)
        Me.Button13.Name = "Button13"
        Me.Button13.Size = New System.Drawing.Size(162, 65)
        Me.Button13.TabIndex = 18
        Me.Button13.Text = "Exportar Base de Dados deste Computador"
        Me.Button13.UseVisualStyleBackColor = True
        '
        'GroupBox10
        '
        Me.GroupBox10.Controls.Add(Me.pbar)
        Me.GroupBox10.Location = New System.Drawing.Point(231, 574)
        Me.GroupBox10.Name = "GroupBox10"
        Me.GroupBox10.Size = New System.Drawing.Size(238, 50)
        Me.GroupBox10.TabIndex = 17
        Me.GroupBox10.TabStop = False
        Me.GroupBox10.Text = "Progresso"
        '
        'pbar
        '
        Me.pbar.Location = New System.Drawing.Point(6, 19)
        Me.pbar.Name = "pbar"
        Me.pbar.Size = New System.Drawing.Size(224, 23)
        Me.pbar.TabIndex = 2
        '
        'Label7
        '
        Me.Label7.AutoSize = True
        Me.Label7.Location = New System.Drawing.Point(228, 514)
        Me.Label7.Name = "Label7"
        Me.Label7.Size = New System.Drawing.Size(227, 39)
        Me.Label7.TabIndex = 16
        Me.Label7.Text = "Clique no botão ao lado para carregar a pasta " & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & """database"" de algum outro local e" & _
            " mesclar ela " & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "com a database deste computador."
        '
        'Button15
        '
        Me.Button15.Location = New System.Drawing.Point(48, 503)
        Me.Button15.Name = "Button15"
        Me.Button15.Size = New System.Drawing.Size(162, 65)
        Me.Button15.TabIndex = 15
        Me.Button15.Text = "Carregar e Mesclar Base de Dados Externa"
        Me.Button15.UseVisualStyleBackColor = True
        '
        'Label5
        '
        Me.Label5.AutoSize = True
        Me.Label5.Location = New System.Drawing.Point(228, 387)
        Me.Label5.Name = "Label5"
        Me.Label5.Size = New System.Drawing.Size(297, 13)
        Me.Label5.TabIndex = 14
        Me.Label5.Text = "Clique para compactar o database até Julho do ano passado."
        Me.Label5.Visible = False
        '
        'btCompact
        '
        Me.btCompact.Location = New System.Drawing.Point(48, 361)
        Me.btCompact.Name = "btCompact"
        Me.btCompact.Size = New System.Drawing.Size(162, 65)
        Me.btCompact.TabIndex = 13
        Me.btCompact.Text = "Compactar Database"
        Me.btCompact.UseVisualStyleBackColor = True
        '
        'GroupBox11
        '
        Me.GroupBox11.Controls.Add(Me.bt_dir_export)
        Me.GroupBox11.Controls.Add(Me.tb_dir_export)
        Me.GroupBox11.Location = New System.Drawing.Point(8, 482)
        Me.GroupBox11.Name = "GroupBox11"
        Me.GroupBox11.Size = New System.Drawing.Size(546, 68)
        Me.GroupBox11.TabIndex = 11
        Me.GroupBox11.TabStop = False
        Me.GroupBox11.Text = "Diretório para exportar resultados"
        '
        'bt_dir_export
        '
        Me.bt_dir_export.Location = New System.Drawing.Point(477, 28)
        Me.bt_dir_export.Name = "bt_dir_export"
        Me.bt_dir_export.Size = New System.Drawing.Size(40, 23)
        Me.bt_dir_export.TabIndex = 1
        Me.bt_dir_export.Text = "..."
        Me.bt_dir_export.UseVisualStyleBackColor = True
        '
        'tb_dir_export
        '
        Me.tb_dir_export.Location = New System.Drawing.Point(6, 28)
        Me.tb_dir_export.Name = "tb_dir_export"
        Me.tb_dir_export.Size = New System.Drawing.Size(465, 20)
        Me.tb_dir_export.TabIndex = 0
        '
        'GroupBox6
        '
        Me.GroupBox6.Controls.Add(Me.Label10)
        Me.GroupBox6.Controls.Add(Me.Button16)
        Me.GroupBox6.Controls.Add(Me.tb_meteo_auto3)
        Me.GroupBox6.Controls.Add(Me.Label9)
        Me.GroupBox6.Controls.Add(Me.Button12)
        Me.GroupBox6.Controls.Add(Me.tb_meteo_auto2)
        Me.GroupBox6.Controls.Add(Me.Label8)
        Me.GroupBox6.Controls.Add(Me.Button6)
        Me.GroupBox6.Controls.Add(Me.tb_meteo_auto)
        Me.GroupBox6.Location = New System.Drawing.Point(8, 358)
        Me.GroupBox6.Name = "GroupBox6"
        Me.GroupBox6.Size = New System.Drawing.Size(546, 118)
        Me.GroupBox6.TabIndex = 10
        Me.GroupBox6.TabStop = False
        Me.GroupBox6.Text = "Diretório dos arquivos de previsão meteorológicas para uso na previsão automática" & _
            ""
        '
        'Label10
        '
        Me.Label10.AutoSize = True
        Me.Label10.Location = New System.Drawing.Point(21, 85)
        Me.Label10.Name = "Label10"
        Me.Label10.Size = New System.Drawing.Size(51, 13)
        Me.Label10.TabIndex = 8
        Me.Label10.Text = "Modelo 3"
        '
        'Button16
        '
        Me.Button16.Location = New System.Drawing.Point(477, 80)
        Me.Button16.Name = "Button16"
        Me.Button16.Size = New System.Drawing.Size(40, 23)
        Me.Button16.TabIndex = 7
        Me.Button16.Text = "..."
        Me.Button16.UseVisualStyleBackColor = True
        '
        'tb_meteo_auto3
        '
        Me.tb_meteo_auto3.Location = New System.Drawing.Point(78, 80)
        Me.tb_meteo_auto3.Name = "tb_meteo_auto3"
        Me.tb_meteo_auto3.Size = New System.Drawing.Size(393, 20)
        Me.tb_meteo_auto3.TabIndex = 6
        '
        'Label9
        '
        Me.Label9.AutoSize = True
        Me.Label9.Location = New System.Drawing.Point(21, 59)
        Me.Label9.Name = "Label9"
        Me.Label9.Size = New System.Drawing.Size(51, 13)
        Me.Label9.TabIndex = 5
        Me.Label9.Text = "Modelo 2"
        '
        'Button12
        '
        Me.Button12.Location = New System.Drawing.Point(477, 54)
        Me.Button12.Name = "Button12"
        Me.Button12.Size = New System.Drawing.Size(40, 23)
        Me.Button12.TabIndex = 4
        Me.Button12.Text = "..."
        Me.Button12.UseVisualStyleBackColor = True
        '
        'tb_meteo_auto2
        '
        Me.tb_meteo_auto2.Location = New System.Drawing.Point(78, 54)
        Me.tb_meteo_auto2.Name = "tb_meteo_auto2"
        Me.tb_meteo_auto2.Size = New System.Drawing.Size(393, 20)
        Me.tb_meteo_auto2.TabIndex = 3
        '
        'Label8
        '
        Me.Label8.AutoSize = True
        Me.Label8.Location = New System.Drawing.Point(21, 33)
        Me.Label8.Name = "Label8"
        Me.Label8.Size = New System.Drawing.Size(51, 13)
        Me.Label8.TabIndex = 2
        Me.Label8.Text = "Modelo 1"
        '
        'Button6
        '
        Me.Button6.Location = New System.Drawing.Point(477, 28)
        Me.Button6.Name = "Button6"
        Me.Button6.Size = New System.Drawing.Size(40, 23)
        Me.Button6.TabIndex = 1
        Me.Button6.Text = "..."
        Me.Button6.UseVisualStyleBackColor = True
        '
        'tb_meteo_auto
        '
        Me.tb_meteo_auto.Location = New System.Drawing.Point(78, 28)
        Me.tb_meteo_auto.Name = "tb_meteo_auto"
        Me.tb_meteo_auto.Size = New System.Drawing.Size(393, 20)
        Me.tb_meteo_auto.TabIndex = 0
        '
        'Form2
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(570, 720)
        Me.Controls.Add(Me.TabControl1)
        Me.FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedToolWindow
        Me.Name = "Form2"
        Me.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen
        Me.Text = "Configuração"
        Me.GroupBox1.ResumeLayout(False)
        CType(Me.DataGridView1, System.ComponentModel.ISupportInitialize).EndInit()
        CType(Me.DataSet1, System.ComponentModel.ISupportInitialize).EndInit()
        CType(Me.DataTable1, System.ComponentModel.ISupportInitialize).EndInit()
        Me.TabControl1.ResumeLayout(False)
        Me.TabPage1.ResumeLayout(False)
        Me.GroupBox7.ResumeLayout(False)
        Me.GroupBox7.PerformLayout()
        Me.GroupBox4.ResumeLayout(False)
        Me.GroupBox4.PerformLayout()
        Me.GroupBox3.ResumeLayout(False)
        Me.GroupBox3.PerformLayout()
        Me.GroupBox2.ResumeLayout(False)
        Me.GroupBox2.PerformLayout()
        Me.TabPage2.ResumeLayout(False)
        Me.TabPage2.PerformLayout()
        Me.GroupBox10.ResumeLayout(False)
        Me.GroupBox11.ResumeLayout(False)
        Me.GroupBox11.PerformLayout()
        Me.GroupBox6.ResumeLayout(False)
        Me.GroupBox6.PerformLayout()
        Me.ResumeLayout(False)

    End Sub
    Friend WithEvents GroupBox1 As System.Windows.Forms.GroupBox
    Friend WithEvents DataGridView1 As System.Windows.Forms.DataGridView
    Friend WithEvents DataSet1 As System.Data.DataSet
    Friend WithEvents Button1 As System.Windows.Forms.Button
    Friend WithEvents Label1 As System.Windows.Forms.Label
    Friend WithEvents DataTable1 As System.Data.DataTable
    Friend WithEvents DataColumn1 As System.Data.DataColumn
    Friend WithEvents TextBox1 As System.Windows.Forms.TextBox
    Friend WithEvents Label2 As System.Windows.Forms.Label
    Friend WithEvents EstaçãoDataGridViewTextBoxColumn As System.Windows.Forms.DataGridViewTextBoxColumn
    Friend WithEvents TabControl1 As System.Windows.Forms.TabControl
    Friend WithEvents TabPage1 As System.Windows.Forms.TabPage
    Friend WithEvents TabPage2 As System.Windows.Forms.TabPage
    Friend WithEvents GroupBox3 As System.Windows.Forms.GroupBox
    Friend WithEvents Button3 As System.Windows.Forms.Button
    Friend WithEvents tb_metereológicas As System.Windows.Forms.TextBox
    Friend WithEvents GroupBox2 As System.Windows.Forms.GroupBox
    Friend WithEvents Button2 As System.Windows.Forms.Button
    Friend WithEvents tb_telemetria As System.Windows.Forms.TextBox
    Friend WithEvents GroupBox4 As System.Windows.Forms.GroupBox
    Friend WithEvents Button4 As System.Windows.Forms.Button
    Friend WithEvents tb_estreito As System.Windows.Forms.TextBox
    Friend WithEvents fbd As System.Windows.Forms.FolderBrowserDialog
    Friend WithEvents Button7 As System.Windows.Forms.Button
    Friend WithEvents tb_telemetria_chuva As System.Windows.Forms.TextBox
    Friend WithEvents Label4 As System.Windows.Forms.Label
    Friend WithEvents Label3 As System.Windows.Forms.Label
    Friend WithEvents Button8 As System.Windows.Forms.Button
    Friend WithEvents GroupBox7 As System.Windows.Forms.GroupBox
    Friend WithEvents Button9 As System.Windows.Forms.Button
    Friend WithEvents tb_previsao As System.Windows.Forms.TextBox
    Friend WithEvents Button14 As System.Windows.Forms.Button
    Friend WithEvents Label6 As System.Windows.Forms.Label
    Friend WithEvents Button13 As System.Windows.Forms.Button
    Friend WithEvents GroupBox10 As System.Windows.Forms.GroupBox
    Friend WithEvents pbar As System.Windows.Forms.ProgressBar
    Friend WithEvents Label7 As System.Windows.Forms.Label
    Friend WithEvents Button15 As System.Windows.Forms.Button
    Friend WithEvents Label5 As System.Windows.Forms.Label
    Friend WithEvents btCompact As System.Windows.Forms.Button
    Friend WithEvents GroupBox11 As System.Windows.Forms.GroupBox
    Friend WithEvents bt_dir_export As System.Windows.Forms.Button
    Friend WithEvents tb_dir_export As System.Windows.Forms.TextBox
    Friend WithEvents GroupBox6 As System.Windows.Forms.GroupBox
    Friend WithEvents Label10 As System.Windows.Forms.Label
    Friend WithEvents Button16 As System.Windows.Forms.Button
    Friend WithEvents tb_meteo_auto3 As System.Windows.Forms.TextBox
    Friend WithEvents Label9 As System.Windows.Forms.Label
    Friend WithEvents Button12 As System.Windows.Forms.Button
    Friend WithEvents tb_meteo_auto2 As System.Windows.Forms.TextBox
    Friend WithEvents Label8 As System.Windows.Forms.Label
    Friend WithEvents Button6 As System.Windows.Forms.Button
    Friend WithEvents tb_meteo_auto As System.Windows.Forms.TextBox
End Class
