use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::analysis::{
    BulletinContent, BulletinFigure, BulletinTable, HourlyWindow, HydrographAdjustment,
    HydrographThreshold, RainfallSource, accumulated_rain, bulletin_html, bulletin_markdown,
    bulletin_pdf, render_rain_map_png, signed_rainfall_window, threshold_status, upstream_minis,
};
use crate::store::{
    ObservationReplacement, ObservationStore, ResultSeries, TimeInterval, read_mini_map_layer,
    read_mini_topology,
};

use super::hydrograph::{analyze_hydrograph_from_model, hydrograph_csv, hydrograph_png};
use super::series::{ModelSource, read_model};
use super::{
    HydrographReport, HydrographRequest, Location, SeriesQuery, SeriesSelection, Variable,
    query_series,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulletinRequest {
    pub trial: PathBuf,
    pub scenario_id: String,
    pub observations: PathBuf,
    pub geometry: PathBuf,
    pub map_geometry: Option<PathBuf>,
    pub profile: PathBuf,
    pub output: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct BulletinExport {
    pub output: PathBuf,
    pub reference_time: NaiveDateTime,
    pub time_zone: String,
    pub html: PathBuf,
    pub markdown: PathBuf,
    pub pdf: PathBuf,
    pub csv: Vec<PathBuf>,
    pub figures: Vec<PathBuf>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulletinStation {
    pub station_id: String,
    pub mini_id: Option<i64>,
    pub river: Option<String>,
    pub municipality: Option<String>,
    pub level_series: Option<String>,
    pub flow_series: Option<String>,
    pub rainfall_series: Option<String>,
    #[serde(default)]
    pub adjustment: HydrographAdjustment,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulletinProfile {
    pub title: String,
    pub basin: String,
    pub author: Option<String>,
    pub contact: Option<String>,
    pub introduction: Option<String>,
    pub outlet_mini_id: i64,
    pub history_hours: u32,
    pub forecast_hours: Option<u32>,
    pub forecast_lead_hours: Vec<u32>,
    #[serde(default)]
    pub comparison_scenarios: Vec<String>,
    pub rainfall_windows: Vec<i32>,
    pub catchment_layer: Option<String>,
    pub stations: Vec<BulletinStation>,
    #[serde(default)]
    pub replacement_ids: Vec<String>,
}

impl BulletinProfile {
    fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty()
            || self.basin.trim().is_empty()
            || self.outlet_mini_id <= 0
            || self.history_hours == 0
            || self.forecast_hours == Some(0)
            || self.forecast_lead_hours.contains(&0)
            || self.rainfall_windows.contains(&0)
            || self.stations.is_empty()
            || self
                .catchment_layer
                .as_ref()
                .is_some_and(|name| name.trim().is_empty())
        {
            return Err(
                "invalid bulletin title, basin, outlet, station list or time windows".into(),
            );
        }
        let mut seen = HashSet::new();
        for station in &self.stations {
            if station.station_id.trim().is_empty()
                || station.mini_id.is_some_and(|id| id <= 0)
                || !seen.insert(station.station_id.as_str())
            {
                return Err("invalid or duplicate bulletin station".into());
            }
        }
        Ok(())
    }
}

fn display(value: Option<f64>) -> String {
    value.map_or_else(|| "—".into(), |v| format!("{v:.1}"))
}

fn table(title: &str, columns: &[&str]) -> BulletinTable {
    BulletinTable {
        title: title.into(),
        columns: columns.iter().map(|value| (*value).into()).collect(),
        rows: Vec::new(),
    }
}

fn table_csv(table: &BulletinTable) -> Result<Vec<u8>, String> {
    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record(&table.columns)
        .map_err(|e| e.to_string())?;
    for row in &table.rows {
        csv.write_record(row).map_err(|e| e.to_string())?;
    }
    csv.into_inner().map_err(|e| e.to_string())
}

struct ReportSources<'a> {
    request: &'a BulletinRequest,
    time_zone: &'a str,
    window: &'a TimeInterval,
    model: &'a ModelSource,
    scenario_id: &'a str,
    replacements: &'a [ObservationReplacement],
}

impl ReportSources<'_> {
    fn series(&self, station: &str, series_id: &str, variable: Variable) -> SeriesSelection {
        let name = match variable {
            Variable::Rain => "rain",
            Variable::Flow => "flow",
            Variable::Level => "level",
        };
        let replacements: Vec<_> = self
            .replacements
            .iter()
            .filter(|replacement| replacement.station_id == station && replacement.variable == name)
            .cloned()
            .collect();
        let effective = matches!(variable, Variable::Rain) || !replacements.is_empty();
        SeriesSelection::Observation {
            database: self.request.observations.clone(),
            series_id: series_id.into(),
            effective,
            replacement_ids: Vec::new(),
            replacements: Some(replacements),
            variable,
        }
    }

    fn hydrograph(
        &self,
        profile: &BulletinStation,
        mini: Option<i64>,
        series: Option<&str>,
        variable: Variable,
        thresholds: Vec<HydrographThreshold>,
    ) -> Result<HydrographReport, String> {
        let mut selections = Vec::new();
        if mini.is_some() {
            selections.push(SeriesSelection::Model {
                root: self.request.trial.clone(),
                scenario_id: self.scenario_id.into(),
                variable: variable.clone(),
            });
        }
        if let Some(series) = series {
            selections.push(self.series(&profile.station_id, series, variable));
        }
        analyze_hydrograph_from_model(
            &HydrographRequest {
                series: SeriesQuery {
                    location: mini.map_or_else(
                        || Location::Station(profile.station_id.clone()),
                        Location::Mini,
                    ),
                    time_zone: self.time_zone.into(),
                    window: HourlyWindow::Explicit {
                        start: self.window.start,
                        end: self.window.end,
                    },
                    selections,
                },
                comparison_station: mini.map(|_| profile.station_id.clone()),
                adjustment: profile.adjustment.clone(),
                thresholds,
            },
            Some((&self.request.trial, &self.request.scenario_id, self.model)),
        )
    }
}

fn point_at(curve: Option<&super::HydrographCurve>, time: NaiveDateTime) -> Option<f64> {
    curve?
        .points
        .iter()
        .find(|point| point.timestamp == time)?
        .value
}

fn forecast_level(report: &HydrographReport) -> Option<&super::HydrographCurve> {
    report
        .curves
        .iter()
        .find(|curve| curve.run_id.is_some() && curve.unit == "cm")
}

fn tendency(
    current: Option<f64>,
    forecast: Option<&super::HydrographCurve>,
    reference: NaiveDateTime,
    target: NaiveDateTime,
) -> String {
    let base = match current {
        Some(value) => Some((value, "observado")),
        None => point_at(forecast, reference).map(|value| (value, "modelado")),
    };
    let Some(((base, kind), future)) = base.zip(point_at(forecast, target)) else {
        return "—".into();
    };
    let delta = future - base;
    let direction = if delta > 0.0 {
        "Subindo"
    } else if delta < 0.0 {
        "Descendo"
    } else {
        "Estável"
    };
    format!(
        "{direction} ({delta:+.1} cm) · {kind} {} até {}",
        reference.format("%d/%m %H:%M"),
        target.format("%d/%m %H:%M"),
    )
}

fn daily_rows(table: &mut BulletinTable, name: &str, report: &HydrographReport) {
    for curve in &report.curves {
        let mut dates = BTreeMap::<(NaiveDate, Option<u8>), (usize, f64, f64)>::new();
        for point in &curve.points {
            if let Some(value) = point.value {
                let stats = dates
                    .entry((point.timestamp.date(), point.segment))
                    .or_insert((0, value, value));
                stats.0 += 1;
                stats.1 = stats.1.min(value);
                stats.2 = stats.2.max(value);
            }
        }
        for ((date, segment), (count, min, max)) in dates {
            let label = match segment {
                Some(0) => "Simulado",
                Some(1) => "Previsto",
                _ => "Observado",
            };
            table.rows.push(vec![
                name.into(),
                date.to_string(),
                label.into(),
                curve.unit.clone(),
                count.to_string(),
                display(Some(min)),
                display(Some(max)),
            ]);
        }
    }
}

fn rain_by_mini(results: &ResultSeries, window: &TimeInterval) -> HashMap<i64, Option<f64>> {
    let wanted = (window.end - window.start).num_hours() as usize;
    let mut counts = vec![0_usize; results.basins.mini_ids.len()];
    let mut sums = vec![0.0; counts.len()];
    for (i, bounds) in results.timeline.bounds.iter().enumerate() {
        if bounds.start >= window.start
            && bounds.end <= window.end
            && bounds.end - bounds.start == Duration::hours(1)
        {
            let row = i * counts.len();
            for (j, (count, sum)) in counts.iter_mut().zip(&mut sums).enumerate() {
                if results.precipitation_valid[row + j] {
                    *count += 1;
                    *sum += results.precipitation[row + j];
                }
            }
        }
    }
    results
        .basins
        .mini_ids
        .iter()
        .copied()
        .zip(
            counts
                .into_iter()
                .zip(sums)
                .map(|(count, sum)| (count == wanted && sum.is_finite()).then_some(sum)),
        )
        .collect()
}

pub(super) fn validate_geometry(
    results: &ResultSeries,
    geometry: &crate::store::BasinGeometry,
) -> Result<(), String> {
    let coordinates: HashMap<_, _> = geometry
        .mini_ids
        .iter()
        .zip(geometry.longitudes.iter().zip(&geometry.latitudes))
        .collect();
    if coordinates.len() != results.basins.mini_ids.len()
        || results
            .basins
            .mini_ids
            .iter()
            .zip(
                results
                    .basins
                    .longitudes
                    .iter()
                    .zip(&results.basins.latitudes),
            )
            .any(|(id, (lon, lat))| {
                coordinates
                    .get(id)
                    .is_none_or(|(x, y)| (lon - *x).abs() > 1e-8 || (lat - *y).abs() > 1e-8)
            })
    {
        return Err("report geometry disagrees with retained basin IDs or coordinates".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct BulletinPreview {
    pub reference_time: NaiveDateTime,
    pub time_zone: String,
    pub content: BulletinContent,
}

pub fn preview_bulletin(request: &BulletinRequest) -> Result<BulletinPreview, String> {
    build_bulletin(request).map(|(preview, _)| preview)
}

type Files = Vec<(String, Vec<u8>)>;

pub(super) fn retained_result(
    root: &Path,
    scenario_id: &str,
) -> Result<(ModelSource, NaiveDateTime), String> {
    let model = read_model(root, scenario_id, None, false)?;
    let reference = model
        .reference
        .parse()
        .map_err(|_| "invalid forecast reference time")?;
    Ok((model, reference))
}

fn build_bulletin(request: &BulletinRequest) -> Result<(BulletinPreview, Files), String> {
    let profile: BulletinProfile =
        toml::from_str(&fs::read_to_string(&request.profile).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    profile.validate()?;
    let (model, reference) = retained_result(&request.trial, &request.scenario_id)?;
    let results = &model.basin;
    let zone = &results.timeline.time_zone;
    let hour = |offset| {
        reference
            .checked_add_signed(Duration::hours(offset))
            .ok_or("bulletin time window overflow")
    };
    let retained_end = results
        .timeline
        .bounds
        .last()
        .ok_or("empty retained result")?
        .end;
    let requested_end = profile
        .forecast_hours
        .map_or(Ok(retained_end), |v| hour(i64::from(v)))?;
    let window = TimeInterval {
        start: hour(-i64::from(profile.history_hours))?,
        end: requested_end.min(retained_end),
    };
    if window.end <= reference {
        return Err("retained scenario lacks forecast coverage".into());
    }
    let (geometry, topology) = read_mini_topology(&request.geometry)?;
    validate_geometry(results, &geometry)?;
    let mut scenarios = HashSet::from([request.scenario_id.as_str()]);
    let mut comparisons = Vec::new();
    for id in &profile.comparison_scenarios {
        if !scenarios.insert(id) {
            return Err(format!("duplicate bulletin scenario {id}"));
        }
        let (source, cycle) = retained_result(&request.trial, id)?;
        if cycle != reference || source.basin.timeline.time_zone != *zone {
            return Err(format!(
                "bulletin scenario {id}: incompatible cycle or time zone"
            ));
        }
        validate_geometry(&source.basin, &geometry)?;
        comparisons.push((id.as_str(), source));
    }
    let upstream: HashSet<_> = upstream_minis(&topology, profile.outlet_mini_id)?
        .into_iter()
        .collect();
    let store =
        ObservationStore::open_read_only(&request.observations).map_err(|e| e.to_string())?;
    if store.time_zone().map_err(|e| e.to_string())? != *zone {
        return Err("bulletin observations and trial time zones differ".into());
    }
    let stations: HashMap<_, _> = store
        .station_locations()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|location| (location.station.station_id.clone(), location))
        .collect();
    let references = store.read_level_references().map_err(|e| e.to_string())?;
    let replacements = store
        .load_replacements(&profile.replacement_ids)
        .map_err(|e| e.to_string())?;
    let sources = ReportSources {
        request,
        time_zone: zone,
        window: &window,
        model: &model,
        scenario_id: &request.scenario_id,
        replacements: &replacements,
    };
    let issue = format!("Referência {} ({zone})", reference.format("%d/%m/%Y %H:%M"));
    let scenario = format!("Cenário {}", request.scenario_id);
    let mut details = vec![profile.basin.clone(), issue, scenario];
    if !comparisons.is_empty() {
        let alternatives = profile.comparison_scenarios.join(", ");
        details.push(format!("Cenários adicionais para a faixa {alternatives}"));
    }

    details.extend(profile.author);
    details.extend(profile.contact);
    details.extend(profile.introduction);
    let mut content = BulletinContent {
        title: profile.title,
        details,
        tables: Vec::new(),
        figures: Vec::new(),
        notes: Vec::new(),
    };
    if requested_end > retained_end {
        content
            .notes
            .push("Previsão limitada ao fim do cenário retido.".into());
    }
    let mut summary = table(
        "Estações e níveis",
        &[
            "Estação",
            "Curso d’água",
            "Município",
            "Chuva 24 h (mm)",
            "Nível atual (cm)",
            "Variação 1 h (cm)",
            "Variação 24 h (cm)",
            "Inundação (cm)",
            "Status",
            "Pico previsto (cm)",
        ],
    );
    for lead in &profile.forecast_lead_hours {
        summary.columns.push(format!("Nível +{lead} h (cm)"));
    }
    if !comparisons.is_empty() {
        for lead in &profile.forecast_lead_hours {
            summary.columns.push(format!("Faixa +{lead} h (cm)"));
        }
    }
    let trend_end = profile
        .forecast_lead_hours
        .iter()
        .copied()
        .max()
        .map_or(Ok(window.end), |lead| hour(i64::from(lead)))?;
    let trend_hours = (trend_end - reference).num_hours();
    summary.columns.push(format!("Tendência +{trend_hours} h"));
    let lead_times: Vec<_> = profile
        .forecast_lead_hours
        .iter()
        .map(|lead| hour(i64::from(*lead)))
        .collect::<Result<_, _>>()?;
    let mut daily = table(
        "Hidrogramas diários",
        &[
            "Estação",
            "Data",
            "Série",
            "Unidade",
            "Amostras",
            "Mínimo",
            "Máximo",
        ],
    );
    let mut rain_summary = table(
        "Chuva média ponderada na bacia",
        &[
            "Origem",
            "Início",
            "Fim",
            "Área (km²)",
            "Cobertura (km²·h)",
            "Chuva (mm)",
        ],
    );
    let mut chart_csv = Vec::new();
    let mut locations = Vec::new();
    for (index, selection) in profile.stations.iter().enumerate() {
        let location = stations
            .get(&selection.station_id)
            .ok_or_else(|| format!("unknown bulletin station {}", selection.station_id))?;
        let name = &location.station.name;
        if let Some((lon, lat)) = location.station.longitude.zip(location.station.latitude) {
            locations.push((name.clone(), lon, lat));
        }
        let mini = selection
            .mini_id
            .or(location.offset.as_ref().map(|offset| offset.mini_id));
        let mut thresholds = Vec::new();
        let mut flood = None;
        for value in references
            .iter()
            .filter(|value| value.station_id == selection.station_id && value.date.is_none())
        {
            let name = match value.kind.as_str() {
                "attention" => "Atenção",
                "alert" => "Alerta",
                "flood" => {
                    flood = Some(value.level_cm);
                    "Inundação"
                }
                "severe" => "Severa",
                _ => continue,
            };
            thresholds.push(HydrographThreshold {
                name: name.into(),
                value: value.level_cm,
            });
        }
        thresholds.sort_by_key(|limit| match limit.name.as_str() {
            "Atenção" => 0,
            "Alerta" => 1,
            "Inundação" => 2,
            _ => 3,
        });
        let mut level = None;
        for (variable, series, label, limits) in [
            (
                Variable::Level,
                selection.level_series.as_deref(),
                "Nível",
                thresholds.clone(),
            ),
            (
                Variable::Flow,
                selection.flow_series.as_deref(),
                "Vazão",
                Vec::new(),
            ),
        ] {
            if series.is_none() && mini.is_none() {
                continue;
            }
            let report = sources.hydrograph(selection, mini, series, variable.clone(), limits)?;
            for finding in &report.findings {
                content
                    .notes
                    .push(format!("{name} · {label} · {}", finding.message));
            }
            daily_rows(&mut daily, name, &report);
            if report
                .curves
                .iter()
                .any(|curve| curve.points.iter().any(|point| point.value.is_some()))
            {
                let kind = if matches!(variable, Variable::Level) {
                    "nivel"
                } else {
                    "vazao"
                };
                let base = format!("hidrograma-{}-{kind}", index + 1);
                chart_csv.push((format!("{base}.csv"), hydrograph_csv(&report)?));
                content.figures.push(BulletinFigure {
                    title: format!("{name} · {label}"),
                    filename: format!("{base}.png"),
                    png: hydrograph_png(&report)?,
                });
            }
            if matches!(variable, Variable::Level) {
                level = Some(report);
            }
        }
        let observed = level.as_ref().and_then(|report| {
            report
                .curves
                .iter()
                .find(|curve| curve.run_id.is_none() && curve.unit == "cm")
        });
        let predicted = level.as_ref().and_then(forecast_level);
        let now = point_at(observed, reference);
        let change = |past| {
            now.zip(point_at(observed, past))
                .map(|(now, old)| now - old)
        };
        let rainfall = if let Some(series) = &selection.rainfall_series {
            let rain = query_series(&SeriesQuery {
                location: Location::Station(selection.station_id.clone()),
                time_zone: zone.into(),
                window: HourlyWindow::Relative {
                    end: reference,
                    hours: 24,
                },
                selections: vec![sources.series(&selection.station_id, series, Variable::Rain)],
            })?;
            for finding in rain.findings {
                content
                    .notes
                    .push(format!("{name} · Chuva · {}", finding.message));
            }
            rain.series.first().and_then(|series| {
                (series.points.len() == 24)
                    .then(|| {
                        series
                            .points
                            .iter()
                            .map(|point| point.value)
                            .sum::<Option<f64>>()
                    })
                    .flatten()
            })
        } else {
            None
        };
        let status = now.filter(|_| !thresholds.is_empty()).map_or("—", |v| {
            threshold_status(v, &thresholds).unwrap_or("Normal")
        });
        let peak = predicted.and_then(|curve| curve.peak.as_ref());
        let predicted_peak = peak.map_or_else(
            || "—".into(),
            |peak| {
                format!(
                    "{:.1} · {}",
                    peak.value,
                    peak.timestamp.format("%d/%m %H:%M")
                )
            },
        );
        let mut row = vec![
            name.clone(),
            selection.river.clone().unwrap_or_default(),
            selection.municipality.clone().unwrap_or_default(),
            display(rainfall),
            display(now),
            display(change(hour(-1)?)),
            display(change(hour(-24)?)),
            display(flood),
            status.into(),
            predicted_peak,
        ];
        let primary: Vec<_> = lead_times
            .iter()
            .map(|time| point_at(predicted, *time))
            .collect();
        row.extend(primary.iter().map(|value| display(*value)));
        if !comparisons.is_empty() {
            let mut ranges: Vec<Vec<_>> = primary
                .into_iter()
                .map(|value| value.into_iter().collect())
                .collect();
            if mini.is_some() {
                for (scenario_id, model) in &comparisons {
                    let source = ReportSources {
                        scenario_id,
                        model,
                        ..sources
                    };
                    let report = source.hydrograph(
                        selection,
                        mini,
                        selection.level_series.as_deref(),
                        Variable::Level,
                        thresholds.clone(),
                    )?;
                    for finding in &report.findings {
                        content
                            .notes
                            .push(format!("{name} · {scenario_id} · {}", finding.message));
                    }
                    let forecast = forecast_level(&report);
                    for (range, time) in ranges.iter_mut().zip(&lead_times) {
                        range.extend(point_at(forecast, *time));
                    }
                }
            }
            row.extend(ranges.into_iter().map(|mut values| {
                values.sort_by(f64::total_cmp);
                values.first().zip(values.last()).map_or_else(
                    || "—".into(),
                    |(min, max)| {
                        format!("{min:.1} a {max:.1} ({}/{})", values.len(), scenarios.len())
                    },
                )
            }));
        }
        row.push(tendency(now, predicted, reference, trend_end));
        summary.rows.push(row);
    }
    let features = profile
        .catchment_layer
        .as_ref()
        .map(|layer| {
            let path = request
                .map_geometry
                .as_deref()
                .ok_or("catchment_layer requires map_geometry")?;
            read_mini_map_layer(path, layer, &topology)
        })
        .transpose()?;
    for (index, hours) in profile.rainfall_windows.iter().enumerate() {
        let (origin, bounds) = signed_rainfall_window(zone, reference, *hours)?;
        let label = match origin {
            RainfallSource::Observed => "Observada",
            RainfallSource::Forecast => "Prevista",
        };
        let explicit = HourlyWindow::Explicit {
            start: bounds.start,
            end: bounds.end,
        };
        let rain = accumulated_rain(results, &topology, profile.outlet_mini_id, true, &explicit)?;
        rain_summary.rows.push(vec![
            label.into(),
            bounds.start.to_string(),
            bounds.end.to_string(),
            display(Some(rain.total_area_km2)),
            display(Some(rain.covered_area_hours)),
            display(rain.mean_mm),
        ]);
        if let Some(features) = &features {
            let values = rain_by_mini(results, &bounds);
            let selected: Vec<_> = features
                .iter()
                .filter(|feature| upstream.contains(&feature.mini_id))
                .map(|feature| (feature, values.get(&feature.mini_id).copied().flatten()))
                .collect();
            content.figures.push(BulletinFigure {
                title: format!(
                    "Chuva {} · {} h",
                    label.to_lowercase(),
                    hours.unsigned_abs()
                ),
                filename: format!("chuva-{}.png", index + 1),
                png: render_rain_map_png(&selected, &locations)?,
            });
        }
    }
    content.tables = vec![summary, rain_summary, daily];
    Ok((
        BulletinPreview {
            reference_time: reference,
            time_zone: zone.clone(),
            content,
        },
        chart_csv,
    ))
}

pub fn generate_bulletin(request: &BulletinRequest) -> Result<BulletinExport, String> {
    if request.output.exists() {
        return Err("bulletin output already exists".into());
    }
    let (
        BulletinPreview {
            reference_time,
            time_zone,
            content,
        },
        chart_csv,
    ) = build_bulletin(request)?;
    let mut files = vec![
        ("boletim.html".into(), bulletin_html(&content).into_bytes()),
        (
            "boletim.md".into(),
            bulletin_markdown(&content).into_bytes(),
        ),
        ("boletim.pdf".into(), bulletin_pdf(&content)?),
    ];
    for (filename, table) in ["estacoes.csv", "chuva.csv", "diario.csv"]
        .into_iter()
        .zip(&content.tables)
    {
        files.push((filename.into(), table_csv(table)?));
    }
    files.extend(chart_csv);
    let csv = files
        .iter()
        .filter(|(name, _)| name.ends_with(".csv"))
        .map(|(name, _)| request.output.join(name))
        .collect();
    let figures = content
        .figures
        .into_iter()
        .map(|figure| {
            let path = request.output.join(&figure.filename);
            files.push((figure.filename, figure.png));
            path
        })
        .collect();
    write_bulletin_files(&request.output, files)?;
    Ok(BulletinExport {
        output: request.output.clone(),
        reference_time,
        time_zone,
        html: request.output.join("boletim.html"),
        markdown: request.output.join("boletim.md"),
        pdf: request.output.join("boletim.pdf"),
        csv,
        figures,
        notes: content.notes,
    })
}

fn write_bulletin_files(output: &Path, files: Vec<(String, Vec<u8>)>) -> Result<(), String> {
    crate::store::publish_directory(output, |staging| {
        for (name, bytes) in files {
            fs::write(staging.join(name), &bytes).map_err(|error| error.to_string())?;
        }
        Ok(())
    })
}
