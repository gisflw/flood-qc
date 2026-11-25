//! Bounded, cancellable catchment mesh preparation outside the frame thread.
use super::{OverlayMesh, PreparedMini, ScreenTransform};
use eframe::egui::{self, Color32, Stroke};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicU64, Ordering},
};

const CHUNK_VERTICES: usize = 16_384;
const PATH_POINTS: usize = 2_048;

#[derive(Clone, Copy, PartialEq)]
struct Key {
    geometry: usize,
    transform: ScreenTransform,
    clip: egui::Rect,
    zoom: f64,
    pixels_per_point: f32,
    style: (f32, Color32, f32),
}
struct Request {
    key: Key,
    geometry: Arc<Vec<PreparedMini>>,
    epoch: u64,
}
struct Ready {
    key: Key,
    meshes: Vec<OverlayMesh>,
    epoch: u64,
}
#[derive(Default)]
struct State {
    pending: Option<Request>,
    ready: Option<Ready>,
    stopping: bool,
}
struct Worker {
    shared: Arc<(Mutex<State>, Condvar)>,
    epoch: Arc<AtomicU64>,
}
impl Worker {
    fn new(context: egui::Context) -> Self {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let epoch = Arc::new(AtomicU64::new(0));
        let worker_shared = shared.clone();
        let worker_epoch = epoch.clone();
        std::thread::spawn(move || {
            loop {
                let (lock, wake) = &*worker_shared;
                let mut state = lock.lock().unwrap();
                while !state.stopping && state.pending.is_none() {
                    state = wake.wait(state).unwrap();
                }
                if state.stopping {
                    break;
                }
                let request = state.pending.take().unwrap();
                drop(state);
                if let Some(meshes) = build(&request, &worker_epoch) {
                    let mut state = lock.lock().unwrap();
                    if request.epoch == worker_epoch.load(Ordering::Relaxed) && !state.stopping {
                        state.ready = Some(Ready {
                            key: request.key,
                            epoch: request.epoch,
                            meshes,
                        });
                        drop(state);
                        context.request_repaint();
                    }
                }
            }
        });
        Self { shared, epoch }
    }
    fn cancel(&self) {
        self.epoch.fetch_add(1, Ordering::Relaxed);
        let mut state = self.shared.0.lock().unwrap();
        state.pending = None;
        state.ready = None;
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        self.shared.0.lock().unwrap().stopping = true;
        self.shared.1.notify_one();
        // No UI-thread join: the cancelled worker releases its geometry at the
        // next bounded chunk, independently of closing or switching a project.
    }
}

#[derive(Default)]
pub(super) struct Renderer {
    worker: Option<Worker>,
    requested: Option<Key>,
    ready: Option<Ready>,
}
impl Renderer {
    pub fn clear(&mut self) {
        if self.requested.is_some()
            && let Some(worker) = &self.worker
        {
            worker.cancel();
        }
        self.requested = None;
        self.ready = None;
    }
    pub fn prepare(
        &mut self,
        geometry: &Arc<Vec<PreparedMini>>,
        transform: ScreenTransform,
        clip: egui::Rect,
        zoom: f64,
        style: (f32, Color32, f32),
        context: &egui::Context,
    ) {
        let key = Key {
            geometry: Arc::as_ptr(geometry) as usize,
            transform,
            clip,
            zoom,
            style,
            pixels_per_point: context.pixels_per_point(),
        };
        let worker = self
            .worker
            .get_or_insert_with(|| Worker::new(context.clone()));
        if self.requested != Some(key) {
            let epoch = worker.epoch.fetch_add(1, Ordering::Relaxed) + 1;
            let mut state = worker.shared.0.lock().unwrap();
            state.ready = None;
            state.pending = Some(Request {
                key,
                geometry: geometry.clone(),
                epoch,
            });
            worker.shared.1.notify_one();
            self.requested = Some(key);
            self.ready = None;
        }
        let ready = worker.shared.0.lock().unwrap().ready.take();
        if let Some(ready) = ready
            && Some(ready.key) == self.requested
            && ready.epoch == worker.epoch.load(Ordering::Relaxed)
        {
            self.ready = Some(ready);
        }
    }
    pub fn paint(&mut self, ui: &egui::Ui, offset: egui::Vec2, pt: bool) {
        if let Some(ready) = &mut self.ready {
            for mesh in &mut ready.meshes {
                mesh.paint(ui, offset);
            }
        } else if self.requested.is_some() {
            ui.painter().text(
                ui.clip_rect().left_top() + egui::vec2(12., 12.),
                egui::Align2::LEFT_TOP,
                super::super::text(pt, "Preparing catchments…", "Preparando bacias…"),
                egui::FontId::proportional(12.),
                ui.visuals().text_color(),
            );
        }
    }
}

