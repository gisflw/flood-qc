use super::OperationControl;
use crate::{
    analysis::{InundationStages, update_tile, wet_fraction},
    store::{
        FLOOD_MASK_FILE, FLOOD_SOURCE_DIAGNOSTIC, FLOOD_SOURCE_FILE, FloodSource, INUNDATION_TILE,
        InundationLevel, InundationMask, InundationMetadata, InundationProvenance,
        InundationRasters, InundationStage, InundationWriter, StoreError, active_run,
        cache_catchments, digest_file, read_flood_source, read_retained_manifest, run_cache_root,
        verify_inundation_run,
    },
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use terrio::Window;

#[derive(Clone, Debug)]
pub struct InundationProcessRequest {
    pub run: PathBuf,
    pub scenario: String,
    pub hand: PathBuf,
    pub catchments: PathBuf,
    pub replace: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InundationProcessStatus {
    Created,
    Reused,
    Replaced,
}

#[derive(Clone, Debug, Serialize)]
pub struct InundationProcessReport {
    pub path: PathBuf,
    pub scenario: String,
    pub status: InundationProcessStatus,
    pub width: usize,
    pub height: usize,
    pub historical_count: usize,
    pub forecast_count: usize,
}

pub fn process_workspace_inundation(
    workspace: &Path,
    run: Option<PathBuf>,
    scenario: String,
    replace: bool,
    control: &OperationControl,
) -> Result<InundationProcessReport, StoreError> {
    let workspace = workspace.canonicalize()?;
    let config = super::read_workspace_config(&super::workspace_config_path(&workspace))
        .map_err(StoreError::Format)?;
    let spatial = config.spatial.unwrap_or_default();
    let hand = workspace.join(
        spatial
            .hand_raster
            .ok_or_else(|| StoreError::Format("Set spatial.hand_raster".into()))?,
    );
    let catchments = match spatial.catchment_raster {
        Some(path) => workspace.join(path),
        None => {
            let geometry = workspace.join(
                spatial
                    .gpkg_path
                    .ok_or_else(|| StoreError::Format("Set spatial.mini_layers".into()))?,
            );
            cache_catchments(&geometry, &hand, &workspace.join("cache/spatial"))?
        }
    };
    let run = match run {
        Some(path) => path,
        None => active_run(&workspace)
            .map_err(StoreError::Format)?
            .ok_or_else(|| StoreError::Format("Workspace has no active run".into()))?,
    };
    process_inundation(
        InundationProcessRequest {
            run,
            scenario,
            hand,
            catchments,
            replace,
        },
        control,
    )
}

/// Calculate and retain an inundation mask for a completed scenario using the supplied rasters.
pub fn process_inundation(
    request: InundationProcessRequest,
    control: &OperationControl,
) -> Result<InundationProcessReport, StoreError> {
    cancelled(control)?;
    let run = request.run.canonicalize()?;
    let workspace = run.ancestors().nth(3).filter(|workspace| {
        run.parent()
            .is_some_and(|parent| parent == run_cache_root(workspace))
    });
    let _operation_lock = workspace
        .map(crate::app::operation::lock_workspace)
        .transpose()
        .map_err(|error| StoreError::Integrity(error.to_string()))?;
    if let Some(workspace) = workspace
        && active_run(workspace)
            .map_err(StoreError::Integrity)?
            .as_deref()
            != Some(&run)
    {
        return Err(StoreError::Integrity(
            "inundation target is no longer the active run".into(),
        ));
    }
    validate_scenario_id(&request.scenario)?;
    let manifest_path = run.join("manifest.json");
    let run_manifest_sha256 = digest(&manifest_path)?;
    let manifest = read_retained_manifest(&run).map_err(StoreError::Schema)?;
    let scenario_matches = manifest
        .scenarios
        .iter()
        .filter(|s| s.id == request.scenario);
    let mut scenario_matches = scenario_matches.peekable();
    if scenario_matches
        .next()
        .is_none_or(|s| s.state != "completed")
        || scenario_matches.peek().is_some()
    {
        return Err(StoreError::InvalidValue(format!(
            "retained run has no unique completed scenario {}",
            request.scenario
        )));
    }
    let results = run.join("results");
    let scenario_dir = results.join(&request.scenario);
    if scenario_dir.canonicalize()? != scenario_dir {
        return Err(StoreError::InvalidPath(
            "scenario results path must be retained in the selected run".into(),
        ));
    }
    let mut stage = InundationStage::lock(&results, &request.scenario)?;
    let path = scenario_dir.join(FLOOD_MASK_FILE);
    let source_path = scenario_dir.join(FLOOD_SOURCE_FILE);
    if !source_path.is_file() {
        let reason = fs::read_to_string(scenario_dir.join(FLOOD_SOURCE_DIAGNOSTIC))
            .unwrap_or_else(|_| "this run did not retain the required HANDTUDO source".to_owned());
        return Err(StoreError::InvalidValue(format!(
            "inundation unavailable for scenario {}. {}",
            request.scenario,
            reason.trim()
        )));
    }
    if source_path.symlink_metadata()?.file_type().is_symlink() {
        return Err(StoreError::InvalidPath(
            "the retained flood source must be a regular file".into(),
        ));
    }
    let flood_source_sha256 = digest(&source_path)?;
    let source = read_flood_source(&source_path)?;
    let flood_source = &source.provenance.processing["flood_source"];
    if flood_source["scenario"].as_str() != Some(&request.scenario)
        || flood_source["depth_datum"].as_str() != Some("model_channel_bed")
    {
        return Err(StoreError::Integrity(
            "flood source provenance does not match the selected scenario".into(),
        ));
    }
    cancelled(control)?;
    let mut rasters = InundationRasters::open(&request.hand, &request.catchments)?;
    let (hand_hash, catchment_hash) = rasters.input_hashes();
    let (hand_sidecars, catchments_sidecars) = rasters.sidecar_hashes();
    let metadata = InundationMetadata {
        width: rasters.width,
        height: rasters.height,
        epsg: rasters.epsg,
        origin_x: rasters.transform[0],
        origin_y: rasters.transform[3],
        pixel_width: rasters.transform[1],
        pixel_height: rasters.transform[5],
        timeline: source.timeline.clone(),
        historical_count: source.historical_count,
        provenance: InundationProvenance {
            run_manifest_sha256,
            scenario: request.scenario.clone(),
            hand_sha256: hand_hash.into(),
            catchments_sha256: catchment_hash.into(),
            flood_source_sha256,
            hand_sidecars: hand_sidecars.to_vec(),
            catchments_sidecars: catchments_sidecars.to_vec(),
            vertical_datum: "Z0".into(),
            hand_vertical_shift_m: 0,
            include_nonpositive_hand: true,
            hand_in_decimetres: true,
        },
    };
    let verify_inputs = |rasters: &InundationRasters| -> Result<(), StoreError> {
        rasters.verify()?;
        if digest(&source_path)? != metadata.provenance.flood_source_sha256
            || digest(&manifest_path)? != metadata.provenance.run_manifest_sha256
        {
            return Err(StoreError::Integrity(
                "retained run inputs changed during inundation processing".into(),
            ));
        }
        cancelled(control)
    };
    let mut report = InundationProcessReport {
        path,
        scenario: request.scenario.clone(),
        status: InundationProcessStatus::Created,
        width: metadata.width,
        height: metadata.height,
        historical_count: metadata.historical_count,
        forecast_count: metadata.timeline.bounds.len() - metadata.historical_count,
    };
    let replace = match report.path.symlink_metadata() {
        Ok(info) if info.is_file() => {
            let same = InundationMask::open(&report.path).and_then(|mask| {
                let old = mask.metadata();
                let mut candidate = metadata.clone();
                candidate
                    .provenance
                    .run_manifest_sha256
                    .clone_from(&old.provenance.run_manifest_sha256);
                if old != &candidate
                    || verify_inundation_run(&run, &request.scenario, &old.provenance).is_err()
                {
                    return Ok(None);
                }
                mask.validate_payload(&report.path, || cancelled(control))?;
                Ok(Some(old.provenance.clone()))
            });
            if let Ok(Some(_)) = same {
                verify_inputs(&rasters)?;
                report.status = InundationProcessStatus::Reused;
                return Ok(report);
            }
            cancelled(control)?;
            if !request.replace {
                return Err(StoreError::Integrity(format!(
                    "existing flood mask differs from the selected inputs or is invalid. Use --replace to rebuild {}",
                    report.path.display()
                )));
            }
            true
        }
        Ok(_) => {
            return Err(StoreError::InvalidPath(format!(
                "flood mask destination must be a regular file: {}",
                report.path.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    let staged = stage.temporary_file(&results)?;
    let mut writer = InundationWriter::create(&staged, &metadata)?;
    write_rasters(&mut rasters, &source, &mut writer, control)?;
    writer.close()?;
    let mask = InundationMask::open(&staged)?;
    if mask.metadata() != &metadata {
        return Err(StoreError::Integrity(
            "reopened flood mask metadata differs from the processed inputs".into(),
        ));
    }
    mask.validate_payload(&staged, || cancelled(control))?;
    drop(mask);
    verify_inputs(&rasters)?;
    stage.publish(&staged, &report.path, replace, || cancelled(control))?;
    if replace {
        report.status = InundationProcessStatus::Replaced;
    }
    Ok(report)
}

fn write_rasters(
    rasters: &mut InundationRasters,
    source: &FloodSource,
    writer: &mut InundationWriter,
    control: &OperationControl,
) -> Result<(), StoreError> {
    let stages = InundationStages::new(&source.depth_m, &source.channel_depth_m);
    let minis: HashMap<_, _> = source
        .mini_ids
        .iter()
        .copied()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let mut mask = Vec::new();
    let mut changing = Vec::new();
    let mut overviews = [Vec::new(), Vec::new()];
    let chunk = INUNDATION_TILE;
    for y in (0..rasters.height).step_by(chunk) {
        let h = chunk.min(rasters.height - y);
        for x in (0..rasters.width).step_by(chunk) {
            cancelled(control)?;
            let w = chunk.min(rasters.width - x);
            let tile = Window::new(x, y, w, h);
            let cells = rasters.read_cells(tile, &minis)?;
            if !stages.initialize_tile(&cells, &mut mask, &mut changing) {
                continue;
            }
            for (time, frame) in stages.frames().enumerate() {
                cancelled(control)?;
                let refresh = time == 0 || update_tile(&changing, frame, &mut mask);
                writer.write(InundationLevel::Full, time, tile, &mask)?;
                for (level, overview) in [InundationLevel::Overview4, InundationLevel::Overview16]
                    .into_iter()
                    .zip(&mut overviews)
                {
                    let factor = level.factor();
                    let window = Window::new(
                        x / factor,
                        y / factor,
                        w.div_ceil(factor),
                        h.div_ceil(factor),
                    );
                    if refresh {
                        wet_fraction(&mask, w, h, factor, overview);
                    }
                    writer.write(level, time, window, overview)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_scenario_id(id: &str) -> Result<(), StoreError> {
    if id.len() <= 64
        && id.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Ok(())
    } else {
        Err(StoreError::InvalidValue("invalid scenario ID".into()))
    }
}

fn digest(path: &Path) -> Result<String, StoreError> {
    digest_file(path).map(|(hash, _)| hash).map_err(|e| {
        StoreError::Integrity(format!("reading {} for provenance: {e}", path.display()))
    })
}

fn cancelled(control: &OperationControl) -> Result<(), StoreError> {
    if control.is_cancelled() {
        return Err(StoreError::InvalidValue(
            "inundation processing cancelled".into(),
        ));
    }
    Ok(())
}
