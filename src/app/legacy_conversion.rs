//! Explicit conversion of previously exported legacy records. The source is never modified.
use crate::store::{
    ForecastCatalog, ForecastSnapshotSelection, InputReference, InputSetManifest, ObservationStore,
    copy_staged_tree, read_draft, write_draft,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
};

fn member(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("invalid legacy member path".into());
    }
    let mut target = root.to_path_buf();
    for c in path.components() {
        target.push(c.as_os_str());
        if fs::symlink_metadata(&target)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("legacy member contains a symlink".into());
        }
    }
    if !target.is_file() {
        return Err("legacy member is not a regular file".into());
    }
    Ok(target)
}
fn checked_member(root: &Path, entry: &Value) -> Result<PathBuf, String> {
    let path = member(root, entry["path"].as_str().ok_or("missing member path")?)?;
    let expected = (
        entry["sha256"]
            .as_str()
            .ok_or("missing member digest")?
            .to_owned(),
        entry["size"].as_u64().ok_or("missing member size")?,
    );
    if crate::store::digest_file(&path)? != expected {
        return Err("legacy input member integrity mismatch".into());
    }
    Ok(path)
}
fn read_legacy(root: &Path) -> Result<Value, String> {
    let index: Value = read_draft(&root.join("input-set.json"))?;
    if index["version"] != 1 || !index["files"].is_array() {
        return Err("unsupported legacy input set".into());
    }
    let recipe: Value = read_draft(&member(root, "sources/recipe.json")?)?;
    if index["recipe"] != recipe || recipe["engine_identity"] != index["engine"] {
        return Err("legacy input recipe or engine binding differs".into());
    }
    for entry in index["files"].as_array().unwrap() {
        checked_member(root, entry)?;
    }
    Ok(index)
}
fn map_forecast(
    catalog: &mut ForecastCatalog,
    root: &Path,
    old_id: &str,
) -> Result<String, String> {
    if old_id.is_empty()
        || !old_id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid legacy forecast identity".into());
    }
    let base = format!("sources/forecasts/{old_id}");
    let record: Value = read_draft(&member(root, &format!("{base}/metadata.json"))?)?;
    if record["asset_id"].as_str() != Some(old_id) {
        return Err("legacy forecast identity differs".into());
    }
    let native = member(root, &format!("{base}/native.nc"))?;
    if crate::store::digest_file(&native)?
        != (
            record["native_sha256"]
                .as_str()
                .ok_or("missing forecast digest")?
                .into(),
            record["native_byte_length"]
                .as_u64()
                .ok_or("missing forecast size")?,
        )
    {
        return Err("legacy forecast native grid changed".into());
    }
    let metadata = crate::store::CanonicalForecastMetadata {
        provider: record["provider"]
            .as_str()
            .ok_or("missing provider")?
            .into(),
        model: record["model"].as_str().ok_or("missing model")?.into(),
        product: record["product"].as_str().ok_or("missing product")?.into(),
        cycle_utc: record["cycle_utc"]
            .as_str()
            .ok_or("missing cycle")?
            .parse::<chrono::DateTime<chrono::Utc>>()
            .map_err(|e| e.to_string())?,
    };
    catalog
        .register_canonical(&native, &metadata)
        .map(|a| a.asset_id)
        .map_err(|e| e.to_string())
}
fn convert_saved_inputs_locked(
    workspace: &Path,
    interchange: &Path,
) -> Result<InputSetManifest, String> {
    let manifest: Value = read_draft(&interchange.join("manifest.json"))?;
    let legacy = manifest["legacy_input_set"]
        .as_str()
        .ok_or("interchange does not contain an independently saved input set")?;
    let index_path = member(interchange, legacy)?;
    let root = index_path.parent().ok_or("invalid saved input root")?;
    let index = read_legacy(root)?;
    let work = workspace.join("work");
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let temp = tempfile::Builder::new()
        .prefix(".legacy-inputs-")
        .tempdir_in(work)
        .map_err(|e| e.to_string())?;
    let stage = temp.path();
    fs::create_dir_all(stage.join("sources")).map_err(|e| e.to_string())?;
    for entry in index["files"].as_array().unwrap() {
        let rel = entry["path"].as_str().ok_or("missing path")?;
        if rel == "sources/recipe.json"
            || rel.starts_with("sources/observations/")
            || rel.starts_with("sources/forecasts/")
            || rel == "sources/wind-observations.parquet"
            || rel == "sources/reservoir-schedules.parquet"
        {
            continue;
        }
        let src = checked_member(root, entry)?;
        let target = stage.join(rel);
        fs::create_dir_all(target.parent().ok_or("invalid target")?).map_err(|e| e.to_string())?;
        copy_staged_tree(&src, &target)?;
    }
    for (kind, name) in [
        ("wind_observations", "wind-observations.toml"),
        ("reservoir_schedules", "reservoir-schedules.toml"),
    ] {
        if let Some(file) = manifest["forcing"][kind]["file"].as_str() {
            let source = member(interchange, file)?;
            if crate::store::digest_file(&source)?.0
                != manifest["forcing"][kind]["sha256"]
                    .as_str()
                    .ok_or("missing forcing digest")?
            {
                return Err("converted forcing changed".into());
            }
            copy_staged_tree(&source, &stage.join("sources").join(name))?;
        }
    }
    let db = stage.join("sources/observations.sqlite");
    let observation =
        ObservationStore::import_interchange(&db, interchange).map_err(|e| e.to_string())?;
    let revision = observation.revision().map_err(|e| e.to_string())?;
    let observation_registry_id = observation.registry_id().map_err(|e| e.to_string())?;
    drop(observation);
    let mut recipe = index["recipe"].clone();
    let document = recipe["document"]
        .as_object_mut()
        .ok_or("missing legacy preparation document")?;
    document.remove("schema_version");
    let observed = document
        .get_mut("observed")
        .and_then(Value::as_object_mut)
        .ok_or("missing observation selection")?;
    observed.insert("revision".into(), revision.into());
    observed.insert("path".into(), "sources/observations.sqlite".into());
    observed.insert(
        "historical_rainfall".into(),
        serde_json::json!({"kind":"stations"}),
    );
    let scenarios = document
        .get_mut("scenarios")
        .and_then(Value::as_array_mut)
        .ok_or("missing scenarios")?;
    let catalog_root = super::forecast_assets(workspace);
    let mut catalog = ForecastCatalog::open(workspace, &catalog_root, &catalog_root)
        .map_err(|e| e.to_string())?;
    let mut mapping: HashMap<String, String> = HashMap::new();
    for scenario in scenarios {
        if let Some(id) = scenario.get("asset_id").and_then(Value::as_str) {
            let new = if let Some(found) = mapping.get(id) {
                found.clone()
            } else {
                let value = map_forecast(&mut catalog, root, id)?;
                mapping.insert(id.to_owned(), value.clone());
                value
            };
            scenario["asset_id"] = new.into();
        }
    }
    let selections = mapping
        .values()
        .map(|id| ForecastSnapshotSelection {
            asset_id: id.clone(),
        })
        .collect::<Vec<_>>();
    if !selections.is_empty() {
        catalog
            .snapshot_selected_values(&stage.join("sources/forecasts"), &selections)
            .map_err(|e| e.to_string())?;
    }
    recipe["registered_replacements"] = recipe["document"]["replacement_values"].clone();
    recipe["observation_registry_id"] = observation_registry_id.into();
    write_draft(&stage.join("sources/recipe.toml"), &recipe)?;
    let files = crate::store::collect_inputs(stage)?;
    let engine: InputReference =
        serde_json::from_value(index["engine"].clone()).map_err(|e| e.to_string())?;
    let original_id = index["id"].as_str().ok_or("missing legacy input ID")?;
    let id = format!(
        "legacy-{}-{}",
        original_id.chars().take(48).collect::<String>(),
        chrono::Utc::now().format("%Y%m%d%H%M%S%9f")
    );
    let converted = InputSetManifest {
        id,
        created_at: index["created_at"]
            .as_str()
            .ok_or("missing creation time")?
            .into(),
        engine,
        recipe,
        files,
        objects: Vec::new(),
        observation_snapshot: None,
    };
    let path = crate::store::register_imported_selection(workspace, stage, converted)?;
    crate::store::read_input_set(&path)
}
pub fn convert_saved_inputs(
    workspace: &Path,
    interchange: &Path,
) -> Result<InputSetManifest, String> {
    let workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let _lock = super::operation::lock_workspace_job(&workspace).map_err(|e| e.to_string())?;
    convert_saved_inputs_locked(&workspace, interchange)
}

