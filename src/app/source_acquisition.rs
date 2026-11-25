use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Datelike, Days, Months, NaiveDate, TimeDelta, Timelike, Utc};
use chrono_tz::Tz;
use reqwest::header::HeaderValue;
use serde::Serialize;

use super::OperationControl;
use crate::adapters::{
    BndmetProduct, ProviderError, SourceClient, hourly_bndmet_rain, hourly_bndmet_wind,
    imerg_catalog, imerg_hour_depth, observation_csv, parse_bndmet, parse_ons, read_imerg_granule,
};
use crate::store::{
    GridSeries, IntervalAxis, Provenance, TimeInterval, WindObservations, digest_file,
    sync_directory, write_grid_series, write_new_atomic,
};

#[derive(Clone, Debug)]
pub struct SourceAcquisitionRequest {
    pub source: AcquisitionSource,
    pub output: PathBuf,
    pub time_zone: Tz,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum AcquisitionSource {
    Bndmet {
        first_date: NaiveDate,
        last_date: NaiveDate,
        stations: Vec<String>,
        wind: bool,
    },
    OnsHourly {
        first_date: NaiveDate,
        last_date: NaiveDate,
        reservoirs: Vec<String>,
    },
    ImergEarly {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bbox: [f64; 4],
    },
}

#[derive(Debug, Default, Serialize)]
pub struct SourceAcquisitionReport {
    pub originals: Vec<PathBuf>,
    pub normalized: Vec<PathBuf>,
    pub issues: Vec<String>,
    pub metadata: PathBuf,
}

fn codes(input: &[String]) -> Result<BTreeSet<String>, String> {
    let output = input
        .iter()
        .map(|s| s.trim().to_ascii_uppercase())
        .collect::<BTreeSet<_>>();
    if output.is_empty()
        || output.len() != input.len()
        || output.iter().any(|code| {
            code.is_empty()
                || code.len() > 24
                || !code.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
    {
        return Err(
            "supply distinct nonempty source IDs containing letters, digits or hyphens".into(),
        );
    }
    Ok(output)
}

fn check_dates(first: NaiveDate, last: NaiveDate) -> Result<(), String> {
    if first > last || !(1..=9999).contains(&first.year()) || !(1..=9999).contains(&last.year()) {
        Err("dates must be in ascending order within years 0001–9999".into())
    } else {
        Ok(())
    }
}

fn secret(name: &str, prefix: &str) -> Result<HeaderValue, String> {
    let value = std::env::var(name).map_err(|_| format!("Set {name} to enable acquisition"))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("Set {name} to enable acquisition"));
    }
    let mut header = HeaderValue::from_str(&format!("{prefix}{value}"))
        .map_err(|_| format!("{name} is not a usable HTTP credential"))?;
    header.set_sensitive(true);
    Ok(header)
}

struct Attempt<'a> {
    output: &'a Path,
    zone: Tz,
    control: &'a OperationControl,
    client: SourceClient,
    report: SourceAcquisitionReport,
}

impl<'a> Attempt<'a> {
    fn new(
        request: &'a SourceAcquisitionRequest,
        control: &'a OperationControl,
    ) -> Result<Self, String> {
        control.check().map_err(|e| e.to_string())?;
        let client = SourceClient::new().map_err(|e| e.to_string())?;
        fs::create_dir(&request.output).map_err(|e| {
            format!(
                "create new output directory {}: {e}",
                request.output.display()
            )
        })?;
        fs::create_dir(request.output.join("raw")).map_err(|e| e.to_string())?;
        Ok(Self {
            output: &request.output,
            zone: request.time_zone,
            control,
            client,
            report: SourceAcquisitionReport::default(),
        })
    }

