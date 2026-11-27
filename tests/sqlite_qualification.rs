//! Disposable qualification, not an operational registry or production schema.
//! Run with --features sqlite-qualification --test sqlite_qualification -- --ignored.
#![cfg(feature = "sqlite-qualification")]

use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use turso::{Connection, Database, IntoParams, Value, transaction::TransactionBehavior};

mod support {
    pub mod sqlite_benchmark;
}

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
const ZONE: &str = "America/Sao_Paulo";
const LABEL: &str = "2026-07-20T00:00:00";
const CHILD_MODE: &str = "MGB_SQLITE_QUALIFICATION_CHILD";
const CHILD_ROOT: &str = "MGB_SQLITE_QUALIFICATION_ROOT";
const DEADLINE: Duration = Duration::from_secs(30);

const SCHEMA: &str = r#"
CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE revision (
 id INTEGER PRIMARY KEY, acquisition_id TEXT NOT NULL UNIQUE,
 created_at TEXT NOT NULL, source_sha256 TEXT NOT NULL);
CREATE TABLE station (id TEXT PRIMARY KEY);
CREATE TABLE station_version (
 station_id TEXT NOT NULL REFERENCES station(id), revision INTEGER NOT NULL REFERENCES revision(id),
 name TEXT NOT NULL, latitude REAL, longitude REAL, PRIMARY KEY(station_id, revision));
CREATE TABLE series (
 id TEXT PRIMARY KEY, station_id TEXT NOT NULL REFERENCES station(id),
 variable TEXT NOT NULL CHECK(variable IN ('rain','flow','level')),
 unit TEXT NOT NULL, interval_seconds INTEGER,
 CHECK((variable='rain' AND unit='mm' AND interval_seconds=3600) OR
       (variable='flow' AND unit='m3/s' AND interval_seconds IS NULL) OR
       (variable='level' AND unit='cm' AND interval_seconds IS NULL)));
CREATE TABLE sample_version (
 series_id TEXT NOT NULL REFERENCES series(id), observed_at TEXT NOT NULL,
 revision INTEGER NOT NULL REFERENCES revision(id), value REAL, level_cm INTEGER,
 PRIMARY KEY(series_id, observed_at, revision), CHECK(value IS NULL OR level_cm IS NULL));
CREATE TABLE replacement_version (
 id TEXT NOT NULL, revision INTEGER NOT NULL REFERENCES revision(id),
 station_id TEXT NOT NULL REFERENCES station(id), position INTEGER NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('missing','value')), value REAL,
 start_at TEXT NOT NULL, end_at TEXT, PRIMARY KEY(id, revision),
 CHECK((kind='missing' AND value IS NULL) OR (kind='value' AND value IS NOT NULL)));
CREATE TABLE flag_version (
 series_id TEXT NOT NULL REFERENCES series(id), observed_at TEXT NOT NULL,
 revision INTEGER NOT NULL REFERENCES revision(id), excluded INTEGER NOT NULL CHECK(excluded IN (0,1)),
 PRIMARY KEY(series_id, observed_at, revision));
CREATE TABLE offset_version (
 station_id TEXT NOT NULL REFERENCES station(id), revision INTEGER NOT NULL REFERENCES revision(id),
 mini_id INTEGER NOT NULL CHECK(mini_id>0), offset_m REAL NOT NULL, PRIMARY KEY(station_id,revision));
CREATE TABLE reference_version (
 station_id TEXT NOT NULL REFERENCES station(id), revision INTEGER NOT NULL REFERENCES revision(id),
 kind TEXT NOT NULL, level_cm REAL NOT NULL, PRIMARY KEY(station_id,kind,revision));
CREATE TABLE inventory_version (
 station_id TEXT NOT NULL REFERENCES station(id), variable TEXT NOT NULL,
 revision INTEGER NOT NULL REFERENCES revision(id), PRIMARY KEY(station_id,variable,revision));
CREATE TABLE coverage_version (
 station_id TEXT NOT NULL REFERENCES station(id), day TEXT NOT NULL, variables_key TEXT NOT NULL,
 revision INTEGER NOT NULL REFERENCES revision(id), PRIMARY KEY(station_id,day,variables_key,revision));
