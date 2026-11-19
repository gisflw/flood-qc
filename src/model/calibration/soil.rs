use super::fixed::{formatted, lines, number, parse};
use crate::store::{HydrologicClass, SoilWatershed};
use std::{collections::HashSet, io::Write, ops::Range};

const HEADER: &[u8] = b"       use      Wm       b    Kbas    Kint      XL     CAP      Wc";
const CLASS_FIELDS: [(Range<usize>, usize); 7] = [
    (10..18, 1),
    (18..26, 2),
    (26..34, 2),
    (34..42, 2),
    (42..50, 2),
    (50..58, 2),
    (58..66, 2),
];
const COEFFICIENTS: [(&str, usize, usize); 4] = [
    ("CS", 1, 2),
    ("CI", 1, 2),
    ("CB", 1, 2),
    ("QB_M3/SKM2", 6, 4),
];

struct WatershedOffsets {
    classes: Vec<usize>,
    coefficients: Vec<Range<usize>>,
}

pub(crate) struct SoilFile {
    bytes: Vec<u8>,
    watersheds: Vec<SoilWatershed>,
    offsets: Vec<WatershedOffsets>,
}

pub(crate) fn parse_classes(bytes: &[u8]) -> Result<Vec<HydrologicClass>, String> {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let mut names = HashSet::new();
    let mut classes = Vec::new();
    for (_, line) in lines(bytes) {
        let mut parts = line.splitn(2, |byte| *byte == b'/');
        let name = class_name(parts.next().unwrap_or_default())?;
        if !names.insert(name) {
            return Err(format!("duplicate hydrologic class {name}"));
        }
        let desc = parts
            .next()
            .map(|rest| {
                rest.strip_prefix(b"/")
                    .ok_or("invalid HRC description delimiter")
            })
            .transpose()?
            .unwrap_or_default()
            .trim_ascii();
        let description = match std::str::from_utf8(desc) {
            Ok(text) => text.to_owned(),
            Err(_) => desc.iter().copied().map(char::from).collect(),
        };
        classes.push(HydrologicClass {
            name: name.into(),
            description,
        });
    }
    if classes.is_empty() {
        return Err("missing hydrologic classes".into());
    }
    Ok(classes)
}

fn class_name(bytes: &[u8]) -> Result<&str, String> {
    let name = bytes.trim_ascii();
    if name.is_empty() || name.len() > 9 || !name.iter().all(u8::is_ascii_graphic) {
        return Err(
            "hydrologic class name must contain 1 to 9 printable ASCII nonspace characters".into(),
        );
    }
    std::str::from_utf8(name).map_err(|_| "invalid hydrologic class".into())
}

fn soil_number(bytes: &[u8], implicit_decimals: usize) -> Result<f64, String> {
    let mut value = number(bytes)?;
    if !bytes.contains(&b'.') {
        value /= 10.0_f64.powi(implicit_decimals as i32);
    }
    if value < 0.0 || !(value as f32).is_finite() {
        return Err("soil parameter must be finite and nonnegative".into());
    }
    Ok(value)
}

pub(super) fn baseline_classes(bytes: &[u8], count: usize) -> Result<Vec<String>, String> {
    let names: Vec<_> = lines(bytes)
        .skip(2)
        .take(count)
        .map(|(_, row)| {
            class_name(row.get(..10).ok_or("short soil class record")?).map(str::to_owned)
        })
        .collect::<Result<_, _>>()?;
    if names.len() != count || names.iter().collect::<HashSet<_>>().len() != count {
        return Err(
            "baseline soil file requires distinct classes for every geometry BLC column".into(),
        );
    }
    Ok(names)
}

