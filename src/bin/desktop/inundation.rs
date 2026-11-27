use super::{map, text};
use eframe::egui::{self, Color32};
#[cfg(test)]
use mgb_ops::store::digest_file;
use mgb_ops::store::{
    FLOOD_MASK_FILE, InundationLevel, InundationMask, InundationMetadata, InundationProvenance,
    verify_inundation_run,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::thread;
use terrio::crs::{SpatialRef, Transformer};
use walkers::Projector;

pub const KIND: &str = "inundation";
pub const COLOR: Color32 = Color32::from_rgb(135, 44, 191);

const TILE_SIZE: usize = 256;
const CACHE_SIZE: usize = 168;
const TILES_PER_PANE: usize = 40;
const BATCH_SIZE: usize = 4;

#[derive(Clone, PartialEq, Eq)]
pub struct Source {
    pub selection: Option<super::data::SelectedRun>,
    pub root: PathBuf,
    pub scenario: String,
}

impl Source {
    fn path(&self) -> PathBuf {
        self.root
            .join("results")
            .join(&self.scenario)
            .join(FLOOD_MASK_FILE)
    }

    fn verify_provenance(&self, provenance: &InundationProvenance) -> Result<(), String> {
        verify_inundation_run(&self.root, &self.scenario, provenance)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tile {
    level: InundationLevel,
    x: usize,
    y: usize,
}

impl Hash for Tile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.level.factor(), self.x, self.y).hash(state);
    }
}

impl Tile {
    fn window(self, meta: &InundationMetadata) -> (usize, usize, usize, usize) {
        let (width, height) = meta.dimensions(self.level);
        let x = self.x * TILE_SIZE;
        let y = self.y * TILE_SIZE;
        (x, y, TILE_SIZE.min(width - x), TILE_SIZE.min(height - y))
    }
}

struct Grid {
    meta: InundationMetadata,
    to_source: Transformer,
}

struct TileBounds {
    min: [usize; 2],
    max: [usize; 2],
}

struct ViewportTiles {
    tiles: Vec<(Tile, f64)>,
    truncated: bool,
}

impl Grid {
    fn bounds(&self, projector: &Projector, clip: egui::Rect) -> Option<TileBounds> {
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        // Sample each edge to account for curvature in the datum transformation.
        for i in 0..=4 {
            let f = i as f32 / 4.;
            let x = egui::lerp(clip.x_range(), f);
            let y = egui::lerp(clip.y_range(), f);
            for point in [
                egui::pos2(x, clip.top()),
                egui::pos2(x, clip.bottom()),
                egui::pos2(clip.left(), y),
                egui::pos2(clip.right(), y),
            ] {
                let geo = projector.unproject(point.to_vec2());
                let (x, y) = self
                    .to_source
                    .transform(geo.x().clamp(-180., 180.), geo.y())
                    .ok()?;
                let cells = [
                    (x - self.meta.origin_x) / self.meta.pixel_width,
                    (y - self.meta.origin_y) / self.meta.pixel_height,
                ];
                for axis in 0..2 {
                    min[axis] = min[axis].min(cells[axis]);
                    max[axis] = max[axis].max(cells[axis]);
                }
            }
        }
        let dimensions = [self.meta.width, self.meta.height];
        if (0..2).any(|axis| max[axis] <= 0. || min[axis] >= dimensions[axis] as f64) {
            return None;
        }
        // Include one native cell around the viewport to tolerate projection rounding.
        Some(TileBounds {
            min: std::array::from_fn(|axis| (min[axis].floor() - 1.).max(0.) as usize),
            max: std::array::from_fn(|axis| {
                ((max[axis].ceil() + 1.) as usize).min(dimensions[axis]) - 1
            }),
        })
    }

    fn visible_tiles(
        &self,
        projector: &Projector,
        clip: egui::Rect,
        physical_world_width: f64,
    ) -> Option<ViewportTiles> {
        let bounds = self.bounds(projector, clip)?;
        let latitude = projector.unproject(clip.center().to_vec2()).y();
        let pixel_span = physical_world_width / 360.
            * self
                .meta
                .pixel_width
                .min(self.meta.pixel_height.abs() / latitude.to_radians().cos());
        let preferred = if pixel_span >= 0.85 {
            InundationLevel::Full
        } else if pixel_span * 4. >= 0.85 {
            InundationLevel::Overview4
        } else {
            InundationLevel::Overview16
        };
        for level in [
            InundationLevel::Full,
            InundationLevel::Overview4,
            InundationLevel::Overview16,
        ]
        .into_iter()
        .filter(|level| level.factor() >= preferred.factor())
        {
            let step = TILE_SIZE * level.factor();
            let mut min = bounds.min.map(|cell| cell / step);
            let mut max = bounds.max.map(|cell| cell / step);
            let count = (max[0] - min[0] + 1).saturating_mul(max[1] - min[1] + 1);
            if count > TILES_PER_PANE && level != InundationLevel::Overview16 {
                continue;
            }
            let center = std::array::from_fn::<_, 2, _>(|axis| {
                min[axis] as f64 + (max[axis] - min[axis]) as f64 / 2.
            });
            if count > TILES_PER_PANE {
                // Limit pathological viewport requests even at the coarsest resolution.
                let size = TILES_PER_PANE.isqrt();
                for axis in 0..2 {
                    min[axis] =
                        min[axis].max((center[axis].floor() as usize).saturating_sub(size / 2));
                    max[axis] = max[axis].min(min[axis] + size - 1);
                }
            }
            let mut tiles = Vec::new();
            for y in min[1]..=max[1] {
                for x in min[0]..=max[0] {
                    let distance = (x as f64 - center[0]).abs() + (y as f64 - center[1]).abs();
                    tiles.push((Tile { level, x, y }, distance));
                }
            }
            return Some(ViewportTiles {
                tiles,
                truncated: count > TILES_PER_PANE,
            });
        }
        None
    }
}

