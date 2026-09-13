use flamewm_x11_desktop::{ICONIC_STATE, X11Desktop};

#[test]
fn iconic_state_is_three() {
    assert_eq!(ICONIC_STATE, 3);
}

#[test]
fn minimize_message_shape_is_icccm_iconic() {
    let window = 0x00AB_CDEF;
    let (target, name, data) = X11Desktop::minimize_message(window);
    assert_eq!(target, window);
    assert_eq!(name, "WM_CHANGE_STATE");
    assert_eq!(data.len(), 5);
    assert_eq!(data[0], ICONIC_STATE);
    assert_eq!(data, [3, 0, 0, 0, 0]);
}

#[test]
fn minimize_source_avoids_raw_unmap() {
    let source = include_str!("../src/lib.rs");
    let body = source
        .split("fn minimize")
        .nth(1)
        .expect("minimize must exist")
        .split("fn maximize")
        .next()
        .expect("minimize body must end before maximize");
    assert!(
        body.contains("WM_CHANGE_STATE"),
        "minimize must use WM_CHANGE_STATE"
    );
    assert!(
        !body.contains("unmap_window"),
        "minimize must not call raw unmap_window"
    );
}

#[test]
fn restore_keeps_activation_path() {
    let source = include_str!("../src/lib.rs");
    assert!(
        source.contains("\"_NET_ACTIVE_WINDOW\""),
        "activation path via _NET_ACTIVE_WINDOW must remain"
    );
}
