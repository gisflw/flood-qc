use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeDelta, Utc};
use chrono_tz::Tz;

use crate::adapters::{
    NormalizationRequest, ProviderClient, ProviderError, cycle_hours, forecast_leads,
    normalize_saved_forecast,
};
use crate::store::{
    ForecastAsset, ForecastAssetChoice, ForecastCatalog, ForecastProvider, GridSeries,
};

use super::{DownloadEvent, ForecastAcquisitionOutcome, ForecastDocument, OperationControl};

fn product(provider: ForecastProvider) -> (&'static str, &'static str) {
    match provider {
        ForecastProvider::Ecmwf => ("ecmwf", "ifs"),
        ForecastProvider::Noaa => ("noaa", "gfs"),
    }
}

fn candidates(
    reference: DateTime<Tz>,
    provider: ForecastProvider,
    lookback: u32,
) -> impl Iterator<Item = DateTime<Utc>> {
    let interval = cycle_hours(provider) * 3600;
    let seconds = reference.timestamp();
    let latest = seconds - seconds.rem_euclid(interval);
    (0..lookback)
        .map_while(move |index| DateTime::from_timestamp(latest - i64::from(index) * interval, 0))
}

fn complete_grid(
    grid: &GridSeries,
    request: &NormalizationRequest,
    model_bbox: [f64; 4],
    leads: &[i64],
) -> bool {
    let [west, south, east, north] = grid.grid.effective_bbox;
    let [model_west, model_south, model_east, model_north] = model_bbox;
    if grid.grid.requested_bbox != request.crop_bbox
        || grid.timeline.time_zone != request.operation_timezone.name()
        || (grid.grid.resolution_degrees - 0.25).abs() > 1e-9
        || west > model_west + 1e-9
        || south > model_south + 1e-9
        || east < model_east - 1e-9
        || north < model_north - 1e-9
    {
        return false;
    }
    let metadata = &grid.provenance.processing;
    let Some(unique) = metadata["selected_message_count"]
        .as_u64()
        .zip(metadata["duplicate_message_count"].as_u64())
        .and_then(|(selected, duplicates)| selected.checked_sub(duplicates))
    else {
        return false;
    };
    if unique != leads.len() as u64 {
        return false;
    }
    let labels = leads.iter().copied().filter(|lead| *lead > 0);
    let mut prior = request
        .cycle_utc
        .with_timezone(&request.operation_timezone)
        .naive_local();
    let mut bounds = grid.timeline.bounds.iter();
    for lead in labels {
        let Some(end) = request
            .cycle_utc
            .checked_add_signed(TimeDelta::hours(lead))
            .map(|utc| utc.with_timezone(&request.operation_timezone).naive_local())
        else {
            return false;
        };
        let Some(bound) = bounds.next() else {
            return false;
        };
        if bound.start != prior || bound.end != end {
            return false;
        }
        prior = end;
    }
    bounds.next().is_none()
}

struct Download<'a> {
    workspace: &'a Path,
    config: &'a ForecastDocument,
    bbox: [f64; 4],
    client: &'a ProviderClient,
    control: &'a OperationControl,
}