CREATE TABLE partial_coverage_version (
 station_id TEXT NOT NULL REFERENCES station(id), day TEXT NOT NULL, through_hour INTEGER NOT NULL,
 revision INTEGER NOT NULL REFERENCES revision(id), PRIMARY KEY(station_id,day,revision));
"#;

// Fields drop in declaration order: every database handle precedes the OS guard.
struct LockedDb {
    conn: Connection,
    _database: Database,
    _lock: RegistryLock,
}

struct RegistryLock(File);

impl Drop for RegistryLock {
    fn drop(&mut self) {
        // A concurrently spawned child can briefly inherit a close-on-exec file
        // before exec. Explicitly release the lock after the DB fields dropped,
        // rather than waiting for every inherited descriptor to close.
        let _ = self.0.unlock();
    }
}

impl LockedDb {
    async fn open(root: &Path, read_only: bool) -> Result<Self> {
        let guard = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("registry.lock"))?;
        guard.try_lock()?;
        let guard = RegistryLock(guard);
        let path = root.join("registry.sqlite");
        let db = turso::Builder::new_local(path.to_str().ok_or("database path is not UTF-8")?)
            .read_only(read_only)
            .build()
            .await?;
        let conn = db.connect()?;
        conn.busy_timeout(Duration::from_millis(100))?;
        conn.set_query_timeout(Duration::from_secs(10))?;
        conn.execute("PRAGMA foreign_keys=ON", ()).await?;
        conn.execute("PRAGMA synchronous=FULL", ()).await?;
        assert_eq!(
            scalar(&conn, "PRAGMA foreign_keys", ()).await?,
            Value::Integer(1)
        );
        assert_eq!(
            scalar(&conn, "PRAGMA synchronous", ()).await?,
            Value::Integer(2)
        );
        assert_eq!(
            scalar(&conn, "PRAGMA journal_mode", ()).await?,
            Value::Text("wal".into())
        );
        Ok(Self {
            conn,
            _database: db,
            _lock: guard,
        })
    }

    async fn initialize(&self) -> Result<()> {
        self.conn.execute_batch(SCHEMA).await?;
        self.conn
            .execute("INSERT INTO metadata VALUES ('time_zone', ?1)", [ZONE])
            .await?;
        Ok(())
    }
}

async fn scalar(conn: &Connection, sql: &str, params: impl IntoParams) -> Result<Value> {
    let mut rows = conn.query(sql, params).await?;
    let value = rows
        .next()
        .await?
        .ok_or("expected scalar row")?
        .get_value(0)?;
    assert!(rows.next().await?.is_none(), "multiple scalar rows");
    Ok(value)
}

async fn query(conn: &Connection, sql: &str, params: impl IntoParams) -> Result<Vec<Vec<Value>>> {
    let mut rows = conn.query(sql, params).await?;
    let mut values = Vec::new();
    while let Some(row) = rows.next().await? {
        values.push(
            (0..row.column_count())
                .map(|i| row.get_value(i))
                .collect::<turso::Result<_>>()?,
        );
    }
    Ok(values)
}

