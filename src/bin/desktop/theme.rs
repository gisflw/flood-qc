use eframe::egui::{self, Color32, Stroke, Theme};

pub fn install(context: &egui::Context) {
    for theme in [Theme::Light, Theme::Dark] {
        context.style_mut_of(theme, |style| {
            let dark = theme == Theme::Dark;
            let color = |light, dark_color| rgb(if dark { dark_color } else { light });
            let surface = color(0xf7f9fc, 0x202a34);
            let raised = color(0xffffff, 0x293746);
            let recessed = color(0xeceff4, 0x18222d);
            let edge = color(0xcbd5e1, 0x4a5d70);
            let ink = color(0x203043, 0xedf3f9);
            let secondary = color(0x4d6074, 0xb8c7d7);
            let accent = color(0x086282, 0x8ed7f1);
            let selected = color(0xd7edf6, 0x274d61);

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
                widget.corner_radius = egui::CornerRadius::same(4);
                widget.expansion = 0.;
            }
            style.spacing.button_padding = egui::vec2(7., 3.);
            style.spacing.item_spacing = egui::vec2(6., 5.);
            style.spacing.interact_size.y = 23.;
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
        .inner_margin(8)
        .fill(style.visuals.extreme_bg_color)
}

pub fn primary(ui: &egui::Ui, label: String) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label).strong()).fill(ui.visuals().selection.bg_fill)
}

pub fn swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10., 10.), egui::Sense::hover());
    paint_swatch(ui, rect, color);
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
