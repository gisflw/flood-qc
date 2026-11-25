use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Days, NaiveDate, TimeZone, Timelike, Utc};
use chrono_tz::{America::Sao_Paulo, Tz};

use crate::adapters::{
    AnaAggregation, AnaReading, AnaStation, ProviderClient, aggregate_ana, ana_csv,
    ana_station_code, parse_ana_xml, read_ana_csv,
};
use crate::store::{ObservationStore, create_workspace_dir, write_new_atomic};

use super::{
    AnaAcquisitionReport, AnaDocument, AnaStationOutcome, DownloadEvent, OperationControl,
};

fn prepare(workspace: &Path, zone: Tz) -> Result<(ObservationStore, Vec<AnaStation>), String> {
    let database = super::super::observation_assets(workspace);
    let store = ObservationStore::open_writable(&database).map_err(|error| error.to_string())?;
    let db_zone = store.time_zone().map_err(|e| e.to_string())?;
    if db_zone != zone.name() {
        return Err("ANA observation database timezone differs from acquisition settings".into());
    }
    let mut stations = Vec::new();
    for (station, observed_variables) in store.station_inventory().map_err(|e| e.to_string())? {
        if station.provider_code == "ana" {
            let variables = observed_variables
                .iter()
                .map(|value| crate::adapters::AnaVariable::parse(value))
                .collect::<Result<BTreeSet<_>, _>>()?;
            if !variables.is_empty() {
                stations.push(AnaStation { station, variables });
            }
        }
    }
    if stations.is_empty() {
        return Err("Import the ANA station inventory before acquisition".into());
    }
    Ok((store, stations))
}

fn date_range(reference: DateTime<Tz>, days: u32) -> Result<(NaiveDate, NaiveDate), String> {
    let end = reference.with_timezone(&Sao_Paulo).date_naive();
    let begin = end
        .checked_sub_days(Days::new(u64::from(days - 1)))
        .ok_or("ANA request starts outside the supported calendar")?;
    if begin < NaiveDate::from_ymd_opt(1, 1, 1).unwrap() {
        return Err("ANA request starts before year 1".into());
    }
    Ok((begin, end))
}

fn request_dates(
    store: &ObservationStore,
    station: &AnaStation,
    zone: Tz,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<NaiveDate>, String> {
    if station.variables.is_empty() {
        return Ok(Vec::new());
    }
    let _ = zone;
    let variables = station
        .variables
        .iter()
        .map(|v| v.code().to_owned())
        .collect::<Vec<_>>();
    let completed = store
        .ana_coverage(&station.station.station_id, &variables)
        .map_err(|e| e.to_string())?;
    Ok(start
        .iter_days()
        .take_while(|day| *day <= end)
        .filter(|day| !completed.contains(day))
        .collect())
}

fn windows(dates: Vec<NaiveDate>, limit: u32) -> Vec<(NaiveDate, NaiveDate)> {
    let mut windows: Vec<(NaiveDate, NaiveDate)> = Vec::new();
    for day in dates {
        if let Some((start, end)) = windows.last_mut()
            && end.succ_opt() == Some(day)
            && (day - *start).num_days() < i64::from(limit)
        {
            *end = day;
        } else {
            windows.push((day, day));
        }
    }
    windows
}

fn import_readings(
    store: &mut ObservationStore,
    reference: DateTime<Tz>,
    station: &AnaStation,
    readings: Vec<AnaReading>,
) -> Result<usize, String> {
    let hourly = aggregate_ana(
        readings,
        station,
        reference.timezone(),
        AnaAggregation::default(),
    )?;
    let values: Vec<_> = hourly
        .into_iter()
        .map(|(key, values)| (key.code(), values))
        .collect();
    store
        .upsert_ana_raw(
            &station.station.station_id,
            reference.naive_local(),
            &values,
        )
        .map_err(|e| e.to_string())
}

struct Fetch<'a> {
    cutoff: DateTime<Utc>,
    partial: std::cell::RefCell<crate::store::AnaPartialCoverage>,
    config: &'a AnaDocument,
    reference: DateTime<Tz>,
    dates: (NaiveDate, NaiveDate),
    directory: PathBuf,
    client: &'a ProviderClient,
    control: &'a OperationControl,
}

#[derive(Default)]
struct StationProgress {
    dates: Option<(NaiveDate, NaiveDate)>,
    rows: usize,
}

