use crate::state::*;
use folio_core::{Goal, NoteSummary, Task};
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};
#[component]
pub fn Icon(#[prop(default = "notes")] name: &'static str) -> impl IntoView {
    let svg = match name {
        "home" => include_str!("../../../design/folio/icons/wb_sunny.svg"),
        "planner" => include_str!("../../../design/folio/icons/calendar_month.svg"),
        "todo" => include_str!("../../../design/folio/icons/check_box.svg"),
        "search" => include_str!("../../../design/folio/icons/search.svg"),
        "folder" => include_str!("../../../design/folio/icons/folder.svg"),
        "settings" => include_str!("../../../design/folio/icons/settings.svg"),
        "add" => include_str!("../../../design/folio/icons/add.svg"),
        "menu" => include_str!("../../../design/folio/icons/dock_to_left.svg"),
        "arrow" => include_str!("../../../design/folio/icons/north_east.svg"),
        "goal" => include_str!("../../../design/folio/icons/explore.svg"),
        _ => include_str!("../../../design/folio/icons/menu_book.svg"),
    };
    view! {<span class="icon" aria-hidden="true" inner_html=svg></span>}
}
#[component]
pub fn Brand() -> impl IntoView {
    view! {<span class="brand"><span class="emblem" aria-hidden="true" inner_html=include_str!("../../../design/folio/brand-emblem.svg")></span><span>"Folio"</span></span>}
}
#[component]
pub fn Sidebar(state: AppState) -> impl IntoView {
    view! {
        <aside class="sidebar" class:open=move||state.drawer.get() aria-label="Workspace navigation">
            <div class="brand-row"><Brand/><button class="icon-button mobile-only" aria-label="Close navigation" on:click=move |_|state.drawer.set(false)>"×"</button><span class="meta workspace-label">"Personal"</span></div>
            <button class="search-trigger" on:click=move |_|state.search_open.set(true)><Icon name="search"/><span>"Search or jump…"</span><kbd>"⌘K"</kbd></button>
            <nav aria-label="Main navigation">
                {["Home","Planner","Todo","Notes"].into_iter().map(|page|{
                    let icon=match page{"Home"=>"home","Planner"=>"planner","Todo"=>"todo",_=>"notes"};
                    view!{<a class="nav-item" class:selected=move||state.page.get()==page && state.folder.get().is_empty()
                        aria-current=move||if state.page.get()==page{"page"}else{"false"}
                        href=if page=="Home"{"/".to_string()}else{format!("/{}",page.to_lowercase())}
                        on:click=move|e|{e.prevent_default();spawn_local(async move{if !state.busy.get_untracked() && state.save().await{state.folder.set(String::new());if page=="Notes"{state.load_notes(false).await;}state.navigate(page).await;}});}>
                        <Icon name=icon/><span>{page}</span>
                    </a>}
                }).collect_view()}
            </nav>
            <crate::folder_tree::FolderTree state/>
            <div class="sidebar-bottom">
                <button class="primary new-note" disabled=move||state.busy.get() on:click=move |_|spawn_local(async move{state.create_note().await;})><Icon name="add"/><span>"New Note"</span><kbd>"⌘N"</kbd></button>
                <a class="nav-item" href="/settings" on:click=move|e|{e.prevent_default();spawn_local(async move{state.navigate("Settings").await;});}><Icon name="settings"/><span>"Settings"</span></a>
                <button class="theme-toggle" aria-pressed=move||state.dark.get() on:click=move |_|state.toggle_theme()><Icon name="home"/>{move||if state.dark.get(){"Light appearance"}else{"Dark appearance"}}</button>
            </div>
        </aside>
    }
}
#[component]
pub fn SectionHeading(
    title: &'static str,
    #[prop(default = "notes")] icon: &'static str,
    children: Children,
) -> impl IntoView {
    view! {<div class="section-heading"><h2><Icon name=icon/>{title}</h2><div class="section-actions">{children()}</div></div>}
}
#[component]
pub fn NoteCard(note: NoteSummary, on_open: Callback<String>) -> impl IntoView {
    let id = note.id.clone();
    let edited = js_sys::Date::new(&(note.updated_at as f64).into())
        .to_date_string()
        .as_string()
        .unwrap_or_default();
    view! {<button class="note-card" on:click=move |_|on_open.run(id.clone())>
        <div class="note-card-title"><strong>{note.title}</strong><Icon name="arrow"/></div><p>{note.preview}</p>
        <div class="note-meta"><span>{note.folder}</span><time>{edited}</time></div>
    </button>}
}
#[component]
pub fn TaskRow(
    task: Task,
    current_day: RwSignal<String>,
    busy: Signal<bool>,
    on_change: Callback<(String, Value)>,
    on_open: Callback<String>,
    on_delete: Callback<String>,
) -> impl IntoView {
    let id = task.id.clone();
    let check_id = id.clone();
    let delete_id = StoredValue::new(id.clone());
    let title = RwSignal::new(task.title.clone());
    let date = RwSignal::new(task.due_date.clone().unwrap_or_default());
    let time = RwSignal::new(task.due_time.clone().unwrap_or_default());
    let source = task.source_note_id.clone();
    let has_source = source.is_some();
    view! {
        <div class="task-row" class:completed=task.completed>
            <label class="task-check"><input type="checkbox" aria-label=format!("Complete {}",task.title) prop:checked=task.completed disabled=move||busy.get()
                on:change=move|e|{let checked=event_target_checked(&e);on_change.run((check_id.clone(),json!({"completed":checked})));}/></label>
            <span class="task-title">{task.title.clone()}</span>
            {source.map(|note|view!{<button class="source-link" aria-label="Open source note" on:click=move |_|on_open.run(note.clone())><Icon name="notes"/><span>"Source note"</span></button>})}
            <span class="task-date meta">{move || task.due_date.clone().filter(|d|d!=&current_day.get()).unwrap_or_default()}</span><span class="task-time">{task.due_time.clone().unwrap_or_default()}</span>
            <details class="task-details"><summary aria-label=format!("Edit task {}",task.title)>"···"</summary>
                <form class="task-edit" on:submit=move|e|{e.prevent_default();on_change.run((id.clone(),json!({"title":title.get_untracked(),"due_date":date.get_untracked(),"due_time":time.get_untracked()})));}>
                    <label>"Task"<input required maxlength="500" prop:value=move||title.get() on:input=move|e|title.set(event_target_value(&e))/></label>
                    <label>"Due date"<input type="date" prop:value=move||date.get() on:input=move|e|date.set(event_target_value(&e))/></label>
                    <label>"Time"<input type="time" prop:value=move||time.get() on:input=move|e|time.set(event_target_value(&e))/></label>
                    <button class="primary" disabled=move||busy.get() type="submit">"Update task"</button>
                    <Show when=move||!has_source><button class="danger" type="button" disabled=move||busy.get() on:click=move |_|on_delete.run(delete_id.get_value())>"Delete task"</button></Show>
                </form>
            </details>
        </div>
    }
}
#[component]
pub fn TaskList(state: AppState, items: Signal<Vec<Task>>) -> impl IntoView {
    let change = Callback::new(move |(id, body)| {
        spawn_local(async move {
            state.mutate_task(Some(id), body).await;
        })
    });
    let open = Callback::new(move |id| {
        spawn_local(async move {
            state.open_note(id).await;
        })
    });
    let delete = Callback::new(move |id: String| {
        if state.pending_tasks.get_untracked().contains(&id) {
            return;
        }
        state.pending_tasks.update(|pending| {
            pending.insert(id.clone());
        });
        state.task_version.update(|v| *v += 1);
        spawn_local(async move {
            match state
                .api("DELETE", &format!("/api/tasks/{id}"), json!({}))
                .await
            {
                Ok(_) => state.tasks.update(|t| t.retain(|t| t.id != id)),
                Err(e) => state.error.set(e),
            }
            state.pending_tasks.update(|pending| {
                pending.remove(&id);
            });
            state.task_version.update(|v| *v += 1);
            state.task_query.set(String::new());
        })
    });
    view! {
        <div class="task-list">
            <Show when=move||items.get().is_empty()><p class="empty">"A little breathing room. Add an intention when you're ready."</p></Show>
            <For each=move||items.get() key=|t|(t.id.clone(),t.updated_at,t.completed,t.title.clone()) children=move|task|{let id=task.id.clone();view!{<TaskRow task current_day=state.current_day busy=Signal::derive(move||state.pending_tasks.get().contains(&id)) on_change=change on_open=open on_delete=delete/>}}/>
        </div>
    }
}
#[component]
pub fn TaskForm(
    state: AppState,
    #[prop(default=String::new())] initial_date: String,
    #[prop(default = false)] follow_today: bool,
) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let date = RwSignal::new(initial_date);
    let previous_day = StoredValue::new(state.current_day.get_untracked());
    Effect::new(move |_| {
        let next = state.current_day.get();
        if follow_today && date.get_untracked() == previous_day.get_value() {
            date.set(next.clone());
        }
        previous_day.set_value(next);
    });
    let time = RwSignal::new(String::new());
    view! {<form class="task-form" on:submit=move|e|{
        e.prevent_default();let body=json!({"title":title.get_untracked(),"due_date":date.get_untracked(),"due_time":time.get_untracked()});
        spawn_local(async move{if state.mutate_task(None,body).await{title.try_set(String::new());}});
    }>
        <Icon name="add"/><input class="task-add-title" aria-label="New task" disabled=move||state.pending_tasks.get().contains("") placeholder="Add an intention…" required maxlength="500" prop:value=move||title.get() on:input=move|e|title.set(event_target_value(&e))/>
        <input aria-label="New task due date" disabled=move||state.pending_tasks.get().contains("") type="date" prop:value=move||date.get() on:input=move|e|date.set(event_target_value(&e))/>
        <input aria-label="New task time" disabled=move||state.pending_tasks.get().contains("") type="time" prop:value=move||time.get() on:input=move|e|time.set(event_target_value(&e))/>
        <button type="submit" disabled=move||state.pending_tasks.get().contains("")>"Add"</button>
    </form>}
}
#[component]
pub fn GoalCard(goal: Goal, state: AppState) -> impl IntoView {
    let original = StoredValue::new(goal.clone());
    let title = RwSignal::new(goal.title.clone());
    let description = RwSignal::new(goal.description.clone());
    let position = RwSignal::new(goal.position);
    let status = RwSignal::new(goal.status.clone());
    let editing = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let heading_id = format!("goal-editor-{}", goal.id);
    let pending_id = goal.id.clone();
    let saving = Signal::derive(move || state.pending_goals.get().contains(&pending_id));
    Effect::new(move |_| {
        if let Some(dialog) = dialog.get() {
            if editing.get() {
                let _ = dialog.show_modal();
            } else {
                dialog.close();
            }
        }
    });
    let open = move |_| {
        let goal = original.get_value();
        title.set(goal.title);
        description.set(goal.description);
        position.set(goal.position);
        status.set(goal.status);
        error.set(String::new());
        editing.set(true);
    };
    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        if saving.get_untracked() {
            return;
        }
        let id = original.get_value().id;
        let body = json!({"title":title.get_untracked(),"description":description.get_untracked(),"position":position.get_untracked(),"status":status.get_untracked()});
        error.set(String::new());
        spawn_local(async move {
            match state.mutate_goal(Some(id), body).await {
                Ok(()) => {
                    editing.try_set(false);
                }
                Err(message) => {
                    error.try_set(message);
                }
            }
        });
    };
    view! {
        <article class="goal-card">
            <span class="eyebrow editorial">"Direction"</span><h3>{goal.title}</h3><p>{goal.description}</p>
            <div class="goal-footer"><span class="meta">{goal.status}</span><button class="text-button" aria-haspopup="dialog" on:click=open>"Edit goal"</button></div>
            <dialog node_ref=dialog class="goal-dialog" aria-labelledby=heading_id.clone()
                on:keydown=move|e|e.stop_propagation()
                on:cancel=move|e:web_sys::Event|{if saving.get_untracked(){e.prevent_default();}else{editing.set(false);}}
                on:close=move|_:web_sys::Event|editing.set(false)>
                <form on:submit=submit>
                    <div class="goal-dialog-heading"><div><span class="eyebrow editorial">"Your direction"</span><h2 id=heading_id.clone()>"Edit goal"</h2></div><button class="icon-button" type="button" aria-label="Close goal editor" disabled=move||saving.get() on:click=move |_|editing.set(false)>"×"</button></div>
                    <div class="goal-edit">
                        <label>"Title"<input autofocus required disabled=move||saving.get() prop:value=move||title.get() on:input=move|e|title.set(event_target_value(&e))/></label>
                        <label>"Description"<textarea rows="10" disabled=move||saving.get() prop:value=move||description.get() on:input=move|e|description.set(event_target_value(&e))></textarea></label>
                        <div class="goal-edit-options">
                            <label>"Order"<input type="number" disabled=move||saving.get() prop:value=move||position.get() on:input=move|e|position.set(event_target_value(&e).parse().unwrap_or(0))/></label>
                            <label>"Status"<select disabled=move||saving.get() prop:value=move||status.get() on:change=move|e|status.set(event_target_value(&e))><option value="active">"Active"</option><option value="completed">"Completed"</option><option value="archived">"Archived"</option></select></label>
                        </div>
                        <Show when=move||!error.get().is_empty()><p class="error-text" role="alert">{move||error.get()}</p></Show>
                    </div>
                    <div class="goal-dialog-actions"><button type="button" disabled=move||saving.get() on:click=move |_|editing.set(false)>"Cancel"</button><button class="primary" type="submit" disabled=move||saving.get()>{move||if saving.get(){"Saving…"}else{"Save goal"}}</button></div>
                </form>
            </dialog>
        </article>
    }
}
