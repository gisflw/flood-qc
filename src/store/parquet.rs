use std::{collections::HashMap, fs::File, path::Path, sync::Arc};

use arrow_array::{
    Array, ArrayRef, Float64Array, Int32Array, Int64Array, RecordBatch, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::{Compression, ZstdLevel},
    file::properties::{WriterProperties, WriterVersion},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Text,
    Float,
    Int64,
    Int32,
}

fn data_type(kind: Kind) -> DataType {
    match kind {
        Kind::Text => DataType::Utf8,
        Kind::Float => DataType::Float64,
        Kind::Int64 => DataType::Int64,
        Kind::Int32 => DataType::Int32,
    }
}

pub(crate) fn write_with_metadata<T: Serialize>(
    path: &Path,
    columns: &[(&str, Kind)],
    rows: &[T],
    metadata: HashMap<String, String>,
) -> Result<(), String> {
    write_configured(path, columns, rows, metadata, 65_536, true)
}

pub(crate) fn write_staged<T: Serialize>(
    path: &Path,
    columns: &[(&str, Kind)],
    rows: &[T],
) -> Result<(), String> {
    write_configured(path, columns, rows, HashMap::new(), 65_536, false)
}

#[cfg(test)]
pub(crate) fn write_samples<T: Serialize>(
    path: &Path,
    columns: &[(&str, Kind)],
    rows: &[T],
) -> Result<(), String> {
    write_configured(path, columns, rows, HashMap::new(), 1024, true)
}

fn write_configured<T: Serialize>(
    path: &Path,
    columns: &[(&str, Kind)],
    rows: &[T],
    metadata: HashMap<String, String>,
    row_group_size: usize,
    durable: bool,
) -> Result<(), String> {
    let values = rows
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut arrays: Vec<ArrayRef> = Vec::new();
    for row in &values {
        for (name, kind) in columns {
            let v = row
                .get(*name)
                .ok_or_else(|| format!("missing field {name}"))?;
            if !v.is_null()
                && !match kind {
                    Kind::Text => v.is_string(),
                    Kind::Float => v.as_f64().is_some_and(f64::is_finite),
                    Kind::Int64 => v.as_i64().is_some(),
                    Kind::Int32 => v.as_i64().is_some_and(|v| i32::try_from(v).is_ok()),
                }
            {
                return Err(format!("invalid field {name}"));
            }
        }
    }
    for (name, kind) in columns {
        let vals = values
            .iter()
            .map(|row| row.get(*name).unwrap_or(&Value::Null))
            .collect::<Vec<_>>();
        let array: ArrayRef = match kind {
            Kind::Text => Arc::new(StringArray::from(
                vals.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
            )),
            Kind::Float => Arc::new(Float64Array::from(
                vals.iter().map(|v| v.as_f64()).collect::<Vec<_>>(),
            )),
            Kind::Int64 => Arc::new(Int64Array::from(
                vals.iter().map(|v| v.as_i64()).collect::<Vec<_>>(),
            )),
            Kind::Int32 => Arc::new(Int32Array::from(
                vals.iter()
                    .map(|v| v.as_i64().and_then(|v| i32::try_from(v).ok()))
                    .collect::<Vec<_>>(),
            )),
        };
        arrays.push(array);
    }
    let schema = Arc::new(Schema::new_with_metadata(
        columns
            .iter()
            .map(|(name, kind)| Field::new(*name, data_type(*kind), true))
            .collect::<Vec<_>>(),
        metadata,
    ));
    let batch = RecordBatch::try_new(schema, arrays).map_err(|e| e.to_string())?;
    write_batch_inner(path, &batch, row_group_size, durable)
}