    fn fetch(
        &self,
        mut request: impl FnMut() -> Result<Vec<u8>, ProviderError>,
    ) -> Result<Vec<u8>, String> {
        let mut retry = 0;
        loop {
            self.control.check().map_err(|e| e.to_string())?;
            match request() {
                Err(ProviderError::Unavailable(_)) if retry < 2 => {
                    retry += 1;
                    for _ in 0..10 {
                        self.control.check().map_err(|e| e.to_string())?;
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                result => return result.map_err(|e| e.to_string()),
            }
        }
    }

    fn original(&mut self, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        let path = self.output.join("raw").join(name);
        write_new_atomic(&path, bytes)?;
        self.report.originals.push(path.clone());
        Ok(path)
    }

    fn normalized(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.control.check().map_err(|e| e.to_string())?;
        let path = self.output.join(name);
        write_new_atomic(&path, bytes)?;
        self.report.normalized.push(path);
        Ok(())
    }

    fn bndmet(
        &mut self,
        key: &HeaderValue,
        first: NaiveDate,
        last: NaiveDate,
        stations: BTreeSet<String>,
        wind: bool,
    ) -> Result<(), String> {
        let products: &[_] = if wind {
            &[BndmetProduct::Speed, BndmetProduct::Direction]
        } else {
            &[BndmetProduct::Rain]
        };
        for station in stations {
            let mut readings = [BTreeMap::new(), BTreeMap::new()];
            let mut start = first;
            loop {
                self.control.check().map_err(|e| e.to_string())?;
                let end = start
                    .checked_add_days(Days::new(29))
                    .map_or(last, |date| date.min(last));
                for (index, product) in products.iter().copied().enumerate() {
                    let code = product.code();
                    let data = self.fetch(|| self.client.bndmet(key, &station, code, start, end));
                    let result = match data {
                        Ok(bytes) => {
                            self.original(
                                &format!("{station}-{code}-{start}--{end}.json"),
                                &bytes,
                            )?;
                            parse_bndmet(&bytes, &station, product, start, end)
                        }
                        Err(error) => Err(error),
                    };
                    match result {
                        Ok(values) => readings[index].extend(values),
                        Err(error) => self
                            .report
                            .issues
                            .push(format!("BNDMET {station} {code} {start}..{end}: {error}")),
                    }
                }
                if end == last {
                    break;
                }
                start = end.succ_opt().ok_or("BNDMET date overflow")?;
            }
            let result = if wind {
                self.save_wind(&station, &readings[0], &readings[1])
            } else {
                let [rain, _] = readings;
                self.save_rain(&station, rain)
            };
            if let Err(error) = result {
                self.report
                    .issues
                    .push(format!("BNDMET {station}: {error}"));
            }
        }
        Ok(())
    }

    fn save_wind(
        &mut self,
        station: &str,
        speed: &BTreeMap<DateTime<Utc>, Option<f64>>,
        direction: &BTreeMap<DateTime<Utc>, Option<f64>>,
    ) -> Result<(), String> {
        let samples = hourly_bndmet_wind(station, speed, direction, self.zone)?;
        if !samples.iter().any(|sample| sample.wind.is_some()) {
            return Err("no complete wind observations in the requested range".into());
        }
        let wind = WindObservations {
            schema_version: 1,
            time_zone: self.zone.name().into(),
            samples,
        };
        wind.validate().map_err(|e| e.to_string())?;
        self.normalized(
            &format!("wind-{station}.json"),
            &serde_json::to_vec_pretty(&wind).map_err(|e| e.to_string())?,
        )
    }

    fn save_rain(
        &mut self,
        station: &str,
        readings: BTreeMap<DateTime<Utc>, Option<f64>>,
    ) -> Result<(), String> {
        let rows = hourly_bndmet_rain(readings, self.zone)?;
        if !rows.iter().any(|(_, value)| value.is_some()) {
            return Err("no rainfall observations in the requested range".into());
        }
        let bytes = observation_csv(
            rows.into_iter()
                .map(|(at, value)| ("inmet", station, at, "rain", value)),
        )?;
        self.normalized(&format!("rain-{station}.csv"), &bytes)
    }

    fn ons(
        &mut self,
        first: NaiveDate,
        last: NaiveDate,
        reservoirs: BTreeSet<String>,
    ) -> Result<(), String> {
        let mut month = first.with_day(1).unwrap();
        let last_month = last.with_day(1).unwrap();
        let mut selected = BTreeMap::new();
        let mut conflicted = BTreeSet::new();
        loop {
            self.control.check().map_err(|e| e.to_string())?;
            let result = match self.fetch(|| self.client.ons(month)) {
                Ok(bytes) => {
                    self.original(
                        &format!("DADOS_HIDROLOGICOS_HO_{}.csv", month.format("%Y_%m")),
                        &bytes,
                    )?;
                    parse_ons(&bytes, &reservoirs, first, last, self.zone)
                }
                Err(error) => Err(error),
            };
            match result {
                Ok(data) => {
                    if data.non_hourly > 0 {
                        self.report.issues.push(format!(
                            "ONS {}: skipped {} unexpected non-hourly labels",
                            month.format("%Y-%m"),
                            data.non_hourly
                        ));
                    }
                    for (id, count) in data.invalid_flows {
                        self.report.issues.push(format!(
                            "ONS {id} {}: marked {count} invalid flow measurements missing",
                            month.format("%Y-%m")
                        ));
                    }
                    for (id, hours) in data.reservoirs {
                        for hour in hours {
                            let key = (id.clone(), hour.local_hour);
                            let values = (hour.inflow, hour.outflow);
                            if selected
                                .insert(key.clone(), values)
                                .is_some_and(|prior| prior != values)
                            {
                                conflicted.insert(key);
                            }
                        }
                    }
                }
                Err(error) => self
                    .report
                    .issues
                    .push(format!("ONS {}: {error}", month.format("%Y-%m"))),
            }
            if month == last_month {
                break;
            }
            month = month
                .checked_add_months(Months::new(1))
                .ok_or("ONS month overflow")?;
        }
        for (id, at) in &conflicted {
            selected.remove(&(id.clone(), *at));
            self.report
                .issues
                .push(format!("ONS {id}: conflicting readings at {at}; omitted"));
        }
        for id in reservoirs {
            let rows: Vec<_> = selected.iter().filter(|((key, _), _)| key == &id).collect();
            if !rows
                .iter()
                .any(|(_, (inflow, outflow))| inflow.is_some() || outflow.is_some())
            {
                self.report.issues.push(format!(
                    "ONS {id}: no flow observations in the requested range"
                ));
                continue;
            }
            let names = [format!("{id}:inflow"), format!("{id}:outflow")];
            let bytes =
                observation_csv(rows.into_iter().flat_map(|((_, at), (inflow, outflow))| {
                    [
                        ("ons", names[0].as_str(), *at, "flow", *inflow),
                        ("ons", names[1].as_str(), *at, "flow", *outflow),
                    ]
                }))?;
            self.normalized(&format!("flow-{id}.csv"), &bytes)?;
        }
        Ok(())
    }

    fn imerg(
        &mut self,
        key: &HeaderValue,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bbox: [f64; 4],
        timeline: IntervalAxis,
    ) -> Result<(), String> {
        let temporal = format!(
            "{},{}",
            start.to_rfc3339(),
            (end - TimeDelta::milliseconds(1)).to_rfc3339()
        );
        let catalog = self.fetch(|| self.client.imerg_catalog(&temporal))?;
        self.original("cmr-granules.json", &catalog)?;
        let granules = imerg_catalog(&catalog, start, end)?;
        let mut grid = None;
        let mut values = Vec::new();
        let mut valid = Vec::new();
        let mut sources = Vec::new();
        let mut at = start;
        while at < end {
            let mut fields = Vec::with_capacity(2);
            for half in [at, at + TimeDelta::minutes(30)] {
                let granule = &granules[&half];
                let bytes = self
                    .fetch(|| self.client.imerg_granule(&granule.url, key))
                    .map_err(|e| format!("IMERG {}: {e}", granule.id))?;
                let path = self.original(&granule.id, &bytes)?;
                sources.push((granule.id.clone(), digest_file(&path)?.0));
                fields.push(
                    read_imerg_granule(bytes, bbox)
                        .map_err(|e| format!("IMERG {}: {e}", granule.id))?,
                );
            }
            if grid.as_ref().is_some_and(|g| g != &fields[0].grid) {
                return Err("IMERG grid changed across hours".into());
            }
            for value in imerg_hour_depth(&fields[0], &fields[1])? {
                valid.push(value.is_some());
                values.push(value.unwrap_or(f64::NAN));
            }
            grid.get_or_insert_with(|| fields[0].grid.clone());
            at += TimeDelta::hours(1);
        }
        let series = GridSeries {
            timeline,
            grid: grid.ok_or("no IMERG fields")?,
            values,
            valid,
            provenance: Provenance {
                source_ids: sources.iter().map(|(id, _)| id.clone()).collect(),
                processing: serde_json::json!({
                    "product": "GPM_3IMERGHHE.07",
                    "half_hour_rate_units": "mm/hr",
                    "hour_depth": "sum of both half-hour rates multiplied by 0.5 hours",
                    "missing": "missing if either half hour is missing at the cell",
                    "granule_sha256": sources,
                }),
            },
        };
        self.control.check().map_err(|e| e.to_string())?;
        let staged = tempfile::Builder::new()
            .prefix(".imerg-")
            .tempdir_in(self.output)
            .map_err(|e| e.to_string())?;
        let source = staged.path().join("imerg.nc");
        write_grid_series(&source, &series).map_err(|e| e.to_string())?;
        fs::File::open(&source)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        self.control.check().map_err(|e| e.to_string())?;
        let output = self.output.join("imerg.nc");
        fs::hard_link(source, &output).map_err(|e| e.to_string())?;
        sync_directory(self.output)?;
        self.report.normalized.push(output);
        Ok(())
    }
}

fn imerg_timeline(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    bbox: [f64; 4],
    zone: Tz,
) -> Result<IntervalAxis, String> {
    let [w, s, e, n] = bbox;
    if !bbox.iter().all(|v| v.is_finite())
        || w < -180.0
        || e > 180.0
        || s < -90.0
        || n > 90.0
        || w >= e
        || s >= n
    {
        return Err("IMERG bounds must be west south east north within geographic limits".into());
    }
    let duration = end - start;
    if duration <= TimeDelta::zero()
        || duration > TimeDelta::days(31)
        || [start, end]
            .iter()
            .any(|time| time.minute() != 0 || time.second() != 0 || time.nanosecond() != 0)
    {
        return Err("use an ascending range of UTC hours spanning at most 31 days".into());
    }
    if start.with_timezone(&zone).minute() != 0 {
        return Err("IMERG UTC hours must align with local hours in the output timezone".into());
    }
    let hours = duration.num_hours();
    let estimated_cells = ((e - w) * 10.0).ceil() + 1.0;
    let estimated_rows = ((n - s) * 10.0).ceil() + 1.0;
    if estimated_cells * estimated_rows * hours as f64 > 12_000_000.0 {
        return Err("IMERG output is too large; reduce bounds or split the range".into());
    }
    let bounds = (0..hours)
        .map(|offset| {
            let at = start + TimeDelta::hours(offset);
            TimeInterval {
                start: at.with_timezone(&zone).naive_local(),
                end: (at + TimeDelta::hours(1))
                    .with_timezone(&zone)
                    .naive_local(),
            }
        })
        .collect();
    let axis = IntervalAxis {
        time_zone: zone.name().into(),
        bounds,
    };
    axis.validate().map_err(|e| e.to_string())?;
    Ok(axis)
}

/// Acquire and normalize standalone inputs into a new directory.
pub fn acquire_source(
    request: &SourceAcquisitionRequest,
    control: &OperationControl,
) -> Result<SourceAcquisitionReport, String> {
    let attempt = match &request.source {
        AcquisitionSource::Bndmet {
            first_date,
            last_date,
            stations,
            wind,
        } => {
            check_dates(*first_date, *last_date)?;
            let stations = codes(stations)?;
            let key = secret("INMET_API_KEY", "")?;
            let mut attempt = Attempt::new(request, control)?;
            attempt.bndmet(&key, *first_date, *last_date, stations, *wind)?;
            attempt
        }
        AcquisitionSource::OnsHourly {
            first_date,
            last_date,
            reservoirs,
        } => {
            check_dates(*first_date, *last_date)?;
            let reservoirs = codes(reservoirs)?;
            let mut attempt = Attempt::new(request, control)?;
            attempt.ons(*first_date, *last_date, reservoirs)?;
            attempt
        }
        AcquisitionSource::ImergEarly { start, end, bbox } => {
            let timeline = imerg_timeline(*start, *end, *bbox, request.time_zone)?;
            let key = secret("EARTHDATA_TOKEN", "Bearer ")?;
            let mut attempt = Attempt::new(request, control)?;
            attempt.imerg(&key, *start, *end, *bbox, timeline)?;
            attempt
        }
    };
    control.check().map_err(|e| e.to_string())?;
    let mut report = attempt.report;
    report.metadata = request.output.join("acquisition.json");
    let relative_paths = |paths: &[PathBuf]| {
        paths
            .iter()
            .map(|path| path.strip_prefix(&request.output).unwrap().to_path_buf())
            .collect::<Vec<_>>()
    };
    let document = serde_json::json!({
        "schema_version": 1,
        "time_zone": request.time_zone.name(),
        "source": request.source,
        "originals": relative_paths(&report.originals),
        "normalized": relative_paths(&report.normalized),
        "issues": report.issues,
    });
    write_new_atomic(
        &report.metadata,
        &serde_json::to_vec_pretty(&document).map_err(|e| e.to_string())?,
    )?;
    Ok(report)
}
