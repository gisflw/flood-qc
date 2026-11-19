use super::fixed::{formatted, lines, number, parse};
use crate::store::{CalibrationGeometryRow, GeometryField, MiniTopology, validate_mini_topology};
use std::{collections::HashSet, ops::Range};

const HEADER: &[(Range<usize>, &[u8])] = &[
    (0..8, b"CatID"),
    (8..16, b"Mini"),
    (16..31, b"Xcen"),
    (31..46, b"Ycen"),
    (46..54, b"Sub"),
    (54..69, b"Area_(km2)"),
    (69..84, b"AreaM_(km2)"),
    (84..99, b"Ltr_(km)"),
    (99..114, b"Str_(m/km)"),
    (114..129, b"Lrl_(km)"),
    (129..144, b"Srl_(m/km)"),
    (144..152, b"MiniJus"),
    (152..160, b"Ordem"),
    (160..165, b"Hdr"),
    (165..175, b"Width"),
    (175..185, b"Depth"),
    (185..195, b"NMAN"),
];

pub(crate) struct GeometryFile {
    bytes: Vec<u8>,
    rows: Vec<CalibrationGeometryRow>,
    offsets: Vec<usize>,
    block_count: usize,
}

impl GeometryFile {
    pub(crate) fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        let mut records = lines(&bytes);
        let (_, header) = records.next().ok_or("missing geometry header")?;
        let header = header.strip_prefix(b"\xef\xbb\xbf").unwrap_or(header);
        for (range, label) in HEADER {
            let field = header.get(range.clone()).map(<[u8]>::trim_ascii);
            if field != Some(*label)
                && !matches!(
                    (*label, field),
                    (b"Lrl_(km)", Some(b"Ltl_(km)")) | (b"Srl_(m/km)", Some(b"Stl_(m/km)"))
                )
            {
                return Err(format!(
                    "missing fixed-width geometry column {}",
                    String::from_utf8_lossy(label)
                ));
            }
        }
        let blocks: Vec<_> = header[195..]
            .split(|byte| byte.is_ascii_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if blocks.is_empty() || blocks.len() > 99 {
            return Err("geometry requires one or more BLC columns".into());
        }
        for (index, label) in blocks.iter().enumerate() {
            let range = layout(GeometryField::Block(index + 1)).0;
            let field = header.get(range).map(<[u8]>::trim_ascii);
            if field != Some(*label) || *label != format!("BLC_{:02}", index + 1).as_bytes() {
                return Err(
                    "geometry BLC columns must appear in fixed-width consecutive order".into(),
                );
            }
        }
        let width = 195 + blocks.len() * 10;
        let mut rows = Vec::new();
        let mut offsets = Vec::new();
        let mut cat_ids = HashSet::new();
        for (index, (range, record)) in records.enumerate() {
            let mini = index + 1;
            if record.len() < width
                || !record[width..].iter().all(|byte| *byte == b' ')
                || record
                    .split(|byte| byte.is_ascii_whitespace())
                    .filter(|field| !field.is_empty())
                    .count()
                    != HEADER.len() + blocks.len()
            {
                return Err(format!("invalid fixed-width record for MINI {mini}"));
            }
            let row = read_row(record, blocks.len()).map_err(|e| format!("MINI {mini}: {e}"))?;
            if row.mini != mini || row.cat_id == 0 || !cat_ids.insert(row.cat_id) {
                return Err(format!(
                    "invalid ordered MINI or unique CatID at record {mini}"
                ));
            }
            offsets.push(range.start);
            rows.push(row);
        }
        validate_mini_topology(&topology(&rows))?;
        let block_count = blocks.len();
        let geometry = Self {
            bytes,
            rows,
            offsets,
            block_count,
        };
        geometry.validate_blocks()?;
        Ok(geometry)
    }

    pub(crate) fn rows(&self) -> &[CalibrationGeometryRow] {
        &self.rows
    }

