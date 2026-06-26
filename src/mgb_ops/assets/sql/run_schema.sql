PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS run (
    run_id TEXT PRIMARY KEY,
    reference_time TEXT NOT NULL,
    run_kind TEXT NOT NULL CHECK (run_kind IN ('automatic', 'manual')),
    parent_run_id TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'ready', 'executed', 'reviewed', 'published')),
    operator TEXT,
    note TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS run_input_series (
    series_id TEXT PRIMARY KEY,
    history_series_id TEXT,
    station_id TEXT,
    provider_code TEXT,
    variable_code TEXT NOT NULL,
    unit TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'raw' CHECK (state IN ('raw', 'curated', 'approved')),
    source_asset_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS run_input_value (
    series_id TEXT NOT NULL REFERENCES run_input_series(series_id) ON DELETE CASCADE,
    observed_at TEXT NOT NULL,
    value REAL,
    PRIMARY KEY (series_id, observed_at)
);

CREATE TABLE IF NOT EXISTS run_asset (
    asset_id TEXT PRIMARY KEY,
    asset_role TEXT NOT NULL,
    asset_kind TEXT NOT NULL,
    format TEXT NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    source_history_asset_id TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS derived_series (
    series_id TEXT PRIMARY KEY,
    source_series_id TEXT,
    variable_code TEXT NOT NULL,
    unit TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'raw' CHECK (state IN ('raw', 'curated', 'approved')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS derived_value (
    derived_value_id INTEGER PRIMARY KEY,
    series_id TEXT NOT NULL REFERENCES derived_series(series_id) ON DELETE CASCADE,
    observed_at TEXT,
    window_start TEXT,
    window_end TEXT,
    horizon_h INTEGER,
    value REAL
);

CREATE TABLE IF NOT EXISTS model_execution (
    model_execution_id INTEGER PRIMARY KEY,
    model_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'planned', 'running', 'completed', 'failed')),
    planned_command TEXT,
    started_at TEXT,
    finished_at TEXT,
    setup_gpkg_path TEXT NOT NULL,
    setup_version TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS mgb_output_series (
    series_id INTEGER PRIMARY KEY,
    model_execution_id INTEGER NOT NULL REFERENCES model_execution(model_execution_id) ON DELETE CASCADE,
    variable_code TEXT NOT NULL,
    cell_id INTEGER NOT NULL,
    prev_flag INTEGER NOT NULL DEFAULT 0 CHECK (prev_flag IN (0, 1)),
    unit TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (model_execution_id, variable_code, cell_id, prev_flag)
);

CREATE TABLE IF NOT EXISTS mgb_output_value (
    series_id INTEGER NOT NULL REFERENCES mgb_output_series(series_id) ON DELETE CASCADE,
    dt TEXT NOT NULL,
    value REAL,
    PRIMARY KEY (series_id, dt)
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
    scope_type TEXT NOT NULL,
    scope_key TEXT NOT NULL,
    field_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    editor TEXT,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS report_artifact (
    report_artifact_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    format TEXT NOT NULL,
    note TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_run_input_value_observed_at ON run_input_value(observed_at);
CREATE INDEX IF NOT EXISTS idx_derived_value_series ON derived_value(series_id);
CREATE INDEX IF NOT EXISTS idx_mgb_output_value_dt ON mgb_output_value(dt);
CREATE INDEX IF NOT EXISTS idx_mgb_output_series_cell ON mgb_output_series(cell_id, variable_code);
