use super::*;
pub(crate) fn task_text(text: &str) -> (String, Option<String>, Option<String>) {
    let mut title = text.to_string();
    let mut date = None;
    let mut time = None;
    for (marker, is_date) in [(" @due(", true), (" @time(", false)] {
        if let Some(start) = title.find(marker)
            && let Some(end) = title[start + marker.len()..].find(')')
        {
            let end = start + marker.len() + end;
            let value = title[start + marker.len()..end].to_string();
            if (is_date && valid_date(&value)) || (!is_date && valid_time(&value)) {
                title.replace_range(start..=end, "");
                if is_date {
                    date = Some(value)
                } else {
                    time = Some(value)
                }
            }
        }
    }
    (title.trim().into(), date, time)
}
pub(crate) fn task_statement(t: &Task) -> Statement {
    statement(
        "INSERT INTO tasks(id,title,completed,due_date,due_time,source_note_id,source_line,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET title=excluded.title,completed=excluded.completed,due_date=excluded.due_date,due_time=excluded.due_time,source_line=excluded.source_line,updated_at=excluded.updated_at",
        vec![
            json!(t.id),
            json!(t.title),
            json!(t.completed),
            json!(t.due_date),
            json!(t.due_time),
            json!(t.source_note_id),
            json!(t.source_line),
            json!(t.created_at),
            json!(t.updated_at),
        ],
    )
}
impl<S: Store> Application<S> {
    pub(crate) async fn list_tasks(
        &mut self,
        q: &std::collections::HashMap<String, String>,
    ) -> Result<Value> {
        let from = q.get("from").cloned().unwrap_or_default();
        let to = q.get("to").cloned().unwrap_or_default();
        if (!from.is_empty() && !valid_date(&from)) || (!to.is_empty() && !valid_date(&to)) {
            return Err(fail(400, "Invalid date"));
        }
        let offset = q
            .get("offset")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0)
            .min(100_000);
        let group = q.get("group").map(String::as_str).unwrap_or("");
        let today = q.get("today").map(String::as_str).unwrap_or("");
        if !group.is_empty() && !valid_date(today) {
            return Err(fail(400, "A valid current date is required"));
        }
        let filter = match group {
            "today" => " AND due_date<=?4",
            "upcoming" => " AND completed=0 AND due_date>?4",
            "someday" => " AND completed=0 AND due_date IS NULL AND ?4<>''",
            "completed" => " AND completed=1 AND ?4<>''",
            "" => "",
            _ => return Err(fail(400, "Unknown task group")),
        };
        let sql = format!(
            "SELECT * FROM tasks WHERE (?1='' OR due_date>=?1) AND (?2='' OR due_date<=?2) {filter} ORDER BY completed,due_date IS NULL,due_date,due_time,created_at,id LIMIT 201 OFFSET ?3"
        );
        let mut params = vec![json!(from), json!(to), json!(offset)];
        if !group.is_empty() {
            params.push(json!(today));
        }
        let rows = self.store.query(statement(&sql, params)).await?;
        let more = rows.len() > 200;
        Ok(
            json!({"items":normalize(rows.into_iter().take(200).collect(),&["completed"]),"next_offset":if more{Some(offset+200)}else{None}}),
        )
    }
    pub(crate) async fn save_task(&mut self, id: Option<&str>, input: Value) -> Result<Value> {
        let mut task = if let Some(id) = id {
            if !valid_id(id) {
                return Err(fail(400, "Invalid task ID"));
            }
            let canonical = self.record(&format!(".folio/tasks/{id}.json")).await?;
            let row = self
                .store
                .query(statement(
                    "SELECT * FROM tasks WHERE id=?1",
                    vec![json!(id)],
                ))
                .await?;
            serde_json::from_value::<Task>(
                canonical
                    .or_else(|| normalize(row, &["completed"]).into_iter().next())
                    .ok_or_else(|| fail(404, "Task not found"))?,
            )
            .map_err(|_| fail(503, "Invalid task index"))?
        } else {
            Task {
                id: uid(),
                created_at: self.now,
                ..Default::default()
            }
        };
        let original_title = task.title.clone();
        if let Some(title) = input["title"].as_str() {
            task.title = title.trim().into();
        }
        if task.title.is_empty() || task.title.len() > 500 || task.title.contains(['\n', '\r']) {
            return Err(fail(400, "Task title must be 1–500 bytes on one line"));
        }
        if let Some(completed) = input["completed"].as_bool() {
            task.completed = completed;
        }
        for (key, date) in [("due_date", true), ("due_time", false)] {
            if let Some(value) = input.get(key) {
                let value = if value.is_null() || value.as_str() == Some("") {
                    None
                } else {
                    Some(
                        value
                            .as_str()
                            .ok_or_else(|| fail(400, "Invalid due date/time"))?
                            .to_string(),
                    )
                };
                if value
                    .as_ref()
                    .is_some_and(|v| if date { !valid_date(v) } else { !valid_time(v) })
                {
                    return Err(fail(400, "Invalid due date/time"));
                }
                if date {
                    task.due_date = value
                } else {
                    task.due_time = value
                }
            }
        }
        task.updated_at = self.now;
        if let Some(ref note_id) = task.source_note_id {
            let note: Note = serde_json::from_value(self.read_note(note_id).await?)
                .map_err(|_| fail(503, "Invalid note"))?;
            let line = task
                .source_line
                .ok_or_else(|| fail(409, "Task index needs repair. Save the source note."))?;
            let parsed = checkboxes(&note.markdown);
            if !parsed
                .iter()
                .any(|(n, text, _)| *n == line && task_text(text).0 == original_title)
            {
                return Err(fail(
                    409,
                    "Source note changed. Open and save it before editing this task.",
                ));
            }
            let mut lines: Vec<String> = note
                .markdown
                .split_inclusive('\n')
                .map(String::from)
                .collect();
            let old = &lines[line];
            let indent = old.len() - old.trim_start().len();
            let ending = if old.ends_with("\r\n") {
                "\r\n"
            } else if old.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            let due = task
                .due_date
                .as_ref()
                .map(|d| format!(" @due({d})"))
                .unwrap_or_default();
            let time = task
                .due_time
                .as_ref()
                .map(|d| format!(" @time({d})"))
                .unwrap_or_default();
            lines[line] = format!(
                "{}- [{}] {}{due}{time}{ending}",
                &old[..indent],
                if task.completed { "x" } else { " " },
                task.title
            );
            let summary = self
                .save_note(
                    note_id,
                    SaveNote {
                        markdown: lines.concat(),
                        folder: note.folder,
                        revision: Some(note.revision),
                    },
                )
                .await?;
            // Reindex preserves IDs by title or line; read the authoritative updated record.
            let rows = self
                .store
                .query(statement(
                    "SELECT * FROM tasks WHERE source_note_id=?1 AND source_line=?2",
                    vec![json!(note_id), json!(line)],
                ))
                .await?;
            let mut result = normalize(rows, &["completed"])
                .into_iter()
                .next()
                .ok_or_else(|| fail(409, "Task was removed while saving."))?;
            result["note_summary"] = summary;
            return Ok(result);
        }
        self.put_record(&format!(".folio/tasks/{}.json", task.id), &json!(task))
            .await?;
        self.store.execute(task_statement(&task)).await?;
        Ok(json!(task))
    }
    pub(crate) async fn delete_task(&mut self, id: &str) -> Result<Value> {
        let rows = self
            .store
            .query(statement(
                "SELECT source_note_id FROM tasks WHERE id=?1",
                vec![json!(id)],
            ))
            .await?;
        let row = rows.first().ok_or_else(|| fail(404, "Task not found"))?;
        if !row["source_note_id"].is_null() {
            return Err(fail(409, "Remove this checkbox in its source note."));
        }
        self.store
            .delete(&format!(".folio/tasks/{id}.json"))
            .await?;
        self.store
            .execute(statement("DELETE FROM tasks WHERE id=?1", vec![json!(id)]))
            .await?;
        Ok(json!({"deleted":true}))
    }
    pub(crate) async fn save_goal(&mut self, id: Option<&str>, input: Value) -> Result<Value> {
        let mut goal = if let Some(id) = id {
            if !valid_id(id) {
                return Err(fail(400, "Invalid goal ID"));
            }
            let canonical = self.record(&format!(".folio/goals/{id}.json")).await?;
            let rows = self
                .store
                .query(statement(
                    "SELECT * FROM goals WHERE id=?1",
                    vec![json!(id)],
                ))
                .await?;
            serde_json::from_value::<Goal>(
                canonical
                    .or_else(|| rows.into_iter().next())
                    .ok_or_else(|| fail(404, "Goal not found"))?,
            )
            .map_err(|_| fail(503, "Invalid goal"))?
        } else {
            Goal {
                id: uid(),
                created_at: self.now,
                status: "active".into(),
                ..Default::default()
            }
        };
        if let Some(s) = input["title"].as_str() {
            goal.title = s.trim().into();
        }
        if let Some(s) = input["description"].as_str() {
            goal.description = s.into();
        }
        if let Some(s) = input["status"].as_str() {
            goal.status = s.into();
        }
        if let Some(p) = input["position"].as_i64() {
            goal.position = p;
        }
        if goal.title.is_empty()
            || goal.title.len() > 300
            || goal.description.len() > 4000
            || !["active", "completed", "archived"].contains(&goal.status.as_str())
        {
            return Err(fail(400, "Invalid goal fields"));
        }
        goal.updated_at = self.now;
        self.put_record(&format!(".folio/goals/{}.json", goal.id), &json!(goal))
            .await?;
        self.store.execute(statement("INSERT INTO goals(id,title,description,position,status,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET title=excluded.title,description=excluded.description,position=excluded.position,status=excluded.status,updated_at=excluded.updated_at",vec![json!(goal.id),json!(goal.title),json!(goal.description),json!(goal.position),json!(goal.status),json!(goal.created_at),json!(self.now)])).await?;
        Ok(json!(goal))
    }
}