impl Download<'_> {
    fn provider(
        &self,
        catalog: &mut ForecastCatalog,
        choices: &[ForecastAssetChoice],
        reference: DateTime<Tz>,
        outcome: &mut ForecastAcquisitionOutcome,
    ) -> Result<(), String> {
        let provider = outcome.provider;
        let (code, model) = product(provider);
        let leads = forecast_leads(provider);
        let mut noaa_last = None;
        let mut last_unavailable = String::new();
        for cycle in candidates(reference, provider, self.config.lookback_cycles) {
            self.control.check().map_err(|e| e.to_string())?;
            outcome.attempted_cycles.push(cycle);
            self.control
                .download(|| DownloadEvent::ForecastCycle { provider, cycle });
            let request = NormalizationRequest {
                provider,
                model: model.into(),
                product: "fc".into(),
                cycle_utc: cycle,
                operation_timezone: reference.timezone(),
                crop_bbox: self.bbox,
            };
            let cycle_id = cycle.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            let cycle_local = cycle.with_timezone(&reference.timezone());
            let mut matching = None;
            for choice in choices.iter().filter(|choice| {
                choice.provider == code
                    && choice.model == model
                    && choice.product == "fc"
                    && choice.cycle_utc == cycle_id
            }) {
                let (asset, grid) = catalog
                    .load_native(&choice.asset_id)
                    .map_err(|e| e.to_string())?;
                if asset
                    .retained_source
                    .extension()
                    .is_some_and(|ext| ext == "grib2")
                    && complete_grid(&grid, &request, self.config.model_bbox, &leads)
                {
                    if matching.is_some() {
                        return Err(format!(
                            "multiple matching {code} forecast assets for {}",
                            cycle_local.format("%Y-%m-%d %H:%M")
                        ));
                    }
                    matching = Some((asset, grid));
                }
            }
            if let Some((asset, grid)) = matching {
                self.record(outcome, cycle, asset, &grid, true);
                return Ok(());
            }
            let retained = self
                .workspace
                .join("downloads")
                .join(code)
                .join(format!("{model}_fc_{}.grib2", cycle.format("%Y%m%dT%H%MZ")));
            if self.config.artifact_root == Path::new("assets") && retained.is_file() {
                let forecast =
                    normalize_saved_forecast(&retained, &request).map_err(|e| e.to_string())?;
                if !complete_grid(&forecast.native, &request, self.config.model_bbox, &leads) {
                    return Err(format!(
                        "{code}: retained forecast has incomplete schedule or coverage"
                    ));
                }
                self.control.check().map_err(|e| e.to_string())?;
                let asset = catalog
                    .register_native(&retained, &forecast.source, &forecast.native)
                    .map_err(|e| e.to_string())?;
                self.record(outcome, cycle, asset, &forecast.native, true);
                return Ok(());
            }
            match self.download(&request, &leads, &mut noaa_last) {
                Ok(staged) => {
                    let forecast = normalize_saved_forecast(staged.path(), &request)
                        .map_err(|e| e.to_string())?;
                    if !complete_grid(&forecast.native, &request, self.config.model_bbox, &leads) {
                        return Err(format!(
                            "{code} {}: incomplete forecast schedule or spatial coverage",
                            cycle_local.format("%Y-%m-%d %H:%M")
                        ));
                    }
                    self.control.check().map_err(|e| e.to_string())?;
                    let asset = catalog
                        .register_native(staged.path(), &forecast.source, &forecast.native)
                        .map_err(|e| e.to_string())?;
                    self.record(outcome, cycle, asset, &forecast.native, false);
                    return Ok(());
                }
                Err(ProviderError::Unavailable(message)) => last_unavailable = message,
                Err(ProviderError::Invalid(message)) => return Err(message),
            }
        }
        Err(format!("{code} lookback exhausted: {last_unavailable}"))
    }

    fn download(
        &self,
        request: &NormalizationRequest,
        leads: &[i64],
        noaa_last: &mut Option<Instant>,
    ) -> Result<tempfile::NamedTempFile, ProviderError> {
        let local_error = |e: std::io::Error| ProviderError::Invalid(e.to_string());
        let mut staged = tempfile::Builder::new()
            .prefix(".download-")
            .suffix(".grib2")
            .tempfile_in(self.workspace.join(&self.config.artifact_root))
            .map_err(local_error)?;
        let provider = request.provider;
        let (code, _) = product(provider);
        let cycle_local = request.cycle_utc.with_timezone(&request.operation_timezone);
        // Request the final lead first to detect an incomplete provider cycle quickly.
        for (index, &lead) in leads.iter().rev().enumerate() {
            self.control.download(|| DownloadEvent::ForecastLead {
                provider,
                cycle: request.cycle_utc,
                lead,
                index: index + 1,
                total: leads.len(),
            });
            self.control.acquisition_progress(
                format!(
                    "{} {} +{lead} h",
                    code.to_uppercase(),
                    cycle_local.format("%Y-%m-%d %H:%M")
                ),
                index,
                leads.len(),
            );
            if provider == ForecastProvider::Noaa {
                wait_for_noaa(*noaa_last, self.control)?;
            }
            self.control
                .check()
                .map_err(|e| ProviderError::Invalid(e.to_string()))?;
            let bytes = self
                .client
                .forecast(provider, request.cycle_utc, lead, self.bbox);
            if provider == ForecastProvider::Noaa {
                *noaa_last = Some(Instant::now());
            }
            staged.write_all(&bytes?).map_err(local_error)?;
        }
        staged.as_file().sync_all().map_err(local_error)?;
        self.control
            .acquisition_progress(code.to_uppercase(), leads.len(), leads.len());
        Ok(staged)
    }

    fn record(
        &self,
        outcome: &mut ForecastAcquisitionOutcome,
        cycle: DateTime<Utc>,
        asset: ForecastAsset,
        grid: &GridSeries,
        reused: bool,
    ) {
        outcome.cycle_utc = Some(cycle);
        outcome.reused = reused;
        outcome.asset_id = Some(asset.asset_id);
        outcome.retained_source = Some(self.workspace.join(asset.retained_source));
        outcome.native_artifact = Some(self.workspace.join(asset.native_artifact));
        outcome.valid_from = grid.timeline.bounds.first().map(|bound| bound.start);
        outcome.valid_to = grid.timeline.bounds.last().map(|bound| bound.end);
    }
}

