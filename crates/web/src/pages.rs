use crate::{components::*, state::*};
use folio_core::Task;
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};
#[component]
pub fn Home(state: AppState) -> impl IntoView {
    let adding = RwSignal::new(false);
    let title = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let quick = state.quick;
    Effect::new(move |_| {
        spawn_local(async move {
            state.load_tasks(state.tasks_for_page(), false).await;
        });
    });
    let open = Callback::new(move |id| {
        spawn_local(async move {
            state.open_note(id).await;
        })
    });
    let tasks = Signal::derive(move || {
        state
            .tasks
            .get()
            .into_iter()
            .filter(|t| t.due_date.as_deref().is_some_and(|d| d <= today().as_str()))
            .collect::<Vec<_>>()
    });
    view! {<div class="reading-page home">
        <div class="date-strip"><span class="meta"><span class="dot"></span>{nice_date(&today())}</span><button class="text-button" on:click=move |_|spawn_local(async move{state.daily().await;})>"Open today's Daily Note"<Icon name="arrow"/></button></div>
        <div class="page-intro"><h1>{move||format!("A little space, {}.",if state.username.get().is_empty(){"for you".into()}else{state.username.get()})}</h1><p class="serif-subtitle">"Make room for what matters."</p></div>
        <section>
            <SectionHeading title="What Matters" icon="goal"><button class="text-button" on:click=move |_|adding.update(|v|*v = !*v)><Icon name="add"/>"Add goal"</button></SectionHeading>
            <Show when=move||adding.get()><form class="inline-form" on:submit=move|e|{e.prevent_default();let body=json!({"title":title.get_untracked(),"description":description.get_untracked(),"position":state.goals.get_untracked().len()});spawn_local(async move{if state.mutate_goal(None,body).await.is_ok(){title.try_set(String::new());description.try_set(String::new());adding.try_set(false);}});}>
                <label>"Long-term direction"<input required placeholder="What do you want to make room for?" prop:value=move||title.get() on:input=move|e|title.set(event_target_value(&e))/></label>
                <label>"A little context"<textarea prop:value=move||description.get() on:input=move|e|description.set(event_target_value(&e))></textarea></label>
                <button class="primary" disabled=move||state.pending_goals.get().contains("")>"Create goal"</button>
            </form></Show>
            <Show when=move||!state.goals.get().iter().any(|g|g.status=="active")><p class="empty">"Give your days a direction. Add a goal you want to return to."</p></Show>
            <div class="goals-grid"><For each={move||state.goals.get().into_iter().filter(|g|g.status=="active").collect::<Vec<_>>()} key=|g|(g.id.clone(),g.updated_at,g.title.clone(),g.description.clone(),g.position,g.status.clone()) children=move|goal|view!{<GoalCard goal state/>}/></div>
        </section>
        <section class="agenda"><SectionHeading title="Today's intentions" icon="planner"><span class="meta">{move||format!("{} open",tasks.get().iter().filter(|t|!t.completed).count())}</span></SectionHeading><TaskList state items=tasks/><TaskForm state initial_date=today()/><Show when=move||state.next_tasks.get().is_some()><button class="load-more" on:click=move |_|spawn_local(async move{state.load_tasks(state.tasks_for_page(),true).await;})>"More intentions"</button></Show></section>
        <section class="quick-note"><SectionHeading title="Quick Note" icon="notes"><span class="meta">"A thought worth keeping"</span></SectionHeading>
            <form on:submit=move|e|{e.prevent_default();spawn_local(async move{state.save_quick_note().await;});}>
                <textarea id="quick-note" disabled=move||state.quick_saving.get() aria-label="Quick note" placeholder="Write a fleeting thought or capture an idea…" prop:value=move||quick.get() on:input=move|e|{let value=event_target_value(&e);if let Some(s)=storage(){let _=s.set_item("folio-quick-draft",&value);}quick.set(value);}></textarea>
                <div class="quick-footer"><span class="meta">"Saved as a new note"</span><button class="primary" disabled=move||state.quick_saving.get()||quick.get().trim().is_empty()>"Save note"<Icon name="arrow"/></button></div>
            </form>
        </section>
        <section><SectionHeading title="Recently edited" icon="notes"><button class="text-button" on:click=move |_|spawn_local(async move{state.folder.set(String::new());state.load_notes(false).await;state.navigate("Notes").await;})>"All notes"<Icon name="arrow"/></button></SectionHeading>
            <Show when=move||state.notes.get().is_empty()><p class="empty">"Your notes will find their home here."</p></Show>
            <div class="notes-grid"><For each={move||state.notes.get().into_iter().take(4).collect::<Vec<_>>()} key=|n|(n.id.clone(),n.updated_at) children=move|note|view!{<NoteCard note on_open=open/>}/></div>
        </section>
        <footer class="shortcut-strip"><span><kbd>"⌘N"</kbd>" New note"</span><span><kbd>"⌘K"</kbd>" Search"</span><span><kbd>"⌘S"</kbd>" Save"</span></footer>
    </div>}
}
#[component]
pub fn Todo(state: AppState) -> impl IntoView {
    Effect::new(move |_| {
        let _ = state.task_group.get();
        spawn_local(async move {
            state.load_tasks(state.tasks_for_page(), false).await;
        });
    });
    view! {<div class="wide-page"><div class="page-intro"><span class="eyebrow editorial">"One thing at a time"</span><h1>"Tasks & Commitments"</h1><p>"A place for today's intentions and everything that can wait."</p></div>
        <div class="tabs" role="group" aria-label="Task groups">{["Today","Upcoming","Someday","Completed"].into_iter().map(|group|view!{<button class:selected=move||state.task_group.get()==group aria-pressed=move||state.task_group.get()==group on:click=move |_|state.task_group.set(group.into())>{group}</button>}).collect_view()}</div>
        <div class="todo-layout"><section>
            <h2 class="section-title">{move||state.task_group.get()}</h2>
            <TaskList state items=Signal::derive(move||{
                let today=today();let group=state.task_group.get();
                state.tasks.get().into_iter().filter(|t|match group.as_str(){
                    "Completed"=>t.completed,"Upcoming"=>!t.completed&&t.due_date.as_ref().is_some_and(|d|d>&today),
                    "Someday"=>!t.completed&&t.due_date.is_none(),_=>!t.completed&&t.due_date.as_ref().is_some_and(|d|d<=&today),
                }).collect()
            })/>
            <TaskForm state initial_date=today()/>
            <Show when=move||state.next_tasks.get().is_some()><button class="load-more" on:click=move |_|spawn_local(async move{
                state.load_tasks(state.tasks_for_page(),true).await;
            })>"Load more tasks"</button></Show>
        </section><aside class="margin-note"><span class="eyebrow">"A note to yourself"</span><p class="serif-subtitle">"Keep what matters in view. Let the rest wait its turn."</p><hr/><h3>"From your notes"</h3><p>"Markdown checkboxes appear here when you save a note. Completing a task updates its source note."</p><code>"- [ ] Your next intention"</code></aside></div>
    </div>}
}
#[component]
pub fn Planner(state: AppState) -> impl IntoView {
    let backlog = RwSignal::new(Vec::<Task>::new());
    Effect::new(move |_| {
        let _ = state.date.get();
        let _ = state.weekly.get();
        spawn_local(async move {
            state.load_tasks(state.tasks_for_page(), false).await;
        });
    });
    spawn_local(async move {
        match state
            .api(
                "GET",
                &format!("/api/tasks?group=someday&today={}", today()),
                Value::Null,
            )
            .await
        {
            Ok(v) => backlog.set(serde_json::from_value(v["items"].clone()).unwrap_or_default()),
            Err(e) => state.error.set(e),
        }
    });
    view! {<div class="wide-page planner"><div class="planner-heading"><div class="page-intro"><span class="eyebrow editorial">"The shape of your days"</span><h1>{move||if state.weekly.get(){"Weekly Horizon"}else{"A Focused Day"}}</h1></div>
        <div class="planner-controls"><button aria-label="Previous period" on:click=move |_|state.date.update(|d|*d=shift_date(d,if state.weekly.get_untracked(){-7}else{-1}))>"‹"</button>
        <input aria-label="Planner date" type="date" prop:value=move||state.date.get() on:change=move|e|{let d=event_target_value(&e);if folio_core::valid_date(&d){state.date.set(d);}}/>
        <button aria-label="Next period" on:click=move |_|state.date.update(|d|*d=shift_date(d,if state.weekly.get_untracked(){7}else{1}))>"›"</button>
        <button on:click=move |_|state.date.set(today())>"Today"</button></div>
        <div class="tabs"><button class:selected=move||state.weekly.get() aria-pressed=move||state.weekly.get() on:click=move |_|state.weekly.set(true)>"Week"</button><button class:selected=move||!state.weekly.get() aria-pressed=move||!state.weekly.get() on:click=move |_|state.weekly.set(false)>"Day"</button></div>
        </div>
        <div class="week-grid" class:daily=move||!state.weekly.get()>
            <For each=move||if state.weekly.get(){week(&state.date.get())}else{vec![state.date.get()]} key=|d|d.clone() children=move|day|view!{<PlannerDay state day/>}/>
        </div>
        <Show when=move||state.next_tasks.get().is_some()><button class="load-more" on:click=move |_|spawn_local(async move{state.load_tasks(state.tasks_for_page(),true).await;})>"Load more for this period"</button></Show>
        <section class="unscheduled"><SectionHeading title="Unscheduled intentions" icon="todo"><button class="text-button" on:click=move |_|spawn_local(async move{state.task_group.set("Someday".into());state.navigate("Todo").await;})>"View all unscheduled"</button></SectionHeading><TaskList state items=Signal::derive(move||backlog.get().into_iter().filter_map(|original|{let task=state.tasks.get().into_iter().find(|t|t.id==original.id).unwrap_or(original);if !task.completed&&task.due_date.is_none(){Some(task)}else{None}}).collect())/></section>
    </div>}
}
#[component]
fn PlannerDay(state: AppState, day: String) -> impl IntoView {
    let selected = day.clone();
    let date = day.clone();
    let initial = day.clone();
    let items = Signal::derive(move || {
        state
            .tasks
            .get()
            .into_iter()
            .filter(|t| t.due_date.as_deref() == Some(selected.as_str()))
            .collect()
    });
    let adding = RwSignal::new(false);
    view! {<section class="planner-day" class:today=day==today()><div class="day-label"><span>{nice_date(&day)}</span>{(day==today()).then(||view!{<span class="badge">"Today"</span>})}</div>
        <TaskList state items/>
        <button class="text-button add-day" on:click=move |_|adding.update(|v|*v = !*v)><Icon name="add"/>"Add intention"</button>
        <Show when=move||adding.get()><TaskForm state initial_date=initial.clone()/></Show>
        <button class="text-button day-focus" on:click=move |_|{state.date.set(date.clone());state.weekly.set(false);}>"Focus this day"</button>
    </section>}
}
#[component]
pub fn Settings(state: AppState) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let signing_out = RwSignal::new(false);
    let finish = Callback::new(move |save_quick: bool| {
        if signing_out.get_untracked() || state.quick_saving.get_untracked() {
            return;
        }
        signing_out.set(true);
        spawn_local(async move {
            let signed_out = state.sign_out(save_quick).await;
            signing_out.try_set(false);
            if signed_out && let Some(dialog) = dialog.get_untracked() {
                dialog.close();
            }
        });
    });
    view! {<div class="reading-page settings"><div class="page-intro"><span class="eyebrow">"Make yourself at home"</span><h1>"Your workspace"</h1><p>"A few quiet preferences for your Folio."</p></div>
        <section><SectionHeading title="Appearance" icon="home"><span></span></SectionHeading><div class="setting-row"><div><h3>"Light & dark"</h3><p>"Your choice stays with this browser."</p></div><button aria-pressed=move||state.dark.get() on:click=move |_|state.toggle_theme()>{move||if state.dark.get(){"Switch to light"}else{"Switch to dark"}}</button></div></section>
        <section><SectionHeading title="Folders" icon="folder"><span></span></SectionHeading><p class="muted">"Create folders from the sidebar. Empty folders can be removed here."</p>
        <For each=move||state.folders.get() key=|p|p.clone() children=move|path|{let label=path.clone();view!{<div class="setting-row"><span>{label}</span><button class="text-button danger" on:click=move |_|{let path=path.clone();spawn_local(async move{match state.api("DELETE","/api/folders",json!({"path":path})).await{Ok(_)=>state.folders.update(|f|f.retain(|f|f!=&path&&!f.starts_with(&format!("{path}/")))),Err(e)=>state.error.set(e)}});}>"Remove empty folder"</button></div>}}/>
        </section>
        <section><SectionHeading title="All goals" icon="goal"><span></span></SectionHeading><p class="muted">"Review, reorder, complete, archive, or return to a direction."</p><div class="goals-grid"><For each=move||state.goals.get() key=|g|(g.id.clone(),g.updated_at,g.title.clone(),g.description.clone(),g.position,g.status.clone()) children=move|goal|view!{<GoalCard goal state/>}/></div></section>
        <section><SectionHeading title="File storage" icon="folder"><span></span></SectionHeading>
        <p class="muted">"Folders and Markdown files are the originals. Rebuild search and planner lists after restoring files or editing them outside Folio."</p>
        <div class="setting-row"><span>"Refresh search and planner lists"</span><button disabled=move||state.maintaining.get() on:click=move |_|spawn_local(state.maintain_storage(false))>"Rebuild from files"</button></div>
        <div class="setting-row"><span>"Move files from the older ID-based storage"</span><button disabled=move||state.maintaining.get() on:click=move |_|spawn_local(state.maintain_storage(true))>"Migrate existing files"</button></div>
        </section>
        <section><SectionHeading title="Keyboard shortcuts" icon="notes"><span></span></SectionHeading><dl class="shortcuts"><dt>"Search"</dt><dd><kbd>"Ctrl / ⌘ K"</kbd></dd><dt>"New note"</dt><dd><kbd>"Ctrl / ⌘ N"</kbd></dd><dt>"Save"</dt><dd><kbd>"Ctrl / ⌘ S"</kbd></dd><dt>"Quick Note"</dt><dd><kbd>"Ctrl / ⌘ Shift N"</kbd></dd></dl></section>
        <section><SectionHeading title="Account" icon="settings"><span></span></SectionHeading><div class="setting-row"><span>{move||state.username.get()}</span><button disabled=move||signing_out.get()||state.quick_saving.get() on:click=move |_|{
            if !state.quick.get_untracked().is_empty() {
                if let Some(dialog)=dialog.get_untracked(){let _=dialog.show_modal();}
            } else {finish.run(false);}
        }>"Sign out"</button></div></section>
        <dialog node_ref=dialog class="folder-dialog" aria-labelledby="logout-title" on:cancel=move|e: web_sys::Event|{if signing_out.get_untracked(){e.prevent_default();}}>
            <h2 id="logout-title">"Save your Quick Note?"</h2>
            <p class="meta">"Save this draft as a note, or discard it before signing out. Cancel to keep writing."</p>
            <Show when=move||!state.error.get().is_empty()><p role="alert" class="error-text">{move||state.error.get()}</p></Show>
            <div class="folder-dialog-actions" style="flex-wrap:wrap">
                <button disabled=move||signing_out.get() on:click=move |_|{if let Some(dialog)=dialog.get_untracked(){dialog.close();}}>"Cancel"</button>
                <button class="danger" disabled=move||signing_out.get() on:click=move |_|finish.run(false)>"Discard and sign out"</button>
                <button class="primary" disabled=move||signing_out.get()||state.quick.get().trim().is_empty() on:click=move |_|finish.run(true)>"Save and sign out"</button>
            </div>
        </dialog>
    </div>}
}
