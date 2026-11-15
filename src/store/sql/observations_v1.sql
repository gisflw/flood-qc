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
 variable TEXT NOT NULL CHECK(variable='rain'),
 kind TEXT NOT NULL CHECK(kind IN ('point','period')),
 start_at TEXT NOT NULL,end_at TEXT,value REAL,reason TEXT NOT NULL,
 CHECK((kind='point' AND end_at IS NULL) OR
       (kind='period' AND (end_at IS NULL OR start_at<end_at))));
CREATE TABLE station_offset(station_id TEXT PRIMARY KEY REFERENCES station(station_id),
 mini_id INTEGER NOT NULL CHECK(mini_id>0),offset_m REAL NOT NULL);
CREATE INDEX observation_time ON observation(observed_at,series_id);
CREATE INDEX observation_selection ON
 observation_series(station_id,variable,state,created_at);
PRAGMA user_version=1;
