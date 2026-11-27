#!/usr/bin/env python3
"""Export legacy observations as JSONL for registry migration (never edits source).

Usage: export_legacy_registry.py SOURCE OUTPUT [--time-zone ZONE]
SOURCE is an operation.sqlite, Parquet observation directory, workspace root,
or independently saved input set. OUTPUT must not exist. Parquet inputs require
`pyarrow`; SQLite inputs use only the Python standard library. JSONL rows use
original column names and preserve JSON nulls, numeric zeros, and ordering of
replacement inputs. The manifest gives table columns, row counts and SHA-256.
Saved input set files are verified against their complete index and copied under
`legacy-input-set/`, preserving the recipe and engine identity for the importer.
This is an offline migration tool, not an application dependency.
"""
import argparse
import contextlib
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import sqlite3
import tempfile

TABLES = {
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
SQLITE_TABLES = {'stations':'station','series':'observation_series','samples':'observation',
                 'replacements':'observation_replacement','offsets':'station_offset',
                 'references':'level_reference','variables':'station_observed_variable',
                 'flags':'precipitation_flag'}
SQLITE_EXTRA = ('metadata','asset','correction','run_attempt','active_run')


@contextlib.contextmanager
def source_lock(path):
    """Coordinate with Rust file locks without creating or modifying a lock file."""
    if path.is_symlink() or not path.is_file():
        raise ValueError(f'missing regular source lock: {path}')
    try:
        import fcntl
    except ImportError as error:
        raise ValueError('legacy export locking is not supported on this platform') from error
    with path.open('rb') as handle:
        try:
            fcntl.flock(handle.fileno(), fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError(f'source is being modified: {path}') from error
        try:
            yield
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def safe_member(root, relative):
    rel = Path(relative)
    if rel.is_absolute() or not rel.parts or any(p in ('..', '.') for p in rel.parts):
        raise ValueError(f'unsafe path {relative!r}')
    path = root.joinpath(rel)
    if any(x.is_symlink() for x in [path, *list(path.parents)[:len(rel.parts)-1]]):
        raise ValueError(f'symlink in source path {relative!r}')
    if not path.is_file():
        raise ValueError(f'missing file {relative!r}')
    return path


def write_table(output, name, columns, rows):
    path = output / f'{name}.jsonl'
    count = 0
    with path.open('xb') as f:
        for row in rows:
            if set(row) != set(columns):
                raise ValueError(f'{name}: unexpected columns')
            encoded = json.dumps(row, ensure_ascii=False, allow_nan=False, separators=(',', ':')).encode('utf-8') + b'\n'
            f.write(encoded)
            count += 1
        f.flush()
        os.fsync(f.fileno())
    return {'file': path.name, 'columns': list(columns), 'count': count, 'sha256': digest(path)}


def sqlite_files(path):
    files = {}
    for candidate in (path, path.with_name(path.name + '-wal')):
        if candidate.exists():
            if candidate.is_symlink() or not candidate.is_file():
                raise ValueError(f'invalid SQLite source: {candidate}')
            files[candidate.name] = {'size':candidate.stat().st_size, 'sha256':digest(candidate)}
    if path.name not in files:
        raise ValueError('SQLite source is missing')
    return files


def sqlite_export(path, output, requested_zone):
    before = sqlite_files(path)
    if path.name + '-wal' in before:
        raise ValueError('SQLite source has a WAL; close and checkpoint it before export')
    connection = sqlite3.connect(path.resolve().as_uri() + '?mode=ro&immutable=1', uri=True)
    connection.row_factory = sqlite3.Row
    try:
        connection.execute('BEGIN')
        names = {r[0] for r in connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")}
        expected = set(SQLITE_TABLES.values()) | set(SQLITE_EXTRA)
        if names != expected:
            raise ValueError(f'unexpected SQLite tables: missing={sorted(expected-names)} extra={sorted(names-expected)}')
        if connection.execute('PRAGMA integrity_check').fetchone()[0] != 'ok':
            raise ValueError('SQLite integrity check failed')
        if connection.execute('PRAGMA foreign_key_check').fetchone() is not None:
            raise ValueError('SQLite foreign key check failed')
        metadata = dict(connection.execute('SELECT key, value FROM metadata'))
        zone = metadata.get('time_zone')
        if metadata.get('format') != 'mgb-ops-observations' or not zone:
            raise ValueError('unrecognized SQLite metadata or missing time_zone')
        if requested_zone and zone != requested_zone:
            raise ValueError('requested time zone disagrees with SQLite')
        tables = {}
        for dest, source in SQLITE_TABLES.items():
            actual = tuple(row[1] for row in connection.execute(f'PRAGMA table_info("{source}")'))
            expected_columns = ('series_id','observed_at','value') if dest == 'samples' else TABLES[dest]
            if actual != expected_columns:
                raise ValueError(f'{source}: unexpected schema {actual}')
            if dest == 'samples':
                sql = '''SELECT o.series_id, o.observed_at, o.value, s.variable FROM observation o
                         JOIN observation_series s ON o.series_id=s.series_id ORDER BY o.series_id,o.observed_at'''
                def rows():
                    for row in connection.execute(sql):
                        variable = row['variable']
                        if variable not in ('rain','flow','level'):
                            raise ValueError(f'unknown observation variable {variable!r}')
                        value = row['value']
                        if variable == 'level' and value is not None:
                            if not math.isfinite(value) or int(value) != value or not -(2**31) <= value < 2**31:
                                raise ValueError('non-integral or out-of-range level')
                            value = int(value)
                        yield {'series_id': row['series_id'], 'observed_at': row['observed_at'],
                               **{key: value if key == variable else None for key in ('rain','flow','level')}}
                tables[dest] = write_table(output, dest, TABLES[dest], rows())
            else:
                tables[dest] = write_table(output, dest, TABLES[dest],
                    (dict(row) for row in connection.execute(f'SELECT * FROM "{source}" ORDER BY rowid')))
        # Legacy database has no full coverage table: retain that fact, not invented coverage.
        tables['coverage'] = write_table(output, 'coverage', TABLES['coverage'], ())
        extras = {}
        for source in SQLITE_EXTRA:
            columns = tuple(row[1] for row in connection.execute(f'PRAGMA table_info("{source}")'))
            if not columns:
                raise ValueError(f'no columns in {source}')
            extras[source] = write_table(output, 'legacy_'+source, columns,
                (dict(row) for row in connection.execute(f'SELECT * FROM "{source}" ORDER BY rowid')))
        if sqlite_files(path) != before:
            raise ValueError('SQLite source changed during export')
        return {'source_kind':'legacy_sqlite', 'time_zone':zone, 'tables':tables,
                'legacy_tables':extras, 'coverage_available':False, 'source_sqlite_files':before}
    finally:
        connection.close()


def read_parquet(path, columns):
    try:
        import pyarrow as pa
        import pyarrow.parquet as pq
    except ImportError as e:
        raise ValueError('Parquet conversion requires optional external Python package pyarrow') from e
    parquet = pq.ParquetFile(path)
    schema = parquet.schema_arrow
    if schema.names != list(columns):
        raise ValueError(f'{path}: unexpected Parquet columns')
    types = {'latitude':pa.float64(),'longitude':pa.float64(),'rain':pa.float64(),
             'flow':pa.float64(),'level':pa.int32(),'value':pa.float64(),
             'offset_m':pa.float64(),'level_cm':pa.float64(), 'flow_m3s':pa.float64(),
             'speed_mps':pa.float64(), 'direction_degrees':pa.float64(), 'mini_id':pa.int64(),
             'interval_seconds':pa.int64()}
    if any(field.type != types.get(field.name, pa.string()) for field in schema):
        raise ValueError(f'{path}: unexpected Parquet column types')
    for batch in parquet.iter_batches(batch_size=65536):
        yield from batch.to_pylist()


def parquet_export(source, output, requested_zone):
    with source_lock(source / 'access.lock'):
        return _parquet_export_locked(source, output, requested_zone)


def _parquet_export_locked(source, output, requested_zone):
    pointer = source / 'current.json'
    original_pointer = pointer.read_bytes()
    manifest = json.loads(original_pointer)
    if set(manifest) != {'time_zone','generation','partitions','deltas','tables'}:
        raise ValueError('unrecognized observation manifest')
    zone = manifest['time_zone']
    if requested_zone and zone != requested_zone:
        raise ValueError('requested time zone disagrees with observation store')
    if set(manifest['tables']) != set(TABLES) - {'samples'}:
        raise ValueError('unexpected observation table set')
    paths = list(manifest['tables'].values()) + list(manifest['partitions'].values())
    paths += [path for values in manifest['deltas'].values() for path in values]
    fingerprints = {relative: (safe_member(source, relative).stat().st_size,
                               digest(safe_member(source, relative))) for relative in paths}
    tables = {}
    for name in TABLES:
        if name == 'samples':
            continue
        path = safe_member(source, manifest['tables'][name])
        tables[name] = write_table(output, name, TABLES[name], read_parquet(path, TABLES[name]))
    def samples():
        keys = sorted(set(manifest['partitions']) | set(manifest['deltas']))
        for key in keys:
            rows = {}
            paths = ([manifest['partitions'][key]] if key in manifest['partitions'] else []) + manifest['deltas'].get(key, [])
            for relative in paths:
                for row in read_parquet(safe_member(source, relative), TABLES['samples']):
                    identity = (row['series_id'], row['observed_at'])
                    if None in identity:
                        raise ValueError('null observation identity')
                    rows[identity] = row
            for identity in sorted(rows):
                yield rows[identity]
    tables['samples'] = write_table(output, 'samples', TABLES['samples'], samples())
    if pointer.read_bytes() != original_pointer:
        raise ValueError('observation pointer changed during export')
    for relative, expected in fingerprints.items():
        path = safe_member(source, relative)
        if (path.stat().st_size, digest(path)) != expected:
            raise ValueError(f'observation file changed during export: {relative}')
    return {'source_kind':'parquet', 'time_zone':zone, 'tables':tables, 'coverage_available':True, 'source_pointer_sha256':hashlib.sha256(original_pointer).hexdigest(), 'source_members':{relative:{'size':size, 'sha256':sha} for relative,(size,sha) in fingerprints.items()}}


def export_forcing(source, output):
    """Convert indexed legacy forcing Parquet to the current TOML wire shape."""
    import pyarrow.parquet as pq
    converted = {}
    for filename, kind, columns in (
        ('reservoir-schedules', 'reservoir_schedules', ('series_id','at','flow_m3s')),
        ('wind-observations', 'wind_observations', ('series_id','at','speed_mps','direction_degrees')),
    ):
        path = source / 'sources' / (filename + '.parquet')
        if not path.exists():
            continue
        metadata = pq.ParquetFile(path).schema_arrow.metadata or {}
        def meta(key):
            return metadata.get(key.encode(), b'').decode('utf-8')
        if meta('version') != '1' or meta('kind') != kind or not meta('time_zone'):
            raise ValueError(f'{filename}: invalid forcing metadata')
        target = output / (filename + '.toml')
        with target.open('x', encoding='utf-8') as f:
            f.write('time_zone = ' + json.dumps(meta('time_zone'), ensure_ascii=False) + '\n')
            for row in read_parquet(path, columns):
                if row['series_id'] is None or row['at'] is None:
                    raise ValueError(f'{filename}: null sample identity')
                f.write('\n[[samples]]\n')
                f.write('series_id = ' + json.dumps(row['series_id'], ensure_ascii=False) + '\n')
                f.write('at = ' + json.dumps(row['at'], ensure_ascii=False) + '\n')
                if kind == 'reservoir_schedules':
                    value = row['flow_m3s']
                    if value is not None:
                        if not math.isfinite(value):
                            raise ValueError('nonfinite reservoir value')
                        f.write('flow_m3s = ' + repr(value) + '\n')
                else:
                    speed, direction = row['speed_mps'], row['direction_degrees']
                    if (speed is None) != (direction is None):
                        raise ValueError('unpaired wind components')
                    if speed is not None:
                        if not math.isfinite(speed) or not math.isfinite(direction):
                            raise ValueError('nonfinite wind value')
                        f.write(f'wind = {{ speed_mps = {speed!r}, direction_degrees = {direction!r} }}\n')
        converted[kind] = {'file':target.name, 'sha256':digest(target)}
    return converted


def _export_unlocked(source, destination, zone):
    source = source.resolve(strict=True)
    if destination.exists():
        raise ValueError('output already exists')
    parent = destination.parent.resolve(strict=True)
    if parent == source or source in parent.parents:
        raise ValueError('output must be outside source')
    stage = Path(tempfile.mkdtemp(prefix='.legacy-export-', dir=parent))
    try:
        saved = source.is_dir() and (source / 'input-set.json').is_file()
        workspace = source.is_dir() and (source / 'config.toml').is_file()
        if saved:
            index = json.loads((source / 'input-set.json').read_text())
            if index.get('version') != 1 or not isinstance(index.get('files'), list):
                raise ValueError('unsupported legacy input set')
            seen = set()
            copy = stage / 'legacy-input-set'
            copy.mkdir()
            for entry in index['files']:
                rel = entry['path']
                if rel in seen or not (rel.startswith('sources/') or rel.startswith('baseline/Input/')):
                    raise ValueError('invalid or duplicate input member')
                seen.add(rel)
                path = safe_member(source, rel)
                if path.stat().st_size != entry['size'] or digest(path) != entry['sha256']:
                    raise ValueError(f'input member integrity mismatch: {rel}')
                target = copy / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, target)
            actual = {str(p.relative_to(source)) for folder in ('sources','baseline/Input')
                      for p in (source / folder).rglob('*') if p.is_file()}
            if actual != seen:
                raise ValueError('input set inventory does not match index')
            recipe_path = safe_member(source, 'sources/recipe.json')
            if json.loads(recipe_path.read_text()) != index.get('recipe'):
                raise ValueError('indexed recipe differs from input manifest')
            if index.get('recipe', {}).get('engine_identity') != index.get('engine'):
                raise ValueError('engine identity differs from indexed recipe')
            shutil.copyfile(source / 'input-set.json', copy / 'input-set.json')
            obs = source / 'sources/observations'
            result = parquet_export(obs, stage, zone)
            result['legacy_input_set'] = 'legacy-input-set/input-set.json'
            result['forcing'] = export_forcing(source, stage)
        elif workspace:
            obs = source / 'assets/observations'
            if (obs / 'current.json').is_file():
                result = parquet_export(obs, stage, zone)
            else:
                result = sqlite_export(source / 'operation.sqlite', stage, zone)
            legacy_sets = source / 'cache/inputs'
            if legacy_sets.exists():
                if not legacy_sets.is_dir():
                    raise ValueError('legacy input cache is not a directory')
                nested = stage / 'legacy-selections'
                nested.mkdir()
                names = []
                for entry in sorted(legacy_sets.iterdir()):
                    if entry.name == 'current.json':
                        continue
                    if entry.is_symlink() or not entry.is_dir() or not (entry / 'input-set.json').is_file():
                        raise ValueError(f'unsupported legacy input cache entry: {entry.name}')
                    export(entry, nested / entry.name, zone)
                    names.append('legacy-selections/' + entry.name)
                result['legacy_selections'] = names
                pointer = legacy_sets / 'current.json'
                if pointer.exists():
                    data = json.loads(pointer.read_text())
                    if set(data) != {'id', 'saved_to'} or data['id'] not in [Path(x).name for x in names]:
                        raise ValueError('invalid or incomplete legacy input selection')
                    result['legacy_current_input'] = data
            partial = source / 'assets/ana-partial-coverage.json'
            if partial.is_file():
                data = json.loads(partial.read_text())
                if set(data) != {'stations'} or not isinstance(data['stations'], dict):
                    raise ValueError('unrecognized partial acquisition coverage')
                shutil.copyfile(partial, stage / 'partial_coverage.json')
                result['partial_coverage'] = 'partial_coverage.json'
        elif source.is_file():
            result = sqlite_export(source, stage, zone)
        else:
            result = parquet_export(source, stage, zone)
        (stage / 'manifest.json').write_text(json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False)+'\n')
        os.rename(stage, destination)
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def export(source, destination, zone):
    source = source.resolve(strict=True)
    if source.is_dir() and (source / 'config.toml').is_file():
        with source_lock(source / '.operation.lock'):
            return _export_unlocked(source, destination, zone)
    return _export_unlocked(source, destination, zone)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--time-zone')
    args = parser.parse_args()
    try:
        export(args.source, args.output, args.time_zone)
    except (ValueError, OSError, sqlite3.Error, KeyError, TypeError) as error:
        parser.exit(1, f'legacy export failed: {error}\n')


if __name__ == '__main__':
    main()
