CREATE TABLE observation_replacement_v3(
 replacement_id TEXT PRIMARY KEY NOT NULL,
 station_id TEXT NOT NULL REFERENCES station(station_id),
 variable TEXT NOT NULL CHECK(variable IN ('rain','flow')),
 kind TEXT NOT NULL CHECK(kind IN ('point','period')),
 start_at TEXT NOT NULL,end_at TEXT,value REAL,reason TEXT NOT NULL,
 CHECK((kind='point' AND end_at IS NULL) OR
       (kind='period' AND (end_at IS NULL OR start_at<end_at))));
INSERT INTO observation_replacement_v3(replacement_id,station_id,variable,kind,start_at,end_at,value,reason)
 SELECT replacement_id,station_id,variable,kind,start_at,end_at,value,reason FROM observation_replacement;
DROP TABLE observation_replacement;
ALTER TABLE observation_replacement_v3 RENAME TO observation_replacement;
PRAGMA user_version=3;
