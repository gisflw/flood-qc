use super::data::{Project, Rain};
use eframe::egui::{self, Color32, Pos2, Stroke};
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers, math::point,
    path::Path,
};
use mgb_ops::app::Location;
use mgb_ops::store::{MapPolygon, MiniMapFeature};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use walkers::{MapMemory, Projector, lon_lat};

#[derive(Clone, Serialize, Deserialize)]
pub struct LayerControl {
    pub kind: String,
    pub visible: bool,
    pub opacity: f32,
}
pub fn default_layers() -> Vec<LayerControl> {
    [
        ("catchments", false, 0.25),
        ("rainfall", true, 0.65),
        ("upstream", true, 0.3),
        ("rivers", true, 0.85),
        ("stations", true, 1.0),
    ]
    .into_iter()
    .map(|(kind, visible, opacity)| LayerControl {
        kind: kind.into(),
        visible,
        opacity,
    })
    .collect()
}

pub struct Fill {
    pub vertices: Vec<(f64, f64)>,
    projected: Vec<(f64, f64)>,
    pub indices: Vec<u32>,
}
/// Tessellate relative coordinates so regional longitude does not consume f32 precision.
pub fn prepare_fill(polygons: &[MapPolygon]) -> Result<Fill, String> {
    let mut result = Fill {
        vertices: vec![],
        projected: vec![],
        indices: vec![],
    };
    for polygon in polygons {
        let origin = polygon.exterior[0];
        let mut builder = Path::builder();
        for ring in std::iter::once(&polygon.exterior).chain(&polygon.interiors) {
            builder.begin(point(
                (ring[0].0 - origin.0) as f32,
                (ring[0].1 - origin.1) as f32,
            ));
            for &(x, y) in &ring[1..] {
                builder.line_to(point((x - origin.0) as f32, (y - origin.1) as f32));
            }
            builder.end(true);
        }
        let mut mesh: VertexBuffers<(f64, f64), u32> = VertexBuffers::new();
        FillTessellator::new()
            .tessellate_path(
                &builder.build(),
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut mesh, |v: FillVertex<'_>| {
                    let p = v.position();
                    (origin.0 + f64::from(p.x), origin.1 + f64::from(p.y))
                }),
            )
            .map_err(|e| e.to_string())?;
        let base = result.vertices.len() as u32;
        result.vertices.extend(mesh.vertices);
        result
            .indices
            .extend(mesh.indices.into_iter().map(|i| base + i));
    }
    result.projected = result.vertices.iter().copied().map(world).collect();
    Ok(result)
}

pub struct PreparedMini {
    pub mini_id: i64,
    pub fill: Fill,
    pub bounds: [f64; 4],
    boundary: Vec<Vec<(f64, f64)>>,
}
pub fn prepare_catchments(features: &[MiniMapFeature]) -> Result<Vec<PreparedMini>, String> {
    features
        .iter()
        .map(|feature| {
            Ok(PreparedMini {
                mini_id: feature.mini_id,
                fill: prepare_fill(&feature.polygons)?,
                boundary: project_paths(&feature.paths),
                bounds: feature.paths.iter().flatten().fold(
                    [
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ],
                    |b, &(x, y)| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                ),
            })
        })
        .collect()
}

