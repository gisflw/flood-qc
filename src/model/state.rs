use super::ModelError;
use crate::store::{copy_tree, sync_tree, whole_hour};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    path::Path,
};

pub(crate) const QUALIFIED_RS_ENGINE_SHA256: &str =
    "263381e450484e5bc345e269f21c54e84d89afdff97ca2d9752eb951907f88eb";

const STATE_TABLES: [(&str, &[usize]); 5] = [
    ("CI_W.HOT", &[9, 9, 151]),
    ("CI_VOL.HOT", &[22]),
    ("CI_HFL.HOT", &[2]),
    ("CI_VOL1_FL.HOT", &[3]),
    ("CI_Q2FL.HOT", &[6]),
];

pub(crate) fn write_hot_save_request(input: &Path, at: NaiveDateTime) -> Result<(), ModelError> {
    if !whole_hour(at) || !(1..=9999).contains(&at.year()) {
        return Err(ModelError::InvalidTiming(
            "state save needs a whole-hour local timestamp".into(),
        ));
    }
    let path = input.join("DATA.hot");
    fs::write(
        &path,
        format!(
            "{:6}{:6}{:6}{:6}\n",
            at.year(),
            at.month(),
            at.day(),
            at.hour()
        ),
    )
    .map_err(|error| ModelError::Io(format!("{}: {error}", path.display())))
}

fn output_file(directory: &Path, name: &str) -> Result<File, ModelError> {
    let path = directory.join(name);
    let failure = |e: io::Error| {
        if e.kind() == io::ErrorKind::NotFound {
            ModelError::MissingOutput(format!("missing hot-start output {}", path.display()))
        } else {
            ModelError::Io(format!("{}: {e}", path.display()))
        }
    };
    let metadata = fs::symlink_metadata(&path).map_err(failure)?;
    if !metadata.is_file() {
        return Err(ModelError::MalformedOutput(format!(
            "invalid hot-start file {}",
            path.display()
        )));
    }
    File::open(&path).map_err(failure)
}

fn malformed(name: &str) -> ModelError {
    ModelError::MalformedOutput(format!("invalid hot-start output {name}"))
}

pub(crate) fn validate_hot_start(
    directory: &Path,
    at: NaiveDateTime,
    mini_count: usize,
) -> Result<(), ModelError> {
    if mini_count == 0 {
        return Err(ModelError::InvalidGeometry(
            "hot-start state requires mini-basins".into(),
        ));
    }
    let mut content = String::new();
    output_file(directory, "CI.meta")?
        .take(256)
        .read_to_string(&mut content)
        .map_err(|e| ModelError::Io(format!("CI.meta: {e}")))?;
    let fields = content
        .split_whitespace()
        .map(str::parse::<i32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| malformed("CI.meta"))?;
    let [year, month, day, hour] = fields.as_slice() else {
        return Err(malformed("CI.meta"));
    };
    let actual = NaiveDate::from_ymd_opt(*year, *month as u32, *day as u32)
        .and_then(|date| date.and_hms_opt(*hour as u32, 0, 0));
    if actual != Some(at) || content.len() >= 256 {
        return Err(ModelError::MalformedOutput(format!(
            "CI.meta differs from requested save time {at}"
        )));
    }
    for (name, fields_per_row) in STATE_TABLES {
        let mut rows = BufReader::new(output_file(directory, name)?).lines();
        for _ in 0..mini_count {
            for &expected in fields_per_row {
                let row = rows
                    .next()
                    .ok_or_else(|| malformed(name))?
                    .map_err(|e| ModelError::Io(format!("{name}: {e}")))?;
                let mut fields = row.split_ascii_whitespace();
                for _ in 0..expected {
                    if fields
                        .next()
                        .and_then(|field| field.parse::<f64>().ok())
                        .is_none_or(|value| !value.is_finite())
                    {
                        return Err(malformed(name));
                    }
                }
                if fields.next().is_some() {
                    return Err(malformed(name));
                }
            }
        }
        if rows.next().is_some() {
            return Err(malformed(name));
        }
    }
    Ok(())
}

pub(crate) fn package_hot_start(
    output: &Path,
    destination: &Path,
    at: NaiveDateTime,
    mini_count: usize,
) -> Result<(), ModelError> {
    fs::create_dir(destination)
        .map_err(|e| ModelError::Io(format!("{}: {e}", destination.display())))?;
    let result = (|| {
        for name in std::iter::once("CI.meta").chain(STATE_TABLES.iter().map(|table| table.0)) {
            output_file(output, name)?;
            copy_tree(&output.join(name), &destination.join(name)).map_err(ModelError::Io)?;
        }
        validate_hot_start(destination, at, mini_count)?;
        sync_tree(destination).map_err(ModelError::Io)
    })();
    if result.is_err() {
        fs::remove_dir_all(destination)
            .map_err(|e| ModelError::Io(format!("discarding {}: {e}", destination.display())))?;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_package_requires_matching_complete_finite_outputs() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let package = dir.path().join("state");
        fs::create_dir(&output).unwrap();
        let at: NaiveDateTime = "2026-07-11T01:00:00".parse().unwrap();
        write_hot_save_request(&output, at).unwrap();
        let meta = fs::read_to_string(output.join("DATA.hot")).unwrap();
        assert_eq!(meta, "  2026     7    11     1\n");
        fs::write(output.join("CI.meta"), meta).unwrap();
        for (name, rows) in STATE_TABLES {
            let content: String = rows
                .iter()
                .map(|&columns| format!("{}\n", " 0.1".repeat(columns)))
                .collect();
            fs::write(output.join(name), content).unwrap();
        }
        package_hot_start(&output, &package, at, 1).unwrap();
        assert_eq!(fs::read_dir(&package).unwrap().count(), 6);
        assert!(validate_hot_start(&package, at + chrono::Duration::hours(1), 1).is_err());
        fs::write(output.join("CI_HFL.HOT"), "NaN 0.0\n").unwrap();
        let rejected = dir.path().join("rejected");
        assert!(package_hot_start(&output, &rejected, at, 1).is_err());
        let removed = !rejected.exists();
        assert!(removed);
        assert!(package_hot_start(&output, &package, at, 1).is_err());
        assert_eq!(fs::read_dir(&package).unwrap().count(), 6);
    }
}
