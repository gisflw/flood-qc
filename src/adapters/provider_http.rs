use std::time::Duration;
use std::{error::Error, io::Read};

use chrono::{DateTime, NaiveDate, Utc};
use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{ACCEPT_ENCODING, CONTENT_RANGE, RANGE};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

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
