use crate::{components::Icon, state::*};
use folio_core::{Note, NoteSummary, valid_folder};
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::JsCast;

#[derive(Clone, Default)]
pub struct FolderListing {
    pub items: Vec<NoteSummary>,
    pub next: Option<u64>,
    pub loading: bool,
    pub error: String,
}

#[derive(Clone)]
enum Entry {
    Folder(String, usize),
    File(NoteSummary, usize),
    Footer(String, usize),
}

fn ancestors_open(path: &str, expanded: &BTreeSet<String>) -> bool {
    let mut parent = path;
    while let Some((next, _)) = parent.rsplit_once('/') {
        if !expanded.contains(next) {
            return false;
        }
        parent = next;
    }
    true
}

fn entries(
    folders: &[String],
    expanded: &BTreeSet<String>,
    files: &BTreeMap<String, FolderListing>,
) -> Vec<Entry> {
    fn visit(
        parent: &str,
        depth: usize,
        folders: &[String],
        expanded: &BTreeSet<String>,
        files: &BTreeMap<String, FolderListing>,
        out: &mut Vec<Entry>,
    ) {
        for path in folders
            .iter()
            .filter(|p| p.rsplit_once('/').map(|(parent, _)| parent).unwrap_or("") == parent)
        {
            out.push(Entry::Folder(path.clone(), depth));
            if expanded.contains(path) {
                visit(path, depth + 1, folders, expanded, files, out);
                if let Some(list) = files.get(path) {
                    out.extend(
                        list.items
                            .iter()
                            .cloned()
                            .map(|n| Entry::File(n, depth + 1)),
                    );
                }
                out.push(Entry::Footer(path.clone(), depth + 1));
            }
        }
    }
    let mut out = Vec::new();
    visit("", 0, folders, expanded, files, &mut out);
    out
}

fn focus(id: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = el.focus();
    }
}

fn renamed_path(path: &str, source: &str, target: &str) -> String {
    if path == source || path.starts_with(&format!("{source}/")) {
        format!("{target}{}", &path[source.len()..])
    } else {
        path.to_string()
    }
}

fn preserve_renamed_draft(mut draft: Note, source: &str, target: &str) -> Note {
    draft.folder = renamed_path(&draft.folder, source, target);
    draft
}

