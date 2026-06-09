PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS provider (
    provider_code TEXT PRIMARY KEY,
    provider_name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS variable (
    variable_code TEXT PRIMARY KEY,
    variable_name TEXT NOT NULL,
    default_unit TEXT NOT NULL,
    description TEXT
);

CREATE TABLE IF NOT EXISTS station (
    station_uid INTEGER PRIMARY KEY,
    station_code TEXT NOT NULL,
    station_name TEXT NOT NULL,
    provider_code TEXT NOT NULL REFERENCES provider(provider_code),
    latitude REAL,
    longitude REAL,
    altitude_m INTEGER,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (provider_code, station_code)
);

CREATE TABLE IF NOT EXISTS asset (
    asset_id TEXT PRIMARY KEY,
    asset_kind TEXT NOT NULL,
    format TEXT NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    provider_code TEXT REFERENCES provider(provider_code),
    checksum TEXT,
    valid_from TEXT,
    valid_to TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS observed_series (
    series_id TEXT PRIMARY KEY,
    station_uid INTEGER NOT NULL REFERENCES station(station_uid) ON DELETE CASCADE,
    variable_code TEXT NOT NULL REFERENCES variable(variable_code),
    state TEXT NOT NULL DEFAULT 'raw' CHECK (state IN ('raw', 'curated', 'approved')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (station_uid, variable_code, state)
);

CREATE TABLE IF NOT EXISTS observed_value (
    series_id TEXT NOT NULL REFERENCES observed_series(series_id) ON DELETE CASCADE,
    observed_at TEXT NOT NULL,
    value REAL,
    PRIMARY KEY (series_id, observed_at)
);

CREATE TABLE IF NOT EXISTS qc_flag (
    qc_flag_id INTEGER PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_key TEXT NOT NULL,
    rule_code TEXT NOT NULL,
    severity TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'accepted', 'rejected', 'resolved')),
    message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS manual_edit (
    manual_edit_id INTEGER PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES asset(asset_id) ON DELETE CASCADE,
    t0_step INTEGER NOT NULL,
    t1_step INTEGER NOT NULL,
    shift_lat REAL NOT NULL DEFAULT 0,
    shift_lon REAL NOT NULL DEFAULT 0,
    rotation_deg REAL NOT NULL DEFAULT 0,
    multiplication_factor REAL NOT NULL DEFAULT 1 CHECK (multiplication_factor > 0),
    editor TEXT,
    reason TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (t1_step >= t0_step)
);

CREATE TRIGGER IF NOT EXISTS trg_manual_edit_no_overlap_insert
BEFORE INSERT ON manual_edit
FOR EACH ROW
WHEN EXISTS (
    SELECT 1
    FROM manual_edit AS existing
    WHERE existing.asset_id = NEW.asset_id
      AND NEW.t0_step < existing.t1_step
      AND NEW.t1_step > existing.t0_step
)
BEGIN
    SELECT RAISE(ABORT, 'manual_edit overlap for asset_id');
END;

CREATE TRIGGER IF NOT EXISTS trg_manual_edit_no_overlap_update
BEFORE UPDATE ON manual_edit
FOR EACH ROW
WHEN EXISTS (
    SELECT 1
    FROM manual_edit AS existing
    WHERE existing.asset_id = NEW.asset_id
      AND existing.manual_edit_id <> NEW.manual_edit_id
      AND NEW.t0_step < existing.t1_step
      AND NEW.t1_step > existing.t0_step
)
BEGIN
    SELECT RAISE(ABORT, 'manual_edit overlap for asset_id');
END;

CREATE TABLE IF NOT EXISTS run_catalog (
    run_id TEXT PRIMARY KEY,
    run_kind TEXT NOT NULL CHECK (run_kind IN ('automatic', 'manual')),
    parent_run_id TEXT,
    reference_time TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'ready', 'executed', 'reviewed', 'published')),
    run_db_path TEXT NOT NULL,
    summary_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_observed_series_station_var ON observed_series(station_uid, variable_code);
CREATE INDEX IF NOT EXISTS idx_observed_value_observed_at ON observed_value(observed_at);
CREATE INDEX IF NOT EXISTS idx_qc_flag_scope ON qc_flag(scope_type, scope_key);
CREATE INDEX IF NOT EXISTS idx_manual_edit_asset_step ON manual_edit(asset_id, t0_step, t1_step, created_at);
CREATE INDEX IF NOT EXISTS idx_run_catalog_status ON run_catalog(status);

INSERT OR IGNORE INTO provider (provider_code, provider_name, provider_type) VALUES
    ('ana', 'Agencia Nacional de Aguas e Saneamento Basico', 'observed'),
    ('inmet', 'Instituto Nacional de Meteorologia', 'observed'),
    ('ecmwf', 'European Centre for Medium-Range Weather Forecasts', 'forecast');

INSERT OR IGNORE INTO variable (variable_code, variable_name, default_unit, description) VALUES
    ('rain', 'Precipitacao observada', 'mm', 'Valor observado no timestamp original'),
    ('level', 'Nivel observado', 'cm', 'Nivel hidrometrico observado'),
    ('flow', 'Vazao observada', 'm3/s', 'Vazao observada no timestamp original');
