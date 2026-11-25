use eframe::egui::{self, Color32, Stroke, Theme};

pub const FONT: &[u8] = include_bytes!("../../../assets/typography/NotoSans-Regular.ttf");

pub fn chat_font() -> egui::FontFamily {
    egui::FontFamily::Name("Assistant".into())
}

pub fn install(context: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let name = "Noto Sans".to_owned();
    fonts
        .font_data
        .insert(name.clone(), egui::FontData::from_static(FONT).into());
    let mut chat = fonts.families[&egui::FontFamily::Proportional].clone();
    chat.insert(0, name.clone());
    chat.push("Hack".into());
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .expect("proportional fonts")
        .insert(0, name);
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .push("Hack".into());
    fonts.families.insert(chat_font(), chat);
    context.set_fonts(fonts);
    for theme in [Theme::Light, Theme::Dark] {
        context.style_mut_of(theme, |style| {
            style.interaction.selectable_labels = true;
            let dark = theme == Theme::Dark;
            let color = |light, dark_color| rgb(if dark { dark_color } else { light });
            let surface = color(0xf5f7fb, 0x18222d);
            let raised = color(0xffffff, 0x243240);
            let recessed = color(0xedf1f7, 0x1b2835);
            let edge = color(0xd8e0eb, 0x3c5063);
            let ink = color(0x203043, 0xedf3f9);
            let secondary = color(0x4d6074, 0xb8c7d7);
            let accent = color(0x087ea4, 0x76d0ef);
            let selected = color(0xe1f2fb, 0x274d61);

            let visuals = &mut style.visuals;
            visuals.panel_fill = surface;
            visuals.window_fill = raised;
            visuals.window_stroke = Stroke::new(1., edge);
            visuals.faint_bg_color = recessed;
            visuals.extreme_bg_color = raised;
            visuals.code_bg_color = recessed;
            visuals.weak_text_color = Some(secondary);
            visuals.hyperlink_color = accent;
            visuals.warn_fg_color = color(0x854900, 0xffc07b);
            visuals.error_fg_color = color(0xa22135, 0xff9da9);
            visuals.selection.bg_fill = selected;
            visuals.selection.stroke = Stroke::new(1., ink);
            visuals.text_cursor.stroke = Stroke::new(2., accent);

            let widgets = &mut visuals.widgets;
            widgets.noninteractive.bg_fill = surface;
            widgets.noninteractive.weak_bg_fill = surface;
            widgets.noninteractive.bg_stroke = Stroke::new(1., edge);
            widgets.noninteractive.fg_stroke.color = ink;
            for (widget, fill, stroke) in [
                (&mut widgets.inactive, raised, Stroke::new(1., edge)),
                (&mut widgets.hovered, selected, Stroke::new(1., accent)),
                (&mut widgets.active, selected, Stroke::new(2., accent)),
                (&mut widgets.open, selected, Stroke::new(1., accent)),
            ] {
                widget.bg_fill = fill;
                widget.weak_bg_fill = fill;
                widget.bg_stroke = stroke;
                widget.fg_stroke.color = ink;
                widget.corner_radius = egui::CornerRadius::same(8);
                widget.expansion = 0.;
            }
            style.spacing.button_padding = egui::vec2(10., 5.);
            style.spacing.item_spacing = egui::vec2(7., 6.);
            style.spacing.interact_size.y = 28.;
            for (text_style, size) in [
                (egui::TextStyle::Body, 13.),
                (egui::TextStyle::Button, 13.),
                (egui::TextStyle::Heading, 18.),
                (egui::TextStyle::Small, 11.),
            ] {
                style
                    .text_styles
                    .insert(text_style, egui::FontId::proportional(size));
            }
        });
    }
}

fn rgb(value: u32) -> Color32 {
    let [_, red, green, blue] = value.to_be_bytes();
    Color32::from_rgb(red, green, blue)
}

pub fn toolbar(style: &egui::Style) -> egui::Frame {
    egui::Frame::side_top_panel(style)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .fill(style.visuals.faint_bg_color)
}

pub fn inset(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .inner_margin(10)
        .corner_radius(10.)
        .fill(style.visuals.extreme_bg_color)
}

