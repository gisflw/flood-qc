use super::operation::{ExecutionRecipe, lock_workspace};
use crate::store::{
    RunAttemptManifest, active_run, adopt_legacy_rainfall, digest_file, new_run_stage_for,
    publish_run, read_basin_forcing, read_result_series, read_retained_manifest, read_run_attempt,
    recover_run_cache, sync_directory, write_run_attempt,
};
use serde::Serialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct CacheMigrationReport {
    pub active_run: PathBuf,
    pub legacy_runs: Vec<PathBuf>,
    pub legacy_payload_bytes: u64,
    pub estimated_reclaimed_bytes: u64,
    pub applied: bool,
}

pub fn migrate_cache(
    workspace: &Path,
    keep_run: &str,
    inundation_donor: Option<&str>,
    apply: bool,
) -> Result<CacheMigrationReport, String> {
    valid_legacy_id(keep_run)?;
    if let Some(id) = inundation_donor {
        valid_legacy_id(id)?;
        if id == keep_run {
            return Err("inundation donor must differ from the run being kept".into());
        }
    }
    let workspace = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let base = workspace.join("data/runs");
    let keep = base.join(keep_run);
    let destination = workspace
        .join("cache/run")
        .join(format!("generation-{keep_run}"));
    let current = active_run(&workspace)?;
    if current.as_ref().is_some_and(|path| *path != destination) {
        return Err("workspace already has a different active run".into());
    }
    let inventory = inventory(&base, inundation_donor)?;
    let mut report = CacheMigrationReport {
        active_run: destination.clone(),
        legacy_runs: inventory.iter().map(|(path, _)| path.clone()).collect(),
        legacy_payload_bytes: 0,
        estimated_reclaimed_bytes: 0,
        applied: false,
    };
    for (path, _) in &inventory {
        let (bytes, reclaimable) = payload_size(path, path == &keep, inundation_donor)?;
        report.legacy_payload_bytes += bytes;
        report.estimated_reclaimed_bytes += reclaimable;
    }
    if current.is_none() && !report.legacy_runs.contains(&keep) {
        return Err(format!("legacy run does not exist: {}", keep.display()));
    }
    if current.is_none()
        && let Some(id) = inundation_donor
        && !report.legacy_runs.contains(&base.join(id))
    {
        return Err(format!("inundation donor does not exist: {id}"));
    }
    if !apply {
        return Ok(report);
    }
    let _lock = lock_workspace(&workspace).map_err(|error| error.to_string())?;
    if active_run(&workspace)? != current || self::inventory(&base, inundation_donor)? != inventory
    {
        return Err("cache changed during migration inspection; retry".into());
    }
    recover_run_cache(&workspace)?;
    if current.is_none() {
        let manifest = read_retained_manifest(&keep)?;
        if manifest.state != "completed"
            || manifest
                .scenarios
                .iter()
                .any(|scenario| scenario.state != "completed")
        {
            return Err("select a legacy run whose scenarios all completed".into());
        }
        let trial = super::read_trial(&keep).map_err(|error| error.to_string())?;
        let stage = new_run_stage_for(&workspace, keep_run)?;
        let stage_result = (|| {
            link_tree(&keep, &stage)?;
            let recipes = match inundation_donor {
                Some(id) => adopt_inundation(&workspace, &keep, &base.join(id), &stage, &trial)?,
                None => Default::default(),
            };
            if adopt_legacy_rainfall(&workspace, &stage)? && !rainfall_matches(&stage, &manifest)? {
                fs::remove_dir_all(stage.join("rainfall")).map_err(|error| error.to_string())?;
            }
            for (path, _) in &inventory {
                let id = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or("invalid run ID")?;
                let mut attempt = legacy_attempt(path, id)?;
                if path == &keep {
                    attempt.inundation = recipes.clone();
                }
                write_run_attempt(&workspace, &attempt)?;
            }
            publish_run(&workspace, &stage)
        })();
        if let Err(error) = stage_result {
            if active_run(&workspace).is_ok_and(|path| path.as_deref() != Some(&stage)) {
                fs::remove_dir_all(&stage).map_err(|cleanup| {
                    format!("{error}; could not remove {}: {cleanup}", stage.display())
                })?;
            }
            return Err(error);
        }
    } else {
        // A pointer update may have succeeded before deletion finished.
        // Revalidate the exact legacy manifests before resuming deletion.
        if keep.is_dir()
            && digest_file(&keep.join("manifest.json"))?
                != digest_file(&destination.join("manifest.json"))?
        {
            return Err("legacy run is different from the migrated active run".into());
        }
        for (path, _) in &inventory {
            let id = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("invalid run ID")?;
            let archived =
                read_run_attempt(&workspace.join("runs/history").join(format!("{id}.json")))?;
            if serde_json::to_value(&archived.operation).map_err(|error| error.to_string())?
                != serde_json::to_value(&legacy_attempt(path, id)?.operation)
                    .map_err(|error| error.to_string())?
            {
                return Err(format!("legacy run manifest changed: {}", path.display()));
            }
        }
    }
    crate::store::retire_legacy_rainfall(&workspace)?;
    let retired = destination.parent().ok_or("invalid active cache path")?;
    for (path, _) in inventory
        .iter()
        .filter(|(path, _)| path != &keep)
        .chain(inventory.iter().filter(|(path, _)| path == &keep))
    {
        let id = path
            .file_name()
            .and_then(|id| id.to_str())
            .ok_or("invalid run ID")?;
        let tombstone = retired.join(format!("generation-legacy-retired-{id}"));
        fs::rename(path, &tombstone).map_err(|error| format!("{}: {error}", path.display()))?;
        sync_directory(&base)?;
        sync_directory(retired)?;
        fs::remove_dir_all(&tombstone)
            .map_err(|error| format!("{}: {error}", tombstone.display()))?;
    }
    if base.is_dir() {
        sync_directory(&base)?;
    }
    report.applied = true;
    Ok(report)
}

