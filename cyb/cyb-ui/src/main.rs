use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div {
            style: "font-family: sans-serif; padding: 2rem; color: #e0e0e0; background: #1a1a2e; min-height: 100vh;",
            h1 { "cyb UI" }
            p { "Dioxus native UI for cyb shell." }
            button {
                onclick: move |_| count += 1,
                "Clicked {count} times"
            }
        }
    }
}
