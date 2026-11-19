use super::super::{data::timestamp, text};
use eframe::egui::{self, Ui};
use mgb_ops::app::{Draft, DraftCorrection, DraftReplacement};
use mgb_ops::store::{
    ForecastCorrection, ObservationReplacement, ReplacementKind, check_correction,
    validate_replacement,
};
use std::collections::HashSet;

pub enum InstructionAction {
    MaterializeReplacements,
    MaterializeCorrections(usize),
    PreviewObservations,
    PreviewForecast(usize),
}

#[derive(Default)]
pub struct InstructionTools {
    forecast: bool,
    scenario: usize,
    replacement: ReplacementForm,
    correction: CorrectionForm,
    status: String,
}

impl InstructionTools {
    pub fn show(&mut self, ui: &mut Ui, pt: bool, draft: &mut Draft) -> Option<InstructionAction> {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut self.forecast,
                false,
                text(
                    pt,
                    "Observation replacements",
                    "Substituições de observações",
                ),
            );
            ui.selectable_value(
                &mut self.forecast,
                true,
                text(pt, "Forecast corrections", "Correções de previsão"),
            );
        });
        let action = if self.forecast {
            self.forecast(ui, pt, draft)
        } else {
            self.observations(ui, pt, draft)
        };
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        action
    }

    fn observations(
        &mut self,
        ui: &mut Ui,
        pt: bool,
        draft: &mut Draft,
    ) -> Option<InstructionAction> {
        let registered = draft
            .document
            .operation
            .observed
            .as_ref()
            .and_then(|observed| observed.replacement_ids.as_deref())
            .unwrap_or(&[]);
        let items = &mut draft.document.instructions.replacements;
        if items.is_none() && !registered.is_empty() && materialize(ui, pt, registered) {
            return Some(InstructionAction::MaterializeReplacements);
        }
        if items.is_some() || registered.is_empty() {
            ui.separator();
            let mut modified = false;
            if let Some(values) = items.as_mut() {
                let rows = values
                    .iter()
                    .map(|item| (&item.draft_id, replacement_summary(&item.value)));
                if let Some((index, action)) = rows_ui(ui, pt, rows, values.len()) {
                    if action == RowAction::Edit {
                        self.replacement = ReplacementForm::from_item(&values[index]);
                        self.status.clear();
                    } else {
                        modify(values, index, action);
                        self.replacement = ReplacementForm::default();
                        modified = true;
                    }
                }
            }
            if self.replacement.show(ui, pt) {
                match self.replacement.save(items) {
                    Ok(()) => {
                        self.replacement = ReplacementForm::default();
                        self.status.clear();
                        modified = true;
                    }
                    Err(error) => self.status = error,
                }
            }
            if modified {
                draft.document.sources.accepted.remove("observations");
            }
        }
        ui.button(text(
            pt,
            "Preview observations",
            "Pré-visualizar observações",
        ))
        .clicked()
        .then_some(InstructionAction::PreviewObservations)
    }

    fn forecast(&mut self, ui: &mut Ui, pt: bool, draft: &mut Draft) -> Option<InstructionAction> {
        let scenarios = &draft.document.operation.scenarios;
        let choices: Vec<_> = scenarios
            .iter()
            .enumerate()
            .filter(|(_, scenario)| scenario.asset_id.as_ref().is_some_and(|id| !id.is_empty()))
            .collect();
        let Some((first, _)) = choices.first() else {
            ui.label(text(
                pt,
                "Select a forecast asset in the draft.",
                "Selecione um ativo de previsão no rascunho.",
            ));
            return None;
        };
        let previous = self.scenario;
        if !choices.iter().any(|(index, _)| *index == self.scenario) {
            self.scenario = *first;
        }
        let label = |index: usize| -> String {
            scenarios[index]
                .id
                .clone()
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| format!("{} {}", text(pt, "Scenario", "Cenário"), index + 1))
        };
        egui::ComboBox::from_id_salt("instruction_scenario")
            .selected_text(label(self.scenario))
            .show_ui(ui, |ui| {
                for (index, _) in choices {
                    ui.selectable_value(&mut self.scenario, index, label(index));
                }
            });
        if previous != self.scenario {
            self.correction = CorrectionForm::default();
            self.status.clear();
        }
        let index = self.scenario;
        let scenario = &scenarios[index];
        let asset = scenario.asset_id.as_deref()?;
        ui.label(asset);
        let registered = scenario.correction_ids.as_deref().unwrap_or(&[]);
        let mut modified = false;
        let local = draft
            .document
            .instructions
            .corrections
            .get(index)
            .and_then(Option::as_ref);
        if local.is_none() && !registered.is_empty() && materialize(ui, pt, registered) {
            return Some(InstructionAction::MaterializeCorrections(index));
        }
        if local.is_some() || registered.is_empty() {
            ui.separator();
            if let Some(values) = draft
                .document
                .instructions
                .corrections
                .get_mut(index)
                .and_then(Option::as_mut)
            {
                let rows = values
                    .iter()
                    .map(|item| (&item.draft_id, correction_summary(&item.value)));
                if let Some((position, action)) = rows_ui(ui, pt, rows, values.len()) {
                    if action == RowAction::Edit {
                        self.correction = CorrectionForm::from_item(&values[position]);
                        self.status.clear();
                    } else {
                        modify(values, position, action);
                        self.correction = CorrectionForm::default();
                        modified = true;
                    }
                }
            }
            if self.correction.show(ui, pt) {
                match self.correction.save(
                    &mut draft.document.instructions.corrections,
                    index,
                    asset,
                ) {
                    Ok(()) => {
                        self.correction = CorrectionForm::default();
                        self.status.clear();
                        modified = true;
                    }
                    Err(error) => self.status = error,
                }
            }
            if modified {
                draft
                    .document
                    .sources
                    .accepted
                    .remove(&format!("forecast:{index}"));
            }
        }
        ui.button(text(pt, "Preview forecast", "Pré-visualizar previsão"))
            .clicked()
            .then_some(InstructionAction::PreviewForecast(index))
    }
}