pub(crate) fn write_batch_staged(
    path: &Path,
    batch: &RecordBatch,
    row_group_size: usize,
) -> Result<(), String> {
    write_batch_inner(path, batch, row_group_size, false)
}
fn write_batch_inner(
    path: &Path,
    batch: &RecordBatch,
    row_group_size: usize,
    durable: bool,
) -> Result<(), String> {
    let props = WriterProperties::builder()
        .set_writer_version(WriterVersion::PARQUET_2_0)
        .set_max_row_group_row_count(Some(row_group_size))
        .set_data_page_size_limit(1_048_576)
        .set_compression(Compression::ZSTD(
            ZstdLevel::try_new(1).map_err(|e| e.to_string())?,
        ))
        .build();
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer =
        ArrowWriter::try_new(file, batch.schema(), Some(props)).map_err(|e| e.to_string())?;
    writer.write(batch).map_err(|e| e.to_string())?;
    writer.close().map_err(|e| e.to_string())?;
    if durable {
        File::open(path)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn read<T: DeserializeOwned>(
    path: &Path,
    columns: &[(&str, Kind)],
) -> Result<Vec<T>, String> {
    read_inner(path, columns, None)
}

#[cfg(test)]
pub(crate) fn read_range<T: DeserializeOwned>(
    path: &Path,
    columns: &[(&str, Kind)],
    field: &str,
    start: &str,
    end: &str,
) -> Result<Vec<T>, String> {
    read_inner(path, columns, Some((field, start, end)))
}

fn read_inner<T: DeserializeOwned>(
    path: &Path,
    columns: &[(&str, Kind)],
    range: Option<(&str, &str, &str)>,
) -> Result<Vec<T>, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;
    for (name, kind) in columns {
        let field = builder
            .schema()
            .field_with_name(name)
            .map_err(|e| e.to_string())?;
        if field.data_type() != &data_type(*kind) {
            return Err(format!("invalid column {name}"));
        }
    }
    if let Some((field, start, end)) = range {
        let index = builder
            .schema()
            .index_of(field)
            .map_err(|e| e.to_string())?;
        let groups = builder
            .metadata()
            .row_groups()
            .iter()
            .enumerate()
            .filter_map(|(i, group)| {
                let overlaps = group
                    .column(index)
                    .statistics()
                    .and_then(|stats| Some((stats.min_bytes_opt()?, stats.max_bytes_opt()?)))
                    .is_none_or(|(min, max)| min <= end.as_bytes() && max >= start.as_bytes());
                overlaps.then_some(i)
            })
            .collect();
        builder = builder.with_row_groups(groups);
    }
    let reader = builder.build().map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for batch in reader {
        let batch = batch.map_err(|e| e.to_string())?;
        for row in 0..batch.num_rows() {
            if let Some((field, start, end)) = range {
                let column = batch
                    .column_by_name(field)
                    .ok_or_else(|| format!("missing column {field}"))?;
                let timestamps = column
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or("invalid range column")?;
                if timestamps.is_null(row) {
                    return Err("null range value".into());
                }
                let at = timestamps.value(row);
                if at < start || at > end {
                    continue;
                }
            }
            let mut object = Map::new();
            for (name, kind) in columns {
                let col = batch
                    .column_by_name(name)
                    .ok_or_else(|| format!("missing column {name}"))?;
                if col.data_type() != &data_type(*kind) {
                    return Err(format!("invalid column {name}"));
                }
                let value = if col.is_null(row) {
                    Value::Null
                } else {
                    match kind {
                        Kind::Text => Value::String(
                            col.as_any()
                                .downcast_ref::<StringArray>()
                                .ok_or("invalid text")?
                                .value(row)
                                .to_owned(),
                        ),
                        Kind::Float => serde_json::Number::from_f64(
                            col.as_any()
                                .downcast_ref::<Float64Array>()
                                .ok_or("invalid float")?
                                .value(row),
                        )
                        .map(Value::Number)
                        .ok_or("nonfinite float")?,
                        Kind::Int64 => Value::from(
                            col.as_any()
                                .downcast_ref::<Int64Array>()
                                .ok_or("invalid int64")?
                                .value(row),
                        ),
                        Kind::Int32 => Value::from(
                            col.as_any()
                                .downcast_ref::<Int32Array>()
                                .ok_or("invalid int32")?
                                .value(row),
                        ),
                    }
                };
                object.insert((*name).to_owned(), value);
            }
            rows.push(serde_json::from_value(Value::Object(object)).map_err(|e| e.to_string())?);
        }
    }
    Ok(rows)
}

pub(crate) fn read_batches(
    path: &Path,
    columns: &[(&str, Kind)],
) -> Result<Vec<RecordBatch>, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;
    for (name, kind) in columns {
        let field = builder
            .schema()
            .field_with_name(name)
            .map_err(|e| e.to_string())?;
        if field.data_type() != &data_type(*kind) {
            return Err(format!("invalid column {name}"));
        }
    }
    builder
        .build()
        .map_err(|e| e.to_string())?
        .map(|batch| batch.map_err(|e| e.to_string()))
        .collect()
}

pub(crate) fn sample_row_count(path: &Path) -> Result<usize, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;
    usize::try_from(builder.metadata().file_metadata().num_rows()).map_err(|e| e.to_string())
}

pub(crate) fn sample_batches(
    path: &Path,
    columns: &[(&str, Kind)],
    start: &str,
    end: &str,
) -> Result<Vec<RecordBatch>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;
    for (name, kind) in columns {
        let f = builder
            .schema()
            .field_with_name(name)
            .map_err(|e| e.to_string())?;
        if f.data_type() != &data_type(*kind) {
            return Err(format!("invalid column {name}"));
        }
    }
    let index = builder
        .schema()
        .index_of("observed_at")
        .map_err(|e| e.to_string())?;
    let groups = builder
        .metadata()
        .row_groups()
        .iter()
        .enumerate()
        .filter_map(|(i, g)| {
            g.column(index)
                .statistics()
                .and_then(|s| Some((s.min_bytes_opt()?, s.max_bytes_opt()?)))
                .is_none_or(|(min, max)| min <= end.as_bytes() && max >= start.as_bytes())
                .then_some(i)
        })
        .collect();
    builder
        .with_row_groups(groups)
        .build()
        .map_err(|e| e.to_string())?
        .map(|b| b.map_err(|e| e.to_string()))
        .collect()
}

pub(crate) fn read_metadata(path: &Path) -> Result<HashMap<String, String>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;
    Ok(reader.schema().metadata().clone().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    struct Row {
        observed_at: String,
        value: i32,
    }
    #[test]
    fn sample_range_prunes_row_groups() {
        std::fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let path = dir.path().join("samples.parquet");
        let rows = (0..3072)
            .map(|i| Row {
                observed_at: format!("{i:04}"),
                value: i,
            })
            .collect::<Vec<_>>();
        let columns = &[("observed_at", Kind::Text), ("value", Kind::Int32)];
        write_samples(&path, columns, &rows).unwrap();
        let batches = sample_batches(&path, columns, "1200", "1250").unwrap();
        assert_eq!(
            batches.iter().map(RecordBatch::num_rows).sum::<usize>(),
            1024
        );
        let middle = read_range::<Row>(&path, columns, "observed_at", "1200", "1250").unwrap();
        assert_eq!(middle.len(), 51);
        assert_eq!(middle.first().unwrap().value, 1200);
        assert_eq!(middle.last().unwrap().value, 1250);
        assert!(
            read_range::<Row>(&path, columns, "observed_at", "4000", "4100")
                .unwrap()
                .is_empty()
        );
    }
}