fn valid_legacy_id(id: &str) -> Result<(), String> {
    let (time, pid) = id
        .rsplit_once('-')
        .ok_or("expected a generated legacy run ID")?;
    if chrono::NaiveDateTime::parse_from_str(time, "%Y%m%dT%H%M%S%.f").is_err()
        || pid.parse::<u32>().is_err()
    {
        return Err("expected a generated legacy run ID".into());
    }
    Ok(())
}

type LegacyInventory = Vec<(PathBuf, (String, u64))>;

fn inventory(base: &Path, donor: Option<&str>) -> Result<LegacyInventory, String> {
    let entries = match fs::symlink_metadata(base) {
        Ok(metadata) if metadata.is_dir() => {
            fs::read_dir(base).map_err(|error| error.to_string())?
        }
        Ok(_) => return Err(format!("expected a real directory: {}", base.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let owned = entry.file_type().is_ok_and(|kind| kind.is_dir())
            && entry
                .file_name()
                .to_str()
                .is_some_and(|id| valid_legacy_id(id).is_ok())
            && (donor.is_some_and(|id| entry.file_name() == id)
                || fs::symlink_metadata(path.join("bundle.json"))
                    .is_err_and(|error| error.kind() == io::ErrorKind::NotFound))
            && fs::symlink_metadata(path.join("manifest.json"))
                .is_ok_and(|metadata| metadata.is_file())
            && read_retained_manifest(&path).is_ok_and(|manifest| manifest.state != "prepared");
        if !owned {
            return Err(format!(
                "unclassified legacy entry. Move it outside {} before migration: {}",
                base.display(),
                path.display()
            ));
        }
        paths.push((path.clone(), digest_file(&path.join("manifest.json"))?));
    }
    paths.sort();
    Ok(paths)
}

fn payload_size(path: &Path, keep: bool, donor: Option<&str>) -> Result<(u64, u64), String> {
    fn walk(path: &Path, skip: bool) -> Result<(u64, u64), String> {
        let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            let mut bytes = 0;
            let mut reclaimable = 0;
            for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
                let (total, released) =
                    walk(&entry.map_err(|error| error.to_string())?.path(), skip)?;
                bytes += total;
                reclaimable += released;
            }
            Ok((bytes, reclaimable))
        } else if metadata.is_file() {
            #[cfg(unix)]
            let reclaimable = {
                use std::os::unix::fs::MetadataExt as _;
                if !skip && metadata.nlink() == 1 {
                    metadata.blocks() * 512
                } else {
                    0
                }
            };
            #[cfg(not(unix))]
            let reclaimable = if skip { 0 } else { metadata.len() };
            Ok((metadata.len(), reclaimable))
        } else {
            Err(format!(
                "unexpected file type in legacy run: {}",
                path.display()
            ))
        }
    }
    if !keep && donor.is_some_and(|id| path.ends_with(id)) {
        let (total, reclaimable) = walk(path, false)?;
        let mut reused = 0;
        for entry in fs::read_dir(path.join("results")).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                for name in ["flood_source.nc", "flood_mask.nc"] {
                    if entry.path().join(name).is_file() {
                        reused += walk(&entry.path().join(name), false)?.1;
                    }
                }
            }
        }
        return Ok((total, reclaimable.saturating_sub(reused)));
    }
    walk(path, keep)
}