pub fn primary(_ui: &egui::Ui, label: String) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label).strong()).selected(true)
}

pub fn map_surface(ui: &egui::Ui) -> Color32 {
    ui.visuals().window_fill()
}

#[derive(Clone, Copy)]
pub enum ActionIcon {
    Send,
    Copy,
    Stop,
    Check,
}

pub fn action_button(
    ui: &mut egui::Ui,
    icon: ActionIcon,
    label: &str,
    enabled: bool,
    primary: bool,
) -> egui::Response {
    let compact = matches!(icon, ActionIcon::Copy | ActionIcon::Check);
    let response = ui
        .scope(|ui| {
            if compact {
                ui.spacing_mut().interact_size = egui::Vec2::splat(22.);
                ui.spacing_mut().button_padding = egui::Vec2::splat(3.);
            }
            ui.add_enabled(
                enabled,
                egui::Button::new("")
                    .frame(!compact)
                    .selected(primary)
                    .min_size(egui::Vec2::splat(if compact { 22. } else { 32. })),
            )
        })
        .inner;
    let size = if compact { 14. } else { 16. };
    let rect = egui::Rect::from_center_size(response.rect.center(), egui::Vec2::splat(size));
    let mut ink = ui.style().interact(&response).fg_stroke.color;
    if !response.enabled() {
        ink = ink.gamma_multiply(0.45);
    }
    let stroke = Stroke::new(1.7, ink);
    let p = |x: f32, y: f32| rect.min + egui::vec2(x, y) * (size / 16.);
    let painter = ui
        .painter()
        .with_clip_rect(response.rect.intersect(ui.clip_rect()));
    match icon {
        ActionIcon::Send => {
            painter.add(egui::Shape::line(
                vec![p(1., 1.), p(15., 8.), p(1., 15.), p(4., 8.), p(1., 1.)],
                stroke,
            ));
            painter.line_segment([p(4., 8.), p(15., 8.)], stroke);
        }
        ActionIcon::Copy => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(5., 1.), p(15., 11.)),
                2.,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                egui::Rect::from_min_max(p(1., 5.), p(11., 15.)),
                2.,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        ActionIcon::Stop => {
            painter.rect_filled(rect.shrink(2.), 2., stroke.color);
        }
        ActionIcon::Check => {
            painter.add(egui::Shape::line(
                vec![p(1., 8.), p(6., 13.), p(15., 3.)],
                stroke,
            ));
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), label)
    });
    response.on_hover_text(label).on_disabled_hover_text(label)
}

pub fn copy_button(ui: &mut egui::Ui, value: &str, label: &str, pt: bool) {
    let id = ui.next_auto_id().with("copied-at");
    let copied = ui
        .data(|data| data.get_temp::<std::time::Instant>(id))
        .is_some_and(|at| at.elapsed().as_secs_f32() < 1.4);
    let hint = if copied {
        super::text(pt, "Copied", "Copiado")
    } else {
        label.into()
    };
    if action_button(
        ui,
        if copied {
            ActionIcon::Check
        } else {
            ActionIcon::Copy
        },
        &hint,
        true,
        false,
    )
    .clicked()
    {
        ui.copy_text(value.to_owned());
        ui.data_mut(|data| data.insert_temp(id, std::time::Instant::now()));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(1400));
    }
}

pub fn table_text(ui: &mut egui::Ui, text: egui::RichText) {
    ui.add(egui::Label::new(text).truncate());
}

