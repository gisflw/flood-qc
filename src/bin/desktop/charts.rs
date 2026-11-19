use eframe::egui::{self, Color32};
use mgb_ops::app::{Location, SeriesPoint, SeriesReport};
use mgb_ops::store::LevelReference;
use rsplot::{CurveSpec, ItemHandle, LineStyle, Plot1D, TickMode, YAxis};

pub struct Charts {
    pub state: eframe::egui_wgpu::RenderState,
    pub plots: [Plot1D; 3],
    cursors: [ItemHandle; 3],
    pub report: Option<SeriesReport>,
    pub comparison_station: Option<String>,
    pub source_labels: Vec<String>,
    pub scenario_order: Vec<String>,
    pub references: Vec<LevelReference>,
    pub labels: Vec<(String, Color32)>,
    portuguese: bool,
    pub origin: f64,
    pub line_scale: f32,
}
pub fn location_label(location: &Location, portuguese: bool) -> String {
    match location {
        Location::Station(id) => {
            format!("{}: {id}", if portuguese { "Estação" } else { "Station" })
        }
        Location::Mini(id) => format!("MINI: {id}"),
    }
}
pub fn reference_label(reference: &LevelReference, portuguese: bool) -> String {
    let kind = if portuguese {
        match reference.kind.as_str() {
            "attention" => "Atenção",
            "alert" => "Alerta",
            "flood" => "Inundação",
            "severe" => "Grave",
            "historical" => "Histórico",
            other => other,
        }
    } else {
        reference.kind.as_str()
    };
    let date = reference
        .date
        .map(|d| format!(" · {d}"))
        .unwrap_or_default();
    format!("{kind}{date}: {} cm", reference.level_cm)
}
pub fn seconds(timestamp: chrono::NaiveDateTime) -> f64 {
    timestamp.and_utc().timestamp() as f64
        + f64::from(timestamp.and_utc().timestamp_subsec_nanos()) / 1e9
}
fn source_color(scenario_order: &[String], scenario_id: Option<&str>) -> Color32 {
    let colors = [
        Color32::from_rgb(25, 145, 205),
        Color32::from_rgb(190, 85, 190),
        Color32::from_rgb(55, 175, 95),
        Color32::from_rgb(235, 145, 35),
    ];
    scenario_id.map_or(Color32::from_rgb(135, 145, 155), |id| {
        let index = scenario_order.iter().position(|scenario| scenario == id);
        colors[index.unwrap_or(0) % colors.len()]
    })
}
/// Split on masks and history/forecast boundaries; never bridge absent measurements.
fn chunks(points: &[SeriesPoint]) -> Vec<&[SeriesPoint]> {
    let mut chunks = Vec::new();
    let mut start = 0;
    for i in 0..points.len() {
        if points[i].value.is_none()
            || (i > start && points[i].time_segment != points[i - 1].time_segment)
        {
            if start < i {
                chunks.push(&points[start..i]);
            }
            start = if points[i].value.is_none() { i + 1 } else { i };
        }
    }
    if start < points.len() {
        chunks.push(&points[start..]);
    }
    chunks
}
impl Charts {
    pub fn new(state: &eframe::egui_wgpu::RenderState, base: u64) -> Self {
        let mut plots = std::array::from_fn(|i| Plot1D::new(state, base + i as u64));
        for (i, plot) in plots.iter_mut().enumerate() {
            plot.set_graph_y_label(["mm", "m³/s", "cm"][i], YAxis::Left);
            plot.plot_mut().set_x_tick_mode(TickMode::TimeSeries);
            plot.set_graph_cursor(true);
            plot.set_auto_reset_zoom(false);
        }
        let cursors = std::array::from_fn(|i| plots[i].add_x_marker(0., Color32::GRAY));
        Self {
            state: state.clone(),
            plots,
            cursors,
            report: None,
            comparison_station: None,
            source_labels: vec![],
            scenario_order: vec![],
            references: vec![],
            labels: vec![],
            portuguese: false,
            origin: 0.,
            line_scale: 1.,
        }
    }
    pub fn clear(&mut self) {
        for plot in &mut self.plots {
            plot.clear();
            plot.backend().flush_scene();
        }
        self.report = None;
        self.comparison_station = None;
        self.source_labels.clear();
        self.scenario_order.clear();
        self.references.clear();
        self.labels.clear();
    }
    pub fn location_caption(&self, report: &SeriesReport, portuguese: bool) -> String {
        let mini = location_label(&report.location, portuguese);
        self.comparison_station
            .as_ref()
            .map_or(mini.clone(), |station| {
                format!(
                    "{mini} × {}",
                    location_label(&Location::Station(station.clone()), portuguese)
                )
            })
    }

