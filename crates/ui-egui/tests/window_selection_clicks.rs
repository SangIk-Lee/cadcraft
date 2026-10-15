//! Two-corner selection must retain releases while the mouse moves or frames are slow.
use cadcraft_engine::{Input, Session};
use cadcraft_geom::Vec2;
use cadcraft_ui_egui::{CadApp, Services};
use egui::{Event, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::Harness;
use serde_json::json;

fn harness(select_command: bool) -> Harness<'static, CadApp> {
    let mut session = Session::new();
    session.new_drawing(false);
    session.execute("line", &json!({"points":[[40,40],[60,60]]})).unwrap();
    session.execute("view.set", &json!({"center":[50,50],"height":100})).unwrap();
    session.settings.osmode = 0;
    session.settings.snapmode = false;
    let mut app = CadApp::new(session, Services::default());
    app.canvas.zoom_pending = false;
    if select_command {
        app.start("select");
    }
    let mut h = Harness::builder().with_size(vec2(1600.0, 1000.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(5);
    h
}

fn screen(h: &Harness<'_, CadApp>, p: Vec2) -> Pos2 {
    h.state().canvas.xf.unwrap().to_screen(p)
}

fn queue_button(h: &mut Harness<'_, CadApp>, pos: Pos2, pressed: bool) {
    h.input_mut().events.push(Event::PointerMoved(pos));
    h.input_mut().events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
}

fn click(h: &mut Harness<'_, CadApp>, pos: Pos2) {
    queue_button(h, pos, true);
    h.run_steps(1);
    queue_button(h, pos, false);
    h.run_steps(1);
}

fn assert_selected(h: &mut Harness<'_, CadApp>, select_command: bool) {
    assert!(h.state().session.pending_window.is_none(), "the second corner must finish the window");
    if select_command {
        h.state_mut().session.input(Input::Enter).unwrap();
    }
    assert_eq!(h.state().session.selection().len(), 1);
    assert_eq!(h.state().session.doc().unwrap().model.len(), 1, "selection must not edit the drawing");
}

#[test]
fn moving_second_click_finishes_window_in_both_directions() {
    for select_command in [false, true] {
        for crossing in [false, true] {
            let mut h = harness(select_command);
            let (a, b) = if crossing { (Vec2::new(70.0, 70.0), Vec2::new(30.0, 30.0)) } else { (Vec2::new(30.0, 30.0), Vec2::new(70.0, 70.0)) };
            let first = screen(&h, a);
            click(&mut h, first);
            assert!(h.state().session.pending_window.is_some());
            let second = screen(&h, b);
            queue_button(&mut h, second, true);
            h.run_steps(1);
            queue_button(&mut h, second + vec2(12.0, 0.0), false);
            h.run_steps(2);
            assert_selected(&mut h, select_command);
        }
    }
}

#[test]
fn both_corners_arriving_in_one_frame_are_not_collapsed() {
    for select_command in [false, true] {
        let mut h = harness(select_command);
        for p in [Vec2::new(30.0, 30.0), Vec2::new(70.0, 70.0)] {
            let pos = screen(&h, p);
            queue_button(&mut h, pos, true);
            queue_button(&mut h, pos, false);
        }
        h.run_steps(2);
        assert_selected(&mut h, select_command);
    }
}

#[test]
fn slight_motion_on_first_click_still_starts_two_corner_selection() {
    let mut h = harness(false);
    let first = screen(&h, Vec2::new(30.0, 30.0));
    queue_button(&mut h, first, true);
    h.run_steps(1);
    queue_button(&mut h, first + vec2(8.0, 0.0), false);
    h.run_steps(1);
    assert!(h.state().session.pending_window.is_some(), "small pointer motion must not consume both corners");
    let second = screen(&h, Vec2::new(70.0, 70.0));
    click(&mut h, second);
    assert_selected(&mut h, false);
}

#[test]
fn deliberate_drag_still_selects_once() {
    let mut h = harness(false);
    let first = screen(&h, Vec2::new(30.0, 30.0));
    let second = screen(&h, Vec2::new(70.0, 70.0));
    queue_button(&mut h, first, true);
    h.run_steps(1);
    h.input_mut().events.push(Event::PointerMoved(second));
    h.run_steps(2);
    if let Ok(directory) = std::env::var("CRAFT_SELECTION_VISUAL_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        h.render().unwrap().save(format!("{directory}/window-preview.png")).unwrap();
    }
    queue_button(&mut h, second, false);
    h.run_steps(2);
    assert_selected(&mut h, false);
    if let Ok(directory) = std::env::var("CRAFT_SELECTION_VISUAL_DIR") {
        h.render().unwrap().save(format!("{directory}/window-selected.png")).unwrap();
    }
}

#[test]
fn release_without_a_canvas_press_does_not_finish_a_pending_window() {
    let mut h = harness(false);
    let first = screen(&h, Vec2::new(30.0, 30.0));
    click(&mut h, first);
    let outside = Pos2::new(5.0, 5.0);
    queue_button(&mut h, outside, true);
    h.run_steps(1);
    let second = screen(&h, Vec2::new(70.0, 70.0));
    queue_button(&mut h, second, false);
    h.run_steps(1);
    assert!(h.state().session.pending_window.is_some());
    assert!(h.state().session.selection().is_empty());
}

#[test]
fn escape_during_a_press_does_not_restart_selection_on_release() {
    let mut h = harness(false);
    let first = screen(&h, Vec2::new(30.0, 30.0));
    click(&mut h, first);
    let second = screen(&h, Vec2::new(70.0, 70.0));
    queue_button(&mut h, second, true);
    h.run_steps(1);
    h.key_press(egui::Key::Escape);
    h.run_steps(1);
    queue_button(&mut h, second, false);
    h.run_steps(1);
    assert!(h.state().session.pending_window.is_none());
    assert!(h.state().session.selection().is_empty());
}

#[test]
fn losing_focus_cancels_a_drag_without_selecting_on_release() {
    let mut h = harness(false);
    let first = screen(&h, Vec2::new(30.0, 30.0));
    let second = screen(&h, Vec2::new(70.0, 70.0));
    queue_button(&mut h, first, true);
    h.run_steps(1);
    h.input_mut().events.push(Event::PointerMoved(second));
    h.run_steps(1);
    assert!(h.state().session.pending_window.is_some());
    h.input_mut().focused = false;
    h.input_mut().events.push(Event::WindowFocused(false));
    h.run_steps(1);
    h.input_mut().focused = true;
    h.input_mut().events.push(Event::WindowFocused(true));
    queue_button(&mut h, second, false);
    h.run_steps(1);
    assert!(h.state().session.pending_window.is_none());
    assert!(h.state().session.selection().is_empty());
}

#[test]
fn second_corner_on_a_selected_grip_completes_the_window_without_editing() {
    let mut h = harness(false);
    let line = h.state().session.doc().unwrap().model.iter().next().unwrap().handle;
    h.state_mut().session.set_selection(vec![line]);
    let first = screen(&h, Vec2::new(30.0, 30.0));
    click(&mut h, first);
    let second = screen(&h, Vec2::new(60.0, 60.0));
    click(&mut h, second);
    assert_selected(&mut h, false);
    assert!(!h.state().session.prompting(), "a pending window must take priority over grip editing");
}

#[test]
fn single_click_on_a_grip_still_starts_grip_edit() {
    let mut h = harness(false);
    let line = h.state().session.doc().unwrap().model.iter().next().unwrap().handle;
    h.state_mut().session.set_selection(vec![line]);
    let grip = screen(&h, Vec2::new(40.0, 40.0));
    click(&mut h, grip);
    assert_eq!(h.state().session.running.as_ref().map(|r| r.id.as_str()), Some("grip"));
}
