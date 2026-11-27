use std::{io::Write, path::PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use clap::{Args, Subcommand, ValueEnum};
use mgb_ops::{
    OperationControl,
    app::{AcquisitionSource, SourceAcquisitionRequest, acquire_source},
};

#[derive(Subcommand)]
pub(super) enum AcquisitionCommand {
    /// Download BNDMET hourly rain or paired wind observations.
    Bndmet {
        #[arg(long, value_enum)]
        product: BndmetProduct,
        #[arg(long)]
        station: Vec<String>,
        #[command(flatten)]
        dates: Dates,
        #[command(flatten)]
        destination: Destination,
    },
    /// Download ONS hourly inflow and defluence observations.
    Ons {
        #[arg(long)]
        reservoir: Vec<String>,
        #[command(flatten)]
        dates: Dates,
        #[command(flatten)]
        destination: Destination,
    },
    /// Download NASA IMERG Early half-hours and write hourly rainfall NetCDF.
    Imerg {
        /// First UTC hour boundary, in RFC 3339.
        #[arg(long)]
        start: DateTime<Utc>,
        /// Exclusive UTC end boundary, in RFC 3339.
        #[arg(long)]
        end: DateTime<Utc>,
        /// Geographic west,south,east,north crop without an antimeridian crossing.
        #[arg(long)]
        bbox: String,
        /// Register completed regional UTC days in this workspace.
        #[arg(long)]
        workspace: Option<PathBuf>,
        #[command(flatten)]
        destination: Destination,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(super) enum BndmetProduct {
    Rain,
    Wind,
}

#[derive(Args)]
pub(super) struct Dates {
    /// First provider calendar date, inclusive.
    #[arg(long)]
    start: NaiveDate,
    /// Last provider calendar date, inclusive.
    #[arg(long)]
    end: NaiveDate,
}

#[derive(Args)]
pub(super) struct Destination {
    /// Create a new directory at this path.
    #[arg(long)]
    output: PathBuf,
    /// Timezone of normalized timestamps, defaulting to America/Sao_Paulo.
    #[arg(long)]
    time_zone: Option<Tz>,
}

fn parse_bbox(input: &str) -> Result<[f64; 4], String> {
    input
        .split(',')
        .map(|coordinate| {
            coordinate
                .trim()
                .parse()
                .map_err(|_| format!("invalid bbox coordinate {coordinate}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "expected four comma-separated values for --bbox".into())
}

pub(super) fn run(
    command: AcquisitionCommand,
    control: &OperationControl,
    out: &mut impl Write,
) -> Result<bool, Box<dyn std::error::Error>> {
    let (source, destination, workspace) = match command {
        AcquisitionCommand::Bndmet {
            product,
            station,
            dates,
            destination,
        } => (
            AcquisitionSource::Bndmet {
                first_date: dates.start,
                last_date: dates.end,
                stations: station,
                wind: matches!(product, BndmetProduct::Wind),
            },
            destination,
            None,
        ),
        AcquisitionCommand::Ons {
            reservoir,
            dates,
            destination,
        } => (
            AcquisitionSource::OnsHourly {
                first_date: dates.start,
                last_date: dates.end,
                reservoirs: reservoir,
            },
            destination,
            None,
        ),
        AcquisitionCommand::Imerg {
            start,
            end,
            bbox,
            destination,
            workspace,
        } => (
            AcquisitionSource::ImergEarly {
                start,
                end,
                bbox: parse_bbox(&bbox)?,
            },
            destination,
            workspace,
        ),
    };
    let report = acquire_source(
        &SourceAcquisitionRequest {
            source,
            output: destination.output,
            time_zone: destination
                .time_zone
                .unwrap_or(chrono_tz::America::Sao_Paulo),
        },
        control,
    )?;
    let mut value = serde_json::to_value(&report)?;
    if let Some(workspace) = workspace
        && report.issues.is_empty()
    {
        let ids = mgb_ops::app::register_imerg_acquisition(&workspace, &report.normalized)?;
        value["registered_objects"] = serde_json::to_value(ids)?;
    }
    serde_json::to_writer_pretty(&mut *out, &value)?;
    writeln!(out)?;
    Ok(report.issues.is_empty())
}

#[cfg(test)]
mod tests {
    use super::super::{Cli, Command};
    use super::*;
    use clap::Parser;

    #[test]
    fn imerg_accepts_comma_separated_negative_bounds() {
        let cli = Cli::try_parse_from([
            "mgb-ops",
            "acquisition",
            "imerg",
            "--bbox=-59,-35,-48,-26",
            "--start",
            "2026-11-22T00:00:00Z",
            "--end",
            "2026-11-22T01:00:00Z",
            "--output",
            "standalone",
        ])
        .unwrap();
        let Command::Acquisition {
            command: AcquisitionCommand::Imerg { bbox, .. },
        } = cli.command
        else {
            panic!("expected IMERG acquisition");
        };
        assert_eq!(parse_bbox(&bbox).unwrap(), [-59.0, -35.0, -48.0, -26.0]);
    }
}