async fn stage_registration(
    conn: &Connection,
    revision: i64,
    acquisition: &str,
    station: &str,
    signal_early: bool,
) -> Result<()> {
    conn.execute(
        "INSERT INTO revision VALUES (?1,?2,?3,?4)",
        (revision, acquisition, LABEL, "a".repeat(64)),
    )
    .await?;
    if signal_early {
        signal("READY")?;
    }
    conn.execute("INSERT OR IGNORE INTO station VALUES (?1)", [station])
        .await?;
    conn.execute(
        "INSERT INTO station_version VALUES (?1,?2,?3,NULL,NULL)",
        (station, revision, format!("Station {revision}")),
    )
    .await?;
    for (variable, unit, interval) in [
        ("rain", "mm", Some(3600i64)),
        ("flow", "m3/s", None),
        ("level", "cm", None),
    ] {
        conn.execute(
            "INSERT OR IGNORE INTO series VALUES (?1,?2,?3,?4,?5)",
            (
                format!("{station}-{variable}"),
                station,
                variable,
                unit,
                interval,
            ),
        )
        .await?;
    }
    // An empty successful response still commits provenance and complete coverage.
    if station != "empty" {
        for (at, value) in [
            (LABEL, Some(if revision == 1 { 0.0 } else { 8.125 })),
            ("2026-07-20T01:00:00", None),
            ("2026-07-20T02:00:00", Some(1.23456789012345)),
        ] {
            conn.execute(
                "INSERT INTO sample_version VALUES (?1,?2,?3,?4,NULL)",
                (format!("{station}-rain"), at, revision, value),
            )
            .await?;
        }
        conn.execute(
            "INSERT INTO sample_version VALUES (?1,?2,?3,NULL,?4)",
            (format!("{station}-level"), LABEL, revision, -14572i64),
        )
        .await?;
        conn.execute(
            "INSERT INTO sample_version VALUES (?1,?2,?3,?4,NULL)",
            (format!("{station}-flow"), LABEL, revision, 3.125f64),
        )
        .await?;
    }
    for (id, position, kind, value) in [
        ("exclude", 0i64, "missing", None),
        ("zero", 1, "value", Some(0.0)),
    ] {
        conn.execute(
            "INSERT INTO replacement_version VALUES (?1,?2,?3,?4,?5,?6,?7,NULL)",
            (
                format!("{station}-{id}"),
                revision,
                station,
                position,
                kind,
                value,
                LABEL,
            ),
        )
        .await?;
    }
    conn.execute(
        "INSERT INTO flag_version VALUES (?1,?2,?3,?4)",
        (
            format!("{station}-rain"),
            LABEL,
            revision,
            i64::from(revision == 1),
        ),
    )
    .await?;
    conn.execute(
        "INSERT INTO offset_version VALUES (?1,?2,42,?3)",
        (station, revision, revision as f64 * -0.125),
    )
    .await?;
    conn.execute(
        "INSERT INTO reference_version VALUES (?1,?2,'attention',?3)",
        (station, revision, 120.5f64 + revision as f64),
    )
    .await?;
    conn.execute(
        "INSERT INTO inventory_version VALUES (?1,'rain',?2)",
        (station, revision),
    )
    .await?;
    conn.execute(
        "INSERT INTO coverage_version VALUES (?1,'2026-07-19','flow,level,rain',?2)",
        (station, revision),
    )
    .await?;
    conn.execute(
        "INSERT INTO partial_coverage_version VALUES (?1,'2026-07-20',?2,?3)",
        (station, revision.min(23), revision),
    )
    .await?;
    Ok(())
}

async fn register(
    db: &mut LockedDb,
    revision: i64,
    acquisition: &str,
    station: &str,
) -> Result<i64> {
    let found = query(
        &db.conn,
        "SELECT id FROM revision WHERE acquisition_id=?1",
        [acquisition],
    )
    .await?;
    if let Some(row) = found.first() {
        return row[0]
            .as_integer()
            .copied()
            .ok_or_else(|| "invalid revision type".into());
    }
    let tx = db
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .await?;
    if let Err(error) = stage_registration(&tx, revision, acquisition, station, false).await {
        tx.rollback().await?;
        return Err(error);
    }
    tx.commit().await?;
    Ok(revision)
}

async fn snapshot(conn: &Connection, revision: i64) -> Result<Vec<Vec<Value>>> {
    query(
        conn,
        r#"
SELECT s.observed_at,s.value,s.level_cm FROM sample_version s
WHERE s.series_id='s-rain' AND s.revision=(
 SELECT MAX(v.revision) FROM sample_version v
 WHERE v.series_id=s.series_id AND v.observed_at=s.observed_at AND v.revision<=?1)
ORDER BY s.observed_at
"#,
        [revision],
    )
    .await
}

fn fixture() -> Result<tempfile::TempDir> {
    fs::create_dir_all("runs/sqlite-qualification")?;
    Ok(tempfile::tempdir_in("runs/sqlite-qualification")?)
}

