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
    station_id TEXT PRIMARY KEY,
    station_code TEXT NOT NULL,
    station_name TEXT NOT NULL,
    provider_code TEXT NOT NULL REFERENCES provider(provider_code),
    mini_id INTEGER,
    latitude REAL,
    longitude REAL,
    altitude_m REAL,
    river_name TEXT,
    drainage_area_km2 REAL,
    municipality TEXT,
    state TEXT,
    responsible_agency TEXT,
    operator TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (provider_code, station_code)
);

CREATE TABLE IF NOT EXISTS station_observed_variable (
    station_id TEXT NOT NULL REFERENCES station(station_id) ON DELETE CASCADE,
    variable_code TEXT NOT NULL REFERENCES variable(variable_code),
    PRIMARY KEY (station_id, variable_code)
);

CREATE TABLE IF NOT EXISTS station_level_reference (
    station_id TEXT NOT NULL REFERENCES station(station_id) ON DELETE CASCADE,
    reference_code TEXT NOT NULL CHECK (reference_code IN ('attention', 'alert', 'flood', 'severe')),
    level_cm REAL NOT NULL,
    PRIMARY KEY (station_id, reference_code)
);

CREATE TABLE IF NOT EXISTS historical_flood_level (
    station_id TEXT NOT NULL REFERENCES station(station_id) ON DELETE CASCADE,
    level_cm REAL NOT NULL,
    event_date TEXT NOT NULL,
    PRIMARY KEY (station_id, level_cm, event_date)
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
    station_id TEXT NOT NULL REFERENCES station(station_id) ON DELETE CASCADE,
    variable_code TEXT NOT NULL REFERENCES variable(variable_code),
    state TEXT NOT NULL DEFAULT 'raw' CHECK (state IN ('raw', 'curated', 'approved')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (station_id, variable_code, state)
);

CREATE TABLE IF NOT EXISTS observed_value (
    series_id TEXT NOT NULL REFERENCES observed_series(series_id) ON DELETE CASCADE,
    observed_at TEXT NOT NULL,
    value REAL,
    PRIMARY KEY (series_id, observed_at)
);

CREATE INDEX IF NOT EXISTS idx_station_level_reference_station ON station_level_reference(station_id);
CREATE INDEX IF NOT EXISTS idx_historical_flood_level_station ON historical_flood_level(station_id, event_date);
CREATE INDEX IF NOT EXISTS idx_observed_series_station_var ON observed_series(station_id, variable_code);
CREATE INDEX IF NOT EXISTS idx_observed_value_observed_at ON observed_value(observed_at);
CREATE INDEX IF NOT EXISTS idx_station_observed_variable_variable ON station_observed_variable(variable_code);

INSERT OR IGNORE INTO provider (provider_code, provider_name, provider_type) VALUES
    ('ana', 'National Water and Basic Sanitation Agency', 'observed'),
    ('inmet', 'National Institute of Meteorology', 'observed'),
    ('ecmwf', 'European Centre for Medium-Range Weather Forecasts', 'forecast'),
    ('noaa', 'National Oceanic and Atmospheric Administration', 'forecast');

INSERT OR IGNORE INTO variable (variable_code, variable_name, default_unit, description) VALUES
    ('rain', 'Observed precipitation', 'mm', 'Observed value at the original timestamp'),
    ('level', 'Observed level', 'cm', 'Observed hydrometric level'),
    ('flow', 'Observed flow', 'm3/s', 'Observed flow at the original timestamp');