    pub fn load(
        &mut self,
        report: SeriesReport,
        references: &[LevelReference],
        portuguese: bool,
    ) -> Result<(), String> {
        self.labels.clear();
        self.portuguese = portuguese;
        self.origin = seconds(report.window.start);
        for plot in &mut self.plots {
            plot.plot_mut().set_x_time_offset(self.origin);
        }
        for (i, plot) in self.plots.iter_mut().enumerate() {
            plot.clear();
            plot.set_graph_title(if portuguese {
                ["Chuva", "Vazão", "Nível"][i]
            } else {
                ["Rainfall", "Flow", "Level"][i]
            });
            if i == 2 {
                plot.set_graph_x_label(format!(
                    "{} · {}",
                    if portuguese {
                        "Hora local"
                    } else {
                        "Local time"
                    },
                    report.time_zone
                ));
            } else {
                plot.clear_graph_x_label();
            }
        }
        let has_level = |raw| {
            report.series.iter().any(|series| {
                series.variable == "level"
                    && (series.series_kind == "model_raw_level") == raw
                    && series.points.iter().any(|point| point.value.is_some())
            })
        };
        let has_station_levels = has_level(false);
        self.references.clear();
        if has_station_levels {
            self.references.extend_from_slice(references);
        }
        let separate_raw = has_station_levels && has_level(true);
        self.plots[2].set_graph_y_label(
            if separate_raw {
                if portuguese {
                    "Estação (cm)"
                } else {
                    "Station (cm)"
                }
            } else {
                "cm"
            },
            YAxis::Left,
        );
        if separate_raw {
            self.plots[2].set_graph_y_label(
                if portuguese {
                    "MINI bruto (cm)"
                } else {
                    "Raw MINI (cm)"
                },
                YAxis::Right,
            );
        } else {
            self.plots[2].clear_graph_y_label(YAxis::Right);
            self.plots[2].plot_mut().y2 = None;
        }
        for series in &report.series {
            let index = match series.variable.as_str() {
                "rain" => 0,
                "flow" => 1,
                _ => 2,
            };
            let color = source_color(&self.scenario_order, series.scenario_id.as_deref());
            let name = series
                .scenario_id
                .clone()
                .unwrap_or_else(|| if portuguese { "Observado" } else { "Observed" }.into());
            let raw_level = series.series_kind == "model_raw_level";
            let label = if raw_level {
                format!(
                    "{name} · {}",
                    if portuguese {
                        "nível bruto da MINI"
                    } else {
                        "raw MINI level"
                    }
                )
            } else if series.series_kind == "model_station_level" {
                format!(
                    "{name} · {}",
                    if portuguese {
                        "datum da estação"
                    } else {
                        "station datum"
                    }
                )
            } else if series.scenario_id.is_none() {
                self.source_labels
                    .get(series.selection_index)
                    .map_or(name.clone(), |source| format!("{name} · {source}"))
            } else {
                name
            };
            if !self.labels.iter().any(|(s, _)| s == &label) {
                self.labels.push((label.clone(), color));
            }
            let plot = &mut self.plots[index];
            for (part, chunk) in chunks(&series.points).into_iter().enumerate() {
                let x: Vec<_> = chunk
                    .iter()
                    .map(|p| seconds(p.timestamp) - self.origin)
                    .collect();
                let y: Vec<_> = chunk.iter().map(|p| p.value.unwrap()).collect();
                let handle = if index == 0 {
                    // Shared series queries admit hourly rainfall only; these are interval edges.
                    let mut edges = vec![x[0] - 3600.];
                    edges.extend(&x);
                    plot.add_histogram(&edges, &y, color)
                        .map_err(|e| e.to_string())?
                } else {
                    let mut spec = CurveSpec::new(&x, &y, color);
                    spec.line_width = 1.5 * self.line_scale;
                    spec.symbol_size = 7. * self.line_scale;
                    if raw_level && separate_raw {
                        spec.y_axis = YAxis::Right;
                    }
                    if chunk[0].time_segment == Some(1) {
                        spec.line_style = LineStyle::Dashed;
                    }
                    let handle = plot.add_curve_spec(spec);
                    if chunk.len() == 1 {
                        plot.set_curve_points_visible(handle, true);
                    }
                    handle
                };
                if part == 0 {
                    plot.set_item_legend(handle, &label);
                }
            }
            if let Some(reference) = &series.forecast_reference_time
                && let Ok(reference) = super::data::timestamp(reference)
            {
                plot.add_x_marker(seconds(reference) - self.origin, Color32::RED);
            }
        }
        for reference in &self.references {
            let label = reference_label(reference, portuguese);
            let handle = self.plots[2].add_y_marker(
                reference.level_cm,
                Color32::from_rgb(200, 70, 50),
                YAxis::Left,
            );
            self.plots[2].set_item_legend(handle, &label);
            self.labels.push((label, Color32::from_rgb(200, 70, 50)));
        }
        for plot in &mut self.plots {
            plot.reset_zoom();
            plot.set_graph_x_limits(0., seconds(report.window.end) - self.origin);
        }
        self.cursors = std::array::from_fn(|i| self.plots[i].add_x_marker(0., Color32::GRAY));
        self.report = Some(report);
        Ok(())
    }
    pub fn show(&mut self, ui: &mut egui::Ui, portuguese: bool, height: f32) {
        for plot in &mut self.plots {
            plot.set_background_colors(ui.visuals().panel_fill, ui.visuals().extreme_bg_color);
            plot.set_foreground_colors(
                ui.visuals().text_color(),
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            );
        }

        let Some(report) = &self.report else {
            return;
        };
        let source = report
            .series
            .iter()
            .find_map(|s| s.run_id.as_deref())
            .unwrap_or("OBS");
        ui.small(format!(
            "{} · {} · {source}",
            self.location_caption(report, portuguese),
            report.time_zone
        ));
        ui.horizontal_wrapped(|ui| {
            for (label, color) in &self.labels {
                ui.colored_label(*color, label);
            }
            ui.small(if self.portuguese {
                "— histórico · --- previsão · vermelho: referência"
            } else {
                "— history · --- forecast · red: reference"
            });
        });
        let mut limits = None;
        let mut cursor = None;
        for plot in &mut self.plots {
            plot.drain_events();
        }
        for plot in &mut self.plots {
            let response = ui
                .allocate_ui(egui::vec2(ui.available_width(), height), |ui| plot.show(ui))
                .inner;
            if response.response.hovered() {
                if let Some(pointer) = response.response.hover_pos() {
                    cursor = Some(response.transform.pixel_to_data(pointer).0);
                }
                let (a, b, _, _) = plot.plot().limits;
                limits = Some((a, b));
            }
        }
        if let Some((a, b)) = limits {
            for plot in &mut self.plots {
                plot.set_graph_x_limits(a, b);
            }
        }
        if let Some(x) = cursor {
            for (plot, handle) in self.plots.iter_mut().zip(self.cursors) {
                plot.set_marker_position(handle, x, 0.);
            }
            if let Some(report) = &self.report {
                let x = x + self.origin;
                let label = chrono::DateTime::from_timestamp(x as i64, 0)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default();
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{label} · {}", report.time_zone));
                    for series in &report.series {
                        if let Some(point) = series.points.iter().min_by(|a, b| {
                            (seconds(a.timestamp) - x)
                                .abs()
                                .total_cmp(&(seconds(b.timestamp) - x).abs())
                        }) {
                            let value = point
                                .value
                                .map(|v| format!("{v:.2}"))
                                .unwrap_or_else(|| "NA".into());
                            ui.colored_label(
                                source_color(&self.scenario_order, series.scenario_id.as_deref()),
                                format!(
                                    "{} {}: {} {} @ {}",
                                    series.scenario_id.as_deref().unwrap_or("OBS"),
                                    series.variable,
                                    value,
                                    series.unit,
                                    point.timestamp.format("%m-%d %H:%M:%S")
                                ),
                            );
                        }
                    }
                });
            }
        }
    }
}