#[test]
#[ignore = "explicit SQLite qualification; disposable databases under runs/"]
fn sqlite_values_and_revisions() -> Result<()> {
    let dir = fixture()?;
    pollster::block_on(async {
        let mut db = LockedDb::open(dir.path(), false).await?;
        db.initialize().await?;
        assert_eq!(register(&mut db, 1, "first", "s").await?, 1);
        let original = snapshot(&db.conn, 1).await?;
        assert_eq!(original.len(), 3);
        assert_eq!(original[0][1], Value::Real(0.0));
        assert_eq!(original[1][1], Value::Null);
        assert_eq!(original[2][1], Value::Real(1.23456789012345));
        assert_eq!(register(&mut db, 2, "first", "s").await?, 1);
        assert_eq!(
            scalar(&db.conn, "SELECT COUNT(*) FROM revision", ()).await?,
            Value::Integer(1)
        );
        assert_eq!(register(&mut db, 2, "revised", "s").await?, 2);
        assert_eq!(snapshot(&db.conn, 1).await?, original);
        assert_eq!(snapshot(&db.conn, 2).await?[0][1], Value::Real(8.125));
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT value FROM metadata WHERE key='time_zone'",
                ()
            )
            .await?,
            Value::Text(ZONE.into())
        );
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT level_cm FROM sample_version WHERE series_id='s-level' AND revision=1",
                ()
            )
            .await?,
            Value::Integer(-14572)
        );
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT value FROM sample_version WHERE series_id='s-flow' AND revision=1",
                ()
            )
            .await?,
            Value::Real(3.125)
        );
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT name FROM station_version WHERE revision=1",
                ()
            )
            .await?,
            Value::Text("Station 1".into())
        );
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT name FROM station_version WHERE revision=2",
                ()
            )
            .await?,
            Value::Text("Station 2".into())
        );
        assert_eq!(
            query(&db.conn,"SELECT kind,value,end_at FROM replacement_version WHERE revision=1 ORDER BY position",()).await?,
            vec![vec![Value::Text("missing".into()),Value::Null,Value::Null],
                 vec![Value::Text("value".into()),Value::Real(0.0),Value::Null]]
        );
        for (sql, expected) in [
            (
                "SELECT excluded FROM flag_version WHERE revision=1",
                Value::Integer(1),
            ),
            (
                "SELECT excluded FROM flag_version WHERE revision=2",
                Value::Integer(0),
            ),
            (
                "SELECT offset_m FROM offset_version WHERE revision=1",
                Value::Real(-0.125),
            ),
            (
                "SELECT level_cm FROM reference_version WHERE revision=1",
                Value::Real(121.5),
            ),
            (
                "SELECT through_hour FROM partial_coverage_version WHERE revision=1",
                Value::Integer(1),
            ),
            (
                "SELECT through_hour FROM partial_coverage_version WHERE revision=2",
                Value::Integer(2),
            ),
            (
                "SELECT created_at FROM revision WHERE id=1",
                Value::Text(LABEL.into()),
            ),
            (
                "SELECT source_sha256 FROM revision WHERE id=1",
                Value::Text("a".repeat(64)),
            ),
        ] {
            assert_eq!(scalar(&db.conn, sql, ()).await?, expected, "{sql}");
        }
        Ok(())
    })
}

#[test]
#[ignore = "explicit SQLite qualification; disposable databases under runs/"]
fn sqlite_registration_rollback_and_constraints() -> Result<()> {
    let dir = fixture()?;
    pollster::block_on(async {
        let mut db = LockedDb::open(dir.path(), false).await?;
        db.initialize().await?;
        register(&mut db, 1, "first", "s").await?;
        let tx = db
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .await?;
        stage_registration(&tx, 2, "failed", "s", false).await?;
        assert!(
            tx.execute(
                "INSERT INTO sample_version VALUES ('orphan',?1,2,1,NULL)",
                [LABEL]
            )
            .await
            .is_err()
        );
        tx.rollback().await?;
        assert_consistent(&db.conn, 1).await?;
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT COUNT(*) FROM coverage_version WHERE revision=2",
                ()
            )
            .await?,
            Value::Integer(0)
        );
        {
            let tx = db.conn.transaction().await?;
            stage_registration(&tx, 2, "dropped", "s", false).await?;
        }
        assert_consistent(&db.conn, 1).await?;
        register(&mut db, 2, "empty-response", "empty").await?;
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT COUNT(*) FROM sample_version WHERE series_id LIKE 'empty-%'",
                ()
            )
            .await?,
            Value::Integer(0)
        );
        assert_eq!(
            scalar(
                &db.conn,
                "SELECT COUNT(*) FROM coverage_version WHERE station_id='empty'",
                ()
            )
            .await?,
            Value::Integer(1)
        );
        assert!(
            db.conn
                .execute("INSERT INTO series VALUES ('bad','s','rain','cm',3600)", ())
                .await
                .is_err()
        );
        assert!(
            db.conn
                .execute("INSERT INTO offset_version VALUES ('s',2,0,1)", ())
                .await
                .is_err()
        );
        assert!(
            db.conn
                .execute(
                    "INSERT INTO sample_version VALUES ('s-rain',?1,999,1,NULL)",
                    [LABEL]
                )
                .await
                .is_err()
        );
        assert!(
            query(&db.conn, "PRAGMA foreign_key_check", ())
                .await?
                .is_empty()
        );
        Ok(())
    })
}