struct Meshes {
    current: egui::Mesh,
    ready: Vec<OverlayMesh>,
}
impl Meshes {
    fn new() -> Self {
        Self {
            current: egui::Mesh::default(),
            ready: Vec::new(),
        }
    }
    fn flush(&mut self) {
        if !self.current.is_empty() {
            self.ready
                .push(OverlayMesh::new(std::mem::take(&mut self.current)));
        }
    }
    fn append(&mut self, mesh: egui::Mesh) {
        if self.current.vertices.len() + mesh.vertices.len() > CHUNK_VERTICES {
            self.flush();
        }
        self.current.append(mesh);
    }
}

fn build(request: &Request, epoch: &AtomicU64) -> Option<Vec<OverlayMesh>> {
    let current = || epoch.load(Ordering::Relaxed) == request.epoch;
    let key = &request.key;
    let tint = key.style.1.gamma_multiply(key.style.0);
    let mut meshes = Meshes::new();
    let mut tessellator = egui::epaint::Tessellator::new(
        key.pixels_per_point,
        egui::epaint::TessellationOptions::default(),
        [1, 1],
        Vec::new(),
    );
    tessellator.set_clip_rect(key.clip);
    for feature in request.geometry.iter() {
        if !current() {
            return None;
        }
        if !feature
            .projected_bounds
            .intersects(&key.transform, key.clip)
        {
            continue;
        }
        let (fill, paths) = feature.detail(key.zoom, key.pixels_per_point);
        if fill.projected.len() <= CHUNK_VERTICES {
            meshes.append(super::mesh(fill, &key.transform, tint));
        } else {
            // Large individual polygons are split as well as batches of small
            // features, so cancellation and panning never depend on a giant mesh.
            for indices in fill.indices.chunks(3 * (CHUNK_VERTICES / 3)) {
                if !current() {
                    return None;
                }
                let mut mesh = egui::Mesh::default();
                for &index in indices {
                    mesh.indices.push(mesh.vertices.len() as u32);
                    mesh.colored_vertex(
                        key.transform.project(fill.projected[index as usize]),
                        tint,
                    );
                }
                meshes.append(mesh);
            }
        }
        if key.style.2 <= 0. {
            continue;
        }
        for path in paths
            .iter()
            .filter(|path| path.bounds.intersects(&key.transform, key.clip))
        {
            for start in (0..path.points.len().saturating_sub(1)).step_by(PATH_POINTS - 1) {
                if !current() {
                    return None;
                }
                let end = (start + PATH_POINTS).min(path.points.len());
                let points = path.points[start..end]
                    .iter()
                    .map(|point| key.transform.project(*point))
                    .collect();
                let mut mesh = egui::Mesh::default();
                tessellator.tessellate_shape(
                    egui::Shape::line(points, Stroke::new(key.style.2, tint)),
                    &mut mesh,
                );
                meshes.append(mesh);
            }
        }
    }
    meshes.flush();
    current().then_some(meshes.ready)
}

#[cfg(test)]
mod tests {
    use super::super::{MapPolygon, MiniMapFeature, prepare_catchments, world};
    use super::*;
    use std::time::{Duration, Instant};

