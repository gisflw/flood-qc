use super::text;
use eframe::egui::{self, Ui};
use mgb_ops::app::{
    CalibrationGeometry, CalibrationGeometryRow, ChannelDimension, GeometryEdit, GeometryField,
    SoilCalibration, SoilEdit,
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

enum Effect {
    Opened(Option<PathBuf>),
    Changed(usize),
    Saved(PathBuf),
}

struct Reply<T> {
    data: Option<T>,
    result: Result<Effect, String>,
}

struct Editor<T> {
    data: Option<T>,
    job: Option<mpsc::Receiver<Reply<T>>>,
    status: String,
    source: Option<PathBuf>,
    saved: Option<PathBuf>,
    dirty: bool,
}

impl<T> Default for Editor<T> {
    fn default() -> Self {
        Self {
            data: None,
            job: None,
            status: String::new(),
            source: None,
            saved: None,
            dirty: false,
        }
    }
}

impl<T: Send + 'static> Editor<T> {
    fn start(
        &mut self,
        ui: &Ui,
        work: impl FnOnce(&mut Option<T>) -> Result<Effect, String> + Send + 'static,
    ) {
        let (send, receive) = mpsc::channel();
        let ctx = ui.ctx().clone();
        let mut data = self.data.take();
        std::thread::spawn(move || {
            let result = work(&mut data);
            let _ = send.send(Reply { data, result });
            ctx.request_repaint();
        });
        self.job = Some(receive);
    }

    fn poll(&mut self, pt: bool) -> bool {
        let Some(job) = &self.job else { return false };
        match job.try_recv() {
            Ok(reply) => {
                self.job = None;
                self.data = reply.data;
                self.finish(reply.result, pt)
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.job = None;
                self.status = text(
                    pt,
                    "Calibration job stopped",
                    "Tarefa de calibração interrompida",
                );
                false
            }
        }
    }

    fn finish(&mut self, result: Result<Effect, String>, pt: bool) -> bool {
        match result {
            Ok(Effect::Opened(source)) => {
                self.source = source;
                self.saved = None;
                self.dirty = false;
                self.status.clear();
                true
            }
            Ok(Effect::Changed(n)) => {
                if n > 0 {
                    self.saved = None;
                    self.dirty = true;
                }
                self.status = format!("{} {n}", text(pt, "Rows updated", "Linhas alteradas"));
                false
            }
            Ok(Effect::Saved(path)) => {
                self.status.clear();
                self.saved = Some(path);
                self.dirty = false;
                false
            }
            Err(error) => {
                self.status = error;
                false
            }
        }
    }

    fn footer(&self, ui: &mut Ui, pt: bool) -> Option<PathBuf> {
        if self.job.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(text(pt, "Working…", "Em andamento…"));
            });
        }
        if self.dirty {
            ui.label(text(pt, "Unsaved changes", "Alterações não salvas"));
        }
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        let path = self.saved.as_ref()?;
        ui.label(path.display().to_string());
        ui.add_enabled(
            self.job.is_none(),
            egui::Button::new(text(pt, "Use in Draft", "Usar no rascunho")),
        )
        .clicked()
        .then(|| path.clone())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GeometryMode {
    Cell,
    Manning,
    Width,
    Depth,
}

struct GeometryFields {
    mode: GeometryMode,
    field: GeometryField,
    mini: usize,
    block: usize,
    selection: Option<(usize, GeometryField, f64)>,
    value: f64,
    manning: f64,
    subbasin: usize,
    use_subbasin: bool,
    a: f64,
    b: f64,
}

impl Default for GeometryFields {
    fn default() -> Self {
        Self {
            mode: GeometryMode::Cell,
            field: GeometryField::Width,
            mini: 1,
            block: 1,
            selection: None,
            value: 0.0,
            manning: 0.03,
            subbasin: 1,
            use_subbasin: false,
            a: 1.0,
            b: 0.5,
        }
    }
}