fn tile_color(level: InundationLevel, value: u8) -> Color32 {
    let alpha = match (level, value) {
        (_, 0 | 255) => return Color32::TRANSPARENT,
        (InundationLevel::Full, 1) => 255,
        (InundationLevel::Full, _) => return Color32::TRANSPARENT,
        (_, coverage) => (u16::from(coverage) * 255 / 100) as u8,
    };
    Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
}

struct TileImage {
    image: egui::ColorImage,
    geometry: TileGeometry,
}

#[derive(Clone)]
struct TileGeometry {
    vertices: Vec<((f64, f64), egui::Pos2)>,
    indices: Vec<u32>,
}

fn tile_geometry(
    meta: &InundationMetadata,
    to_wgs84: &Transformer,
    tile: Tile,
) -> Result<TileGeometry, String> {
    let (x, y, width, height) = tile.window(meta);
    let factor = tile.level.factor();
    let cuts = |offset: usize, length: usize, native: usize, angular_step: f64| {
        // Densify the texture mesh so Web Mercator scale and the datum shift
        // vary across the tile. Preserve a short final overview cell exactly.
        let step = (0.125 / (angular_step * factor as f64)).clamp(1., 16.) as usize;
        let mut edges: Vec<_> = (0..length).step_by(step).collect();
        if (offset + length) * factor > native && edges.last() != Some(&(length - 1)) {
            edges.push(length - 1);
        }
        edges.push(length);
        edges
    };
    let xs = cuts(x, width, meta.width, meta.pixel_width);
    let ys = cuts(y, height, meta.height, meta.pixel_height.abs());
    let mut vertices = Vec::with_capacity(xs.len() * ys.len());
    for row in &ys {
        let lat = meta.origin_y + ((y + row) * factor).min(meta.height) as f64 * meta.pixel_height;
        for col in &xs {
            let lon =
                meta.origin_x + ((x + col) * factor).min(meta.width) as f64 * meta.pixel_width;
            let wgs84 = to_wgs84.transform(lon, lat).map_err(|e| e.to_string())?;
            vertices.push((
                map::world(wgs84),
                egui::pos2(*col as f32 / width as f32, *row as f32 / height as f32),
            ));
        }
    }
    let stride = xs.len() as u32;
    let mut indices = Vec::new();
    for row in 0..ys.len() as u32 - 1 {
        for col in 0..stride - 1 {
            let nw = row * stride + col;
            let sw = nw + stride;
            indices.extend([nw, nw + 1, sw, nw + 1, sw + 1, sw]);
        }
    }
    Ok(TileGeometry { vertices, indices })
}

struct OpenMask {
    reader: InundationMask,
    to_wgs84: Transformer,
    geometry: std::cell::RefCell<HashMap<Tile, TileGeometry>>,
}

impl OpenMask {
    fn load(source: &Source) -> Result<(Self, Grid), Finding> {
        if let Some(selection) = &source.selection {
            selection
                .read(|| Ok(Self::open(source)))
                .map_err(Finding::Invalid)?
        } else {
            Self::open(source)
        }
    }

    fn open(source: &Source) -> Result<(Self, Grid), Finding> {
        let path = source.path();
        if matches!(path.try_exists(), Ok(false)) {
            return Err(Finding::Missing);
        }
        let reader = InundationMask::open(&path)
            .map_err(|e| Finding::Invalid(format!("{}: {e}", path.display())))?;
        let meta = reader.metadata().clone();
        source
            .verify_provenance(&meta.provenance)
            .map_err(|e| Finding::Invalid(format!("{}: {e}", path.display())))?;
        let transform = |from, to| {
            SpatialRef::from_epsg(from)
                .and_then(|from| Transformer::new(&from, &SpatialRef::from_epsg(to)?))
                .map_err(|e| Finding::Invalid(e.to_string()))
        };
        let to_wgs84 = transform(meta.epsg as u32, 4326)?;
        let to_source = transform(4326, meta.epsg as u32)?;
        Ok((
            Self {
                reader,
                to_wgs84,
                geometry: std::cell::RefCell::new(HashMap::new()),
            },
            Grid { meta, to_source },
        ))
    }

    fn tile(&self, frame: usize, tile: Tile) -> Result<Option<TileImage>, String> {
        let meta = self.reader.metadata();
        let (x, y, width, height) = tile.window(meta);
        let pixels: Vec<_> = self
            .reader
            .read_window(tile.level, frame, x, y, width, height)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|value| tile_color(tile.level, value))
            .collect();
        if pixels.iter().all(|color| *color == Color32::TRANSPARENT) {
            return Ok(None);
        }
        let cached_geometry = self.geometry.borrow().get(&tile).cloned();
        let geometry = if let Some(geometry) = cached_geometry {
            geometry
        } else {
            let geometry = tile_geometry(meta, &self.to_wgs84, tile)?;
            let mut cache = self.geometry.borrow_mut();
            if cache.len() >= CACHE_SIZE {
                cache.clear();
            }
            cache.insert(tile, geometry.clone());
            geometry
        };
        Ok(Some(TileImage {
            image: egui::ColorImage::new([width, height], pixels),
            geometry,
        }))
    }
}

#[derive(Debug)]
enum Finding {
    Missing,
    Invalid(String),
}

enum Job {
    Close,
    Open(Source),
    Tiles { frame: usize, tiles: Vec<Tile> },
}

type TileResult = (Tile, Result<Option<TileImage>, String>);

enum Loaded {
    Open(Result<Box<Grid>, Finding>),
    Tiles(Vec<TileResult>),
}

struct Worker {
    jobs: mpsc::Sender<(u64, Job)>,
    replies: mpsc::Receiver<(u64, Loaded)>,
    epoch: Arc<AtomicU64>,
    in_flight: bool,
}