impl Fetch<'_> {
    fn station(
        &self,
        store: &mut ObservationStore,
        station: &AnaStation,
        outcome: &mut AnaStationOutcome,
        progress: &mut StationProgress,
    ) -> Result<(), String> {
        let (first, last) = self.dates;
        let dates = if self.control.force_acquisition() {
            first.iter_days().take_while(|day| *day <= last).collect()
        } else {
            request_dates(store, station, self.reference.timezone(), first, last)?
        };
        progress.dates = dates.first().copied().zip(dates.last().copied());
        outcome.requested_days = dates.len();
        outcome.skipped_days = self.config.request_days as usize - dates.len();
        let variables = station
            .variables
            .iter()
            .map(|v| v.code().to_owned())
            .collect::<Vec<_>>();
        let dates = if self.control.force_acquisition() {
            dates
        } else {
            dates
                .into_iter()
                .filter_map(|day| {
                    match self.partial.borrow().covers(
                        &station.station.station_id,
                        &variables,
                        day,
                        self.reference.timestamp(),
                    ) {
                        Ok(true) => None,
                        Ok(false) => Some(Ok(day)),
                        Err(e) => Some(Err(e)),
                    }
                })
                .collect::<Result<Vec<_>, String>>()?
        };
        outcome.requested_days = dates.len();
        outcome.skipped_days = self.config.request_days as usize - dates.len();
        let completed_dates = dates.clone();
        let code = &outcome.station_code;
        let station_dir = self.directory.join(code);
        fs::create_dir(&station_dir).map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        for (start, end) in windows(dates, self.config.fetch_window_days) {
            self.control.check().map_err(|e| e.to_string())?;
            let update = |rows, loading| {
                self.control.download(|| DownloadEvent::AnaWindow {
                    station: code.to_owned(),
                    start,
                    end,
                    rows,
                    loading,
                });
            };
            update(rows.len(), false);
            let body = self
                .client
                .ana(
                    code,
                    start,
                    end,
                    Duration::from_secs(self.config.timeout_seconds),
                )
                .map_err(|e| format!("{start} to {end}: {e}"))?;
            update(rows.len(), true);
            let path = station_dir.join(format!(
                "{}__{}.xml",
                start.format("%Y%m%d"),
                end.format("%Y%m%d")
            ));
            write_new_atomic(&path, &body)?;
            outcome.raw_xml.push(path);
            rows.extend(
                parse_ana_xml(&body, code, &station.variables)?
                    .into_iter()
                    .filter(|row| (start..=end).contains(&row.time.date()))
                    .filter(|row| {
                        Sao_Paulo
                            .from_local_datetime(&row.time)
                            .single()
                            .is_some_and(|time| time.to_utc() <= self.cutoff)
                    }),
            );
            progress.rows = rows.len();
            update(progress.rows, true);
        }
        self.control.check().map_err(|e| e.to_string())?;
        rows.sort_by_key(|row| row.time);
        let path = station_dir.join("observed.csv");
        let csv = ana_csv(code, &rows)?;
        write_new_atomic(&path, &csv)?;
        outcome.csv = Some(path);
        outcome.normalized_rows = rows.len();
        self.control.check().map_err(|e| e.to_string())?;
        let replay_rows = read_ana_csv(csv.as_slice())?
            .remove(code)
            .unwrap_or_default();
        outcome.changed_samples = import_readings(store, self.reference, station, replay_rows)?;
        let variables = station
            .variables
            .iter()
            .map(|v| v.code().to_owned())
            .collect::<Vec<_>>();
        let today = self.cutoff.with_timezone(&Sao_Paulo).date_naive();
        let full_days = completed_dates
            .iter()
            .copied()
            .filter(|day| *day < today)
            .collect::<Vec<_>>();
        if !full_days.is_empty() {
            store
                .mark_ana_coverage(&station.station.station_id, &variables, &full_days)
                .map_err(|e| e.to_string())?;
        }
        if completed_dates.contains(&today) {
            self.partial.borrow_mut().record(
                &station.station.station_id,
                &variables,
                today,
                self.cutoff.timestamp(),
            )?;
        }
        Ok(())
    }
}