impl GeometryFields {
    fn show(&mut self, ui: &mut Ui, pt: bool, data: &CalibrationGeometry) -> Option<GeometryEdit> {
        let modes = [
            (GeometryMode::Cell, text(pt, "Edit a field", "Editar campo")),
            (
                GeometryMode::Manning,
                text(pt, "Manning coefficient", "Coeficiente de Manning"),
            ),
            (
                GeometryMode::Width,
                text(pt, "Width equation", "Equação de largura"),
            ),
            (
                GeometryMode::Depth,
                text(pt, "Depth equation", "Equação de profundidade"),
            ),
        ];
        let selected = modes.iter().find(|(mode, _)| *mode == self.mode).unwrap();
        egui::ComboBox::from_id_salt("geometry_mode")
            .selected_text(&selected.1)
            .show_ui(ui, |ui| {
                for (mode, label) in modes {
                    ui.selectable_value(&mut self.mode, mode, label);
                }
            });
        let edit = match self.mode {
            GeometryMode::Cell => {
                ui.horizontal_wrapped(|ui| {
                    ui.label("MINI");
                    ui.add(egui::DragValue::new(&mut self.mini).range(1..=data.rows().len()));
                    let fields = [
                        (GeometryField::Area, "Area"),
                        (GeometryField::UpstreamArea, "AreaM"),
                        (GeometryField::RiverLength, "Ltr"),
                        (GeometryField::RiverSlope, "Str"),
                        (GeometryField::LongestLength, "Lrl"),
                        (GeometryField::LongestSlope, "Srl"),
                        (GeometryField::Downstream, "MiniJus"),
                        (GeometryField::Order, "Ordem"),
                        (GeometryField::Hdr, "Hdr"),
                        (GeometryField::Width, "Width"),
                        (GeometryField::Depth, "Depth"),
                        (GeometryField::Manning, "NMAN"),
                        (GeometryField::Block(1), "BLC"),
                    ];
                    let label = fields
                        .iter()
                        .find(|(field, _)| *field == self.field)
                        .unwrap()
                        .1;
                    egui::ComboBox::from_id_salt("geometry_field")
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for (field, label) in fields {
                                ui.selectable_value(&mut self.field, field, label);
                            }
                        });
                    if matches!(self.field, GeometryField::Block(_)) {
                        ui.add(egui::DragValue::new(&mut self.block).range(1..=data.block_count()));
                    }
                });
                let field = match self.field {
                    GeometryField::Block(_) => GeometryField::Block(self.block),
                    field => field,
                };
                let row = data.rows().get(self.mini - 1)?;
                let current = geometry_value(row, field);
                let selection = (self.mini, field, current);
                if self.selection != Some(selection) {
                    self.value = current;
                    self.selection = Some(selection);
                }
                ui.horizontal(|ui| {
                    ui.label(text(pt, "Value", "Valor"));
                    let speed = match field {
                        GeometryField::Hdr | GeometryField::Order | GeometryField::Downstream => {
                            1.0
                        }
                        GeometryField::Manning => 0.001,
                        _ => 0.01,
                    };
                    ui.add(egui::DragValue::new(&mut self.value).speed(speed));
                });
                if matches!(field, GeometryField::Block(_)) {
                    ui.label(text(pt, "Apply all class changes before saving. BLC percentages must total 100.", "Aplique todas as mudanças de classe antes de salvar. As porcentagens BLC devem somar 100."));
                }
                GeometryEdit::Cell {
                    mini: self.mini,
                    field,
                    value: self.value,
                }
            }
            GeometryMode::Manning => {
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(
                        &mut self.use_subbasin,
                        text(pt, "Limit to subbasin", "Limitar à sub-bacia"),
                    );
                    if self.use_subbasin {
                        ui.add(egui::DragValue::new(&mut self.subbasin).range(1..=usize::MAX));
                    }
                    ui.add(egui::DragValue::new(&mut self.manning).speed(0.001));
                });
                GeometryEdit::Manning {
                    subbasin: self.use_subbasin.then_some(self.subbasin),
                    value: self.manning,
                }
            }
            GeometryMode::Width | GeometryMode::Depth => {
                ui.label("a × AreaMᵇ");
                ui.horizontal_wrapped(|ui| {
                    ui.label("a");
                    ui.add(egui::DragValue::new(&mut self.a).speed(0.01));
                    ui.label("b");
                    ui.add(egui::DragValue::new(&mut self.b).speed(0.01));
                });
                let dimension = if self.mode == GeometryMode::Width {
                    ChannelDimension::Width
                } else {
                    ChannelDimension::Depth
                };
                GeometryEdit::Equation {
                    dimension,
                    a: self.a,
                    b: self.b,
                }
            }
        };
        ui.button(text(pt, "Apply edit", "Aplicar edição"))
            .clicked()
            .then_some(edit)
    }
}