impl Worker {
    fn new(context: egui::Context) -> Self {
        let (jobs, pending) = mpsc::channel::<(u64, Job)>();
        let (loaded, replies) = mpsc::channel();
        let epoch = Arc::new(AtomicU64::new(0));
        let active = epoch.clone();
        thread::spawn(move || {
            let mut open = None;
            while let Ok((revision, job)) = pending.recv() {
                let result = match job {
                    Job::Close => {
                        open = None;
                        continue;
                    }
                    Job::Open(source) => {
                        open = None;
                        Loaded::Open(OpenMask::load(&source).map(|(mask, grid)| {
                            open = Some((mask, source));
                            Box::new(grid)
                        }))
                    }
                    Job::Tiles { frame, tiles } => {
                        let results = match &open {
                            Some((mask, source)) => {
                                let read = || {
                                    Ok(tiles
                                        .iter()
                                        .copied()
                                        .take_while(|_| revision == active.load(Ordering::Relaxed))
                                        .map(|tile| (tile, mask.tile(frame, tile)))
                                        .collect())
                                };
                                match &source.selection {
                                    Some(selection) => selection.read(read),
                                    None => read(),
                                }
                            }
                            None => Err("No inundation mask is open".into()),
                        }
                        .unwrap_or_else(|error| {
                            tiles
                                .into_iter()
                                .map(|tile| (tile, Err(error.clone())))
                                .collect()
                        });
                        Loaded::Tiles(results)
                    }
                };
                if loaded.send((revision, result)).is_err() {
                    break;
                }
                context.request_repaint();
            }
        });
        Self {
            jobs,
            replies,
            epoch,
            in_flight: false,
        }
    }

    fn invalidate(&self) {
        self.epoch.fetch_add(1, Ordering::Relaxed);
    }

    fn close(&self) {
        self.invalidate();
        let _ = self.jobs.send((0, Job::Close));
    }

    fn submit(&mut self, job: Job) -> Result<(), Finding> {
        self.jobs
            .send((self.epoch.load(Ordering::Relaxed), job))
            .map_err(|e| Finding::Invalid(e.to_string()))?;
        self.in_flight = true;
        Ok(())
    }
}

struct Texture {
    handle: egui::TextureHandle,
    geometry: TileGeometry,
}

struct Previous {
    source: Source,
    grid: Box<Grid>,
    frame: usize,
    cache: HashMap<(usize, Tile), CachedTile>,
}

struct CachedTile {
    texture: Option<Texture>,
    used: u64,
}

pub struct Inundation {
    worker: Worker,
    source: Option<Source>,
    grid: Option<Box<Grid>>,
    finding: Option<Finding>,
    frame: Option<usize>,
    displayed: Option<usize>,
    previous: Option<Previous>,
    calendar_month: Option<chrono::NaiveDate>,
    viewport_seen: bool,
    cache: HashMap<(usize, Tile), CachedTile>,
    demand: HashMap<Tile, f64>,
    pass: u64,
    truncated: bool,
    previous_truncated: bool,
}

impl Inundation {
    pub fn new(context: egui::Context) -> Self {
        Self {
            worker: Worker::new(context),
            source: None,
            grid: None,
            finding: None,
            frame: None,
            displayed: None,
            previous: None,
            calendar_month: None,
            viewport_seen: false,
            cache: HashMap::new(),
            demand: HashMap::new(),
            pass: 0,
            truncated: false,
            previous_truncated: false,
        }
    }

    pub fn select(&mut self, source: Option<Source>) {
        if source != self.source {
            if source.is_none() {
                self.previous = None;
                self.displayed = None;
                self.grid = None;
                self.cache.clear();
            } else {
                self.preserve_display();
            }
            self.source = source;
            self.frame = None;
            self.reset_pending();
        }
    }

    fn preserve_display(&mut self) {
        if let (Some(source), Some(grid), Some(frame)) =
            (self.source.clone(), self.grid.take(), self.displayed.take())
        {
            let mut cache = std::mem::take(&mut self.cache);
            cache.retain(|(cached_frame, _), _| *cached_frame == frame);
            while cache.len() > TILES_PER_PANE * 2 {
                let Some((&key, _)) = cache.iter().min_by_key(|(_, tile)| tile.used) else {
                    break;
                };
                cache.remove(&key);
            }
            self.previous = Some(Previous {
                source,
                grid,
                frame,
                cache,
            });
        }
    }

    fn reset_pending(&mut self) {
        self.worker.close();
        self.calendar_month = None;
        self.viewport_seen = false;
        self.grid = None;
        self.displayed = None;
        self.finding = None;
        self.cache.clear();
        self.demand.clear();
        self.previous_truncated = false;
    }

    pub fn refresh(&mut self) {
        self.preserve_display();
        self.reset_pending();
    }

    fn change_frame(&mut self, frame: usize) {
        if self.frame != Some(frame) {
            self.frame = Some(frame);
            self.worker.invalidate();
            self.finding = None;
            self.demand.clear();
        }
    }

