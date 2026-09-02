use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use chrono::Local;

static NOTE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize, Clone, Debug)]
struct NoteMeta {
    title: String,
    created: String,
    modified: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    floating: bool,
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
            floating: false,
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
    error: bool,
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

// 以 _pins.json 为准则，纠正 _pin_order.json 的不一致：
// - order 中多余的 id（不在 pins 里）剔除
// - pins 里有但 order 缺失的 id 追加到末尾（按置顶时间倒序）
fn reconcile_pins() {
    let pins = load_pins();
    let order = load_pin_order();
    let mut cleaned: Vec<String> = order
        .iter()
        .filter(|id| pins.contains_key(*id))
        .cloned()
        .collect();
    let mut changed = cleaned.len() != order.len();
    let mut missing: Vec<String> = pins
        .keys()
        .filter(|id| !cleaned.iter().any(|o| o == *id))
        .cloned()
        .collect();
    if !missing.is_empty() {
        missing.sort_by(|a, b| {
            let ta = pins.get(a).cloned().unwrap_or_default();
            let tb = pins.get(b).cloned().unwrap_or_default();
            tb.cmp(&ta)
        });
        cleaned.extend(missing);
        changed = true;
    }
    if changed {
        save_pin_order(&cleaned);
    }
}

fn note_path(id: &str) -> PathBuf {
    notes_dir().join(format!("{}.md", id))
}

fn ensure_dir() {
    fs::create_dir_all(notes_dir()).ok();
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // 仅当 '<' 后紧跟字母、'/' 或 '!' 时视为标签开头；否则是字面量（如 "a < b"、"<3"）
            '<' => {
                let looks_like_tag = match chars.peek() {
                    Some(&'/') | Some(&'!') => true,
                    Some(&ch) => ch.is_ascii_alphabetic(),
                    None => false,
                };
                if looks_like_tag {
                    // 吞掉直到下一个 '>'（含）；未闭合的标签吞到结尾
                    for c2 in chars.by_ref() {
                        if c2 == '>' {
                            break;
                        }
                    }
                } else {
                    out.push('<');
                }
            }
            // '>' 只可能作为标签结束符出现，标签内容已在上面循环内消费；
            // 走到这里说明 '>' 是字面量（如 "a > b"），原样保留
            '>' => out.push('>'),
            _ => out.push(c),
        }
    }
    out
}

fn parse_file(raw: &str) -> (NoteMeta, String) {
    // 起始分隔线兼容 \r\n 与 \n
    let Some(content) = raw
        .strip_prefix("---\r\n")
        .or_else(|| raw.strip_prefix("---\n"))
    else {
        return (NoteMeta::default(), raw.to_string());
    };

    // 逐行定位结束分隔线（单独一行的 ---），兼容 \r\n 与 \n
    let mut offset = 0usize;
    let mut fm_len: Option<usize> = None;
    let mut body_start: Option<usize> = None;
    for line in content.split_inclusive('\n') {
        let core = line.strip_suffix('\n').unwrap_or(line);
        let core = core.strip_suffix('\r').unwrap_or(core);
        if core == "---" {
            fm_len = Some(offset);
            body_start = Some(offset + line.len());
            break;
        }
        offset += line.len();
    }

    let (Some(fm_len), Some(body_start)) = (fm_len, body_start) else {
        return (NoteMeta::default(), raw.to_string());
    };

    let fm = &content[..fm_len];
    let body = &content[body_start..];
    let mut meta = NoteMeta::default();
    for line in fm.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        // 只在第一个冒号处切分，值中允许出现冒号
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "title" => meta.title = val.to_string(),
                "created" => meta.created = val.to_string(),
                "modified" => meta.modified = val.to_string(),
                "x" => meta.x = val.parse().unwrap_or(200.0),
                "y" => meta.y = val.parse().unwrap_or(200.0),
                "width" => meta.width = val.parse().unwrap_or(320.0),
                "height" => meta.height = val.parse().unwrap_or(420.0),
                "floating" => meta.floating = val == "true",
                // 向后兼容：旧版本用 pinned 表示"便签窗口悬浮置顶"
                "pinned" => meta.floating = val == "true",
                _ => {}
            }
        }
    }
    if meta.width < 200.0 { meta.width = 320.0; }
    if meta.height < 150.0 { meta.height = 420.0; }
    (meta, body.to_string())
}

