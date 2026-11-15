use crate::store::{
    BasinForcing, BasinGeometry, GridSeries, IdwSettings, RegularGridSpec, ValueError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialError(pub String);
impl std::fmt::Display for SpatialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SpatialError {}
impl From<ValueError> for SpatialError {
    fn from(value: ValueError) -> Self {
        Self(value.0)
    }
}
fn inverse_weight(distance: f64, power: f64) -> f64 {
    // NumPy squares directly for power two; powf can differ by an ULP.
    1.0 / if power == 2.0 {
        distance * distance
    } else {
        distance.powf(power)
    }
}
fn error(message: &str) -> SpatialError {
    SpatialError(message.into())
}

pub fn interpolate_stations(
    ids: &[String],
    lons: &[f64],
    lats: &[f64],
    values: &[f64],
    target_lons: &[f64],
    target_lats: &[f64],
    settings: &IdwSettings,
) -> Result<Vec<f64>, SpatialError> {
    settings.validate()?;
    if ids.is_empty()
        || ids.len() != lons.len()
        || ids.len() != lats.len()
        || ids.len() != values.len()
        || target_lons.len() != target_lats.len()
        || lons
            .iter()
            .chain(lats)
            .chain(values)
            .chain(target_lons)
            .chain(target_lats)
            .any(|v| !v.is_finite())
    {
        return Err(error("invalid station interpolation input"));
    }
    let mut result = Vec::with_capacity(target_lons.len());
    for (&x, &y) in target_lons.iter().zip(target_lats) {
        let mut neighbors: Vec<_> = lons
            .iter()
            .zip(lats)
            .enumerate()
            .map(|(i, (&sx, &sy))| ((x - sx).hypot(y - sy), i))
            .collect();
        neighbors.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| ids[a.1].cmp(&ids[b.1])));
        neighbors.truncate(settings.nearest_stations);
        let exact: Vec<_> = neighbors.iter().filter(|(d, _)| *d == 0.0).collect();
        let value = if exact.is_empty() {
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            for (distance, i) in neighbors {
                let weight = inverse_weight(distance, settings.power);
                sum += weight * values[i];
                weight_sum += weight;
            }
            sum / weight_sum
        } else {
            exact.iter().map(|(_, i)| values[*i]).sum::<f64>() / exact.len() as f64
        };
        if !value.is_finite() {
            return Err(error("station interpolation has no finite coverage"));
        }
        result.push(value);
    }
    Ok(result)
}
#[derive(Clone, Debug)]
pub struct GridIdwNeighbors {
    pub indices: Vec<Vec<usize>>,
    pub weights: Vec<Vec<f64>>,
}
pub fn build_grid_idw_neighbors(
    grid: &RegularGridSpec,
    basins: &BasinGeometry,
    settings: &IdwSettings,
) -> Result<GridIdwNeighbors, SpatialError> {
    grid.validate()?;
    basins.validate()?;
    settings.validate()?;
    let mut indices = Vec::with_capacity(basins.mini_ids.len());
    let mut weights = Vec::with_capacity(basins.mini_ids.len());
    for (&x, &y) in basins.longitudes.iter().zip(&basins.latitudes) {
        let mut neighbors: Vec<_> = grid
            .latitudes
            .iter()
            .flat_map(|&lat| {
                grid.longitudes
                    .iter()
                    .map(move |&lon| (x - lon).hypot(y - lat))
            })
            .enumerate()
            .map(|(i, d)| (d, i))
            .collect();
        neighbors.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        neighbors.truncate(settings.nearest_stations);
        indices.push(neighbors.iter().map(|(_, i)| *i).collect());
        let row: Vec<_> = neighbors
            .iter()
            .map(|(d, _)| inverse_weight(if *d == 0.0 { 1e-12_f64 } else { *d }, settings.power))
            .collect();
        if row.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err(error("nonfinite IDW weight"));
        }
        weights.push(row);
    }
    Ok(GridIdwNeighbors { indices, weights })
}
pub fn interpolate_grid_to_basins(
    grid: &GridSeries,
    basins: &BasinGeometry,
    settings: &IdwSettings,
) -> Result<BasinForcing, SpatialError> {
    grid.validate()?;
    let neighbors = build_grid_idw_neighbors(&grid.grid, basins, settings)?;
    let cells = grid.grid.latitudes.len() * grid.grid.longitudes.len();
    let size = grid
        .timeline
        .bounds
        .len()
        .checked_mul(basins.mini_ids.len())
        .ok_or_else(|| error("forcing shape overflow"))?;
    let mut values = Vec::with_capacity(size);
    for t in 0..grid.timeline.bounds.len() {
        for b in 0..basins.mini_ids.len() {
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            for (&i, &weight) in neighbors.indices[b].iter().zip(&neighbors.weights[b]) {
                if grid.valid[t * cells + i] {
                    sum += grid.values[t * cells + i] * weight;
                    weight_sum += weight;
                }
            }
            let value = sum / weight_sum;
            if !value.is_finite() {
                return Err(SpatialError(format!(
                    "no coverage at time {t}, basin {}",
                    basins.mini_ids[b]
                )));
            }
            values.push(value);
        }
    }
    let forcing = BasinForcing {
        timeline: grid.timeline.clone(),
        basins: basins.clone(),
        valid: vec![true; size],
        values,
        provenance: grid.provenance.clone(),
    };
    forcing.validate()?;
    Ok(forcing)
}
fn bracket(axis: &[f64], target: f64) -> Option<(usize, usize)> {
    if target < axis[0] || target > axis[axis.len() - 1] {
        return None;
    }
    match axis.binary_search_by(|v| v.total_cmp(&target)) {
        Ok(i) => Some((i, i)),
        Err(i) => Some((i - 1, i)),
    }
}
fn linear(a: f64, b: f64, x0: f64, x1: f64, x: f64) -> f64 {
    if x0 == x1 {
        a
    } else {
        (b - a) / (x1 - x0) * (x - x0) + a
    }
}
pub fn bilinear_resample(
    values: &[f64],
    valid: &[bool],
    source_lats: &[f64],
    source_lons: &[f64],
    target_lats: &[f64],
    target_lons: &[f64],
) -> Result<(Vec<f64>, Vec<bool>), SpatialError> {
    if source_lats.is_empty()
        || source_lons.is_empty()
        || source_lats.len().checked_mul(source_lons.len()) != Some(values.len())
        || values.len() != valid.len()
        || source_lats
            .iter()
            .chain(source_lons)
            .chain(target_lats)
            .chain(target_lons)
            .any(|v| !v.is_finite())
        || source_lats
            .windows(2)
            .chain(source_lons.windows(2))
            .any(|p| p[0] >= p[1])
        || values.iter().zip(valid).any(|(v, m)| *m && !v.is_finite())
    {
        return Err(error("invalid bilinear input"));
    }
    let size = target_lats
        .len()
        .checked_mul(target_lons.len())
        .ok_or_else(|| error("resampling shape overflow"))?;
    let mut result = vec![0.0; size];
    let mut masks = vec![false; size];
    for (row, &y) in target_lats.iter().enumerate() {
        for (column, &x) in target_lons.iter().enumerate() {
            let (Some((y0, y1)), Some((x0, x1))) =
                (bracket(source_lats, y), bracket(source_lons, x))
            else {
                continue;
            };
            let indices = [
                y0 * source_lons.len() + x0,
                y0 * source_lons.len() + x1,
                y1 * source_lons.len() + x0,
                y1 * source_lons.len() + x1,
            ];
            if indices.iter().any(|i| !valid[*i]) {
                continue;
            }
            let low = linear(
                values[indices[0]],
                values[indices[1]],
                source_lons[x0],
                source_lons[x1],
                x,
            );
            let high = linear(
                values[indices[2]],
                values[indices[3]],
                source_lons[x0],
                source_lons[x1],
                x,
            );
            let value = linear(low, high, source_lats[y0], source_lats[y1], y);
            if !value.is_finite() {
                return Err(error("nonfinite bilinear result"));
            }
            let i = row * target_lons.len() + column;
            result[i] = value;
            masks[i] = true;
        }
    }
    Ok((result, masks))
}