    pub(crate) fn block_count(&self) -> usize {
        self.block_count
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn validate_blocks(&self) -> Result<(), String> {
        for row in &self.rows {
            let sum: f32 = row.blocks.iter().map(|value| *value as f32).sum();
            if !(99.99..=100.01).contains(&sum) {
                return Err(format!(
                    "MINI {} BLC percentages must sum to 100 (found {sum})",
                    row.mini
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn apply(
        &mut self,
        edits: impl IntoIterator<Item = (usize, GeometryField, f64)>,
    ) -> Result<usize, String> {
        let mut changes = Vec::new();
        let mut changed = HashSet::new();
        let mut check_topology = None;
        for (mini, field, value) in edits {
            let row = self
                .rows
                .get(mini.wrapping_sub(1))
                .ok_or_else(|| format!("unknown MINI {mini}"))?;
            if matches!(field, GeometryField::Block(block) if block == 0 || block > self.block_count)
            {
                return Err(format!("unknown BLC column {field:?}"));
            }
            let (range, decimals) = layout(field);
            validate(field, value).map_err(|e| format!("MINI {mini}: {e}"))?;
            let original = get(row, field);
            if original == value {
                continue;
            }
            let field_width = range.len() - 1;
            let text = format!("{value:.decimals$}");
            let field_bytes =
                formatted(text, field_width).map_err(|e| format!("MINI {mini} {field:?}: {e}"))?;
            let mut replacement = vec![b' '];
            replacement.extend_from_slice(&field_bytes);
            let value = number(&replacement)?;
            validate(field, value).map_err(|e| format!("MINI {mini} after rounding: {e}"))?;
            if original == value {
                continue;
            }
            let index = mini - 1;
            if field == GeometryField::Downstream {
                let topology = check_topology.get_or_insert_with(|| topology(&self.rows));
                topology[index].downstream = (value != -1.0).then_some(value as i64);
            }
            changes.push((index, field, range, value, replacement));
            changed.insert(mini);
        }
        if let Some(topology) = check_topology {
            validate_mini_topology(&topology)?;
        }
        for (index, field, range, value, replacement) in changes {
            let start = self.offsets[index];
            self.bytes[start + range.start..start + range.end].copy_from_slice(&replacement);
            set(&mut self.rows[index], field, value);
        }
        Ok(changed.len())
    }
}

fn read_row(record: &[u8], block_count: usize) -> Result<CalibrationGeometryRow, String> {
    let integer = |index: usize| parse::<usize>(&record[HEADER[index].0.clone()]);
    let longitude = number(&record[16..31])?;
    let latitude = number(&record[31..46])?;
    let subbasin = integer(4)?;
    if !(-180.0..=180.0).contains(&longitude)
        || !(-90.0..=90.0).contains(&latitude)
        || subbasin == 0
    {
        return Err("invalid coordinates or subbasin".into());
    }
    let read = |field| {
        let range = layout(field).0;
        let value = match field {
            GeometryField::Downstream | GeometryField::Order | GeometryField::Hdr => {
                parse::<i64>(&record[range])? as f64
            }
            _ => number(&record[range])?,
        };
        validate(field, value)?;
        Ok::<_, String>(value)
    };
    use GeometryField::*;
    Ok(CalibrationGeometryRow {
        cat_id: integer(0)?,
        mini: integer(1)?,
        longitude,
        latitude,
        subbasin,
        area: read(Area)?,
        upstream_area: read(UpstreamArea)?,
        river_length: read(RiverLength)?,
        river_slope: read(RiverSlope)?,
        longest_length: read(LongestLength)?,
        longest_slope: read(LongestSlope)?,
        downstream: read(Downstream)? as i64,
        order: read(Order)? as usize,
        hdr: read(Hdr)? as usize,
        width: read(Width)?,
        depth: read(Depth)?,
        manning: read(Manning)?,
        blocks: (1..=block_count)
            .map(|block| read(Block(block)))
            .collect::<Result<_, _>>()?,
    })
}

fn layout(field: GeometryField) -> (Range<usize>, usize) {
    use GeometryField::*;
    match field {
        Area => (54..69, 5),
        UpstreamArea => (69..84, 5),
        RiverLength => (84..99, 5),
        RiverSlope => (99..114, 5),
        LongestLength => (114..129, 5),
        LongestSlope => (129..144, 5),
        Downstream => (144..152, 0),
        Order => (152..160, 0),
        Hdr => (160..165, 0),
        Width => (165..175, 2),
        Depth => (175..185, 2),
        Manning => (185..195, 4),
        Block(index) => (195 + (index - 1) * 10..195 + index * 10, 5),
    }
}

fn validate(field: GeometryField, value: f64) -> Result<(), String> {
    use GeometryField::*;
    let valid = (value as f32).is_finite()
        && match field {
            Area | UpstreamArea | RiverLength | Width | Depth | Manning => value > 0.0,
            RiverSlope | LongestLength | LongestSlope => value >= 0.0,
            Downstream => value.fract() == 0.0 && (value == -1.0 || value > 0.0),
            Order => value.fract() == 0.0 && value > 0.0,
            Hdr => value == 0.0 || value == 1.0,
            Block(_) => (0.0..=100.0).contains(&value),
        };
    if valid {
        Ok(())
    } else {
        Err(format!("invalid {field:?} value {value}"))
    }
}

fn get(row: &CalibrationGeometryRow, field: GeometryField) -> f64 {
    use GeometryField::*;
    match field {
        Area => row.area,
        UpstreamArea => row.upstream_area,
        RiverLength => row.river_length,
        RiverSlope => row.river_slope,
        LongestLength => row.longest_length,
        LongestSlope => row.longest_slope,
        Downstream => row.downstream as f64,
        Order => row.order as f64,
        Hdr => row.hdr as f64,
        Width => row.width,
        Depth => row.depth,
        Manning => row.manning,
        Block(index) => row.blocks[index - 1],
    }
}

fn set(row: &mut CalibrationGeometryRow, field: GeometryField, value: f64) {
    use GeometryField::*;
    match field {
        Area => row.area = value,
        UpstreamArea => row.upstream_area = value,
        RiverLength => row.river_length = value,
        RiverSlope => row.river_slope = value,
        LongestLength => row.longest_length = value,
        LongestSlope => row.longest_slope = value,
        Downstream => row.downstream = value as i64,
        Order => row.order = value as usize,
        Hdr => row.hdr = value as usize,
        Width => row.width = value,
        Depth => row.depth = value,
        Manning => row.manning = value,
        Block(index) => row.blocks[index - 1] = value,
    }
}

fn topology(rows: &[CalibrationGeometryRow]) -> Vec<MiniTopology> {
    rows.iter()
        .map(|row| MiniTopology {
            mini_id: row.mini as i64,
            downstream: (row.downstream != -1).then_some(row.downstream),
            area_km2: row.area,
        })
        .collect()
}