fn write_file(id: &str, meta: &NoteMeta, body: &str) -> Result<(), String> {
    ensure_dir();
    let content = format!(
        "---\ntitle: {}\ncreated: {}\nmodified: {}\nx: {}\ny: {}\nwidth: {}\nheight: {}\nfloating: {}\n---\n{}",
        meta.title,
        meta.created,
        meta.modified,
        meta.x,
        meta.y,
        meta.width,
        meta.height,
        meta.floating,
        body
    );
    fs::write(note_path(id), content).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_notes() -> Vec<NoteListItem> {
    ensure_dir();
    reconcile_pins();
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
            let raw = match fs::read_to_string(&path) {
                Ok(r) => r,
                Err(_) => {
                    // 单个文件读取失败（编码异常/损坏）：保留占位项，不静默消失
                    return Some(NoteListItem {
                        id,
                        title: String::new(),
                        modified: String::new(),
                        preview: String::new(),
                        pinned: false,
                        pin_time: String::new(),
                        error: true,
                    });
                }
            };
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
                .map(|l| strip_html(l.trim_start_matches(['#', '-', '*', ' '])))
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
                error: false,
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
    floating: bool,
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
    let meta = NoteMeta { title, created, modified: now, x, y, width, height, floating };
    write_file(&id, &meta, &content)
}

#[tauri::command]
fn create_note() -> Result<String, String> {
    let now = Local::now();
    let secs = now.format("%Y%m%d_%H%M%S").to_string();
    let nanos = now.timestamp_subsec_nanos();
    let seq = NOTE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let id = format!("{}_{:09}_{}", secs, nanos, seq);
    let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let meta = NoteMeta { created: now_str.clone(), modified: now_str, ..NoteMeta::default() };
    ensure_dir();
    write_file(&id, &meta, "")?;
    Ok(id)
}

#[tauri::command]
fn delete_note(id: String) -> Result<(), String> {
    fs::remove_file(note_path(&id)).map_err(|e| e.to_string())?;
    // 同步清理置顶记录，避免孤儿数据累积
    let mut pins = load_pins();
    if pins.remove(&id).is_some() {
        save_pins(&pins);
    }
    let mut order = load_pin_order();
    let before = order.len();
    order.retain(|x| x != &id);
    if order.len() != before {
        save_pin_order(&order);
    }
    Ok(())
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
        .always_on_top(meta.floating)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_reads_new_floating_field() {
        let raw = "---\ntitle: 测试\ncreated: 2026-01-01 00:00:00\nmodified: 2026-01-01 00:00:00\nx: 100\ny: 200\nwidth: 320\nheight: 420\nfloating: true\n---\n正文内容";
        let (meta, body) = parse_file(raw);
        assert!(meta.floating, "floating 字段应被读为 true");
        assert_eq!(meta.title, "测试");
        assert_eq!(meta.x, 100.0);
        assert_eq!(meta.y, 200.0);
        assert_eq!(body, "正文内容");
    }

    #[test]
    fn parse_file_falls_back_to_old_pinned_field() {
        let raw = "---\ntitle: 旧便签\npinned: true\n---\n正文";
        let (meta, body) = parse_file(raw);
        assert!(meta.floating, "旧 pinned 字段应被兼容读为 floating=true");
        assert_eq!(meta.title, "旧便签");
        assert_eq!(body, "正文");
    }

    #[test]
    fn parse_file_no_frontmatter_uses_defaults() {
        let raw = "没有任何 frontmatter 的正文";
        let (meta, body) = parse_file(raw);
        assert!(!meta.floating);
        assert_eq!(meta.title, "");
        assert_eq!(body, raw);
    }

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(
            strip_html("<span data-status=\"todo\">待处理事项</span>"),
            "待处理事项"
        );
    }

    #[test]
    fn strip_html_passthrough_plain_text() {
        assert_eq!(strip_html("普通文本，无标签"), "普通文本，无标签");
    }

    #[test]
    fn strip_html_empty_input() {
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn strip_html_keeps_literal_angle_brackets() {
        assert_eq!(strip_html("a < b"), "a < b");
        assert_eq!(strip_html("a > b"), "a > b");
        assert_eq!(strip_html("1 < 2 > 1"), "1 < 2 > 1");
    }

    #[test]
    fn strip_html_keeps_heart_emoji() {
        assert_eq!(strip_html("我 <3 你"), "我 <3 你");
    }

    #[test]
    fn strip_html_keeps_trailing_open_bracket() {
        assert_eq!(strip_html("abc<"), "abc<");
    }

    #[test]
    fn strip_html_removes_nested_tags() {
        assert_eq!(strip_html("<p><b>加粗</b>正文</p>"), "加粗正文");
    }

    #[test]
    fn strip_html_unclosed_tag_swallows_rest() {
        // 未闭合标签：从 '<' 起无 '>'，剩余内容被吞（与旧实现语义一致，防御性测试锁定）
        assert_eq!(strip_html("<span 未闭合"), "");
    }
}