fn checked_legacy_table(interchange: &Path, descriptor: &Value) -> Result<Vec<Value>, String> {
    let file = member(
        interchange,
        descriptor["file"].as_str().ok_or("missing table file")?,
    )?;
    if crate::store::digest_file(&file)?.0
        != descriptor["sha256"]
            .as_str()
            .ok_or("missing table digest")?
    {
        return Err("legacy table changed".into());
    }
    let text = fs::read_to_string(file).map_err(|e| e.to_string())?;
    let rows = text
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if Some(rows.len() as u64) != descriptor["count"].as_u64() {
        return Err("legacy table count changed".into());
    }
    Ok(rows)
}
fn converted_result_file(path: &Path, id: &str) -> Result<(), String> {
    let mut record: Value = read_draft(path)?;
    if let Some(object) = record.as_object_mut() {
        object.remove("schema_version");
    }
    if path.file_name().is_some_and(|name| name == "manifest.json")
        && path.parent().is_some_and(|p| p.ends_with("rainfall"))
    {
        record["info"]["generation"] = id.into();
    }
    let destination = path.with_extension("toml");
    write_draft(&destination, &record)?;
    fs::remove_file(path).map_err(|e| e.to_string())
}
fn convert_result_manifests(root: &Path, id: &str, old_id: &str) -> Result<(), String> {
    let original = root.join("manifest.json");
    let mut manifest: Value = read_draft(&original)?;
    let (sha256, size) = crate::store::digest_file(&original)?;
    fs::rename(&original, root.join("legacy-manifest.json")).map_err(|e| e.to_string())?;
    manifest
        .as_object_mut()
        .ok_or("invalid legacy result manifest")?
        .remove("schema_version");
    let recipe = manifest["recipe"]
        .as_object_mut()
        .ok_or("invalid legacy result recipe")?;
    recipe.insert("legacy_attempt_id".into(), old_id.into());
    recipe.insert("legacy_manifest_sha256".into(), sha256.clone().into());
    recipe.insert("legacy_manifest_path".into(), "legacy-manifest.json".into());
    manifest["inputs"].as_array_mut().ok_or("invalid legacy result input index")?.push(serde_json::json!({"path":"legacy-manifest.json","kind":"legacy","size":size,"sha256":sha256}));
    write_draft(&root.join("manifest.toml"), &manifest)?;
    let rainfall = root.join("rainfall/manifest.json");
    if rainfall.exists() {
        converted_result_file(&rainfall, id)?;
    }
    Ok(())
}