pub fn fitted_columns(
    ui: &mut egui::Ui,
    widths: &[f32],
    mut cell: impl FnMut(&mut egui::Ui, usize),
) {
    if widths.is_empty() {
        return;
    }
    let gap = ui
        .spacing()
        .item_spacing
        .x
        .min(ui.available_width() / (2. * widths.len() as f32));
    let width = (ui.available_width() - gap * (widths.len() - 1) as f32).max(0.);
    let rect = ui.available_rect_before_wrap();
    let mut row = egui::Rect::from_min_max(rect.min, rect.right_top());
    let mut x = rect.left();
    for (index, fraction) in widths.iter().enumerate() {
        let right = (x + width * fraction).min(rect.right());
        let column =
            egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(right, rect.bottom()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(column)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(
            ui.clip_rect()
                .intersect(column.with_max_y(ui.clip_rect().bottom())),
        );
        cell(&mut child, index);
        row.max.y = row.max.y.max(child.min_rect().bottom());
        x = right + gap;
    }
    ui.advance_cursor_after_rect(row);
}

pub fn fitted_table(
    ui: &mut egui::Ui,
    columns: &[String],
    rows: &[Vec<String>],
    mut cell: impl FnMut(&mut egui::Ui, &str, bool),
) {
    if columns.is_empty() {
        return;
    }
    let widths = vec![1. / columns.len() as f32; columns.len()];
    if !rows.is_empty() && ui.available_width() / (columns.len() as f32) < 75. {
        for row in rows {
            ui.separator();
            for (heading, value) in columns.iter().zip(row) {
                fitted_columns(ui, &[0.4, 0.6], |ui, index| {
                    if index == 0 {
                        cell(ui, heading, true);
                    } else {
                        cell(ui, value, false);
                    }
                });
            }
        }
    } else {
        fitted_columns(ui, &widths, |ui, index| cell(ui, &columns[index], true));
        for row in rows {
            ui.separator();
            fitted_columns(ui, &widths, |ui, index| {
                if let Some(value) = row.get(index) {
                    cell(ui, value, false);
                }
            });
        }
    }
}

pub fn color_palette(ui: &mut egui::Ui, color: &mut Color32, label: &str, pt: bool) {
    let (rect, button) = ui.allocate_exact_size(egui::Vec2::splat(16.), egui::Sense::click());
    ui.painter().rect(
        rect,
        3.,
        *color,
        Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let button = button.on_hover_text(super::text(
        pt,
        "Choose layer colour",
        "Escolher cor da camada",
    ));
    button.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    egui::Popup::from_toggle_button_response(&button).show(|ui| {
        ui.strong(label);
        let palette: [u32; 40] = [
            0xffffff, 0xe2e8f0, 0x94a3b8, 0x475569, 0x1e293b, 0x000000, 0xffedd5, 0x7c2d12,
            0xfee2e2, 0xfecaca, 0xf87171, 0xef4444, 0xb91c1c, 0x7f1d1d, 0xfed7aa, 0xf97316,
            0xfef9c3, 0xfde047, 0xeab308, 0xa16207, 0xdcfce7, 0x86efac, 0x22c55e, 0x15803d,
            0xccfbf1, 0x5eead4, 0x0d9488, 0x115e59, 0xdbeafe, 0x93c5fd, 0x3b82f6, 0x052887,
            0xede9fe, 0xc4b5fd, 0x8b5cf6, 0x6d28d9, 0xfae8ff, 0xe879f9, 0xdb2777, 0x831843,
        ];
        egui::Grid::new("colour-matrix")
            .spacing(egui::vec2(3., 3.))
            .show(ui, |ui| {
                for (index, value) in palette.into_iter().enumerate() {
                    let choice = rgb(value);
                    let name = format!("#{value:06X}");
                    let (rect, response) =
                        ui.allocate_exact_size(egui::Vec2::splat(14.), egui::Sense::click());
                    let response = response.on_hover_text(&name);
                    ui.painter().rect_filled(rect, 3., choice);
                    if choice == *color || response.hovered() || response.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            3.,
                            Stroke::new(2., ui.visuals().text_color()),
                            egui::StrokeKind::Inside,
                        );
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &name)
                    });
                    if response.clicked() {
                        *color = Color32::from_rgba_unmultiplied(
                            choice.r(),
                            choice.g(),
                            choice.b(),
                            color.a(),
                        );
                        ui.close();
                    }
                    if index % 8 == 7 {
                        ui.end_row();
                    }
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(super::text(pt, "Custom colour", "Cor personalizada"));
            ui.color_edit_button_srgba(color);
        });
    });
}

#[derive(Clone)]
struct TextActions {
    text: String,
    selection: egui::text::CCursorRange,
    position: egui::Pos2,
    initialized: bool,
}