fn materialize(ui: &mut Ui, pt: bool, ids: &[String]) -> bool {
    ui.label(ids.join(", "));
    ui.button(text(
        pt,
        "Load registered instructions for editing",
        "Carregar instruções registradas para edição",
    ))
    .clicked()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowAction {
    Edit,
    Remove,
    Up,
    Down,
}

fn rows_ui<'a>(
    ui: &mut Ui,
    pt: bool,
    rows: impl Iterator<Item = (&'a String, String)>,
    count: usize,
) -> Option<(usize, RowAction)> {
    let mut selected = None;
    for (index, (id, summary)) in rows.enumerate() {
        ui.push_id(index, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(id);
                ui.label(summary);
            });
            ui.horizontal_wrapped(|ui| {
                for (action, enabled, label) in [
                    (RowAction::Edit, true, text(pt, "Edit", "Editar")),
                    (RowAction::Remove, true, text(pt, "Remove", "Remover")),
                    (
                        RowAction::Up,
                        index > 0,
                        text(pt, "Move up", "Mover para cima"),
                    ),
                    (
                        RowAction::Down,
                        index + 1 < count,
                        text(pt, "Move down", "Mover para baixo"),
                    ),
                ] {
                    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                        selected = Some((index, action));
                    }
                }
            });
        });
    }
    selected
}

fn modify<T>(items: &mut Vec<T>, index: usize, action: RowAction) {
    match action {
        RowAction::Edit => {}
        RowAction::Remove => {
            items.remove(index);
        }
        RowAction::Up if index > 0 => items.swap(index, index - 1),
        RowAction::Down if index + 1 < items.len() => items.swap(index, index + 1),
        RowAction::Up | RowAction::Down => {}
    }
}

fn unique_id<'a>(prefix: &str, ids: impl Iterator<Item = &'a str>) -> String {
    let existing: HashSet<_> = ids.collect();
    let timestamp = chrono::Utc::now().timestamp_micros();
    (1..)
        .map(|index| format!("{prefix}-{timestamp}-{index}"))
        .find(|id| !existing.contains(id.as_str()))
        .unwrap()
}