pub(super) fn acquire(
    workspace: &Path,
    config: &AnaDocument,
    reference: DateTime<Tz>,
    client: &ProviderClient,
    control: &OperationControl,
) -> AnaAcquisitionReport {
    let mut report = AnaAcquisitionReport::default();
    let result = (|| -> Result<(), String> {
        control.check().map_err(|e| e.to_string())?;
        let dates = date_range(reference, config.request_days)?;
        let (mut store, stations) = prepare(workspace, reference.timezone())?;
        let root = Path::new("downloads/ana");
        create_workspace_dir(workspace, root).map_err(|e| e.to_string())?;
        let directory = tempfile::Builder::new()
            .prefix(&format!("{}-", reference.format("%Y%m%dT%H%M%S")))
            .tempdir_in(workspace.join(root))
            .map_err(|e| e.to_string())?
            .keep();
        let local_now = Utc::now().with_timezone(&Sao_Paulo);
        let cutoff = local_now
            .with_minute(0)
            .and_then(|v| v.with_second(0))
            .and_then(|v| v.with_nanosecond(0))
            .ok_or("invalid acquisition hour")?
            .to_utc();
        let fetch = Fetch {
            cutoff,
            partial: std::cell::RefCell::new(crate::store::AnaPartialCoverage::read(workspace)?),
            config,
            reference,
            dates,
            directory,
            client,
            control,
        };
        for (index, station) in stations.iter().enumerate() {
            if control.is_cancelled() {
                break;
            }
            let code = ana_station_code(&station.station.station_code)?.to_owned();
            control.acquisition_progress(format!("ANA {code}"), index, stations.len());
            let mut outcome = AnaStationOutcome {
                station_code: code,
                ..AnaStationOutcome::default()
            };
            let mut progress = StationProgress::default();
            outcome.error = fetch
                .station(&mut store, station, &mut outcome, &mut progress)
                .err();
            control.download(|| DownloadEvent::AnaFinished {
                station: outcome.station_code.clone(),
                dates: progress.dates,
                rows: progress.rows,
                changed_samples: outcome.changed_samples,
                error: outcome.error.clone(),
                cancelled: control.is_cancelled() && outcome.error.is_some(),
            });
            report.stations.push(outcome);
        }
        fetch.partial.borrow().write(workspace)?;
        control.acquisition_progress("ANA".into(), report.stations.len(), stations.len());
        Ok(())
    })();
    report.error = result.err();
    if let Some(error) = &report.error {
        control.download(|| DownloadEvent::AnaFailed(error.clone()));
    }
    report
}

