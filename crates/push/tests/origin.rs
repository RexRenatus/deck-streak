//! SPEC-343 A10: the origin rule (R7). A sender reaches only an origin that is `https:`, or `http:`
//! to a loopback host, with no user, path, query or fragment; that rule is what keeps every test on
//! a loopback fake and every production request on TLS.

use deck_streak_push::{BuildError, Origin};

#[test]
fn a10_an_origin_is_https_or_loopback_http_and_nothing_more() {
    // The refusals first: plain HTTP off loopback, and anything after the host.
    let refused = [
        "http://push.synthetic.invalid",
        "http://push.synthetic.invalid:8080",
        "http://localhost.synthetic.invalid",
        "ftp://push.synthetic.invalid",
        "https://push.synthetic.invalid/3/device",
        "https://push.synthetic.invalid?topic=1",
        "https://push.synthetic.invalid#fragment",
        "https://user@push.synthetic.invalid",
        "https://user:secret@push.synthetic.invalid",
        "https://",
        "https:// push.synthetic.invalid",
        "push.synthetic.invalid",
        "",
    ];
    let wrongly_admitted: Vec<&str> = refused
        .iter()
        .copied()
        .filter(|text| Origin::new(text) != Err(BuildError::Origin))
        .collect();
    println!("examined {} refused origin(s)", refused.len());
    assert_eq!(wrongly_admitted, Vec::<&str>::new());

    // TLS anywhere, and plain HTTP to a loopback host, are admitted, a root slash dropped.
    let admitted = [
        (
            "https://push.synthetic.invalid",
            "https://push.synthetic.invalid",
        ),
        (
            "https://push.synthetic.invalid:8443",
            "https://push.synthetic.invalid:8443",
        ),
        (
            "https://push.synthetic.invalid/",
            "https://push.synthetic.invalid",
        ),
        ("http://127.0.0.1:8080", "http://127.0.0.1:8080"),
        ("http://localhost:3000", "http://localhost:3000"),
        ("http://[::1]:4000", "http://[::1]:4000"),
    ];
    let read: Vec<Option<String>> = admitted
        .iter()
        .map(|(text, _)| {
            Origin::new(text)
                .ok()
                .map(|origin| origin.as_str().to_owned())
        })
        .collect();
    let expected: Vec<Option<String>> = admitted
        .iter()
        .map(|(_, origin)| Some((*origin).to_owned()))
        .collect();
    println!("examined {} admitted origin(s)", admitted.len());
    assert_eq!(read, expected);
}
