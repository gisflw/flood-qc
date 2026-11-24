use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Timelike};
use chrono_tz::{America::Sao_Paulo, Tz};
use serde::{Deserialize, Serialize};

use crate::store::{ObservationValue, Station};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnaVariable {
    Rain,
    Level,
    Flow,
}

impl AnaVariable {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Rain => "rain",
            Self::Level => "level",
            Self::Flow => "flow",
        }
    }

    fn parse(text: &str) -> Result<Self, String> {
        match text.trim().to_ascii_lowercase().as_str() {
            "rain" => Ok(Self::Rain),
            "level" => Ok(Self::Level),
            "flow" => Ok(Self::Flow),
            _ => Err(format!("unsupported ANA variable {text:?}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Aggregation {
    Sum,
    Mean,
    Last,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct AnaAggregation {
    rain: Aggregation,
    level: Aggregation,
    flow: Aggregation,
}

impl Default for AnaAggregation {
    fn default() -> Self {
        Self {
            rain: Aggregation::Sum,
            level: Aggregation::Mean,
            flow: Aggregation::Mean,
        }
    }
}

pub(crate) struct AnaStation {
    pub station: Station,
    pub variables: BTreeSet<AnaVariable>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct AnaReading {
    pub time: NaiveDateTime,
    variable: AnaVariable,
    value: f64,
}

pub(crate) type AnaHourly = BTreeMap<AnaVariable, Vec<(NaiveDateTime, ObservationValue)>>;

pub(crate) fn ana_station_code(value: &str) -> Result<&str, String> {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("invalid ANA station code {value:?}"));
    }
    let code = value.trim_start_matches('0');
    Ok(if code.is_empty() { "0" } else { code })
}

#[derive(Deserialize)]
struct InventoryRow {
    provider_code: String,
    station_code: String,
    station_name: String,
    latitude: String,
    longitude: String,
    observed_variables: String,
}

pub(crate) fn read_ana_inventory(input: impl Read) -> Result<Vec<AnaStation>, String> {
    let mut reader = csv::Reader::from_reader(input);
    let mut stations = BTreeMap::new();
    for row in reader.deserialize::<InventoryRow>() {
        let row = row.map_err(|e| e.to_string())?;
        if !row.provider_code.trim().eq_ignore_ascii_case("ana") {
            continue;
        }
        let code = ana_station_code(&row.station_code)?;
        let coordinate = |value: &str| -> Result<Option<f64>, String> {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                value
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| format!("invalid coordinate for ANA station {code}"))
            }
        };
        let name = row.station_name.trim();
        if name.is_empty() {
            return Err(format!("ANA station {code} requires a name"));
        }
        let mut variables = BTreeSet::new();
        let declared = row.observed_variables.trim();
        if !declared.eq_ignore_ascii_case("none") {
            for code in declared.split([',', ';', '|']) {
                let variable = AnaVariable::parse(code)?;
                if !variables.insert(variable) {
                    return Err(format!("duplicate ANA variable {code:?}"));
                }
            }
        }
        let station = Station {
            station_id: format!("ana:{code}"),
            provider_code: "ana".into(),
            station_code: code.into(),
            name: name.into(),
            latitude: coordinate(&row.latitude)?,
            longitude: coordinate(&row.longitude)?,
        };
        if stations
            .insert(code.to_owned(), AnaStation { station, variables })
            .is_some()
        {
            return Err(format!("duplicate ANA station {code}"));
        }
    }
    if stations.is_empty() {
        return Err("inventory has no ANA stations".into());
    }
    Ok(stations.into_values().collect())
}

fn local_time(value: &str) -> Result<NaiveDateTime, String> {
    let value = value.trim();
    let time = DateTime::parse_from_rfc3339(value)
        .map(|at| at.with_timezone(&Sao_Paulo).naive_local())
        .ok()
        .or_else(|| {
            [
                "%Y-%m-%dT%H:%M:%S%.f",
                "%Y-%m-%d %H:%M:%S%.f",
                "%Y-%m-%dT%H:%M",
                "%Y-%m-%d %H:%M",
            ]
            .into_iter()
            .find_map(|format| NaiveDateTime::parse_from_str(value, format).ok())
        })
        .ok_or_else(|| format!("invalid ANA timestamp {value:?}"))?;
    Ok(time.with_second(0).unwrap().with_nanosecond(0).unwrap())
}

pub(crate) fn parse_ana_xml(
    xml: &[u8],
    station_code: &str,
    variables: &BTreeSet<AnaVariable>,
) -> Result<Vec<AnaReading>, String> {
    let text = std::str::from_utf8(xml).map_err(|e| e.to_string())?;
    let document = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
    let root = document.root_element();
    let mut rows = Vec::new();
    let mut recognized = matches!(
        root.tag_name().name(),
        "DataTable" | "NewDataSet" | "DocumentElement"
    );
    for node in document.descendants().filter(|node| node.is_element()) {
        match node.tag_name().name() {
            "Fault" => return Err("ANA returned a SOAP fault".into()),
            "DadosHidrometereologicos" => recognized = true,
            _ => continue,
        }
        let field = |name| {
            node.children()
                .find(|child| child.has_tag_name(name))
                .and_then(|child| child.text())
        };
        if field("CodEstacao").and_then(|code| ana_station_code(code).ok()) != Some(station_code) {
            continue;
        }
        let Some(time) = field("DataHora").and_then(|time| local_time(time).ok()) else {
            continue;
        };
        for (name, variable) in [
            ("Chuva", AnaVariable::Rain),
            ("Nivel", AnaVariable::Level),
            ("Vazao", AnaVariable::Flow),
        ] {
            if variables.contains(&variable)
                && let Some(value) =
                    field(name).and_then(|text| text.trim().replace(',', ".").parse::<f64>().ok())
                && value.is_finite()
            {
                rows.push(AnaReading {
                    time,
                    variable,
                    value,
                });
            }
        }
    }
    if !recognized {
        return Err("ANA response has an unexpected XML root".into());
    }
    rows.sort_by_key(|row| (row.time, row.variable));
    Ok(rows)
}

const CSV_HEADER: [&str; 5] = [
    "provider_code",
    "station_code",
    "observed_at",
    "variable_code",
    "value",
];

pub(crate) fn ana_csv(code: &str, rows: &[AnaReading]) -> Result<Vec<u8>, String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(CSV_HEADER).map_err(|e| e.to_string())?;
    for row in rows {
        writer
            .write_record([
                "ana",
                code,
                &row.time.format("%Y-%m-%d %H:%M").to_string(),
                row.variable.code(),
                &row.value.to_string(),
            ])
            .map_err(|e| e.to_string())?;
    }
    writer.into_inner().map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct CsvRow {
    provider_code: String,
    station_code: String,
    observed_at: String,
    variable_code: String,
    value: f64,
    station_id: Option<String>,
    state: Option<String>,
}

pub(crate) fn read_ana_csv(input: impl Read) -> Result<BTreeMap<String, Vec<AnaReading>>, String> {
    let mut reader = csv::Reader::from_reader(input);
    let headers = reader.headers().map_err(|e| e.to_string())?;
    let actual: BTreeSet<_> = headers.iter().collect();
    let current: BTreeSet<_> = CSV_HEADER.into_iter().collect();
    let transition: BTreeSet<_> = current
        .iter()
        .copied()
        .chain(["station_id", "state"])
        .collect();
    if actual.len() != headers.len() || (actual != current && actual != transition) {
        return Err("expected legacy normalized observation CSV columns".into());
    }
    let mut stations = BTreeMap::<String, Vec<_>>::new();
    for row in reader.deserialize::<CsvRow>() {
        let row = row.map_err(|e| e.to_string())?;
        let code = ana_station_code(&row.station_code)?;
        if !row.provider_code.trim().eq_ignore_ascii_case("ana")
            || !row.value.is_finite()
            || row.state.is_some_and(|value| value != "raw")
            || row
                .station_id
                .is_some_and(|value| value != format!("ana:{code}"))
        {
            return Err(format!("invalid raw ANA CSV row for station {code}"));
        }
        stations.entry(code.into()).or_default().push(AnaReading {
            time: local_time(&row.observed_at)?,
            variable: AnaVariable::parse(&row.variable_code)?,
            value: row.value,
        });
    }
    Ok(stations)
}

pub(crate) fn aggregate_ana(
    mut readings: Vec<AnaReading>,
    station: &AnaStation,
    zone: Tz,
    methods: AnaAggregation,
) -> Result<AnaHourly, String> {
    readings.sort_by_key(|row| row.time);
    let mut buckets = BTreeMap::new();
    for row in readings {
        if !station.variables.contains(&row.variable) {
            return Err(format!(
                "ANA station {} does not declare {}",
                station.station.station_code,
                row.variable.code()
            ));
        }
        let source_time = Sao_Paulo
            .from_local_datetime(&row.time)
            .single()
            .ok_or_else(|| format!("ambiguous or nonexistent ANA source time {}", row.time))?;
        let local = source_time.with_timezone(&zone).naive_local();
        let end = if local.minute() == 0 {
            local
        } else {
            local
                .checked_add_signed(Duration::minutes(i64::from(60 - local.minute())))
                .ok_or("ANA hourly bucket is outside the timestamp range")?
        };
        let (sum, count, last) = buckets
            .entry((row.variable, end))
            .or_insert((0.0, 0u64, 0.0));
        *sum += row.value;
        *count += 1;
        *last = row.value;
    }
    let mut values = AnaHourly::new();
    for ((variable, at), (sum, count, last)) in buckets {
        let method = match variable {
            AnaVariable::Rain => methods.rain,
            AnaVariable::Level => methods.level,
            AnaVariable::Flow => methods.flow,
        };
        let value = match method {
            Aggregation::Sum => sum,
            Aggregation::Mean => sum / count as f64,
            Aggregation::Last => last,
        };
        if !value.is_finite() {
            return Err(format!("nonfinite ANA aggregate at {at}"));
        }
        let value = match variable {
            AnaVariable::Rain => ObservationValue::RainMm(value),
            AnaVariable::Flow => ObservationValue::FlowM3s(value),
            AnaVariable::Level => {
                let rounded = value.round_ties_even();
                if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&rounded) {
                    return Err(format!(
                        "ANA level outside integer centimetre range at {at}"
                    ));
                }
                ObservationValue::LevelCm(rounded as i32)
            }
        };
        values.entry(variable).or_default().push((at, value));
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ana_xml_csv_and_hourly_values_agree() {
        let stations = read_ana_inventory(
            &b"provider_code,station_code,station_name,latitude,longitude,observed_variables\nANA,00123,Station,0,0,\"rain,flow,level\"\ninmet,A01,Other,0,0,rain\n"[..],
        ).unwrap();
        let [station] = stations.as_slice() else {
            panic!("expected one ANA station");
        };
        let xml = br#"<NewDataSet xmlns="urn:ana">
            <DadosHidrometereologicos><CodEstacao>00123</CodEstacao><DataHora>2026-07-20T12:03:44</DataHora><Chuva>1,25</Chuva><Vazao>5</Vazao><Nivel>100</Nivel></DadosHidrometereologicos>
            <DadosHidrometereologicos><CodEstacao>123</CodEstacao><DataHora>2026-07-20T16:00:00Z</DataHora><Chuva>0</Chuva><Vazao>7</Vazao><Nivel>101</Nivel></DadosHidrometereologicos>
            <DadosHidrometereologicos><CodEstacao>123</CodEstacao><DataHora>2026-07-20T13:05:00</DataHora><Chuva>-</Chuva><Vazao>broken</Vazao></DadosHidrometereologicos>
            <DadosHidrometereologicos><CodEstacao>555</CodEstacao><DataHora>2026-07-20T13:00:00</DataHora><Chuva>99</Chuva></DadosHidrometereologicos>
        </NewDataSet>"#;
        let rows = parse_ana_xml(xml, "123", &station.variables).unwrap();
        assert_eq!(rows.len(), 6);
        let csv = ana_csv("123", &rows).unwrap();
        let read = read_ana_csv(csv.as_slice()).unwrap().remove("123").unwrap();
        assert_eq!(read, rows);
        let hourly =
            aggregate_ana(read, station, chrono_tz::UTC, AnaAggregation::default()).unwrap();
        let label = "2026-07-20T16:00:00".parse::<NaiveDateTime>().unwrap();
        assert_eq!(
            hourly[&AnaVariable::Rain],
            [(label, ObservationValue::RainMm(1.25))]
        );
        assert_eq!(
            hourly[&AnaVariable::Level],
            [(label, ObservationValue::LevelCm(100))]
        );
        assert_eq!(
            hourly[&AnaVariable::Flow],
            [(label, ObservationValue::FlowM3s(6.0))]
        );
        assert!(parse_ana_xml(b"<html/>", "123", &station.variables).is_err());
    }
}
