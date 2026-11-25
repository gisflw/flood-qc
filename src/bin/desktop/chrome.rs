//! Application-drawn title-bar controls and frameless window gestures.
use eframe::egui::{self, Color32, CursorIcon, ResizeDirection, Stroke, Vec2, ViewportCommand};

pub const CONTROLS_WIDTH: f32 = 100.;

pub fn title_space(ui: &mut egui::Ui) {
    let (_, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 32.),
        egui::Sense::click_and_drag(),
    );
    drag(ui, &response);
}

#[derive(Clone, Copy)]
enum Action {
    Minimize,
    Maximize,
    Close,
}

fn button(ui: &mut egui::Ui, action: Action, maximized: bool, pt: bool) -> egui::Response {
    let label = match action {
        Action::Minimize => super::text(pt, "Minimize", "Minimizar"),
        Action::Maximize if maximized => super::text(pt, "Restore window", "Restaurar janela"),
        Action::Maximize => super::text(pt, "Maximize", "Maximizar"),
        Action::Close => super::text(pt, "Close application", "Fechar aplicativo"),
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(32., 26.), egui::Sense::click());
    let visuals = ui.style().interact(&response);
    let closing = matches!(action, Action::Close);
    let highlighted = response.hovered() || response.has_focus();
    let fill = if highlighted && closing {
        Color32::from_rgb(200, 51, 70)
    } else if highlighted {
        visuals.bg_fill
    } else {
        Color32::TRANSPARENT
    };
    let ink = if highlighted && closing {
        Color32::WHITE
    } else {
        visuals.fg_stroke.color
    };
    ui.painter().rect_filled(rect, 5., fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            5.,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    let center = rect.center();
    let stroke = Stroke::new(1.4, ink);
    match action {
        Action::Minimize => {
            ui.painter().line_segment(
                [center + egui::vec2(-5., 3.), center + egui::vec2(5., 3.)],
                stroke,
            );
        }
        Action::Maximize => {
            let square = egui::Rect::from_center_size(center, Vec2::splat(10.));
            if maximized {
                ui.painter().add(egui::Shape::line(
                    vec![
                        square.left_top() + egui::vec2(3., 0.),
                        square.left_top() + egui::vec2(3., -3.),
                        square.right_top() + egui::vec2(3., -3.),
                        square.right_bottom() + egui::vec2(3., -3.),
                    ],
                    stroke,
                ));
            }
            ui.painter()
                .rect_stroke(square, 1., stroke, egui::StrokeKind::Inside);
        }
        Action::Close => {
            for sign in [-1., 1.] {
                let delta = egui::vec2(4.5, sign * 4.5);
                ui.painter()
                    .line_segment([center - delta, center + delta], stroke);
            }
        }
    }
    super::button_label(&response, &label);
    response.on_hover_text(label)
}

/// Return a close request so the application can resolve unsaved edits/jobs.
pub fn controls(ui: &mut egui::Ui, pt: bool) -> bool {
    let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
    ui.spacing_mut().item_spacing.x = 2.;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let close = button(ui, Action::Close, maximized, pt).clicked();
        if button(ui, Action::Maximize, maximized, pt).clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        }
        if button(ui, Action::Minimize, maximized, pt).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        close
    })
    .inner
}