fn replacement_summary(value: &ObservationReplacement) -> String {
    let end = value
        .end_at
        .map(|stamp| format!(" → {stamp}"))
        .unwrap_or_default();
    let amount = value
        .value
        .map_or_else(|| "∅".into(), |amount| amount.to_string());
    format!(
        "{} · {} · {}{end} · {amount}",
        value.station_id, value.variable, value.start_at
    )
}

fn correction_summary(value: &ForecastCorrection) -> String {
    format!(
        "{}–{} h · Δlat {} · Δlon {} · {}° · ×{}",
        value.t0_hour,
        value.t1_hour,
        value.shift_lat_pixels,
        value.shift_lon_pixels,
        value.rotation_deg,
        value.multiplication_factor
    )
}

#[derive(Default)]
struct ReplacementForm {
    edit_id: Option<String>,
    station: String,
    flow: bool,
    period: bool,
    start: String,
    end: String,
    missing: bool,
    value: f64,
    reason: String,
}

impl ReplacementForm {
    fn from_item(item: &DraftReplacement) -> Self {
        let value = &item.value;
        Self {
            edit_id: Some(item.draft_id.clone()),
            station: value.station_id.clone(),
            flow: value.variable == "flow",
            period: value.kind == ReplacementKind::Period,
            start: value.start_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
            end: value
                .end_at
                .map(|stamp| stamp.format("%Y-%m-%dT%H:%M:%S").to_string())
                .unwrap_or_default(),
            missing: value.value.is_none(),
            value: value.value.unwrap_or_default(),
            reason: value.reason.clone(),
        }
    }

    fn show(&mut self, ui: &mut Ui, pt: bool) -> bool {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.flow, false, text(pt, "Rain (mm)", "Chuva (mm)"));
            ui.selectable_value(
                &mut self.flow,
                true,
                text(pt, "Flow (m³/s)", "Vazão (m³/s)"),
            );
            ui.selectable_value(&mut self.period, false, text(pt, "Point", "Pontual"));
            ui.selectable_value(&mut self.period, true, text(pt, "Period", "Período"));
        });
        edit_text(
            ui,
            text(pt, "Station ID", "ID da estação"),
            &mut self.station,
        );
        edit_text(
            ui,
            text(
                pt,
                "Local start (YYYY-MM-DDTHH:MM:SS)",
                "Início local (AAAA-MM-DDTHH:MM:SS)",
            ),
            &mut self.start,
        );
        if self.period {
            edit_text(
                ui,
                text(pt, "Local end (optional)", "Fim local (opcional)"),
                &mut self.end,
            );
        }
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(
                &mut self.missing,
                text(pt, "Set missing", "Marcar como ausente"),
            );
            if !self.missing {
                ui.label(text(pt, "Value", "Valor"));
                ui.add(egui::DragValue::new(&mut self.value).speed(0.01));
            }
        });
        edit_text(ui, text(pt, "Reason", "Motivo"), &mut self.reason);
        save_instruction(
            ui,
            pt,
            text(pt, "Add replacement", "Adicionar substituição"),
            &mut self.edit_id,
        )
    }

    fn save(&self, items: &mut Option<Vec<DraftReplacement>>) -> Result<(), String> {
        let current = items.as_deref().unwrap_or_default();
        let edit = self
            .edit_id
            .as_ref()
            .map(|id| {
                current
                    .iter()
                    .position(|item| &item.draft_id == id)
                    .ok_or("Replacement changed; select it again")
            })
            .transpose()?;
        let id = edit
            .map(|index| current[index].draft_id.clone())
            .unwrap_or_else(|| {
                unique_id(
                    "draft-replacement",
                    current.iter().flat_map(|item| {
                        [item.draft_id.as_str(), item.value.replacement_id.as_str()]
                    }),
                )
            });
        let value = ObservationReplacement {
            replacement_id: edit.map_or_else(
                || id.clone(),
                |index| current[index].value.replacement_id.clone(),
            ),
            station_id: self.station.trim().into(),
            variable: if self.flow {
                "flow".into()
            } else {
                "rain".into()
            },
            kind: if self.period {
                ReplacementKind::Period
            } else {
                ReplacementKind::Point
            },
            start_at: timestamp(self.start.trim())?,
            end_at: if self.period && !self.end.trim().is_empty() {
                Some(timestamp(self.end.trim())?)
            } else {
                None
            },
            value: (!self.missing).then_some(self.value),
            reason: self.reason.trim().into(),
        };
        validate_replacement(&value).map_err(|e| e.to_string())?;
        let items = items.get_or_insert_with(Vec::new);
        if let Some(index) = edit {
            items[index].value = value;
        } else {
            items.push(DraftReplacement {
                draft_id: id,
                registered_origin: None,
                value,
            });
        }
        Ok(())
    }
}