fn legacy_reference(attempt: &Value) -> Result<(chrono::NaiveDateTime, String), String> {
    let recipe = &attempt["operation"]["recipe"]["document"];
    let reference = recipe["reference_time"]
        .as_str()
        .ok_or("legacy attempt has no operational reference time")?
        .parse::<chrono::NaiveDateTime>()
        .map_err(|e| e.to_string())?;
    let zone = recipe["time_zone"]
        .as_str()
        .ok_or("legacy attempt has no timezone")?
        .to_owned();
    Ok((reference, zone))
}
fn convert_attempts(
    source: &Path,
    workspace: &Path,
    legacy: &Path,
    active: Option<&str>,
    database_attempts: &[Value],
) -> Result<(), String> {
    let history = source.join("runs/history");
    let mut attempts = BTreeMap::<String, Value>::new();
    if history.is_dir() {
        for entry in fs::read_dir(&history).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                return Err(format!(
                    "unsupported legacy history entry {}",
                    path.display()
                ));
            }
            let attempt: Value = read_draft(&path)?;
            let id = attempt["attempt_id"]
                .as_str()
                .ok_or("legacy attempt lacks ID")?
                .to_owned();
            if attempts.insert(id, attempt).is_some() {
                return Err("duplicate legacy attempt identity".into());
            }
        }
    }
    for row in database_attempts {
        let id = row["attempt_id"]
            .as_str()
            .ok_or("legacy database attempt lacks ID")?;
        let raw = row["manifest"]
            .as_str()
            .ok_or("legacy database attempt lacks manifest")?;
        let attempt: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        if attempt["attempt_id"].as_str() != Some(id) {
            return Err("legacy database attempt ID differs".into());
        }
        if let Some(existing) = attempts.get(id) {
            if existing != &attempt {
                return Err("legacy attempt history disagrees with database".into());
            }
        } else {
            attempts.insert(id.to_owned(), attempt);
        }
    }
    let mut mapped = HashMap::new();
    for (index, (old, mut attempt)) in attempts.into_iter().enumerate() {
        let (reference, zone) = match legacy_reference(&attempt) {
            Ok(value) => value,
            Err(reason) => {
                write_draft(
                    &legacy.join(format!("attempt-unresolved-{index:06}.toml")),
                    &serde_json::json!({"legacy_attempt_id": old, "reason": reason, "attempt": attempt}),
                )?;
                continue;
            }
        };
        let id = crate::store::allocate_run_id(workspace, reference, &zone)?;
        write_draft(&legacy.join(format!("attempt-{id}.toml")), &attempt)?;
        let object = attempt.as_object_mut().ok_or("invalid legacy attempt")?;
        object.remove("schema_version");
        object.remove("inundation");
        object.insert("attempt_id".into(), id.clone().into());
        if let Some(operation) = object.get_mut("operation").and_then(Value::as_object_mut) {
            operation.remove("schema_version");
            if let Some(recipe) = operation.get_mut("recipe").and_then(Value::as_object_mut) {
                recipe.insert("legacy_attempt_id".into(), old.clone().into());
            }
        }
        let typed: crate::store::RunAttemptManifest = serde_json::from_value(attempt)
            .map_err(|e| format!("unsupported legacy attempt {old}: {e}"))?;
        crate::store::write_run_attempt(workspace, &typed)?;
        mapped.insert(old, id);
    }
    let cache = source.join("cache/run");
    if cache.is_dir() {
        for entry in fs::read_dir(&cache).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if entry.file_type().map_err(|e| e.to_string())?.is_dir()
                && Some(name.as_ref()) != active
            {
                return Err(format!(
                    "legacy result generation {name} is not active; preserve or explicitly export it before conversion"
                ));
            }
            if entry.file_type().map_err(|e| e.to_string())?.is_file()
                && name != "access.lock"
                && name != "active.json"
            {
                return Err(format!("unsupported legacy result-cache file {name}"));
            }
        }
    }
    let Some(active) = active else {
        return Ok(());
    };
    let old_id = active
        .strip_prefix("generation-")
        .ok_or("unsupported legacy active generation")?;
    let id = mapped
        .get(old_id)
        .ok_or("active legacy generation has no attempt manifest")?;
    let original = source.join("cache/run").join(active);
    if !original.is_dir() {
        return Err("active legacy generation is missing".into());
    }
    let result = workspace.join("results").join(id);
    copy_staged_tree(&original, &result)?;
    convert_result_manifests(&result, id, old_id)?;
    let trial =
        super::read_trial(&result).map_err(|e| format!("legacy result validation failed: {e}"))?;
    for scenario in &trial.scenarios {
        let path = result
            .join("results")
            .join(&scenario.id)
            .join(crate::store::FLOOD_MASK_FILE);
        if path.is_file() {
            let mask = crate::store::InundationMask::open(&path)
                .map_err(|e| format!("legacy inundation mask validation failed: {e}"))?;
            crate::store::verify_inundation_run(
                &result,
                &scenario.id,
                &mask.metadata().provenance,
            )?;
        }
    }
    fs::write(result.join(".retained"), b"").map_err(|e| e.to_string())?;
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(workspace.join("results/access.lock"))
        .map_err(|e| e.to_string())?;
    write_draft(
        &workspace.join("results/active.toml"),
        &serde_json::json!({"id":id}),
    )?;
    Ok(())
}

