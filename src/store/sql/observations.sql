CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE station(station_id TEXT PRIMARY KEY,provider_code TEXT NOT NULL,
 station_code TEXT NOT NULL,name TEXT NOT NULL,latitude REAL,longitude REAL,
 UNIQUE(provider_code,station_code),
 CHECK((latitude IS NULL AND longitude IS NULL) OR
 (latitude IS NOT NULL AND longitude IS NOT NULL AND
 latitude BETWEEN -90 AND 90 AND longitude BETWEEN -180 AND 180)));
CREATE TABLE observation_series(series_id TEXT PRIMARY KEY,
 station_id TEXT NOT NULL REFERENCES station(station_id),
 variable TEXT NOT NULL CHECK(variable IN ('rain','level','flow')),
 state TEXT NOT NULL CHECK(state IN ('approved','curated','raw')),
 unit TEXT NOT NULL,interval_seconds INTEGER,created_at TEXT NOT NULL,
 CHECK((variable='rain' AND unit='mm' AND interval_seconds=3600) OR
 (variable='level' AND unit='cm' AND interval_seconds IS NULL) OR
 (variable='flow' AND unit='m3/s' AND interval_seconds IS NULL)));
CREATE TABLE observation(series_id TEXT NOT NULL REFERENCES observation_series(series_id),
 observed_at TEXT NOT NULL,value NUMERIC,
 PRIMARY KEY(series_id,observed_at),
 CHECK(value IS NULL OR typeof(value) IN ('integer','real')));
CREATE TABLE observation_replacement(
 replacement_id TEXT PRIMARY KEY NOT NULL,
 station_id TEXT NOT NULL REFERENCES station(station_id),
 variable TEXT NOT NULL CHECK(variable IN ('rain','flow')),
 kind TEXT NOT NULL CHECK(kind IN ('point','period')),
 start_at TEXT NOT NULL,end_at TEXT,value REAL,reason TEXT NOT NULL,
 CHECK((kind='point' AND end_at IS NULL) OR
       (kind='period' AND (end_at IS NULL OR start_at<end_at))));
CREATE TABLE station_offset(station_id TEXT PRIMARY KEY REFERENCES station(station_id),
 mini_id INTEGER NOT NULL CHECK(mini_id>0),offset_m REAL NOT NULL);
CREATE INDEX observation_time ON observation(observed_at,series_id);
CREATE INDEX observation_selection ON
 observation_series(station_id,variable,state,created_at);
CREATE TABLE level_reference(
 station_id TEXT NOT NULL REFERENCES station(station_id),
 kind TEXT NOT NULL,date TEXT NOT NULL,level_cm REAL NOT NULL,
 PRIMARY KEY(station_id,kind,date,level_cm));
CREATE UNIQUE INDEX level_threshold_key ON level_reference(station_id,kind) WHERE date='';
CREATE TABLE station_observed_variable(
 station_id TEXT NOT NULL REFERENCES station(station_id) ON DELETE CASCADE,
 variable TEXT NOT NULL CHECK(variable IN ('rain','level','flow')),
 PRIMARY KEY(station_id,variable));
CREATE TABLE precipitation_flag(
 series_id TEXT NOT NULL, observed_at TEXT NOT NULL,
 reason TEXT NOT NULL CHECK(reason IN ('threshold','sequence')),
 PRIMARY KEY(series_id,observed_at,reason),
 FOREIGN KEY(series_id,observed_at) REFERENCES observation(series_id,observed_at) ON DELETE CASCADE);
CREATE VIEW final_observation AS
 SELECT o.series_id,o.observed_at,o.value AS raw_value,
 CASE WHEN EXISTS (SELECT 1 FROM precipitation_flag f
   WHERE f.series_id=o.series_id AND f.observed_at=o.observed_at)
 THEN NULL ELSE o.value END AS value FROM observation o;