fn text_actions_id() -> egui::Id {
    egui::Id::new("desktop-text-actions")
}
fn text_exclusions_id() -> egui::Id {
    egui::Id::new("desktop-text-exclusions")
}

pub fn begin_text_actions(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(text_exclusions_id(), Vec::<egui::Rect>::new()));
}

pub fn exclude_text_actions(ctx: &egui::Context, rect: egui::Rect) {
    ctx.data_mut(|data| {
        let mut excluded = data
            .get_temp::<Vec<egui::Rect>>(text_exclusions_id())
            .unwrap_or_default();
        excluded.push(rect);
        data.insert_temp(text_exclusions_id(), excluded);
    });
}

fn word_range(text: &str, cursor: usize) -> egui::text::CCursorRange {
    let chars: Vec<_> = text.chars().collect();
    let mut start = cursor.min(chars.len().saturating_sub(1));
    let mut end = start;
    let word = |c: char| c.is_alphanumeric() || c == '_';
    if chars.get(start).is_some_and(|c| word(*c)) {
        while start > 0 && word(chars[start - 1]) {
            start -= 1;
        }
        while end < chars.len() && word(chars[end]) {
            end += 1;
        }
    } else {
        end = (end + 1).min(chars.len());
    }
    egui::text::CCursorRange::two(
        egui::text::CCursor::new(start),
        egui::text::CCursor::new(end),
    )
}

fn text_at(shape: &egui::Shape, pointer: egui::Pos2) -> Option<(String, usize)> {
    match shape {
        egui::Shape::Text(text) if text.visual_bounding_rect().contains(pointer) => {
            let cursor = text.galley.cursor_from_pos(pointer - text.pos).index;
            Some((text.galley.text().to_owned(), cursor.into()))
        }
        egui::Shape::Vec(shapes) => shapes
            .iter()
            .rev()
            .find_map(|shape| text_at(shape, pointer)),
        _ => None,
    }
}

/// Offers text actions over reading surfaces without intercepting their normal gestures.
pub fn text_actions(ctx: &egui::Context, pt: bool) {
    let clicked = ctx.input(|input| {
        input
            .pointer
            .button_double_clicked(egui::PointerButton::Primary)
    });
    if clicked && let Some(pointer) = ctx.pointer_interact_pos() {
        let excluded = ctx.data(|data| {
            data.get_temp::<Vec<egui::Rect>>(text_exclusions_id())
                .unwrap_or_default()
        });
        if !ctx.egui_wants_keyboard_input()
            && !excluded.iter().any(|rect| rect.contains(pointer))
            && let Some(layer) = ctx.layer_id_at(pointer)
        {
            let text = ctx.graphics(|graphics| {
                graphics.get(layer).and_then(|list| {
                    list.all_entries()
                        .filter(|item| item.clip_rect.contains(pointer))
                        .filter_map(|item| text_at(&item.shape, pointer))
                        .last()
                })
            });
            if let Some((text, cursor)) = text {
                ctx.data_mut(|data| {
                    data.insert_temp(
                        text_actions_id(),
                        TextActions {
                            selection: word_range(&text, cursor),
                            text,
                            position: pointer,
                            initialized: false,
                        },
                    )
                });
            }
        }
    }
    let Some(mut actions) = ctx.data(|data| data.get_temp::<TextActions>(text_actions_id())) else {
        return;
    };
    let mut open = true;
    let mut close = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    egui::Window::new(super::text(pt, "Text actions", "Ações de texto"))
        .id(text_actions_id())
        .open(&mut open)
        .default_pos(actions.position)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            let id = ui.id().with("selected-text");
            if !actions.initialized {
                let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                state.cursor.set_char_range(Some(actions.selection));
                state.store(ctx, id);
                actions.initialized = true;
                ui.memory_mut(|memory| memory.request_focus(id));
            }
            let mut readonly = actions.text.as_str();
            let output = egui::ScrollArea::vertical()
                .max_height(150.)
                .show(ui, |ui| {
                    egui::TextEdit::multiline(&mut readonly)
                        .id(id)
                        .desired_width(280.)
                        .desired_rows(1)
                        .show(ui)
                })
                .inner;
            if let Some(selection) = output.cursor_range {
                actions.selection = selection;
            }
            ui.horizontal(|ui| {
                if ui.button(super::text(pt, "Copy", "Copiar")).clicked() {
                    ui.copy_text(actions.selection.slice_str(&actions.text).to_owned());
                    close = true;
                }
                if ui
                    .button(super::text(pt, "Select all", "Selecionar tudo"))
                    .clicked()
                {
                    actions.selection = egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(actions.text.chars().count()),
                    );
                    let mut state = output.state;
                    state.cursor.set_char_range(Some(actions.selection));
                    state.store(ctx, id);
                    output.response.request_focus();
                }
            });
        });
    ctx.data_mut(|data| {
        if !open || close {
            data.remove::<TextActions>(text_actions_id());
        } else {
            data.insert_temp(text_actions_id(), actions);
        }
    });
}