fn legacy_attempt(path: &Path, id: &str) -> Result<RunAttemptManifest, String> {
    let manifest = read_retained_manifest(path)?;
    let mut attempt = RunAttemptManifest::new(id.to_owned());
    attempt.state = manifest.state.clone();
    attempt.created_at = manifest.created_at.clone();
    attempt.finished_at = manifest.finished_at.clone();
    attempt.error = manifest.error.clone();
    attempt.operation = Some(manifest);
    attempt.diagnostics = crate::store::collect_run_diagnostics(path);
    Ok(attempt)
}

fn link_tree(source: &Path, destination: &Path) -> Result<(), String> {
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            fs::create_dir(&target).map_err(|error| error.to_string())?;
            link_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::hard_link(entry.path(), &target).map_err(|error| error.to_string())?;
        } else {
            return Err(format!(
                "unexpected file type in legacy run: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn rainfall_matches(
    stage: &Path,
    manifest: &crate::store::OperationManifest,
) -> Result<bool, String> {
    let recipe: ExecutionRecipe =
        serde_json::from_value(manifest.recipe.clone()).map_err(|error| error.to_string())?;
    for scenario in &manifest.scenarios {
        let forcing = read_basin_forcing(
            &stage
                .join("rainfall/forcing")
                .join(format!("{}.nc", scenario.id)),
        )
        .map_err(|error| error.to_string())?;
        let result = read_result_series(&stage.join("results").join(&scenario.id).join("basin.nc"))
            .map_err(|error| error.to_string())?;
        if forcing.timeline.bounds != result.timeline.bounds
            || forcing.timeline.time_zone != result.timeline.time_zone
            || forcing.basins != result.basins
            || forcing.values != result.precipitation
            || forcing.valid != result.precipitation_valid
            || result
                .time_segment
                .iter()
                .filter(|segment| **segment == 0)
                .count()
                != recipe.historical_count
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(not(feature = "inundation-write"))]
fn adopt_inundation(
    _: &Path,
    _: &Path,
    _: &Path,
    _: &Path,
    _: &super::TrialCache,
) -> Result<std::collections::BTreeMap<String, crate::store::InundationRecipe>, String> {
    Err("inundation adoption requires a build with --features inundation-write".into())
}

#[cfg(feature = "inundation-write")]
fn adopt_inundation(
    workspace: &Path,
    keep: &Path,
    donor: &Path,
    stage: &Path,
    trial: &super::TrialCache,
) -> Result<std::collections::BTreeMap<String, crate::store::InundationRecipe>, String> {
    use crate::store::{
        FLOOD_MASK_FILE, FLOOD_SOURCE_FILE, INUNDATION_ADOPTION, InundationAdoption,
        InundationLevel, InundationMask, InundationRasters, InundationRecipe, read_flood_source,
        write_draft,
    };
    let donor_manifest = read_retained_manifest(donor)?;
    let keep_manifest = read_retained_manifest(keep)?;
    let mut donor_recipe = donor_manifest.recipe.clone();
    let mut keep_recipe = keep_manifest.recipe.clone();
    for recipe in [&mut donor_recipe, &mut keep_recipe] {
        // The donor may have used staging paths for exactly the same data.
        for (section, field) in [
            ("observed", "path"),
            ("geometry", "mini_gtp"),
            ("engine", "executable"),
            ("engine", "input_dir"),
            ("forecasts", "catalog"),
        ] {
            if let Some(object) = recipe["document"][section].as_object_mut() {
                object.remove(field);
            }
        }
    }
    if donor_recipe != keep_recipe || donor_manifest.state != "completed" {
        return Err("inundation donor has a different model recipe or an incomplete run".into());
    }
    let hand = workspace.join("source/HAND_Flood.tif");
    let catchments = workspace.join("source/catchments.tif");
    let rasters = InundationRasters::open(&hand, &catchments).map_err(|error| error.to_string())?;
    let (hand_hash, catchments_hash) = rasters.input_hashes();
    let (hand_sidecars, catchments_sidecars) = rasters.sidecar_hashes();
    let active_manifest_sha256 = digest_file(&keep.join("manifest.json"))?.0;
    let donor_manifest_sha256 = digest_file(&donor.join("manifest.json"))?.0;
    let mut recipes = std::collections::BTreeMap::new();
    for scenario in &trial.scenarios {
        let id = &scenario.id;
        let results = Path::new("results").join(id);
        let streams = Path::new("scenarios").join(id).join("Output");
        for relative in [results.join("basin.nc"), results.join("station.nc")]
            .into_iter()
            .chain(
                [
                    "QTUDO_Inercial_Atual.MGB",
                    "QTUDO_Inercial_Prev.MGB",
                    "YTUDO.MGB",
                    "YTUDO_prev.MGB",
                ]
                .into_iter()
                .map(|name| streams.join(name)),
            )
        {
            if digest_file(&keep.join(&relative))? != digest_file(&donor.join(&relative))? {
                return Err(format!(
                    "inundation donor output differs: {}",
                    relative.display()
                ));
            }
        }
        let source = donor.join(&results).join(FLOOD_SOURCE_FILE);
        let source_metadata = read_flood_source(&source).map_err(|error| error.to_string())?;
        let mask = donor.join(&results).join(FLOOD_MASK_FILE);
        let mask_reader = InundationMask::open(&mask).map_err(|error| error.to_string())?;
        let metadata = mask_reader.metadata();
        let provenance = &metadata.provenance;
        let values = read_result_series(&keep.join(&results).join("basin.nc"))
            .map_err(|error| error.to_string())?;
        if provenance.run_manifest_sha256 != donor_manifest_sha256
            || &provenance.scenario != id
            || provenance.hand_sha256 != hand_hash
            || provenance.catchments_sha256 != catchments_hash
            || provenance.hand_sidecars != hand_sidecars
            || provenance.catchments_sidecars != catchments_sidecars
            || provenance.flood_source_sha256 != digest_file(&source)?.0
            || source_metadata.provenance.processing["flood_source"]["scenario"].as_str()
                != Some(id)
            || source_metadata.provenance.processing["flood_source"]["depth_datum"].as_str()
                != Some("model_channel_bed")
            || metadata.width != rasters.width
            || metadata.height != rasters.height
            || metadata.epsg != rasters.epsg
            || [
                metadata.origin_x,
                metadata.pixel_width,
                0.,
                metadata.origin_y,
                0.,
                metadata.pixel_height,
            ] != rasters.transform
            || metadata.timeline != source_metadata.timeline
            || metadata.timeline.bounds != values.timeline.bounds
            || metadata.timeline.time_zone != values.timeline.time_zone
            || metadata.historical_count != source_metadata.historical_count
            || metadata.historical_count
                != values
                    .time_segment
                    .iter()
                    .filter(|segment| **segment == 0)
                    .count()
            || source_metadata.mini_ids != values.basins.mini_ids
        {
            return Err(format!("inundation donor provenance differs for {id}"));
        }
        let level = InundationLevel::Overview16;
        let (width, height) = metadata.dimensions(level);
        for step in [
            0,
            metadata.historical_count,
            metadata.timeline.bounds.len() - 1,
        ] {
            mask_reader
                .read_window(level, step, 0, 0, width, height)
                .map_err(|error| {
                    format!("inundation donor overview for {id} at step {step}: {error}")
                })?;
        }
        let destination = stage.join(results);
        for (path, name) in [(source, FLOOD_SOURCE_FILE), (mask, FLOOD_MASK_FILE)] {
            fs::hard_link(path, destination.join(name)).map_err(|error| error.to_string())?;
        }
        write_draft(
            &destination.join(INUNDATION_ADOPTION),
            &InundationAdoption {
                active_manifest_sha256: active_manifest_sha256.clone(),
                donor: provenance.clone(),
            },
        )?;
        recipes.insert(
            id.clone(),
            InundationRecipe {
                hand: hand.clone(),
                catchments: catchments.clone(),
                provenance: provenance.clone(),
            },
        );
    }
    Ok(recipes)
}
