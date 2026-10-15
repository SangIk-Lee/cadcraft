//! Selection owns a press until release. A frame can contain more than one complete click,
//! and a moving release is still the second corner of an existing click-click window.
use super::{CadApp, Input, Xf, click_grip, grip_at};
use egui::{Event, Modifiers, PointerButton, Pos2, Response};

// Logical points, independent of monitor DPI. Permit hand jitter without closing a tiny window.
const CLICK_SLOP: f32 = 10.0;

#[derive(Clone, Copy)]
pub(super) struct Press {
    origin: Pos2,
    second_corner: bool,
    on_grip: bool,
}

pub(super) fn active(app: &CadApp) -> bool {
    !app.session.prompting() || app.session.current_prompt().is_some_and(|p| p.accept.select)
}

pub(super) fn cancel(app: &mut CadApp) {
    app.canvas.selection_press = None;
    if app.canvas.drag_window {
        app.session.pending_window = None;
        app.canvas.drag_window = false;
    }
}

pub(super) fn input(app: &mut CadApp, xf: &Xf, response: &Response, events: &[Event]) {
    // A preference changed during a drag cancels that old gesture, not a click-click window.
    if !app.session.settings.pickdrag && app.canvas.drag_window {
        cancel(app);
    }
    let owns_pointer = response.hovered() || response.dragged_by(PointerButton::Primary) || response.drag_stopped_by(PointerButton::Primary);
    for event in events {
        if !active(app) {
            cancel(app);
            break;
        }
        match *event {
            Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, .. } => {
                app.canvas.selection_press = (owns_pointer && xf.rect.contains(pos)).then(|| Press {
                    origin: pos,
                    second_corner: app.session.pending_window.is_some(),
                    on_grip: grip_at(app, xf, Some(pos)).is_some() || crate::dyngrips::at(app, xf, Some(pos)).is_some(),
                });
            }
            Event::PointerMoved(pos) => start_drag(app, xf, pos),
            Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers } => {
                start_drag(app, xf, pos);
                let Some(press) = app.canvas.selection_press.take() else { continue };
                let dragged = app.canvas.drag_window;
                app.canvas.drag_window = false;
                // Esc or a command can have cancelled the window since this press began.
                if press.second_corner && app.session.pending_window.is_none() {
                    continue;
                }
                if app.session.settings.pickdrag && !dragged && !xf.rect.contains(pos) {
                    continue;
                }
                // A deliberate grip drag is not an object-selection window.
                if app.session.settings.pickdrag && !press.second_corner && press.on_grip && press.origin.distance(pos) > CLICK_SLOP {
                    continue;
                }
                // In two-click mode a gesture supplies exactly one press position, regardless
                // of travel before release. Never turn that travel into both window corners.
                let at = if app.session.settings.pickdrag && (dragged || press.second_corner) { pos } else { press.origin };
                click(app, xf, at, modifiers);
            }
            Event::PointerGone => cancel(app),
            _ => {}
        }
    }
}

fn start_drag(app: &mut CadApp, xf: &Xf, pos: Pos2) {
    if let Some(press) = app.canvas.selection_press
        && app.session.settings.pickdrag
        && !press.second_corner
        && !press.on_grip
        && !app.canvas.drag_window
        && app.session.pending_window.is_none()
        && press.origin.distance(pos) > CLICK_SLOP
    {
        app.canvas.drag_window = app.session.begin_window(xf.to_world(press.origin));
    }
}

fn click(app: &mut CadApp, xf: &Xf, pos: Pos2, modifiers: Modifiers) {
    let point = xf.to_world(pos);
    let result = if app.session.prompting() {
        app.session.input(Input::Point(point))
    } else if app.session.pending_window.is_some() {
        // The second corner completes the window even if it happens to land on a selected grip.
        app.session.idle_click(point, modifiers.shift)
    } else if let Some(grip) = crate::dyngrips::at(app, xf, Some(pos)) {
        crate::dyngrips::click(app, grip);
        Ok(())
    } else if let Some(grip) = grip_at(app, xf, Some(pos)) {
        click_grip(app, grip, modifiers.shift);
        Ok(())
    } else {
        app.session.idle_click(point, modifiers.shift)
    };
    if let Err(error) = result {
        app.session.echo(error.to_string());
    }
}