impl AppState {
    pub async fn open_tree_file(self, id: String) {
        if self.busy.get_untracked() {
            return;
        }
        self.open_note(id.clone()).await;
        let note = self.active.get_untracked();
        if note.id != id || self.dirty.get_untracked() || !self.error.get_untracked().is_empty() {
            return;
        }
        if self.folder.get_untracked() != note.folder {
            self.folder.set(note.folder);
            self.load_notes(false).await;
        }
    }
    pub fn persist_expanded(self) {
        if let Some(s) = storage() {
            let _ = s.set_item(
                "folio-expanded-folders",
                &serde_json::to_string(&self.expanded.get_untracked()).unwrap_or_default(),
            );
        }
    }
    pub fn expand_ancestors(self, path: &str) {
        self.expanded.update(|set| {
            let mut part = String::new();
            for name in path.split('/') {
                if !part.is_empty() {
                    part.push('/');
                }
                part.push_str(name);
                set.insert(part.clone());
            }
        });
        self.persist_expanded();
    }
    pub fn toggle_folder(self, path: String) {
        self.expanded.update(|set| {
            if !set.remove(&path) {
                set.insert(path);
            }
        });
        self.persist_expanded();
    }
    pub async fn load_folder_files(self, path: String, more: bool) {
        let cache = self.folder_files.get_untracked();
        if cache.get(&path).is_some_and(|v| v.loading) {
            return;
        }
        let offset = if more {
            cache.get(&path).and_then(|v| v.next).unwrap_or(0)
        } else {
            0
        };
        let epoch = self.folder_epoch.get_untracked();
        self.folder_files.update(|all| {
            let entry = all.entry(path.clone()).or_default();
            entry.loading = true;
            entry.error.clear();
        });
        let encoded = js_sys::encode_uri_component(&path)
            .as_string()
            .unwrap_or_default();
        let result = self
            .api(
                "GET",
                &format!("/api/notes?folder={encoded}&exact=true&offset={offset}"),
                Value::Null,
            )
            .await;
        if epoch != self.folder_epoch.get_untracked() {
            return;
        }
        self.folder_files.update(|all| {
            let entry = all.entry(path).or_default();
            entry.loading = false;
            match result {
                Ok(v) => {
                    let items = serde_json::from_value::<Vec<NoteSummary>>(v["items"].clone())
                        .unwrap_or_default();
                    if more {
                        for item in items {
                            entry.items.retain(|old| old.id != item.id);
                            entry.items.push(item);
                        }
                    } else {
                        entry.items = items;
                    }
                    entry.next = v["next_offset"].as_u64();
                }
                Err(e) => entry.error = e,
            }
        });
    }
    pub fn merge_tree_note(self, note: &NoteSummary) {
        self.folders.update(|folders| {
            let mut path = String::new();
            for segment in note.folder.split('/').filter(|s| !s.is_empty()) {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(segment);
                if !folders.contains(&path) {
                    folders.push(path.clone());
                }
            }
            folders.sort();
        });
        self.folder_files.update(|all| {
            for (folder, list) in all.iter_mut() {
                list.items.retain(|old| old.id != note.id);
                if folder == &note.folder {
                    list.items.insert(0, note.clone());
                }
                if list.next.is_some() {
                    list.next = Some(list.items.len() as u64);
                }
            }
        });
    }
    pub fn remove_tree_note(self, id: &str) {
        self.folder_files.update(|all| {
            for list in all.values_mut() {
                list.items.retain(|n| n.id != id);
                if list.next.is_some() {
                    list.next = Some(list.items.len() as u64);
                }
            }
        });
    }
    pub async fn refresh_folders(self) {
        match self.api("GET", "/api/folders", Value::Null).await {
            Ok(v) => self.folders.set(
                v.as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v["path"].as_str().map(String::from))
                    .collect(),
            ),
            Err(e) => self.error.set(e),
        }
        if let Ok(v) = self.api("GET", "/api/folders/rename", Value::Null).await {
            self.folder_rename.set(if v.is_null() {
                None
            } else {
                Some((
                    v["source"].as_str().unwrap_or_default().into(),
                    v["target"].as_str().unwrap_or_default().into(),
                ))
            });
        }
    }
}

#[derive(Clone, PartialEq)]
struct Action {
    kind: &'static str,
    path: String,
}