fn wait_for_noaa(last: Option<Instant>, control: &OperationControl) -> Result<(), ProviderError> {
    if let Some(last) = last {
        let minimum_wait = Duration::from_secs(10);
        while last.elapsed() < minimum_wait {
            control
                .check()
                .map_err(|e| ProviderError::Invalid(e.to_string()))?;
            std::thread::sleep(
                Duration::from_millis(100).min(minimum_wait.saturating_sub(last.elapsed())),
            );
        }
    }
    Ok(())
}

pub(super) fn acquire(
    workspace: &Path,
    config: &ForecastDocument,
    reference: DateTime<Tz>,
    client: &ProviderClient,
    control: &OperationControl,
) -> Vec<ForecastAcquisitionOutcome> {
    let setup = (|| {
        let bbox = config.buffered_bbox()?;
        let catalog = ForecastCatalog::open(workspace, &config.catalog, &config.artifact_root)
            .map_err(|e| e.to_string())?;
        let assets = catalog.asset_choices().map_err(|e| e.to_string())?;
        let download = Download {
            workspace,
            config,
            bbox,
            client,
            control,
        };
        Ok::<_, String>((catalog, assets, download))
    })();
    let mut setup = setup;
    let mut reports = Vec::new();
    for &provider in &config.providers {
        if control.is_cancelled() {
            break;
        }
        let mut outcome = ForecastAcquisitionOutcome {
            provider,
            attempted_cycles: Vec::new(),
            cycle_utc: None,
            reused: false,
            asset_id: None,
            retained_source: None,
            native_artifact: None,
            valid_from: None,
            valid_to: None,
            error: None,
        };
        outcome.error = match &mut setup {
            Ok((catalog, assets, download)) => download
                .provider(catalog, assets, reference, &mut outcome)
                .err(),
            Err(error) => Some(error.clone()),
        };
        control.download(|| DownloadEvent::ForecastFinished(outcome.clone()));
        reports.push(outcome);
    }
    reports
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    #[test]
    fn unavailable_cycle_tries_an_older_cycle_without_registering_partial_data() {
        std::fs::create_dir_all("runs").unwrap();
        let workspace = tempfile::tempdir_in("runs").unwrap();
        let root = workspace.path().canonicalize().unwrap();
        let service = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", service.local_addr().unwrap());
        let responder = std::thread::spawn(move || {
            for expected in ["12z", "00z"] {
                let (mut socket, _) = service.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(&socket);
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                assert!(request.contains(expected) && request.contains("360h-oper-fc.index"));
                loop {
                    let mut line = String::new();
                    assert!(reader.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                }
                socket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
            }
        });
        let client = ProviderClient::new().unwrap().test_ecmwf(endpoint);
        let config = ForecastDocument {
            catalog: "catalog.sqlite".into(),
            artifact_root: "forecasts".into(),
            providers: vec![ForecastProvider::Ecmwf],
            model_bbox: [-52., -30., -51., -29.],
            lookback_cycles: 2,
            buffer_fraction: 0.0,
        };
        let reference = "2026-07-20T13:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = events.clone();
        let control = OperationControl::with_observer(move |event| {
            if let super::super::super::OperationEvent::Download(event) = event {
                observed.lock().unwrap().push(event);
            }
        });
        let reports = acquire(
            &root,
            &config,
            reference.with_timezone(&chrono_tz::UTC),
            &client,
            &control,
        );
        responder.join().unwrap();
        assert_eq!(reports[0].attempted_cycles.len(), 2);
        assert!(reports[0].error.is_some() && reports[0].asset_id.is_none());
        let events = events.lock().unwrap();
        let followed_retry = matches!(
            events.as_slice(),
            [
                DownloadEvent::ForecastCycle { .. },
                DownloadEvent::ForecastLead { lead: 360, index: 1, .. },
                DownloadEvent::ForecastCycle { .. },
                DownloadEvent::ForecastLead { lead: 360, index: 1, .. },
                DownloadEvent::ForecastFinished(outcome),
            ] if outcome.error.is_some()
        );
        assert!(followed_retry);
        assert_eq!(
            std::fs::read_dir(root.join("forecasts")).unwrap().count(),
            0
        );
    }
}