fn verify_interchange_source(source: &Path, manifest: &Value) -> Result<(), String> {
    match manifest["source_kind"].as_str() {
        Some("legacy_sqlite") => {
            let db = source.join("operation.sqlite");
            if db.with_file_name("operation.sqlite-wal").exists() {
                return Err("legacy SQLite source has a WAL; checkpoint before export".into());
            }
            let files = manifest["source_sqlite_files"]
                .as_object()
                .ok_or("interchange has no SQLite source fingerprint; export it again")?;
            if files.len() != 1 {
                return Err("invalid SQLite source fingerprint".into());
            }
            let expected = files
                .get("operation.sqlite")
                .ok_or("missing SQLite source fingerprint")?;
            if crate::store::digest_file(&member(source, "operation.sqlite")?)?
                != (
                    expected["sha256"]
                        .as_str()
                        .ok_or("missing SQLite source digest")?
                        .into(),
                    expected["size"]
                        .as_u64()
                        .ok_or("missing SQLite source length")?,
                )
            {
                return Err("legacy SQLite source changed after interchange export".into());
            }
        }
        Some("parquet") => {
            let obs = source.join("assets/observations");
            if crate::store::digest_file(&member(&obs, "current.json")?)?.0
                != manifest["source_pointer_sha256"]
                    .as_str()
                    .ok_or("interchange has no source pointer fingerprint; export it again")?
            {
                return Err("legacy observation pointer changed after interchange export".into());
            }
            let files = manifest["source_members"]
                .as_object()
                .ok_or("interchange has no observation file fingerprints; export it again")?;
            for (relative, expected) in files {
                if crate::store::digest_file(&member(&obs, relative)?)?
                    != (
                        expected["sha256"]
                            .as_str()
                            .ok_or("missing observation source digest")?
                            .into(),
                        expected["size"]
                            .as_u64()
                            .ok_or("missing observation source length")?,
                    )
                {
                    return Err(format!(
                        "legacy observation source changed after export: {relative}"
                    ));
                }
            }
        }
        _ => return Err("unsupported legacy workspace interchange".into()),
    }
    Ok(())
}

