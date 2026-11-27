#!/usr/bin/env python3
"""Build a legacy Parquet saved set from a synthetic portable run fixture.

Qualification only: requires PyArrow and deliberately writes outside application code.
"""
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tomllib
import pyarrow as pa
import pyarrow.parquet as pq

portable, recipe_path, output = map(Path, sys.argv[1:])
recipe = json.loads(recipe_path.read_text())
output.mkdir()
shutil.copytree(portable / 'baseline' / 'Input', output / 'baseline' / 'Input')
obs = output / 'sources' / 'observations'
obs.mkdir(parents=True)
(obs/'access.lock').write_bytes(b'')
columns = {
    'stations': ('station_id','provider_code','station_code','name','latitude','longitude'),
    'series': ('series_id','station_id','variable','state','unit','interval_seconds','created_at'),
    'samples': ('series_id','observed_at','rain','flow','level'),
    'replacements': ('replacement_id','station_id','variable','kind','start_at','end_at','value','reason'),
    'offsets': ('station_id','mini_id','offset_m'),
    'references': ('station_id','kind','date','level_cm'),
    'variables': ('station_id','variable'),
    'coverage': ('station_id','variables_key','day'),
    'flags': ('series_id','observed_at','reason'),
}
rows = {name: [] for name in columns}
rows['stations'] = [dict(station_id='s',provider_code='ana',station_code='1',name='station',latitude=0.0,longitude=0.0)]
rows['series'] = [dict(series_id='r',station_id='s',variable='rain',state='raw',unit='mm',interval_seconds=3600,created_at='2026-01-01T00:00:00')]
rows['samples'] = [dict(series_id='r',observed_at=f'2026-01-01T{hour:02}:00:00',rain=1.0,flow=None,level=None) for hour in range(3)]
rows['offsets'] = [dict(station_id='s',mini_id=1,offset_m=0.609)]
float_fields = {'latitude','longitude','rain','flow','value','offset_m','level_cm'}
int64_fields = {'interval_seconds','mini_id'}
tables = {}
for name, fields in columns.items():
    relative = f'generations/one/{name}.parquet'
    path = obs / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    schema = pa.schema([pa.field(field, pa.float64() if field in float_fields else pa.int64() if field in int64_fields else pa.int32() if field == 'level' else pa.string()) for field in fields])
    pq.write_table(pa.Table.from_pylist(rows[name], schema=schema), path)
    if name != 'samples':
        tables[name] = relative
current = {'time_zone':'UTC','generation':'one','partitions':{hashlib.sha256(b'r').hexdigest():'generations/one/samples.parquet'},'deltas':{},'tables':tables}
(obs/'current.json').write_text(json.dumps(current))

# Old catalog snapshots identified the product by a hexadecimal asset ID.
for scenario in recipe['document']['scenarios']:
    new_id = scenario.get('asset_id')
    if not new_id:
        continue
    metadata_path = portable/'sources'/'forecasts'/new_id/'metadata.toml'
    metadata = tomllib.loads(metadata_path.read_text())['record']
    metadata['logical_identity'] = json.loads(metadata['logical_identity'])
    metadata['normalization'] = json.loads(metadata['normalization'])
    identity = json.dumps(metadata['logical_identity'], sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
    old_id = hashlib.sha256(b'forecast-asset-v2:' + identity).hexdigest()
    source = portable/'sources'/'forecasts'/new_id/'native.nc'
    target = output/'sources'/'forecasts'/old_id/'native.nc'
    target.parent.mkdir(parents=True, exist_ok=True)
    (target.parent.parent/'access.lock').touch(exist_ok=True)
    shutil.copyfile(source, target)
    metadata.pop('registered_at', None)
    metadata['asset_id'] = old_id
    metadata['native_artifact'] = f'sources/forecasts/{old_id}/native.nc'
    metadata['retained_source'] = f'sources/forecasts/{old_id}/source.nc'
    metadata['native_sha256'] = hashlib.sha256(target.read_bytes()).hexdigest()
    metadata['native_byte_length'] = target.stat().st_size
    (target.parent/'metadata.json').write_text(json.dumps(metadata))
    scenario['asset_id'] = old_id
recipe['document']['schema_version'] = 1
recipe['document']['observed'].pop('revision', None)
recipe['document']['observed'].pop('historical_rainfall', None)
recipe.pop('observation_registry_id', None)
recipe.pop('registered_replacements', None)
(output/'sources'/'recipe.json').write_text(json.dumps(recipe))
files = sorted(p for base in (output/'sources', output/'baseline'/'Input') for p in base.rglob('*') if p.is_file())
entries = [{'path':str(p.relative_to(output)), 'kind':p.relative_to(output).parts[0], 'size':p.stat().st_size,
            'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in files]
manifest = {'version':1, 'id':'qualification', 'created_at':'2026-01-01T02:00:00Z',
            'engine':recipe['engine_identity'], 'recipe':recipe, 'files':entries}
(output/'input-set.json').write_text(json.dumps(manifest))
