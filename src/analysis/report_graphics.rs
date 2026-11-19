use std::fmt::Write;

use crate::store::{MiniMapFeature, TimeInterval};

use super::{HydrographPoint, HydrographThreshold};

pub(super) const FONT: &[u8] = include_bytes!("assets/NotoSans-Regular.ttf");

pub(super) fn markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn raster(svg: &str) -> Result<Vec<u8>, String> {
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_font_data(FONT.to_vec());
    options.font_family = "Noto Sans".into();
    let tree = resvg::usvg::Tree::from_str(svg, &options).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or("cannot allocate report image")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

const COLORS: [&str; 6] = [
    "#237a68", "#305cb0", "#c26033", "#8954a0", "#9c7812", "#415d63",
];

pub fn render_hydrograph_png(
    curves: &[(String, &[HydrographPoint])],
    unit: &str,
    window: &TimeInterval,
    thresholds: &[HydrographThreshold],
) -> Result<Vec<u8>, String> {
    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    for value in curves
        .iter()
        .flat_map(|(_, points)| {
            points.iter().flat_map(|point| {
                [point.value, point.lower, point.upper]
                    .into_iter()
                    .flatten()
            })
        })
        .chain(thresholds.iter().map(|v| v.value))
    {
        minimum = minimum.min(value);
        maximum = maximum.max(value);
    }
    if !minimum.is_finite() || !maximum.is_finite() {
        return Err("no finite hydrograph values to plot".into());
    }
    if minimum == maximum {
        let pad = (minimum.abs() * 0.05).max(1.0);
        minimum -= pad;
        maximum += pad;
    }
    let seconds = (window.end - window.start).num_seconds() as f64;
    let x = |at: chrono::NaiveDateTime| {
        84.0 + 815.0 * (at - window.start).num_seconds() as f64 / seconds
    };
    let y = |value: f64| 345.0 - 280.0 * (value - minimum) / (maximum - minimum);
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="960" height="440"><rect width="960" height="440" fill="#fff"/><g font-family="Noto Sans" font-size="14" fill="#273d48">"##,
    );
    let unit = markup(unit);
    let _ = write!(svg, r#"<text x="28" y="37">{unit}</text>"#);
    for i in 0..=4 {
        let value = minimum + (maximum - minimum) * f64::from(i) / 4.0;
        let at = window.start + (window.end - window.start) * i / 4;
        let (px, py) = (x(at), y(value));
        let tick = at.format("%d/%m %H:%M");
        let _ = write!(
            svg,
            r##"<path d="M84 {py}H899" stroke="#dce6e9"/><text x="76" y="{py}" text-anchor="end">{value:.1}</text><text x="{px}" y="371" text-anchor="middle">{tick}</text>"##
        );
    }
    for threshold in thresholds {
        let py = y(threshold.value);
        let name = markup(&threshold.name);
        let _ = write!(
            svg,
            r##"<path d="M84 {py}H899" stroke="#b95f37" stroke-dasharray="5 4"/><text x="895" y="{py}" text-anchor="end">{name}</text>"##
        );
    }
    for (index, (label, points)) in curves.iter().enumerate() {
        let color = COLORS[index % COLORS.len()];
        for pair in points.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if b.timestamp - a.timestamp > chrono::Duration::hours(1) {
                continue;
            }
            if let (Some(lo1), Some(hi1), Some(lo2), Some(hi2)) =
                (a.lower, a.upper, b.lower, b.upper)
            {
                let _ = write!(
                    svg,
                    r#"<path d="M{} {}L{} {}L{} {}L{} {}Z" fill="{color}" opacity=".14"/>"#,
                    x(a.timestamp),
                    y(lo1),
                    x(a.timestamp),
                    y(hi1),
                    x(b.timestamp),
                    y(hi2),
                    x(b.timestamp),
                    y(lo2),
                );
            }
            if let (Some(v1), Some(v2)) = (a.value, b.value) {
                let dash = if b.segment == Some(1) { "6 4" } else { "none" };
                let _ = write!(
                    svg,
                    r#"<path d="M{} {}L{} {}" fill="none" stroke="{color}" stroke-width="2" stroke-dasharray="{dash}"/>"#,
                    x(a.timestamp),
                    y(v1),
                    x(b.timestamp),
                    y(v2)
                );
            }
        }
        for point in *points {
            if let Some(value) = point.value {
                let _ = write!(
                    svg,
                    r#"<circle cx="{}" cy="{}" r="1.7" fill="{color}"/>"#,
                    x(point.timestamp),
                    y(value)
                );
            }
        }
        if index < 5 {
            let px = 84 + index * 175;
            let label = markup(&label.chars().take(23).collect::<String>());
            let _ = write!(
                svg,
                r#"<text x="{px}" y="412" fill="{color}">{label}</text>"#
            );
        }
    }
    svg.push_str("</g></svg>");
    raster(&svg)
}

