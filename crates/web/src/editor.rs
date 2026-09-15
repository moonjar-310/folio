use crate::{components::*, state::*};
use leptos::{prelude::*, task::spawn_local};
use serde_json::json;
pub fn render(markdown: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser, html};
    let parser = Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    )
    .filter(|e| !matches!(e, Event::Html(_) | Event::InlineHtml(_)));
    let mut html = String::new();
    html::push_html(&mut html, parser);
    ammonia::Builder::default()
        .add_tags(&["input"])
        .add_tag_attributes("input", &["type", "checked", "disabled"])
        .clean(&html)
        .to_string()
}
#[component]
pub fn Notes(state: AppState) -> impl IntoView {
    view! {<div class="notes-layout">
        <section class="note-list" aria-label="Notes">
            <button class="search-trigger" on:click=move |_|state.search_open.set(true)><Icon name="search"/><span>"Search all notes…"</span><kbd>"⌘K"</kbd></button>
            <div class="note-list-heading"><span class="eyebrow">{move||if state.folder.get().is_empty(){"All notes".into()}else{state.folder.get()}}</span><span class="meta">"Last edited ↓"</span></div>
            <Show when=move||state.notes.get().is_empty()><p class="empty">"A blank page is a good beginning."</p></Show>
            <For each=move||state.notes.get() key=|n|(n.id.clone(),n.updated_at) children=move|note|{
                let id=note.id.clone();let selected=id.clone();
                view!{<button class="note-item" class:selected=move||state.active.get().id==selected disabled=move||state.busy.get() on:click=move |_|{let id=id.clone();spawn_local(async move{state.open_note(id).await;});}>
                    <strong>{note.title}</strong><p>{note.preview}</p><span class="note-meta"><span>{note.folder}</span><time>{js_sys::Date::new(&(note.updated_at as f64).into()).to_date_string().as_string().unwrap_or_default()}</time></span>
                </button>}
            }/>
            <Show when=move||state.next_notes.get().is_some()><button class="load-more" disabled=move||state.loading.get() on:click=move |_|spawn_local(async move{state.load_notes(true).await;})>"Load more notes"</button></Show>
        </section>
        <section class="editor" aria-label="Note editor">
            <div class="editor-toolbar"><label class="folder-picker"><Icon name="folder"/><select aria-label="Move note to folder" disabled=move||state.busy.get() prop:value=move||state.active.get().folder on:change=move|e|{state.active.update(|n|n.folder=event_target_value(&e));state.mark_dirty();}>
                <For each=move||state.folders.get() key=|f|f.clone() children=move|folder|view!{<option value=folder.clone()>{folder.clone()}</option>}/>
            </select></label>
            <div class="tabs"><button class:selected=move||!state.preview.get() aria-pressed=move||!state.preview.get() on:click=move |_|state.preview.set(false)>"Edit"</button><button class:selected=move||state.preview.get() aria-pressed=move||state.preview.get() on:click=move |_|state.preview.set(true)>"Preview"</button></div>
            <button class="primary" disabled=move||state.busy.get()||!state.dirty.get() on:click=move |_|spawn_local(async move{state.save().await;})>"Save"</button>
            <details class="note-actions"><summary aria-label="Note actions">"···"</summary><div class="action-menu">
                <button disabled=move||state.busy.get() on:click=move |_|{state.active.update(|n|n.folder="Archive".into());state.mark_dirty();spawn_local(async move{state.save().await;});}>"Archive note"</button>
                <button disabled=move||state.busy.get() on:click=move |_|{state.active.update(|n|{n.id=uuid::Uuid::new_v4().to_string();n.revision.clear();});state.mark_dirty();spawn_local(async move{state.save().await;});}>"Save draft as a new note"</button>
                <details><summary class="danger">"Delete note…"</summary><p>"Permanently delete this note and its tasks?"</p><button class="danger" disabled=move||state.busy.get() on:click=move |_|spawn_local(async move{
                    if !state.save().await{return;}
                    let note=state.active.get_untracked();
                    match state.api("DELETE",&format!("/api/notes/{}",note.id),json!({"revision":note.revision})).await{
                        Ok(_)=>{state.remove_tree_note(&note.id);state.tasks.update(|t|t.retain(|t|t.source_note_id.as_deref()!=Some(note.id.as_str())));state.notes.update(|n|n.retain(|n|n.id!=note.id));state.active.set(new_note("Personal".into()));state.message.set("Note deleted".into());},
                        Err(e)=>state.error.set(e),
                    }
                })>"Delete permanently"</button></details>
            </div></details></div>
            <div class="writing-column">
                <Show when=move||!state.preview.get()>
                    <label class="sr-only" for="note-title">"Note title"</label>
                    <input id="note-title" class="note-title-input" placeholder="Untitled note" disabled=move||state.busy.get()
                        prop:value=move||state.active.get().markdown.lines().find(|line|!line.trim().is_empty()).unwrap_or("").trim_start_matches('#').trim().to_string()
                        on:input=move|e|{
                            let title=event_target_value(&e);let note=state.active.get_untracked();
                            state.edit(folio_core::rename_title(&note.markdown,&title));
                        }/>
                    <div class="editor-subtitle meta">"A little space to think · Markdown"</div>
                    <label class="sr-only" for="markdown">"Markdown body"</label>
                    <textarea id="markdown" class="markdown-editor" spellcheck="true" placeholder="# Your next thought\n\nBegin anywhere." prop:value=move||state.active.get().markdown disabled=move||state.busy.get()
                        on:input=move|e|state.edit(event_target_value(&e))
                        on:blur=move |_|spawn_local(async move{state.save().await;})></textarea>
                </Show>
                <Show when=move||state.preview.get()><article class="markdown-preview" inner_html=move||render(&state.active.get().markdown)></article></Show>
                <footer class="editor-footer"><span class="meta">{move||format!("{} words · {} bytes",state.active.get().markdown.split_whitespace().count(),state.active.get().markdown.len())}</span><span role="status" class="meta">{move||state.message.get()}</span></footer>
            </div>
        </section>
    </div>}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_removes_active_html_and_unsafe_links() {
        let rendered = render(
            "# Hello\n\n<script>alert(1)</script>\n\n[bad](javascript:alert(1))\n\n| A | B |\n|---|---|\n| 1 | 2 |",
        );
        assert!(!rendered.contains("<script"));
        assert!(!rendered.contains("href=\"javascript:"));
        assert!(rendered.contains("<table>"));
    }
}
