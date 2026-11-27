//! Matched synthetic storage workloads; not full operational performance qualification.
use super::super::{LockedDb, Probe, Result, ZONE, fixture, query, signal};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use mgb_ops::store::{
    IntervalAxis, ObservationBatch, ObservationSample, ObservationSelection, ObservationSeries,
    ObservationStore, ObservationValue, Station, TimeInterval,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Instant};
use turso::{Value, transaction::TransactionBehavior};

const STATIONS: usize = 440;
const HOURS: usize = 362;
const SELECTED: usize = 32;
const REPEATS: usize = 5;
type Samples = Vec<(String, String, Option<f64>)>;

#[derive(Debug, Deserialize, Serialize)]
struct Measurement {
    backend: String,
    samples: usize,
    ingestion_ms: f64,
    cold_open_read_ms: f64,
    warm_read_median_ms: f64,
    rolling_update_ms: f64,
    historical_read_ms: f64,
    peak_rss_kib: Option<u64>,
    selected_sha256: String,
}

fn start() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 7, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn at(hour: usize) -> NaiveDateTime {
    start() + Duration::hours(hour as i64)
}
fn id(station: usize) -> String {
    format!("s{station:04}")
}

fn batch(first: usize, last: usize, stations: usize) -> ObservationBatch {
    let mut batch = ObservationBatch::default();
    for station in 0..stations {
        let id = id(station);
        batch.stations.push(Station {
            station_id: id.clone(),
            provider_code: "fixture".into(),
            station_code: id.clone(),
            name: id.clone(),
            latitude: Some(-30.0 + station as f64 / 1000.0),
            longitude: Some(-51.0),
        });
        batch.series.push(ObservationSeries {
            series_id: id.clone(),
            station_id: id.clone(),
            variable: "rain".into(),
            state: "raw".into(),
            unit: "mm".into(),
            interval_seconds: Some(3600),
            created_at: start(),
        });
        for hour in first..last {
            batch.samples.push(ObservationSample {
                series_id: id.clone(),
                observed_at: at(hour),
                value: (hour % 23 != 0).then_some(ObservationValue::RainMm(
                    ((station * 17 + hour * 7) % 1000) as f64 / 8.0,
                )),
            });
        }
    }
    batch
}

fn axis() -> IntervalAxis {
    IntervalAxis {
        time_zone: ZONE.into(),
        bounds: (HOURS - 72..HOURS)
            .map(|hour| TimeInterval {
                start: at(hour) - Duration::hours(1),
                end: at(hour),
            })
            .collect(),
    }
}