pub(super) fn replay(
    workspace: &Path,
    reference: DateTime<Tz>,
    paths: &[PathBuf],
    control: &OperationControl,
) -> Result<AnaAcquisitionReport, String> {
    if paths.is_empty() {
        return Err("select at least one normalized ANA CSV".into());
    }
    let mut groups = BTreeMap::<String, Vec<AnaReading>>::new();
    let mut files = BTreeSet::new();
    let mut report = AnaAcquisitionReport::default();
    for path in paths {
        if control.is_cancelled() {
            return Ok(report);
        }
        let path = workspace
            .join(path)
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !files.insert(path.clone()) {
            return Err(format!("duplicate ANA CSV {}", path.display()));
        }
        let input = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (code, rows) in read_ana_csv(input).map_err(|e| format!("{}: {e}", path.display()))? {
            groups.entry(code).or_default().extend(rows);
        }
    }
    if control.is_cancelled() {
        return Ok(report);
    }
    let (mut store, stations) = prepare(workspace, reference.timezone())?;
    let total = groups.len();
    for (index, (code, rows)) in groups.into_iter().enumerate() {
        if control.is_cancelled() {
            break;
        }
        control.acquisition_progress(format!("ANA {code}"), index, total);
        let station = stations
            .iter()
            .find(|entry| ana_station_code(&entry.station.station_code) == Ok(code.as_str()));
        let mut outcome = AnaStationOutcome {
            station_code: code,
            normalized_rows: rows.len(),
            ..AnaStationOutcome::default()
        };
        match station
            .ok_or_else(|| "ANA station is outside the selected inventory".into())
            .and_then(|station| import_readings(&mut store, reference, station, rows))
        {
            Ok(changed) => outcome.changed_samples = changed,
            Err(error) => outcome.error = Some(error),
        }
        report.stations.push(outcome);
    }
    control.acquisition_progress("ANA".into(), report.stations.len(), total);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_day_cache_reuses_completed_hours_and_fetches_the_next_hour() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;
        fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let workspace = dir.path().canonicalize().unwrap();
        let mut store = ObservationStore::create(
            &super::super::super::observation_assets(&workspace),
            Sao_Paulo.name(),
        )
        .unwrap();
        let station = AnaStation {
            station: crate::store::Station {
                station_id: "ana:123".into(),
                provider_code: "ana".into(),
                station_code: "123".into(),
                name: "Test".into(),
                latitude: None,
                longitude: None,
            },
            variables: [crate::adapters::AnaVariable::Rain].into(),
        };
        store
            .import_ana_inventory(
                std::slice::from_ref(&station.station),
                &[(station.station.station_id.clone(), vec!["rain".into()])],
            )
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let body = r#"<NewDataSet><DadosHidrometereologicos><CodEstacao>123</CodEstacao><DataHora>2026-07-20T12:00:00</DataHora><Chuva>1</Chuva></DadosHidrometereologicos><DadosHidrometereologicos><CodEstacao>123</CodEstacao><DataHora>2026-07-20T13:00:00</DataHora><Chuva>2</Chuva></DadosHidrometereologicos></NewDataSet>"#;
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(&socket);
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                }
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let client = ProviderClient::new().unwrap().test_ana(endpoint);
        let day = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let options = AnaDocument {
            request_days: 1,
            fetch_window_days: 1,
            timeout_seconds: 5,
        };
        let control = OperationControl::default();
        for (hour, requested) in [(12, 1), (12, 0), (13, 1)] {
            let reference = Sao_Paulo.with_ymd_and_hms(2026, 7, 20, hour, 0, 0).unwrap();
            let output = tempfile::tempdir_in(&workspace).unwrap();
            let fetch = Fetch {
                cutoff: reference.to_utc(),
                partial: std::cell::RefCell::new(
                    crate::store::AnaPartialCoverage::read(&workspace).unwrap(),
                ),
                config: &options,
                reference,
                dates: (day, day),
                directory: output.path().into(),
                client: &client,
                control: &control,
            };
            let mut outcome = AnaStationOutcome {
                station_code: "123".into(),
                ..Default::default()
            };
            fetch
                .station(
                    &mut store,
                    &station,
                    &mut outcome,
                    &mut StationProgress::default(),
                )
                .unwrap();
            assert_eq!(outcome.requested_days, requested);
            fetch.partial.borrow().write(&workspace).unwrap();
            let times = store
                .ana_raw_times(
                    "ana:123",
                    "rain",
                    day.and_hms_opt(0, 0, 0).unwrap(),
                    day.and_hms_opt(23, 0, 0).unwrap(),
                )
                .unwrap();
            assert_eq!(times.last().unwrap().hour(), hour);
        }
        server.join().unwrap();
    }

    #[test]
    fn coverage_records_completed_requests_including_empty_days() {
        fs::create_dir_all("runs").unwrap();
        let workspace = tempfile::tempdir_in("runs").unwrap();
        let inventory = workspace.path().join("inventory.csv");
        fs::write(&inventory, "provider_code,station_code,station_name,latitude,longitude,observed_variables\nana,123,Station,0,0,\"rain,level\"\n").unwrap();
        fs::write(
            workspace.path().join("config.toml"),
            "[run]\ntime_zone='America/Sao_Paulo'\n",
        )
        .unwrap();
        super::super::import_workspace_inventory(workspace.path(), &inventory).unwrap();
        let (mut store, stations) = prepare(workspace.path(), Sao_Paulo).unwrap();
        let station = &stations[0];
        let dates: Vec<_> = NaiveDate::from_ymd_opt(2026, 7, 20)
            .unwrap()
            .iter_days()
            .take(5)
            .collect();
        let at = |index: usize| dates[index].and_hms_opt(12, 0, 0).unwrap();
        let mut samples = Vec::new();
        for index in [0, 2, 3, 4] {
            samples.push((at(index), crate::store::ObservationValue::RainMm(0.)));
        }
        let variables = [
            ("rain", samples),
            (
                "level",
                vec![
                    (at(0), crate::store::ObservationValue::LevelCm(0)),
                    (at(4), crate::store::ObservationValue::LevelCm(0)),
                ],
            ),
        ];
        store
            .upsert_ana_raw(&station.station.station_id, at(0), &variables)
            .unwrap();
        let planned = request_dates(&store, station, Sao_Paulo, dates[0], dates[4]).unwrap();
        assert_eq!(planned, dates);
        let variables = station
            .variables
            .iter()
            .map(|v| v.code().to_owned())
            .collect::<Vec<_>>();
        store
            .mark_ana_coverage(
                &station.station.station_id,
                &variables,
                &[dates[0], dates[1], dates[4]],
            )
            .unwrap();
        let planned = request_dates(&store, station, Sao_Paulo, dates[0], dates[4]).unwrap();
        assert_eq!(planned, dates[2..4]);
        assert_eq!(windows(planned, 2), [(dates[2], dates[3])]);
    }
}
