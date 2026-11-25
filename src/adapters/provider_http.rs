use std::time::Duration;
use std::{error::Error, io::Read};

use chrono::{DateTime, NaiveDate, Utc};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_RANGE, LOCATION, RANGE};
use reqwest::{StatusCode, Url};
use serde::Deserialize;

use crate::store::ForecastProvider;

const ANA: &str = "https://telemetriaws1.ana.gov.br/serviceana.asmx/DadosHidrometeorologicos";
const ECMWF: &str = "https://data.ecmwf.int/forecasts";
const NOAA: &str = "https://nomads.ncep.noaa.gov/cgi-bin/filter_gfs_0p25.pl";
const MAX_GRIB: u64 = 32 * 1024 * 1024;

#[derive(Debug)]
pub(crate) enum ProviderError {
    Unavailable(String),
    Invalid(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) | Self::Invalid(message) => f.write_str(message),
        }
    }
}

pub(crate) fn cycle_hours(provider: ForecastProvider) -> i64 {
    match provider {
        ForecastProvider::Ecmwf => 12,
        ForecastProvider::Noaa => 6,
    }
}

pub(crate) fn forecast_leads(provider: ForecastProvider) -> Vec<i64> {
    match provider {
        ForecastProvider::Ecmwf => (0..=144).step_by(3).chain((150..=360).step_by(6)).collect(),
        ForecastProvider::Noaa => (3..=240)
            .step_by(3)
            .chain((252..=384).step_by(12))
            .collect(),
    }
}

pub(crate) struct ProviderClient {
    client: Client,
    ana: String,
    ecmwf: String,
    noaa: String,
}

#[derive(Deserialize)]
struct IndexField<'a> {
    param: &'a str,
    levtype: &'a str,
    #[serde(rename = "_offset")]
    offset: u64,
    #[serde(rename = "_length")]
    length: u64,
}

impl ProviderClient {
    pub(crate) fn new() -> Result<Self, ProviderError> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(120))
                .user_agent("mgb-ops")
                .build()
                .map_err(|e| ProviderError::Invalid(e.to_string()))?,
            ana: ANA.into(),
            ecmwf: ECMWF.into(),
            noaa: NOAA.into(),
        })
    }

    #[cfg(test)]
    pub(crate) fn test_ecmwf(mut self, endpoint: String) -> Self {
        self.ecmwf = endpoint;
        self
    }

    #[cfg(test)]
    pub(crate) fn test_ana(mut self, endpoint: String) -> Self {
        self.ana = endpoint;
        self
    }

    pub(crate) fn ana(
        &self,
        station: &str,
        start: NaiveDate,
        end: NaiveDate,
        timeout: Duration,
    ) -> Result<Vec<u8>, ProviderError> {
        let query = [
            ("codEstacao", station.to_string()),
            ("dataInicio", start.format("%d/%m/%Y").to_string()),
            ("dataFim", end.format("%d/%m/%Y").to_string()),
        ];
        read_response(
            send(self.client.get(&self.ana).query(&query).timeout(timeout))?,
            MAX_GRIB,
        )
    }

    pub(crate) fn forecast(
        &self,
        provider: ForecastProvider,
        cycle: DateTime<Utc>,
        lead: i64,
        bbox: [f64; 4],
    ) -> Result<Vec<u8>, ProviderError> {
        let bytes = match provider {
            ForecastProvider::Ecmwf => self.ecmwf(cycle, lead)?,
            ForecastProvider::Noaa => self.noaa(cycle, lead, bbox)?,
        };
        if bytes.len() < 20 || &bytes[..4] != b"GRIB" || bytes[7] != 2 {
            return Err(ProviderError::Invalid(format!(
                "lead {lead}: expected GRIB2 response"
            )));
        }
        Ok(bytes)
    }

    fn ecmwf(&self, cycle: DateTime<Utc>, lead: i64) -> Result<Vec<u8>, ProviderError> {
        let base = format!(
            "{}/{}/{}z/ifs/0p25/oper/{}-{lead}h-oper-fc",
            self.ecmwf,
            cycle.format("%Y%m%d"),
            cycle.format("%H"),
            cycle.format("%Y%m%d%H0000")
        );
        let index = read_response(
            send(self.client.get(format!("{base}.index")))?,
            4 * 1024 * 1024,
        )?;
        let (start, length) = precipitation_range(&index)?;
        let end = start + length - 1;
        let response = send(
            self.client
                .get(format!("{base}.grib2"))
                .header(ACCEPT_ENCODING, "identity")
                .header(RANGE, format!("bytes={start}-{end}")),
        )?;
        let expected_range = format!("bytes {start}-{end}/");
        let range = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|v| v.to_str().ok());
        if response.status() != StatusCode::PARTIAL_CONTENT
            || range
                .and_then(|v| v.strip_prefix(&expected_range))
                .and_then(|total| total.parse::<u64>().ok())
                .is_none_or(|total| total <= end)
        {
            return Err(ProviderError::Invalid(
                "ECMWF response has an unexpected Content-Range".into(),
            ));
        }
        let bytes = read_response(response, length)?;
        if bytes.len() as u64 != length {
            return Err(ProviderError::Invalid(
                "ECMWF returned an incomplete byte range".into(),
            ));
        }
        Ok(bytes)
    }

    fn noaa(
        &self,
        cycle: DateTime<Utc>,
        lead: i64,
        bbox: [f64; 4],
    ) -> Result<Vec<u8>, ProviderError> {
        let [west, south, east, north] = bbox;
        let query = [
            (
                "file",
                format!("gfs.t{}z.pgrb2.0p25.f{lead:03}", cycle.format("%H")),
            ),
            ("dir", format!("/gfs.{}/atmos", cycle.format("%Y%m%d/%H"))),
            ("var_APCP", "on".into()),
            ("lev_surface", "on".into()),
            ("subregion", String::new()),
            ("leftlon", west.to_string()),
            ("rightlon", east.to_string()),
            ("bottomlat", south.to_string()),
            ("toplat", north.to_string()),
        ];
        read_response(send(self.client.get(&self.noaa).query(&query))?, MAX_GRIB)
    }
}

