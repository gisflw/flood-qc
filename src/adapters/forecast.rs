use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::store::{ForecastProvider, ForecastSource, GridSeries};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum NormalizationError {
    Io(std::io::Error),
    Format(String),
    UnsupportedProduct(String),
    Metadata(String),
    CycleMismatch(String),
    GridMismatch(String),
    ConflictingDuplicate(String),
    Coverage(String),
    InvalidRequest(String),
}
impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for NormalizationError {}
impl From<std::io::Error> for NormalizationError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizationRequest {
    pub provider: ForecastProvider,
    pub model: String,
    pub product: String,
    pub cycle_utc: DateTime<Utc>,
    pub operation_timezone: chrono_tz::Tz,
    pub crop_bbox: [f64; 4],
}
#[derive(Clone, Debug)]
pub struct ForecastMessageEvidence {
    pub band: usize,
    pub start_lead_seconds: i64,
    pub end_lead_seconds: i64,
    pub product_template: u16,
    pub units: String,
    pub selected: bool,
    pub metadata: Vec<(String, String)>,
}
#[derive(Clone, Debug)]
pub struct NormalizedForecast {
    pub source: ForecastSource,
    pub native: GridSeries,
    pub messages: Vec<ForecastMessageEvidence>,
    pub clipped_negative_count: usize,
    pub clipped_negative_total_mm: f64,
}

// Terrio uses the first message's grid for all bands, including mismatched bands.
fn validate_grid_sections<R: Read + Seek>(input: &mut R) -> Result<usize, NormalizationError> {
    let fail = |s: &str| NormalizationError::Format(s.into());
    let length = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(0))?;
    let mut offset = 0u64;
    let mut expected: Option<Vec<u8>> = None;
    let mut messages = 0usize;
    while offset < length {
        if length - offset < 20 {
            return Err(fail("truncated GRIB2 message"));
        }
        let mut header = [0u8; 16];
        input.read_exact(&mut header)?;
        if &header[..4] != b"GRIB" || header[7] != 2 {
            return Err(fail("expected GRIB2 header"));
        }
        if header[6] != 0 {
            return Err(NormalizationError::Metadata(
                "expected meteorological discipline 0".into(),
            ));
        }
        let size = u64::from_be_bytes(header[8..16].try_into().expect("header length"));
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= length && size >= 20)
            .ok_or_else(|| fail("invalid GRIB2 message length"))?;
        let trailer = end - 4;
        let mut position = offset + 16;
        let mut grid = None;
        let mut field_count = 0usize;
        let mut previous = 0u8;
        let mut seen = 0u8;
        while position < trailer {
            if trailer - position < 5 {
                return Err(fail("truncated GRIB2 section header"));
            }
            let mut section_header = [0u8; 5];
            input.read_exact(&mut section_header)?;
            let section_size = u64::from(u32::from_be_bytes(
                section_header[..4]
                    .try_into()
                    .expect("section header length"),
            ));
            let section = section_header[4];
            let next = position
                .checked_add(section_size)
                .filter(|n| section_size >= 5 && *n <= trailer)
                .ok_or_else(|| fail("invalid GRIB2 section length"))?;
            if !(1..=7).contains(&section) || section <= previous {
                return Err(fail(
                    "unsupported GRIB2 section ordering or multi-field message",
                ));
            }
            previous = section;
            seen |= 1 << section;
            if section == 3 {
                if !(14..=1_048_576).contains(&section_size) {
                    return Err(fail("invalid grid section length"));
                }
                let mut bytes = Vec::with_capacity(section_size as usize);
                bytes.extend_from_slice(&section_header);
                bytes.resize(section_size as usize, 0);
                input.read_exact(&mut bytes[5..])?;
                if u16::from_be_bytes([bytes[12], bytes[13]]) != 0 {
                    return Err(fail("unsupported grid template"));
                }
                grid = Some(bytes);
            } else {
                input.seek(SeekFrom::Start(next))?;
            }
            if section == 7 {
                field_count += 1;
            }
            position = next;
        }
        let mut marker = [0u8; 4];
        input.read_exact(&mut marker)?;
        if &marker != b"7777"
            || field_count != 1
            || previous != 7
            || (seen & 0b1111_1010) != 0b1111_1010
        {
            return Err(fail("invalid GRIB2 trailer or field"));
        }
        let grid = grid.ok_or_else(|| fail("missing grid section"))?;
        if let Some(old) = &expected {
            if old != &grid {
                return Err(NormalizationError::GridMismatch(format!(
                    "message {} grid differs",
                    messages + 1
                )));
            }
        } else {
            expected = Some(grid);
        }
        offset = end;
        messages += 1;
    }
    if messages == 0 {
        return Err(fail("empty GRIB2 source"));
    }
    Ok(messages)
}

