use crate::model::{GeometryFile, SoilFile, parse_classes, watershed_count};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub type CalibrationGeometryRow = crate::store::CalibrationGeometryRow;
pub type GeometryField = crate::store::GeometryField;
pub type HydrologicClass = crate::store::HydrologicClass;
pub type SoilWatershed = crate::store::SoilWatershed;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelDimension {
    Width,
    Depth,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeometryEdit {
    Cell {
        mini: usize,
        field: GeometryField,
        value: f64,
    },
    Manning {
        subbasin: Option<usize>,
        value: f64,
    },
    Equation {
        dimension: ChannelDimension,
        a: f64,
        b: f64,
    },
}

pub struct CalibrationGeometry(GeometryFile);

impl CalibrationGeometry {
    pub fn read(path: &Path) -> Result<Self, String> {
        read(path, GeometryFile::parse).map(Self)
    }

    pub fn rows(&self) -> &[CalibrationGeometryRow] {
        self.0.rows()
    }

    pub fn block_count(&self) -> usize {
        self.0.block_count()
    }

    pub fn edit(&mut self, edit: GeometryEdit) -> Result<usize, String> {
        match edit {
            GeometryEdit::Cell { mini, field, value } => self.0.apply([(mini, field, value)]),
            GeometryEdit::Manning { subbasin, value } => {
                let edits: Vec<_> = self
                    .rows()
                    .iter()
                    .filter(|row| subbasin.is_none_or(|id| row.subbasin == id))
                    .map(|row| (row.mini, GeometryField::Manning, value))
                    .collect();
                if edits.is_empty() {
                    return Err("no MINIs in the selected subbasin".into());
                }
                self.0.apply(edits)
            }
            GeometryEdit::Equation { dimension, a, b } => {
                let field = match dimension {
                    ChannelDimension::Width => GeometryField::Width,
                    ChannelDimension::Depth => GeometryField::Depth,
                };
                let edits = self
                    .rows()
                    .iter()
                    .map(|row| {
                        crate::analysis::channel_dimension(row.upstream_area, a, b)
                            .map(|value| (row.mini, field, value))
                            .map_err(|e| format!("MINI {}: {e}", row.mini))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.0.apply(edits)
            }
        }
    }

    pub fn apply_channel_table(&mut self, path: &Path) -> Result<usize, String> {
        let edits = crate::analysis::channel_table(path)?;
        self.0
            .apply(edits.into_iter().flat_map(|(mini, width, depth)| {
                [
                    (mini, GeometryField::Width, width),
                    (mini, GeometryField::Depth, depth),
                ]
            }))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.0.validate_blocks()?;
        crate::store::save_calibration(path, self.0.bytes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SoilEdit {
    Class {
        watershed: usize,
        class: usize,
        values: [f64; 7],
    },
    Coefficients {
        watershed: usize,
        values: [f64; 4],
    },
    CopyWatershed {
        from: usize,
        to: usize,
    },
    CopyToAllWatersheds {
        from: usize,
    },
    CopyToAllClasses {
        watershed: usize,
        from: usize,
    },
}

pub struct SoilCalibration {
    file: SoilFile,
    classes: Vec<HydrologicClass>,
}

impl SoilCalibration {
    pub fn read(path: &Path, geometry: &CalibrationGeometry, hrc: &Path) -> Result<Self, String> {
        let classes = read_classes(geometry, hrc)?;
        let names: Vec<_> = classes.iter().map(|class| class.name.as_str()).collect();
        let file = read(path, |bytes| {
            SoilFile::parse(bytes, watershed_count(&geometry.0), &names)
        })?;
        Ok(Self { file, classes })
    }

    pub fn create(geometry: &CalibrationGeometry, hrc: &Path) -> Result<Self, String> {
        let classes = read_classes(geometry, hrc)?;
        let names: Vec<_> = classes.iter().map(|class| class.name.as_str()).collect();
        let file = SoilFile::create(watershed_count(&geometry.0), &names)?;
        Ok(Self { file, classes })
    }

    pub fn classes(&self) -> &[HydrologicClass] {
        &self.classes
    }

    pub fn watersheds(&self) -> &[SoilWatershed] {
        self.file.watersheds()
    }

    pub fn edit(&mut self, edit: SoilEdit) -> Result<usize, String> {
        let watershed = |id: usize| {
            self.watersheds()
                .get(id.wrapping_sub(1))
                .cloned()
                .ok_or_else(|| format!("unknown Watershed {id}"))
        };
        let class_index = |class: usize| {
            if (1..=self.classes.len()).contains(&class) {
                Ok(class - 1)
            } else {
                Err(format!("unknown hydrologic class {class}"))
            }
        };
        let changes = match edit {
            SoilEdit::Class {
                watershed: id,
                class,
                values,
            } => {
                let mut target = watershed(id)?;
                target.classes[class_index(class)?] = values;
                vec![target]
            }
            SoilEdit::Coefficients {
                watershed: id,
                values,
            } => {
                let mut target = watershed(id)?;
                target.coefficients = values;
                vec![target]
            }
            SoilEdit::CopyWatershed { from, to } => {
                let mut target = watershed(from)?;
                watershed(to)?;
                target.id = to;
                vec![target]
            }
            SoilEdit::CopyToAllWatersheds { from } => {
                let source = watershed(from)?;
                (1..=self.watersheds().len())
                    .filter(|id| *id != from)
                    .map(|id| SoilWatershed {
                        id,
                        ..source.clone()
                    })
                    .collect()
            }
            SoilEdit::CopyToAllClasses {
                watershed: id,
                from,
            } => {
                let mut target = watershed(id)?;
                let values = target.classes[class_index(from)?];
                target.classes.fill(values);
                vec![target]
            }
        };
        self.file.apply(changes)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        crate::store::save_calibration(path, self.file.bytes())
    }
}

fn read_classes(
    geometry: &CalibrationGeometry,
    path: &Path,
) -> Result<Vec<HydrologicClass>, String> {
    let classes = read(path, |bytes| parse_classes(&bytes))?;
    if classes.len() != geometry.block_count() {
        return Err(format!(
            "{}: HRC count does not match geometry BLC count",
            path.display()
        ));
    }
    Ok(classes)
}

fn read<T>(path: &Path, parse: impl FnOnce(Vec<u8>) -> Result<T, String>) -> Result<T, String> {
    fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(parse)
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn validate_selected_calibration(
    baseline_input: &Path,
    geometry: Option<&Path>,
    soil: Option<&Path>,
) -> Result<(), String> {
    crate::model::validate_calibration(baseline_input, geometry, soil)
}

#[cfg(test)]
mod tests;
