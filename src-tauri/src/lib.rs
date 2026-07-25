use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use chrono::Local;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct NoteMeta {
    title: String,
    created: String,
    modified: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    pinned: bool,
}

impl Default for NoteMeta {
    fn default() -> Self {
        Self {
            title: String::new(),
            created: String::new(),
            modified: String::new(),
            x: 200.0,
            y: 200.0,
            width: 320.0,
            height: 420.0,
            pinned: false,
        }
    }
}

#[derive(Serialize)]
struct NoteListItem {
    id: String,
    title: String,
    modified: String,
    preview: String,
    pinned: bool,
    pin_time: String,
}

fn notes_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("StickyNotesData")
}

fn pins_path() -> PathBuf {
    notes_dir().join("_pins.json")
}

fn pin_order_path() -> PathBuf {
    notes_dir().join("_pin_order.json")
}

fn load_pins() -> HashMap<String, String> {
    fs::read_to_string(pins_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_pins(pins: &HashMap<String, String>) {
    if let Ok(json) = serde_json::to_string_pretty(pins) {
        fs::write(pins_path(), json).ok();
    }
}

fn load_pin_order() -> Vec<String> {
    fs::read_to_string(pin_order_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_pin_order(order: &[String]) {
    if let Ok(json) = serde_json::to_string_pretty(order) {
        fs::write(pin_order_path(), json).ok();
    }
}

fn note_path(id: &str) -> PathBuf {
    notes_dir().join(format!("{}.md", id))
}

fn ensure_dir() {
    fs::create_dir_all(notes_dir()).ok();
}

fn parse_file(raw: &str) -> (NoteMeta, String) {
    if raw.starts_with("---\n") {
        if let Some(end_pos) = raw[4..].find("\n---\n") {
            let fm = &raw[4..4 + end_pos];
            let body = &raw[4 + end_pos + 5..];
            let mut meta = NoteMeta::default();
            for line in fm.lines() {
                if let Some((k, v)) = line.split_once(": ") {
                    match k.trim() {
                        "title" => meta.title = v.trim().to_string(),
                        "created" => meta.created = v.trim().to_string(),
                        "modified" => meta.modified = v.trim().to_string(),
                        "x" => meta.x = v.trim().parse().unwrap_or(200.0),
                        "y" => meta.y = v.trim().parse().unwrap_or(200.0),
                        "width" => meta.width = v.trim().parse().unwrap_or(320.0),
                        "height" => meta.height = v.trim().parse().unwrap_or(420.0),
                        "pinned" => meta.pinned = v.trim() == "true",
                        _ => {}
                    }
                }
            }
            if meta.width < 200.0 { meta.width = 320.0; }
            if meta.height < 150.0 { meta.height = 420.0; }
            return (meta, body.to_string());
        }
    }
    (NoteMeta::default(), raw.to_string())
}

fn write_file(id: &str, meta: &NoteMeta, body: &str) -> Result<(), String> {
    ensure_dir();
    let content = format!(
        "---\ntitle: {}\ncreated: {}\nmodified: {}\nx: {}\ny: {}\nwidth: {}\nheight: {}\npinned: {}\n---\n{}",
        meta.title,
        meta.created,
        meta.modified,
        meta.x as i64,
        meta.y as i64,
        meta.width as i64,
        meta.height as i64,
        meta.pinned,
        body
    );
    fs::write(note_path(id), content).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_notes() -> Vec<NoteListItem> {
    ensure_dir();
    let pins = load_pins();
    let mut items: Vec<NoteListItem> = fs::read_dir(notes_dir())
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .filter_map(|e| {
            let path = e.path();
            let id = path.file_stem()?.to_str()?.to_string();
            let raw = fs::read_to_string(&path).ok()?;
            let (meta, body) = parse_file(&raw);
            let title = if !meta.title.is_empty() {
                meta.title.clone()
            } else {
                body.lines()
                    .next()
                    .unwrap_or("无标题")
                    .trim_start_matches('#')
                    .trim()
                    .to_string()
            };
            let title = if title.is_empty() { "无标题".to_string() } else { title };
            let preview: String = body
                .lines()
                .map(|l| l.trim_start_matches(['#', '-', '*', ' ']))
                .filter(|l| !l.is_empty())
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(100)
                .collect();
            let pin_time = pins.get(&id).cloned().unwrap_or_default();
            let is_pinned = !pin_time.is_empty();
            Some(NoteListItem {
                id,
                title,
                modified: meta.modified.clone(),
                preview,
                pinned: is_pinned,
                pin_time,
            })
        })
        .collect();
    // 置顶的排前面；置顶内优先按手动顺序，无手动顺序则按置顶时间倒序；未置顶按修改时间倒序
    let order = load_pin_order();
    items.sort_by(|a, b| {
        match (a.pinned, b.pinned) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (true, true) => {
                if order.is_empty() {
                    b.pin_time.cmp(&a.pin_time)
                } else {
                    let pos_a = order.iter().position(|x| x == &a.id).unwrap_or(usize::MAX);
                    let pos_b = order.iter().position(|x| x == &b.id).unwrap_or(usize::MAX);
                    pos_a.cmp(&pos_b)
                }
            },
            (false, false) => b.modified.cmp(&a.modified),
        }
    });
    items
}

#[tauri::command]
fn toggle_pin(id: String) -> Result<bool, String> {
    ensure_dir();
    let mut pins = load_pins();
    let mut order = load_pin_order();
    if pins.contains_key(&id) {
        pins.remove(&id);
        order.retain(|x| x != &id);
        save_pins(&pins);
        save_pin_order(&order);
        Ok(false)
    } else {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        pins.insert(id.clone(), now);
        order.insert(0, id); // 新置顶的插入最前面
        save_pins(&pins);
        save_pin_order(&order);
        Ok(true)
    }
}

#[tauri::command]
fn reorder_pins(ids: Vec<String>) -> Result<(), String> {
    save_pin_order(&ids);
    Ok(())
}

#[tauri::command]
fn read_note(id: String) -> Result<serde_json::Value, String> {
    let raw = fs::read_to_string(note_path(&id)).map_err(|e| e.to_string())?;
    let (meta, body) = parse_file(&raw);
    Ok(serde_json::json!({ "meta": meta, "content": body }))
}

#[tauri::command]
fn save_note(
    id: String,
    title: String,
    content: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    pinned: bool,
) -> Result<(), String> {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let title = if !title.is_empty() {
        title
    } else {
        content
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim_start_matches(['#', ' '])
            .trim()
            .to_string()
    };
    let created = fs::read_to_string(note_path(&id))
        .ok()
        .map(|raw| parse_file(&raw).0.created)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| now.clone());
    let meta = NoteMeta { title, created, modified: now, x, y, width, height, pinned };
    write_file(&id, &meta, &content)
}

#[tauri::command]
fn create_note() -> String {
    let id = Local::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let meta = NoteMeta { created: now.clone(), modified: now, ..NoteMeta::default() };
    ensure_dir();
    write_file(&id, &meta, "").ok();
    id
}

#[tauri::command]
fn delete_note(id: String) -> Result<(), String> {
    fs::remove_file(note_path(&id)).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_note_window(app: AppHandle, id: String) -> Result<(), String> {
    let label = format!("note-{}", id);
    if let Some(win) = app.get_webview_window(&label) {
        win.show().ok();
        win.set_focus().ok();
        return Ok(());
    }
    let meta = fs::read_to_string(note_path(&id))
        .ok()
        .map(|raw| parse_file(&raw).0)
        .unwrap_or_default();

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("/".into()))
        .title("便签")
        .inner_size(meta.width.max(200.0), meta.height.max(150.0))
        .position(meta.x, meta.y)
        .decorations(false)
        .shadow(true)
        .always_on_top(meta.pinned)
        .resizable(true)
        .min_inner_size(200.0, 150.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_always_on_top(window: tauri::WebviewWindow, value: bool) -> Result<(), String> {
    window.set_always_on_top(value).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_window_position(window: tauri::WebviewWindow) -> Result<serde_json::Value, String> {
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let sf = window.scale_factor().unwrap_or(1.0);
    Ok(serde_json::json!({
        "x": pos.x as f64 / sf,
        "y": pos.y as f64 / sf,
        "width": size.width as f64 / sf,
        "height": size.height as f64 / sf,
    }))
}

#[tauri::command]
fn close_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_notes,
            read_note,
            save_note,
            create_note,
            delete_note,
            open_note_window,
            set_always_on_top,
            get_window_position,
            close_window,
            toggle_pin,
            reorder_pins,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
