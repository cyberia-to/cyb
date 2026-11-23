//! The `net ...` verbs of the commander — the one place the network
//! config is edited by hand. Split out of `networks.rs` so that file
//! stays under the 500-line limit (cyberia/quality.md).

use super::{load_config, save_config, short_root, NetHub};

/// `net ...` from com. Returns what to say; the hub reloads after edits.
pub fn handle_command(rest: &str, hub: &NetHub) -> String {
    let rest = rest.trim();
    let mut nets = load_config();
    if rest.is_empty() || rest == "list" {
        if nets.is_empty() {
            return "net: none configured - net add <name> <url>".into();
        }
        let states = hub.snapshot();
        return nets
            .iter()
            .map(|(n, u)| {
                let st = states.iter().find(|s| &s.name == n);
                match st {
                    Some(s) if s.height > 0 => {
                        format!("{n} {u} - h={} {}", s.height, short_root(&s.root))
                    }
                    _ => format!("{n} {u} - not reached yet"),
                }
            })
            .collect::<Vec<_>>()
            .join("  |  ");
    }
    if let Some(spec) = rest.strip_prefix("add ") {
        let Some((name, url)) = split_name_url(spec) else {
            return "net add <name> <url>".into();
        };
        if nets.iter().any(|(n, _)| *n == name) {
            return format!("net: {name} exists - net set {name} <url> to change it");
        }
        nets.push((name.clone(), url));
        return apply(&nets, hub, format!("net: {name} added"));
    }
    if let Some(spec) = rest.strip_prefix("set ") {
        let Some((name, url)) = split_name_url(spec) else {
            return "net set <name> <url>".into();
        };
        match nets.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => {
                entry.1 = url;
                apply(&nets, hub, format!("net: {name} repointed"))
            }
            None => format!("net: no network named {name}"),
        }
    } else if let Some(name) = rest.strip_prefix("rm ") {
        let name = name.trim();
        let before = nets.len();
        nets.retain(|(n, _)| n != name);
        if nets.len() == before {
            return format!("net: no network named {name}");
        }
        apply(&nets, hub, format!("net: {name} gone"))
    } else {
        "net [list]  |  net add <name> <url>  |  net set <name> <url>  |  net rm <name>".into()
    }
}

fn split_name_url(spec: &str) -> Option<(String, String)> {
    let mut it = spec.split_whitespace();
    let name = it.next()?.to_string();
    let url = it.next()?.trim_end_matches('/').to_string();
    (!url.is_empty()).then_some((name, url))
}

fn apply(nets: &[(String, String)], hub: &NetHub, ok: String) -> String {
    match save_config(nets) {
        Ok(()) => {
            hub.reload();
            ok
        }
        Err(e) => format!("net: {e}"),
    }
}
