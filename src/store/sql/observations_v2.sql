CREATE TABLE level_reference(
 station_id TEXT NOT NULL REFERENCES station(station_id),
 kind TEXT NOT NULL,date TEXT NOT NULL,level_cm REAL NOT NULL,
 PRIMARY KEY(station_id,kind,date,level_cm));
CREATE UNIQUE INDEX level_threshold_key ON level_reference(station_id,kind) WHERE date='';
PRAGMA user_version=2;