fn precipitation_range(index: &[u8]) -> Result<(u64, u64), ProviderError> {
    let text = std::str::from_utf8(index)
        .map_err(|_| ProviderError::Invalid("ECMWF index must be UTF-8".into()))?;
    let mut matched = None;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let field: IndexField<'_> = serde_json::from_str(line)
            .map_err(|e| ProviderError::Invalid(format!("invalid ECMWF index entry: {e}")))?;
        if field.param == "tp" && field.levtype == "sfc" {
            if matched.is_some()
                || !(20..=MAX_GRIB).contains(&field.length)
                || field.offset.checked_add(field.length).is_none()
            {
                return Err(ProviderError::Invalid(
                    "invalid or duplicate ECMWF precipitation range".into(),
                ));
            }
            matched = Some((field.offset, field.length));
        }
    }
    matched.ok_or_else(|| ProviderError::Invalid("ECMWF index has no surface tp field".into()))
}

fn send(request: RequestBuilder) -> Result<Response, ProviderError> {
    let response = request.send().map_err(|error| {
        let error = error.without_url();
        let mut detail = error.to_string();
        let mut cause = error.source();
        while let Some(error) = cause {
            detail.push_str(&format!(": {error}"));
            cause = error.source();
        }
        ProviderError::Unavailable(detail)
    })?;
    check_status(response)
}

fn check_status(response: Response) -> Result<Response, ProviderError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::GONE | StatusCode::REQUEST_TIMEOUT
    ) || status.is_server_error()
    {
        return Err(ProviderError::Unavailable(format!(
            "provider HTTP {status}"
        )));
    }
    let retry = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .map(|v| format!(" (Retry-After {v})"))
        .unwrap_or_default();
    Err(ProviderError::Invalid(format!(
        "provider HTTP {status}{retry}"
    )))
}

fn read_response(response: Response, limit: u64) -> Result<Vec<u8>, ProviderError> {
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ProviderError::Unavailable(format!("provider response transfer: {e}")))?;
    if bytes.len() as u64 > limit {
        return Err(ProviderError::Invalid(
            "provider response exceeds expected size".into(),
        ));
    }
    Ok(bytes)
}

/// Standalone requests do not follow redirects automatically. IMERG handles
/// archive redirects separately from requests carrying a BNDMET API key.
pub(crate) struct SourceClient(Client);

impl SourceClient {
    pub(crate) fn new() -> Result<Self, ProviderError> {
        Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent("mgb-ops")
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map(Self)
            .map_err(|e| ProviderError::Invalid(e.to_string()))
    }