struct SoilFields {
    watershed: usize,
    class: usize,
    target: usize,
    selection: Option<(usize, usize)>,
    parameters: [f64; 7],
    coefficients: [f64; 4],
}

impl Default for SoilFields {
    fn default() -> Self {
        Self {
            watershed: 1,
            class: 1,
            target: 1,
            selection: None,
            parameters: [0.0; 7],
            coefficients: [0.0; 4],
        }
    }
}

impl SoilFields {
    fn show(&mut self, ui: &mut Ui, pt: bool, data: &SoilCalibration) -> Option<SoilEdit> {
        let count = data.watersheds().len();
        self.watershed = self.watershed.clamp(1, count);
        self.target = self.target.clamp(1, count);
        let classes = data.classes();
        self.class = self.class.clamp(1, classes.len());
        ui.horizontal_wrapped(|ui| {
            ui.label(text(pt, "Watershed", "Sub-bacia"));
            ui.add(egui::DragValue::new(&mut self.watershed).range(1..=count));
        });
        let watershed = data.watersheds().get(self.watershed - 1)?;
        ui.label(text(pt, "Soil class", "Classe de solo"));
        egui::ComboBox::from_id_salt("soil-class")
            .truncate()
            .selected_text(&classes[self.class - 1].name)
            .show_ui(ui, |ui| {
                for (index, class) in classes.iter().enumerate() {
                    ui.selectable_value(&mut self.class, index + 1, &class.name);
                }
            });
        let class = &classes[self.class - 1];
        if !class.description.is_empty() {
            ui.label(&class.description);
        }
        let selection = (self.watershed, self.class);
        if self.selection != Some(selection) {
            self.parameters = watershed.classes[self.class - 1];
            self.coefficients = watershed.coefficients;
            self.selection = Some(selection);
        }
        numeric_fields(
            ui,
            ["Wm", "b", "Kbas", "Kint", "XL", "CAP", "Wc"],
            &mut self.parameters,
        );
        if ui
            .button(text(pt, "Apply class values", "Aplicar valores da classe"))
            .clicked()
        {
            return Some(SoilEdit::Class {
                watershed: self.watershed,
                class: self.class,
                values: self.parameters,
            });
        }
        numeric_fields(ui, ["CS", "CI", "CB", "QB"], &mut self.coefficients);
        if ui
            .button(text(pt, "Apply coefficients", "Aplicar coeficientes"))
            .clicked()
        {
            return Some(SoilEdit::Coefficients {
                watershed: self.watershed,
                values: self.coefficients,
            });
        }
        let mut edit = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(text(pt, "Copy to watershed", "Copiar para a sub-bacia"));
            ui.add(egui::DragValue::new(&mut self.target).range(1..=count));
            if ui.button(text(pt, "Copy", "Copiar")).clicked() {
                edit = Some(SoilEdit::CopyWatershed {
                    from: self.watershed,
                    to: self.target,
                });
            }
        });
        if ui
            .button(text(
                pt,
                "Copy this watershed to all",
                "Copiar esta sub-bacia para todas",
            ))
            .clicked()
        {
            edit = Some(SoilEdit::CopyToAllWatersheds {
                from: self.watershed,
            });
        }
        if ui
            .button(text(
                pt,
                "Copy this class to all in this watershed",
                "Copiar esta classe para todas nesta sub-bacia",
            ))
            .clicked()
        {
            edit = Some(SoilEdit::CopyToAllClasses {
                watershed: self.watershed,
                from: self.class,
            });
        }
        edit
    }
}

