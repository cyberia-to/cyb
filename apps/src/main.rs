use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::path;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet href="/style.css"/>
        <Router>
            <div class="app">
                <main class="content">
                    <Routes fallback=|| view! { <p class="not-found">"404"</p> }>
                        <Route path=path!("/") view=OraclePage/>
                        <Route path=path!("/search/:query") view=SearchPage/>
                        <Route path=path!("/particles") view=ParticlesPage/>
                        <Route path=path!("/graph") view=GraphPage/>
                        <Route path=path!("/settings") view=SettingsPage/>
                    </Routes>
                </main>
                <Commander/>
            </div>
        </Router>
    }
}

#[component]
fn Commander() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let (focused, set_focused) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let q = query.get();
        if !q.is_empty() {
            let navigate = leptos_router::hooks::use_navigate();
            navigate(&format!("/search/{}", q), Default::default());
        }
    };

    let wrapper_class = move || {
        if focused.get() {
            "commander-wrapper active"
        } else {
            "commander-wrapper"
        }
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

#[component]
fn OraclePage() -> impl IntoView {
    view! {
        <div class="oracle">
            <h1 class="oracle-title">"cyb"</h1>
            <p class="oracle-sub">"your immortal robot for the great web"</p>
        </div>
    }
}

#[component]
fn SearchPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let query = move || {
        params.with(|p| p.get("query").unwrap_or_default())
    };

    view! {
        <div class="search">
            <h2 class="search-query">{query}</h2>
            <div class="search-results">
                <p class="placeholder">"searching the knowledge graph..."</p>
            </div>
        </div>
    }
}

#[component]
fn ParticlesPage() -> impl IntoView {
    view! {
        <div class="page">
            <h2>"particles"</h2>
            <p class="placeholder">"cyberlinks and particles explorer"</p>
        </div>
    }
}

#[component]
fn GraphPage() -> impl IntoView {
    view! {
        <div class="page">
            <h2>"graph"</h2>
            <p class="placeholder">"knowledge graph visualization"</p>
        </div>
    }
}

#[component]
fn SettingsPage() -> impl IntoView {
    view! {
        <div class="page">
            <h2>"settings"</h2>
            <p class="placeholder">"keys, backends, preferences"</p>
        </div>
    }
}