pub fn color_key(ui: &mut egui::Ui, color: Color32, label: &str) -> egui::Response {
    legend_key(ui, egui::vec2(10., 10.), label, |ui, rect| {
        paint_swatch(ui, rect, color);
    })
}

fn paint_swatch(ui: &egui::Ui, rect: egui::Rect, color: Color32) {
    ui.painter().rect(
        rect,
        2.,
        color,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
}

pub fn legend_key(
    ui: &mut egui::Ui,
    mark_size: egui::Vec2,
    label: &str,
    paint_mark: impl FnOnce(&egui::Ui, egui::Rect),
) -> egui::Response {
    let mark_and_gap = mark_size.x + ui.spacing().item_spacing.x;
    let text = ui.painter().layout(
        label.into(),
        egui::TextStyle::Small.resolve(ui.style()),
        ui.visuals().text_color(),
        (ui.max_rect().width() - mark_and_gap).max(1.),
    );
    // Reserve the measured width so a wrapping parent can move the mark and label together.
    let size = egui::vec2(text.size().x + mark_and_gap, text.size().y.max(mark_size.y));
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let (rect, _) = ui.allocate_exact_size(mark_size, egui::Sense::hover());
            paint_mark(ui, rect);
            ui.label(text)
        },
    )
    .response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_word_selection_uses_unicode_character_offsets() {
        assert_eq!(
            word_range("Rio São João", 5).slice_str("Rio São João"),
            "São"
        );
        assert_eq!(word_range("流域 dados", 1).slice_str("流域 dados"), "流域");
        assert_eq!(word_range("", 0).slice_str(""), "");
    }

    #[test]
    fn double_click_offers_label_actions_and_preserves_editable_field_focus() {
        for editable in [false, true] {
            let ctx = egui::Context::default();
            let mut value = "Rio São Francisco".to_owned();
            let mut target = egui::Rect::NOTHING;
            let mut time = 0.;
            let mut frame = |events| {
                time += 0.04;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events,
                        time: Some(time),
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(500., 400.),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        begin_text_actions(ui.ctx());
                        target = if editable {
                            ui.text_edit_singleline(&mut value).rect
                        } else {
                            ui.label(&value).rect
                        };
                        text_actions(ui.ctx(), false);
                    },
                );
                output.textures_delta.clear();
                output
            };
            frame(Vec::new());
            frame(Vec::new());
            let pointer = egui::pos2(25., 12.);
            frame(vec![egui::Event::PointerMoved(pointer)]);
            for _ in 0..2 {
                for pressed in [true, false] {
                    frame(vec![egui::Event::PointerButton {
                        pos: pointer,
                        pressed,
                        button: egui::PointerButton::Primary,
                        modifiers: egui::Modifiers::NONE,
                    }]);
                }
            }
            let menu = ctx.data(|data| data.get_temp::<TextActions>(text_actions_id()));
            if editable {
                assert!(
                    menu.is_none(),
                    "editing a word must not open a competing window"
                );
                assert!(ctx.memory(|memory| memory.focused().is_some()));
            } else {
                let menu = menu.expect("double-click reading text opens copy actions");
                assert_eq!(menu.text, "Rio São Francisco");
                let has_word = !menu.selection.slice_str(&menu.text).is_empty();
                assert!(has_word);
            }
        }
    }
}
