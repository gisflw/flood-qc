use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CalibrationGeometryRow {
    pub cat_id: usize,
    pub mini: usize,
    pub longitude: f64,
    pub latitude: f64,
    pub subbasin: usize,
    pub area: f64,
    pub upstream_area: f64,
    pub river_length: f64,
    pub river_slope: f64,
    pub longest_length: f64,
    pub longest_slope: f64,
    pub downstream: i64,
    pub order: usize,
    pub hdr: usize,
    pub width: f64,
    pub depth: f64,
    pub manning: f64,
    pub blocks: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HydrologicClass {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SoilWatershed {
    pub id: usize,
    pub classes: Vec<[f64; 7]>,
    pub coefficients: [f64; 4],
}

pub(crate) fn save_calibration(path: &Path, bytes: &[u8]) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path.file_name().ok_or("destination needs a filename")?;
    let temporary = path.with_file_name(format!(
        ".{}.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = written.and_then(|()| fs::hard_link(&temporary, path));
    let _ = fs::remove_file(&temporary);
    #[cfg(unix)]
    let result = result.and_then(|()| {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::File::open(parent)?.sync_all()
    });
    result.map_err(|e| format!("{}: {e}", path.display()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryField {
    Area,
    UpstreamArea,
    RiverLength,
    RiverSlope,
    LongestLength,
    LongestSlope,
    Downstream,
    Order,
    Hdr,
    Width,
    Depth,
    Manning,
    Block(usize),
}