    pub fn controls(&mut self, ui: &mut egui::Ui, pt: bool) {
        if self.source.is_none() {
            return;
        }
        if let (Some(grid), Some(mut frame)) = (&self.grid, self.frame) {
            let meta = &grid.meta;
            let last = meta.timeline.bounds.len() - 1;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        frame > 0,
                        egui::Button::new(egui::RichText::new("←").size(20.0))
                            .min_size(egui::vec2(32.0, 28.0)),
                    )
                    .on_hover_text(text(pt, "Previous hour", "Hora anterior"))
                    .clicked()
                {
                    frame -= 1;
                }
                hour_slider(ui, &mut frame, last, pt);
                if ui
                    .add_enabled(
                        frame < last,
                        egui::Button::new(egui::RichText::new("→").size(20.0))
                            .min_size(egui::vec2(32.0, 28.0)),
                    )
                    .on_hover_text(text(pt, "Next hour", "Próxima hora"))
                    .clicked()
                {
                    frame += 1;
                }
            });
            let current = meta.timeline.bounds[frame].end;
            ui.horizontal_wrapped(|ui| {
                ui.label(text(pt, "Date", "Data"));
                if let Some(date) = calendar_picker(
                    ui,
                    current.date(),
                    &mut self.calendar_month,
                    &meta.timeline.bounds,
                    pt,
                ) && let Some(index) =
                    frame_for_date(&meta.timeline.bounds, date, current.time())
                {
                    frame = index;
                }
                let date = meta.timeline.bounds[frame].end.date();
                use chrono::Timelike;
                let format = if meta
                    .timeline
                    .bounds
                    .iter()
                    .any(|bound| bound.end.second() != 0)
                {
                    "%H:%M:%S"
                } else {
                    "%H:%M"
                };
                egui::ComboBox::from_id_salt("inundation-time")
                    .selected_text(meta.timeline.bounds[frame].end.format(format).to_string())
                    .show_ui(ui, |ui| {
                        for (index, bound) in meta
                            .timeline
                            .bounds
                            .iter()
                            .enumerate()
                            .filter(|(_, bound)| bound.end.date() == date)
                        {
                            ui.selectable_value(
                                &mut frame,
                                index,
                                bound.end.format(format).to_string(),
                            );
                        }
                    });
            });
            let forecast = meta.timeline.bounds.get(meta.historical_count);
            let current = if frame < meta.historical_count {
                text(pt, "Historical", "Histórico")
            } else {
                text(pt, "Forecast", "Previsão")
            };
            let display_format = if meta.timeline.bounds.iter().any(|bound| {
                use chrono::Timelike;
                bound.start.second() != 0 || bound.end.second() != 0
            }) {
                text(pt, "%Y-%m-%d %H:%M:%S", "%d/%m/%Y %H:%M:%S")
            } else {
                text(pt, "%Y-%m-%d %H:%M", "%d/%m/%Y %H:%M")
            };
            ui.label(format!(
                "{} · {current}",
                meta.timeline.bounds[frame].end.format(&display_format),
            ));
            if let Some(forecast) = forecast {
                ui.small(format!(
                    "{} · {}",
                    text(pt, "Forecast starts", "Início da previsão"),
                    forecast.start.format(&display_format),
                ));
            } else {
                ui.small(text(pt, "No forecast hours", "Sem horas de previsão"));
            }
            if self.previous_truncated {
                ui.small(text(
                    pt,
                    "Zoom in to see the rest of the inundation mask.",
                    "Amplie para ver o restante da máscara de inundação.",
                ));
            }
            self.change_frame(frame);
        }
        if let Some(previous) = &self.previous {
            ui.small(format!(
                "{}: {} · {}",
                text(
                    pt,
                    "Showing previous inundation",
                    "Exibindo inundação anterior"
                ),
                previous.source.scenario,
                local_timestamp(previous.grid.meta.timeline.bounds[previous.frame].end, pt)
            ));
        } else if let (Some(displayed), Some(requested), Some(grid)) =
            (self.displayed, self.frame, &self.grid)
            && displayed != requested
        {
            ui.small(format!(
                "{}: {}",
                text(
                    pt,
                    "Showing previous inundation",
                    "Exibindo inundação anterior"
                ),
                local_timestamp(grid.meta.timeline.bounds[displayed].end, pt)
            ));
        }
        if self.finding.is_none()
            && (self.previous.is_some() || (self.frame.is_some() && self.displayed != self.frame))
        {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(text(pt, "Loading inundation", "Carregando inundação"));
            });
        }
        match self.finding.as_ref() {
            Some(Finding::Missing) => {
                ui.label(text(
                    pt,
                    "No inundation mask for this scenario. Use Refresh (F5) after generating it.",
                    "Este cenário não tem máscara de inundação. Use Atualizar (F5) após gerá-la.",
                ));
            }
            Some(Finding::Invalid(error)) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    text(
                        pt,
                        "Inundation mask unavailable",
                        "Máscara de inundação indisponível",
                    ),
                )
                .on_hover_text(error);
            }
            None if self.grid.is_none() && self.previous.is_none() => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(text(pt, "Loading inundation", "Carregando inundação"));
                });
            }
            None => {}
        }
    }

    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        projector: &Projector,
        rect: egui::Rect,
        zoom: f64,
        opacity: f32,
        tint: Color32,
    ) {
        let clip = ui.clip_rect().intersect(rect);
        if !clip.is_positive() {
            return;
        }
        self.viewport_seen = true;
        let world_width = 256. * 2f64.powf(zoom) * f64::from(ui.ctx().pixels_per_point());
        if let Some(grid) = &self.grid
            && let Some(visible) = grid.visible_tiles(projector, clip, world_width)
        {
            self.truncated |= visible.truncated;
            for (tile, distance) in visible.tiles {
                self.demand
                    .entry(tile)
                    .and_modify(|old| *old = old.min(distance))
                    .or_insert(distance);
            }
        }
        let (grid, frame, cache) = if let Some(previous) = &mut self.previous {
            (&previous.grid, previous.frame, &mut previous.cache)
        } else if let (Some(grid), Some(frame)) = (&self.grid, self.displayed) {
            (grid, frame, &mut self.cache)
        } else {
            return;
        };
        let Some(visible) = grid.visible_tiles(projector, clip, world_width) else {
            return;
        };
        let painter = ui.painter().with_clip_rect(clip);
        let transform = map::ScreenTransform::new(projector, rect, zoom);
        let color = tint.gamma_multiply(opacity);
        for (tile, _) in visible.tiles {
            let Some(cached) = cache.get_mut(&(frame, tile)) else {
                continue;
            };
            cached.used = self.pass;
            if let Some(texture) = &cached.texture {
                let geometry = &texture.geometry;
                painter.add(egui::Shape::mesh(egui::Mesh {
                    texture_id: texture.handle.id(),
                    indices: geometry.indices.clone(),
                    vertices: geometry
                        .vertices
                        .iter()
                        .map(|&(point, uv)| egui::epaint::Vertex {
                            pos: transform.project(point),
                            uv,
                            color,
                        })
                        .collect(),
                }));
            }
        }
    }

    pub fn finish_view(&mut self, context: &egui::Context, visible: bool) {
        if self.worker.in_flight {
            match self.worker.replies.try_recv() {
                Ok((revision, loaded)) => {
                    self.worker.in_flight = false;
                    if revision == self.worker.epoch.load(Ordering::Relaxed) {
                        self.receive(loaded, context);
                        context.request_repaint();
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.worker.in_flight = false;
                    self.finding = Some(Finding::Invalid("Inundation worker has stopped".into()));
                    context.request_repaint();
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if visible
            && self.finding.is_none()
            && let Some(frame) = self.frame
            && self.grid.is_some()
            && self.viewport_seen
            && self
                .demand
                .keys()
                .all(|tile| self.cache.contains_key(&(frame, *tile)))
            && (self.displayed != Some(frame) || self.previous.is_some())
        {
            self.displayed = Some(frame);
            self.previous = None;
            context.request_repaint();
        }
        if visible && !self.worker.in_flight && self.finding.is_none() {
            let job = match (&self.source, &self.grid, self.frame) {
                (Some(source), None, _) => Some(Job::Open(source.clone())),
                (Some(_), Some(_), Some(frame)) => {
                    let mut tiles: Vec<_> = self
                        .demand
                        .iter()
                        .filter(|(tile, _)| !self.cache.contains_key(&(frame, **tile)))
                        .collect();
                    tiles.sort_unstable_by(|(a, ad), (b, bd)| {
                        ad.total_cmp(bd).then_with(|| (a.y, a.x).cmp(&(b.y, b.x)))
                    });
                    (!tiles.is_empty()).then(|| Job::Tiles {
                        frame,
                        tiles: tiles
                            .into_iter()
                            .take(BATCH_SIZE)
                            .map(|(&t, _)| t)
                            .collect(),
                    })
                }
                _ => None,
            };
            if let Some(job) = job
                && let Err(error) = self.worker.submit(job)
            {
                self.finding = Some(error);
                context.request_repaint();
            }
        }
        if self.previous_truncated != self.truncated {
            self.previous_truncated = self.truncated;
            context.request_repaint();
        }
        self.truncated = false;
        self.viewport_seen = false;
        self.demand.clear();
        self.pass += 1;
    }

    fn receive(&mut self, loaded: Loaded, context: &egui::Context) {
        match loaded {
            Loaded::Open(Ok(grid)) => {
                let last = grid.meta.timeline.bounds.len() - 1;
                let frame = self.frame.unwrap_or(grid.meta.historical_count).min(last);
                self.frame = Some(frame);
                self.grid = Some(grid);
            }
            Loaded::Open(Err(error)) => self.finding = Some(error),
            Loaded::Tiles(tiles) => {
                let Some(frame) = self.frame else {
                    return;
                };
                for (tile, result) in tiles {
                    if !self.demand.contains_key(&tile) {
                        continue;
                    }
                    match result {
                        Ok(tile_image) => {
                            let texture = tile_image.map(|tile_image| Texture {
                                handle: context.load_texture(
                                    "modeled-inundation",
                                    tile_image.image,
                                    egui::TextureOptions::NEAREST,
                                ),
                                geometry: tile_image.geometry,
                            });
                            self.cache.insert(
                                (frame, tile),
                                CachedTile {
                                    texture,
                                    used: self.pass,
                                },
                            );
                        }
                        Err(error) => {
                            self.finding = Some(Finding::Invalid(error));
                            break;
                        }
                    }
                }
                while self.cache.len() > CACHE_SIZE {
                    let Some((&key, _)) = self
                        .cache
                        .iter()
                        .filter(|((frame, tile), _)| {
                            !(self.demand.contains_key(tile)
                                && (Some(*frame) == self.frame || Some(*frame) == self.displayed))
                        })
                        .min_by_key(|(_, cached)| cached.used)
                    else {
                        break;
                    };
                    self.cache.remove(&key);
                }
            }
        }
    }
}

fn local_timestamp(value: chrono::NaiveDateTime, pt: bool) -> String {
    use chrono::Timelike;
    let format = match (pt, value.second() != 0) {
        (false, false) => "%Y-%m-%d %H:%M",
        (false, true) => "%Y-%m-%d %H:%M:%S",
        (true, false) => "%d/%m/%Y %H:%M",
        (true, true) => "%d/%m/%Y %H:%M:%S",
    };
    value.format(format).to_string()
}

fn calendar_picker(
    ui: &mut egui::Ui,
    selected: chrono::NaiveDate,
    month: &mut Option<chrono::NaiveDate>,
    bounds: &[mgb_ops::store::TimeInterval],
    pt: bool,
) -> Option<chrono::NaiveDate> {
    use chrono::Datelike;
    let first = bounds.first()?.end.date().with_day(1)?;
    let last = bounds.last()?.end.date().with_day(1)?;
    let current = month.get_or_insert(selected.with_day(1)?);
    *current = (*current).clamp(first, last);
    let months = if pt {
        [
            "Janeiro",
            "Fevereiro",
            "Março",
            "Abril",
            "Maio",
            "Junho",
            "Julho",
            "Agosto",
            "Setembro",
            "Outubro",
            "Novembro",
            "Dezembro",
        ]
    } else {
        [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ]
    };
    let weekdays = if pt {
        ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"]
    } else {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    };
    let mut choice = None;
    let response = ui.menu_button(
        selected
            .format(if pt { "%d/%m/%Y" } else { "%Y-%m-%d" })
            .to_string(),
        |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(*current > first, egui::Button::new("←"))
                    .clicked()
                    && let Some(previous) = current.checked_sub_months(chrono::Months::new(1))
                {
                    *current = previous;
                }
                ui.label(format!(
                    "{} {}",
                    months[current.month0() as usize],
                    current.year()
                ));
                if ui
                    .add_enabled(*current < last, egui::Button::new("→"))
                    .clicked()
                    && let Some(next) = current.checked_add_months(chrono::Months::new(1))
                {
                    *current = next;
                }
            });
            egui::Grid::new("inundation-calendar").show(ui, |ui| {
                for day in weekdays {
                    ui.label(day);
                }
                ui.end_row();
                let offset = current.weekday().num_days_from_monday();
                for slot in 0..42 {
                    let day = slot as i64 - i64::from(offset) + 1;
                    if day > 0
                        && let Some(date) =
                            current.checked_add_signed(chrono::Duration::days(day - 1))
                        && date.month() == current.month()
                    {
                        let available = bounds.iter().any(|bound| bound.end.date() == date);
                        if ui
                            .add_enabled(
                                available,
                                egui::Button::selectable(date == selected, day.to_string()),
                            )
                            .clicked()
                        {
                            choice = Some(date);
                            ui.close();
                        }
                    } else {
                        ui.label("");
                    }
                    if slot % 7 == 6 {
                        ui.end_row();
                    }
                }
            });
        },
    );
    if response.response.clicked() {
        *month = selected.with_day(1);
    }
    choice
}