impl SoilFile {
    pub(crate) fn parse(
        bytes: Vec<u8>,
        count: usize,
        names: &[impl AsRef<str>],
    ) -> Result<Self, String> {
        let mut records = lines(&bytes);
        let mut watersheds = Vec::new();
        let mut offsets = Vec::new();
        for id in 1..=count {
            let mut read = || records.next().ok_or("missing soil record");
            let (range, title) = read()?;
            let title = if range.start == 0 {
                title.strip_prefix(b"\xef\xbb\xbf").unwrap_or(title)
            } else {
                title
            };
            let title = title
                .trim_ascii()
                .strip_prefix(b"Watershed")
                .filter(|number| number.starts_with(b" "))
                .ok_or_else(|| format!("missing soil Watershed {id}"))?;
            if parse::<usize>(title)? != id {
                return Err(format!("expected soil Watershed {id}"));
            }
            if !read()?
                .1
                .split(|b| b.is_ascii_whitespace())
                .filter(|col| !col.is_empty())
                .eq(HEADER.split(|b| *b == b' ').filter(|col| !col.is_empty()))
            {
                return Err(format!("invalid soil header for Watershed {id}"));
            }
            let mut class_offsets = Vec::new();
            let mut classes = Vec::new();
            for name in names {
                let (range, row) = read()?;
                if row.len() < 66 || !row[66..].iter().all(|byte| *byte == b' ') {
                    return Err(format!("invalid soil class record for Watershed {id}"));
                }
                let name = name.as_ref();
                if class_name(&row[..10])? != name {
                    return Err(format!(
                        "expected hydrologic class {name} in Watershed {id}"
                    ));
                }
                let mut fields = [0.0; 7];
                for ((range, _), field) in CLASS_FIELDS.iter().zip(&mut fields) {
                    *field = soil_number(&row[range.clone()], 3)
                        .map_err(|e| format!("Watershed {id}, {name}: {e}"))?;
                }
                classes.push(fields);
                class_offsets.push(range.start);
            }
            let mut coefficients = [0.0; 4];
            let mut coefficient_offsets = Vec::new();
            for ((label, implicit_decimals, _), value) in COEFFICIENTS.iter().zip(&mut coefficients)
            {
                let (range, row) = read()?;
                if row.len() < 18 || row[..10].trim_ascii() != label.as_bytes() {
                    return Err(format!("expected {label} in Watershed {id}"));
                }
                // The engine only reads bytes 10..18, even on longer legacy QB records.
                *value = soil_number(&row[10..18], *implicit_decimals)
                    .map_err(|e| format!("Watershed {id}, {label}: {e}"))?;
                let end = if *label == "QB_M3/SKM2" {
                    20.min(row.len())
                } else {
                    18
                };
                coefficient_offsets.push(range.start + 10..range.start + end);
            }
            offsets.push(WatershedOffsets {
                classes: class_offsets,
                coefficients: coefficient_offsets,
            });
            watersheds.push(SoilWatershed {
                id,
                classes,
                coefficients,
            });
        }
        if records.next().is_some() {
            return Err("unexpected records after last soil watershed".into());
        }
        drop(records);
        Ok(Self {
            bytes,
            watersheds,
            offsets,
        })
    }

    pub(crate) fn create(count: usize, names: &[impl AsRef<str>]) -> Result<Self, String> {
        let mut bytes = Vec::new();
        let bytes_per_watershed = 176 + 68 * names.len();
        if count > 64 * 1024 * 1024 / bytes_per_watershed {
            return Err(
                "new soil calibration would exceed 64 MiB; check the largest geometry Sub value"
                    .into(),
            );
        }
        for id in 1..=count {
            write!(bytes, "Watershed {id}\r\n").unwrap();
            bytes.extend_from_slice(HEADER);
            bytes.extend_from_slice(b"\r\n");
            for name in names {
                write!(bytes, "{:<10}{:>8}", name.as_ref(), "0.0").unwrap();
                for _ in 1..7 {
                    write!(bytes, "{:>8}", "0.00").unwrap();
                }
                bytes.extend_from_slice(b"\r\n");
            }
            for (label, _, decimals) in COEFFICIENTS {
                let zero = format!("{:.decimals$}", 0.0);
                write!(bytes, "{label:<10}{zero:>8}\r\n").unwrap();
            }
        }
        Self::parse(bytes, count, names)
    }

    pub(crate) fn watersheds(&self) -> &[SoilWatershed] {
        &self.watersheds
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn apply(&mut self, changes: Vec<SoilWatershed>) -> Result<usize, String> {
        let mut updates = Vec::new();
        let mut replacements = Vec::new();
        for mut change in changes {
            let id = change.id;
            let existing = &self.watersheds[id - 1];
            let offsets = &self.offsets[id - 1];
            for (class, fields) in change.classes.iter_mut().enumerate() {
                let start = offsets.classes[class];
                for (index, (range, decimals)) in CLASS_FIELDS.iter().enumerate() {
                    let range = start + range.start..start + range.end;
                    if let Some(value) = update(
                        existing.classes[class][index],
                        &mut fields[index],
                        range.len(),
                        *decimals,
                    )
                    .map_err(|e| {
                        format!(
                            "Watershed {id}, class {} field {}: {e}",
                            class + 1,
                            index + 1
                        )
                    })? {
                        replacements.push((range, value));
                    }
                }
            }
            for (index, ((name, _, decimals), range)) in
                COEFFICIENTS.iter().zip(&offsets.coefficients).enumerate()
            {
                if let Some(mut value) = update(
                    existing.coefficients[index],
                    &mut change.coefficients[index],
                    8,
                    *decimals,
                )
                .map_err(|e| format!("Watershed {id}, {name}: {e}"))?
                {
                    value.resize(range.len(), b' ');
                    replacements.push((range.clone(), value));
                }
            }
            if change.classes != existing.classes || change.coefficients != existing.coefficients {
                updates.push(change);
            }
        }
        for (range, replacement) in replacements {
            self.bytes[range].copy_from_slice(&replacement);
        }
        let count = updates.len();
        for change in updates {
            let index = change.id - 1;
            self.watersheds[index] = change;
        }
        Ok(count)
    }
}

fn update(
    old: f64,
    new: &mut f64,
    width: usize,
    decimals: usize,
) -> Result<Option<Vec<u8>>, String> {
    if old == *new {
        return Ok(None);
    }
    if !new.is_finite() || *new < 0.0 {
        return Err("soil parameter must be finite and nonnegative".into());
    }
    let field = formatted(format!("{new:.decimals$}"), width)?;
    *new = number(&field)?;
    Ok((old != *new).then_some(field))
}