pub struct RiverPaths {
    pub mini_id: i64,
    pub paths: Vec<Vec<(f64, f64)>>,
}
pub fn river_segments(features: &[MiniMapFeature]) -> Vec<RiverPaths> {
    features
        .iter()
        .map(|feature| RiverPaths {
            mini_id: feature.mini_id,
            paths: project_paths(&feature.paths),
        })
        .collect()
}
pub fn overview_segments(features: &[MiniMapFeature]) -> Vec<RiverPaths> {
    features
        .iter()
        .map(|feature| RiverPaths {
            mini_id: feature.mini_id,
            paths: feature
                .paths
                .iter()
                .map(|path| simplify(path, 0.00015).into_iter().map(world).collect())
                .collect(),
        })
        .collect()
}
pub fn project_paths(paths: &[Vec<(f64, f64)>]) -> Vec<Vec<(f64, f64)>> {
    paths
        .iter()
        .map(|path| path.iter().copied().map(world).collect())
        .collect()
}
// Display-only Web Mercator; authoritative WGS84 coordinates remain in store values.
fn world((lon, lat): (f64, f64)) -> (f64, f64) {
    (
        (1. + lon / 180.) / 2.,
        (1. - lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.,
    )
}
struct ScreenTransform {
    center: (f64, f64),
    scale: f64,
    origin: Pos2,
}
impl ScreenTransform {
    fn new(projector: &Projector, rect: egui::Rect, zoom: f64) -> Self {
        let center = projector.unproject(rect.center().to_vec2());
        Self {
            center: world((center.x(), center.y())),
            scale: 256. * 2f64.powf(zoom),
            origin: rect.center(),
        }
    }
    fn project(&self, point: (f64, f64)) -> Pos2 {
        self.origin
            + egui::vec2(
                ((point.0 - self.center.0) * self.scale) as f32,
                ((point.1 - self.center.1) * self.scale) as f32,
            )
    }
}
fn simplify(path: &[(f64, f64)], tolerance: f64) -> Vec<(f64, f64)> {
    if path.len() <= 2 {
        return path.to_vec();
    }
    let mut keep = vec![false; path.len()];
    keep[0] = true;
    keep[path.len() - 1] = true;
    let mut pending = vec![(0, path.len() - 1)];
    while let Some((first, last)) = pending.pop() {
        let a = path[first];
        let b = path[last];
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let length = dx * dx + dy * dy;
        let mut farthest = None;
        let mut maximum = tolerance * tolerance;
        for (index, p) in path.iter().enumerate().take(last).skip(first + 1) {
            let t = if length == 0. {
                0.
            } else {
                ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length
            }
            .clamp(0., 1.);
            let distance = (p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2);
            if distance > maximum {
                maximum = distance;
                farthest = Some(index);
            }
        }
        if let Some(index) = farthest {
            keep[index] = true;
            pending.push((first, index));
            pending.push((index, last));
        }
    }
    path.iter()
        .zip(keep)
        .filter_map(|(p, keep)| keep.then_some(*p))
        .collect()
}

fn color(kind: &str) -> Color32 {
    match kind {
        "catchments" => Color32::from_rgb(105, 150, 110),
        "upstream" => Color32::from_rgb(255, 185, 30),
        "rivers" => Color32::from_rgb(58, 136, 180),
        _ => Color32::LIGHT_GREEN,
    }
}
pub fn rain_color(value: Option<f64>) -> Color32 {
    match value {
        None => Color32::TRANSPARENT,
        Some(value) if value < 1.0 => Color32::from_rgb(210, 235, 250),
        Some(value) if value < 10.0 => Color32::from_rgb(80, 190, 240),
        Some(value) if value < 25.0 => Color32::from_rgb(40, 185, 95),
        Some(value) if value < 50.0 => Color32::from_rgb(240, 220, 40),
        Some(value) if value < 100.0 => Color32::from_rgb(245, 135, 35),
        Some(_) => Color32::from_rgb(190, 40, 130),
    }
}
pub fn layer_swatch(kind: &str) -> Color32 {
    if kind == "rainfall" {
        rain_color(Some(10.))
    } else {
        color(kind)
    }
}

pub fn rain_legend(ui: &mut egui::Ui, pt: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.strong(super::text(
            pt,
            "Accumulated rainfall (mm)",
            "Chuva acumulada (mm)",
        ));
        for (value, label) in [
            (Some(0.), "0–<1"),
            (Some(1.), "1–<10"),
            (Some(10.), "10–<25"),
            (Some(25.), "25–<50"),
            (Some(50.), "50–<100"),
            (Some(100.), "≥100"),
            (None, "NA"),
        ] {
            let response = super::theme::color_key(ui, rain_color(value), label);
            if value.is_none() {
                response.on_hover_text(super::text(
                    pt,
                    "Incomplete coverage",
                    "Cobertura incompleta",
                ));
            }
        }
    });
}

fn screen(projector: &Projector, p: (f64, f64)) -> Pos2 {
    projector.project(lon_lat(p.0, p.1)).to_pos2()
}
fn map_label(ui: &egui::Ui, position: Pos2, anchor: egui::Align2, text: String, opacity: f32) {
    let color = Color32::BLACK.gamma_multiply(opacity);
    let galley = ui
        .painter()
        .layout_no_wrap(text, egui::FontId::proportional(11.), color);
    let rect = anchor.anchor_rect(egui::Rect::from_min_size(position, galley.size()));
    ui.painter().rect_filled(
        rect.expand(2.),
        egui::CornerRadius::same(2),
        Color32::WHITE.gamma_multiply(0.9 * opacity),
    );
    ui.painter().galley(rect.min, galley, color);
}
fn mesh(fill: &Fill, transform: &ScreenTransform, color: Color32) -> egui::Mesh {
    egui::Mesh {
        indices: fill.indices.clone(),
        vertices: fill
            .projected
            .iter()
            .map(|p| egui::epaint::Vertex {
                pos: transform.project(*p),
                uv: egui::epaint::WHITE_UV,
                color,
            })
            .collect(),
        ..Default::default()
    }
}