fn numeric_fields<const N: usize>(ui: &mut Ui, labels: [&str; N], values: &mut [f64; N]) {
    ui.horizontal_wrapped(|ui| {
        for (name, value) in labels.into_iter().zip(values) {
            ui.horizontal(|ui| {
                ui.label(name);
                ui.add(egui::DragValue::new(value).speed(0.01));
            });
        }
    });
}

#[derive(Default)]
pub struct CalibrationTools {
    geometry: Editor<CalibrationGeometry>,
    soil: Editor<SoilCalibration>,
    geometry_fields: GeometryFields,
    soil_fields: SoilFields,
    soil_geometry_path: Option<PathBuf>,
    hrc_path: Option<PathBuf>,
}

impl CalibrationTools {
    pub fn geometry(
        &mut self,
        ui: &mut Ui,
        pt: bool,
        source_hint: Option<&Path>,
    ) -> Option<PathBuf> {
        if self.geometry.poll(pt) {
            self.geometry_fields.mini = 1;
            self.geometry_fields.block = 1;
            self.geometry_fields.selection = None;
        }
        ui.add_enabled_ui(self.geometry.job.is_none(), |ui| {
            let mut open = None;
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(text(pt, "Open geometry…", "Abrir geometria…"))
                    .clicked()
                {
                    open = pick(
                        ui.ctx(),
                        "geometry-open",
                        "MGB",
                        &["gtp", "mgb", "hig", "txt"],
                    );
                }
                if let Some(path) = source_hint
                    && ui
                        .button(text(
                            pt,
                            "Open selected geometry",
                            "Abrir geometria selecionada",
                        ))
                        .clicked()
                {
                    open = Some(path.into());
                }
            });
            if let Some(path) = open {
                self.geometry.start(ui, move |data| {
                    *data = Some(CalibrationGeometry::read(&path)?);
                    Ok(Effect::Opened(Some(path)))
                });
                return;
            }
            if let Some(data) = &mut self.geometry.data {
                if let Some(source) = &self.geometry.source {
                    ui.label(source.display().to_string());
                }
                ui.label(format!(
                    "{} MINI · {} BLC",
                    data.rows().len(),
                    data.block_count()
                ));
                geometry_rows(ui, data, &mut self.geometry_fields.mini, pt);
                if let Some(edit) = self.geometry_fields.show(ui, pt, data) {
                    let result = data.edit(edit).map(Effect::Changed);
                    self.geometry.finish(result, pt);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(text(
                            pt,
                            "Apply width/depth table…",
                            "Aplicar tabela de largura/profundidade…",
                        ))
                        .clicked()
                        && let Some(path) =
                            pick(ui.ctx(), "geometry-csv-edits", "CSV", &["csv", "txt"])
                    {
                        self.geometry.start(ui, move |data| {
                            data.as_mut()
                                .ok_or("Open geometry first")?
                                .apply_channel_table(&path)
                                .map(Effect::Changed)
                        });
                    }
                    if self.geometry.job.is_none()
                        && ui
                            .button(text(pt, "Save geometry as…", "Salvar geometria como…"))
                            .clicked()
                        && let Some(path) = save_file(ui.ctx(), "geometry-export", "MINI.gtp")
                    {
                        self.geometry.start(ui, move |data| {
                            data.as_ref().ok_or("Open geometry first")?.save(&path)?;
                            Ok(Effect::Saved(path))
                        });
                    }
                });
            }
        });
        self.geometry.footer(ui, pt)
    }

    pub fn soil(
        &mut self,
        ui: &mut Ui,
        pt: bool,
        geometry_hint: Option<&Path>,
        soil_hint: Option<&Path>,
    ) -> Option<PathBuf> {
        if self.soil.poll(pt) {
            self.soil_fields = SoilFields::default();
        }
        ui.add_enabled_ui(self.soil.job.is_none(), |ui| {
            if ui
                .button(text(pt, "Select geometry…", "Selecionar geometria…"))
                .clicked()
                && let Some(path) = pick(
                    ui.ctx(),
                    "geometry-import",
                    "MGB",
                    &["gtp", "mgb", "hig", "txt"],
                )
            {
                self.soil_geometry_path = Some(path);
            }
            let geometry = self.soil_geometry_path.as_deref().or(geometry_hint);
            if let Some(path) = geometry {
                ui.label(path.display().to_string());
            }
            if ui
                .button(text(
                    pt,
                    "Select HRC descriptions…",
                    "Selecionar descrições HRC…",
                ))
                .clicked()
                && let Some(path) = pick(ui.ctx(), "geometry-hrc", "HRC", &["hrc", "HRC"])
            {
                self.hrc_path = Some(path);
            }
            if let Some(path) = &self.hrc_path {
                ui.label(path.display().to_string());
            }
            let mut open = None;
            if let Some((geometry, hrc)) = geometry.zip(self.hrc_path.as_deref()) {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(text(
                            pt,
                            "Open soil parameters…",
                            "Abrir parâmetros de solo…",
                        ))
                        .clicked()
                    {
                        open = pick(ui.ctx(), "soil-open", "MGB", &["hig", "cal", "CAL"]).map(Some);
                    }
                    if let Some(path) = soil_hint
                        && ui
                            .button(text(
                                pt,
                                "Open selected soil parameters",
                                "Abrir parâmetros de solo selecionados",
                            ))
                            .clicked()
                    {
                        open = Some(Some(path.into()));
                    }
                    if ui
                        .button(text(pt, "New soil parameters", "Novos parâmetros de solo"))
                        .clicked()
                    {
                        open = Some(None);
                    }
                });
                if let Some(path) = open {
                    let geometry = geometry.to_path_buf();
                    let hrc = hrc.to_path_buf();
                    self.soil.start(ui, move |data| {
                        let geometry = CalibrationGeometry::read(&geometry)?;
                        *data = Some(match &path {
                            Some(path) => SoilCalibration::read(path, &geometry, &hrc)?,
                            None => SoilCalibration::create(&geometry, &hrc)?,
                        });
                        Ok(Effect::Opened(path))
                    });
                    return;
                }
            }
            if let Some(data) = &mut self.soil.data {
                if let Some(source) = &self.soil.source {
                    ui.label(source.display().to_string());
                }
                ui.label(format!(
                    "{} · {}",
                    data.watersheds().len(),
                    text(pt, "watersheds", "sub-bacias")
                ));
                if let Some(edit) = self.soil_fields.show(ui, pt, data) {
                    let result = data.edit(edit).map(Effect::Changed);
                    if result.is_ok() {
                        self.soil_fields.selection = None;
                    }
                    self.soil.finish(result, pt);
                }
                if ui
                    .button(text(
                        pt,
                        "Save soil parameters as…",
                        "Salvar parâmetros de solo como…",
                    ))
                    .clicked()
                    && let Some(path) = save_file(ui.ctx(), "soil-export", "paruso.CAL")
                {
                    self.soil.start(ui, move |data| {
                        data.as_ref()
                            .ok_or("Open soil parameters first")?
                            .save(&path)?;
                        Ok(Effect::Saved(path))
                    });
                }
            }
        });
        self.soil.footer(ui, pt)
    }
}

