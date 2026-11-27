#!/usr/bin/env python3
import importlib.util
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('bridge', Path(__file__).with_name('export_legacy_registry.py'))
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)


class SQLiteBridge(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.db = self.root / 'operation.sqlite'
        self.connection = sqlite3.connect(self.db)
        for dest, source in bridge.SQLITE_TABLES.items():
            cols = ('series_id','observed_at','value') if dest == 'samples' else bridge.TABLES[dest]
            self.connection.execute('CREATE TABLE "' + source + '" (' + ', '.join('"'+col+'"' for col in cols) + ')')
        for table in bridge.SQLITE_EXTRA:
            cols = ('key','value') if table == 'metadata' else ('id',)
            self.connection.execute('CREATE TABLE "' + table + '" (' + ', '.join('"'+col+'"' for col in cols) + ')')
        self.connection.executemany('INSERT INTO metadata VALUES (?,?)', [('format','mgb-ops-observations'), ('time_zone','UTC')])
        self.connection.execute('INSERT INTO station VALUES (?,?,?,?,?,?)', ('s','p','1','name',None,None))
        self.connection.execute('INSERT INTO observation_series VALUES (?,?,?,?,?,?,?)', ('r','s','rain','raw','mm',3600,'2026-01-01T00:00:00'))
        self.connection.executemany('INSERT INTO observation VALUES (?,?,?)', [('r','2026-01-01T00:00:00',0.0),('r','2026-01-01T01:00:00',None)])
        self.connection.commit()
        self.connection.close()

    def tearDown(self):
        self.tmp.cleanup()

    def test_null_zero_and_original_preserved(self):
        before = bridge.digest(self.db)
        dest = self.root / 'export'
        bridge.export(self.db, dest, None)
        manifest = json.loads((dest / 'manifest.json').read_text())
        self.assertFalse(manifest['coverage_available'])
        self.assertEqual(manifest['tables']['samples']['count'], 2)
        rows = [json.loads(line) for line in (dest / 'samples.jsonl').read_text().splitlines()]
        self.assertEqual(rows[0]['rain'], 0)
        self.assertIsNone(rows[1]['rain'])
        self.assertEqual(bridge.digest(self.db), before)
        self.assertEqual(bridge.digest(dest / 'samples.jsonl'), manifest['tables']['samples']['sha256'])
        with self.assertRaises(ValueError):
            bridge.export(self.db, dest, None)

    def test_rejects_unknown_tables_without_publishing(self):
        with sqlite3.connect(self.db) as connection:
            connection.execute('CREATE TABLE surprise (id)')
        dest = self.root / 'invalid'
        with self.assertRaises(ValueError):
            bridge.export(self.db, dest, None)
        self.assertFalse(dest.exists())


class ParquetBridge(unittest.TestCase):
    def setUp(self):
        try:
            import pyarrow as pa
            import pyarrow.parquet as pq
        except ImportError:
            self.skipTest('pyarrow is an external converter prerequisite')
        self.pa, self.pq = pa, pq
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        if hasattr(self, 'tmp'):
            self.tmp.cleanup()

    def table(self, path, columns, rows, metadata=None):
        pa = self.pa
        floating = {'latitude','longitude','rain','flow','value','offset_m','level_cm',
                    'flow_m3s','speed_mps','direction_degrees'}
        integer = {'interval_seconds','mini_id'}
        schema = pa.schema([pa.field(c, pa.float64() if c in floating else pa.int64() if c in integer
                                     else pa.int32() if c == 'level' else pa.string()) for c in columns],
                           metadata=metadata)
        path.parent.mkdir(parents=True, exist_ok=True)
        self.pq.write_table(pa.Table.from_pylist(rows, schema=schema), path)

    def make_observations(self, root):
        tables = {}
        for name, columns in bridge.TABLES.items():
            if name == 'samples':
                continue
            relative = 'generations/one/' + name + '.parquet'
            self.table(root / relative, columns, [])
            tables[name] = relative
        base = 'generations/one/base.parquet'
        delta = 'generations/two/delta.parquet'
        common = {'series_id':'r','observed_at':'2026-01-01T00:00:00','flow':None,'level':None}
        self.table(root/base, bridge.TABLES['samples'], [{**common,'rain':2.0}])
        self.table(root/delta, bridge.TABLES['samples'], [{**common,'rain':0.0}])
        current = {'time_zone':'UTC','generation':'two','partitions':{'key':base},'deltas':{'key':[delta]},'tables':tables}
        (root/'current.json').write_text(json.dumps(current))
        (root/'access.lock').touch()

    def test_workspace_current_pointer_and_delta_overlay(self):
        workspace = self.root/'workspace'
        obs = workspace/'assets/observations'
        obs.mkdir(parents=True)
        (workspace/'config.toml').write_text('[run]\ntime_zone = "UTC"\n')
        (workspace/'.operation.lock').touch()
        self.make_observations(obs)
        bridge.export(workspace, self.root/'export', None)
        manifest = json.loads((self.root/'export/manifest.json').read_text())
        self.assertEqual(manifest['source_kind'], 'parquet')
        self.assertEqual(json.loads((self.root/'export/samples.jsonl').read_text())['rain'], 0.0)

    def test_rejects_locked_observation_store(self):
        import fcntl
        source = self.root/'observations'
        source.mkdir()
        self.make_observations(source)
        output = self.root/'locked-output'
        with (source/'access.lock').open('rb') as handle:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaisesRegex(ValueError, 'being modified'):
                bridge.export(source, output, None)
        self.assertFalse(output.exists())

    def test_saved_set_preserves_payload_and_forcing(self):
        source = self.root/'set'
        obs = source/'sources/observations'
        obs.mkdir(parents=True)
        self.make_observations(obs)
        engine = {'path':'baseline/engine','kind':'baseline','size':1,'sha256':'0'*64}
        recipe = {'engine_identity':engine,'document':{'scenarios':[]},'workspace_scenarios':[]}
        (source/'sources/recipe.json').write_text(json.dumps(recipe))
        self.table(source/'sources/wind-observations.parquet',
                   ('series_id','at','speed_mps','direction_degrees'),
                   [{'series_id':'w','at':'2026-01-01T00:00:00','speed_mps':0.0,'direction_degrees':0.0},
                    {'series_id':'w','at':'2026-01-01T01:00:00','speed_mps':None,'direction_degrees':None}],
                   {b'kind':b'wind_observations',b'version':b'1',b'time_zone':b'UTC'})
        payload = sorted(p for p in (source/'sources').rglob('*') if p.is_file())
        entries = [{'path':str(p.relative_to(source)), 'kind':'sources', 'size':p.stat().st_size, 'sha256':bridge.digest(p)} for p in payload]
        (source/'input-set.json').write_text(json.dumps({'version':1,'id':'old','created_at':'now','engine':engine,'recipe':recipe,'files':entries}))
        bridge.export(source, self.root/'export', None)
        manifest = json.loads((self.root/'export/manifest.json').read_text())
        self.assertEqual(manifest['forcing']['wind_observations']['file'], 'wind-observations.toml')
        self.assertIn('speed_mps = 0.0', (self.root/'export/wind-observations.toml').read_text())
        self.assertTrue((self.root/'export/legacy-input-set/sources/recipe.json').exists())


if __name__ == '__main__':
    unittest.main()