pub struct RainImage {
    pub texture: egui::TextureHandle,
    pub rain: Arc<Rain>,
}
impl RainImage {
    pub fn new(context: &egui::Context, rain: Arc<Rain>) -> Self {
        let image = egui::ColorImage::new(
            [rain.grid.longitudes.len(), rain.grid.latitudes.len()],
            rain.values.iter().map(|v| rain_color(*v)).collect(),
        );
        let texture =
            context.load_texture("accumulated-rainfall", image, egui::TextureOptions::NEAREST);
        Self { texture, rain }
    }
    fn draw(&self, ui: &egui::Ui, projector: &Projector, opacity: f32) {
        ui.painter().add(egui::Shape::mesh(rain_mesh(
            &self.rain,
            self.texture.id(),
            projector,
            opacity,
        )));
    }
    fn inspect(&self, geographic: (f64, f64)) -> Option<(usize, usize, Option<f64>)> {
        cell_at(&self.rain, geographic)
    }
}
fn rain_mesh(
    rain: &Rain,
    texture: egui::TextureId,
    projector: &Projector,
    opacity: f32,
) -> egui::Mesh {
    let grid = &rain.grid;
    let height = grid.latitudes.len();
    let west = grid.effective_bbox[0];
    let east = grid.effective_bbox[2];
    // One projected strip per latitude row preserves Mercator cell edges and orientation.
    let mut mesh = egui::Mesh::with_texture(texture);
    for row in 0..height {
        let south = grid.latitudes[row] - grid.resolution_degrees / 2.0;
        let north = south + grid.resolution_degrees;
        let base = mesh.vertices.len() as u32;
        for (p, uv) in [
            ((west, south), (0., row as f32 / height as f32)),
            ((east, south), (1., row as f32 / height as f32)),
            ((east, north), (1., (row + 1) as f32 / height as f32)),
            ((west, north), (0., (row + 1) as f32 / height as f32)),
        ] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: screen(projector, p),
                uv: egui::pos2(uv.0, uv.1),
                color: Color32::WHITE.gamma_multiply(opacity),
            });
        }
        mesh.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh
}

pub(super) fn cell_at(rain: &Rain, (lon, lat): (f64, f64)) -> Option<(usize, usize, Option<f64>)> {
    let grid = &rain.grid;
    let col = ((lon - grid.effective_bbox[0]) / grid.resolution_degrees).floor();
    let row = ((lat - grid.effective_bbox[1]) / grid.resolution_degrees).floor();
    if col < 0.
        || row < 0.
        || col >= grid.longitudes.len() as f64
        || row >= grid.latitudes.len() as f64
    {
        return None;
    }
    let (col, row) = (col as usize, row as usize);
    Some((col, row, rain.values[row * grid.longitudes.len() + col]))
}

