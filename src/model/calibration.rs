mod fixed;
mod geometry;
mod soil;

pub(crate) use self::geometry::GeometryFile;
pub(crate) use self::soil::{SoilFile, parse_classes};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn with_path<T>(path: &Path, result: Result<T, String>) -> Result<T, String> {
    result.map_err(|error| format!("{}: {error}", path.display()))
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    with_path(path, fs::read(path).map_err(|error| error.to_string()))
}

fn replacements(
    input: &Path,
    geometry: Option<&Path>,
    soil: Option<&Path>,
) -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
    if geometry.is_none() && soil.is_none() {
        return Ok(Vec::new());
    }

    let cell_path = input.join("Cell.hig");
    let base = with_path(&cell_path, GeometryFile::parse(read_bytes(&cell_path)?))?;
    let mut replacements = Vec::new();
    if let Some(path) = geometry {
        let selected = with_path(path, GeometryFile::parse(read_bytes(path)?))?;
        let compatible = selected.rows().len() == base.rows().len()
            && selected.block_count() == base.block_count()
            && selected.rows().iter().zip(base.rows()).all(|(new, old)| {
                (
                    new.cat_id,
                    new.mini,
                    new.longitude,
                    new.latitude,
                    new.subbasin,
                ) == (
                    old.cat_id,
                    old.mini,
                    old.longitude,
                    old.latitude,
                    old.subbasin,
                )
            });
        if !compatible {
            return Err(format!(
                "{}: geometry changes ordered MINI, CatID, coordinates, Sub, or BLC count",
                path.display()
            ));
        }

        let gtp_path = input.join("MINI.gtp");
        let gtp = with_path(&gtp_path, crate::store::read_basin_geometry(&gtp_path))?;
        if gtp.mini_ids.len() != base.rows().len()
            || base.rows().iter().enumerate().any(|(index, row)| {
                gtp.mini_ids[index] != row.mini as i64
                    || gtp.longitudes[index] != row.longitude
                    || gtp.latitudes[index] != row.latitude
            })
        {
            return Err(format!(
                "{}: ordered MINI centers differ from Cell.hig",
                gtp_path.display()
            ));
        }
        replacements.push((cell_path, selected.bytes().to_vec()));
        replacements.push((gtp_path, selected.bytes().to_vec()));
        let mgb_path = input.join("MINI.mgb");
        match fs::read(&mgb_path) {
            Ok(bytes) if bytes == base.bytes() => {
                replacements.push((mgb_path, selected.bytes().to_vec()))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", mgb_path.display())),
        }
    }
    if let Some(path) = soil {
        let soil_path = input.join("paruso.hig");
        let baseline = read_bytes(&soil_path)?;
        let count = watershed_count(&base);
        let names = with_path(
            &soil_path,
            soil::baseline_classes(&baseline, base.block_count()),
        )?;
        with_path(&soil_path, SoilFile::parse(baseline, count, &names))?;
        let selected = read_bytes(path)?;
        let selected = with_path(path, SoilFile::parse(selected, count, &names))?;
        replacements.push((soil_path, selected.bytes().to_vec()));
    }
    Ok(replacements)
}

pub(crate) fn watershed_count(geometry: &GeometryFile) -> usize {
    geometry
        .rows()
        .iter()
        .map(|row| row.subbasin)
        .max()
        .unwrap_or(0)
}

pub(crate) fn validate_calibration(
    input: &Path,
    geometry: Option<&Path>,
    soil: Option<&Path>,
) -> Result<(), String> {
    replacements(input, geometry, soil).map(|_| ())
}

pub(crate) fn install_calibration(
    stage_input: &Path,
    geometry: Option<&Path>,
    soil: Option<&Path>,
) -> Result<(), String> {
    for (path, bytes) in replacements(stage_input, geometry, soil)? {
        with_path(&path, crate::store::write_atomic(&path, &bytes))?;
    }
    Ok(())
}
