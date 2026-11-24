use std::{collections::HashSet, path::Path};

pub(crate) fn channel_dimension(area: f64, a: f64, b: f64) -> Result<f64, String> {
    if !a.is_finite() || a <= 0.0 || !b.is_finite() {
        return Err("channel equation requires finite coefficients and positive a".into());
    }
    let value = a * area.powf(b);
    if !value.is_finite() || value <= 0.0 {
        return Err("channel equation must produce a finite positive dimension".into());
    }
    Ok(value)
}

pub(crate) fn channel_table(path: &Path) -> Result<Vec<(usize, f64, f64)>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| e.to_string())?;
    let mut minis = HashSet::new();
    let mut values = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| e.to_string())?;
        if index == 0
            && record.len() == 3
            && record
                .iter()
                .zip(["mini", "width", "depth"])
                .all(|(field, name)| field.eq_ignore_ascii_case(name))
        {
            continue;
        }
        if record.len() != 3 {
            return Err(format!(
                "channel table row {} needs Mini;Width;Depth",
                index + 1
            ));
        }
        let mini = record[0]
            .parse()
            .map_err(|_| format!("invalid channel table MINI on row {}", index + 1))?;
        let parse = |index: usize| {
            record[index]
                .parse::<f64>()
                .map_err(|_| format!("invalid channel table dimension for MINI {mini}"))
        };
        if !minis.insert(mini) {
            return Err(format!("duplicate channel table MINI {mini}"));
        }
        values.push((mini, parse(1)?, parse(2)?));
    }
    if values.is_empty() {
        return Err("empty channel table".into());
    }
    Ok(values)
}
