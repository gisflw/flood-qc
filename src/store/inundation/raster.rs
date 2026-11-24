use super::{format_error, path_str, schema, validate_grid};
use crate::store::{StoreError, digest_file};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use terrio::{DataType, LazyRaster, Window, WindowOptions, raster::RasterData};

pub(crate) struct InundationRasters {
    hand: InputRaster,
    catchments: InputRaster,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) epsg: i32,
    pub(crate) transform: [f64; 6],
}

type SidecarHash = (String, String);

struct InputRaster {
    path: PathBuf,
    raster: LazyRaster,
    pub(crate) sha256: String,
    pub(crate) sidecars: Vec<SidecarHash>,
}

impl InputRaster {
    fn open(path: &Path) -> Result<Self, StoreError> {
        let path = path.canonicalize()?;
        if !path.is_file() {
            return Err(schema(format!(
                "expected a GeoTIFF file: {}",
                path.display()
            )));
        }
        let (sha256, _) = digest_file(&path).map_err(format_error)?;
        let name = path_str(&path)?;
        // Select the TIFF driver by inspecting headers before allowing a normal open.
        if LazyRaster::open_tiff(name).map_err(format_error)?.is_none() {
            return Err(schema(format!(
                "expected a GeoTIFF file: {}",
                path.display()
            )));
        }
        let raster = terrio::open_raster_lazy(name).map_err(format_error)?;
        if raster.band_count() != 1 || raster.is_complex() || !raster.has_geo_transform() {
            return Err(schema(format!(
                "expected one georeferenced real band: {}",
                path.display()
            )));
        }
        let mut sidecars = Vec::new();
        for file in raster.file_list().map_err(format_error)? {
            if file == name || file.ends_with(".ovr") || file.ends_with(".ovr.msk") {
                continue; // Input overviews are never read.
            }
            let basename = Path::new(&file)
                .file_name()
                .ok_or_else(|| schema("raster sidecar filename is missing"))?;
            sidecars.push((
                basename.to_string_lossy().into_owned(),
                digest_file(Path::new(&file)).map_err(format_error)?.0,
            ));
        }
        sidecars.sort_unstable();
        Ok(Self {
            path,
            raster,
            sha256,
            sidecars,
        })
    }

    fn read(&mut self, window: Window) -> Result<(RasterData, Option<Vec<u8>>), StoreError> {
        let data = self
            .raster
            .read_window(window, &WindowOptions::default())
            .map_err(format_error)?
            .raster
            .data;
        let mask = self.raster.read_mask(window, None).map_err(format_error)?;
        Ok((data, mask))
    }

    fn verify(&self) -> Result<(), StoreError> {
        let current = Self::open(&self.path)?;
        if current.sha256 != self.sha256 || current.sidecars != self.sidecars {
            return Err(StoreError::Integrity(format!(
                "raster inputs changed during processing: {}",
                self.path.display()
            )));
        }
        Ok(())
    }
}

impl InundationRasters {
    pub(crate) fn open(hand: &Path, catchments: &Path) -> Result<Self, StoreError> {
        let hand = InputRaster::open(hand)?;
        let catchments = InputRaster::open(catchments)?;
        let width = hand.raster.width();
        let height = hand.raster.height();
        let epsg = hand
            .raster
            .crs()
            .epsg
            .and_then(|n| i32::try_from(n).ok())
            .unwrap_or_default();
        let transform = hand.raster.geo_transform();
        validate_grid(width, height, epsg, transform)?;
        if catchments.raster.width() != width
            || catchments.raster.height() != height
            || catchments.raster.crs().epsg != Some(epsg as u32)
            || catchments.raster.geo_transform() != transform
        {
            return Err(schema(
                "HAND and catchments must share the same pixel grid and assigned CRS",
            ));
        }
        let catchment_type = catchments.raster.data_type();
        if matches!(catchment_type, DataType::F32 | DataType::F64) {
            return Err(schema("catchments must be stored in an integer band"));
        }
        if matches!(catchment_type, DataType::I64 | DataType::U64)
            && catchments
                .raster
                .nodata()
                .is_some_and(|value| value.abs() >= 9_007_199_254_740_992.0)
        {
            return Err(schema(
                "catchment NoData exceeds the exact TIFF metadata range",
            ));
        }
        Ok(Self {
            hand,
            catchments,
            width,
            height,
            epsg,
            transform,
        })
    }

    pub(crate) fn input_hashes(&self) -> (&str, &str) {
        (&self.hand.sha256, &self.catchments.sha256)
    }

    pub(crate) fn sidecar_hashes(&self) -> (&[SidecarHash], &[SidecarHash]) {
        (&self.hand.sidecars, &self.catchments.sidecars)
    }

    pub(crate) fn verify(&self) -> Result<(), StoreError> {
        self.hand.verify()?;
        self.catchments.verify()
    }

    pub(crate) fn read_cells(
        &mut self,
        window: Window,
        minis: &HashMap<i64, usize>,
    ) -> Result<Vec<Option<(f64, usize)>>, StoreError> {
        let (hands, hand_mask) = self.hand.read(window)?;
        let (catchments, catchment_mask) = self.catchments.read(window)?;
        let hand_nodata = self.hand.raster.nodata();
        let catchment_nodata = self.catchments.raster.nodata();
        let mut cells = Vec::with_capacity(window.width * window.height);
        let push = |i: usize, id: i128| -> Result<(), StoreError> {
            if id == 0
                || catchment_mask.as_ref().is_some_and(|mask| mask[i] == 0)
                || catchment_nodata.is_some_and(|n| id == n as i128 && n.fract() == 0.)
            {
                cells.push(None);
                return Ok(());
            }
            let mini = i64::try_from(id)
                .ok()
                .and_then(|id| minis.get(&id))
                .ok_or_else(|| {
                    let x = window.x + i % window.width;
                    let y = window.y + i / window.width;
                    StoreError::InvalidValue(format!(
                        "unknown catchment MINI ID {id} at pixel ({x}, {y})"
                    ))
                })?;
            let hand = hands.get_f64(i);
            cells.push(
                (hand.is_finite()
                    && hand > 0.
                    && hand_nodata != Some(hand)
                    && hand_mask.as_ref().is_none_or(|mask| mask[i] != 0))
                .then_some((hand, *mini)),
            );
            Ok(())
        };
        match catchments {
            RasterData::U8(v) => visit_ids(v, push)?,
            RasterData::I8(v) => visit_ids(v, push)?,
            RasterData::U16(v) => visit_ids(v, push)?,
            RasterData::I16(v) => visit_ids(v, push)?,
            RasterData::U32(v) => visit_ids(v, push)?,
            RasterData::I32(v) => visit_ids(v, push)?,
            RasterData::U64(v) => visit_ids(v, push)?,
            RasterData::I64(v) => visit_ids(v, push)?,
            RasterData::F32(_) | RasterData::F64(_) => {
                return Err(schema("catchments must be stored in an integer band"));
            }
        }
        Ok(cells)
    }
}

fn visit_ids(
    values: Vec<impl Into<i128>>,
    mut visit: impl FnMut(usize, i128) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    for (index, id) in values.into_iter().enumerate() {
        visit(index, id.into())?;
    }
    Ok(())
}
