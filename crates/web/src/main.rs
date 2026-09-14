use folio_core::{ApiError, MAX_MARKDOWN_BYTES, Note, NoteSummary, SaveNote};
use gloo_net::http::Request;
use leptos::{prelude::*, task::spawn_local};
use wasm_bindgen::JsCast;

async fn decode<T: serde::de::DeserializeOwned>(
    response: gloo_net::http::Response,
) -> Result<T, String> {
    if !response.ok() {
        return Err(response
            .json::<ApiError>()
            .await
            .map(|e| e.message)
            .unwrap_or_else(|_| format!("Request failed ({})", response.status())));
    }
    response
        .json()
        .await
        .map_err(|_| "Could not read the server response".into())
}

fn can_discard(dirty: bool) -> bool {
    !dirty
        || web_sys::window().is_some_and(|w| {
            w.confirm_with_message("Discard unsaved changes?")
                .unwrap_or(false)
        })
}

#[component]
fn App() -> impl IntoView {
    let notes = RwSignal::new(Vec::<NoteSummary>::new());
    let active = RwSignal::new(uuid::Uuid::new_v4().to_string());
    let markdown = RwSignal::new(String::new());
    let dirty = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let loading = RwSignal::new(true);
    let message = RwSignal::new("Loading notes…".to_string());
    let dark = RwSignal::new(false);

    // One list request on mount. No polling or effect-driven fetch on each keystroke.
    spawn_local(async move {
        let result = async {
            decode::<Vec<NoteSummary>>(
                Request::get("/api/notes")
                    .send()
                    .await
                    .map_err(|e| e.to_string())?,
            )
            .await
        }
        .await;
        match result {
            Ok(items) => {
                notes.set(items);
                message.set("Ready".into());
            }
            Err(e) => message.set(e),
        }
        loading.set(false);
    });

    let unload = gloo_events::EventListener::new(
        &web_sys::window().unwrap(),
        "beforeunload",
        move |event| {
            if dirty.get_untracked() {
                event.prevent_default();
                if let Some(event) = event.dyn_ref::<web_sys::BeforeUnloadEvent>() {
                    event.set_return_value("");
                }
            }
        },
    );
    // Kept for the lifetime of this root component; no leaked browser callbacks.
    let unload = StoredValue::new_local(unload);
    on_cleanup(move || {
        unload.dispose();
    });

    let new_note = move |_| {
        if can_discard(dirty.get_untracked()) {
            active.set(uuid::Uuid::new_v4().to_string());
            markdown.set(String::new());
            dirty.set(false);
            message.set("New note".into());
        }
    };

    let save = move |_| {
        if loading.get_untracked() || busy.get_untracked() || !dirty.get_untracked() {
            return;
        }
        let text = markdown.get_untracked();
        if text.len() > MAX_MARKDOWN_BYTES {
            message.set("Note exceeds 128 KiB".into());
            return;
        }
        busy.set(true);
        message.set("Saving…".into());
        let id = active.get_untracked();
        spawn_local(async move {
            let result = async {
                let request = Request::put(&format!("/api/notes/{id}"))
                    .json(&SaveNote { markdown: text })
                    .map_err(|e| e.to_string())?;
                decode::<NoteSummary>(request.send().await.map_err(|e| e.to_string())?).await
            }
            .await;
            match result {
                Ok(summary) => {
                    notes.update(|items| {
                        items.retain(|n| n.id != summary.id);
                        items.insert(0, summary);
                        items.truncate(50);
                    });
                    dirty.set(false);
                    message.set("Saved".into());
                }
                Err(e) => message.set(format!("{e} Your draft is still here.")),
            }
            busy.set(false);
        });
    };

    view! {
        <div class:dark=move || dark.get() class="app">
            <aside class="sidebar">
                <a class="brand" href="/">"▤ " <span>"Folio"</span></a>
                <span class="eyebrow">"YOUR WORKSPACE"</span>
                <span class="nav-selected">"Notes"</span>
                <button class="primary" on:click=new_note disabled=move || busy.get()>"+ New note"</button>
                <button class="theme" on:click=move |_| dark.update(|v| *v = !*v)>
                    {move || if dark.get() { "Light appearance" } else { "Dark appearance" }}
                </button>
            </aside>
            <main>
                <header><span>"Folio / Notes"</span><span role="status" aria-live="polite">{move || message.get()}</span></header>
                <div class="notes-layout">
                    <section class="note-list" aria-label="Recent notes">
                        <h2>"Recent notes"</h2>
                        <Show when=move || !loading.get() && notes.get().is_empty()><p class="muted">"Your notes will appear here."</p></Show>
                        <For each=move || notes.get() key=|note| note.id.clone() children=move |note| {
                            let id = note.id.clone();
                            let selected_id = id.clone();
                            view! {
                                <button class="note-item" class:selected=move || active.get() == selected_id disabled=move || busy.get()
                                    on:click=move |_| {
                                        if active.get_untracked() == id || !can_discard(dirty.get_untracked()) { return; }
                                        let id = id.clone();
                                        busy.set(true);
                                        message.set("Opening…".into());
                                        spawn_local(async move {
                                            let result = async { decode::<Note>(Request::get(&format!("/api/notes/{id}")).send().await.map_err(|e| e.to_string())?).await }.await;
                                            match result {
                                                Ok(note) => { active.set(note.id); markdown.set(note.markdown); dirty.set(false); message.set("Ready".into()); }
                                                Err(e) => message.set(e),
                                            }
                                            busy.set(false);
                                        });
                                    }>
                                    <strong>{note.title}</strong><span>{note.preview}</span>
                                </button>
                            }
                        }/>
                    </section>
                    <section class="editor" aria-label="Markdown editor">
                        <div class="editor-heading"><div><span class="eyebrow">"A LITTLE SPACE TO THINK"</span><h1>"Your next thought."</h1></div>
                            <button class="primary" on:click=save disabled=move || loading.get() || busy.get() || !dirty.get()>
                                {move || if busy.get() { "Please wait…" } else { "Save note" }}
                            </button>
                        </div>
                        <label for="markdown">"Markdown"</label>
                        <textarea id="markdown" spellcheck="true" placeholder="# A new thought" prop:value=move || markdown.get()
                            disabled=move || busy.get()
                            on:input=move |event| { markdown.set(event_target_value(&event)); dirty.set(true); message.set("Unsaved changes".into()); }></textarea>
                        <p class="muted">{move || format!("{} bytes · {}", markdown.get().len(), if dirty.get() { "Unsaved" } else { "Up to date" })}</p>
                    </section>
                </div>
            </main>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
