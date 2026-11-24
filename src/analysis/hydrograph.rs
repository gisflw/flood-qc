use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationBias {
    #[default]
    None,
    Recent {
        max_age_hours: u32,
    },
    Ar1 {
        #[serde(default = "default_alpha")]
        alpha: f64,
    },
}

fn default_alpha() -> f64 {
    0.97
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatingPoint {
    pub level_cm: f64,
    pub flow_m3s: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HydrographAdjustment {
    pub bias: ObservationBias,
    pub offset: f64,
    pub band_percent: Option<f64>,
    pub rating_curve: Vec<RatingPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HydrographThreshold {
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HydrographPoint {
    pub timestamp: NaiveDateTime,
    pub raw: Option<f64>,
    pub value: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub segment: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HydrographPeak {
    pub timestamp: NaiveDateTime,
    pub value: f64,
    pub threshold: Option<String>,
}

pub fn threshold_status(value: f64, thresholds: &[HydrographThreshold]) -> Option<&str> {
    thresholds
        .iter()
        .filter(|threshold| value >= threshold.value)
        .max_by(|a, b| a.value.total_cmp(&b.value))
        .map(|threshold| threshold.name.as_str())
}

pub fn validate_hydrograph(
    settings: &HydrographAdjustment,
    thresholds: &[HydrographThreshold],
) -> Result<(), String> {
    let valid_bias = match settings.bias {
        ObservationBias::None | ObservationBias::Recent { .. } => true,
        ObservationBias::Ar1 { alpha } => (0.0..=1.0).contains(&alpha),
    };
    if !settings.offset.is_finite()
        || settings
            .band_percent
            .is_some_and(|percent| !percent.is_finite() || percent < 0.0)
        || !valid_bias
    {
        return Err("invalid hydrograph adjustment".into());
    }
    let curve = &settings.rating_curve;
    if curve.len() == 1
        || curve
            .iter()
            .any(|v| !v.level_cm.is_finite() || !v.flow_m3s.is_finite() || v.flow_m3s < 0.0)
        || curve
            .windows(2)
            .any(|v| v[0].level_cm >= v[1].level_cm || v[0].flow_m3s >= v[1].flow_m3s)
    {
        return Err("rating curve needs increasing finite level and flow pairs".into());
    }
    for (index, threshold) in thresholds.iter().enumerate() {
        if threshold.name.trim().is_empty()
            || !threshold.value.is_finite()
            || thresholds[..index].iter().any(|v| v.name == threshold.name)
        {
            return Err("invalid or duplicate hydrograph threshold".into());
        }
    }
    Ok(())
}

pub fn rating_value(value: f64, flow_to_level: bool, curve: &[RatingPoint]) -> Option<f64> {
    if curve.is_empty() {
        return Some(value);
    }
    let position = |point: &RatingPoint| {
        if flow_to_level {
            (point.flow_m3s, point.level_cm)
        } else {
            (point.level_cm, point.flow_m3s)
        }
    };
    for p in curve.windows(2) {
        let ((start, from), (end, to)) = (position(&p[0]), position(&p[1]));
        if (start..=end).contains(&value) {
            let fraction = (value - start) / (end - start);
            return Some(from * (1.0 - fraction) + to * fraction);
        }
    }
    None
}

type AdjustedHydrograph = (
    Vec<HydrographPoint>,
    Option<HydrographPeak>,
    Option<NaiveDateTime>,
);

pub fn analyze_model_hydrograph(
    mut points: Vec<HydrographPoint>,
    observations: &BTreeMap<NaiveDateTime, f64>,
    reference: NaiveDateTime,
    flow_to_level: bool,
    settings: &HydrographAdjustment,
    thresholds: &[HydrographThreshold],
) -> Result<AdjustedHydrograph, String> {
    validate_hydrograph(settings, thresholds)?;
    if observations.values().any(|value| !value.is_finite())
        || points
            .iter()
            .any(|point| point.raw.is_some_and(|value| !value.is_finite()))
    {
        return Err("hydrograph requires finite values or explicit gaps".into());
    }
    if points.windows(2).any(|v| v[0].timestamp >= v[1].timestamp) {
        return Err("hydrograph times must be strictly ordered".into());
    }
    let residual = |point: &HydrographPoint| {
        if point.segment != Some(0) || point.timestamp > reference {
            return None;
        }
        observations
            .get(&point.timestamp)
            .copied()
            .zip(point.raw)
            .map(|(obs, model)| obs - model)
    };
    let mut anchor = None;
    let recent = if let ObservationBias::Recent { max_age_hours } = settings.bias {
        points
            .iter()
            .rev()
            .find_map(|point| {
                let age = reference - point.timestamp;
                if age > chrono::Duration::hours(i64::from(max_age_hours)) {
                    return None;
                }
                residual(point).inspect(|_| {
                    anchor = Some(point.timestamp);
                })
            })
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let mut state = None;
    let mut previous: Option<NaiveDateTime> = None;
    let mut peak: Option<HydrographPeak> = None;
    for point in &mut points {
        let bias = if let ObservationBias::Ar1 { alpha } = settings.bias {
            let elapsed = previous.map_or(1.0, |last| {
                (point.timestamp - last).num_seconds() as f64 / 3600.0
            });
            let measured = residual(point);
            if measured.is_some() {
                anchor = Some(point.timestamp);
            }
            state = match (state, measured) {
                (Some(old), Some(error)) => Some(alpha.powf(elapsed) * old + (1.0 - alpha) * error),
                (Some(old), None) => Some(alpha.powf(elapsed) * old),
                (None, error) => error,
            };
            state.unwrap_or(0.0)
        } else {
            recent
        };
        previous = Some(point.timestamp);
        point.value = None;
        point.lower = None;
        point.upper = None;
        if let Some(raw) = point.raw {
            let adjusted = raw + bias + settings.offset;
            if !adjusted.is_finite() {
                return Err("hydrograph adjustment overflow".into());
            }
            point.value = rating_value(adjusted, flow_to_level, &settings.rating_curve);
        }
        if let Some(value) = point.value {
            if let Some(percent) = settings.band_percent {
                let difference = value.abs() * (percent / 100.0);
                let lower = value - difference;
                let upper = value + difference;
                if !lower.is_finite() || !upper.is_finite() {
                    return Err("hydrograph band overflow".into());
                }
                point.lower = Some(lower);
                point.upper = Some(upper);
            }
            if point.segment == Some(1) && peak.as_ref().is_none_or(|p| value > p.value) {
                peak = Some(HydrographPeak {
                    timestamp: point.timestamp,
                    value,
                    threshold: threshold_status(value, thresholds).map(str::to_owned),
                });
            }
        }
    }
    Ok((points, peak, anchor))
}