fn canonical(samples: &[ObservationSample]) -> Samples {
    let mut values = samples
        .iter()
        .map(|sample| {
            (
                sample.series_id.clone(),
                sample.observed_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                sample.value.map(|value| match value {
                    ObservationValue::RainMm(value) => value,
                    _ => panic!("rainfall fixture expected"),
                }),
            )
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    values
}

fn expected(batch: &ObservationBatch) -> Samples {
    canonical(
        &batch
            .samples
            .iter()
            .filter(|sample| {
                sample.series_id < id(SELECTED) && sample.observed_at >= at(HOURS - 72)
            })
            .cloned()
            .collect::<Vec<_>>(),
    )
}

fn checksum(values: &Samples) -> String {
    let mut hash = Sha256::new();
    for (id, at, value) in values {
        hash.update(id.as_bytes());
        hash.update(at.as_bytes());
        hash.update([u8::from(value.is_some())]);
        if let Some(value) = value {
            hash.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

fn rss() -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
}

fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn read_parquet(store: &ObservationStore) -> Result<Samples> {
    let selection = ObservationSelection {
        station_ids: (0..SELECTED).map(id).collect(),
        ..Default::default()
    };
    Ok(canonical(
        &store.read_selection(&selection, &axis(), &[])?.0.samples,
    ))
}

async fn read_turso(db: &LockedDb) -> Result<Samples> {
    let rows = query(
        &db.conn,
        r#"
SELECT s.series_id,s.observed_at,s.value FROM sample_version s
WHERE s.series_id<?1 AND s.observed_at>=?2 AND s.observed_at<=?3
 AND s.revision=(SELECT MAX(v.revision) FROM sample_version v
  WHERE v.series_id=s.series_id AND v.observed_at=s.observed_at AND v.revision<=1)
ORDER BY s.series_id,s.observed_at
"#,
        (
            id(SELECTED),
            at(HOURS - 72).format("%Y-%m-%dT%H:%M:%S").to_string(),
            at(HOURS - 1).format("%Y-%m-%dT%H:%M:%S").to_string(),
        ),
    )
    .await?;
    rows.into_iter()
        .map(|row| {
            let value = match &row[2] {
                Value::Null => None,
                Value::Real(value) => Some(*value),
                Value::Integer(value) => Some(*value as f64),
                _ => return Err("invalid rainfall type".into()),
            };
            Ok((
                row[0].as_text().ok_or("invalid series")?.clone(),
                row[1].as_text().ok_or("invalid timestamp")?.clone(),
                value,
            ))
        })
        .collect()
}

async fn ingest_turso(db: &mut LockedDb, batch: &ObservationBatch, revision: i64) -> Result<()> {
    let tx = db
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .await?;
    tx.execute(
        "INSERT INTO revision VALUES (?1,?2,'2026-07-01T00:00:00','fixture')",
        (revision, format!("batch-{revision}")),
    )
    .await?;
    for station in &batch.stations {
        tx.execute(
            "INSERT OR IGNORE INTO station VALUES (?1)",
            [station.station_id.as_str()],
        )
        .await?;
        tx.execute(
            "INSERT INTO station_version VALUES (?1,?2,?3,?4,?5)",
            (
                station.station_id.as_str(),
                revision,
                station.name.as_str(),
                station.latitude,
                station.longitude,
            ),
        )
        .await?;
        tx.execute(
            "INSERT OR IGNORE INTO series VALUES (?1,?1,'rain','mm',3600)",
            [station.station_id.as_str()],
        )
        .await?;
    }
    let mut statement = tx
        .prepare("INSERT INTO sample_version VALUES (?1,?2,?3,?4,NULL)")
        .await?;
    for sample in &batch.samples {
        let value = sample.value.map(|value| match value {
            ObservationValue::RainMm(value) => value,
            _ => panic!("rainfall fixture expected"),
        });
        statement
            .execute((
                sample.series_id.as_str(),
                sample.observed_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                revision,
                value,
            ))
            .await?;
    }
    drop(statement);
    tx.commit().await?;
    Ok(())
}

pub fn child(root: &Path, mode: &str) -> Result<()> {
    let data = batch(0, HOURS, STATIONS);
    let expected = expected(&data);
    let mut update = batch(HOURS, HOURS + 24, 1);
    update.stations.clear();
    update.series.clear();
    let mut measure = Measurement {
        backend: mode.strip_prefix("benchmark-").unwrap().into(),
        samples: data.samples.len(),
        ingestion_ms: 0.0,
        cold_open_read_ms: 0.0,
        warm_read_median_ms: 0.0,
        rolling_update_ms: 0.0,
        historical_read_ms: 0.0,
        peak_rss_kib: None,
        selected_sha256: checksum(&expected),
    };
    if mode == "benchmark-parquet" {
        let path = root.join("observations");
        let began = Instant::now();
        let mut store = ObservationStore::create(&path, ZONE)?;
        store.write_batch(&data)?;
        measure.ingestion_ms = milliseconds(began);
        drop(store);
        let began = Instant::now();
        let store = ObservationStore::open_read_only(&path)?;
        let actual = read_parquet(&store)?;
        measure.cold_open_read_ms = milliseconds(began);
        assert_eq!(actual, expected);
        let mut times = Vec::new();
        for _ in 0..9 {
            let began = Instant::now();
            let actual = read_parquet(&store)?;
            times.push(milliseconds(began));
            assert_eq!(actual, expected);
        }
        measure.warm_read_median_ms = median(times);
        drop(store);
        let began = Instant::now();
        let mut store = ObservationStore::open_writable(&path)?;
        store.write_batch(&update)?;
        measure.rolling_update_ms = milliseconds(began);
        let began = Instant::now();
        let actual = read_parquet(&store)?;
        measure.historical_read_ms = milliseconds(began);
        assert_eq!(actual, expected);
    } else if mode == "benchmark-turso" {
        pollster::block_on(async {
            let began = Instant::now();
            let mut db = LockedDb::open(root, false).await?;
            db.initialize().await?;
            ingest_turso(&mut db, &data, 1).await?;
            measure.ingestion_ms = milliseconds(began);
            drop(db);
            let began = Instant::now();
            let db = LockedDb::open(root, false).await?;
            let actual = read_turso(&db).await?;
            measure.cold_open_read_ms = milliseconds(began);
            assert_eq!(actual, expected);
            let mut times = Vec::new();
            for _ in 0..9 {
                let began = Instant::now();
                let actual = read_turso(&db).await?;
                times.push(milliseconds(began));
                assert_eq!(actual, expected);
            }
            measure.warm_read_median_ms = median(times);
            drop(db);
            let began = Instant::now();
            let mut db = LockedDb::open(root, false).await?;
            ingest_turso(&mut db, &update, 2).await?;
            measure.rolling_update_ms = milliseconds(began);
            let began = Instant::now();
            let actual = read_turso(&db).await?;
            measure.historical_read_ms = milliseconds(began);
            assert_eq!(actual, expected);
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?;
    } else {
        return Err("unknown benchmark backend".into());
    }
    measure.peak_rss_kib = rss();
    fs::write(
        root.join("measurement.json"),
        serde_json::to_vec_pretty(&measure)?,
    )?;
    signal("MEASURED")
}

pub fn run() -> Result<()> {
    if cfg!(debug_assertions) {
        return Err("run the performance comparison with cargo test --release".into());
    }
    let root = fixture()?;
    let mut measurements = Vec::new();
    for repeat in 0..REPEATS {
        let modes = if repeat % 2 == 0 {
            ["benchmark-parquet", "benchmark-turso"]
        } else {
            ["benchmark-turso", "benchmark-parquet"]
        };
        for mode in modes {
            let case = root.path().join(format!("{repeat}-{mode}"));
            fs::create_dir(&case)?;
            let mut child = Probe::spawn(&case, mode)?;
            child.expect("MEASURED")?;
            child.finish()?;
            let measure: Measurement =
                serde_json::from_slice(&fs::read(case.join("measurement.json"))?)?;
            assert_eq!(measure.samples, STATIONS * HOURS);
            measurements.push(measure);
        }
    }
    assert!(
        measurements
            .iter()
            .all(|m| m.selected_sha256 == measurements[0].selected_sha256)
    );
    for backend in ["parquet", "turso"] {
        let rows = measurements
            .iter()
            .filter(|m| m.backend == backend)
            .collect::<Vec<_>>();
        println!(
            "{backend}: ingestion {:.2} ms, cold open/read {:.2} ms, warm read {:.2} ms, rolling update {:.2} ms, historical read {:.2} ms; peak RSS {:?} KiB",
            median(rows.iter().map(|m| m.ingestion_ms).collect()),
            median(rows.iter().map(|m| m.cold_open_read_ms).collect()),
            median(rows.iter().map(|m| m.warm_read_median_ms).collect()),
            median(rows.iter().map(|m| m.rolling_update_ms).collect()),
            median(rows.iter().map(|m| m.historical_read_ms).collect()),
            rows.iter().filter_map(|m| m.peak_rss_kib).max()
        );
    }
    fs::write(
        "runs/sqlite-qualification/benchmark.json",
        serde_json::to_vec_pretty(&measurements)?,
    )?;
    Ok(())
}