fn pick(
    context: &egui::Context,
    purpose: &str,
    label: &str,
    extensions: &[&str],
) -> Option<PathBuf> {
    super::dialogs::file(
        context,
        purpose,
        std::path::Path::new(""),
        label,
        extensions,
    )
}
fn save_file(context: &egui::Context, purpose: &str, name: &str) -> Option<PathBuf> {
    super::dialogs::save(context, purpose, std::path::Path::new(""), name)
}

fn geometry_value(row: &CalibrationGeometryRow, field: GeometryField) -> f64 {
    match field {
        GeometryField::Area => row.area,
        GeometryField::UpstreamArea => row.upstream_area,
        GeometryField::RiverLength => row.river_length,
        GeometryField::RiverSlope => row.river_slope,
        GeometryField::LongestLength => row.longest_length,
        GeometryField::LongestSlope => row.longest_slope,
        GeometryField::Downstream => row.downstream as f64,
        GeometryField::Order => row.order as f64,
        GeometryField::Hdr => row.hdr as f64,
        GeometryField::Width => row.width,
        GeometryField::Depth => row.depth,
        GeometryField::Manning => row.manning,
        GeometryField::Block(index) => row.blocks[index - 1],
    }
}

fn geometry_rows(ui: &mut Ui, data: &CalibrationGeometry, mini: &mut usize, pt: bool) {
    let row_height = ui.spacing().interact_size.y;
    egui::ScrollArea::vertical()
        .id_salt("geometry-rows")
        .max_height(140.)
        .show_rows(ui, row_height, data.rows().len(), |ui, visible| {
            for index in visible {
                let row = &data.rows()[index];
                let label = format!("MINI {} · CatID {}", row.mini, row.cat_id);
                if ui
                    .add_sized(
                        [ui.available_width(), row_height],
                        egui::Button::selectable(row.mini == *mini, label).truncate(),
                    )
                    .clicked()
                {
                    *mini = row.mini;
                }
            }
        });
    if let Some(row) = data.rows().get(mini.saturating_sub(1)) {
        ui.collapsing(
            text(pt, "Selected MINI values", "Valores da MINI selecionada"),
            |ui| {
                let fields = [
                    ("Sub", row.subbasin.to_string()),
                    ("AreaM", row.upstream_area.to_string()),
                    ("Width", row.width.to_string()),
                    ("Depth", row.depth.to_string()),
                    ("NMAN", row.manning.to_string()),
                    ("MiniJus", row.downstream.to_string()),
                    ("Hdr", row.hdr.to_string()),
                ];
                for (label, value) in fields
                    .into_iter()
                    .map(|(label, value)| (label.into(), value))
                    .chain(row.blocks.iter().enumerate().map(|(index, percentage)| {
                        (format!("BLC_{:02}", index + 1), percentage.to_string())
                    }))
                {
                    super::theme::fitted_columns(ui, &[0.4, 0.6], |ui, column| {
                        if column == 0 {
                            ui.strong(&label);
                        } else {
                            ui.label(&value);
                        }
                    });
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_selection_requires_a_current_successful_save() {
        let mut editor = Editor::<()>::default();
        let path = PathBuf::from("geometry-v1.gtp");
        editor.finish(Ok(Effect::Opened(Some(PathBuf::from("Cell.hig")))), false);
        assert!(editor.saved.is_none());
        editor.finish(Ok(Effect::Saved(path.clone())), false);
        editor.finish(Ok(Effect::Changed(0)), false);
        assert_eq!(editor.saved.as_ref(), Some(&path));
        editor.finish(Ok(Effect::Changed(4)), false);
        editor.finish(Err("destination already exists".into()), false);
        assert!(editor.saved.is_none());
        assert!(editor.dirty);
        editor.finish(Ok(Effect::Saved(path.clone())), false);
        assert_eq!(editor.saved, Some(path));
        let saved_clean = !editor.dirty;
        assert!(saved_clean);
    }
}
