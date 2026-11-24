use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Days, NaiveDate, TimeZone};
use chrono_tz::{America::Sao_Paulo, Tz};

use crate::adapters::{
    AnaReading, AnaStation, ProviderClient, aggregate_ana, ana_csv, ana_station_code,
    parse_ana_xml, read_ana_csv, read_ana_inventory,
};
use crate::store::{ObservationStore, create_workspace_dir, write_new_atomic};

use super::{AnaAcquisitionReport, AnaDocument, AnaStationOutcome, OperationControl};

fn prepare(
    workspace: &Path,
    config: &AnaDocument,
    zone: Tz,
) -> Result<(ObservationStore, Vec<AnaStation>), String> {
    let inventory = workspace.join(&config.inventory);
    let mut stations = read_ana_inventory(
        File::open(&inventory).map_err(|e| format!("{}: {e}", inventory.display()))?,
    )?;
    let mut selected = BTreeSet::new();
    for code in &config.station_codes {
        let code = ana_station_code(code)?;
        if !selected.insert(code) || !stations.iter().any(|s| s.station.station_code == code) {
            return Err(format!("unknown or duplicate requested ANA station {code}"));
        }
    }
    let database = workspace.join(&config.database);
    let mut store = match database.try_exists() {
        Ok(true) => ObservationStore::open_writable(&database),
        Ok(false) => {
            fs::create_dir_all(
                database
                    .parent()
                    .ok_or("observation database requires a parent")?,
            )
            .map_err(|e| e.to_string())?;
            ObservationStore::create(&database, zone.name())
        }
        Err(error) => return Err(format!("{}: {error}", database.display())),
    }
    .map_err(|e| e.to_string())?;
    let db_zone = store.time_zone().map_err(|e| e.to_string())?;
    if db_zone != zone.name() {
        return Err("ANA observation database timezone differs from acquisition settings".into());
    }
    let mut existing = BTreeMap::<String, Vec<_>>::new();
    for entry in store.station_locations().map_err(|e| e.to_string())? {
        let station = entry.station;
        if station.provider_code == "ana" {
            let code = ana_station_code(&station.station_code)?.to_owned();
            existing.entry(code).or_default().push(station);
        }
    }
    for ana in &mut stations {
        if let Some(matching) = existing.get(&ana.station.station_code) {
            let [previous] = matching.as_slice() else {
                return Err(format!(
                    "ambiguous ANA station identity {}",
                    ana.station.station_code
                ));
            };
            ana.station.station_id = previous.station_id.clone();
            ana.station.station_code = previous.station_code.clone();
        }
    }
    store
        .seed_ana_stations(
            &stations
                .iter()
                .map(|entry| entry.station.clone())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| e.to_string())?;
    if !selected.is_empty() {
        stations.retain(|s| {
            ana_station_code(&s.station.station_code).is_ok_and(|code| selected.contains(code))
        });
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
    // Include adjacent local dates so the query also covers workspaces in other zones.
    let first = start
        .checked_sub_days(Days::new(2))
        .ok_or("ANA coverage start overflow")?;
    let last = end
        .checked_add_days(Days::new(2))
        .ok_or("ANA coverage end overflow")?;
    let first = first.and_hms_opt(0, 0, 0).unwrap();
    let last = last.and_hms_opt(0, 0, 0).unwrap();
    let mut counts = BTreeMap::<NaiveDate, usize>::new();
    for variable in &station.variables {
        let mut days = BTreeSet::new();
        for at in store
            .ana_raw_times(&station.station.station_id, variable.code(), first, last)
            .map_err(|e| e.to_string())?
        {
            let at = zone
                .from_local_datetime(&at)
                .single()
                .ok_or_else(|| format!("invalid local raw time {at}"))?;
            days.insert(at.with_timezone(&Sao_Paulo).date_naive());
        }
        for day in days {
            *counts.entry(day).or_default() += 1;
        }
    }
    Ok(start
        .iter_days()
        .take_while(|day| *day <= end)
        .filter(|day| *day == end || counts.get(day).copied() != Some(station.variables.len()))
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
    config: &AnaDocument,
    reference: DateTime<Tz>,
    station: &AnaStation,
    readings: Vec<AnaReading>,
) -> Result<usize, String> {
    let hourly = aggregate_ana(readings, station, reference.timezone(), config.aggregation)?;
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
    config: &'a AnaDocument,
    reference: DateTime<Tz>,
    dates: (NaiveDate, NaiveDate),
    directory: PathBuf,
    client: &'a ProviderClient,
    control: &'a OperationControl,
}

impl Fetch<'_> {
    fn station(
        &self,
        store: &mut ObservationStore,
        station: &AnaStation,
        outcome: &mut AnaStationOutcome,
    ) -> Result<(), String> {
        let (first, last) = self.dates;
        let dates = request_dates(store, station, self.reference.timezone(), first, last)?;
        outcome.requested_days = dates.len();
        outcome.skipped_days = self.config.request_days as usize - dates.len();
        let code = &outcome.station_code;
        let station_dir = self.directory.join(code);
        fs::create_dir(&station_dir).map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        for (start, end) in windows(dates, self.config.fetch_window_days) {
            self.control.check().map_err(|e| e.to_string())?;
            let body = self
                .client
                .ana(
                    code,
                    start,
                    end,
                    Duration::from_secs(self.config.timeout_seconds),
                )
                .map_err(|e| format!("{start} to {end}: {e}"))?;
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
                    .filter(|row| (start..=end).contains(&row.time.date())),
            );
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
        outcome.changed_samples =
            import_readings(store, self.config, self.reference, station, replay_rows)?;
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
        let (mut store, stations) = prepare(workspace, config, reference.timezone())?;
        let root = Path::new("data/downloads/ana");
        create_workspace_dir(workspace, root).map_err(|e| e.to_string())?;
        let directory = tempfile::Builder::new()
            .prefix(&format!("{}-", reference.format("%Y%m%dT%H%M%S")))
            .tempdir_in(workspace.join(root))
            .map_err(|e| e.to_string())?
            .keep();
        let fetch = Fetch {
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
            outcome.error = fetch.station(&mut store, station, &mut outcome).err();
            report.stations.push(outcome);
        }
        control.acquisition_progress("ANA".into(), report.stations.len(), stations.len());
        Ok(())
    })();
    report.error = result.err();
    report
}

pub(super) fn replay(
    workspace: &Path,
    config: &AnaDocument,
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
    let (mut store, stations) = prepare(workspace, config, reference.timezone())?;
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
            .and_then(|station| import_readings(&mut store, config, reference, station, rows))
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
    fn historical_coverage_requires_each_variable_and_refreshes_reference_day() {
        fs::create_dir_all("runs").unwrap();
        let workspace = tempfile::tempdir_in("runs").unwrap();
        let inventory = workspace.path().join("inventory.csv");
        fs::write(&inventory, "provider_code,station_code,station_name,latitude,longitude,observed_variables\nana,123,Station,0,0,\"rain,level\"\n").unwrap();
        let config = AnaDocument {
            inventory,
            ..Default::default()
        };
        let (mut store, stations) = prepare(workspace.path(), &config, Sao_Paulo).unwrap();
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
        assert_eq!(planned, dates[1..]);
        assert_eq!(
            windows(planned, 2),
            [(dates[1], dates[2]), (dates[3], dates[4])]
        );
    }
}
