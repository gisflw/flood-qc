use std::collections::BTreeMap;
use std::io::{self, Write};

pub(super) fn acquisition_text(
    report: &mgb_ops::app::AcquisitionReport,
    out: &mut impl Write,
) -> io::Result<()> {
    writeln!(out, "Reference {}", report.reference_time)?;
    if let Some(ana) = &report.ana {
        if let Some(error) = &ana.error {
            writeln!(out, "ANA: {error}")?;
        }
        for station in &ana.stations {
            writeln!(
                out,
                "ANA {}: {} requested days, {} skipped, {} rows, {} changed samples",
                station.station_code,
                station.requested_days,
                station.skipped_days,
                station.normalized_rows,
                station.changed_samples
            )?;
            if let Some(error) = &station.error {
                writeln!(out, "  {error}")?;
            }
            if let Some(path) = &station.csv {
                writeln!(out, "  {}", path.display())?;
            }
            for path in &station.raw_xml {
                writeln!(out, "  {}", path.display())?;
            }
        }
    }
    for forecast in &report.forecasts {
        if let Some(error) = &forecast.error {
            writeln!(
                out,
                "{:?} ({} cycles tried): {error}",
                forecast.provider,
                forecast.attempted_cycles.len()
            )?;
        } else if let Some(cycle) = forecast.cycle_utc {
            let action = if forecast.reused {
                "reused"
            } else {
                "registered"
            };
            writeln!(out, "{:?} {action} {cycle}", forecast.provider)?;
            for path in [&forecast.retained_source, &forecast.native_artifact]
                .into_iter()
                .flatten()
            {
                writeln!(out, "  {}", path.display())?;
            }
        }
    }
    if report.cancelled {
        writeln!(out, "Acquisition cancelled")?;
    }
    Ok(())
}

pub(super) fn acquisition_result(
    result: Result<mgb_ops::app::AcquisitionReport, String>,
    json: bool,
    out: &mut impl Write,
) -> Result<bool, Box<dyn std::error::Error>> {
    match result {
        Ok(report) => {
            if json {
                serde_json::to_writer(&mut *out, &report)?;
                writeln!(out)?;
            } else {
                acquisition_text(&report, out)?;
            }
            Ok(report.is_complete())
        }
        Err(error) if json => {
            serde_json::to_writer(&mut *out, &BTreeMap::from([("error", error)]))?;
            writeln!(out)?;
            Ok(false)
        }
        Err(error) => Err(error.into()),
    }
}
pub(super) fn operation_result<T>(
    result: Result<T, mgb_ops::OperationError>,
    json: bool,
    out: &mut impl Write,
) -> Result<Option<T>, Box<dyn std::error::Error>> {
    match result {
        Ok(report) => Ok(Some(report)),
        Err(error) if json => {
            serde_json::to_writer(&mut *out, &error)?;
            writeln!(out)?;
            out.flush()?;
            Ok(None)
        }
        Err(error) => Err(Box::new(error)),
    }
}
