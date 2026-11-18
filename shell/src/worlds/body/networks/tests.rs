//! Tests for `networks` — the chain probes, the config form, and the
//! root shortener. Kept beside the module (`networks/tests.rs`) so the
//! source file stays under the house's 500-line cap.

use super::*;

#[test]
fn config_round_trips() {
    let text = "\n[[network]]\nname = \"pussy\"\nurl = \"https://cyb.ai/spacepussy-test\"\n\
                \n[[network]]\nname = \"local\"\nurl = \"http://127.0.0.1:9911/\"\n";
    let nets = parse_config(text);
    assert_eq!(
        nets,
        vec![
            ("pussy".into(), "https://cyb.ai/spacepussy-test".into()),
            ("local".into(), "http://127.0.0.1:9911".into()),
        ]
    );
}

#[test]
fn status_fields_parse_from_soft3_form() {
    let body = "particle: status\nchain: spacepussy-test\nheight: 3\n\
                bbg-root: 3acf50598b5f855224a4d253e5a1395c01d2ee17\n";
    let field = |key: &str| -> Option<String> {
        body.lines()
            .find(|l| l.trim_start().starts_with(key))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string())
    };
    assert_eq!(field("height").unwrap(), "3");
    assert!(field("bbg-root").unwrap().starts_with("3acf5059"));
}

/// The regression for the frozen-h incident: a socket that accepts
/// and then says nothing must cost a bounded timeout, never a thread.
/// (ureq's defaults have NO timeouts; networks::agent adds them.)
#[test]
fn blackhole_times_out_instead_of_hanging() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            // Hold the connection open, send nothing.
            let s = stream;
            std::thread::sleep(Duration::from_secs(120));
            drop(s);
        }
    });
    let t0 = Instant::now();
    let r = step_status_with(&agent(), &format!("http://{addr}"));
    assert!(r.is_err(), "a silent socket must not look like a chain");
    assert!(
        t0.elapsed() < Duration::from_secs(15),
        "timeout too slow: {:?}",
        t0.elapsed()
    );
}

/// Live probe of the first network — run by hand:
/// `cargo test -p cyb net_step_live -- --ignored --nocapture`
#[test]
#[ignore]
fn net_step_live() {
    let (h, root, bytes) =
        step_status_with(&agent(), "https://cyb.ai/spacepussy-test").expect("reachable");
    eprintln!("pussy: h={h} root={root} ({bytes} bytes)");
    assert!(h >= 1);
    assert!(!root.is_empty());
}

#[test]
fn short_root_passes_through_at_or_under_eight() {
    assert_eq!(short_root("abcd1234"), "abcd1234");
    assert_eq!(short_root("abcd"), "abcd");
}

#[test]
fn short_root_truncates_ascii() {
    assert_eq!(short_root("abcd1234ef567890"), "abcd1234..");
}

#[test]
fn short_root_does_not_panic_on_multibyte_head() {
    // A char straddling byte offset 8: byte-index slicing at
    // `root[..8]` would land mid-character and panic before this fix
    // ("123456" is 6 ASCII bytes, then a 3-byte char occupies bytes
    // 6..9, so byte offset 8 falls inside it). `root` is parsed
    // straight out of an untrusted peer's plaintext status response
    // (`field("bbg-root")` above), so a relay can pick this input.
    let root = "123456中789";
    let _ = short_root(root);
}

#[test]
fn short_root_does_not_panic_on_short_multibyte() {
    // Fewer than 8 chars but more than 8 bytes: the old `root.len() <= 8`
    // byte-length check would miss this and fall into the slicing branch.
    let root = "中中中";
    assert_eq!(short_root(root), root);
}