pub fn normalize_saved_forecast(
    path: &Path,
    request: &NormalizationRequest,
) -> Result<NormalizedForecast, NormalizationError> {
    use crate::store::{IntervalAxis, Provenance, RegularGridSpec, TimeInterval};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;
    use terrio::{FloatReadOptions, Window, open_raster_lazy};
    let invalid = |message: &str| NormalizationError::InvalidRequest(message.into());
    if request.product != "fc"
        || !matches!(
            (request.provider, request.model.as_str()),
            (ForecastProvider::Ecmwf, "ifs") | (ForecastProvider::Noaa, "gfs")
        )
    {
        return Err(NormalizationError::UnsupportedProduct(
            "provider/model/product mismatch".into(),
        ));
    }
    let [west, south, east, north] = request.crop_bbox;
    if !request.crop_bbox.iter().all(|v| v.is_finite())
        || west >= east
        || south >= north
        || west < -180.
        || east > 180.
        || south < -90.
        || north > 90.
    {
        return Err(invalid("invalid crop bbox"));
    }
    let mut file = std::fs::File::open(path)?;
    let count = validate_grid_sections(&mut file)?;
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let byte_length = std::io::copy(&mut file, &mut hasher)?;
    let sha256 = format!("{:x}", hasher.finalize());
    let settings = serde_json::to_value(request).map_err(|e| invalid(&e.to_string()))?;
    let settings_bytes = serde_json::to_vec(&settings).map_err(|e| invalid(&e.to_string()))?;
    let mut id = Sha256::new();
    id.update(sha256.as_bytes());
    id.update(&settings_bytes);
    let source_id = format!("{:x}", id.finalize());
    let source = ForecastSource {
        source_id: source_id.clone(),
        provider: request.provider,
        model: request.model.clone(),
        product: request.product.clone(),
        cycle_utc: request.cycle_utc,
        sha256,
        byte_length,
        normalization_json: settings.clone(),
    };
    let mut raster = open_raster_lazy(
        path.to_str()
            .ok_or_else(|| invalid("source path is not UTF-8"))?,
    )
    .map_err(|e| NormalizationError::Format(e.to_string()))?;
    if raster.band_count() != count {
        return Err(NormalizationError::Metadata(
            "band/message count mismatch".into(),
        ));
    }
    let t = raster.geo_transform();
    let resolution = t[1];
    if !raster.crs().is_geographic()
        || !t.iter().all(|v| v.is_finite())
        || resolution <= 0.
        || t[2] != 0.
        || t[4] != 0.
        || t[5] >= 0.
        || (resolution + t[5]).abs() > resolution * 1e-9
    {
        return Err(NormalizationError::GridMismatch(
            "expected square regular geographic grid".into(),
        ));
    }
    let xs: Vec<usize> = (0..raster.width())
        .filter(|&i| {
            let x = t[0] + (i as f64 + 0.5) * resolution;
            x + resolution / 2. >= west && x - resolution / 2. <= east
        })
        .collect();
    let ys: Vec<usize> = (0..raster.height())
        .filter(|&i| {
            let y = t[3] + (i as f64 + 0.5) * t[5];
            y + resolution / 2. >= south && y - resolution / 2. <= north
        })
        .collect();
    if xs.is_empty() || ys.is_empty() {
        return Err(NormalizationError::Coverage(
            "crop does not intersect source".into(),
        ));
    }
    let (x0, x1, y0, y1) = (xs[0], *xs.last().unwrap(), ys[0], *ys.last().unwrap());
    let width = x1 - x0 + 1;
    let height = y1 - y0 + 1;
    let longitudes: Vec<f64> = xs
        .iter()
        .map(|i| t[0] + (*i as f64 + 0.5) * resolution)
        .collect();
    let latitudes: Vec<f64> = ys
        .iter()
        .rev()
        .map(|i| t[3] + (*i as f64 + 0.5) * t[5])
        .collect();
    let grid = RegularGridSpec {
        requested_bbox: request.crop_bbox,
        resolution_degrees: resolution,
        effective_bbox: [
            longitudes[0] - resolution / 2.,
            latitudes[0] - resolution / 2.,
            longitudes[width - 1] + resolution / 2.,
            latitudes[height - 1] + resolution / 2.,
        ],
        latitudes,
        longitudes,
    };
    grid.validate()
        .map_err(|e| NormalizationError::GridMismatch(e.to_string()))?;
    let metadata = raster.band_metadata().to_vec();
    let mut messages = Vec::new();
    let mut duplicate_count = 0usize;
    let mut cumulative: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
    for (index, band) in metadata.iter().enumerate() {
        let get = |key: &str| {
            band.items
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
                .ok_or_else(|| {
                    NormalizationError::Metadata(format!("band {} missing {key}", index + 1))
                })
        };
        let number = |key: &str| -> Result<i64, NormalizationError> {
            get(key)?.parse().map_err(|_| {
                NormalizationError::Metadata(format!("band {} invalid {key}", index + 1))
            })
        };
        let ref_time = number("GRIB_REF_TIME")?;
        let valid_time = number("GRIB_VALID_TIME")?;
        let start = number("GRIB_FORECAST_SECONDS")?;
        let end = valid_time
            .checked_sub(ref_time)
            .ok_or_else(|| NormalizationError::Metadata("lead overflow".into()))?;
        if ref_time != request.cycle_utc.timestamp() {
            return Err(NormalizationError::CycleMismatch(format!(
                "band {} cycle differs",
                index + 1
            )));
        }
        let pdt = number("GRIB_PDS_PDTN")?;
        let assembled: Vec<i64> = get("GRIB_PDS_TEMPLATE_ASSEMBLED_VALUES")?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| NormalizationError::Metadata(format!("band {} invalid PDT", index + 1)))?;
        if pdt != 8
            || assembled.len() < 29
            || assembled[7] != 1
            || assembled[12] != 255
            || assembled[21] != 1
            || assembled[23] != 1
            || assembled[25] != 1
            || assembled[9] != 1
            || start < 0
            || end < start
            || assembled[26].checked_mul(3600) != Some(end - start)
        {
            return Err(NormalizationError::Metadata(format!(
                "band {} unsupported precipitation accumulation metadata",
                index + 1
            )));
        }
        let ids = get("GRIB_IDS")?;
        let units = match request.provider {
            ForecastProvider::Ecmwf
                if ids.contains("CENTER=98(")
                    && ids.split_whitespace().any(|field| field == "LOCAL_TABLE=1")
                    && assembled[0] == 1
                    && assembled[1] == 193 =>
            {
                "m"
            }
            ForecastProvider::Noaa
                if ids.contains("CENTER=7(") && assembled[0] == 1 && assembled[1] == 8 =>
            {
                "kg m^-2"
            }
            _ => {
                return Err(NormalizationError::Metadata(format!(
                    "band {} unrecognized provider or precipitation parameter",
                    index + 1
                )));
            }
        };
        let selected = start == 0;
        messages.push(ForecastMessageEvidence {
            band: index + 1,
            start_lead_seconds: start,
            end_lead_seconds: end,
            product_template: 8,
            units: units.into(),
            selected,
            metadata: band.items.clone(),
        });
        if !selected {
            continue;
        }
        let data = raster
            .read_window_f64(
                Window::new(x0, y0, width, height),
                &FloatReadOptions {
                    bands: vec![index + 1],
                    ..Default::default()
                },
            )
            .map_err(|e| NormalizationError::Format(format!("band {}: {e}", index + 1)))?;
        let mut values = Vec::with_capacity(data.len());
        for row in data.chunks_exact(width).rev() {
            for v in row {
                if v.is_infinite() {
                    return Err(NormalizationError::Metadata(format!(
                        "band {} infinite value",
                        index + 1
                    )));
                }
                values.push(if request.provider == ForecastProvider::Ecmwf {
                    *v * 1000.
                } else {
                    *v
                });
            }
        }
        if let Some(old) = cumulative.get(&end) {
            duplicate_count += 1;
            if old.len() != values.len()
                || old
                    .iter()
                    .zip(&values)
                    .any(|(a, b)| a != b && !(a.is_nan() && b.is_nan()))
            {
                return Err(NormalizationError::ConflictingDuplicate(format!(
                    "lead {end} seconds"
                )));
            }
        } else {
            cumulative.insert(end, values);
        }
    }
    if cumulative.is_empty() {
        return Err(NormalizationError::Coverage("no cumulative fields".into()));
    }
    let cells = width
        .checked_mul(height)
        .ok_or_else(|| invalid("grid shape overflow"))?;
    let mut previous = vec![0.; cells];
    let mut previous_end = 0i64;
    let mut values = Vec::new();
    let mut valid = Vec::new();
    let mut bounds = Vec::new();
    let mut clipped_negative_count = 0;
    let mut clipped_negative_total_mm = 0.;
    for (end, current) in cumulative {
        if end == 0 {
            previous = current;
            continue;
        }
        if end <= previous_end {
            return Err(NormalizationError::Coverage("non-increasing lead".into()));
        }
        let start_time = request
            .cycle_utc
            .checked_add_signed(
                chrono::Duration::try_seconds(previous_end)
                    .ok_or_else(|| invalid("lead overflow"))?,
            )
            .ok_or_else(|| invalid("lead overflow"))?
            .with_timezone(&request.operation_timezone)
            .naive_local();
        let end_time = request
            .cycle_utc
            .checked_add_signed(
                chrono::Duration::try_seconds(end).ok_or_else(|| invalid("lead overflow"))?,
            )
            .ok_or_else(|| invalid("lead overflow"))?
            .with_timezone(&request.operation_timezone)
            .naive_local();
        bounds.push(TimeInterval {
            start: start_time,
            end: end_time,
        });
        for (a, b) in current.iter().zip(&previous) {
            let d = *a - *b;
            if d.is_nan() {
                values.push(f64::NAN);
                valid.push(false);
            } else if d.is_finite() {
                if d < 0. {
                    clipped_negative_count += 1;
                    clipped_negative_total_mm -= d;
                }
                values.push(d.max(0.));
                valid.push(true);
            } else {
                return Err(NormalizationError::Metadata(
                    "infinite accumulated difference".into(),
                ));
            }
        }
        previous = current;
        previous_end = end;
    }
    if !clipped_negative_total_mm.is_finite() {
        return Err(NormalizationError::Metadata(
            "clipped magnitude overflow".into(),
        ));
    }
    let mut processing = serde_json::Map::new();
    processing.insert("normalization".into(), settings);
    processing.insert(
        "clipped_negative_count".into(),
        clipped_negative_count.into(),
    );
    processing.insert(
        "clipped_negative_total_mm".into(),
        clipped_negative_total_mm.into(),
    );
    processing.insert(
        "selected_message_count".into(),
        messages.iter().filter(|m| m.selected).count().into(),
    );
    processing.insert(
        "ignored_message_count".into(),
        messages.iter().filter(|m| !m.selected).count().into(),
    );
    processing.insert("duplicate_message_count".into(), duplicate_count.into());
    let native = GridSeries {
        timeline: IntervalAxis {
            time_zone: request.operation_timezone.name().into(),
            bounds,
        },
        grid,
        values,
        valid,
        provenance: Provenance {
            source_ids: vec![source_id],
            processing: processing.into(),
        },
    };
    native
        .validate()
        .map_err(|e| NormalizationError::InvalidRequest(e.to_string()))?;
    Ok(NormalizedForecast {
        source,
        native,
        messages,
        clipped_negative_count,
        clipped_negative_total_mm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn msg(grid: u8) -> Vec<u8> {
        let mut body = Vec::new();
        for id in [1u8, 3, 4, 5, 6, 7] {
            let len = if id == 3 { 14u32 } else { 5u32 };
            body.extend(len.to_be_bytes());
            body.push(id);
            if id == 3 {
                body.extend([grid, 0, 0, 0, 0, 0, 0, 0, 0]);
            }
        }
        let mut message = b"GRIB\0\0\0\x02".to_vec();
        message.extend(((16 + body.len() + 4) as u64).to_be_bytes());
        message.extend(body);
        message.extend(b"7777");
        message
    }
    #[test]
    fn stable_grids_and_malformed_sources() {
        let one = msg(1);
        let mut same = one.clone();
        same.extend(msg(1));
        assert_eq!(validate_grid_sections(&mut Cursor::new(same)).unwrap(), 2);
        let mut changed = one.clone();
        changed.extend(msg(2));
        let mismatch = matches!(
            validate_grid_sections(&mut Cursor::new(changed)),
            Err(NormalizationError::GridMismatch(_))
        );
        assert!(mismatch);
        assert!(validate_grid_sections(&mut Cursor::new(&one[..one.len() - 1])).is_err());
        let mut bad = one.clone();
        bad[16..20].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(validate_grid_sections(&mut Cursor::new(bad)).is_err());
        let mut bad = one.clone();
        bad[one.len() - 1] = 0;
        assert!(validate_grid_sections(&mut Cursor::new(bad)).is_err());
    }
}