fn frame_for_date(
    bounds: &[mgb_ops::store::TimeInterval],
    date: chrono::NaiveDate,
    time: chrono::NaiveTime,
) -> Option<usize> {
    bounds
        .iter()
        .enumerate()
        .filter(|(_, bound)| bound.end.date() == date)
        .min_by_key(|(_, bound)| (bound.end.time() - time).num_seconds().unsigned_abs())
        .map(|(index, _)| index)
}

fn hour_slider(ui: &mut egui::Ui, frame: &mut usize, last: usize, pt: bool) -> egui::Response {
    let response = ui.add(
        egui::Slider::new(frame, 0..=last)
            .step_by(1.)
            .text(text(pt, "Hour", "Hora"))
            .show_value(false),
    );
    if response.drag_started() || response.clicked() {
        response.request_focus();
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, NaiveDate};
    use mgb_ops::store::{IntervalAxis, TimeInterval};
    use std::time::{Duration as Wait, Instant};
    use walkers::{MapMemory, lon_lat};

    fn timestamp(value: chrono::NaiveDateTime) -> String {
        value.format("%Y%m%d-%H").to_string()
    }

    fn transformer(from: u32, to: u32) -> Transformer {
        Transformer::new(
            &SpatialRef::from_epsg(from).unwrap(),
            &SpatialRef::from_epsg(to).unwrap(),
        )
        .unwrap()
    }

    fn grid(epsg: i32) -> Grid {
        let start = NaiveDate::from_ymd_opt(2026, 11, 21)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        Grid {
            meta: InundationMetadata {
                width: 7005,
                height: 8009,
                epsg,
                origin_x: -52.5,
                origin_y: -29.,
                pixel_width: 0.00025,
                pixel_height: -0.00025,
                timeline: IntervalAxis {
                    time_zone: "America/Sao_Paulo".into(),
                    bounds: (0..10)
                        .map(|hour| TimeInterval {
                            start: start + Duration::hours(hour),
                            end: start + Duration::hours(hour + 1),
                        })
                        .collect(),
                },
                historical_count: 3,
                provenance: InundationProvenance {
                    run_manifest_sha256: "0".repeat(64),
                    scenario: "test".into(),
                    hand_sha256: "1".repeat(64),
                    catchments_sha256: "2".repeat(64),
                    flood_source_sha256: "3".repeat(64),
                    hand_sidecars: vec![],
                    catchments_sidecars: vec![],
                    vertical_datum: "Z0".into(),
                    hand_vertical_shift_m: 0,
                    include_nonpositive_hand: true,
                    hand_in_decimetres: true,
                },
            },
            to_source: transformer(4326, epsg as u32),
        }
    }

    #[test]
    fn native_and_overview_tiles_follow_viewport_scale_and_clipping() {
        let grid = grid(4618);
        let center = lon_lat(-51.7, -30.);
        let rect = egui::Rect::from_min_size(egui::pos2(57., 13.), egui::vec2(1170., 710.));
        let mut memory = MapMemory::default();
        for (zoom, expected) in [
            (8., InundationLevel::Overview16),
            (12., InundationLevel::Overview4),
            (14., InundationLevel::Full),
        ] {
            memory.set_zoom(zoom).unwrap();
            let projector = Projector::new(rect, &memory, center);
            let view = grid
                .visible_tiles(&projector, rect, 256. * 2f64.powf(zoom))
                .unwrap();
            let complete = !view.truncated;
            assert!(complete);
            assert!(view.tiles.iter().all(|(tile, _)| tile.level == expected));
            for &(tile, _) in &view.tiles {
                let (x, y, width, height) = tile.window(&grid.meta);
                let dimensions = grid.meta.dimensions(tile.level);
                assert!(x + width <= dimensions.0 && y + height <= dimensions.1);
                assert!(width <= TILE_SIZE && height <= TILE_SIZE);
            }
            if zoom == 14. {
                let narrow = egui::Rect::from_min_max(
                    rect.left_top(),
                    rect.left_top() + egui::vec2(30., 30.),
                );
                let clipped = grid
                    .visible_tiles(&projector, narrow, 256. * 2f64.powf(zoom))
                    .unwrap();
                assert!(clipped.tiles.len() < view.tiles.len());
                assert!(
                    clipped
                        .tiles
                        .iter()
                        .all(|(key, _)| { view.tiles.iter().any(|(full_key, _)| key == full_key) })
                );
                let outside = Projector::new(rect, &memory, lon_lat(0., 0.));
                assert!(
                    grid.visible_tiles(&outside, rect, 256. * 2f64.powf(zoom))
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn projected_mesh_preserves_outside_corner_and_partial_last_cell() {
        for epsg in [4326, 4618] {
            let grid = grid(epsg);
            let meta = &grid.meta;
            let level = InundationLevel::Overview16;
            let dimensions = meta.dimensions(level);
            let tile = Tile {
                level,
                x: dimensions.0.div_ceil(TILE_SIZE) - 1,
                y: dimensions.1.div_ceil(TILE_SIZE) - 1,
            };
            let to_wgs84 = transformer(epsg as u32, 4326);
            let geometry = tile_geometry(meta, &to_wgs84, tile).unwrap();
            let corner = |x, y| {
                let lon = meta.origin_x + x as f64 * meta.pixel_width;
                let lat = meta.origin_y + y as f64 * meta.pixel_height;
                map::world(to_wgs84.transform(lon, lat).unwrap())
            };
            let (x, y, width, height) = tile.window(meta);
            let factor = level.factor();
            assert_eq!(
                geometry.vertices[0],
                (corner(x * factor, y * factor), egui::Pos2::ZERO)
            );
            assert_eq!(
                *geometry.vertices.last().unwrap(),
                (corner(meta.width, meta.height), egui::pos2(1., 1.))
            );
            assert!(geometry.vertices.contains(&(
                corner((x + width - 1) * factor, (y + height - 1) * factor),
                egui::pos2(
                    (width - 1) as f32 / width as f32,
                    (height - 1) as f32 / height as f32
                ),
            )));
        }
        assert_eq!(tile_color(InundationLevel::Full, 0), Color32::TRANSPARENT);
        assert_eq!(tile_color(InundationLevel::Full, 1), Color32::WHITE);
        assert_eq!(
            tile_color(InundationLevel::Overview4, 255),
            Color32::TRANSPARENT
        );
        assert_eq!(tile_color(InundationLevel::Overview16, 50).a(), 127);
    }

    #[test]
    fn forecast_starts_at_retained_boundary_and_slider_steps_hourly() {
        let context = egui::Context::default();
        let mut state = Inundation::new(context.clone());
        state.receive(Loaded::Open(Ok(Box::new(grid(4326)))), &context);
        let grid = state.grid.as_ref().unwrap();
        let boundary = grid.meta.historical_count;
        assert_eq!(state.frame, Some(boundary));
        assert_eq!(
            timestamp(grid.meta.timeline.bounds[boundary].start),
            "20261121-12"
        );
        assert_eq!(
            timestamp(grid.meta.timeline.bounds[boundary].end),
            "20261121-13"
        );

        assert_eq!(
            frame_for_date(
                &grid.meta.timeline.bounds,
                chrono::NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
                chrono::NaiveTime::from_hms_opt(13, 0, 0).unwrap()
            ),
            Some(boundary)
        );
        assert_eq!(
            frame_for_date(
                &grid.meta.timeline.bounds,
                chrono::NaiveDate::from_ymd_opt(2026, 12, 21).unwrap(),
                chrono::NaiveTime::from_hms_opt(13, 0, 0).unwrap()
            ),
            None
        );

        let mut frame = boundary;
        let mut pass = |events, focus| {
            context
                .run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        let slider = hour_slider(ui, &mut frame, 500, false);
                        if focus {
                            slider.request_focus();
                        }
                    },
                )
                .textures_delta
                .clear();
            frame
        };
        pass(vec![], true);
        let arrow = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let forward = pass(vec![arrow(egui::Key::ArrowRight)], false);
        let backward = pass(vec![arrow(egui::Key::ArrowLeft)], false);
        assert_eq!(forward, boundary + 1);
        assert_eq!(backward, boundary);
    }

    #[test]
    fn old_run_missing_mask_and_corrupt_file_are_local_findings_and_refresh_retries() {
        let directory = tempfile::tempdir().unwrap();
        let context = egui::Context::default();
        let mut state = Inundation::new(context.clone());
        let source = Source {
            selection: None,
            root: directory.path().into(),
            scenario: "old".into(),
        };
        let path = source.path();
        state.select(Some(source));
        let wait_for_open = |state: &mut Inundation| {
            state.finish_view(&context, true);
            let deadline = Instant::now() + Wait::from_secs(5);
            while state.worker.in_flight && Instant::now() < deadline {
                thread::sleep(Wait::from_millis(5));
                state.finish_view(&context, true);
            }
            let done = !state.worker.in_flight;
            assert!(done);
        };
        wait_for_open(&mut state);
        let missing = matches!(state.finding, Some(Finding::Missing));
        assert!(missing);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"not NetCDF").unwrap();
        state.refresh();
        wait_for_open(&mut state);
        let invalid = matches!(state.finding, Some(Finding::Invalid(_)));
        assert!(invalid);
        assert!(state.grid.is_none());
    }

    #[test]
    fn a_same_named_scenario_requires_the_selected_manifest_digest() {
        let directory = tempfile::tempdir().unwrap();
        let source = Source {
            selection: None,
            root: directory.path().into(),
            scenario: "test".into(),
        };
        let manifest = source.root.join("manifest.toml");
        std::fs::write(&manifest, b"first manifest").unwrap();
        let mut provenance = grid(4326).meta.provenance;
        provenance.run_manifest_sha256 = digest_file(&manifest).unwrap().0;
        assert!(source.verify_provenance(&provenance).is_ok());
        std::fs::write(manifest, b"other manifest").unwrap();
        assert!(
            source
                .verify_provenance(&provenance)
                .unwrap_err()
                .contains("different run manifest")
        );
    }

    #[test]
    fn frame_source_and_viewport_changes_discard_stale_work() {
        let context = egui::Context::default();
        let mut state = Inundation::new(context.clone());
        let (jobs, pending) = mpsc::channel();
        let (ready, replies) = mpsc::channel();
        state.worker = Worker {
            jobs,
            replies,
            epoch: Arc::new(AtomicU64::new(0)),
            in_flight: false,
        };
        let source = Source {
            selection: None,
            root: PathBuf::from("run-a"),
            scenario: "test".into(),
        };
        state.select(Some(source.clone()));
        state.finish_view(&context, true);
        let closing = matches!(pending.try_recv().unwrap().1, Job::Close);
        assert!(closing);
        let (opening, _) = pending.try_recv().unwrap();
        state.select(Some(Source {
            selection: None,
            root: PathBuf::from("run-b"),
            ..source
        }));
        state.finish_view(&context, true);
        let closing = matches!(pending.try_recv().unwrap().1, Job::Close);
        assert!(closing);
        assert!(pending.try_recv().is_err());
        ready
            .send((opening, Loaded::Open(Ok(Box::new(grid(4326))))))
            .unwrap();
        state.finish_view(&context, true);
        assert!(state.grid.is_none());
        let (opening, _) = pending.try_recv().unwrap();
        ready
            .send((opening, Loaded::Open(Ok(Box::new(grid(4618))))))
            .unwrap();
        state.finish_view(&context, true);
        assert_eq!(state.grid.as_ref().map(|grid| grid.meta.epsg), Some(4618));
        let tile = Tile {
            level: InundationLevel::Full,
            x: 0,
            y: 0,
        };
        state.demand.insert(tile, 0.);
        state.finish_view(&context, true);
        let (reading, job) = pending.try_recv().unwrap();
        let old_frame = matches!(job, Job::Tiles { frame: 3, .. });
        assert!(old_frame);
        state.change_frame(4);
        state.demand.insert(tile, 0.);
        ready
            .send((reading, Loaded::Tiles(vec![(tile, Ok(None))])))
            .unwrap();
        state.finish_view(&context, true);
        assert!(state.cache.is_empty());
        let (reading, job) = pending.try_recv().unwrap();
        let new_frame = matches!(job, Job::Tiles { frame: 4, .. });
        assert!(new_frame);
        ready
            .send((reading, Loaded::Tiles(vec![(tile, Ok(None))])))
            .unwrap();
        state.finish_view(&context, true);
        assert!(
            state.cache.is_empty(),
            "request fell outside the new viewport"
        );
    }
    #[test]
    fn visible_tiles_swap_together_and_failure_keeps_displayed_frame() {
        let context = egui::Context::default();
        let mut state = Inundation::new(context.clone());
        let (jobs, _pending) = mpsc::channel();
        let (_ready, replies) = mpsc::channel();
        state.worker = Worker {
            jobs,
            replies,
            epoch: Arc::new(AtomicU64::new(0)),
            in_flight: true,
        };
        state.source = Some(Source {
            selection: None,
            root: PathBuf::from("run"),
            scenario: "test".into(),
        });
        state.grid = Some(Box::new(grid(4326)));
        state.frame = Some(4);
        state.displayed = Some(3);
        let left = Tile {
            level: InundationLevel::Full,
            x: 0,
            y: 0,
        };
        let right = Tile { x: 1, ..left };
        state.viewport_seen = true;
        state.demand.extend([(left, 0.), (right, 0.)]);
        state.cache.insert(
            (4, left),
            CachedTile {
                texture: None,
                used: 0,
            },
        );
        state.finish_view(&context, true);
        assert_eq!(state.displayed, Some(3));
        state.viewport_seen = true;
        state.demand.extend([(left, 0.), (right, 0.)]);
        state.cache.insert(
            (4, right),
            CachedTile {
                texture: None,
                used: 0,
            },
        );
        state.finish_view(&context, true);
        assert_eq!(
            state.displayed,
            Some(4),
            "transparent tiles count as completed"
        );
        state.change_frame(5);
        state.finding = Some(Finding::Invalid("read failed".into()));
        state.demand.insert(left, 0.);
        state.finish_view(&context, true);
        assert_eq!(state.displayed, Some(4));
    }

    #[test]
    fn replacing_a_source_retains_old_identity_until_ready() {
        let context = egui::Context::default();
        let mut state = Inundation::new(context);
        let old = Source {
            selection: None,
            root: PathBuf::from("old"),
            scenario: "first".into(),
        };
        state.source = Some(old.clone());
        state.grid = Some(Box::new(grid(4326)));
        state.frame = Some(3);
        state.displayed = Some(3);
        state.select(Some(Source {
            root: PathBuf::from("new"),
            scenario: "second".into(),
            ..old.clone()
        }));
        assert!(
            state
                .previous
                .as_ref()
                .is_some_and(|previous| previous.source == old)
        );
        assert_eq!(
            state.previous.as_ref().map(|previous| previous.frame),
            Some(3)
        );
        state.select(None);
        assert!(state.previous.is_none());
    }
    #[test]
    fn empty_visible_set_completes_only_after_a_viewport_was_drawn() {
        let context = egui::Context::default();
        let mut state = Inundation::new(context.clone());
        state.grid = Some(Box::new(grid(4326)));
        state.frame = Some(4);
        state.displayed = Some(3);
        state.finish_view(&context, true);
        assert_eq!(state.displayed, Some(3));
        state.viewport_seen = true;
        state.finish_view(&context, true);
        assert_eq!(state.displayed, Some(4));
    }

    #[test]
    fn failed_visible_tile_stays_failed_without_automatic_retry() {
        let context = egui::Context::default();
        let mut state = Inundation::new(context.clone());
        let (jobs, pending) = mpsc::channel();
        let (ready, replies) = mpsc::channel();
        state.worker = Worker {
            jobs,
            replies,
            epoch: Arc::new(AtomicU64::new(0)),
            in_flight: true,
        };
        state.grid = Some(Box::new(grid(4326)));
        state.frame = Some(4);
        state.displayed = Some(3);
        let tile = Tile {
            level: InundationLevel::Full,
            x: 0,
            y: 0,
        };
        state.demand.insert(tile, 0.);
        state.viewport_seen = true;
        ready
            .send((0, Loaded::Tiles(vec![(tile, Err("failure".into()))])))
            .unwrap();
        state.finish_view(&context, true);
        let failed = matches!(state.finding, Some(Finding::Invalid(_)));
        assert!(failed);
        assert_eq!(state.displayed, Some(3));
        state.demand.insert(tile, 0.);
        state.viewport_seen = true;
        state.finish_view(&context, true);
        let failed = matches!(state.finding, Some(Finding::Invalid(_)));
        assert!(failed);
        assert!(pending.try_recv().is_err());
    }
}
