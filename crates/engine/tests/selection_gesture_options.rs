//! Selection gesture preferences persist independently of the drawing and undo history.
use cadcraft_engine::{Session, cmd::options, geom::Vec2};
use serde_json::json;

#[test]
fn two_click_preference_round_trips_and_rejects_unsupported_modes() {
    let mut session = Session::new();
    assert_eq!(options::profile(&session)["PICKDRAG"], 2);
    session.cmdline("PICKDRAG 0").unwrap();
    let saved = options::profile(&session);
    let mut restored = Session::new();
    options::restore(&mut restored, &saved);
    assert!(!restored.settings.pickdrag);
    for invalid in [json!(1), json!(-1), json!(3), json!(0.5), json!(true), json!(null), json!("invalid"), json!([0]), json!(1e100)] {
        assert!(session.execute("options", &json!({"PICKDRAG": invalid})).is_err());
        assert_eq!(options::profile(&session), saved);
    }
    options::restore(&mut restored, &json!({"PICKDRAG": 1}));
    assert!(!restored.settings.pickdrag);
    session.execute("options", &json!({"reset":true})).unwrap();
    assert!(session.settings.pickdrag);
}

#[test]
fn disabling_drag_keeps_click_selection_and_typed_window_selection() {
    let mut s = Session::new();
    s.execute("line", &json!({"points":[[40,40],[60,60]]})).unwrap();
    s.execute("options", &json!({"PICKDRAG":0})).unwrap();
    assert!(!s.begin_window(Vec2::new(30.0, 30.0)));
    assert!(s.pending_window.is_none());
    s.idle_click(Vec2::new(30.0, 30.0), false).unwrap();
    s.idle_click(Vec2::new(70.0, 70.0), false).unwrap();
    assert_eq!(s.selection().len(), 1);
    s.set_selection(Vec::new());
    s.cmdline("SELECT W 30,30 70,70").unwrap();
    s.cmdline("").unwrap();
    assert_eq!(s.selection().len(), 1);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    s.execute("setvar", &json!({"name":"PICKDRAG","value":2})).unwrap();
    assert!(s.begin_window(Vec2::new(30.0, 30.0)));
}