#[test]
#[ignore = "explicit SQLite qualification; disposable databases under runs/"]
fn sqlite_read_only_and_extensions_disabled() -> Result<()> {
    let dir = fixture()?;
    pollster::block_on(async {
        {
            let mut db = LockedDb::open(dir.path(), false).await?;
            db.initialize().await?;
            register(&mut db, 1, "first", "s").await?;
            let error = match db
                .conn
                .query(
                    "SELECT load_extension('mgb-qualification-must-not-load')",
                    (),
                )
                .await
            {
                Err(error) => error,
                Ok(mut rows) => rows
                    .next()
                    .await
                    .expect_err("extension loading must be disabled"),
            };
            let message = error.to_string().to_ascii_lowercase();
            assert!(
                message.contains("disabled") || message.contains("not authorized"),
                "{message}"
            );
        }
        let db = LockedDb::open(dir.path(), true).await?;
        assert_eq!(snapshot(&db.conn, 1).await?.len(), 3);
        assert!(db.conn.execute("DELETE FROM revision", ()).await.is_err());
        Ok(())
    })
}

async fn assert_consistent(conn: &Connection, expected: i64) -> Result<()> {
    assert_eq!(
        scalar(conn, "SELECT MAX(id) FROM revision", ()).await?,
        Value::Integer(expected)
    );
    for (table, per_revision) in [
        ("station_version", 1),
        ("sample_version", 5),
        ("replacement_version", 2),
        ("flag_version", 1),
        ("offset_version", 1),
        ("reference_version", 1),
        ("inventory_version", 1),
        ("coverage_version", 1),
        ("partial_coverage_version", 1),
    ] {
        assert_eq!(
            scalar(conn, &format!("SELECT COUNT(*) FROM {table}"), ()).await?,
            Value::Integer(expected * per_revision),
            "{table}"
        );
    }
    assert!(
        query(conn, "PRAGMA foreign_key_check", ())
            .await?
            .is_empty()
    );
    assert_eq!(
        scalar(conn, "PRAGMA integrity_check", ()).await?,
        Value::Text("ok".into())
    );
    Ok(())
}

fn signal(message: &str) -> Result<()> {
    println!("{message}");
    std::io::stdout().flush()?;
    Ok(())
}

struct Probe {
    child: Child,
    lines: Receiver<String>,
    reader: Option<JoinHandle<()>>,
}

