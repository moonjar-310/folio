mod components;
mod editor;
mod folder_tree;
mod overlays;
mod pages;
mod state;
use components::*;
use leptos::{prelude::*, task::spawn_local};
use state::*;
use wasm_bindgen::JsCast;
#[component]
fn App() -> impl IntoView {
    let state = AppState::new();
    spawn_local(async move {
        state.start().await;
    });
    let window = web_sys::window().unwrap();
    let unload = gloo_events::EventListener::new_with_options(
        &window,
        "beforeunload",
        gloo_events::EventListenerOptions::enable_prevent_default(),
        move |event| {
            if state.dirty.get_untracked() || !state.quick.get_untracked().trim().is_empty() {
                event.prevent_default();
                if let Some(e) = event.dyn_ref::<web_sys::BeforeUnloadEvent>() {
                    e.set_return_value("");
                }
            }
        },
    );
    let keys = gloo_events::EventListener::new_with_options(
        &window,
        "keydown",
        gloo_events::EventListenerOptions::enable_prevent_default(),
        move |event| {
            let Some(e) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            if e.key() == "Escape" {
                state.drawer.set(false);
                state.search_open.set(false);
            }
            if !state.authenticated.get_untracked() || !(e.ctrl_key() || e.meta_key()) {
                return;
            }
            match e.key().to_lowercase().as_str() {
                "s" => {
                    e.prevent_default();
                    spawn_local(async move {
                        state.save().await;
                    });
                }
                "k" => {
                    e.prevent_default();
                    state.search_open.update(|v| *v = !*v);
                }
                "n" => {
                    e.prevent_default();
                    if e.shift_key() {
                        spawn_local(async move {
                            state.navigate("Home").await;
                            set_timeout(
                                move || {
                                    if let Some(el) = web_sys::window()
                                        .and_then(|w| w.document())
                                        .and_then(|d| d.get_element_by_id("quick-note"))
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                                    {
                                        let _ = el.focus();
                                    }
                                },
                                std::time::Duration::from_millis(50),
                            );
                        });
                    } else {
                        spawn_local(async move {
                            state.create_note().await;
                        });
                    }
                }
                _ => {}
            }
        },
    );
    let pop = gloo_events::EventListener::new(&window, "popstate", move |_| {
        spawn_local(async move {
            if !state.save().await {
                let note = state.active.get_untracked();
                set_route(
                    &state.page.get_untracked(),
                    if state.page.get_untracked() == "Notes" {
                        Some(&note.id)
                    } else {
                        None
                    },
                    true,
                );
                return;
            }
            let path = web_sys::window()
                .unwrap()
                .location()
                .pathname()
                .unwrap_or_default();
            state.page.set(
                match path.trim_matches('/') {
                    "notes" => "Notes",
                    "todo" => "Todo",
                    "planner" => "Planner",
                    "settings" => "Settings",
                    _ => "Home",
                }
                .into(),
            );
        });
    });
    let listeners = StoredValue::new_local((unload, keys, pop));
    on_cleanup(move || listeners.dispose());
    view! {
        <Show when=move||state.ready.get() fallback=||view!{<div class="boot"><Brand/><p>"Opening your workspace…"</p></div>}>
            <Show when=move||state.authenticated.get() fallback=move||view!{<overlays::Auth state/>}>
                <div class="app-shell"><a class="skip-link" href="#main">"Skip to content"</a><Sidebar state/>
                    <Show when=move||state.drawer.get()><button class="drawer-backdrop" aria-label="Close navigation" on:click=move |_|state.drawer.set(false)></button></Show>
                    <main id="main" tabindex="-1" inert=move||state.drawer.get()><header class="topbar"><div class="topbar-title"><button class="icon-button mobile-only" aria-label="Open navigation" aria-expanded=move||state.drawer.get() on:click=move |_|state.drawer.set(true)><Icon name="menu"/></button><Icon name="notes"/><span class="breadcrumb">"Folio / "</span><span>{move||state.page.get()}</span></div><div class="topbar-actions"><span class="save-status" role="status"><span class="dot"></span>{move||if state.busy.get(){"Working…"}else if state.dirty.get(){"Unsaved draft"}else{"Ready"}}</span><button class="icon-button" aria-label="Search notes" on:click=move |_|state.search_open.set(true)><Icon name="search"/></button></div></header>
                    <Show when=move||!state.error.get().is_empty()><div class="error-banner" role="alert"><span>{move||state.error.get()}</span><button on:click=move |_|state.error.set(String::new()) aria-label="Dismiss error">"×"</button></div></Show>
                    <Show when=move||state.loading.get()><div class="loading-line" role="status">"Loading your workspace…"</div></Show>
                    {move||match state.page.get().as_str(){
                        "Notes"=>view!{<editor::Notes state/>}.into_any(),
                        "Todo"=>view!{<pages::Todo state/>}.into_any(),
                        "Planner"=>view!{<pages::Planner state/>}.into_any(),
                        "Settings"=>view!{<pages::Settings state/>}.into_any(),
                        _=>view!{<pages::Home state/>}.into_any(),
                    }}
                    </main><overlays::Search state/>
                </div>
            </Show>
        </Show>
    }
}
fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