struct CorrectionForm {
    edit_id: Option<String>,
    start: i64,
    end: i64,
    lat: f64,
    lon: f64,
    rotation: f64,
    factor: f64,
    editor: String,
    reason: String,
}

impl Default for CorrectionForm {
    fn default() -> Self {
        Self {
            edit_id: None,
            start: 0,
            end: 0,
            lat: 0.0,
            lon: 0.0,
            rotation: 0.0,
            factor: 1.0,
            editor: String::new(),
            reason: String::new(),
        }
    }
}

impl CorrectionForm {
    fn from_item(item: &DraftCorrection) -> Self {
        let value = &item.value;
        Self {
            edit_id: Some(item.draft_id.clone()),
            start: value.t0_hour,
            end: value.t1_hour,
            lat: value.shift_lat_pixels,
            lon: value.shift_lon_pixels,
            rotation: value.rotation_deg,
            factor: value.multiplication_factor,
            editor: value.editor.clone().unwrap_or_default(),
            reason: value.reason.clone(),
        }
    }

    fn show(&mut self, ui: &mut Ui, pt: bool) -> bool {
        ui.horizontal_wrapped(|ui| {
            ui.label(text(pt, "Start lead hour", "Hora inicial de previsão"));
            ui.add(egui::DragValue::new(&mut self.start).range(0..=i64::MAX));
            ui.label(text(pt, "End lead hour", "Hora final de previsão"));
            ui.add(egui::DragValue::new(&mut self.end).range(0..=i64::MAX));
        });
        let mut values = [self.lat, self.lon, self.rotation, self.factor];
        super::numeric_fields(
            ui,
            [
                &text(pt, "Latitude shift (pixels)", "Deslocamento lat. (pixels)"),
                &text(pt, "Longitude shift (pixels)", "Deslocamento lon. (pixels)"),
                &text(pt, "Rotation (°)", "Rotação (°)"),
                &text(pt, "Multiplier", "Multiplicador"),
            ],
            &mut values,
        );
        [self.lat, self.lon, self.rotation, self.factor] = values;
        edit_text(
            ui,
            text(pt, "Editor (optional)", "Editor (opcional)"),
            &mut self.editor,
        );
        edit_text(ui, text(pt, "Reason", "Motivo"), &mut self.reason);
        save_instruction(
            ui,
            pt,
            text(pt, "Add correction", "Adicionar correção"),
            &mut self.edit_id,
        )
    }

    fn save(
        &self,
        all: &mut Vec<Option<Vec<DraftCorrection>>>,
        index: usize,
        asset: &str,
    ) -> Result<(), String> {
        let current = all
            .get(index)
            .and_then(Option::as_deref)
            .unwrap_or_default();
        let edit = self
            .edit_id
            .as_ref()
            .map(|id| {
                current
                    .iter()
                    .position(|item| &item.draft_id == id)
                    .ok_or("Correction changed; select it again")
            })
            .transpose()?;
        let new_id = || {
            unique_id(
                "draft-correction",
                all.iter()
                    .flatten()
                    .flatten()
                    .flat_map(|item| [item.draft_id.as_str(), item.value.correction_id.as_str()]),
            )
        };
        let mut value = ForecastCorrection {
            correction_id: edit.map_or_else(new_id, |position| {
                current[position].value.correction_id.clone()
            }),
            asset_id: asset.into(),
            t0_hour: self.start,
            t1_hour: self.end,
            shift_lat_pixels: self.lat,
            shift_lon_pixels: self.lon,
            rotation_deg: self.rotation,
            multiplication_factor: self.factor,
            editor: (!self.editor.trim().is_empty()).then(|| self.editor.trim().into()),
            reason: self.reason.trim().into(),
            created_at: edit.map_or_else(
                || chrono::Utc::now().to_rfc3339(),
                |position| current[position].value.created_at.clone(),
            ),
        };
        if let Some(position) = edit
            && value != current[position].value
            && (current[position].registered_origin.is_some()
                || all
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .filter_map(|(_, values)| values.as_ref())
                    .flatten()
                    .any(|item| item.value.correction_id == value.correction_id))
        {
            value.correction_id = new_id();
        }
        check_correction(&value).map_err(|e| e.to_string())?;
        if all.len() <= index {
            all.resize_with(index + 1, || None);
        }
        let items = all[index].get_or_insert_with(Vec::new);
        if let Some(position) = edit {
            items[position].value = value;
        } else {
            items.push(DraftCorrection {
                draft_id: value.correction_id.clone(),
                registered_origin: None,
                value,
            });
        }
        Ok(())
    }
}

