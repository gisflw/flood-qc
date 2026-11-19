use super::charts::{Charts, seconds};
use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use eframe::egui::{self, Color32};
use mgb_ops::app::SeriesReport;
use mgb_ops::store::LevelReference;
use std::path::Path;

pub struct Export {
    pub state: eframe::egui_wgpu::RenderState,
    pub report: SeriesReport,
    pub source_labels: Vec<String>,
    pub scenario_order: Vec<String>,
    pub references: Vec<LevelReference>,
    pub portuguese: bool,
    pub scale: u32,
    pub title: String,
    pub limits: [(f64, f64, f64, f64); 3],
    pub right_level_limits: Option<(f64, f64)>,
}
struct Canvas {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    font: FontArc,
    scale: u32,
}
impl Canvas {
    fn line(&mut self, x: f64, y: f64, end_x: f64, end_y: f64, color: Color32) {
        let steps =
            ((end_x - x).abs().max((end_y - y).abs()) * f64::from(self.scale)).ceil() as u32;
        for step in 0..=steps {
            let t = f64::from(step) / f64::from(steps.max(1));
            let px = ((x + (end_x - x) * t) * f64::from(self.scale)) as u32;
            let py = ((y + (end_y - y) * t) * f64::from(self.scale)) as u32;
            for dy in 0..self.scale {
                for dx in 0..self.scale {
                    if px + dx < self.width && py + dy < self.height {
                        let index = ((py + dy) * self.width + px + dx) as usize * 4;
                        self.pixels[index..index + 4].copy_from_slice(&color.to_array());
                    }
                }
            }
        }
    }
    fn text(&mut self, x: f32, y: f32, value: &str, color: Color32) {
        let scale = PxScale::from(13. * self.scale as f32);
        let scaled = self.font.as_scaled(scale);
        let mut position = ab_glyph::point(x * self.scale as f32, (y + 13.) * self.scale as f32);
        for ch in value.chars() {
            let glyph = scaled.scaled_glyph(ch);
            let advance = scaled.h_advance(glyph.id);
            let glyph = ab_glyph::Glyph { position, ..glyph };
            if let Some(outline) = self.font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|gx, gy, coverage| {
                    let px = bounds.min.x as i32 + gx as i32;
                    let py = bounds.min.y as i32 + gy as i32;
                    if px >= 0 && py >= 0 && px < self.width as i32 && py < self.height as i32 {
                        let index = (py as usize * self.width as usize + px as usize) * 4;
                        for (i, channel) in color.to_array()[..3].iter().enumerate() {
                            self.pixels[index + i] = (f32::from(*channel) * coverage
                                + f32::from(self.pixels[index + i]) * (1. - coverage))
                                as u8;
                        }
                    }
                });
            }
            position.x += advance;
        }
    }
}
/// rsplot exports only the data region; add the monitoring axes, titles and legends here.
pub fn write(export: Export, destination: &Path) -> Result<(), String> {
    if !(1..=3).contains(&export.scale) {
        return Err("PNG scale must be 1, 2 or 3".into());
    }
    let mut charts = Charts::new(&export.state, 100);
    charts.source_labels = export.source_labels;
    charts.scenario_order = export.scenario_order;
    charts.line_scale = export.scale as f32;
    charts.load(export.report.clone(), &export.references, export.portuguese)?;
    let scale = export.scale;
    let width = 1200 * scale;
    let legend_rows = charts.labels.len().div_ceil(3) as u32;
    let height = (960 + legend_rows * 20) * scale;
    let fonts = egui::FontDefinitions::default();
    let bytes = fonts
        .font_data
        .values()
        .next()
        .ok_or("no export font")?
        .font
        .to_vec();
    let font = FontArc::try_from_vec(bytes).map_err(|e| e.to_string())?;
    let mut canvas = Canvas {
        pixels: vec![255; width as usize * height as usize * 4],
        width,
        height,
        font,
        scale,
    };
    canvas.text(15., 10., &export.title, Color32::BLACK);
    canvas.text(
        15.,
        30.,
        &format!(
            "{} · {} → {}",
            export.report.time_zone, export.report.window.start, export.report.window.end
        ),
        Color32::BLACK,
    );
    for (i, plot) in charts.plots.iter_mut().enumerate() {
        let (xmin, xmax, ymin, ymax) = export.limits[i];
        plot.set_graph_x_limits(xmin, xmax);
        plot.set_graph_y_limits(ymin, ymax, rsplot::YAxis::Left);
        plot.plot_mut().data_background = Color32::WHITE;
        plot.backend().flush_scene();
        let right = (i == 2).then_some(export.right_level_limits).flatten();
        if let Some((min, max)) = right {
            plot.set_graph_y_limits(min, max, rsplot::YAxis::Right);
        }
        let logical_width = if right.is_some() { 980 } else { 1050 };
        let end_x = 120. + f64::from(logical_width);
        let data_width = logical_width * scale;
        let data_height = 210 * scale;
        let rgba = rsplot::render::save::render_plot_rgba(
            &export.state,
            plot.plot(),
            (data_width, data_height),
        )
        .map_err(|e| e.to_string())?;
        let top = 100 + i as u32 * 285;
        for row in 0..data_height {
            let to = ((top * scale + row) * width + 120 * scale) as usize * 4;
            let from = (row * data_width) as usize * 4;
            canvas.pixels[to..to + data_width as usize * 4]
                .copy_from_slice(&rgba[from..from + data_width as usize * 4]);
        }
        let title = if export.portuguese {
            ["Chuva (mm)", "Vazão (m³/s)", "Nível (cm)"][i]
        } else {
            ["Rainfall (mm)", "Flow (m³/s)", "Level (cm)"][i]
        };
        canvas.line(
            120.,
            f64::from(top),
            120.,
            f64::from(top + 210),
            Color32::BLACK,
        );
        canvas.line(
            120.,
            f64::from(top + 210),
            end_x,
            f64::from(top + 210),
            Color32::BLACK,
        );
        if right.is_some() {
            canvas.line(
                end_x,
                f64::from(top),
                end_x,
                f64::from(top + 210),
                Color32::BLACK,
            );
            canvas.text(
                15.,
                top as f32 - 24.,
                if export.portuguese {
                    "Nível da estação (cm)"
                } else {
                    "Station level (cm)"
                },
                Color32::BLACK,
            );
            canvas.text(
                end_x as f32,
                top as f32 - 24.,
                if export.portuguese {
                    "MINI bruto"
                } else {
                    "Raw MINI (cm)"
                },
                Color32::BLACK,
            );
        } else {
            canvas.text(15., top as f32 - 24., title, Color32::BLACK);
        }
        if i == 2 {
            for reference in &charts.references {
                let y = f64::from(top) + 210. * (ymax - reference.level_cm) / (ymax - ymin);
                if (f64::from(top)..=f64::from(top + 210)).contains(&y) {
                    canvas.line(120., y, end_x, y, Color32::RED);
                    canvas.text(
                        125.,
                        y as f32 - 16.,
                        &super::charts::reference_label(reference, export.portuguese),
                        Color32::RED,
                    );
                }
            }
        }
        for tick in 0..=4 {
            let fraction = f64::from(tick) / 4.;
            let x = xmin + (xmax - xmin) * fraction + seconds(export.report.window.start);
            let label = chrono::DateTime::from_timestamp(x as i64, 0)
                .map(|t| t.format("%m-%d %H:%M").to_string())
                .unwrap_or_default();
            canvas.text(
                (120. + f64::from(logical_width) * fraction - 40.).min(end_x - 75.) as f32,
                (top + 215) as f32,
                &label,
                Color32::BLACK,
            );
            canvas.text(
                15.,
                (f64::from(top) + 210. * (1. - fraction) - 7.) as f32,
                &format!("{:.2}", ymin + (ymax - ymin) * fraction),
                Color32::BLACK,
            );
            if let Some((min, max)) = right {
                canvas.text(
                    end_x as f32 + 12.,
                    (f64::from(top) + 210. * (1. - fraction) - 7.) as f32,
                    &format!("{:.2}", min + (max - min) * fraction),
                    Color32::BLACK,
                );
            }
        }
        // Reference and thresholds are UI overlays in rsplot, so retain their labels in the export.
        if let Some(reference) = export
            .report
            .series
            .iter()
            .find_map(|s| s.forecast_reference_time.as_ref())
            .and_then(|r| super::data::timestamp(r).ok())
        {
            let x = 120.
                + f64::from(logical_width)
                    * (seconds(reference) - seconds(export.report.window.start) - xmin)
                    / (xmax - xmin);
            if (120.0..=end_x).contains(&x) {
                canvas.line(x, f64::from(top), x, f64::from(top + 210), Color32::RED);
                canvas.text(
                    x as f32,
                    (top + 240) as f32,
                    &format!("Ref. {}", reference.format("%m-%d %H:%M")),
                    Color32::RED,
                );
            }
        }
    }
    for (index, (label, color)) in charts.labels.iter().enumerate() {
        canvas.text(
            15. + (index % 3) as f32 * 395.,
            960. + (index / 3) as f32 * 20.,
            label,
            *color,
        );
    }
    let png = rsplot::encode_png(&canvas.pixels, width, height).map_err(|e| e.to_string())?;
    mgb_ops::store::write_atomic(destination, &png)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a graphics adapter; software rendering is not operator-hardware qualification"]
    fn complete_png_has_titles_axes_legends_and_scaled_dimensions() {
        use mgb_ops::app::{ChartSeries, Location, SeriesPoint};
        let instance = eframe::egui_wgpu::wgpu::Instance::new(
            eframe::egui_wgpu::wgpu::InstanceDescriptor::new_without_display_handle(),
        );
        let state = pollster::block_on(eframe::egui_wgpu::RenderState::create(
            &Default::default(),
            &instance,
            None,
            Default::default(),
        ))
        .unwrap();
        println!("Adapter: {:?}", state.adapter.get_info());
        let start = super::super::data::timestamp("2026-01-01T00:00:00").unwrap();
        let end = start + chrono::Duration::hours(4);
        let series = [("rain", "mm"), ("flow", "m3/s"), ("level", "cm")]
            .into_iter()
            .enumerate()
            .map(|(i, (variable, unit))| ChartSeries {
                selection_index: i,
                series_kind: "observed_raw".into(),
                run_id: None,
                scenario_id: None,
                forecast_reference_time: None,
                variable: variable.into(),
                unit: unit.into(),
                points: (1..=4)
                    .map(|hour| SeriesPoint {
                        timestamp: start + chrono::Duration::hours(hour),
                        value: if hour == 2 {
                            None
                        } else {
                            Some(hour as f64 - 1.)
                        },
                        time_segment: None,
                    })
                    .collect(),
            })
            .collect();
        let mut series: Vec<_> = series;
        let mut raw = series[2].clone();
        raw.selection_index = 3;
        raw.series_kind = "model_raw_level".into();
        for point in &mut raw.points {
            point.value = point.value.map(|value| value + 100.);
        }
        series.push(raw);
        let report = SeriesReport {
            location: Location::Mini(1),
            time_zone: "America/Sao_Paulo".into(),
            window: mgb_ops::store::TimeInterval { start, end },
            series,
            findings: vec![],
        };
        let mut charts = Charts::new(&state, 10);
        charts.load(report.clone(), &[], false).unwrap();
        assert_eq!(charts.plots[0].plot().limits.0, 0.);
        assert_eq!(charts.plots[0].plot().limits.1, 14400.);
        assert_eq!(charts.origin, seconds(start));
        assert!(charts.plots[2].plot().y2.is_some());
        let context = egui::Context::default();
        let mut frame = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200., 600.),
                )),
                ..Default::default()
            },
            |ui| {
                let available_height = ui.available_height();
                charts.show(ui, false, available_height);
            },
        );
        frame.textures_delta.clear();
        for plot in &charts.plots {
            assert_eq!(plot.x_limits(), (0., 14400.));
            assert!(plot.plot_bounds_in_pixels().unwrap().height() > 50.);
        }

        let level_only = SeriesReport {
            location: Location::Mini(2),
            series: vec![report.series[2].clone()],
            ..report.clone()
        };
        let right_level_limits = charts.plots[2].plot().y2;
        charts.load(level_only, &[], false).unwrap();
        assert!(charts.plots[0].get_all_histograms().is_empty());
        assert!(charts.plots[1].get_all_curves().is_empty());
        assert!(charts.plots[2].plot().y2.is_none());
        assert_eq!(charts.plots[2].get_all_curves().len(), 2);

        std::fs::create_dir_all("runs/desktop-qualification").unwrap();
        for scale in 1..=3 {
            let destination =
                Path::new("runs/desktop-qualification").join(format!("export-{scale}x.png"));
            write(
                Export {
                    state: state.clone(),
                    report: report.clone(),
                    source_labels: vec![],
                    scenario_order: vec![],
                    references: vec![],
                    portuguese: true,
                    scale,
                    title: "MGB Ops · MINI: 1 × Estação: São José".into(),
                    right_level_limits,
                    limits: [(0., seconds(end) - seconds(start), 0., 4.); 3],
                },
                &destination,
            )
            .unwrap();
            let bytes = std::fs::read(&destination).unwrap();
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
            assert_eq!(
                u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
                1200 * scale
            );
            assert!(bytes.len() > 10000);
        }
    }
}
