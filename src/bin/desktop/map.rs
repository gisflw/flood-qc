use super::data::{Project, Rain, RainSide};
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
use walkers::{Map, MapMemory, Projector, Tiles, lon_lat};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Single,
    Double,
    Swipe,
}

impl Mode {
    pub fn comparing(self) -> bool {
        self != Self::Single
    }
}

pub fn side_label(side: RainSide, pt: bool) -> String {
    match side {
        RainSide::Left => super::text(pt, "Left", "Esquerda"),
        RainSide::Right => super::text(pt, "Right", "Direita"),
    }
}

fn split_rect(rect: egui::Rect, fraction: f32) -> [egui::Rect; 2] {
    let x = egui::lerp(rect.x_range(), fraction);
    [
        egui::Rect::from_min_max(rect.min, egui::pos2(x, rect.bottom())),
        egui::Rect::from_min_max(egui::pos2(x, rect.top()), rect.max),
    ]
}

fn side_at(left: egui::Rect, x: f32) -> RainSide {
    if x < left.right() {
        RainSide::Left
    } else {
        RainSide::Right
    }
}

pub fn double(
    ui: &mut egui::Ui,
    camera: &mut MapMemory,
    mut show: impl FnMut(
        &mut egui::Ui,
        RainSide,
        &mut MapMemory,
        bool,
    ) -> (egui::Response, Option<Location>),
) -> [(egui::Response, Option<Location>); 2] {
    let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
    let panes = split_rect(rect, 0.5);
    let id = ui.id().with("double-map-responses");
    let last_ids = ui.data(|data| data.get_temp::<[egui::Id; 2]>(id));
    let dragged = ui.ctx().dragged_id().or_else(|| ui.ctx().drag_stopped_id());
    let dragged_side = last_ids.and_then(|ids| {
        RainSide::BOTH
            .into_iter()
            .find(|side| Some(ids[side.index()]) == dragged)
    });
    let active = dragged_side.unwrap_or_else(|| {
        ui.input(|input| {
            input
                .pointer
                .hover_pos()
                .map_or(RainSide::Left, |p| side_at(panes[0], p.x))
        })
    });
    let render = |side: RainSide| {
        let rect = panes[side.index()];
        let mut pane = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(side.index()));
        pane.set_clip_rect(pane.clip_rect().intersect(rect));
        pane
    };
    let [mut left, mut right] = RainSide::BOTH.map(render);
    let [first, second] = match active {
        RainSide::Left => [&mut left, &mut right],
        RainSide::Right => [&mut right, &mut left],
    };
    let other = match active {
        RainSide::Left => RainSide::Right,
        RainSide::Right => RainSide::Left,
    };
    let active_result = show(first, active, camera, true);
    // Only the active map advances Walkers' momentum. Draw the second from
    // that same frame's center and zoom without advancing the camera again.
    let mut snapshot = camera.clone();
    if let Some(center) = camera.detached() {
        snapshot.center_at(center);
    }
    let other_result = show(second, other, &mut snapshot, false);
    let responses = match active {
        RainSide::Left => [active_result, other_result],
        RainSide::Right => [other_result, active_result],
    };
    ui.data_mut(|data| data.insert_temp(id, [responses[0].0.id, responses[1].0.id]));
    ui.painter().vline(
        panes[0].right(),
        rect.y_range(),
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    responses
}

fn swipe_control(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    fraction: &mut f32,
    pt: bool,
) -> egui::Response {
    let name = super::text(
        pt,
        "Rainfall swipe divider",
        "Divisor da comparação de chuva",
    );
    let x = egui::lerp(rect.x_range(), *fraction);
    let hit_rect = egui::Rect::from_x_y_ranges(x - 8. ..=x + 8., rect.y_range()).intersect(rect);
    let mut response = ui
        .interact(
            hit_rect,
            ui.id().with("rainfall-swipe"),
            egui::Sense::click_and_drag(),
        )
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
    let before = *fraction;
    if response.clicked() || response.drag_started() {
        response.request_focus();
    }
    if response.dragged()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        *fraction = ((pointer.x - rect.left()) / rect.width()).clamp(0., 1.);
    }
    if response.has_focus() {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    ..Default::default()
                },
            );
        });
        ui.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) {
                *fraction = (*fraction - 0.025).max(0.);
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                *fraction = (*fraction + 0.025).min(1.);
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
                *fraction = 0.;
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::End) {
                *fraction = 1.;
            }
        });
    }
    if *fraction != before {
        response.mark_changed();
        ui.request_repaint();
    }
    response.widget_info(|| {
        egui::WidgetInfo::slider(ui.is_enabled(), 100. * f64::from(*fraction), &name)
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_min_numeric_value(0.);
        node.set_max_numeric_value(100.);
        node.set_numeric_value_step(2.5);
    });
    response
}

