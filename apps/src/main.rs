use js_sys::{Function, Reflect};
use leptos::prelude::*;
use leptos_meta::*;
use wasm_bindgen::JsCast;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet href="/style.css"/>
        <Commander />
        <NavBottom />
    }
}

fn ipc_post(msg: &str) {
    let Some(win) = web_sys::window() else { return };
    let Ok(ipc) = Reflect::get(&win, &"ipc".into()) else { return };
    let Ok(post_msg) = Reflect::get(&ipc, &"postMessage".into()) else { return };
    if let Some(f) = post_msg.dyn_ref::<Function>() {
        let _ = f.call1(&ipc, &msg.into());
    }
}

// ── Commander ──────────────────────────────────────────────────────────────

#[component]
fn Commander() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let (focused, set_focused) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let q = query.get();
        if !q.is_empty() {
            ipc_post(&format!(r#"{{"type":"command","text":"{}"}}"#,
                q.replace('\\', "\\\\").replace('"', "\\\"")));
            set_query.set(String::new());
        }
    };

    let wrapper_class = move || {
        if focused.get() { "commander-wrapper active" } else { "commander-wrapper" }
    };

    view! {
        <div class="commander-bar">
            <form class=wrapper_class on:submit=on_submit>
                <div class="commander-gradient"></div>
                <input
                    type="text"
                    class="commander-input"
                    placeholder="ask, search, transact..."
                    prop:value=query
                    on:input=move |ev| set_query.set(event_target_value(&ev))
                    on:focus=move |_| set_focused.set(true)
                    on:blur=move |_| set_focused.set(false)
                />
                <div class="commander-line"></div>
            </form>
        </div>
    }
}

// ── Mode switcher ──────────────────────────────────────────────────────────

#[component]
fn NavBottom() -> impl IntoView {
    let nav = move |path: &'static str| {
        move |_: web_sys::MouseEvent| {
            ipc_post(&format!(r#"{{"type":"navigate","path":"{}"}}"#, path));
        }
    };

    view! {
        <div class="nav-bottom">
            <button class="nav-link" on:click=nav("/brain")   >"graph"</button>
            <button class="nav-link" on:click=nav("/terminal")>"terminal"</button>
        </div>
    }
}
