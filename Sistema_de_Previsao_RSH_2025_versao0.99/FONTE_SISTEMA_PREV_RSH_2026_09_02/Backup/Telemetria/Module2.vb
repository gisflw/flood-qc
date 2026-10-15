Module Module2
    'Este módulo contém as funções de leitura de números com diferentes separadores decimais


    ''' <summary>
    ''' Função usada para ler os dados com separador decimal virgula ou ponto.
    ''' </summary>
    ''' <param name="valor">O dado (string de texto) a ser lido para retornar como valor decimal (Single).</param>

    Public Function AjustarDecimal(ByVal valor As String) As Single

        'Se o valor lido á vazio, identifiquei ele como erro = -1
        If valor = "" Then Return -1

        'Atribui o separador decimal que vem no valor de entrada, para um valor nulo
        Dim separadorEntrada As String = ""

        'Cria as condições para avaliar o separador decimal do texto inserido
        Dim padrao As String = "\d" 'identifica qualquer dígito o que pode parecer como símbolo decimal
        Dim r As New System.Text.RegularExpressions.Regex(padrao)
        Dim valoresSeparados() As String = r.Split(valor)

        'Verifica se um dos valores separados é ponto ou vírgula
        For i = valoresSeparados.Count - 1 To 0 Step -1
            If valoresSeparados(i) = "." Then
                separadorEntrada = "."
                Exit For
            ElseIf valoresSeparados(i) = "," Then
                separadorEntrada = ","
                Exit For
            End If
        Next

        'Caso não haja separador ou; caso o separador utilizado no valor de entrada seja o mesmo das configurações regionais do computador retorna o próprio valor
        If separadorEntrada = "" Or separadorEntrada = Globalization.CultureInfo.CurrentCulture.NumberFormat.NumberDecimalSeparator Then Return CSng(valor)

        'Cria um Formatador de número no padrão americano, com o separador de decimal -> ponto
        Dim formatadorNumero As Globalization.NumberFormatInfo = New System.Globalization.CultureInfo("en-US").NumberFormat

        Select Case separadorEntrada
            Case "."
                'Utiliza diretamente o padrão americano com o decimal ponto.
                Return Convert.ToSingle(valor, formatadorNumero)
            Case ","
                'Substitui o separador decimal pela vírgula 
                formatadorNumero.NumberDecimalSeparator = ","
                formatadorNumero.NumberGroupSeparator = "."
                Return Convert.ToSingle(valor, formatadorNumero)
            Case Else
                Throw New Exception("O separador decimal nos dados de entrada não foi reconhecido.")
        End Select

    End Function


    'ESCREVER



    '''
    ''' Função que substitui o separador decimal para ponto, caso necessário
    '''
    ''' O valor em formato de texto, com o número de casas decimais já corrigido.
    ''' 
    Public Function SubstituirSeparador(ByVal valor As Single) As String
        Dim value As String
        Dim ValorFinal As String

        value = Convert.ToString(FormatNumber(valor, 2, TriState.True, TriState.False, TriState.False))
        Dim separador As String = Globalization.CultureInfo.CurrentCulture.NumberFormat.NumberDecimalSeparator

        ValorFinal = value

        If valor < 1000 Then

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                ValorFinal = value.Replace(separador, ".")
            End If

        ElseIf valor >= 1000 And valor < 10000 Then

            'MsgBox(valor & " " & value)

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                'ValorFinal = value.Substring(0, 1) & value.Substring(2, 6).Replace(separador, ".")
                ValorFinal = value.Replace(separador, ".")

            End If

            ' MsgBox(valor & " " & value)


        ElseIf valor >= 10000 And valor < 100000 Then

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                'ValorFinal = value.Substring(0, 2) & value.Substring(3, 6).Replace(separador, ".")
                ValorFinal = value.Replace(separador, ".")

            End If

        ElseIf valor >= 100000 And valor < 1000000 Then

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                'ValorFinal = value.Substring(0, 3) & value.Substring(4, 6).Replace(separador, ".")
                ValorFinal = value.Replace(separador, ".")

            End If

        Else
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                'ValorFinal = value.Substring(0, 3) & value.Substring(4, 6).Replace(separador, ".")
                ValorFinal = value.Replace(separador, ".")

            End If
        End If


        Return ValorFinal
    End Function

    Public Function SubstituirSeparador1Dig(ByVal valor As Single) As String
        Dim value As String
        Dim ValorFinal As String

        value = Convert.ToString(FormatNumber(valor, 1, TriState.True, TriState.False, TriState.False))
        Dim separador As String = Globalization.CultureInfo.CurrentCulture.NumberFormat.NumberDecimalSeparator

        ValorFinal = value

        If valor < 1000 Then

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                ValorFinal = value.Replace(separador, ".")
            End If

        ElseIf valor >= 1000 And valor < 10000 Then

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                'ValorFinal = value.Substring(0, 1) & value.Substring(2, 5).Replace(separador, ".")
                ValorFinal = value.Replace(separador, ".")

            End If

        ElseIf valor >= 10000 And valor < 100000 Then

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                'ValorFinal = value.Substring(0, 2) & value.Substring(3, 5).Replace(separador, ".")
                ValorFinal = value.Replace(separador, ".")

            End If

        ElseIf valor >= 100000 And valor < 1000000 Then

            'Se o separador decimal utilizado for ".", retorna o próprio valor já em forma de texto
            If separador = "." Then
                ValorFinal = value
            Else
                'Se o separador decimal utilizado for qualquer outro, substitui o mesmo por ponto 
                'ValorFinal = value.Substring(0, 3) & value.Substring(4, 5).Replace(separador, ".")
                ValorFinal = value.Replace(separador, ".")

            End If

        End If


        Return ValorFinal
    End Function

End Module