impl Drop for Charts {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_colors_follow_the_run_order() {
        let scenario_order = ["zero", "ecmwf-raw", "noaa-raw", "ecmwf-corrected"].map(String::from);
        let raw = Color32::from_rgb(190, 85, 190);
        let corrected = Color32::from_rgb(235, 145, 35);
        assert_eq!(
            ["ecmwf-corrected", "ecmwf-raw", "ecmwf-corrected"]
                .map(|id| source_color(&scenario_order, Some(id))),
            [corrected, raw, corrected]
        );
        assert_eq!(
            source_color(&scenario_order, None),
            Color32::from_rgb(135, 145, 155)
        );
    }

    #[test]
    fn masks_and_segment_boundaries_break_lines() {
        let time = super::super::data::timestamp("2026-01-01T00:00:00").unwrap();
        let points: Vec<_> = [
            (Some(0.), Some(0)),
            (None, Some(0)),
            (Some(-3.), Some(0)),
            (Some(2.), Some(1)),
        ]
        .into_iter()
        .map(|(value, time_segment)| SeriesPoint {
            timestamp: time,
            value,
            time_segment,
        })
        .collect();
        let groups = chunks(&points);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0][0].value, Some(0.));
        assert_eq!(groups[1][0].value, Some(-3.));
    }
}