#[component]
pub fn FolderTree(state: AppState) -> impl IntoView {
    let menu = RwSignal::new(None::<String>);
    let position = RwSignal::new((16, 320));
    let menu_dialog = NodeRef::<leptos::html::Dialog>::new();
    let form_dialog = NodeRef::<leptos::html::Dialog>::new();
    let action = RwSignal::new(None::<Action>);
    let name = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let progress = RwSignal::new(String::new());
    let menu_index = RwSignal::new(0usize);
    let open_menu = Callback::new(move |(path, x, y): (String, i32, i32)| {
        let (w, h) = web_sys::window()
            .map(|w| {
                (
                    w.inner_width()
                        .ok()
                        .and_then(|n| n.as_f64())
                        .unwrap_or(390.) as i32,
                    w.inner_height()
                        .ok()
                        .and_then(|n| n.as_f64())
                        .unwrap_or(844.) as i32,
                )
            })
            .unwrap_or((390, 844));
        position.set((x.clamp(8, (w - 248).max(8)), y.clamp(8, (h - 200).max(8))));
        menu_index.set(0);
        menu.set(Some(path));
    });
    let begin = Callback::new(move |kind: &'static str| {
        let path = menu.get_untracked().unwrap_or_default();
        name.set(if kind == "Rename folder" {
            path.rsplit('/').next().unwrap_or_default().into()
        } else {
            String::new()
        });
        error.set(String::new());
        progress.set(String::new());
        menu.set(None);
        action.set(Some(Action { kind, path }));
    });
    Effect::new(move |_| {
        let expanded = state.expanded.get();
        let folders = state.folders.get();
        let _ = state.folder_epoch.get();
        for path in expanded
            .iter()
            .filter(|p| folders.contains(p) && ancestors_open(p, &expanded))
            .cloned()
        {
            if !state.folder_files.get_untracked().contains_key(&path) {
                spawn_local(async move {
                    state.load_folder_files(path, false).await;
                });
            }
        }
    });
    spawn_local(async move {
        if let Ok(v) = state.api("GET", "/api/folders/rename", Value::Null).await
            && !v.is_null()
        {
            state.folder_rename.set(Some((
                v["source"].as_str().unwrap_or_default().into(),
                v["target"].as_str().unwrap_or_default().into(),
            )));
        }
    });
    Effect::new(move |_| {
        if let Some(d) = menu_dialog.get() {
            if menu.get().is_some() {
                let _ = d.show_modal();
            } else {
                d.close();
            }
        }
    });
    Effect::new(move |_| {
        if let Some(d) = form_dialog.get() {
            if action.get().is_some() {
                let _ = d.show_modal();
                focus("folder-name");
            } else {
                d.close();
            }
        }
    });
    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let Some(a) = action.get_untracked() else {
            return;
        };
        let value = name.get_untracked().trim().to_string();
        if value.contains('/') || !valid_folder(&value) {
            error.set("Enter one name without slashes or special path characters.".into());
            return;
        }
        if state.busy.get_untracked() {
            return;
        }
        spawn_local(async move {
            let resuming = a.kind == "Rename folder"
                && state
                    .folder_rename
                    .get_untracked()
                    .is_some_and(|(source, target)| {
                        source == a.path && target.rsplit('/').next() == Some(value.as_str())
                    });
            if !resuming && !state.save().await {
                error.set(state.error.get_untracked());
                return;
            }
            // A pending hierarchy blocks note writes. Keep a recovered draft out
            // of that write path so it cannot prevent the rename from resuming.
            let preserved_draft =
                (resuming && state.dirty.get_untracked()).then(|| state.active.get_untracked());
            if preserved_draft.is_some() {
                state.epoch.update(|n| *n += 1);
            }
            state.busy.set(true);
            error.set(String::new());
            progress.set("Saving…".into());
            let mut renamed_to = None::<String>;
            let result: Result<(), String> = async {
                if a.kind == "Rename folder" {
                    loop {
                        let v = state
                            .api("PUT", "/api/folders", json!({"path":a.path,"name":value}))
                            .await?;
                        if v["done"].as_bool() == Some(true) {
                            let target = v["path"].as_str().unwrap_or_default().to_string();
                            let remap = |path: &str| renamed_path(path, &a.path, &target);
                            state
                                .expanded
                                .update(|set| *set = set.iter().map(|p| remap(p)).collect());
                            state.persist_expanded();
                            state.folder.update(|path| *path = remap(path));
                            renamed_to = Some(target);
                            break;
                        }
                        progress.set("Renaming notes and subfolders…".into());
                    }
                } else {
                    let parent = if a.path.is_empty() && a.kind == "New file" {
                        "Personal".to_string()
                    } else {
                        a.path.clone()
                    };
                    if a.kind == "New folder" {
                        let path = if parent.is_empty() {
                            value.clone()
                        } else {
                            format!("{parent}/{value}")
                        };
                        let v = state
                            .api("POST", "/api/folders", json!({"path":path}))
                            .await?;
                        state.folders.update(|folders| {
                            for path in v["paths"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|v| v.as_str())
                            {
                                if !folders.iter().any(|p| p == path) {
                                    folders.push(path.into());
                                }
                            }
                            folders.sort();
                        });
                        state.expand_ancestors(&path);
                    } else {
                        let id = uuid::Uuid::new_v4().to_string();
                        let markdown =
                            format!("# {}\n\n", value.strip_suffix(".md").unwrap_or(&value));
                        let v = state
                            .api(
                                "PUT",
                                &format!("/api/notes/{id}"),
                                json!({"markdown":markdown,"folder":parent}),
                            )
                            .await?;
                        let summary: NoteSummary =
                            serde_json::from_value(v).map_err(|e| e.to_string())?;
                        state.merge_tree_note(&summary);
                        state.notes.update(|n| n.insert(0, summary.clone()));
                        state.active.set(Note {
                            id: id.clone(),
                            markdown,
                            folder: parent.clone(),
                            revision: summary.revision,
                        });
                        state.folder.set(parent.clone());
                        state.expand_ancestors(&parent);
                        state.page.set("Notes".into());
                        state.preview.set(false);
                        state.drawer.set(false);
                        state.message.set("File created".into());
                        set_route("Notes", Some(&id), false);
                        state.load_notes(false).await;
                    }
                }
                Ok(())
            }
            .await;
            if a.kind == "Rename folder" {
                state.refresh_folders().await;
                state.folder_epoch.update(|n| *n += 1);
                state.folder_files.set(BTreeMap::new());
                state.load_notes(false).await;
                let current = state.active.get_untracked();
                if preserved_draft.is_none()
                    && !current.revision.is_empty()
                    && let Ok(v) = state
                        .api("GET", &format!("/api/notes/{}", current.id), Value::Null)
                        .await
                    && let Ok(note) = serde_json::from_value::<Note>(v)
                {
                    state.active.set(note);
                }
            }
            state.busy.set(false);
            progress.set(String::new());
            match result {
                Ok(()) => {
                    action.set(None);
                    state.error.set(String::new());
                    if let Some(draft) = preserved_draft {
                        let draft = renamed_to
                            .map(|target| preserve_renamed_draft(draft.clone(), &a.path, &target))
                            .unwrap_or(draft);
                        // Keep the original revision: never silently rebase over
                        // an edit from another tab while retaining our draft.
                        state.active.set(draft);
                        state.backup_draft();
                        state.message.set(
                            "Folder renamed. Unsaved draft preserved; review before saving.".into(),
                        );
                    }
                }
                Err(e) => error.set(e),
            }
        });
    };
    view! {
        <section class="folder-section" aria-label="Folders" on:contextmenu=move|e|{e.prevent_default();open_menu.run((String::new(),e.client_x(),e.client_y()));}>
            <div class="folder-heading"><span class="eyebrow">"Folders"</span><button class="icon-button folder-more" aria-label="Folders actions" aria-haspopup="dialog" on:click=move|e|open_menu.run((String::new(),e.client_x(),e.client_y()))>"⋮"</button></div>
            <Show when=move||state.folder_rename.get().is_some()><button class="rename-resume" on:click=move |_|{if let Some((source,target))=state.folder_rename.get_untracked(){name.set(target.rsplit('/').next().unwrap_or_default().into());error.set(String::new());action.set(Some(Action{kind:"Rename folder",path:source}));}}>"Resume folder rename…"</button></Show>
            <nav class="folder-list" aria-label="Folder and file navigation"><ul class="folder-tree-list">
                <For each=move||entries(&state.folders.get(),&state.expanded.get(),&state.folder_files.get()) key=|e|match e{Entry::Folder(p,_)=>format!("folder:{p}"),Entry::File(n,_)=>format!("file:{}:{}:{}",n.id,n.revision,n.title),Entry::Footer(p,_)=>format!("footer:{p}")} children=move|entry|{
                    match entry {
                        Entry::Folder(path,depth)=>{
                            let path=StoredValue::new(path);let label=path.get_value().rsplit('/').next().unwrap_or_default().to_string();
                            view!{<li class="folder-tree-row" style=format!("padding-left:{}px;min-width:{}px",depth*14,(depth*14+130).max(208)) on:contextmenu=move|e|{e.prevent_default();e.stop_propagation();open_menu.run((path.get_value(),e.client_x(),e.client_y()));}>
                                <button class="folder-toggle" title=path.get_value() aria-label=format!("Folder {}",path.get_value()) aria-expanded=move||state.expanded.get().contains(&path.get_value()).to_string()
                                    on:click=move |_|state.toggle_folder(path.get_value())
                                    on:keydown=move|e|{let p=path.get_value();if e.key()=="ArrowRight"{e.prevent_default();state.expand_ancestors(&p);}else if e.key()=="ArrowLeft"{e.prevent_default();state.expanded.update(|s|{s.remove(&p);});state.persist_expanded();}else if e.key()=="ContextMenu"||(e.shift_key()&&e.key()=="F10"){e.prevent_default();e.stop_propagation();let r=event_target::<web_sys::HtmlElement>(&e).get_bounding_client_rect();open_menu.run((p,r.x() as i32+20,r.bottom() as i32));}}>
                                    <span class="folder-chevron" aria-hidden="true">{move||if state.expanded.get().contains(&path.get_value()){"⌄"}else{"›"}}</span><Icon name="folder"/><span class="folder-name">{label}</span>
                                </button>
                                <button class="folder-more icon-button" aria-label=format!("Actions for {}",path.get_value()) aria-haspopup="dialog" title="Folder actions" on:click=move|e|{let r=event_target::<web_sys::HtmlElement>(&e).get_bounding_client_rect();open_menu.run((path.get_value(),r.x() as i32,r.bottom() as i32));}>"⋮"</button>
                            </li>}.into_any()
                        },
                        Entry::File(note,depth)=>{let id=note.id.clone();let selected=id.clone();let parent=note.folder.clone();view!{<li style=format!("padding-left:{}px;min-width:{}px",depth*14,(depth*14+130).max(208)) on:contextmenu=move|e|{e.prevent_default();e.stop_propagation();open_menu.run((parent.clone(),e.client_x(),e.client_y()));}><button class="tree-file" title=format!("{}/{}.md",note.folder,note.title) class:selected=move||state.page.get()=="Notes"&&state.active.get().id==selected on:click=move |_|{let id=id.clone();spawn_local(async move{state.open_tree_file(id).await;});}><Icon name="notes"/><span class="folder-name">{note.title.clone()}".md"</span></button></li>}.into_any()},
                        Entry::Footer(path,depth)=>{let path=StoredValue::new(path);view!{<li class="tree-footer" style:padding-left=format!("{}px",depth*14)>
                            {move||{let listing=state.folder_files.get().get(&path.get_value()).cloned().unwrap_or_default();if listing.loading{view!{<span role="status">"Loading…"</span>}.into_any()}else if !listing.error.is_empty(){view!{<span role="alert">{listing.error}</span><button on:click=move |_|spawn_local(async move{state.load_folder_files(path.get_value(),false).await;})>"Retry"</button>}.into_any()}else if listing.next.is_some(){view!{<button on:click=move |_|spawn_local(async move{state.load_folder_files(path.get_value(),true).await;})>"More files…"</button>}.into_any()}else if listing.items.is_empty()&&!state.folders.get().iter().any(|p|p.starts_with(&format!("{}/",path.get_value()))){view!{<span>"Empty folder"</span>}.into_any()}else{().into_any()}}}
                        </li>}.into_any()}
                    }
                }/>
            </ul></nav>
        </section>
        <dialog node_ref=menu_dialog class="folder-context" aria-label="Folder actions" style=move||format!("left:{}px;top:{}px",position.get().0,position.get().1) on:cancel=move|_:web_sys::Event|menu.set(None) on:close=move|_:web_sys::Event|menu.set(None)
            on:click=move|e|{if e.target()==e.current_target(){menu.set(None);}}
            on:keydown=move|e|{e.stop_propagation();let count=if menu.get_untracked().is_some_and(|p|!p.is_empty()){3}else{2};let next=match e.key().as_str(){"ArrowDown"=>Some((menu_index.get_untracked()+1)%count),"ArrowUp"=>Some((menu_index.get_untracked()+count-1)%count),"Home"=>Some(0),"End"=>Some(count-1),_=>None};if let Some(i)=next{e.prevent_default();menu_index.set(i);focus(&format!("folder-menu-{i}"));}}>
            <p class="eyebrow">{move||menu.get().filter(|p|!p.is_empty()).unwrap_or("Folders".into())}</p>
            {move||{let kinds=if menu.get().is_some_and(|p|!p.is_empty()){vec!["Rename folder","New folder","New file"]}else{vec!["New folder","New file"]};kinds.into_iter().enumerate().map(|(i,kind)|view!{<button id=format!("folder-menu-{i}") autofocus=i==0 on:focus=move |_|menu_index.set(i) on:click=move |_|begin.run(kind)>{kind}</button>}).collect_view()}}
        </dialog>
        <dialog node_ref=form_dialog class="folder-dialog" aria-labelledby="folder-dialog-title" on:keydown=move|e|e.stop_propagation() on:cancel=move|e:web_sys::Event|{if state.busy.get_untracked(){e.prevent_default();}else{action.set(None);}} on:close=move|_:web_sys::Event|action.set(None)>
            <form on:submit=submit><h2 id="folder-dialog-title">{move||action.get().map(|a|a.kind).unwrap_or("Folder")}</h2><p class="meta">{move||action.get().map(|a|if a.path.is_empty(){if a.kind=="New file"{"Create inside Personal".into()}else{"Create in Folders".into()}}else{a.path}).unwrap_or_default()}</p>
                <label for="folder-name">"Name"</label><input id="folder-name" autofocus required disabled=move||state.busy.get() prop:value=move||name.get() on:input=move|e|name.set(event_target_value(&e))/>
                <Show when=move||!error.get().is_empty()><p role="alert" class="error-text">{move||error.get()}</p></Show><p role="status" class="meta">{move||progress.get()}</p>
                <div class="folder-dialog-actions"><button type="button" disabled=move||state.busy.get() on:click=move |_|action.set(None)>"Cancel"</button><button type="submit" class="primary" disabled=move||state.busy.get()>{move||if action.get().is_some_and(|a|a.kind=="Rename folder"){"Rename"}else{"Create"}}</button></div>
            </form>
        </dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resumed_rename_preserves_draft_content_and_conflict_revision() {
        let draft = Note {
            id: "draft-note".into(),
            markdown: "# 초안\n\nUnsaved text\n- [ ] Keep me".into(),
            folder: "Projects/원본/Notes".into(),
            revision: "original-base-revision".into(),
        };
        let renamed = preserve_renamed_draft(draft.clone(), "Projects/원본", "Projects/New");
        assert_eq!(renamed.folder, "Projects/New/Notes");
        assert_eq!(renamed.markdown, draft.markdown);
        assert_eq!(renamed.id, draft.id);
        assert_eq!(renamed.revision, draft.revision);
        assert_eq!(
            renamed_path("Projects/원본2/Notes", "Projects/원본", "Projects/New"),
            "Projects/원본2/Notes"
        );
    }
    #[test]
    fn collapsed_tree_hides_descendants_and_files() {
        let folders = vec!["A".into(), "A/B".into(), "A/B/C".into(), "AB".into()];
        let mut files = BTreeMap::new();
        files.insert(
            "A/B".into(),
            FolderListing {
                items: vec![NoteSummary {
                    id: "n".into(),
                    folder: "A/B".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        assert_eq!(entries(&folders, &BTreeSet::new(), &files).len(), 2);
        assert!(!ancestors_open("A/B/C", &BTreeSet::from(["A/B".into()])));
        let open = BTreeSet::from(["A".into(), "A/B".into()]);
        let rows = entries(&folders, &open, &files);
        assert!(
            rows.iter()
                .any(|r| matches!(r,Entry::File(n,2) if n.id=="n"))
        );
        assert!(
            rows.iter()
                .any(|r| matches!(r,Entry::Folder(p,2) if p=="A/B/C"))
        );
    }
}