pub fn drag(ui: &egui::Ui, response: &egui::Response) {
    if response.double_clicked() {
        let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if response.drag_started_by(egui::PointerButton::Primary) {
        // The window manager owns the rest of this gesture. Its pointer grab
        // can consume the release event, so release egui's widget drag now.
        ui.ctx().stop_dragging();
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

pub fn resize_edges(context: &egui::Context) {
    if context.input(|input| {
        input.viewport().maximized.unwrap_or(false) || input.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let rect = context.content_rect();
    let edge = 4.;
    let corner = 9.;
    let regions = [
        (
            ResizeDirection::North,
            CursorIcon::ResizeVertical,
            egui::Rect::from_min_max(
                rect.min + egui::vec2(corner, 0.),
                rect.right_top() + egui::vec2(-corner, edge),
            ),
        ),
        (
            ResizeDirection::South,
            CursorIcon::ResizeVertical,
            egui::Rect::from_min_max(
                rect.left_bottom() + egui::vec2(corner, -edge),
                rect.max - egui::vec2(corner, 0.),
            ),
        ),
        (
            ResizeDirection::West,
            CursorIcon::ResizeHorizontal,
            egui::Rect::from_min_max(
                rect.min + egui::vec2(0., corner),
                rect.left_bottom() + egui::vec2(edge, -corner),
            ),
        ),
        (
            ResizeDirection::East,
            CursorIcon::ResizeHorizontal,
            egui::Rect::from_min_max(
                rect.right_top() + egui::vec2(-edge, corner),
                rect.max - egui::vec2(0., corner),
            ),
        ),
        (
            ResizeDirection::NorthWest,
            CursorIcon::ResizeNwSe,
            egui::Rect::from_min_size(rect.min, Vec2::splat(corner)),
        ),
        (
            ResizeDirection::NorthEast,
            CursorIcon::ResizeNeSw,
            egui::Rect::from_min_size(
                rect.right_top() - egui::vec2(corner, 0.),
                Vec2::splat(corner),
            ),
        ),
        (
            ResizeDirection::SouthWest,
            CursorIcon::ResizeNeSw,
            egui::Rect::from_min_size(
                rect.left_bottom() - egui::vec2(0., corner),
                Vec2::splat(corner),
            ),
        ),
        (
            ResizeDirection::SouthEast,
            CursorIcon::ResizeNwSe,
            egui::Rect::from_min_size(rect.max - Vec2::splat(corner), Vec2::splat(corner)),
        ),
    ];
    for (index, (direction, cursor, rect)) in regions.into_iter().enumerate() {
        egui::Area::new(egui::Id::new(("window-resize", index)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .movable(false)
            .show(context, |ui| {
                let response = ui
                    .allocate_exact_size(rect.size(), egui::Sense::drag())
                    .1
                    .on_hover_cursor(cursor);
                if response.drag_started_by(egui::PointerButton::Primary) {
                    context.stop_dragging();
                    context.send_viewport_cmd(ViewportCommand::BeginResize(direction));
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn title_drag_hands_the_gesture_to_the_window_manager() {
        let context = egui::Context::default();
        let frame = |events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500., 300.),
                    )),
                    events,
                    ..Default::default()
                },
                title_space,
            );
            output.textures_delta.clear();
            output
        };
        frame(Vec::new());
        frame(Vec::new());
        let pos = egui::pos2(100., 20.);
        frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let output = frame(vec![egui::Event::PointerMoved(pos + egui::vec2(30., 0.))]);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&ViewportCommand::StartDrag)
        );
        assert_eq!(context.dragged_id(), None);
    }
    #[test]
    fn title_space_accepts_double_clicks_inside_a_top_panel() {
        let context = egui::Context::default();
        let frame = |events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500., 300.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::Panel::top("title").show(ui, title_space);
                },
            );
            output.textures_delta.clear();
            output
        };
        frame(Vec::new());
        frame(Vec::new());
        let pos = egui::pos2(100., 20.);
        for _ in 0..2 {
            for pressed in [true, false] {
                let output = frame(vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                if output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .contains(&ViewportCommand::Maximized(true))
                {
                    return;
                }
            }
        }
        panic!("title strip did not receive its double click");
    }
    #[test]
    fn custom_controls_emit_window_commands_and_route_close_to_the_application() {
        let context = egui::Context::default();
        let mut close = false;
        let mut frame = |events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(CONTROLS_WIDTH, 26.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    close |= controls(ui, false);
                },
            );
            output.textures_delta.clear();
            output
        };
        frame(Vec::new());
        frame(Vec::new());
        for (x, expected) in [
            (16., Some(ViewportCommand::Minimized(true))),
            (50., Some(ViewportCommand::Maximized(true))),
            (84., None),
        ] {
            let pos = egui::pos2(x, 13.);
            for pressed in [true, false] {
                let output = frame(vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                if !pressed && let Some(expected) = &expected {
                    assert!(
                        output.viewport_output[&egui::ViewportId::ROOT]
                            .commands
                            .contains(expected)
                    );
                }
            }
        }
        assert!(close);
    }
}