impl Probe {
    fn spawn(root: &Path, mode: &str) -> Result<Self> {
        let mut log = File::create(root.join(format!("{mode}.stdout.log")))?;
        let mut child = Command::new(std::env::current_exe()?)
            .args(["--ignored", "--exact", "sqlite_child", "--nocapture"])
            .env(CHILD_MODE, mode)
            .env(CHILD_ROOT, root.canonicalize()?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(File::create(root.join(format!("{mode}.stderr.log")))?)
            .spawn()?;
        let stdout = child.stdout.take().ok_or("child stdout missing")?;
        let (send, lines) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let _ = writeln!(log, "{line}");
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            lines,
            reader: Some(reader),
        })
    }

    fn expect(&self, marker: &str) -> Result<()> {
        let deadline = Instant::now() + DEADLINE;
        let mut output = String::new();
        loop {
            let line = self
                .lines
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|error| {
                    format!("child did not report {marker}: {error}; output: {output}")
                })?;
            if line == marker {
                return Ok(());
            }
            output.push_str(&line);
            output.push('\n');
        }
    }

    fn finish(&mut self) -> Result<()> {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(status) = self.child.try_wait()? {
                assert!(status.success(), "child failed: {status}");
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("child exit deadline exceeded".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        assert!(!self.child.wait()?.success());
        Ok(())
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[test]
#[ignore = "process-kill qualification; does not establish physical power-loss durability"]
fn sqlite_process_lock_and_crash_recovery() -> Result<()> {
    let root = fixture()?;
    let mut db = pollster::block_on(LockedDb::open(root.path(), false))?;
    pollster::block_on(db.initialize())?;
    pollster::block_on(register(&mut db, 1, "first", "s"))?;
    for mode in ["busy", "busy-readonly"] {
        let mut busy = Probe::spawn(root.path(), mode)?;
        busy.expect("BUSY")?;
        busy.finish()?;
    }
    drop(db);
    let mut reopened = Probe::spawn(root.path(), "reopen")?;
    reopened.expect("REOPENED")?;
    reopened.finish()?;
    for mode in ["writing", "uncommitted", "committing", "committed"] {
        for repeat in 0..4 {
            let case = root.path().join(format!("{mode}-{repeat}"));
            fs::create_dir(&case)?;
            pollster::block_on(async {
                let mut db = LockedDb::open(&case, false).await?;
                db.initialize().await?;
                register(&mut db, 1, "first", "s").await?;
                Ok::<_, Box<dyn Error + Send + Sync>>(())
            })?;
            let mut child = Probe::spawn(&case, mode)?;
            child.expect("READY")?;
            child.kill()?;
            pollster::block_on(async {
                let db = LockedDb::open(&case, false).await?;
                let latest = scalar(&db.conn, "SELECT MAX(id) FROM revision", ()).await?;
                let expected = match mode {
                    "committed" => 2,
                    "committing" => *latest.as_integer().ok_or("invalid revision type")?,
                    _ => 1,
                };
                assert!((1..=2).contains(&expected));
                assert_consistent(&db.conn, expected).await?;
                assert_eq!(snapshot(&db.conn, 1).await?[0][1], Value::Real(0.0));
                Ok::<_, Box<dyn Error + Send + Sync>>(())
            })?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "subprocess fixture; invoked only by qualification parents"]
fn sqlite_child() -> Result<()> {
    let Some(mode) = std::env::var_os(CHILD_MODE) else {
        return Ok(());
    };
    let root = PathBuf::from(std::env::var_os(CHILD_ROOT).ok_or("missing child root")?);
    let mode = mode.to_str().ok_or("invalid child mode")?;
    if mode.starts_with("benchmark-") {
        return support::sqlite_benchmark::child(&root, mode);
    }
    pollster::block_on(async {
        if mode == "busy" || mode == "busy-readonly" {
            let error = match LockedDb::open(&root, mode == "busy-readonly").await {
                Ok(_) => return Err("competing process opened guarded database".into()),
                Err(error) => error,
            };
            assert!(
                error
                    .downcast_ref::<std::fs::TryLockError>()
                    .is_some_and(|error| matches!(error, std::fs::TryLockError::WouldBlock)),
                "unexpected lock error: {error}"
            );
            return signal("BUSY");
        }
        let mut db = LockedDb::open(&root, false).await?;
        if mode == "reopen" {
            assert_consistent(&db.conn, 1).await?;
            return signal("REOPENED");
        }
        let tx = db
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .await?;
        stage_registration(&tx, 2, "second", "s", mode == "writing").await?;
        match mode {
            "committed" => {
                tx.commit().await?;
                signal("READY")?;
            }
            "committing" => {
                signal("READY")?;
                tx.commit().await?;
            }
            "uncommitted" => {
                signal("READY")?;
            }
            "writing" => {}
            _ => return Err("unknown child mode".into()),
        }
        let mut byte = [0];
        std::io::stdin().read_exact(&mut byte)?;
        Err("parent must terminate crash fixture".into())
    })
}

#[test]
#[ignore = "explicit release benchmark; synthetic data in independent processes under runs/"]
fn sqlite_parquet_comparison() -> Result<()> {
    support::sqlite_benchmark::run()
}
