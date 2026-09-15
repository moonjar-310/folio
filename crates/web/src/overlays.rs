use crate::{components::*, state::*};
use folio_core::NoteSummary;
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};
use std::time::Duration;
#[component]
pub fn Auth(state: AppState) -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let token = RwSignal::new(String::new());
    view! {<main class="auth-page"><div class="auth-sheet"><Brand/><span class="eyebrow">"Your personal planner & notebook"</span><h1>{move||if state.setup.get(){"A fresh page."}else{"Welcome back."}}</h1><p>{move||if state.setup.get(){"Create your private workspace to begin."}else{"A quiet place for your plans and thoughts."}}</p>
        <form on:submit=move|e|{
            e.prevent_default();if state.busy.get_untracked(){return;}state.busy.set(true);
            let path=if state.setup.get_untracked(){"/api/auth/setup"}else{"/api/auth/login"};
            let input=json!({"username":username.get_untracked(),"password":password.get_untracked()});let setup_token=token.get_untracked();
            spawn_local(async move{
                let result=async{
                    let response=gloo_net::http::Request::post(path).header("x-setup-token",&setup_token).json(&input).map_err(|e|e.to_string())?.send().await.map_err(|e|e.to_string())?;
                    let data=response.json::<Value>().await.map_err(|e|e.to_string())?;
                    if response.ok(){Ok(data)}else{Err(data["message"].as_str().unwrap_or("Sign in failed").to_string())}
                }.await;
                match result{
                    Ok(v)=>{state.accept_auth(&v);state.setup.set(false);password.set(String::new());token.set(String::new());state.error.set(String::new());state.busy.set(false);state.load_workspace().await;},
                    Err(e)=>state.error.set(e),
                }state.busy.set(false);
            });
        }>
            <label>"Username"<input autofocus autocomplete="username" required maxlength="80" prop:value=move||username.get() on:input=move|e|username.set(event_target_value(&e))/></label>
            <label>"Password"<input type="password" autocomplete=move||if state.setup.get(){"new-password"}else{"current-password"} required minlength=move||if state.setup.get(){12}else{1} maxlength="1024" prop:value=move||password.get() on:input=move|e|password.set(event_target_value(&e))/></label>
            <Show when=move||state.setup.get()&&state.runtime.get()=="cloudflare"><label>"Setup token"<input type="password" required autocomplete="off" prop:value=move||token.get() on:input=move|e|token.set(event_target_value(&e))/></label></Show>
            <button class="primary" disabled=move||state.busy.get()>{move||if state.busy.get(){"Opening your workspace…"}else if state.setup.get(){"Create workspace"}else{"Sign in"}}</button>
        </form><Show when=move||!state.error.get().is_empty()><p role="alert" class="error-text">{move||state.error.get()}</p></Show><button class="text-button" on:click=move |_|state.toggle_theme()>"Change appearance"</button>
    </div></main>}
}
#[component]
pub fn Search(state: AppState) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let searching = RwSignal::new(false);
    let next = RwSignal::new(None::<u64>);
    Effect::new(move |_| {
        if let Some(dialog) = dialog.get() {
            if state.search_open.get() {
                let _ = dialog.show_modal();
            } else {
                dialog.close();
            }
        }
    });
    let search = move |value: String| {
        state.search.set(value);
        state.search_epoch.update(|v| *v += 1);
        let epoch = state.search_epoch.get_untracked();
        searching.set(true);
        set_timeout(
            move || {
                if epoch != state.search_epoch.get_untracked() {
                    return;
                }
                spawn_local(async move {
                    let q = js_sys::encode_uri_component(&state.search.get_untracked())
                        .as_string()
                        .unwrap_or_default();
                    let result = state
                        .api("GET", &format!("/api/notes?q={q}"), Value::Null)
                        .await;
                    if epoch != state.search_epoch.get_untracked() {
                        return;
                    }
                    match result {
                        Ok(v) => {
                            state.results.set(
                                serde_json::from_value(v["items"].clone()).unwrap_or_default(),
                            );
                            next.set(v["next_offset"].as_u64());
                        }
                        Err(e) => state.error.set(e),
                    }
                    searching.set(false);
                });
            },
            Duration::from_millis(350),
        );
    };
    view! {<dialog node_ref=dialog class="search-dialog" aria-labelledby="search-title" on:cancel=move |_:web_sys::Event|state.search_open.set(false) on:close=move |_:web_sys::Event|state.search_open.set(false)>
        <div class="search-heading"><h2 id="search-title">"Find a thought"</h2><button class="icon-button" aria-label="Close search" on:click=move |_|state.search_open.set(false)>"×"</button></div>
        <div class="search-input"><Icon name="search"/><input autofocus aria-label="Search notes" placeholder="Search titles and note content…" prop:value=move||state.search.get() on:input=move|e|search(event_target_value(&e))/></div>
        <div class="search-results" aria-live="polite">
            <Show when=move||searching.get()><p class="empty">"Searching…"</p></Show>
            <Show when=move||!searching.get()&&state.results.get().is_empty()><p class="empty">"Type a few words to find your notes."</p></Show>
            <For each=move||state.results.get() key=|n|n.id.clone() children=move|note:NoteSummary|{let id=note.id.clone();view!{<button class="search-result" on:click=move |_|{let id=id.clone();spawn_local(async move{state.open_note(id).await;});}><strong>{note.title}</strong><p>{note.preview}</p><span class="meta">{note.folder}</span></button>}}/>
            <Show when=move||next.get().is_some()><button class="load-more" on:click=move |_|spawn_local(async move{
                let q=js_sys::encode_uri_component(&state.search.get_untracked()).as_string().unwrap_or_default();
                let offset=next.get_untracked().unwrap_or(0);
                match state.api("GET",&format!("/api/notes?q={q}&offset={offset}"),Value::Null).await{
                    Ok(v)=>{let notes=serde_json::from_value::<Vec<NoteSummary>>(v["items"].clone()).unwrap_or_default();state.results.update(|r|r.extend(notes));next.set(v["next_offset"].as_u64());},
                    Err(e)=>state.error.set(e),
                }
            })>"More results"</button></Show>
        </div><div class="search-footer meta">"Search your words. Return to your work."</div>
    </dialog>}
}
