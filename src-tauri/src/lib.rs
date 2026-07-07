mod audio;

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;
use tauri::Manager;

struct AppState {
    cmd_tx: Mutex<Option<mpsc::Sender<audio::AudioCommand>>>,
}

fn send_cmd(
    state: &tauri::State<'_, AppState>,
    cmd: audio::AudioCommand,
) -> Result<(), String> {
    let guard = state.cmd_tx.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(tx) => tx.send(cmd).map_err(|_| "audio channel closed".to_string()),
        None => Err("audio not available".to_string()),
    }
}

#[tauri::command]
fn play_note(state: tauri::State<'_, AppState>, string: u8, fret: u8) -> Result<(), String> {
    let key = audio::midi_key(string, fret);
    send_cmd(&state, audio::AudioCommand::NoteOn(key))
}

#[tauri::command]
fn stop_note(state: tauri::State<'_, AppState>, string: u8, fret: u8) -> Result<(), String> {
    let key = audio::midi_key(string, fret);
    send_cmd(&state, audio::AudioCommand::NoteOff(key))
}

#[tauri::command]
fn stop_all_notes(state: tauri::State<'_, AppState>) -> Result<(), String> {
    send_cmd(&state, audio::AudioCommand::StopAll)
}

#[tauri::command]
fn audio_status(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let guard = state.cmd_tx.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(_) => Ok("ok".to_string()),
        None => Ok("no_soundfont".to_string()),
    }
}

#[tauri::command]
fn load_exercises(app_handle: tauri::AppHandle) -> Result<String, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = app_handle.path().resource_dir() {
        candidates.push(dir.join("exercises.json"));
        candidates.push(dir.join("resources").join("exercises.json"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("exercises.json"));
            candidates.push(dir.join("resources").join("exercises.json"));
        }
    }
    candidates.push(PathBuf::from("resources/exercises.json"));
    candidates.push(PathBuf::from("../resources/exercises.json"));
    candidates.push(PathBuf::from("exercises.json"));
    for p in &candidates {
        if let Ok(content) = std::fs::read_to_string(&p) {
            return Ok(content);
        }
    }
    Err(format!(
        "exercises.json not found, searched: {:?}",
        candidates
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let resource_dir = app.path().resource_dir().ok();
            let sf_path = audio::default_soundfont_path(resource_dir.as_deref());
            let engine = match audio::AudioEngine::start(&sf_path) {
                Ok(e) => {
                    eprintln!("Audio: loaded from {:?}", sf_path);
                    Some(e)
                }
                Err(e) => {
                    eprintln!("Audio: {} — sound disabled", e);
                    None
                }
            };
            let cmd_tx = Mutex::new(engine.map(|e| e.cmd_tx));
            app.manage(AppState { cmd_tx });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            play_note,
            stop_note,
            stop_all_notes,
            audio_status,
            load_exercises,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::audio;

    #[test]
    fn midi_key_mapping() {
        assert_eq!(audio::midi_key(6, 0), 40);
        assert_eq!(audio::midi_key(5, 0), 45);
        assert_eq!(audio::midi_key(1, 0), 64);
        assert_eq!(audio::midi_key(5, 3), 48);
        assert_eq!(audio::midi_key(5, 5), 50);
        assert_eq!(audio::midi_key(4, 2), 52);
        assert_eq!(audio::midi_key(3, 12), 67);
    }

    #[test]
    fn default_sf_path_resolves() {
        audio::default_soundfont_path(None);
    }
}