fn paint_swipe(ui: &egui::Ui, rect: egui::Rect, fraction: f32, response: &egui::Response) {
    let x = egui::lerp(rect.x_range(), fraction);
    let half_width = rect.width().min(16.) / 2.;
    let center = egui::pos2(
        x.clamp(rect.left() + half_width, rect.right() - half_width),
        rect.center().y,
    );
    let painter = ui.painter().with_clip_rect(rect);
    let visuals = ui.style().interact(response);
    painter.vline(x, rect.y_range(), Stroke::new(2., visuals.text_color()));
    painter.rect(
        egui::Rect::from_center_size(center, egui::vec2(2. * half_width, 30.)),
        4.,
        visuals.bg_fill,
        if response.has_focus() {
            ui.visuals().selection.stroke
        } else {
            visuals.bg_stroke
        },
        egui::StrokeKind::Inside,
    );
    for dx in [-2., 2.] {
        painter.vline(
            center.x + dx,
            center.y - 6. ..=center.y + 6.,
            visuals.fg_stroke,
        );
    }
}

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
    projected_bounds: Bounds,
}
pub fn prepare_catchments(features: &[MiniMapFeature]) -> Result<Vec<PreparedMini>, String> {
    features
        .iter()
        .map(|feature| {
            let fill = prepare_fill(&feature.polygons)?;
            Ok(PreparedMini {
                mini_id: feature.mini_id,
                projected_bounds: Bounds::new(&fill.projected),
                fill,
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
    pub paths: Vec<ProjectedPath>,
}

type Point = (f64, f64);

struct Bounds {
    min: Point,
    max: Point,
}
impl Bounds {
    fn new(points: &[Point]) -> Self {
        let mut min = (f64::INFINITY, f64::INFINITY);
        let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &(x, y) in points {
            min = (min.0.min(x), min.1.min(y));
            max = (max.0.max(x), max.1.max(y));
        }
        Self { min, max }
    }
    fn intersects(&self, transform: &ScreenTransform, clip: egui::Rect) -> bool {
        self.min.0 <= self.max.0
            && egui::Rect::from_min_max(transform.project(self.min), transform.project(self.max))
                .intersects(clip)
    }
}
pub struct ProjectedPath {
    points: Vec<Point>,
    bounds: Bounds,
}
impl ProjectedPath {
    pub(super) fn new(points: Vec<Point>) -> Self {
        let bounds = Bounds::new(&points);
        Self { points, bounds }
    }
}
pub fn river_segments(features: &[MiniMapFeature]) -> Vec<RiverPaths> {
    features
        .iter()
        .map(|feature| RiverPaths {
            mini_id: feature.mini_id,
            paths: project_paths(&feature.paths)
                .into_iter()
                .map(ProjectedPath::new)
                .collect(),
        })
        .collect()
}
const OVERVIEW_ZOOMS: [i32; 4] = [7, 9, 11, 12];
pub type RiverOverviews = [Vec<RiverPaths>; OVERVIEW_ZOOMS.len()];

fn overview_tolerance(zoom: i32) -> f64 {
    0.5 / (256. * 2f64.powi(zoom))
}

fn overview_index(zoom: f64, pixels_per_point: f32) -> Option<usize> {
    if zoom >= 12. {
        return None;
    }
    let physical_zoom = zoom + f64::from(pixels_per_point).log2();
    OVERVIEW_ZOOMS
        .iter()
        .position(|&level| physical_zoom <= f64::from(level))
}

pub fn overview_segments(segments: &[RiverPaths]) -> RiverOverviews {
    OVERVIEW_ZOOMS.map(|level| {
        let tolerance = overview_tolerance(level);
        segments
            .iter()
            .map(|feature| RiverPaths {
                mini_id: feature.mini_id,
                paths: feature
                    .paths
                    .iter()
                    .map(|path| ProjectedPath::new(simplify(&path.points, tolerance)))
                    .collect(),
            })
            .collect()
    })
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
const RAIN_MIN_MM: f64 = 1.;
pub const RAIN_MAX_MM: f64 = 100.;

pub fn rain_color(value: Option<f64>, maximum: f64) -> Color32 {
    let Some(value) = value else {
        return Color32::TRANSPARENT;
    };
    let fraction = ((value - RAIN_MIN_MM) / (maximum - RAIN_MIN_MM)).clamp(0., 1.);
    let channel = |light: f64, dark: f64| (light + (dark - light) * fraction).round() as u8;
    Color32::from_rgb(channel(174., 5.), channel(214., 40.), channel(250., 135.))
}
pub fn layer_swatch(kind: &str) -> Color32 {
    if kind == "rainfall" {
        rain_color(Some(10.), RAIN_MAX_MM)
    } else {
        color(kind)
    }
}

pub fn rain_legend(ui: &mut egui::Ui, pt: bool, maximum: f64) {
    ui.strong(super::text(
        pt,
        "Accumulated rainfall (mm)",
        "Chuva acumulada (mm)",
    ));
    let width = ui.available_width().clamp(1., 340.);
    let (bar, _) = ui.allocate_exact_size(egui::vec2(width, 12.), egui::Sense::hover());
    let mut mesh = egui::Mesh::default();
    for step in 0..=32 {
        let fraction = step as f32 / 32.;
        let x = egui::lerp(bar.x_range(), fraction);
        let color = rain_color(
            Some(RAIN_MIN_MM + f64::from(fraction) * (maximum - RAIN_MIN_MM)),
            maximum,
        );
        mesh.colored_vertex(egui::pos2(x, bar.top()), color);
        mesh.colored_vertex(egui::pos2(x, bar.bottom()), color);
        if step > 0 {
            let i = 2 * step;
            mesh.add_triangle(i - 2, i, i - 1);
            mesh.add_triangle(i - 1, i, i + 1);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    let (ticks, _) = ui.allocate_exact_size(egui::vec2(width, 13.), egui::Sense::hover());
    let font = egui::TextStyle::Small.resolve(ui.style());
    let values: &[f64] = if width >= 145. && maximum >= 10. {
        &[1., maximum / 4., maximum / 2., maximum * 0.75, maximum]
    } else {
        &[1., maximum]
    };
    for &value in values {
        let fraction = ((value - RAIN_MIN_MM) / (maximum - RAIN_MIN_MM)) as f32;
        let align = if value == RAIN_MIN_MM {
            egui::Align2::LEFT_TOP
        } else if value == maximum {
            egui::Align2::RIGHT_TOP
        } else {
            egui::Align2::CENTER_TOP
        };
        ui.painter().text(
            egui::pos2(egui::lerp(ticks.x_range(), fraction), ticks.top()),
            align,
            if value.to_string().len() > 8 {
                format!("{value:.3e}")
            } else {
                value.to_string()
            },
            font.clone(),
            ui.visuals().text_color(),
        );
    }
    super::theme::color_key(ui, rain_color(None, RAIN_MAX_MM), "NA").on_hover_text(super::text(
        pt,
        "Incomplete coverage",
        "Cobertura incompleta",
    ));
}

fn screen(projector: &Projector, p: (f64, f64)) -> Pos2 {
    projector.project(lon_lat(p.0, p.1)).to_pos2()
}
fn map_label(
    painter: &egui::Painter,
    position: Pos2,
    anchor: egui::Align2,
    text: String,
    opacity: f32,
) {
    let color = Color32::BLACK.gamma_multiply(opacity);
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(11.), color);
    let rect = anchor.anchor_rect(egui::Rect::from_min_size(position, galley.size()));
    painter.rect_filled(
        rect.expand(2.),
        egui::CornerRadius::same(2),
        Color32::WHITE.gamma_multiply(0.9 * opacity),
    );
    painter.galley(rect.min, galley, color);
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
    fn colors(rain: &Rain, maximum: f64) -> egui::ColorImage {
        egui::ColorImage::new(
            [rain.grid.longitudes.len(), rain.grid.latitudes.len()],
            rain.values
                .iter()
                .map(|v| rain_color(*v, maximum))
                .collect(),
        )
    }
    pub fn recolor(&mut self, maximum: f64) {
        self.texture.set(
            Self::colors(&self.rain, maximum),
            egui::TextureOptions::NEAREST,
        );
    }
    pub fn new(context: &egui::Context, rain: Arc<Rain>, maximum: f64) -> Self {
        let image = Self::colors(&rain, maximum);
        let texture =
            context.load_texture("accumulated-rainfall", image, egui::TextureOptions::NEAREST);
        Self { texture, rain }
    }
    fn draw(&self, painter: &egui::Painter, projector: &Projector, opacity: f32) {
        painter.add(egui::Shape::mesh(rain_mesh(
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
const PAN_MARGIN: f32 = 128.;
// Retain enough geometry for stroke edges and river hit targets at the viewport edge.
const GEOMETRY_MARGIN: f32 = PAN_MARGIN + 16.;
type CameraKey = (egui::Rect, f64, usize, usize, f32);
pub type BasinPaths = Arc<Vec<ProjectedPath>>;
pub fn prepare_basin(paths: Vec<Vec<Point>>) -> BasinPaths {
    Arc::new(paths.into_iter().map(ProjectedPath::new).collect())
}
struct BasinOverlay {
    paths: BasinPaths,
    opacity: f32,
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
    anchor: Option<ScreenTransform>,
    offset: egui::Vec2,
    tinted_rivers: Option<(f32, Vec<Arc<egui::Mesh>>)>,
    basin: Option<BasinOverlay>,
    hits: Vec<RiverHit>,
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
        let key = (
            rect,
            memory.zoom(),
            Arc::as_ptr(&project.fills) as usize,
            Arc::as_ptr(&project.segments) as usize,
            context.pixels_per_point(),
        );
        if self.key == Some(key)
            && let Some(anchor) = &self.anchor
        {
            let offset = transform.project(anchor.center) - anchor.origin;
            if rect
                .expand(PAN_MARGIN)
                .contains_rect(rect.translate(-offset))
            {
                self.offset = offset;
                return;
            }
        }
        self.clear();
        self.key = Some(key);
        self.anchor = Some(transform);
    }

    fn rivers(&mut self, project: &Project, opacity: f32, context: &egui::Context) {
        if self
            .tinted_rivers
            .as_ref()
            .is_some_and(|(cached, _)| *cached == opacity)
        {
            return;
        }
        let (rect, zoom, _, _, pixels_per_point) = self.key.expect("map view prepared");
        let clip = rect.expand(GEOMETRY_MARGIN);
        let transform = self.anchor.as_ref().expect("map view prepared");
        if self.tinted_rivers.is_none() {
            self.hits = project
                .segments
                .iter()
                .flat_map(|feature| {
                    feature
                        .paths
                        .iter()
                        .map(move |path| (feature.mini_id, path))
                })
                .filter(|(_, path)| path.bounds.intersects(transform, clip))
                .map(|(mini_id, path)| {
                    let points: Vec<_> =
                        path.points.iter().map(|p| transform.project(*p)).collect();
                    RiverHit {
                        mini_id,
                        bounds: egui::Rect::from_points(&points),
                        points,
                    }
                })
                .collect();
        }
        let width = if zoom < 10. { 0.8 } else { 1.2 };
        let tint = color("rivers").gamma_multiply(opacity);
        let meshes = line_meshes(
            context,
            clip,
            if let Some(level) = overview_index(zoom, pixels_per_point) {
                project.overview_segments[level]
                    .iter()
                    .flat_map(|feature| &feature.paths)
                    .filter(|path| path.bounds.intersects(transform, clip))
                    .map(|path| {
                        egui::Shape::line(
                            path.points
                                .iter()
                                .map(|point| transform.project(*point))
                                .collect(),
                            Stroke::new(width, tint),
                        )
                    })
                    .collect()
            } else {
                self.hits
                    .iter()
                    .map(|hit| egui::Shape::line(hit.points.clone(), Stroke::new(width, tint)))
                    .collect()
            },
        );
        self.tinted_rivers = Some((opacity, meshes));
    }

    fn catchments(&mut self, project: &Project, opacity: f32) {
        if self
            .tinted_catchments
            .as_ref()
            .is_some_and(|(cached, _)| *cached == opacity)
        {
            return;
        }
        let clip = self
            .key
            .expect("map view prepared")
            .0
            .expand(GEOMETRY_MARGIN);
        let transform = self.anchor.as_ref().expect("map view prepared");
        let mut combined = egui::Mesh::default();
        let tint = color("catchments").gamma_multiply(opacity);
        for feature in project
            .fills
            .iter()
            .filter(|feature| feature.projected_bounds.intersects(transform, clip))
        {
            combined.append(mesh(&feature.fill, transform, tint));
        }
        self.tinted_catchments = Some((opacity, Arc::new(combined)));
    }

    fn paint(&self, ui: &egui::Ui, mesh: &Arc<egui::Mesh>) {
        if self.offset == egui::Vec2::ZERO {
            ui.painter().add(egui::Shape::Mesh(mesh.clone()));
        } else {
            let mut translated = (**mesh).clone();
            translated.translate(self.offset);
            ui.painter().add(egui::Shape::mesh(translated));
        }
    }
    fn basin(&mut self, paths: &BasinPaths, opacity: f32, context: &egui::Context) {
        if self
            .basin
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(&cached.paths, paths) && cached.opacity == opacity)
        {
            return;
        }
        let clip = self
            .key
            .expect("map view prepared")
            .0
            .expand(GEOMETRY_MARGIN);
        let transform = self.anchor.as_ref().expect("map view prepared");
        let tint = color("upstream").gamma_multiply(opacity);
        let outline = paths
            .iter()
            .filter(|path| path.bounds.intersects(transform, clip))
            .map(|path| {
                egui::Shape::line(
                    path.points.iter().map(|p| transform.project(*p)).collect(),
                    Stroke::new(1., tint),
                )
            })
            .collect();
        self.basin = Some(BasinOverlay {
            paths: paths.clone(),
            opacity,
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
    pub rainfall: [Option<&'a RainImage>; 2],
    pub side: Option<RainSide>,
    pub selection: &'a Option<Location>,
    pub upstream: Option<&'a BasinPaths>,
    pub hit: &'a mut Option<Location>,
    pub cache: &'a mut StaticCache,
    pub labels: bool,
    pub local: &'a [LocalLayer],
    pub local_display: &'a [LocalDisplay],
}
impl Overlay<'_> {
    pub fn show(
        mut self,
        ui: &mut egui::Ui,
        tiles: Option<&mut dyn Tiles>,
        camera: &mut MapMemory,
        interactive: bool,
        swipe: Option<&mut f32>,
        pt: bool,
    ) -> egui::InnerResponse<Option<String>> {
        let map = Map::new(tiles, camera, lon_lat(-52., -30.));
        let map = if interactive {
            map
        } else {
            map.zoom_gesture(false)
                .panning(false)
                .drag_pan_buttons(egui::DragPanButtons::empty())
        };
        map.show(ui, |ui, response, projector, memory| {
            let rect = response.rect;
            ui.set_clip_rect(ui.clip_rect().intersect(rect));
            if let Some(fraction) = swipe {
                let handle = swipe_control(ui, rect, fraction, pt);
                let readout = self.draw(ui, response, projector, memory, Some(*fraction), pt);
                paint_swipe(ui, rect, *fraction, &handle);
                readout
            } else {
                self.draw(ui, response, projector, memory, None, pt)
            }
        })
    }

    fn draw(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        projector: &Projector,
        memory: &MapMemory,
        swipe: Option<f32>,
        pt: bool,
    ) -> Option<String> {
        let clips = swipe.map(|fraction| split_rect(response.rect, fraction));
        let dominant_side = self.side.unwrap_or(RainSide::Left);
        self.cache
            .update(self.project, projector, response.rect, memory, ui.ctx());
        let click = response.clicked() || response.secondary_clicked();
        let hover = response
            .interact_pointer_pos()
            .filter(|_| click)
            .or_else(|| response.hover_pos());
        let primary_click = response.clicked_by(egui::PointerButton::Primary);
        let alt = ui.input(|input| {
            input.modifiers.alt
                || primary_click
                    && input.events.iter().any(|event| {
                        matches!(
                            event,
                            egui::Event::PointerButton {
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers,
                                ..
                            } if modifiers.alt
                        )
                    })
        });
        let visible = |kind| {
            self.layers
                .iter()
                .any(|layer| layer.kind == kind && layer.visible && layer.opacity > 0.)
        };
        let mut hit = None;
        let mut hit_label = String::new();
        let mut station_distance = 10.;
        // Station hit targets win over all polygon and line hits.
        for layer in self.layers.iter().filter(|l| l.visible && l.opacity > 0.) {
            match layer.kind.as_str() {
                "rainfall" => {
                    for side in RainSide::BOTH {
                        let clip = match clips {
                            Some(clips) => clips[side.index()],
                            None if side == dominant_side => response.rect,
                            None => continue,
                        };
                        if clip.width() > 0.
                            && let Some(rain) = self.rainfall[side.index()]
                        {
                            draw_rain(
                                ui,
                                projector,
                                memory.zoom(),
                                clip,
                                rain,
                                layer.opacity,
                                self.labels,
                            );
                        }
                    }
                }
                "catchments" => {
                    self.cache.catchments(self.project, layer.opacity);
                    if let Some((_, mesh)) = &self.cache.tinted_catchments {
                        self.cache.paint(ui, mesh);
                    }
                }
                "upstream" => {
                    if let Some(paths) = self.upstream {
                        self.cache.basin(paths, layer.opacity, ui.ctx());
                        if let Some(basin) = &self.cache.basin {
                            for mesh in &basin.outline {
                                self.cache.paint(ui, mesh);
                            }
                        }
                    } else {
                        self.cache.basin = None;
                    }
                }
                "rivers" => {
                    self.cache.rivers(self.project, layer.opacity, ui.ctx());
                    if let Some((_, meshes)) = &self.cache.tinted_rivers {
                        for mesh in meshes {
                            self.cache.paint(ui, mesh);
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
                                    ui.painter(),
                                    p + egui::vec2(7., -7.),
                                    egui::Align2::LEFT_BOTTOM,
                                    station.name.clone(),
                                    layer.opacity,
                                );
                            }
                            if layer.opacity > 0.
                                && let Some(distance) = hover.map(|pointer| pointer.distance(p))
                                && distance <= 10.
                                && (hit.is_none() || distance < station_distance)
                            {
                                station_distance = distance;
                                hit = Some(Location::Station(station.station_id.clone()));
                                hit_label = format!(
                                    "{}: {} ({})",
                                    super::text(pt, "Station", "Estação"),
                                    station.name,
                                    station.station_id
                                );
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
                    selected_line(
                        ui,
                        feature
                            .points
                            .iter()
                            .map(|p| *p + self.cache.offset)
                            .collect(),
                    );
                }
            }
        }
        let mut readout = None;
        if let Some(pointer) = hover {
            let geographic = projector.unproject(pointer.to_vec2());
            if hit.is_none() && !alt {
                if visible("rivers") {
                    let pointer = pointer - self.cache.offset;
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
                    hit_label = format!("MINI: {id}");
                }
            }
            if visible("rivers")
                && let Some(Location::Mini(id)) = &hit
                && !matches!(self.selection, Some(Location::Mini(selected)) if selected == id)
            {
                for feature in self.cache.hits.iter().filter(|f| f.mini_id == *id) {
                    ui.painter().add(egui::Shape::line(
                        feature
                            .points
                            .iter()
                            .map(|p| *p + self.cache.offset)
                            .collect(),
                        Stroke::new(2., Color32::WHITE),
                    ));
                }
            }
            let side = clips.map_or(dominant_side, |clips| side_at(clips[0], pointer.x));
            let coordinates = format!("Lat: {:.5} | Lon: {:.5}", geographic.y(), geographic.x());
            let mut labels = vec![coordinates];
            if !hit_label.is_empty() {
                labels.push(hit_label);
            }
            if visible("rainfall")
                && let Some(rainfall) = self.rainfall[side.index()]
                && let Some((col, row, value)) = rainfall.inspect((geographic.x(), geographic.y()))
            {
                let rainfall_label = super::text(pt, "Rainfall", "Chuva");
                let amount = value.map_or_else(|| "NA".into(), |mm| format!("{mm:.2} mm"));
                labels.push(format!(
                    "{rainfall_label}: {amount} ({}, [{col}, {row}])",
                    rainfall.rain.scenario
                ));
            }
            if !labels.is_empty() {
                if clips.is_some() || self.side.is_some() {
                    labels.insert(0, side_label(side, pt));
                }
                readout = Some(labels.join(" | "));
            }
        }
        if click && !alt {
            *self.hit = hit;
        }
        readout
    }
}
fn draw_rain(
    ui: &egui::Ui,
    projector: &Projector,
    zoom: f64,
    clip: egui::Rect,
    rain: &RainImage,
    opacity: f32,
    labels: bool,
) {
    let painter = ui.painter().with_clip_rect(clip);
    rain.draw(&painter, projector, opacity);
    if !labels || zoom < 10. {
        return;
    }
    let grid = &rain.rain.grid;
    let step = grid.resolution_degrees;
    let distance = screen(projector, (grid.longitudes[0] + step, grid.latitudes[0]))
        .distance(screen(projector, (grid.longitudes[0], grid.latitudes[0])));
    if distance >= 40. {
        for (row, &lat) in grid.latitudes.iter().enumerate() {
            for (col, &lon) in grid.longitudes.iter().enumerate() {
                let p = screen(projector, (lon, lat));
                if clip.contains(p)
                    && let Some(value) = rain.rain.values[row * grid.longitudes.len() + col]
                {
                    map_label(
                        &painter,
                        p + egui::vec2(3., 3.),
                        egui::Align2::LEFT_TOP,
                        format!("{value:.1}"),
                        opacity,
                    );
                }
            }
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

pub fn upstream_boundary(project: &Project, ids: &[i64]) -> Vec<Vec<(f64, f64)>> {
    use geo::{LineString, Orient, Polygon};
    let ids: HashSet<_> = ids.iter().copied().collect();
    let ring = |points: &[(f64, f64)]| LineString::from(points.to_vec());
    let polygons = project
        .catchments
        .iter()
        .filter(|feature| ids.contains(&feature.mini_id))
        .flat_map(|feature| &feature.polygons)
        .map(|p| {
            Polygon::new(
                ring(&p.exterior),
                p.interiors.iter().map(|hole| ring(hole)).collect(),
            )
            .orient(geo::orient::Direction::Default)
        })
        .collect::<Vec<_>>();
    geo::algorithm::bool_ops::unary_union(&polygons)
        .0
        .iter()
        .map(|polygon| {
            polygon
                .exterior()
                .0
                .iter()
                .map(|p| world((p.x, p.y)))
                .collect()
        })
        .collect()
}

pub struct BlueMarble;
impl walkers::sources::TileSource for BlueMarble {
    fn tile_url(&self, tile: walkers::TileId) -> String {
        format!(
            "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_NextGeneration/default/GoogleMapsCompatible_Level8/{}/{}/{}.jpeg",
            tile.zoom, tile.y, tile.x
        )
    }
    fn attribution(&self) -> walkers::sources::Attribution {
        walkers::sources::Attribution {
            text: "NASA Earthdata · Blue Marble",
            url: "https://www.earthdata.nasa.gov/data/projects/gibs",
            logo_light: None,
            logo_dark: None,
        }
    }
    fn max_zoom(&self) -> u8 {
        8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_frame(
        context: &egui::Context,
        events: Vec<egui::Event>,
        mut render: impl FnMut(&mut egui::Ui),
    ) {
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(500., 500.),
                )),
                events,
                ..Default::default()
            },
            |ui| render(ui),
        );
        output.textures_delta.clear();
    }

    fn primary(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn blue_marble_uses_native_wmts_order_and_zoom() {
        use walkers::sources::TileSource;
        let url = BlueMarble.tile_url(walkers::TileId {
            zoom: 8,
            x: 91,
            y: 150,
        });
        assert!(url.ends_with("GoogleMapsCompatible_Level8/8/150/91.jpeg"));
        assert_eq!(BlueMarble.max_zoom(), 8);
        assert_eq!(rain_color(Some(40.), 40.), rain_color(Some(100.), 100.));
    }

    #[test]
    fn double_maps_share_current_camera_when_zooming_and_dragging_either_side() {
        let ctx = egui::Context::default();
        let mut camera = MapMemory::default();
        camera.set_zoom(7.).unwrap();
        camera.center_at(lon_lat(-52., -30.));
        let mut run = |events| {
            let mut rendered = [None; 2];
            render_frame(&ctx, events, |ui| {
                let panes = double(ui, &mut camera, |ui, side, memory, active| {
                    let response = Map::new(None, memory, lon_lat(-52., -30.))
                        .zoom_gesture(active)
                        .panning(active)
                        .show(ui, |_, response, projector, memory| {
                            rendered[side.index()] = Some((
                                memory.zoom(),
                                projector.unproject(response.rect.center().to_vec2()),
                            ));
                        })
                        .response;
                    (response, None)
                });
                assert_eq!(panes[0].0.rect.width(), panes[1].0.rect.width());
            });
            let ((left_zoom, left_center), (right_zoom, right_center)) =
                (rendered[0].unwrap(), rendered[1].unwrap());
            assert_eq!(left_zoom, right_zoom);
            assert!((left_center.x() - right_center.x()).abs() < 1e-10);
            assert!((left_center.y() - right_center.y()).abs() < 1e-10);
            (left_zoom, left_center)
        };
        let left = egui::pos2(100., 100.);
        let right = egui::pos2(400., 100.);
        run(vec![]);
        // Zoom on the right, then drag across the center and release in the left pane.
        run(vec![egui::Event::PointerMoved(right)]);
        let (zoom, before_drag) = run(vec![egui::Event::Zoom(1.2)]);
        assert!(zoom > 7.);
        run(vec![primary(right, true)]);
        let (_, after_drag) = run(vec![egui::Event::PointerMoved(left)]);
        assert_ne!(before_drag, after_drag);
        run(vec![primary(left, false)]);
        // The left pane can also update the shared camera.
        let (zoom, _) = run(vec![egui::Event::Zoom(0.8)]);
        assert!(zoom < 7.4);
        run(vec![primary(left, true)]);
        let (_, before_move) = run(vec![]);
        let (_, after_move) = run(vec![egui::Event::PointerMoved(left + egui::vec2(10., 0.))]);
        assert_ne!(before_move, after_move);
    }

    #[test]
    fn swipe_divider_drag_controls_clips_without_moving_camera_and_accepts_keys() {
        let ctx = egui::Context::default();
        let mut camera = MapMemory::default();
        let center = lon_lat(-52., -30.);
        camera.center_at(center);
        let original = camera.detached();
        let mut fraction = 0.5;
        let mut run = |events| {
            render_frame(&ctx, events, |ui| {
                Map::new(None, &mut camera, center).show(ui, |ui, response, _, _| {
                    swipe_control(ui, response.rect, &mut fraction, false);
                    let [left, right] = split_rect(response.rect, fraction);
                    assert_eq!(left.right(), right.left());
                    assert_eq!(side_at(left, right.left()), RainSide::Right);
                });
            });
            assert_eq!(camera.detached(), original);
            fraction
        };
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let start = egui::pos2(250., 250.);
        let end = egui::pos2(350., 250.);
        run(vec![egui::Event::PointerMoved(start)]);
        run(vec![primary(start, true)]);
        let dragged = run(vec![egui::Event::PointerMoved(end)]);
        assert!((dragged - 0.7).abs() < 0.01);
        run(vec![primary(end, false)]);
        let after_arrow = run(vec![key(egui::Key::ArrowLeft)]);
        assert!((after_arrow - 0.675).abs() < 0.01);
        let home = run(vec![key(egui::Key::Home)]);
        let end = run(vec![key(egui::Key::End)]);
        assert_eq!(home, 0.);
        assert_eq!(end, 1.);
    }

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
        assert_eq!(rain_color(None, RAIN_MAX_MM), Color32::TRANSPARENT);
        assert_ne!(rain_color(Some(0.), RAIN_MAX_MM), Color32::TRANSPARENT);
        assert_eq!(
            rain_color(Some(0.), RAIN_MAX_MM),
            rain_color(Some(1.), RAIN_MAX_MM)
        );
        assert_eq!(
            rain_color(Some(150.), RAIN_MAX_MM),
            rain_color(Some(100.), RAIN_MAX_MM)
        );
        assert!(rain_color(Some(1.), RAIN_MAX_MM).r() > rain_color(Some(50.), RAIN_MAX_MM).r());
        assert!(rain_color(Some(50.), RAIN_MAX_MM).r() > rain_color(Some(100.), RAIN_MAX_MM).r());
    }
    #[test]
    fn overlay_meshes_are_lazy_and_reused_during_short_pans() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("workspace.toml");
        std::fs::write(&config, "[spatial]\ngpkg_path = \"missing.gpkg\"\n").unwrap();
        let mut project = super::super::data::open(config, dir.path().into(), None).unwrap();
        let ring = vec![(0., 0.), (1., 0.), (1., 1.), (0., 1.), (0., 0.)];
        let hole = vec![(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8), (0.2, 0.2)];
        let polygon = MapPolygon {
            exterior: ring.clone(),
            interiors: vec![hole],
        };
        let feature = MiniMapFeature {
            mini_id: 1,
            paths: vec![ring],
            polygons: vec![polygon],
        };
        let adjacent = vec![(1., 0.), (2., 0.), (2., 1.), (1., 1.), (1., 0.)];
        let neighbor = MiniMapFeature {
            mini_id: 2,
            paths: vec![adjacent.clone()],
            polygons: vec![MapPolygon {
                exterior: adjacent,
                interiors: Vec::new(),
            }],
        };
        let project_mut = Arc::get_mut(&mut project).unwrap();
        project_mut.fills = Arc::new(prepare_catchments(std::slice::from_ref(&feature)).unwrap());
        project_mut.segments = Arc::new(river_segments(std::slice::from_ref(&feature)));
        project_mut.overview_segments = Arc::new(overview_segments(&project_mut.segments));
        project_mut.catchments = Arc::new(vec![feature, neighbor]);
        let outline = upstream_boundary(&project, &[1, 2]);
        assert_eq!(outline.len(), 1);
        assert!(
            outline[0]
                .iter()
                .all(|(_, y)| (*y - world((0., 0.)).1).abs() < 1e-8
                    || (*y - world((0., 1.)).1).abs() < 1e-8)
        );
        let context = egui::Context::default();
        let mut memory = MapMemory::default();
        memory.set_zoom(7.).unwrap();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(500., 500.));
        let mut cache = StaticCache::default();
        let boundary = prepare_basin(upstream_boundary(&project, &[1]));
        let mut previous = None;
        for longitude in [0.5, 1.0, 4.0] {
            let mut frame = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    ui.label("cache fixture");
                    let projector = Projector::new(rect, &memory, lon_lat(longitude, 0.5));
                    cache.update(&project, &projector, rect, &memory, ui.ctx());
                    if longitude != 1.0 {
                        assert!(cache.tinted_rivers.is_none());
                        assert!(cache.tinted_catchments.is_none());
                        assert!(cache.hits.is_empty());
                    }
                    cache.rivers(&project, 0.8, ui.ctx());
                    if longitude != 4.0 {
                        let actual = cache.hits[0].points[0] + cache.offset;
                        assert!(actual.distance(screen(&projector, (0., 0.))) < 0.001);
                    }
                    cache.basin(&boundary, 0.3, ui.ctx());
                },
            );
            frame.textures_delta.clear();
            let mesh = cache.basin.as_ref().unwrap().outline[0].clone();
            let outlined = cache
                .basin
                .as_ref()
                .unwrap()
                .outline
                .iter()
                .any(|m| !m.indices.is_empty());
            assert!(outlined);
            if let Some(previous) = previous {
                assert_eq!(Arc::ptr_eq(&previous, &mesh), longitude == 1.0);
            }
            previous = Some(mesh);
        }
        cache.clear();
        assert!(cache.basin.is_none());
    }
    #[test]
    fn overview_selection_bounds_error_in_physical_pixels() {
        for zoom in [5., 7., 7.01, 9., 11., 11.5, 11.99, 12.] {
            for scale in [1., 1.25, 2., 3.] {
                if let Some(index) = overview_index(zoom, scale) {
                    let tolerance = overview_tolerance(OVERVIEW_ZOOMS[index]);
                    assert!(tolerance * 256. * 2f64.powf(zoom) * f64::from(scale) <= 0.5);
                }
            }
        }
        assert_eq!(overview_index(7., 1.), Some(0));
        assert_eq!(overview_index(7., 2.), Some(1));
        assert_eq!(overview_index(11.5, 2.), None);
        assert_eq!(overview_index(12., 1.), None);
    }

    #[test]
    fn overview_paths_keep_feature_ids_and_endpoints() {
        let points = vec![(0., 0.), (0.5, 0.000_01), (1., 0.)];
        let segments = [RiverPaths {
            mini_id: 42,
            paths: vec![ProjectedPath::new(points.clone())],
        }];
        let overviews = overview_segments(&segments);
        assert_eq!(overviews[0][0].mini_id, 42);
        assert_eq!(overviews[0][0].paths[0].points, [points[0], points[2]]);
        assert_eq!(overviews[1][0].paths[0].points, points);
        assert_eq!(segments[0].paths[0].points, points);
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