    pub(crate) fn bndmet(
        &self,
        key: &reqwest::header::HeaderValue,
        station: &str,
        product: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<u8>, ProviderError> {
        let url =
            format!("https://api-bndmet.decea.mil.br/v1/estacoes/{station}/fenomenos/{product}");
        let query = [
            ("dataInicio", start.to_string()),
            ("dataFinal", end.to_string()),
        ];
        read_response(
            send(
                self.0
                    .get(url)
                    .query(&query)
                    .header("x-api-key", key.clone())
                    .header("accept", "application/json"),
            )?,
            8 * 1024 * 1024,
        )
    }

    pub(crate) fn ons(&self, month: NaiveDate) -> Result<Vec<u8>, ProviderError> {
        let url = format!(
            "https://ons-aws-prod-opendata.s3.amazonaws.com/dataset/dados_hidrologicos_ho/DADOS_HIDROLOGICOS_HO_{}.csv",
            month.format("%Y_%m")
        );
        read_response(send(self.0.get(url))?, 64 * 1024 * 1024)
    }

    pub(crate) fn imerg_catalog(&self, temporal: &str) -> Result<Vec<u8>, ProviderError> {
        let query = [
            ("collection_concept_id", "C2723758340-GES_DISC"),
            ("page_size", "2000"),
            ("temporal", temporal),
        ];
        read_response(
            send(
                self.0
                    .get("https://cmr.earthdata.nasa.gov/search/granules.json")
                    .query(&query),
            )?,
            16 * 1024 * 1024,
        )
    }

    pub(crate) fn imerg_granule(
        &self,
        url: &Url,
        token: &reqwest::header::HeaderValue,
    ) -> Result<Vec<u8>, ProviderError> {
        download_imerg(url, |target, send_token| {
            let mut request = self.0.get(target.clone());
            if send_token {
                request = request.header(AUTHORIZATION, token.clone());
            }
            request
                .send()
                .map_err(|error| ProviderError::Unavailable(error.without_url().to_string()))
        })
    }
}

const IMERG_ARCHIVE: &str = "data.gesdisc.earthdata.nasa.gov";
const IMERG_CDN: &str = "d2b3c3wh8s6en5.cloudfront.net";

fn imerg_host(url: &Url) -> Option<&str> {
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    url.host_str()
}

fn download_imerg(
    source: &Url,
    mut get: impl FnMut(&Url, bool) -> Result<Response, ProviderError>,
) -> Result<Vec<u8>, ProviderError> {
    if imerg_host(source) != Some(IMERG_ARCHIVE) {
        return Err(ProviderError::Invalid(
            "unexpected IMERG archive URL".into(),
        ));
    }
    let mut target = source.clone();
    for _ in 0..=5 {
        let response = get(&target, target.origin() == source.origin())?;
        if !matches!(
            response.status(),
            StatusCode::MOVED_PERMANENTLY
                | StatusCode::FOUND
                | StatusCode::SEE_OTHER
                | StatusCode::TEMPORARY_REDIRECT
                | StatusCode::PERMANENT_REDIRECT
        ) {
            return read_response(check_status(response)?, 64 * 1024 * 1024);
        }
        target = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|location| target.join(location).ok())
            .ok_or_else(|| ProviderError::Invalid("invalid IMERG redirect location".into()))?;
        match imerg_host(&target) {
            Some(IMERG_ARCHIVE | IMERG_CDN) => (),
            Some("urs.earthdata.nasa.gov") => return Err(ProviderError::Invalid(
                "GES DISC redirected to Earthdata Login; check the user token and GES DISC access"
                    .into(),
            )),
            _ => {
                return Err(ProviderError::Invalid(
                    "unexpected IMERG redirect destination".into(),
                ));
            }
        }
    }
    Err(ProviderError::Invalid("too many IMERG redirects".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    #[test]
    fn imerg_follows_archive_redirect_without_forwarding_credentials() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            for (path, authorized, response) in [
                ("/granule", true, format!("303 See Other\r\nLocation: https://{IMERG_CDN}/granule?signature=test\r\nContent-Length: 0\r\n\r\n")),
                ("/granule?signature=test", false, "200 OK\r\nContent-Length: 4\r\n\r\nHDF5".into()),
                ("/unknown", true, "302 Found\r\nLocation: https://unrelated.example/granule\r\nContent-Length: 0\r\n\r\n".into()),
            ] {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let lines: Vec<_> = BufReader::new(&socket).lines().map(Result::unwrap)
                    .take_while(|line| !line.is_empty()).collect();
                let expected = format!("GET {path} HTTP/1.1");
                assert_eq!(lines[0], expected);
                assert_eq!(lines.iter().any(|line| line.eq_ignore_ascii_case("authorization: Bearer sample")), authorized);
                write!(socket, "HTTP/1.1 {response}").unwrap();
            }
        });
        let client = SourceClient::new().unwrap();
        let get = |target: &Url, send_token| {
            let mut local = Url::parse(&endpoint).unwrap();
            local.set_path(target.path());
            local.set_query(target.query());
            let mut request = client.0.get(local);
            if send_token {
                request = request.header(AUTHORIZATION, "Bearer sample");
            }
            request
                .send()
                .map_err(|e| ProviderError::Unavailable(e.to_string()))
        };
        let source = Url::parse(&format!("https://{IMERG_ARCHIVE}/granule")).unwrap();
        assert_eq!(download_imerg(&source, get).unwrap(), b"HDF5");
        let untrusted = source.join("/unknown").unwrap();
        assert_eq!(
            download_imerg(&untrusted, get).unwrap_err().to_string(),
            "unexpected IMERG redirect destination"
        );
        worker.join().unwrap();
    }

    #[test]
    fn provider_protocols_and_unavailable_responses() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let mut grib = [0; 20];
        grib[..4].copy_from_slice(b"GRIB");
        grib[7] = 2;
        let serve = std::thread::spawn(move || {
            for index in 0..5 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(&socket);
                let mut headers = String::new();
                loop {
                    let mut line = String::new();
                    assert!(reader.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                    headers.push_str(&line);
                }
                let (status, extra, body) = match index {
                    0 => {
                        assert!(headers.starts_with("GET /ana?codEstacao=123&dataInicio=20%2F07%2F2026&dataFim=21%2F07%2F2026 "));
                        ("200 OK", String::new(), b"<DataTable/>".to_vec())
                    }
                    1 => {
                        assert!(headers.starts_with("GET /ecmwf/20260720/12z/ifs/0p25/oper/20260720120000-360h-oper-fc.index "));
                        let body = format!(
                            r#"{{"param":"tp","levtype":"sfc","_offset":12,"_length":{}}}"#,
                            grib.len()
                        );
                        ("200 OK", String::new(), body.into_bytes())
                    }
                    2 => {
                        assert!(headers.starts_with("GET /ecmwf/20260720/12z/ifs/0p25/oper/20260720120000-360h-oper-fc.grib2 "));
                        let end = grib.len() + 11;
                        let expected_range = format!("range: bytes=12-{end}\r\n");
                        assert!(headers.to_ascii_lowercase().contains(&expected_range));
                        (
                            "206 Partial Content",
                            format!("Content-Range: bytes 12-{end}/{}\r\n", end + 9),
                            grib.to_vec(),
                        )
                    }
                    3 => {
                        assert!(headers.starts_with("GET /noaa?file=gfs.t12z.pgrb2.0p25.f384&"));
                        for query in [
                            "dir=%2Fgfs.20260720%2F12%2Fatmos",
                            "var_APCP=on",
                            "lev_surface=on",
                            "subregion=",
                            "leftlon=-52",
                            "rightlon=-51",
                            "bottomlat=-30",
                            "toplat=-29",
                        ] {
                            assert!(headers.contains(query), "{query}");
                        }
                        ("200 OK", String::new(), b"<html>not GRIB2</html>".to_vec())
                    }
                    4 => ("404 Not Found", String::new(), Vec::new()),
                    _ => unreachable!(),
                };
                write!(
                    socket,
                    "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                socket.write_all(&body).unwrap();
            }
        });
        let mut client = ProviderClient::new().unwrap();
        client.ana = format!("{endpoint}/ana");
        client.ecmwf = format!("{endpoint}/ecmwf");
        client.noaa = format!("{endpoint}/noaa");
        let cycle = "2026-07-20T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let day = cycle.date_naive();
        let bbox = [-52., -30., -51., -29.];
        let fetched = client
            .ana("123", day, day.succ_opt().unwrap(), Duration::from_secs(5))
            .unwrap();
        assert_eq!(fetched, b"<DataTable/>");
        assert_eq!(
            client
                .forecast(ForecastProvider::Ecmwf, cycle, 360, bbox)
                .unwrap(),
            grib
        );
        let bad_payload = matches!(
            client.forecast(ForecastProvider::Noaa, cycle, 384, bbox),
            Err(ProviderError::Invalid(_))
        );
        assert!(bad_payload);
        let unavailable = matches!(
            client.forecast(ForecastProvider::Noaa, cycle, 384, bbox),
            Err(ProviderError::Unavailable(_))
        );
        assert!(unavailable);
        serve.join().unwrap();
    }

    #[test]
    fn published_leads_match_legacy_products() {
        let ecmwf = forecast_leads(ForecastProvider::Ecmwf);
        let noaa = forecast_leads(ForecastProvider::Noaa);
        assert_eq!(
            (ecmwf.len(), ecmwf[0], ecmwf[48], ecmwf[49], ecmwf[84]),
            (85, 0, 144, 150, 360)
        );
        assert_eq!(
            (noaa.len(), noaa[0], noaa[79], noaa[80], noaa[91]),
            (92, 3, 240, 252, 384)
        );
        assert!(
            precipitation_range(
                br#"{"param":"tp","levtype":"sfc","_offset":18446744073709551614,"_length":32}"#
            )
            .is_err()
        );
    }
}
