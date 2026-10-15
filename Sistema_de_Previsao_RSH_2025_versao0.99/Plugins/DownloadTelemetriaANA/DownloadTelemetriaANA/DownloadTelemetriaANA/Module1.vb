Imports System
Imports System.IO
Imports System.Net
Imports System.Text
Imports System.Xml

Module Module1
    'Este programa faz downlod dos dados do webservice da ANA listados no arquivo "Estações.txt" e salva na raiz do programa

    'Variáveis
    'define a data inicial e final de download, para a aquisição dos dados
    Private dataO As Date
    Private dataE As Date
    Public Const AI = 1990 '1880 


    Sub Main()
        Console.WriteLine("Download de dados do Web-service da ANA")

        'Chama a rotina que faz o download
        Download_Manual_WebService2()
    End Sub

    Sub Download_Manual_WebService2() 'Faz download do webservice da ANA



        Console.WriteLine("Entre com a data inicial no formado dd/mm/aaaa hh:00:00")
        dataO = Console.ReadLine()

        Console.WriteLine("Entre com a data final no formado dd/mm/aaaa hh:00:00")
        dataE = Console.ReadLine()



        'número de dias dentro do intervalo
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, dataO, dataE) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        'Cria os valores de dia, mes e ano da maneira correta
        Dim dddi, dddf, mmmi, mmmf As String
        dddi = dataO.Day
        If dddi.ToString.Length < 2 Then
            dddi = "0" & dddi
        End If

        mmmi = dataO.Month
        If mmmi.ToString.Length < 2 Then
            mmmi = "0" & mmmi
        End If

        dddf = dataE.Day
        If dddf.ToString.Length < 2 Then
            dddf = "0" & dddf
        End If

        mmmf = dataE.Month
        If mmmf.ToString.Length < 2 Then
            mmmf = "0" & mmmf
        End If
        'Pegou os valores com o tamanho certo

        'começa o loop de download


        'muda o nome da tela
        Console.WriteLine("Fazendo download...")

        ' ----- Obtem local onde irá salvar o arquivo
        Dim pasta_exec As String
        pasta_exec = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location())

        'Abre o arquivo de estações
        FileOpen(1, pasta_exec & "\" & "Estações.txt", OpenMode.Input)

        'muda o nome da tela
        'Console.WriteLine("Abriu arquivo de entrada...")

        'Variável de leitura
        Dim linha As String

        Dim total_postos As Integer

        total_postos = 0

        'Faz o download dos arquivos individualmente
        Do Until EOF(1)
            linha = LineInput(1)

            Dim arquivoFonte As String 'arquivo que será baixado
            total_postos = total_postos + 1

            'informa o endereço do arquivo de cotas
            arquivoFonte = "http://telemetriaws1.ana.gov.br/serviceana.asmx/" & "DadosHidrometeorologicos?codEstacao=" & linha & "&dataInicio=" & dddi & "/" & mmmi & "/" & dataO.Year & "&dataFim=" & dddf & "/" & mmmf & "/" & dataE.Year

            'muda o nome da tela
            Console.WriteLine("Faz a requisição...")

            'inicia o download via FTP no servidor FTP -voce pode usar seu usuario e senha ou 
            'efetuar o download via FTP anônimo
            Requisitar2(arquivoFonte, linha)
            Converter_Requisitado2(linha)
        Loop

        FileClose()

        Console.WriteLine("Download dos arquivos finalizado.")
        Console.ReadLine()

    End Sub


    Sub Requisitar2(ByVal arquivoFonte As String, ByVal CódigoEstação As String) 'Baixa o XML para diretório determinado

        'Pega a url do arquivo que deve ser baixado no webservice
        Dim url As String
        url = arquivoFonte
        Console.WriteLine(url)


        'Cria o nome do arquivo que deve ser baixado
        Dim ArquivoGerado As String
        ArquivoGerado = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\" & CódigoEstação & ".xml"


        Dim request As WebRequest = _
              WebRequest.Create(url)
        ' If required by the server, set the credentials.  
        request.Credentials = CredentialCache.DefaultCredentials
        ' Get the response.  
        Dim response As WebResponse = request.GetResponse()
        ' Display the status.  
        Console.WriteLine(CType(response, HttpWebResponse).StatusDescription)
        ' Get the stream containing content returned by the server.  
        Dim dataStream As Stream = response.GetResponseStream()
        ' Open the stream using a StreamReader for easy access.  
        Dim reader As New StreamReader(dataStream)
        ' Read the content.  
        Dim responseFromServer As String = reader.ReadToEnd()
        ' Display the content.  
        Console.WriteLine(responseFromServer)
        ' Clean up the streams and the response.  
        reader.Close()
        response.Close()


        ''Cria uma requisição com as credenciais do site
        'Dim fWr As HttpWebRequest = DirectCast(WebRequest.Create(url), HttpWebRequest)
        '' fWr.Credentials = New NetworkCredential("supervisao@uhelajeado.com.br", "<REDACTED>")

        ''Resposta do http fornecido
        'Dim response As HttpWebResponse = fWr.GetResponse

        ''Lê o script retornado com função de obtenção dos dados no database do site
        'Dim inStream As New IO.StreamReader(response.GetResponseStream(), System.Text.Encoding.Default, True)

        ''Cria uma variável do tipo string onde salva todas as informações do script do site
        'Dim l As String = inStream.ReadToEnd

        'Abre o arquivo e salva todo o script dentro dele
        FileOpen(10, ArquivoGerado, OpenMode.Output)
        Print(10, responseFromServer)
        FileClose(10)

        'Terminou a rotina
    End Sub

    Sub Converter_Requisitado(ByVal CódigoEstação As String)
        'Cria o nome do arquivo que foi baixado
        Dim ArquivoGerado As String
        ArquivoGerado = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\" & CódigoEstação & ".xml"

        'Variáveis necessárias para dimensionamento
        Dim hh As Integer 'hora
        Dim mes As Integer 'meses
        Dim dia As Integer 'dias
        Dim year As Integer 'anos

        'Define os valores das variáveis
        hh = 24
        mes = 12
        dia = 31
        year = 2050

        'Dimensiona as variáveis de vazão, chuva e nível que vão conter os resultados
        Dim chuva_hor(hh, dia, mes, Ano(year)) As Double
        Dim vazao_hor(hh, dia, mes, Ano(year)) As Double
        Dim nivel_hor(hh, dia, mes, Ano(year)) As Double

        'Zera as variáveis 
        For aaaa = AI To 2050
            For mm = 1 To 12
                For dd = 1 To 31
                    For h As Integer = 0 To 23
                        chuva_hor(h, dd, mm, Ano(aaaa)) = -1.0
                        vazao_hor(h, dd, mm, Ano(aaaa)) = -1.0
                        nivel_hor(h, dd, mm, Ano(aaaa)) = -1.0
                    Next
                Next
            Next
        Next
        'Zerou as variáveis


        'Dim reader As XmlTextReader = New XmlTextReader(ArquivoGerado)

        Dim settings As New XmlReaderSettings()
        settings.DtdProcessing = DtdProcessing.Parse
        Dim reader As XmlReader = XmlReader.Create(ArquivoGerado, settings)
        reader.MoveToContent()

        Dim data, valor, vazao, chuva, nivel As Integer

        Do While (reader.Read())
            Select Case reader.NodeType
                Case XmlNodeType.Element 'Exibir o início do elemento.

                    'Verifica se o elemento lido é a data, dado de chuva, vazão, ou nível
                    If reader.Name = "DataHora" Then
                        data = 1
                        valor = 0
                        vazao = 0
                        chuva = 0
                        nivel = 0
                    ElseIf reader.Name = "Chuva" Then
                        data = 0
                        valor = 0
                        vazao = 0
                        chuva = 1
                        nivel = 0
                    ElseIf reader.Name = "Vazao" Then
                        data = 0
                        valor = 0
                        vazao = 1
                        chuva = 0
                        nivel = 0
                    ElseIf reader.Name = "Nivel" Then
                        data = 0
                        valor = 0
                        vazao = 0
                        chuva = 0
                        nivel = 1
                    End If

                Case XmlNodeType.Text 'Exibir o texto em cada elemento.
                    If data = 1 Then
                        data = 0
                        valor = 0
                        valor = 0
                        vazao = 0
                        chuva = 0
                        nivel = 0
                        hh = reader.Value.Substring(11, 2)
                        mes = reader.Value.Substring(5, 2)
                        dia = reader.Value.Substring(8, 2)
                        year = reader.Value.Substring(0, 4)
                    ElseIf valor = 1 Then
                        'data = 0
                        'valor = 0
                        'PrintLine(1, reader.Value)
                    ElseIf vazao = 1 Then
                        data = 0
                        valor = 0
                        valor = 0
                        vazao = 0
                        chuva = 0
                        nivel = 0
                        vazao_hor(hh, dia, mes, Ano(year)) = reader.Value
                        If vazao_hor(hh, dia, mes, Ano(year)).ToString.Length < 2 Then
                            ' MsgBox("Vazão = " & reader.Value)
                            vazao_hor(hh, dia, mes, Ano(year)) = -1.0
                        Else
                            vazao_hor(hh, dia, mes, Ano(year)) = AjustarDecimal(reader.Value)
                        End If
                    ElseIf chuva = 1 Then
                        data = 0
                        valor = 0
                        valor = 0
                        vazao = 0
                        chuva = 0
                        nivel = 0
                        chuva_hor(hh, dia, mes, Ano(year)) = reader.Value
                        If chuva_hor(hh, dia, mes, Ano(year)) > 1000 Then
                            'MsgBox("Chuva = " & reader.Value)
                            chuva_hor(hh, dia, mes, Ano(year)) = -1.0
                        Else
                            chuva_hor(hh, dia, mes, Ano(year)) = AjustarDecimal(reader.Value)
                        End If
                    ElseIf nivel = 1 Then
                        data = 0
                        valor = 0
                        valor = 0
                        vazao = 0
                        chuva = 0
                        nivel = 0
                        nivel_hor(hh, dia, mes, Ano(year)) = reader.Value
                        If nivel_hor(hh, dia, mes, Ano(year)).ToString.Length < 2 Then
                            'MsgBox("Nível = " & reader.Value)
                            nivel_hor(hh, dia, mes, Ano(year)) = -1.0
                        Else
                            nivel_hor(hh, dia, mes, Ano(year)) = AjustarDecimal(reader.Value)
                        End If
                    End If
                    'MsgBox(reader.Value)
                Case XmlNodeType.EndElement 'Exibir o fim do elemento.
                    'MsgBox("</" + reader.Name)
                    'MsgBox(">")
            End Select
        Loop

        'Fecha o reader
        reader.Close()

        '------ CRIA O ARQUIVO DE NÍVEL

        'define a data inicial e final de download, para a aquisição dos dados
        Dim datai As Date = dataO.Day & "/" & dataO.Month & "/" & dataO.Year & " " & dataO.Hour & ":00:00"
        Dim dataf As Date = dataE.Day & "/" & dataE.Month & "/" & dataE.Year & " " & dataE.Hour & ":00:00"

        'número de dias dentro do intervalo
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, datai, dataf) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        Dim datacomeço As Date 'define a data inicial da interpolação
        datacomeço = datai
        'data inicial igual ao primeiro dia do intervalo

        'Abre o arquivo novo no local que corresponde ao lugar onde devem ser salvos os dados de nível
        FileOpen(2, "N" & CódigoEstação & ".txt", OpenMode.Output)

        'escreve o arquivo de níveis, começando do tempo 1 até o tempo final
        For aux = 1 To tmaxx
            'Adiciona uma hora no contador somente se tivermos passado do tempo 1
            If aux <> 1 Then
                datacomeço = datacomeço.AddHours(1)
            End If

            'Acerta o arquivo para escrever conforme os espaçamentos
            If datacomeço.Day < 10 Then
                Print(2, "    " & "0" & datacomeço.Day)
            ElseIf datacomeço.Day >= 10 Then
                Print(2, "    " & datacomeço.Day)
            End If
            If datacomeço.Month < 10 Then
                Print(2, "    " & "0" & datacomeço.Month)
            ElseIf datacomeço.Month >= 10 Then
                Print(2, "    " & datacomeço.Month)

            End If

            Print(2, "  " & datacomeço.Year)

            If datacomeço.Hour.ToString.Length = 1 Then
                Print(2, "    " & "0" & datacomeço.Hour.ToString)
            ElseIf datacomeço.Hour.ToString.Length = 2 Then
                Print(2, "    " & datacomeço.Hour.ToString)
            End If

            'Escreveu a data

            'Agora vai escrever os números


            'If FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar
            'End If

            If SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                'não diminui dois espaços pois não tem o separador de milhar
            Else
                Print(2, "       " & "-1.000000")
            End If


            PrintLine(2)

        Next

        FileClose(2)
        '---- Terminou de gerar arquivo de níveis



        '------ CRIA O ARQUIVO DE VAZÃO

        'número de dias dentro do intervalo
        tmaxx = (DateDiff(DateInterval.Hour, datai, dataf) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        datacomeço = datai
        'data inicial igual ao primeiro dia do intervalo

        'Abre o arquivo novo no local que corresponde ao lugar onde devem ser salvos os dados de nível
        FileOpen(2, "Q" & CódigoEstação & ".txt", OpenMode.Output)

        'escreve o arquivo de níveis, começando do tempo 1 até o tempo final
        For aux = 1 To tmaxx
            'Adiciona uma hora no contador somente se tivermos passado do tempo 1
            If aux <> 1 Then
                datacomeço = datacomeço.AddHours(1)
            End If

            'Acerta o arquivo para escrever conforme os espaçamentos
            If datacomeço.Day < 10 Then
                Print(2, "    " & "0" & datacomeço.Day)
            ElseIf datacomeço.Day >= 10 Then
                Print(2, "    " & datacomeço.Day)
            End If
            If datacomeço.Month < 10 Then
                Print(2, "    " & "0" & datacomeço.Month)
            ElseIf datacomeço.Month >= 10 Then
                Print(2, "    " & datacomeço.Month)

            End If

            Print(2, "  " & datacomeço.Year)

            If datacomeço.Hour.ToString.Length = 1 Then
                Print(2, "    " & "0" & datacomeço.Hour.ToString)
            ElseIf datacomeço.Hour.ToString.Length = 2 Then
                Print(2, "    " & datacomeço.Hour.ToString)
            End If

            'Escreveu a data

            'Agora vai escrever os números

            'If FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar

            'End If

            If SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                'não diminui dois espaços pois não tem o separador de milhar

            End If

            PrintLine(2)

        Next

        FileClose(2)
        '---- Terminou de gerar arquivo de vazão

        '------ CRIA O ARQUIVO DE CHUVA

        'número de dias dentro do intervalo
        tmaxx = (DateDiff(DateInterval.Hour, datai, dataf) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        datacomeço = datai
        'data inicial igual ao primeiro dia do intervalo

        'Abre o arquivo novo no local que corresponde ao lugar onde devem ser salvos os dados de nível
        FileOpen(2, "P" & CódigoEstação & ".txt", OpenMode.Output)

        'escreve o arquivo de níveis, começando do tempo 1 até o tempo final
        For aux = 1 To tmaxx
            'Adiciona uma hora no contador somente se tivermos passado do tempo 1
            If aux <> 1 Then
                datacomeço = datacomeço.AddHours(1)
            End If

            'Acerta o arquivo para escrever conforme os espaçamentos
            If datacomeço.Day < 10 Then
                Print(2, "    " & "0" & datacomeço.Day)
            ElseIf datacomeço.Day >= 10 Then
                Print(2, "    " & datacomeço.Day)
            End If
            If datacomeço.Month < 10 Then
                Print(2, "    " & "0" & datacomeço.Month)
            ElseIf datacomeço.Month >= 10 Then
                Print(2, "    " & datacomeço.Month)

            End If

            Print(2, "  " & datacomeço.Year)

            If datacomeço.Hour.ToString.Length = 1 Then
                Print(2, "    " & "0" & datacomeço.Hour.ToString)
            ElseIf datacomeço.Hour.ToString.Length = 2 Then
                Print(2, "    " & datacomeço.Hour.ToString)
            End If

            'Escreveu a data

            'Agora vai escrever os números

            'MsgBox(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))

            'If FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar

            'End If

            If SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                'não diminui dois espaços pois não tem o separador de milhar
            End If

            PrintLine(2)

        Next

        FileClose(2)
        '---- Terminou de gerar arquivo de chuva




    End Sub

    Sub Converter_Requisitado2(ByVal CódigoEstação As String)
        'Cria o nome do arquivo que foi baixado
        Dim ArquivoGerado As String
        ArquivoGerado = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly.Location()) & "\" & CódigoEstação & ".xml"

        'Variáveis necessárias para dimensionamento
        Dim hh As Integer 'hora
        Dim mes As Integer 'meses
        Dim dia As Integer 'dias
        Dim year As Integer 'anos

        'Define os valores das variáveis
        hh = 24
        mes = 12
        dia = 31
        year = 2050

        'Dimensiona as variáveis de vazão, chuva e nível que vão conter os resultados
        Dim chuva_hor(hh, dia, mes, Ano(year)) As Double
        Dim vazao_hor(hh, dia, mes, Ano(year)) As Double
        Dim nivel_hor(hh, dia, mes, Ano(year)) As Double

        'Zera as variáveis 
        For aaaa = AI To 2050
            For mm = 1 To 12
                For dd = 1 To 31
                    For h As Integer = 0 To 23
                        chuva_hor(h, dd, mm, Ano(aaaa)) = -1.0
                        vazao_hor(h, dd, mm, Ano(aaaa)) = -1.0
                        nivel_hor(h, dd, mm, Ano(aaaa)) = -1.0
                    Next
                Next
            Next
        Next
        'Zerou as variáveis


        'Dim reader As XmlTextReader = New XmlTextReader(ArquivoGerado)

        Dim settings As New XmlReaderSettings()
        settings.DtdProcessing = DtdProcessing.Parse
        Dim reader As XmlReader = XmlReader.Create(ArquivoGerado, settings)
        reader.MoveToContent()

        Dim data, valor, vazao, chuva, nivel As Integer

        Try
            Do While (reader.Read())
                Select Case reader.NodeType
                    Case XmlNodeType.Element 'Exibir o início do elemento.

                        'Verifica se o elemento lido é a data, dado de chuva, vazão, ou nível
                        If reader.Name = "DataHora" Then
                            data = 1
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0
                        ElseIf reader.Name = "Chuva" Then
                            data = 0
                            valor = 0
                            vazao = 0
                            chuva = 1
                            nivel = 0
                        ElseIf reader.Name = "Vazao" Then
                            data = 0
                            valor = 0
                            vazao = 1
                            chuva = 0
                            nivel = 0
                        ElseIf reader.Name = "Nivel" Then
                            data = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 1
                        End If

                    Case XmlNodeType.Text 'Exibir o texto em cada elemento.
                        If data = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0
                            hh = reader.Value.Substring(11, 2)
                            mes = reader.Value.Substring(5, 2)
                            dia = reader.Value.Substring(8, 2)
                            year = reader.Value.Substring(0, 4)
                        ElseIf valor = 1 Then
                            'data = 0
                            'valor = 0
                            'PrintLine(1, reader.Value)
                        ElseIf vazao = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0
                            vazao_hor(hh, dia, mes, Ano(year)) = reader.Value
                            If vazao_hor(hh, dia, mes, Ano(year)).ToString.Length < 2 Then
                                ' MsgBox("Vazão = " & reader.Value)
                                vazao_hor(hh, dia, mes, Ano(year)) = -1.0
                            Else
                                vazao_hor(hh, dia, mes, Ano(year)) = AjustarDecimal(reader.Value)
                            End If
                        ElseIf chuva = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0


                            If chuva_hor(hh, dia, mes, Ano(year)) < 0 Then
                                chuva_hor(hh, dia, mes, Ano(year)) = 0
                            End If

                            'MsgBox("Chuva = " & reader.Value)

                            'chuva_hor(hh, dia, mes, Ano(year)) = 0 'reader.Value
                            If chuva_hor(hh, dia, mes, Ano(year)) > 1000 Then
                                'MsgBox("Chuva = " & reader.Value)
                                chuva_hor(hh, dia, mes, Ano(year)) = -1.0
                            Else
                                chuva_hor(hh, dia, mes, Ano(year)) += AjustarDecimal(reader.Value)
                            End If
                        ElseIf nivel = 1 Then
                            data = 0
                            valor = 0
                            valor = 0
                            vazao = 0
                            chuva = 0
                            nivel = 0
                            nivel_hor(hh, dia, mes, Ano(year)) = reader.Value
                            If nivel_hor(hh, dia, mes, Ano(year)).ToString.Length < 2 Then
                                'MsgBox("Nível = " & reader.Value)
                                nivel_hor(hh, dia, mes, Ano(year)) = -1.0
                            Else
                                nivel_hor(hh, dia, mes, Ano(year)) = AjustarDecimal(reader.Value)
                            End If
                        End If
                        'MsgBox(reader.Value)
                    Case XmlNodeType.EndElement 'Exibir o fim do elemento.
                        'MsgBox("</" + reader.Name)
                        'MsgBox(">")
                End Select
            Loop
        Catch ex As Exception

        End Try
        

        'Fecha o reader
        reader.Close()

        '------ CRIA O ARQUIVO DE NÍVEL

        'define a data inicial e final de download, para a aquisição dos dados
        Dim datai As Date = dataO.Day & "/" & dataO.Month & "/" & dataO.Year & " " & dataO.Hour & ":00:00"
        Dim dataf As Date = dataE.Day & "/" & dataE.Month & "/" & dataE.Year & " " & dataE.Hour & ":00:00"

        'número de dias dentro do intervalo
        Dim tmaxx As Integer
        tmaxx = (DateDiff(DateInterval.Hour, datai, dataf) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        Dim datacomeço As Date 'define a data inicial da interpolação
        datacomeço = datai
        'data inicial igual ao primeiro dia do intervalo

        'Abre o arquivo novo no local que corresponde ao lugar onde devem ser salvos os dados de nível
        FileOpen(2, "N" & CódigoEstação & ".txt", OpenMode.Output)

        'escreve o arquivo de níveis, começando do tempo 1 até o tempo final
        For aux = 1 To tmaxx
            'Adiciona uma hora no contador somente se tivermos passado do tempo 1
            If aux <> 1 Then
                datacomeço = datacomeço.AddHours(1)
            End If

            'Acerta o arquivo para escrever conforme os espaçamentos
            If datacomeço.Day < 10 Then
                Print(2, "    " & "0" & datacomeço.Day)
            ElseIf datacomeço.Day >= 10 Then
                Print(2, "    " & datacomeço.Day)
            End If
            If datacomeço.Month < 10 Then
                Print(2, "    " & "0" & datacomeço.Month)
            ElseIf datacomeço.Month >= 10 Then
                Print(2, "    " & datacomeço.Month)

            End If

            Print(2, "  " & datacomeço.Year)

            If datacomeço.Hour.ToString.Length = 1 Then
                Print(2, "    " & "0" & datacomeço.Hour.ToString)
            ElseIf datacomeço.Hour.ToString.Length = 2 Then
                Print(2, "    " & datacomeço.Hour.ToString)
            End If

            'Escreveu a data

            'Agora vai escrever os números


            'If FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar
            'End If

            If SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(nivel_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                'não diminui dois espaços pois não tem o separador de milhar
            Else
                Print(2, "       " & "-1.000000")
            End If


            PrintLine(2)

        Next

        FileClose(2)
        '---- Terminou de gerar arquivo de níveis



        '------ CRIA O ARQUIVO DE VAZÃO

        'número de dias dentro do intervalo
        tmaxx = (DateDiff(DateInterval.Hour, datai, dataf) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        datacomeço = datai
        'data inicial igual ao primeiro dia do intervalo

        'Abre o arquivo novo no local que corresponde ao lugar onde devem ser salvos os dados de nível
        FileOpen(2, "Q" & CódigoEstação & ".txt", OpenMode.Output)

        'escreve o arquivo de níveis, começando do tempo 1 até o tempo final
        For aux = 1 To tmaxx
            'Adiciona uma hora no contador somente se tivermos passado do tempo 1
            If aux <> 1 Then
                datacomeço = datacomeço.AddHours(1)
            End If

            'Acerta o arquivo para escrever conforme os espaçamentos
            If datacomeço.Day < 10 Then
                Print(2, "    " & "0" & datacomeço.Day)
            ElseIf datacomeço.Day >= 10 Then
                Print(2, "    " & datacomeço.Day)
            End If
            If datacomeço.Month < 10 Then
                Print(2, "    " & "0" & datacomeço.Month)
            ElseIf datacomeço.Month >= 10 Then
                Print(2, "    " & datacomeço.Month)

            End If

            Print(2, "  " & datacomeço.Year)

            If datacomeço.Hour.ToString.Length = 1 Then
                Print(2, "    " & "0" & datacomeço.Hour.ToString)
            ElseIf datacomeço.Hour.ToString.Length = 2 Then
                Print(2, "    " & datacomeço.Hour.ToString)
            End If

            'Escreveu a data

            'Agora vai escrever os números

            'If FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar

            'End If

            If SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                Print(2, "     " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                Print(2, "    " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                Print(2, "   " & SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                'não diminui dois espaços pois não tem o separador de milhar
            ElseIf SubstituirSeparador(vazao_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000000 
                Print(2, "       " & "-1.000000")
            Else
                Print(2, "       " & "-1.000000")
            End If

            PrintLine(2)

        Next

        FileClose(2)
        '---- Terminou de gerar arquivo de vazão

        '------ CRIA O ARQUIVO DE CHUVA

        'número de dias dentro do intervalo
        tmaxx = (DateDiff(DateInterval.Hour, datai, dataf) + 1)
        'tmaxx = dif. de tempo + 1, significa o número de dias

        datacomeço = datai
        'data inicial igual ao primeiro dia do intervalo

        'Abre o arquivo novo no local que corresponde ao lugar onde devem ser salvos os dados de nível
        FileOpen(2, "P" & CódigoEstação & ".txt", OpenMode.Output)

        'escreve o arquivo de níveis, começando do tempo 1 até o tempo final
        For aux = 1 To tmaxx
            'Adiciona uma hora no contador somente se tivermos passado do tempo 1
            If aux <> 1 Then
                datacomeço = datacomeço.AddHours(1)
            End If

            'Acerta o arquivo para escrever conforme os espaçamentos
            If datacomeço.Day < 10 Then
                Print(2, "    " & "0" & datacomeço.Day)
            ElseIf datacomeço.Day >= 10 Then
                Print(2, "    " & datacomeço.Day)
            End If
            If datacomeço.Month < 10 Then
                Print(2, "    " & "0" & datacomeço.Month)
            ElseIf datacomeço.Month >= 10 Then
                Print(2, "    " & datacomeço.Month)

            End If

            Print(2, "  " & datacomeço.Year)

            If datacomeço.Hour.ToString.Length = 1 Then
                Print(2, "    " & "0" & datacomeço.Hour.ToString)
            ElseIf datacomeço.Hour.ToString.Length = 2 Then
                Print(2, "    " & datacomeço.Hour.ToString)
            End If

            'Escreveu a data

            'Agora vai escrever os números

            'MsgBox(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))

            'If FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 0 Then
            '    Print(2, "       " & "-1.000000")
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 0 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10 Then
            '    Print(2, "        " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100 Then
            '    Print(2, "       " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000 Then
            '    Print(2, "      " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 1000 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 10000 Then
            '    Print(2, "     " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 1) & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(2, 10))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 10000 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 100000 Then
            '    Print(2, "    " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 2) & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(3, 10))
            'ElseIf FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) >= 100000 And FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6) < 1000000 Then
            '    Print(2, "   " & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(0, 3) & FormatNumber(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)), 6).Substring(4, 10))
            '    'não diminui dois espaços pois não tem o separador de milhar

            'End If
            'If CódigoEstação = "70840080" Then
            '    Console.WriteLine(datacomeço & " " & chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year)))
            '    Console.ReadLine()
            'End If

            If SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 0 Then
                Print(2, "       " & "-1.000000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 0 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10 Then
                Print(2, "        " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100 Then
                Print(2, "       " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000 Then
                Print(2, "      " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
            ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 Then
                Print(2, "       " & "-1.000000")
            Else
                Print(2, "       " & "-1.000000")
                ''ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 1000 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 10000 Then
                ''    Print(2, "     " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ''ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 10000 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 100000 Then
                ''    Print(2, "    " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ''ElseIf SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) >= 100000 And SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) < 1000000 Then
                ''    Print(2, "   " & SubstituirSeparador(chuva_hor(datacomeço.Hour, datacomeço.Day, datacomeço.Month, Ano(datacomeço.Year))) & "0000")
                ''    'não diminui dois espaços pois não tem o separador de milhar
            End If

            PrintLine(2)

        Next

        FileClose(2)
        '---- Terminou de gerar arquivo de chuva




    End Sub

    Public Function Ano(ByVal aaaa As Integer) As Integer
        Ano = aaaa - AI
    End Function

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

        value = Convert.ToString(FormatNumber(valor, 2, Microsoft.VisualBasic.TriState.True, Microsoft.VisualBasic.TriState.False, Microsoft.VisualBasic.TriState.False))
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

        End If


        Return ValorFinal
    End Function

    Public Function SubstituirSeparador1Dig(ByVal valor As Single) As String
        Dim value As String
        Dim ValorFinal As String

        value = Convert.ToString(FormatNumber(valor, 1, Microsoft.VisualBasic.TriState.True, Microsoft.VisualBasic.TriState.False, Microsoft.VisualBasic.TriState.False))
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
