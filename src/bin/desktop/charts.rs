use eframe::egui::{self, Color32};
use mgb_ops::app::{ChartSeries, Location, SeriesPoint, SeriesReport};
use mgb_ops::store::LevelReference;
use rsplot::{CurveSpec, LineStyle, Plot1D, PlotView, TickMode, Transform, YAxis};

pub struct Charts {
    pub state: eframe::egui_wgpu::RenderState,
    pub plots: [Plot1D; 3],
    cursor: Cursor,
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Cursor {
    point: Option<CursorPoint>,
    pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CursorPoint {
    plot: usize,
    time: f64,
    value: f64,
}

impl Cursor {
    fn update(&mut self, point: CursorPoint, clicked: bool) {
        if clicked {
            self.pinned = !self.pinned;
            self.point = Some(point);
        } else if !self.pinned {
            self.point = Some(point);
        }
    }
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
fn padded_y_bounds(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    let mut values = values.filter(|value| value.is_finite());
    let first = values.next()?;
    let (min, max) = values.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    let pad = if min == max {
        (min.abs() * 0.1).max(1.)
    } else {
        (max - min) * 0.1
    };
    Some((min - pad, max + pad))
}

impl Charts {
    pub fn new(state: &eframe::egui_wgpu::RenderState, base: u64) -> Self {
        let mut plots = std::array::from_fn(|i| Plot1D::new(state, base + i as u64));
        for (i, plot) in plots.iter_mut().enumerate() {
            plot.set_graph_y_label(["mm", "m³/s", "cm"][i], YAxis::Left);
            plot.plot_mut().set_x_tick_mode(TickMode::TimeSeries);
            plot.set_auto_reset_zoom(false);
        }
        Self {
            state: state.clone(),
            plots,
            cursor: Cursor::default(),
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
        self.cursor = Cursor::default();
        self.comparison_station = None;
        self.source_labels.clear();
        self.scenario_order.clear();
        self.references.clear();
        self.labels.clear();
    }

    pub fn relabel(&mut self, portuguese: bool) -> Result<(), String> {
        let Some(report) = self.report.clone() else {
            return Ok(());
        };
        let limits = self.plots.each_ref().map(|plot| plot.plot().limits);
        let right = self.plots[2].plot().y2;
        let references = self.references.clone();
        let cursor = self.cursor;
        self.load(report, &references, portuguese)?;
        for (plot, (a, b, c, d)) in self.plots.iter_mut().zip(limits) {
            plot.set_graph_x_limits(a, b);
            plot.set_graph_y_limits(c, d, YAxis::Left);
        }
        if let Some((min, max)) = right {
            self.plots[2].set_graph_y_limits(min, max, YAxis::Right);
        }
        self.cursor = cursor;
        Ok(())
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
        self.cursor = Cursor::default();
        self.portuguese = portuguese;
        self.origin = seconds(report.window.start);
        for plot in &mut self.plots {
            plot.plot_mut().set_x_time_offset(self.origin);
        }
        for (i, plot) in self.plots.iter_mut().enumerate() {
            plot.clear();
            plot.set_graph_title(plot_title(i, portuguese));
            if i == 2 {
                plot.set_graph_x_label(super::text(portuguese, "Local time", "Hora local"));
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
            let raw_level = series.series_kind == "model_raw_level";
            let label = series_label(series, &self.source_labels, portuguese);
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
        for (index, plot) in self.plots.iter_mut().enumerate() {
            plot.reset_zoom();
            plot.set_graph_x_limits(0., seconds(report.window.end) - self.origin);
            let variable = ["rain", "flow", "level"][index];
            let values = |right| {
                report
                    .series
                    .iter()
                    .filter(move |series| {
                        series.variable == variable
                            && (separate_raw && series.series_kind == "model_raw_level") == right
                    })
                    .flat_map(|series| series.points.iter().filter_map(|point| point.value))
            };
            let baseline = (index == 0 && values(false).next().is_some()).then_some(0.);
            let thresholds = self
                .references
                .iter()
                .filter(|_| index == 2)
                .map(|reference| reference.level_cm);
            if let Some((min, max)) =
                padded_y_bounds(values(false).chain(baseline).chain(thresholds))
            {
                plot.set_graph_y_limits(min, max, YAxis::Left);
            }
            if index == 2
                && separate_raw
                && let Some((min, max)) = padded_y_bounds(values(true))
            {
                plot.set_graph_y_limits(min, max, YAxis::Right);
            }
        }
        self.report = Some(report);
        Ok(())
    }
    pub fn show(&mut self, ui: &mut egui::Ui, portuguese: bool, available_height: f32) {
        let Some(report) = &self.report else {
            return;
        };
        for plot in &mut self.plots {
            plot.set_background_colors(ui.visuals().panel_fill, ui.visuals().extreme_bg_color);
            plot.set_foreground_colors(
                ui.visuals().text_color(),
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            );
        }

        let top = ui.cursor().min.y;
        ui.heading(self.location_caption(report, portuguese));
        ui.horizontal_wrapped(|ui| {
            if let Some(run) = report
                .series
                .iter()
                .find_map(|series| series.run_id.as_deref())
            {
                ui.weak(run);
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (label, color) in &self.labels {
                super::theme::color_key(ui, *color, label);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let ink = ui.visuals().text_color();
            legend_key(
                ui,
                ink,
                LegendMark::Solid,
                super::text(portuguese, "History", "Histórico"),
            );
            legend_key(
                ui,
                ink,
                LegendMark::Dashed,
                super::text(portuguese, "Forecast", "Previsão"),
            );
            if report
                .series
                .iter()
                .any(|series| series.forecast_reference_time.is_some())
            {
                legend_key(
                    ui,
                    Color32::RED,
                    LegendMark::Reference,
                    super::text(portuguese, "Forecast reference", "Referência da previsão"),
                );
            }
        });

        let header_height = ui.cursor().min.y - top;
        let gap = ui.spacing().item_spacing.y;
        let height = ((available_height - header_height - 3. * gap) / 3.).max(210.);
        let width = ui.available_width();
        let right_gutter = ui
            .painter()
            .layout_no_wrap(
                "2000-00-00 00:00:00".into(),
                egui::FontId::proportional(11.),
                Color32::WHITE,
            )
            .size()
            .x
            * 0.5;
        let mut responses = Vec::with_capacity(self.plots.len());
        let mut limits = None;
        let mut hovered = None;
        for (index, plot) in self.plots.iter_mut().enumerate() {
            plot.drain_events();
            let before = plot.plot().limits;
            let response = ui
                .allocate_ui(egui::vec2(width, height), |ui| {
                    if !ui.is_rect_visible(ui.max_rect()) {
                        ui.add_space(height);
                        return None;
                    }
                    // The plotting widget centers end tick labels. Leave room on the right
                    // for the overhang before laying out the plot.
                    plot.plot_mut().margins.right = if plot.plot().y2.is_some() {
                        0.
                    } else {
                        ((right_gutter - 12.) / (width - 80.).max(1.)).clamp(0., 0.8)
                    };
                    // The view already supplies the pointer transform and plot controls.
                    // Plot1D's extra item picking scans series on each pointer move.
                    let mode = plot.interaction_mode();
                    let backend = plot.backend_mut();
                    backend.flush_scene();
                    let response =
                        PlotView::new().show_with_interaction(ui, backend.plot_mut(), mode);
                    backend.set_plot_bounds_in_pixels(response.transform.area);
                    Some(response)
                })
                .inner;
            let Some(response) = response else { continue };
            let widget = &response.response;
            let after = plot.plot().limits;
            if before.0 != after.0 || before.1 != after.1 {
                limits = Some((after.0, after.1));
            }
            let clicked = widget.clicked_by(egui::PointerButton::Primary);
            let pointer = if clicked {
                widget.interact_pointer_pos()
            } else {
                widget.hover_pos()
            };
            if let Some(pointer) = pointer
                && response
                    .transform
                    .area
                    .intersect(ui.clip_rect())
                    .contains(pointer)
            {
                hovered = Some(index);
                let (time, value) = response.transform.pixel_to_data(pointer);
                self.cursor.update(
                    CursorPoint {
                        plot: index,
                        time: time + self.origin,
                        value,
                    },
                    clicked,
                );
            }
            responses.push((index, response.transform, response.response.rect));
        }
        if let Some((a, b)) = limits {
            for plot in &mut self.plots {
                let (current_a, current_b, _, _) = plot.plot().limits;
                if (current_a, current_b) != (a, b) {
                    plot.set_graph_x_limits(a, b);
                }
            }
            ui.request_repaint();
        }
        if let Some(point) = self.cursor.point {
            for (index, transform, rect) in responses {
                let crosshair =
                    index == point.plot && (self.cursor.pinned || hovered == Some(index));
                paint_cursor(ui, transform, rect, point, self.origin, crosshair);
            }
            self.cursor_readings(ui, report, point.time, portuguese);
        }
    }

    fn cursor_readings(
        &self,
        ui: &mut egui::Ui,
        report: &SeriesReport,
        cursor: f64,
        portuguese: bool,
    ) {
        let time = chrono::DateTime::from_timestamp(cursor as i64, 0)
            .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        super::theme::inset(ui.style()).show(ui, |ui| {
            ui.strong(super::text(
                portuguese,
                "Cursor readings",
                "Leituras do cursor",
            ));
            ui.weak(time);
            for (index, variable) in ["rain", "flow", "level"].into_iter().enumerate() {
                if !report
                    .series
                    .iter()
                    .any(|series| series.variable == variable)
                {
                    continue;
                }
                ui.add_space(4.);
                ui.strong(plot_title(index, portuguese));
                for series in report
                    .series
                    .iter()
                    .filter(|series| series.variable == variable)
                {
                    let Some(point) = nearest_reading(&series.points, cursor) else {
                        continue;
                    };
                    let value = point
                        .value
                        .map_or_else(|| "NA".into(), |value| format!("{value:.2}"));
                    ui.horizontal_wrapped(|ui| {
                        super::theme::color_key(
                            ui,
                            source_color(&self.scenario_order, series.scenario_id.as_deref()),
                            &series_label(series, &self.source_labels, portuguese),
                        );
                        ui.strong(format!("{value} {}", series.unit));
                        ui.weak(point.timestamp.format("%m-%d %H:%M:%S").to_string());
                    });
                }
            }
        });
    }
}

fn paint_cursor(
    ui: &egui::Ui,
    transform: Transform,
    rect: egui::Rect,
    point: CursorPoint,
    origin: f64,
    crosshair: bool,
) {
    let pos = transform.data_to_pixel(point.time - origin, point.value);
    let area = transform.area;
    if !area.x_range().contains(pos.x) {
        return;
    }
    let ink = ui.visuals().weak_text_color();
    let stroke = egui::Stroke::new(1., ink);
    let plot = ui.painter().with_clip_rect(area);
    plot.vline(pos.x, area.y_range(), stroke);
    if crosshair && area.contains(pos) {
        plot.hline(area.x_range(), pos.y, stroke);
        let time = chrono::DateTime::from_timestamp(point.time as i64, 0)
            .map(|time| time.format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        let painter = ui.painter().with_clip_rect(rect);
        let galley = painter.layout(
            format!("{time}\n{:.2}", point.value),
            egui::TextStyle::Small.resolve(ui.style()),
            ui.visuals().text_color(),
            (rect.width() - 8.).max(1.),
        );
        let pad = egui::vec2(4., 2.);
        let size = galley.size() + 2. * pad;
        let anchor = (pos + egui::vec2(8., 8.))
            .min(rect.max - size)
            .max(rect.min);
        painter.rect_filled(
            egui::Rect::from_min_size(anchor, size),
            3.,
            ui.visuals().extreme_bg_color,
        );
        painter.galley(anchor + pad, galley, ui.visuals().text_color());
    }
}

fn nearest_reading(points: &[SeriesPoint], time: f64) -> Option<&SeriesPoint> {
    let after = points.partition_point(|point| seconds(point.timestamp) < time);
    match (
        after.checked_sub(1).and_then(|index| points.get(index)),
        points.get(after),
    ) {
        (Some(before), Some(next))
            if time - seconds(before.timestamp) <= seconds(next.timestamp) - time =>
        {
            Some(before)
        }
        (before, next) => next.or(before),
    }
}

fn plot_title(index: usize, portuguese: bool) -> &'static str {
    if portuguese {
        ["Chuva", "Vazão", "Nível"][index]
    } else {
        ["Rainfall", "Flow", "Level"][index]
    }
}

fn series_label(series: &ChartSeries, sources: &[String], portuguese: bool) -> String {
    let name = series
        .scenario_id
        .clone()
        .unwrap_or_else(|| super::text(portuguese, "Observed", "Observado"));
    match series.series_kind.as_str() {
        "model_raw_level" => format!(
            "{name} · {}",
            super::text(portuguese, "raw MINI level", "nível bruto da MINI")
        ),
        "model_station_level" => format!(
            "{name} · {}",
            super::text(portuguese, "station datum", "datum da estação")
        ),
        _ if series.scenario_id.is_none() => sources
            .get(series.selection_index)
            .map_or(name.clone(), |source| format!("{name} · {source}")),
        _ => name,
    }
}

enum LegendMark {
    Solid,
    Dashed,
    Reference,
}

fn legend_key(ui: &mut egui::Ui, color: Color32, mark: LegendMark, label: String) {
    super::theme::legend_key(ui, egui::vec2(17., 12.), &label, |ui, rect| {
        let start = rect.left_center();
        let stop = rect.right_center();
        let stroke = egui::Stroke::new(1.5, color);
        match mark {
            LegendMark::Solid => {
                ui.painter().line_segment([start, stop], stroke);
            }
            LegendMark::Dashed => {
                for offset in [0., 6., 12.] {
                    let dash = start + egui::vec2(offset, 0.);
                    ui.painter()
                        .line_segment([dash, dash + egui::vec2(4., 0.)], stroke);
                }
            }
            LegendMark::Reference => {
                ui.painter()
                    .line_segment([rect.center_top(), rect.center_bottom()], stroke);
            }
        }
    });
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
    fn clicks_pin_and_release_the_crosshair() {
        let start = super::super::data::timestamp("2026-01-01T00:00:00").unwrap();
        let points = [Some(0.), None, Some(2.)]
            .into_iter()
            .enumerate()
            .map(|(index, value)| SeriesPoint {
                timestamp: start + chrono::Duration::hours(index as i64),
                value,
                time_segment: None,
            })
            .collect::<Vec<_>>();
        let mut cursor = Cursor::default();
        let at = |index: usize| CursorPoint {
            plot: index,
            time: seconds(points[index].timestamp),
            value: index as f64,
        };
        let reading = |cursor: Cursor| {
            nearest_reading(&points, cursor.point.unwrap().time)
                .unwrap()
                .value
        };
        cursor.update(at(0), false);
        assert_eq!(reading(cursor), Some(0.));
        cursor.update(at(1), true);
        cursor.update(at(2), false);
        let pinned = Some(at(1));
        assert_eq!(cursor.point, pinned);
        assert_eq!(reading(cursor), None);
        cursor.update(at(2), true);
        let released = !cursor.pinned;
        assert!(released);
        assert_eq!(reading(cursor), Some(2.));
        cursor.update(at(0), false);
        assert_eq!(reading(cursor), Some(0.));
    }

    #[test]
    fn default_y_bounds_pad_signed_and_constant_data() {
        assert_eq!(padded_y_bounds([-10., 20.].into_iter()), Some((-13., 23.)));
        assert_eq!(padded_y_bounds([0., 0.].into_iter()), Some((-1., 1.)));
        assert_eq!(padded_y_bounds([50.].into_iter()), Some((45., 55.)));
        assert!(padded_y_bounds(std::iter::empty()).is_none());
    }

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