/// Convert an exported workspace to a new, independent workspace; the source is unchanged.
pub fn convert_workspace(
    source: &Path,
    destination: &Path,
    interchange: &Path,
) -> Result<(), String> {
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    if !fs::symlink_metadata(source.join(".operation.lock"))
        .is_ok_and(|entry| entry.is_file() && !entry.file_type().is_symlink())
    {
        return Err("legacy workspace has no regular .operation.lock; conversion cannot lock it without modifying the source".into());
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(source.join(".operation.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|e| format!("legacy workspace is busy: {e}"))?;
    let interchange = interchange.canonicalize().map_err(|e| e.to_string())?;
    if destination.exists() {
        return Err("conversion destination must not exist".into());
    }
    let parent = destination
        .parent()
        .ok_or("destination needs a parent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if parent.starts_with(&source) {
        return Err("convert outside the source workspace".into());
    }
    let interchange_manifest: Value = read_draft(&interchange.join("manifest.json"))?;
    verify_interchange_source(&source, &interchange_manifest)?;
    crate::store::publish_directory(destination, |stage| {
        for name in ["config.toml", "source", "model", "downloads"] {
            let p = source.join(name);
            if p.exists() {
                copy_staged_tree(&p, &stage.join(name))?;
            }
        }
        let db = super::observation_assets(stage);
        fs::create_dir_all(db.parent().ok_or("invalid registry path")?)
            .map_err(|e| e.to_string())?;
        let registry =
            ObservationStore::import_interchange(&db, &interchange).map_err(|e| e.to_string())?;
        let revision = registry.revision().map_err(|e| e.to_string())?;
        let zone = registry.time_zone().map_err(|e| e.to_string())?;
        drop(registry);
        let legacy = stage.join("source/legacy-records");
        fs::create_dir_all(&legacy).map_err(|e| e.to_string())?;
        write_draft(
            &legacy.join("conversion.toml"),
            &serde_json::json!({"source":source,"observation_revision":revision,"interchange":interchange_manifest}),
        )?;
        for name in ["runs", "work", "results"] {
            fs::create_dir_all(stage.join(name)).map_err(|e| e.to_string())?;
        }
        for (folder, allowed) in [
            ("runs", &["history"][..]),
            ("cache", &["run", "inputs"][..]),
        ] {
            let root = source.join(folder);
            if root.is_dir() {
                for entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if !allowed.iter().any(|name| entry.file_name() == *name) {
                        return Err(format!(
                            "unsupported legacy workspace content {}",
                            entry.path().display()
                        ));
                    }
                }
            }
        }
        let mut active = None;
        let mut database_attempts = Vec::new();
        let mut migrated_native = HashSet::new();
        if let Some(tables) = interchange_manifest["legacy_tables"].as_object() {
            for (name, descriptor) in tables {
                let rows = checked_legacy_table(&interchange, descriptor)?;
                write_draft(&legacy.join(format!("{name}.toml")), &rows)?;
                if name == "run_attempt" {
                    database_attempts = rows.clone();
                }
                if name == "active_run" {
                    active = rows
                        .first()
                        .and_then(|v| v["generation"].as_str())
                        .map(str::to_owned);
                }
                if name == "asset" {
                    let catalog_root = super::forecast_assets(stage);
                    let mut catalog = ForecastCatalog::open(stage, &catalog_root, &catalog_root)
                        .map_err(|e| e.to_string())?;
                    for record in rows {
                        let native = member(
                            &source,
                            record["native_path"]
                                .as_str()
                                .ok_or("missing native path")?,
                        )?;
                        migrated_native.insert(native.clone());
                        if crate::store::digest_file(&native)?
                            != (
                                record["native_sha256"]
                                    .as_str()
                                    .ok_or("missing native hash")?
                                    .into(),
                                record["native_byte_length"]
                                    .as_u64()
                                    .ok_or("missing native length")?,
                            )
                        {
                            return Err("legacy forecast changed".into());
                        }
                        let original = member(
                            &source,
                            record["source_path"]
                                .as_str()
                                .ok_or("missing original path")?,
                        )?;
                        if crate::store::digest_file(&original)?
                            != (
                                record["source_sha256"]
                                    .as_str()
                                    .ok_or("missing original hash")?
                                    .into(),
                                record["source_byte_length"]
                                    .as_u64()
                                    .ok_or("missing original length")?,
                            )
                        {
                            return Err("legacy forecast source changed".into());
                        }
                        let metadata = crate::store::CanonicalForecastMetadata {
                            provider: record["provider"]
                                .as_str()
                                .ok_or("missing provider")?
                                .into(),
                            model: record["model"].as_str().ok_or("missing model")?.into(),
                            product: record["product"].as_str().ok_or("missing product")?.into(),
                            cycle_utc: record["cycle_utc"]
                                .as_str()
                                .ok_or("missing cycle")?
                                .parse::<chrono::DateTime<chrono::Utc>>()
                                .map_err(|e| e.to_string())?,
                        };
                        catalog
                            .register_canonical(&native, &metadata)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        let assets = source.join("assets");
        if assets.is_dir() {
            for entry in fs::read_dir(&assets).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry.file_name();
                let label = name.to_string_lossy();
                if label == "forecasts"
                    || label == "observations"
                    || label == "ana-partial-coverage.json"
                {
                    continue;
                }
                if (label == "ecmwf" || label == "noaa")
                    && entry.file_type().map_err(|e| e.to_string())?.is_dir()
                {
                    for file in fs::read_dir(entry.path()).map_err(|e| e.to_string())? {
                        let file = file.map_err(|e| e.to_string())?;
                        if !file.file_type().map_err(|e| e.to_string())?.is_file()
                            || !migrated_native.contains(&file.path())
                        {
                            return Err(format!(
                                "unsupported legacy asset {}",
                                file.path().display()
                            ));
                        }
                    }
                } else {
                    return Err(format!(
                        "unsupported legacy asset {}",
                        entry.path().display()
                    ));
                }
            }
        }
        let old_forecasts = source.join("assets/forecasts");
        if old_forecasts.is_dir() {
            let catalog_root = super::forecast_assets(stage);
            let mut catalog = ForecastCatalog::open(stage, &catalog_root, &catalog_root)
                .map_err(|e| e.to_string())?;
            for entry in fs::read_dir(&old_forecasts).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                    if entry.file_name() == "access.lock" {
                        continue;
                    }
                    return Err(format!(
                        "unsupported legacy forecast catalog content {}",
                        entry.path().display()
                    ));
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                let record: Value = read_draft(&entry.path().join("metadata.json"))?;
                let native = member(
                    &source,
                    record["native_artifact"]
                        .as_str()
                        .ok_or("missing forecast native path")?,
                )?;
                let original = member(
                    &source,
                    record["retained_source"]
                        .as_str()
                        .ok_or("missing forecast source path")?,
                )?;
                if crate::store::digest_file(&native)?
                    != (
                        record["native_sha256"]
                            .as_str()
                            .ok_or("missing native digest")?
                            .into(),
                        record["native_byte_length"]
                            .as_u64()
                            .ok_or("missing native size")?,
                    )
                    || crate::store::digest_file(&original)?
                        != (
                            record["source_sha256"]
                                .as_str()
                                .ok_or("missing source digest")?
                                .into(),
                            record["source_byte_length"]
                                .as_u64()
                                .ok_or("missing source size")?,
                        )
                {
                    return Err("legacy forecast integrity failed".into());
                }
                let metadata = crate::store::CanonicalForecastMetadata {
                    provider: record["provider"]
                        .as_str()
                        .ok_or("missing provider")?
                        .into(),
                    model: record["model"].as_str().ok_or("missing model")?.into(),
                    product: record["product"].as_str().ok_or("missing product")?.into(),
                    cycle_utc: record["cycle_utc"]
                        .as_str()
                        .ok_or("missing cycle")?
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .map_err(|e| e.to_string())?,
                };
                let new = catalog
                    .register_canonical(&native, &metadata)
                    .map_err(|e| e.to_string())?;
                write_draft(
                    &legacy.join(format!("forecast-{name}.toml")),
                    &serde_json::json!({"legacy":record,"registered_id":new.asset_id}),
                )?;
            }
        }
        if let Some(sets) = interchange_manifest["legacy_selections"].as_array() {
            let prior_current = interchange_manifest["legacy_current_input"]["id"].as_str();
            let mut converted_current = None;
            for set in sets {
                let p = set.as_str().ok_or("invalid legacy selection path")?;
                let path = interchange.join(p);
                if path.canonicalize().map_err(|e| e.to_string())?.parent()
                    != Some(interchange.join("legacy-selections").as_path())
                {
                    return Err("invalid legacy selection location".into());
                }
                let submanifest: Value = read_draft(&path.join("manifest.json"))?;
                let original_index = member(
                    &path,
                    submanifest["legacy_input_set"]
                        .as_str()
                        .ok_or("missing legacy input index")?,
                )?;
                let original: Value = read_draft(&original_index)?;
                let converted = convert_saved_inputs_locked(stage, &path)?;
                if original["id"].as_str() == prior_current {
                    converted_current = Some(converted.id);
                }
            }
            let pointer = stage.join("assets/selections/current.toml");
            if let Some(id) = converted_current {
                write_draft(&pointer, &serde_json::json!({"id":id,"saved_to":null}))?;
            } else if prior_current.is_some() {
                return Err("legacy current selection was not converted".into());
            } else if pointer.exists() {
                fs::remove_file(pointer).map_err(|e| e.to_string())?;
            }
        }
        if active.is_none() {
            let pointer = source.join("cache/run/active.json");
            if pointer.exists() {
                let value: Value = read_draft(&pointer)?;
                active = value.as_str().map(str::to_owned);
            }
        }
        convert_attempts(
            &source,
            stage,
            &legacy,
            active.as_deref(),
            &database_attempts,
        )?;
        let mut config = super::read_workspace_config(&stage.join("config.toml"))?;
        config.run.get_or_insert_with(Default::default).time_zone = Some(
            zone.parse()
                .map_err(|e: chrono_tz::ParseError| e.to_string())?,
        );
        super::write_workspace_config(&stage.join("config.toml"), &config)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_legacy_interchange_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("operation.sqlite");
        fs::write(&source, b"original").unwrap();
        let (sha256, size) = crate::store::digest_file(&source).unwrap();
        let manifest = serde_json::json!({"source_kind":"legacy_sqlite","source_sqlite_files":{"operation.sqlite":{"sha256":sha256,"size":size}}});
        verify_interchange_source(dir.path(), &manifest).unwrap();
        fs::write(source, b"revised").unwrap();
        assert!(verify_interchange_source(dir.path(), &manifest).is_err());
    }

    #[test]
    fn converted_result_preserves_original_manifest_binding() {
        let dir = tempfile::tempdir().unwrap();
        let old = br#"{"schema_version":1,"recipe":{},"inputs":[],"scenarios":[]}"#;
        let original = dir.path().join("manifest.json");
        fs::write(&original, old).unwrap();
        let expected = crate::store::digest_file(&original).unwrap();
        convert_result_manifests(dir.path(), "20261127-12", "old-attempt").unwrap();
        assert_eq!(
            fs::read(dir.path().join("legacy-manifest.json")).unwrap(),
            old
        );
        let manifest: Value = read_draft(&dir.path().join("manifest.toml")).unwrap();
        assert_eq!(manifest["recipe"]["legacy_attempt_id"], "old-attempt");
        assert_eq!(manifest["recipe"]["legacy_manifest_sha256"], expected.0);
        assert_eq!(manifest["inputs"][0]["size"], expected.1);
        assert_eq!(manifest["inputs"][0]["path"], "legacy-manifest.json");
    }
}