pub fn render_rain_map_png(
    features: &[(&MiniMapFeature, Option<f64>)],
    stations: &[(String, f64, f64)],
) -> Result<Vec<u8>, String> {
    let mut west = f64::INFINITY;
    let mut east = f64::NEG_INFINITY;
    let mut south = f64::INFINITY;
    let mut north = f64::NEG_INFINITY;
    for &(x, y) in features.iter().flat_map(|(feature, _)| {
        feature
            .polygons
            .iter()
            .flat_map(|polygon| polygon.exterior.iter())
    }) {
        west = west.min(x);
        east = east.max(x);
        south = south.min(y);
        north = north.max(y);
    }
    for (_, x, y) in stations {
        west = west.min(*x);
        east = east.max(*x);
        south = south.min(*y);
        north = north.max(*y);
    }
    if !(west < east && south < north) {
        return Err("rainfall map has no usable bounds".into());
    }
    let scale = (800.0 / (east - west)).min(340.0 / (north - south));
    let xy = |x: f64, y: f64| {
        (
            480.0 + (x - (west + east) / 2.0) * scale,
            208.0 - (y - (south + north) / 2.0) * scale,
        )
    };
    let maximum = features
        .iter()
        .filter_map(|entry| entry.1)
        .fold(0.0_f64, f64::max);
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="960" height="440"><rect width="960" height="440" fill="#fff"/>"##,
    );
    for (feature, value) in features {
        let fill = match value {
            Some(value) => {
                let fraction = if maximum > 0.0 { value / maximum } else { 0.0 };
                format!(
                    "rgb({},{},{})",
                    231 - (190.0 * fraction) as u8,
                    246 - (111.0 * fraction) as u8,
                    250 - (65.0 * fraction) as u8
                )
            }
            None => "#dadde0".into(),
        };
        let _ = write!(
            svg,
            r##"<path fill="{fill}" fill-rule="evenodd" stroke="#8aa3af" stroke-width=".7" d=""##
        );
        for polygon in &feature.polygons {
            for ring in std::iter::once(&polygon.exterior).chain(&polygon.interiors) {
                for (index, &(lon, lat)) in ring.iter().enumerate() {
                    let (x, y) = xy(lon, lat);
                    let command = if index == 0 { 'M' } else { 'L' };
                    let _ = write!(svg, "{command}{x:.2} {y:.2}");
                }
                svg.push('Z');
            }
        }
        svg.push_str("\"/>");
    }
    for (label, lon, lat) in stations {
        let (x, y) = xy(*lon, *lat);
        let label = markup(label);
        let _ = write!(
            svg,
            r##"<circle cx="{x}" cy="{y}" r="4" fill="#c26033" stroke="#fff"/><text x="{}" y="{}" font-size="11" fill="#273d48">{label}</text>"##,
            x + 5.0,
            y - 4.0
        );
    }
    let _ = write!(
        svg,
        r##"<text x="45" y="416" font-size="15" fill="#273d48">0–{maximum:.1} mm · Cinza = sem cobertura</text></svg>"##
    );
    raster(&svg)
}