struct RiverHit {
    mini_id: i64,
    points: Vec<Pos2>,
    bounds: egui::Rect,
}
type CameraKey = (egui::Rect, f64, f64, f64, usize, f32);
struct BasinOverlay {
    mini_ids: Vec<i64>,
    opacity: f32,
    fill: Arc<egui::Mesh>,
    outline: Vec<Arc<egui::Mesh>>,
}
fn line_meshes(
    context: &egui::Context,
    clip: egui::Rect,
    shapes: Vec<egui::Shape>,
) -> Vec<Arc<egui::Mesh>> {
    context
        .tessellate(
            shapes
                .into_iter()
                .map(|shape| egui::epaint::ClippedShape {
                    clip_rect: clip,
                    shape,
                })
                .collect(),
            context.pixels_per_point(),
        )
        .into_iter()
        .filter_map(|primitive| match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => Some(Arc::new(mesh)),
            egui::epaint::Primitive::Callback(_) => None,
        })
        .collect()
}
#[derive(Default)]
pub struct StaticCache {
    key: Option<CameraKey>,
    rivers: Vec<Arc<egui::Mesh>>,
    tinted_rivers: Option<(f32, Vec<Arc<egui::Mesh>>)>,
    basin: Option<BasinOverlay>,
    hits: Vec<RiverHit>,
    catchments: Option<Arc<egui::Mesh>>,
    tinted_catchments: Option<(f32, Arc<egui::Mesh>)>,
}
impl StaticCache {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    fn update(
        &mut self,
        project: &Project,
        projector: &Projector,
        rect: egui::Rect,
        memory: &MapMemory,
        context: &egui::Context,
    ) {
        let transform = ScreenTransform::new(projector, rect, memory.zoom());
        let center = projector.unproject(rect.center().to_vec2());
        let key = (
            rect,
            center.x(),
            center.y(),
            memory.zoom(),
            Arc::as_ptr(&project.fills) as usize,
            context.pixels_per_point(),
        );
        if self.key.as_ref() == Some(&key) {
            return;
        }
        // shortcut: camera changes transform prepared paths on the UI thread; retain GPU paths if operator pan/zoom latency fails its budget.
        self.key = Some(key);
        self.tinted_catchments = None;
        self.tinted_rivers = None;
        self.basin = None;
        self.hits = if memory.zoom() < 12. {
            &project.overview_segments
        } else {
            &project.segments
        }
        .iter()
        .flat_map(|f| f.paths.iter().map(move |path| (f.mini_id, path)))
        .filter_map(|(mini_id, path)| {
            let points: Vec<_> = path.iter().map(|p| transform.project(*p)).collect();
            let bounds = egui::Rect::from_points(&points);
            bounds.intersects(rect).then_some(RiverHit {
                mini_id,
                points,
                bounds,
            })
        })
        .collect();
        let river_width = if memory.zoom() < 10. { 0.8 } else { 1.2 };
        self.rivers = line_meshes(
            context,
            rect,
            self.hits
                .iter()
                .map(|hit| {
                    egui::Shape::line(
                        hit.points.clone(),
                        Stroke::new(river_width, color("rivers")),
                    )
                })
                .collect(),
        );
        let mut combined = egui::Mesh::default();
        for feature in project.fills.iter() {
            combined.append(mesh(&feature.fill, &transform, color("catchments")));
        }
        self.catchments = Some(Arc::new(combined));
    }
    fn basin(
        &mut self,
        project: &Project,
        mini_ids: &[i64],
        opacity: f32,
        projector: &Projector,
        context: &egui::Context,
        clip: egui::Rect,
    ) {
        if self
            .basin
            .as_ref()
            .is_some_and(|cached| cached.mini_ids == mini_ids && cached.opacity == opacity)
        {
            return;
        }
        let transform =
            ScreenTransform::new(projector, clip, self.key.expect("map view prepared").3);
        let selected: HashSet<_> = mini_ids.iter().copied().collect();
        let tint = color("upstream").gamma_multiply(opacity);
        let mut fill = egui::Mesh::default();
        for feature in project
            .fills
            .iter()
            .filter(|f| selected.contains(&f.mini_id))
        {
            fill.append(mesh(&feature.fill, &transform, tint));
        }
        let outline = project
            .fills
            .iter()
            .filter(|f| selected.contains(&f.mini_id))
            .flat_map(|f| &f.boundary)
            .map(|path| {
                egui::Shape::line(
                    path.iter().map(|p| transform.project(*p)).collect(),
                    Stroke::new(1., tint),
                )
            })
            .collect();
        self.basin = Some(BasinOverlay {
            mini_ids: mini_ids.to_vec(),
            opacity,
            fill: Arc::new(fill),
            outline: line_meshes(context, clip, outline),
        });
    }
}
pub struct LocalLayer {
    pub path: std::path::PathBuf,
    pub name: String,
    pub projected_paths: Vec<Vec<(f64, f64)>>,
    pub fill: Fill,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct LocalDisplay {
    pub path: std::path::PathBuf,
    pub visible: bool,
    pub opacity: f32,
    pub color: [u8; 4],
}

pub struct Overlay<'a> {
    pub project: &'a Project,
    pub layers: &'a [LayerControl],
    pub rainfall: Option<&'a RainImage>,
    pub selection: &'a Option<Location>,
    pub upstream: &'a [i64],
    pub hit: &'a mut Option<Location>,
    pub cache: &'a mut StaticCache,
    pub labels: bool,
    pub local: &'a [LocalLayer],
    pub local_display: &'a [LocalDisplay],
}
impl walkers::Plugin for Overlay<'_> {
    fn run(
        self: Box<Self>,
        ui: &mut egui::Ui,
        response: &egui::Response,
        projector: &Projector,
        memory: &MapMemory,
    ) {
        self.cache
            .update(self.project, projector, response.rect, memory, ui.ctx());
        let click = response.clicked() || response.secondary_clicked();
        let hover = response
            .interact_pointer_pos()
            .filter(|_| click)
            .or_else(|| response.hover_pos());
        let alt = ui.input(|i| i.modifiers.alt);
        let visible = |kind| {
            self.layers
                .iter()
                .any(|layer| layer.kind == kind && layer.visible && layer.opacity > 0.)
        };
        let mut hit = None;
        let mut hit_label = String::new();
        // Station hit targets win over all polygon and line hits.
        for layer in self.layers.iter().filter(|l| l.visible) {
            match layer.kind.as_str() {
                "rainfall" => {
                    if let Some(rain) = self.rainfall {
                        rain.draw(ui, projector, layer.opacity);
                        if self.labels && memory.zoom() >= 10. {
                            let grid = &rain.rain.grid;
                            let step = grid.resolution_degrees;
                            let distance =
                                screen(projector, (grid.longitudes[0] + step, grid.latitudes[0]))
                                    .distance(screen(
                                        projector,
                                        (grid.longitudes[0], grid.latitudes[0]),
                                    ));
                            if distance >= 40. {
                                for (row, &lat) in grid.latitudes.iter().enumerate() {
                                    for (col, &lon) in grid.longitudes.iter().enumerate() {
                                        let p = screen(projector, (lon, lat));
                                        if response.rect.contains(p)
                                            && let Some(value) =
                                                rain.rain.values[row * grid.longitudes.len() + col]
                                        {
                                            map_label(
                                                ui,
                                                p + egui::vec2(3., 3.),
                                                egui::Align2::LEFT_TOP,
                                                format!("{value:.1}"),
                                                layer.opacity,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "catchments" => {
                    if let Some(mesh) = &self.cache.catchments {
                        if self
                            .cache
                            .tinted_catchments
                            .as_ref()
                            .is_none_or(|(opacity, _)| *opacity != layer.opacity)
                        {
                            let mut tinted = (**mesh).clone();
                            for vertex in &mut tinted.vertices {
                                vertex.color = vertex.color.gamma_multiply(layer.opacity);
                            }
                            self.cache.tinted_catchments = Some((layer.opacity, Arc::new(tinted)));
                        }
                        if let Some((_, mesh)) = &self.cache.tinted_catchments {
                            ui.painter().add(egui::Shape::Mesh(mesh.clone()));
                        }
                    }
                }
                "upstream" => {
                    if self.upstream.is_empty() {
                        self.cache.basin = None;
                    } else {
                        self.cache.basin(
                            self.project,
                            self.upstream,
                            layer.opacity,
                            projector,
                            ui.ctx(),
                            response.rect,
                        );
                        if let Some(basin) = &self.cache.basin {
                            ui.painter().add(egui::Shape::Mesh(basin.fill.clone()));
                            for mesh in &basin.outline {
                                ui.painter().add(egui::Shape::Mesh(mesh.clone()));
                            }
                        }
                    }
                }
                "rivers" => {
                    if self
                        .cache
                        .tinted_rivers
                        .as_ref()
                        .is_none_or(|(opacity, _)| *opacity != layer.opacity)
                    {
                        let meshes = self
                            .cache
                            .rivers
                            .iter()
                            .map(|mesh| {
                                let mut tinted = (**mesh).clone();
                                for vertex in &mut tinted.vertices {
                                    vertex.color = vertex.color.gamma_multiply(layer.opacity);
                                }
                                Arc::new(tinted)
                            })
                            .collect();
                        self.cache.tinted_rivers = Some((layer.opacity, meshes));
                    }
                    if let Some((_, meshes)) = &self.cache.tinted_rivers {
                        for mesh in meshes {
                            ui.painter().add(egui::Shape::Mesh(mesh.clone()));
                        }
                    }
                }
                "stations" => {
                    for entry in &self.project.stations {
                        let station = &entry.station;
                        if let Some((lon, lat)) = station.longitude.zip(station.latitude) {
                            let p = screen(projector, (lon, lat));
                            if !response.rect.expand(10.).contains(p) {
                                continue;
                            }
                            let selected = matches!(self.selection, Some(Location::Station(id)) if *id == station.station_id);
                            if selected {
                                ui.painter().circle_stroke(
                                    p,
                                    8.,
                                    Stroke::new(2., Color32::WHITE.gamma_multiply(layer.opacity)),
                                );
                            }
                            ui.painter().circle(
                                p,
                                if selected {
                                    6.
                                } else if memory.zoom() < 10. {
                                    3.
                                } else {
                                    4.5
                                },
                                if selected {
                                    Color32::YELLOW
                                } else if entry.offset.is_some() {
                                    Color32::LIGHT_GREEN
                                } else {
                                    Color32::GRAY
                                }
                                .gamma_multiply(layer.opacity),
                                Stroke::new(1., Color32::BLACK.gamma_multiply(layer.opacity)),
                            );
                            if self.labels && memory.zoom() >= 10.0 {
                                map_label(
                                    ui,
                                    p + egui::vec2(7., -7.),
                                    egui::Align2::LEFT_BOTTOM,
                                    station.name.clone(),
                                    layer.opacity,
                                );
                            }
                            if layer.opacity > 0.
                                && hover.is_some_and(|pointer| pointer.distance(p) <= 10.)
                            {
                                hit = Some(Location::Station(station.station_id.clone()));
                                hit_label = format!("{} · {}", station.name, station.station_id);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let transform = ScreenTransform::new(projector, response.rect, memory.zoom());
        for style in self.local_display.iter().filter(|style| style.visible) {
            let tint = Color32::from_rgba_unmultiplied(
                style.color[0],
                style.color[1],
                style.color[2],
                style.color[3],
            )
            .gamma_multiply(style.opacity);
            for layer in self.local.iter().filter(|layer| layer.path == style.path) {
                ui.painter().add(egui::Shape::mesh(mesh(
                    &layer.fill,
                    &transform,
                    tint.gamma_multiply(0.25),
                )));
                for path in &layer.projected_paths {
                    ui.painter().add(egui::Shape::line(
                        path.iter().map(|p| transform.project(*p)).collect(),
                        Stroke::new(1., tint),
                    ));
                }
            }
        }
        if let Some(Location::Mini(id)) = self.selection {
            if visible("catchments") {
                for feature in self.project.catchments.iter().filter(|f| f.mini_id == *id) {
                    for path in &feature.paths {
                        selected_line(ui, path.iter().map(|p| screen(projector, *p)).collect());
                    }
                }
            }
            if visible("rivers") {
                for feature in self.cache.hits.iter().filter(|f| f.mini_id == *id) {
                    selected_line(ui, feature.points.clone());
                }
            }
        }
        if let Some(pointer) = hover {
            let geographic = projector.unproject(pointer.to_vec2());
            if hit.is_none() && !alt {
                if visible("rivers") {
                    hit = self
                        .cache
                        .hits
                        .iter()
                        .find(|hit| {
                            hit.bounds.expand(6.).contains(pointer)
                                && hit
                                    .points
                                    .windows(2)
                                    .any(|p| distance_to_segment(pointer, p[0], p[1]) <= 6.)
                        })
                        .map(|hit| Location::Mini(hit.mini_id));
                }
                if hit.is_none() && visible("catchments") {
                    let p = (geographic.x(), geographic.y());
                    hit = self
                        .project
                        .fills
                        .iter()
                        .filter(|f| {
                            p.0 >= f.bounds[0]
                                && p.0 <= f.bounds[2]
                                && p.1 >= f.bounds[1]
                                && p.1 <= f.bounds[3]
                        })
                        .find_map(|f| {
                            let index = self
                                .project
                                .catchments
                                .binary_search_by_key(&f.mini_id, |f| f.mini_id)
                                .ok()?;
                            self.project.catchments[index]
                                .polygons
                                .iter()
                                .any(|polygon| contains(polygon, p))
                                .then_some(Location::Mini(f.mini_id))
                        });
                }
                if let Some(Location::Mini(id)) = &hit {
                    hit_label = format!("MINI {id}");
                }
            }
            if visible("rivers")
                && let Some(Location::Mini(id)) = &hit
                && !matches!(self.selection, Some(Location::Mini(selected)) if selected == id)
            {
                for feature in self.cache.hits.iter().filter(|f| f.mini_id == *id) {
                    ui.painter().add(egui::Shape::line(
                        feature.points.clone(),
                        Stroke::new(2., Color32::WHITE),
                    ));
                }
            }
            if !hit_label.is_empty() {
                response.clone().on_hover_text(hit_label);
            }
            if let Some(rainfall) = self.rainfall
                && let Some((col, row, value)) = rainfall.inspect((geographic.x(), geographic.y()))
            {
                let label = value
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "NA".into());
                response
                    .clone()
                    .on_hover_text(format!("[{col}, {row}] · {label} mm"));
            }
        }
        if click && !alt {
            *self.hit = hit;
        }
    }
}
fn selected_line(ui: &egui::Ui, points: Vec<Pos2>) {
    ui.painter().add(egui::Shape::line(
        points.clone(),
        Stroke::new(4., Color32::from_rgb(37, 48, 57)),
    ));
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(2., Color32::YELLOW)));
}

pub fn feature_menu(
    response: &egui::Response,
    location: Option<Location>,
    content: impl FnOnce(&mut egui::Ui, &Location),
) {
    let Some(location) = location else {
        egui::Popup::close_id(&response.ctx, egui::Popup::default_response_id(response));
        return;
    };
    let menu = response.context_menu(|ui| content(ui, &location));
    if response.secondary_clicked() && menu.is_some() {
        // The popup's first pass only measures it. Paint it before waiting for input again.
        response.ctx.request_discard("Measure feature context menu");
        response.ctx.request_repaint();
    }
}

fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let length = ab.length_sq();
    if length == 0. {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / length).clamp(0., 1.);
    p.distance(a + ab * t)
}
fn inside(ring: &[(f64, f64)], (x, y): (f64, f64)) -> bool {
    let mut inside = false;
    for edge in ring.windows(2) {
        let ((ax, ay), (bx, by)) = (edge[0], edge[1]);
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            inside = !inside;
        }
    }
    inside
}
fn contains(polygon: &MapPolygon, p: (f64, f64)) -> bool {
    inside(&polygon.exterior, p) && !polygon.interiors.iter().any(|r| inside(r, p))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secondary_click_paints_feature_menu_without_pointer_movement() {
        let ctx = egui::Context::default();
        let mut memory = MapMemory::default();
        let mut target = None;
        let mut frame = |events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        egui::vec2(500., 500.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let response = walkers::Map::new(None, &mut memory, lon_lat(0., 0.))
                        .show(ui, |_, _, _, _| {})
                        .response;
                    if response.secondary_clicked() {
                        target = Some(Location::Mini(7));
                    }
                    feature_menu(&response, target.clone(), |ui, location| {
                        let target_is_mini = matches!(location, Location::Mini(7));
                        assert!(target_is_mini);
                        ui.label("View time series");
                    });
                },
            );
            output.textures_delta.clear();
            output
        };
        let pos = egui::pos2(200., 200.);
        let click = |pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Secondary,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = frame(vec![egui::Event::PointerMoved(pos)]);
        let _ = frame(vec![click(true)]);
        let release = frame(vec![click(false)]);
        let painted = release.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "View time series")
        });
        assert!(painted);
    }

    #[test]
    fn prepared_projection_matches_map_at_overview_and_close_zoom() {
        let rect = egui::Rect::from_min_size(egui::pos2(283., 71.), egui::vec2(900., 500.));
        for zoom in [7., 12., 18.] {
            let mut memory = MapMemory::default();
            memory.set_zoom(zoom).unwrap();
            memory.center_at(lon_lat(-52., -30.));
            let projector = Projector::new(rect, &memory, lon_lat(-52., -30.));
            let transform = ScreenTransform::new(&projector, rect, zoom);
            for point in [(-52., -30.), (-51.99, -30.01), (-52.001, -30.001)] {
                assert!(
                    transform
                        .project(world(point))
                        .distance(screen(&projector, point))
                        < 0.01
                );
            }
        }
    }
    #[test]
    fn rainfall_rows_follow_projected_cell_edges_and_original_values() {
        let start = super::super::data::timestamp("2026-01-01T00:00:00").unwrap();
        let rain = Rain {
            generation: "generation-test".into(),
            scenario: "zero".into(),
            window: mgb_ops::store::TimeInterval {
                start,
                end: start + chrono::Duration::hours(1),
            },
            grid: mgb_ops::store::RegularGridSpec {
                requested_bbox: [-20., 10., 20., 50.],
                effective_bbox: [-20., 10., 20., 50.],
                resolution_degrees: 20.,
                latitudes: vec![20., 40.],
                longitudes: vec![-10., 10.],
            },
            values: vec![Some(0.), Some(2.), None, Some(4.)],
        };
        rain.grid.validate().unwrap();
        let mut memory = MapMemory::default();
        memory.set_zoom(4.).unwrap();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1000., 1000.));
        let projector = Projector::new(rect, &memory, lon_lat(0., 30.));
        let mesh = rain_mesh(&rain, egui::TextureId::Managed(0), &projector, 0.5);
        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.vertices[2].pos, screen(&projector, (20., 30.)));
        assert_eq!(mesh.vertices[4].pos, screen(&projector, (-20., 30.)));
        assert_eq!(mesh.vertices[0].uv, egui::pos2(0., 0.));
        assert_eq!(mesh.vertices[6].uv, egui::pos2(1., 1.));
        let stretched = (screen(&projector, (20., 10.)).y + screen(&projector, (20., 50.)).y) / 2.;
        assert!((mesh.vertices[2].pos.y - stretched).abs() > 10.);
        assert_eq!(cell_at(&rain, (-10., 20.)).unwrap().2, Some(0.));
        assert_eq!(cell_at(&rain, (-10., 40.)).unwrap().2, None);
        assert_eq!(rain_color(None), Color32::TRANSPARENT);
        assert_ne!(rain_color(Some(0.)), Color32::TRANSPARENT);
    }
    #[test]
    fn selection_meshes_are_reused_until_selection_or_camera_changes() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("workspace.toml");
        std::fs::write(&config, "[spatial]\ngpkg_path = \"missing.gpkg\"\n").unwrap();
        let mut project = super::super::data::open(config, dir.path().into(), None).unwrap();
        let ring = vec![(0., 0.), (1., 0.), (1., 1.), (0., 1.), (0., 0.)];
        let polygon = MapPolygon {
            exterior: ring.clone(),
            interiors: vec![],
        };
        let feature = MiniMapFeature {
            mini_id: 1,
            paths: vec![ring],
            polygons: vec![polygon],
        };
        let project_mut = Arc::get_mut(&mut project).unwrap();
        project_mut.fills = Arc::new(prepare_catchments(std::slice::from_ref(&feature)).unwrap());
        project_mut.catchments = Arc::new(vec![feature]);
        let context = egui::Context::default();
        let mut memory = MapMemory::default();
        memory.set_zoom(7.).unwrap();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(500., 500.));
        let mut cache = StaticCache::default();
        let mut previous = None;
        for _ in 0..2 {
            let mut frame = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    ui.label("cache fixture");
                    let projector = Projector::new(rect, &memory, lon_lat(0.5, 0.5));
                    cache.update(&project, &projector, rect, &memory, ui.ctx());
                    cache.basin(&project, &[1], 0.3, &projector, ui.ctx(), rect);
                },
            );
            frame.textures_delta.clear();
            let mesh = cache.basin.as_ref().unwrap().fill.clone();
            let outlined = cache
                .basin
                .as_ref()
                .unwrap()
                .outline
                .iter()
                .any(|m| !m.indices.is_empty());
            assert!(outlined);
            if let Some(previous) = previous {
                assert!(Arc::ptr_eq(&previous, &mesh));
            }
            previous = Some(mesh);
        }
        cache.clear();
        assert!(cache.basin.is_none());
    }
    #[test]
    fn display_simplification_preserves_endpoints_and_bends() {
        let points = [(0., 0.), (0.5, 0.), (1., 0.), (1., 1.)];
        assert_eq!(simplify(&points, 0.01), [(0., 0.), (1., 0.), (1., 1.)]);
        assert_eq!(points[1], (0.5, 0.));
    }
    #[test]
    fn holes_are_not_selectable_or_filled() {
        let hole = vec![(1., 1.), (3., 1.), (3., 3.), (1., 3.), (1., 1.)];
        let polygon = MapPolygon {
            exterior: vec![(0., 0.), (4., 0.), (4., 4.), (0., 4.), (0., 0.)],
            interiors: vec![hole],
        };
        assert!(contains(&polygon, (0.5, 0.5)));
        let hole_is_empty = !contains(&polygon, (2., 2.));
        assert!(hole_is_empty);
        let fill = prepare_fill(&[polygon]).unwrap();
        let area: f64 = fill
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let (a, b, c) = (
                    fill.vertices[t[0] as usize],
                    fill.vertices[t[1] as usize],
                    fill.vertices[t[2] as usize],
                );
                ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs() / 2.
            })
            .sum();
        assert!((area - 12.).abs() < 1e-6);
    }
    #[test]
    fn screen_tolerance_includes_zero_length_segments() {
        assert_eq!(
            distance_to_segment(egui::pos2(3., 4.), Pos2::ZERO, Pos2::ZERO),
            5.
        );
        assert_eq!(
            distance_to_segment(egui::pos2(2., 3.), Pos2::ZERO, egui::pos2(4., 0.)),
            3.
        );
    }
}
