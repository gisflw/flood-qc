PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS metadata (
    reference_time TEXT NOT NULL,
    reference_date TEXT NOT NULL,
    window_start TEXT NOT NULL,
    window_end_exclusive TEXT NOT NULL,
    dt_seconds INTEGER NOT NULL,
    nc INTEGER NOT NULL,
    nt_current INTEGER NOT NULL,
    nt_forecast INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS variable (
    variable_code TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    unit TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS output_series (
    series_id TEXT PRIMARY KEY,
    variable_code TEXT NOT NULL REFERENCES variable(variable_code) ON DELETE CASCADE,
    mini_id INTEGER NOT NULL,
    prev_flag INTEGER NOT NULL CHECK (prev_flag IN (0, 1)),
    unit TEXT NOT NULL,
    UNIQUE (variable_code, mini_id, prev_flag)
);

CREATE TABLE IF NOT EXISTS output_value (
    series_id TEXT NOT NULL REFERENCES output_series(series_id) ON DELETE CASCADE,
    dt TEXT NOT NULL,
    value REAL,
    PRIMARY KEY (series_id, dt)
);

CREATE INDEX IF NOT EXISTS idx_output_series_mini_variable ON output_series(mini_id, variable_code);
CREATE INDEX IF NOT EXISTS idx_output_value_dt ON output_value(dt);
