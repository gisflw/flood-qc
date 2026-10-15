<Global.Microsoft.VisualBasic.CompilerServices.DesignerGenerated()> _
Partial Class Form15
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
        Me.Label1 = New System.Windows.Forms.Label
        Me.Label3 = New System.Windows.Forms.Label
        Me.Label4 = New System.Windows.Forms.Label
        Me.Label5 = New System.Windows.Forms.Label
        Me.Label6 = New System.Windows.Forms.Label
        Me.cbPOBShora = New System.Windows.Forms.ComboBox
        Me.cbPOBSmin = New System.Windows.Forms.ComboBox
        Me.Label7 = New System.Windows.Forms.Label
        Me.Label9 = New System.Windows.Forms.Label
        Me.Timer1 = New System.Windows.Forms.Timer(Me.components)
        Me.Label8 = New System.Windows.Forms.Label
        Me.Label10 = New System.Windows.Forms.Label
        Me.GroupBox1 = New System.Windows.Forms.GroupBox
        Me.Button2 = New System.Windows.Forms.Button
        Me.Button1 = New System.Windows.Forms.Button
        Me.Label11 = New System.Windows.Forms.Label
        Me.GroupBox2 = New System.Windows.Forms.GroupBox
        Me.rbETA40 = New System.Windows.Forms.RadioButton
        Me.rbGEFS = New System.Windows.Forms.RadioButton
        Me.cbEMAILminCNO = New System.Windows.Forms.ComboBox
        Me.cbEMAILhoraCNO = New System.Windows.Forms.ComboBox
        Me.Label13 = New System.Windows.Forms.Label
        Me.cbPPREVhora = New System.Windows.Forms.ComboBox
        Me.cbPPREVmin = New System.Windows.Forms.ComboBox
        Me.cbEMAILmin = New System.Windows.Forms.ComboBox
        Me.cbEMAILhora = New System.Windows.Forms.ComboBox
        Me.Label12 = New System.Windows.Forms.Label
        Me.cbGRAVAmin = New System.Windows.Forms.ComboBox
        Me.cbGRAVAhora = New System.Windows.Forms.ComboBox
        Me.cbRODARmin = New System.Windows.Forms.ComboBox
        Me.cbRODARhora = New System.Windows.Forms.ComboBox
        Me.cbUHEmin = New System.Windows.Forms.ComboBox
        Me.cbUHEhora = New System.Windows.Forms.ComboBox
        Me.cbQOBSmin = New System.Windows.Forms.ComboBox
        Me.cbQOBShora = New System.Windows.Forms.ComboBox
        Me.checkVerif = New System.Windows.Forms.CheckBox
        Me.cbVERIFICARmin = New System.Windows.Forms.ComboBox
        Me.cbVERIFICARhora = New System.Windows.Forms.ComboBox
        Me.Button3 = New System.Windows.Forms.Button
        Me.GroupBox3 = New System.Windows.Forms.GroupBox
        Me.Label14 = New System.Windows.Forms.Label
        Me.cbPPREVmin2 = New System.Windows.Forms.ComboBox
        Me.cbPPREVhora2 = New System.Windows.Forms.ComboBox
        Me.GroupBox4 = New System.Windows.Forms.GroupBox
        Me.rbETA05 = New System.Windows.Forms.RadioButton
        Me.GroupBox1.SuspendLayout()
        Me.GroupBox2.SuspendLayout()
        Me.GroupBox3.SuspendLayout()
        Me.GroupBox4.SuspendLayout()
        Me.SuspendLayout()
        '
        'Label1
        '
        Me.Label1.AutoSize = True
        Me.Label1.Location = New System.Drawing.Point(20, 36)
        Me.Label1.Name = "Label1"
        Me.Label1.Size = New System.Drawing.Size(93, 13)
        Me.Label1.TabIndex = 0
        Me.Label1.Text = "Chuva Observada"
        '
        'Label3
        '
        Me.Label3.AutoSize = True
        Me.Label3.Location = New System.Drawing.Point(20, 135)
        Me.Label3.Name = "Label3"
        Me.Label3.Size = New System.Drawing.Size(92, 13)
        Me.Label3.TabIndex = 2
        Me.Label3.Text = "Vazão Observada"
        '
        'Label4
        '
        Me.Label4.AutoSize = True
        Me.Label4.Location = New System.Drawing.Point(21, 174)
        Me.Label4.Name = "Label4"
        Me.Label4.Size = New System.Drawing.Size(68, 13)
        Me.Label4.TabIndex = 3
        Me.Label4.Text = "Vazão UHEs"
        '
        'Label5
        '
        Me.Label5.AutoSize = True
        Me.Label5.Location = New System.Drawing.Point(21, 210)
        Me.Label5.Name = "Label5"
        Me.Label5.Size = New System.Drawing.Size(80, 13)
        Me.Label5.TabIndex = 4
        Me.Label5.Text = "Rodar Previsão"
        '
        'Label6
        '
        Me.Label6.AutoSize = True
        Me.Label6.Location = New System.Drawing.Point(21, 245)
        Me.Label6.Name = "Label6"
        Me.Label6.Size = New System.Drawing.Size(95, 13)
        Me.Label6.TabIndex = 5
        Me.Label6.Text = "Gravar Resultados"
        '
        'cbPOBShora
        '
        Me.cbPOBShora.FormattingEnabled = True
        Me.cbPOBShora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbPOBShora.Location = New System.Drawing.Point(175, 33)
        Me.cbPOBShora.Name = "cbPOBShora"
        Me.cbPOBShora.Size = New System.Drawing.Size(54, 21)
        Me.cbPOBShora.TabIndex = 6
        Me.cbPOBShora.Text = "08"
        '
        'cbPOBSmin
        '
        Me.cbPOBSmin.FormattingEnabled = True
        Me.cbPOBSmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbPOBSmin.Location = New System.Drawing.Point(252, 33)
        Me.cbPOBSmin.Name = "cbPOBSmin"
        Me.cbPOBSmin.Size = New System.Drawing.Size(54, 21)
        Me.cbPOBSmin.TabIndex = 7
        Me.cbPOBSmin.Text = "10"
        '
        'Label7
        '
        Me.Label7.AutoSize = True
        Me.Label7.Location = New System.Drawing.Point(172, 14)
        Me.Label7.Name = "Label7"
        Me.Label7.Size = New System.Drawing.Size(30, 13)
        Me.Label7.TabIndex = 8
        Me.Label7.Text = "Hora"
        '
        'Label9
        '
        Me.Label9.AutoSize = True
        Me.Label9.Location = New System.Drawing.Point(249, 14)
        Me.Label9.Name = "Label9"
        Me.Label9.Size = New System.Drawing.Size(24, 13)
        Me.Label9.TabIndex = 9
        Me.Label9.Text = "Min"
        '
        'Timer1
        '
        Me.Timer1.Interval = 1000
        '
        'Label8
        '
        Me.Label8.AutoSize = True
        Me.Label8.Location = New System.Drawing.Point(10, 33)
        Me.Label8.Name = "Label8"
        Me.Label8.Size = New System.Drawing.Size(33, 13)
        Me.Label8.TabIndex = 11
        Me.Label8.Text = "Data:"
        '
        'Label10
        '
        Me.Label10.AutoSize = True
        Me.Label10.Location = New System.Drawing.Point(43, 33)
        Me.Label10.Name = "Label10"
        Me.Label10.Size = New System.Drawing.Size(30, 13)
        Me.Label10.TabIndex = 10
        Me.Label10.Text = "Hora"
        '
        'GroupBox1
        '
        Me.GroupBox1.Controls.Add(Me.Button2)
        Me.GroupBox1.Controls.Add(Me.Button1)
        Me.GroupBox1.Controls.Add(Me.Label11)
        Me.GroupBox1.Controls.Add(Me.Label10)
        Me.GroupBox1.Controls.Add(Me.Label8)
        Me.GroupBox1.Location = New System.Drawing.Point(37, 399)
        Me.GroupBox1.Name = "GroupBox1"
        Me.GroupBox1.Size = New System.Drawing.Size(349, 72)
        Me.GroupBox1.TabIndex = 12
        Me.GroupBox1.TabStop = False
        Me.GroupBox1.Text = "Controle"
        '
        'Button2
        '
        Me.Button2.Location = New System.Drawing.Point(268, 16)
        Me.Button2.Name = "Button2"
        Me.Button2.Size = New System.Drawing.Size(75, 47)
        Me.Button2.TabIndex = 14
        Me.Button2.Text = "Desliga"
        Me.Button2.UseVisualStyleBackColor = True
        '
        'Button1
        '
        Me.Button1.Location = New System.Drawing.Point(187, 16)
        Me.Button1.Name = "Button1"
        Me.Button1.Size = New System.Drawing.Size(75, 47)
        Me.Button1.TabIndex = 13
        Me.Button1.Text = "Liga"
        Me.Button1.UseVisualStyleBackColor = True
        '
        'Label11
        '
        Me.Label11.AutoSize = True
        Me.Label11.ForeColor = System.Drawing.Color.Red
        Me.Label11.Location = New System.Drawing.Point(119, 33)
        Me.Label11.Name = "Label11"
        Me.Label11.Size = New System.Drawing.Size(54, 13)
        Me.Label11.TabIndex = 12
        Me.Label11.Text = "Desligado"
        '
        'GroupBox2
        '
        Me.GroupBox2.Controls.Add(Me.rbETA05)
        Me.GroupBox2.Controls.Add(Me.rbETA40)
        Me.GroupBox2.Controls.Add(Me.rbGEFS)
        Me.GroupBox2.Controls.Add(Me.cbEMAILminCNO)
        Me.GroupBox2.Controls.Add(Me.cbEMAILhoraCNO)
        Me.GroupBox2.Controls.Add(Me.Label13)
        Me.GroupBox2.Controls.Add(Me.cbPPREVhora)
        Me.GroupBox2.Controls.Add(Me.cbPPREVmin)
        Me.GroupBox2.Controls.Add(Me.cbEMAILmin)
        Me.GroupBox2.Controls.Add(Me.cbEMAILhora)
        Me.GroupBox2.Controls.Add(Me.Label12)
        Me.GroupBox2.Controls.Add(Me.cbGRAVAmin)
        Me.GroupBox2.Controls.Add(Me.cbGRAVAhora)
        Me.GroupBox2.Controls.Add(Me.cbRODARmin)
        Me.GroupBox2.Controls.Add(Me.cbRODARhora)
        Me.GroupBox2.Controls.Add(Me.cbUHEmin)
        Me.GroupBox2.Controls.Add(Me.cbUHEhora)
        Me.GroupBox2.Controls.Add(Me.cbQOBSmin)
        Me.GroupBox2.Controls.Add(Me.cbQOBShora)
        Me.GroupBox2.Controls.Add(Me.Label1)
        Me.GroupBox2.Controls.Add(Me.Label9)
        Me.GroupBox2.Controls.Add(Me.Label3)
        Me.GroupBox2.Controls.Add(Me.Label7)
        Me.GroupBox2.Controls.Add(Me.Label4)
        Me.GroupBox2.Controls.Add(Me.cbPOBSmin)
        Me.GroupBox2.Controls.Add(Me.Label5)
        Me.GroupBox2.Controls.Add(Me.cbPOBShora)
        Me.GroupBox2.Controls.Add(Me.Label6)
        Me.GroupBox2.Location = New System.Drawing.Point(37, 6)
        Me.GroupBox2.Name = "GroupBox2"
        Me.GroupBox2.Size = New System.Drawing.Size(349, 387)
        Me.GroupBox2.TabIndex = 13
        Me.GroupBox2.TabStop = False
        Me.GroupBox2.Text = "Janelas"
        '
        'rbETA40
        '
        Me.rbETA40.AutoSize = True
        Me.rbETA40.Location = New System.Drawing.Point(12, 85)
        Me.rbETA40.Name = "rbETA40"
        Me.rbETA40.Size = New System.Drawing.Size(139, 17)
        Me.rbETA40.TabIndex = 1
        Me.rbETA40.Text = "Chuva Prevista (ETA40)"
        Me.rbETA40.UseVisualStyleBackColor = True
        '
        'rbGEFS
        '
        Me.rbGEFS.AutoSize = True
        Me.rbGEFS.Location = New System.Drawing.Point(13, 63)
        Me.rbGEFS.Name = "rbGEFS"
        Me.rbGEFS.Size = New System.Drawing.Size(134, 17)
        Me.rbGEFS.TabIndex = 0
        Me.rbGEFS.Text = "Chuva Prevista (GEFS)"
        Me.rbGEFS.UseVisualStyleBackColor = True
        '
        'cbEMAILminCNO
        '
        Me.cbEMAILminCNO.FormattingEnabled = True
        Me.cbEMAILminCNO.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbEMAILminCNO.Location = New System.Drawing.Point(252, 321)
        Me.cbEMAILminCNO.Name = "cbEMAILminCNO"
        Me.cbEMAILminCNO.Size = New System.Drawing.Size(54, 21)
        Me.cbEMAILminCNO.TabIndex = 29
        Me.cbEMAILminCNO.Text = "40"
        '
        'cbEMAILhoraCNO
        '
        Me.cbEMAILhoraCNO.FormattingEnabled = True
        Me.cbEMAILhoraCNO.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbEMAILhoraCNO.Location = New System.Drawing.Point(175, 321)
        Me.cbEMAILhoraCNO.Name = "cbEMAILhoraCNO"
        Me.cbEMAILhoraCNO.Size = New System.Drawing.Size(54, 21)
        Me.cbEMAILhoraCNO.TabIndex = 28
        Me.cbEMAILhoraCNO.Text = "08"
        '
        'Label13
        '
        Me.Label13.AutoSize = True
        Me.Label13.Location = New System.Drawing.Point(21, 324)
        Me.Label13.Name = "Label13"
        Me.Label13.Size = New System.Drawing.Size(142, 13)
        Me.Label13.TabIndex = 27
        Me.Label13.Text = "Enviar e-mail Campos Novos"
        '
        'cbPPREVhora
        '
        Me.cbPPREVhora.FormattingEnabled = True
        Me.cbPPREVhora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbPPREVhora.Location = New System.Drawing.Point(175, 80)
        Me.cbPPREVhora.Name = "cbPPREVhora"
        Me.cbPPREVhora.Size = New System.Drawing.Size(54, 21)
        Me.cbPPREVhora.TabIndex = 10
        Me.cbPPREVhora.Text = "08"
        '
        'cbPPREVmin
        '
        Me.cbPPREVmin.FormattingEnabled = True
        Me.cbPPREVmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbPPREVmin.Location = New System.Drawing.Point(252, 80)
        Me.cbPPREVmin.Name = "cbPPREVmin"
        Me.cbPPREVmin.Size = New System.Drawing.Size(54, 21)
        Me.cbPPREVmin.TabIndex = 11
        Me.cbPPREVmin.Text = "20"
        '
        'cbEMAILmin
        '
        Me.cbEMAILmin.FormattingEnabled = True
        Me.cbEMAILmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbEMAILmin.Location = New System.Drawing.Point(252, 281)
        Me.cbEMAILmin.Name = "cbEMAILmin"
        Me.cbEMAILmin.Size = New System.Drawing.Size(54, 21)
        Me.cbEMAILmin.TabIndex = 22
        Me.cbEMAILmin.Text = "40"
        '
        'cbEMAILhora
        '
        Me.cbEMAILhora.FormattingEnabled = True
        Me.cbEMAILhora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbEMAILhora.Location = New System.Drawing.Point(175, 281)
        Me.cbEMAILhora.Name = "cbEMAILhora"
        Me.cbEMAILhora.Size = New System.Drawing.Size(54, 21)
        Me.cbEMAILhora.TabIndex = 21
        Me.cbEMAILhora.Text = "08"
        '
        'Label12
        '
        Me.Label12.AutoSize = True
        Me.Label12.Location = New System.Drawing.Point(21, 284)
        Me.Label12.Name = "Label12"
        Me.Label12.Size = New System.Drawing.Size(67, 13)
        Me.Label12.TabIndex = 20
        Me.Label12.Text = "Enviar e-mail"
        '
        'cbGRAVAmin
        '
        Me.cbGRAVAmin.FormattingEnabled = True
        Me.cbGRAVAmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbGRAVAmin.Location = New System.Drawing.Point(252, 242)
        Me.cbGRAVAmin.Name = "cbGRAVAmin"
        Me.cbGRAVAmin.Size = New System.Drawing.Size(54, 21)
        Me.cbGRAVAmin.TabIndex = 19
        Me.cbGRAVAmin.Text = "37"
        '
        'cbGRAVAhora
        '
        Me.cbGRAVAhora.FormattingEnabled = True
        Me.cbGRAVAhora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbGRAVAhora.Location = New System.Drawing.Point(175, 242)
        Me.cbGRAVAhora.Name = "cbGRAVAhora"
        Me.cbGRAVAhora.Size = New System.Drawing.Size(54, 21)
        Me.cbGRAVAhora.TabIndex = 18
        Me.cbGRAVAhora.Text = "08"
        '
        'cbRODARmin
        '
        Me.cbRODARmin.FormattingEnabled = True
        Me.cbRODARmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbRODARmin.Location = New System.Drawing.Point(252, 207)
        Me.cbRODARmin.Name = "cbRODARmin"
        Me.cbRODARmin.Size = New System.Drawing.Size(54, 21)
        Me.cbRODARmin.TabIndex = 17
        Me.cbRODARmin.Text = "32"
        '
        'cbRODARhora
        '
        Me.cbRODARhora.FormattingEnabled = True
        Me.cbRODARhora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbRODARhora.Location = New System.Drawing.Point(175, 207)
        Me.cbRODARhora.Name = "cbRODARhora"
        Me.cbRODARhora.Size = New System.Drawing.Size(54, 21)
        Me.cbRODARhora.TabIndex = 16
        Me.cbRODARhora.Text = "08"
        '
        'cbUHEmin
        '
        Me.cbUHEmin.FormattingEnabled = True
        Me.cbUHEmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbUHEmin.Location = New System.Drawing.Point(252, 171)
        Me.cbUHEmin.Name = "cbUHEmin"
        Me.cbUHEmin.Size = New System.Drawing.Size(54, 21)
        Me.cbUHEmin.TabIndex = 15
        Me.cbUHEmin.Text = "30"
        '
        'cbUHEhora
        '
        Me.cbUHEhora.FormattingEnabled = True
        Me.cbUHEhora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbUHEhora.Location = New System.Drawing.Point(175, 171)
        Me.cbUHEhora.Name = "cbUHEhora"
        Me.cbUHEhora.Size = New System.Drawing.Size(54, 21)
        Me.cbUHEhora.TabIndex = 14
        Me.cbUHEhora.Text = "08"
        '
        'cbQOBSmin
        '
        Me.cbQOBSmin.FormattingEnabled = True
        Me.cbQOBSmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbQOBSmin.Location = New System.Drawing.Point(252, 132)
        Me.cbQOBSmin.Name = "cbQOBSmin"
        Me.cbQOBSmin.Size = New System.Drawing.Size(54, 21)
        Me.cbQOBSmin.TabIndex = 13
        Me.cbQOBSmin.Text = "25"
        '
        'cbQOBShora
        '
        Me.cbQOBShora.FormattingEnabled = True
        Me.cbQOBShora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbQOBShora.Location = New System.Drawing.Point(175, 132)
        Me.cbQOBShora.Name = "cbQOBShora"
        Me.cbQOBShora.Size = New System.Drawing.Size(54, 21)
        Me.cbQOBShora.TabIndex = 12
        Me.cbQOBShora.Text = "08"
        '
        'checkVerif
        '
        Me.checkVerif.AutoSize = True
        Me.checkVerif.Location = New System.Drawing.Point(57, 69)
        Me.checkVerif.Name = "checkVerif"
        Me.checkVerif.Size = New System.Drawing.Size(64, 17)
        Me.checkVerif.TabIndex = 26
        Me.checkVerif.Text = "Verificar"
        Me.checkVerif.UseVisualStyleBackColor = True
        '
        'cbVERIFICARmin
        '
        Me.cbVERIFICARmin.FormattingEnabled = True
        Me.cbVERIFICARmin.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbVERIFICARmin.Location = New System.Drawing.Point(234, 67)
        Me.cbVERIFICARmin.Name = "cbVERIFICARmin"
        Me.cbVERIFICARmin.Size = New System.Drawing.Size(54, 21)
        Me.cbVERIFICARmin.TabIndex = 25
        Me.cbVERIFICARmin.Text = "40"
        '
        'cbVERIFICARhora
        '
        Me.cbVERIFICARhora.FormattingEnabled = True
        Me.cbVERIFICARhora.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbVERIFICARhora.Location = New System.Drawing.Point(157, 67)
        Me.cbVERIFICARhora.Name = "cbVERIFICARhora"
        Me.cbVERIFICARhora.Size = New System.Drawing.Size(54, 21)
        Me.cbVERIFICARhora.TabIndex = 24
        Me.cbVERIFICARhora.Text = "08"
        '
        'Button3
        '
        Me.Button3.Location = New System.Drawing.Point(300, 38)
        Me.Button3.Name = "Button3"
        Me.Button3.Size = New System.Drawing.Size(78, 23)
        Me.Button3.TabIndex = 30
        Me.Button3.Text = "Emitir"
        Me.Button3.UseVisualStyleBackColor = True
        '
        'GroupBox3
        '
        Me.GroupBox3.Controls.Add(Me.Label14)
        Me.GroupBox3.Controls.Add(Me.Button3)
        Me.GroupBox3.Location = New System.Drawing.Point(392, 6)
        Me.GroupBox3.Name = "GroupBox3"
        Me.GroupBox3.Size = New System.Drawing.Size(393, 94)
        Me.GroupBox3.TabIndex = 30
        Me.GroupBox3.TabStop = False
        Me.GroupBox3.Text = "Gerar Boletim da Previsão"
        '
        'Label14
        '
        Me.Label14.AutoSize = True
        Me.Label14.Location = New System.Drawing.Point(22, 35)
        Me.Label14.Name = "Label14"
        Me.Label14.Size = New System.Drawing.Size(261, 26)
        Me.Label14.TabIndex = 31
        Me.Label14.Text = "Clique em ""emitir"" para gerar o boletim na pasta Prev," & Global.Microsoft.VisualBasic.ChrW(13) & Global.Microsoft.VisualBasic.ChrW(10) & "se você já Rodou a previs" & _
            "ão e conferiu os resultados."
        '
        'cbPPREVmin2
        '
        Me.cbPPREVmin2.FormattingEnabled = True
        Me.cbPPREVmin2.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59"})
        Me.cbPPREVmin2.Location = New System.Drawing.Point(234, 40)
        Me.cbPPREVmin2.Name = "cbPPREVmin2"
        Me.cbPPREVmin2.Size = New System.Drawing.Size(54, 21)
        Me.cbPPREVmin2.TabIndex = 32
        Me.cbPPREVmin2.Text = "20"
        '
        'cbPPREVhora2
        '
        Me.cbPPREVhora2.FormattingEnabled = True
        Me.cbPPREVhora2.Items.AddRange(New Object() {"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"})
        Me.cbPPREVhora2.Location = New System.Drawing.Point(157, 40)
        Me.cbPPREVhora2.Name = "cbPPREVhora2"
        Me.cbPPREVhora2.Size = New System.Drawing.Size(54, 21)
        Me.cbPPREVhora2.TabIndex = 31
        Me.cbPPREVhora2.Text = "08"
        '
        'GroupBox4
        '
        Me.GroupBox4.Controls.Add(Me.cbPPREVhora2)
        Me.GroupBox4.Controls.Add(Me.cbPPREVmin2)
        Me.GroupBox4.Controls.Add(Me.checkVerif)
        Me.GroupBox4.Controls.Add(Me.cbVERIFICARhora)
        Me.GroupBox4.Controls.Add(Me.cbVERIFICARmin)
        Me.GroupBox4.Location = New System.Drawing.Point(392, 278)
        Me.GroupBox4.Name = "GroupBox4"
        Me.GroupBox4.Size = New System.Drawing.Size(393, 115)
        Me.GroupBox4.TabIndex = 33
        Me.GroupBox4.TabStop = False
        Me.GroupBox4.Text = "Ferramentas fora de uso"
        Me.GroupBox4.Visible = False
        '
        'rbETA05
        '
        Me.rbETA05.AutoSize = True
        Me.rbETA05.Checked = True
        Me.rbETA05.Location = New System.Drawing.Point(12, 107)
        Me.rbETA05.Name = "rbETA05"
        Me.rbETA05.Size = New System.Drawing.Size(139, 17)
        Me.rbETA05.TabIndex = 30
        Me.rbETA05.Text = "Chuva Prevista (ETA05)"
        Me.rbETA05.UseVisualStyleBackColor = True
        '
        'Form15
        '
        Me.AutoScaleDimensions = New System.Drawing.SizeF(6.0!, 13.0!)
        Me.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font
        Me.ClientSize = New System.Drawing.Size(797, 481)
        Me.Controls.Add(Me.GroupBox4)
        Me.Controls.Add(Me.GroupBox3)
        Me.Controls.Add(Me.GroupBox2)
        Me.Controls.Add(Me.GroupBox1)
        Me.Name = "Form15"
        Me.Text = "TotalCommander"
        Me.GroupBox1.ResumeLayout(False)
        Me.GroupBox1.PerformLayout()
        Me.GroupBox2.ResumeLayout(False)
        Me.GroupBox2.PerformLayout()
        Me.GroupBox3.ResumeLayout(False)
        Me.GroupBox3.PerformLayout()
        Me.GroupBox4.ResumeLayout(False)
        Me.GroupBox4.PerformLayout()
        Me.ResumeLayout(False)

    End Sub
    Friend WithEvents Label1 As System.Windows.Forms.Label
    Friend WithEvents Label3 As System.Windows.Forms.Label
    Friend WithEvents Label4 As System.Windows.Forms.Label
    Friend WithEvents Label5 As System.Windows.Forms.Label
    Friend WithEvents Label6 As System.Windows.Forms.Label
    Friend WithEvents cbPOBShora As System.Windows.Forms.ComboBox
    Friend WithEvents cbPOBSmin As System.Windows.Forms.ComboBox
    Friend WithEvents Label7 As System.Windows.Forms.Label
    Friend WithEvents Label9 As System.Windows.Forms.Label
    Friend WithEvents Timer1 As System.Windows.Forms.Timer
    Friend WithEvents Label8 As System.Windows.Forms.Label
    Friend WithEvents Label10 As System.Windows.Forms.Label
    Friend WithEvents GroupBox1 As System.Windows.Forms.GroupBox
    Friend WithEvents Button1 As System.Windows.Forms.Button
    Friend WithEvents Label11 As System.Windows.Forms.Label
    Friend WithEvents Button2 As System.Windows.Forms.Button
    Friend WithEvents GroupBox2 As System.Windows.Forms.GroupBox
    Friend WithEvents cbQOBSmin As System.Windows.Forms.ComboBox
    Friend WithEvents cbQOBShora As System.Windows.Forms.ComboBox
    Friend WithEvents cbPPREVmin As System.Windows.Forms.ComboBox
    Friend WithEvents cbPPREVhora As System.Windows.Forms.ComboBox
    Friend WithEvents cbRODARmin As System.Windows.Forms.ComboBox
    Friend WithEvents cbRODARhora As System.Windows.Forms.ComboBox
    Friend WithEvents cbUHEmin As System.Windows.Forms.ComboBox
    Friend WithEvents cbUHEhora As System.Windows.Forms.ComboBox
    Friend WithEvents cbGRAVAmin As System.Windows.Forms.ComboBox
    Friend WithEvents cbGRAVAhora As System.Windows.Forms.ComboBox
    Friend WithEvents cbEMAILmin As System.Windows.Forms.ComboBox
    Friend WithEvents cbEMAILhora As System.Windows.Forms.ComboBox
    Friend WithEvents Label12 As System.Windows.Forms.Label
    Friend WithEvents cbVERIFICARmin As System.Windows.Forms.ComboBox
    Friend WithEvents cbVERIFICARhora As System.Windows.Forms.ComboBox
    Friend WithEvents checkVerif As System.Windows.Forms.CheckBox
    Friend WithEvents cbEMAILminCNO As System.Windows.Forms.ComboBox
    Friend WithEvents cbEMAILhoraCNO As System.Windows.Forms.ComboBox
    Friend WithEvents Label13 As System.Windows.Forms.Label
    Friend WithEvents Button3 As System.Windows.Forms.Button
    Friend WithEvents GroupBox3 As System.Windows.Forms.GroupBox
    Friend WithEvents Label14 As System.Windows.Forms.Label
    Friend WithEvents cbPPREVmin2 As System.Windows.Forms.ComboBox
    Friend WithEvents cbPPREVhora2 As System.Windows.Forms.ComboBox
    Friend WithEvents rbETA40 As System.Windows.Forms.RadioButton
    Friend WithEvents rbGEFS As System.Windows.Forms.RadioButton
    Friend WithEvents GroupBox4 As System.Windows.Forms.GroupBox
    Friend WithEvents rbETA05 As System.Windows.Forms.RadioButton
End Class