fn edit_text(ui: &mut Ui, label: String, value: &mut String) {
    let label = ui.label(label);
    ui.text_edit_singleline(value).labelled_by(label.id);
}

fn save_instruction(ui: &mut Ui, pt: bool, add: String, edit: &mut Option<String>) -> bool {
    let mut save = false;
    ui.horizontal_wrapped(|ui| {
        let label = if edit.is_some() {
            text(pt, "Update", "Atualizar")
        } else {
            add
        };
        save = ui.button(label).clicked();
        if edit.is_some() && ui.button(text(pt, "End edit", "Encerrar edição")).clicked() {
            *edit = None;
        }
    });
    save
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_authoring_keeps_id_and_nulls_on_valid_period() {
        let mut entries = None;
        let mut form = ReplacementForm {
            station: "ANA-123".into(),
            start: "2026-01-10T08:00:00".into(),
            value: 1.5,
            ..Default::default()
        };
        form.save(&mut entries).unwrap();
        let original = entries.as_ref().unwrap()[0].clone();
        assert_eq!(original.value.value, Some(1.5));
        assert_eq!(original.value.kind, ReplacementKind::Point);
        form = ReplacementForm::from_item(&original);
        form.period = true;
        form.end = "2026-01-10T07:00:00".into();
        form.missing = true;
        assert!(form.save(&mut entries).is_err());
        assert_eq!(entries.as_ref().unwrap()[0], original);
        form.end = "2026-01-10T10:00:00".into();
        form.save(&mut entries).unwrap();
        let edited = &entries.as_ref().unwrap()[0];
        assert_eq!(edited.draft_id, original.draft_id);
        assert_eq!(edited.value.value, None);
        assert_eq!(edited.value.end_at, timestamp(&form.end).ok());
    }

    #[test]
    fn forecast_authoring_reorders_only_selected_scenario_and_keeps_distinct_ids() {
        let mut all = Vec::new();
        let mut form = CorrectionForm {
            start: 3,
            end: 8,
            lat: -2.5,
            ..Default::default()
        };
        form.save(&mut all, 2, "asset-a").unwrap();
        form.save(&mut all, 0, "asset-b").unwrap();
        form.factor = 2.0;
        form.save(&mut all, 2, "asset-a").unwrap();
        assert!(all[1].is_none());
        let untouched = all[0].clone();
        let values = all[2].as_mut().unwrap();
        let original_id = values[0].draft_id.clone();
        let new_id = values[1].draft_id.clone();
        assert_ne!(original_id, new_id);
        modify(values, 1, RowAction::Up);
        assert_eq!(values[0].draft_id, new_id);
        modify(values, 1, RowAction::Remove);
        assert_eq!(values[0].value.multiplication_factor, 2.0);
        assert_eq!(all[0], untouched);
        let mut form = CorrectionForm::from_item(&all[2].as_ref().unwrap()[0]);
        form.factor = 0.0;
        assert!(form.save(&mut all, 2, "asset-a").is_err());
        assert_eq!(all[2].as_ref().unwrap()[0].value.multiplication_factor, 2.0);
    }
}