    fn geometry(count: usize, vertices: usize) -> Arc<Vec<PreparedMini>> {
        let features: Vec<_> = (0..count)
            .map(|index| {
                let x = (index % 100) as f64 / 10.;
                let y = (index / 100) as f64 / 10.;
                let mut ring: Vec<_> = (0..vertices)
                    .map(|index| {
                        let angle = index as f64 * std::f64::consts::TAU / vertices as f64;
                        (x + 0.04 * angle.cos(), y + 0.04 * angle.sin())
                    })
                    .collect();
                ring.push(ring[0]);
                MiniMapFeature {
                    mini_id: index as i64,
                    paths: vec![ring.clone()],
                    polygons: vec![MapPolygon {
                        exterior: ring,
                        interiors: Vec::new(),
                    }],
                }
            })
            .collect();
        Arc::new(prepare_catchments(&features).unwrap())
    }
    fn key() -> Key {
        Key {
            geometry: 0,
            transform: ScreenTransform {
                center: world((5., 5.)),
                origin: egui::pos2(500., 500.),
                scale: 8192.,
            },
            clip: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(1000.)),
            zoom: 5.,
            pixels_per_point: 1.,
            style: (0.25, Color32::GREEN, 1.),
        }
    }
    #[test]
    fn display_details_keep_polygon_holes_and_visible_boundary_strokes() {
        let exterior = vec![(0., 0.), (4., 0.), (4., 4.), (0., 4.), (0., 0.)];
        let hole = vec![(1., 1.), (3., 1.), (3., 3.), (1., 3.), (1., 1.)];
        let interiors = vec![hole.clone()];
        let feature = MiniMapFeature {
            mini_id: 1,
            paths: vec![exterior.clone(), hole.clone()],
            polygons: vec![MapPolygon {
                exterior,
                interiors,
            }],
        };
        let geometry = Arc::new(prepare_catchments(&[feature]).unwrap());
        let mut key = key();
        key.transform.center = world((2., 2.));
        let center = key.transform.project(world((2., 2.)));
        let shell = key.transform.project(world((0.5, 0.5)));
        for zoom in [3., 5., 7., 9., 11., 12., 15.] {
            key.zoom = zoom;
            key.style.2 = 0.;
            let request = Request {
                key,
                geometry: geometry.clone(),
                epoch: 0,
            };
            let fill = build(&request, &AtomicU64::new(0)).unwrap();
            let covers = |point: egui::Pos2| {
                fill.iter().any(|chunk| {
                    chunk
                        .anchor
                        .indices
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .any(|indices| {
                            let points = [indices[0], indices[1], indices[2]]
                                .map(|index| chunk.anchor.vertices[index as usize].pos);
                            let signs: Vec<_> = (0..3)
                                .map(|index| {
                                    let a = points[index];
                                    let b = points[(index + 1) % 3];
                                    (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x)
                                })
                                .collect();
                            signs.iter().all(|value| *value >= 0.)
                                || signs.iter().all(|value| *value <= 0.)
                        })
                })
            };
            let hole_is_empty = !covers(center);
            assert!(hole_is_empty, "a displayed hole is filled at zoom {zoom}");
            assert!(covers(shell), "the polygon shell is missing at zoom {zoom}");
            key.style.2 = 1.;
            let outlines = build(&Request { key, ..request }, &AtomicU64::new(0)).unwrap();
            assert!(
                outlines
                    .iter()
                    .map(|mesh| mesh.anchor.indices.len())
                    .sum::<usize>()
                    > fill
                        .iter()
                        .map(|mesh| mesh.anchor.indices.len())
                        .sum::<usize>()
            );
        }
    }
    #[test]
    fn preparation_cancels_and_does_not_publish_a_hidden_or_superseded_layer() {
        let geometry = geometry(25, 80);
        let ctx = egui::Context::default();
        let key = key();
        let mut renderer = Renderer::default();
        renderer.prepare(
            &geometry,
            key.transform,
            key.clip,
            key.zoom,
            key.style,
            &ctx,
        );
        renderer.clear();
        assert!(renderer.requested.is_none());
        assert!(renderer.ready.is_none());
        let style = (0.6, Color32::BLUE, 2.);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            renderer.prepare(&geometry, key.transform, key.clip, key.zoom, style, &ctx);
            if renderer.ready.is_some() {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let ready = renderer.ready.as_ref().unwrap();
        assert_eq!(ready.key.style, style);
        let has_meshes = !ready.meshes.is_empty();
        assert!(has_meshes);
        assert!(
            ready
                .meshes
                .iter()
                .all(|mesh| mesh.anchor.is_valid() && mesh.anchor.vertices.len() <= CHUNK_VERTICES)
        );
    }
    #[test]
    #[ignore = "explicit regional-scale CPU rendering qualification"]
    fn regional_scale_preparation_keeps_frames_responsive() {
        for count in [12_000usize, 120_000, 1_200_000] {
            let mut ring: Vec<_> = (0..count)
                .map(|index| (index as f64 / count as f64, (index % 2) as f64 * 0.0001))
                .collect();
            ring.push(ring[0]);
            let started = Instant::now();
            assert_eq!(super::super::simplify_ring(&ring, 1e-10).len(), ring.len());
            eprintln!(
                "dense zigzag ring: {} points in {:?}",
                ring.len(),
                started.elapsed()
            );
        }
        let started = Instant::now();
        let geometry = geometry(10_000, 100);
        eprintln!(
            "prepared 10,000 catchments / 1,010,000 ring points in {:?}",
            started.elapsed()
        );
        let ctx = egui::Context::default();
        let key = key();
        let mut renderer = Renderer::default();
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut slowest = Duration::ZERO;
        let started = Instant::now();
        loop {
            let frame = Instant::now();
            renderer.prepare(
                &geometry,
                key.transform,
                key.clip,
                key.zoom,
                key.style,
                &ctx,
            );
            slowest = slowest.max(frame.elapsed());
            if renderer.ready.is_some() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background rendering did not complete"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        eprintln!(
            "background meshes: {:?}; slowest frame poll: {:?}; chunks: {}",
            started.elapsed(),
            slowest,
            renderer.ready.as_ref().unwrap().meshes.len()
        );
        assert!(slowest < Duration::from_millis(100));
        for offset in [egui::Vec2::ZERO, egui::vec2(20., 10.), egui::vec2(40., 20.)] {
            let started = Instant::now();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(key.clip),
                    ..Default::default()
                },
                |ui| {
                    renderer.paint(ui, offset, false);
                    let _ = ui.button("Hide catchments");
                },
            );
            let primitives = ctx.tessellate(std::mem::take(&mut output.shapes), 1.);
            output.textures_delta.clear();
            eprintln!(
                "paint/tessellate at {offset:?}: {:?}, {} primitives",
                started.elapsed(),
                primitives.len()
            );
            assert!(
                started.elapsed() < Duration::from_millis(100),
                "catchment paint blocks UI controls"
            );
        }
        renderer.clear();
        assert!(renderer.ready.is_none());
    }
}
