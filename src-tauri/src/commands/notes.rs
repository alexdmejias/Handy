use crate::actions::process_transcription_output;
use crate::managers::notes::{Note, NoteBlock, NoteSummary, NoteWithBlocks, NotesManager};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn list_notes(
    notes_manager: State<'_, Arc<NotesManager>>,
) -> Result<Vec<NoteSummary>, String> {
    notes_manager.list_notes().map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_note(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
) -> Result<Option<NoteWithBlocks>, String> {
    notes_manager.get_note(id).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn create_note(
    notes_manager: State<'_, Arc<NotesManager>>,
    title: String,
) -> Result<Note, String> {
    notes_manager.create_note(title).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn rename_note(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
    title: String,
) -> Result<(), String> {
    notes_manager
        .rename_note(id, title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_note(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
) -> Result<(), String> {
    notes_manager.delete_note(id).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn set_default_note(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
) -> Result<(), String> {
    notes_manager
        .set_default_note(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn create_block(
    notes_manager: State<'_, Arc<NotesManager>>,
    note_id: i64,
    content: String,
) -> Result<NoteBlock, String> {
    notes_manager
        .append_block(
            note_id,
            content,
            crate::managers::notes::block_source::MANUAL,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_block(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
    content: String,
) -> Result<NoteBlock, String> {
    notes_manager
        .update_block(id, content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_block(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
) -> Result<(), String> {
    notes_manager.delete_block(id).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn move_block(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
    target_note_id: i64,
) -> Result<NoteBlock, String> {
    notes_manager
        .move_block(id, target_note_id)
        .map_err(|e| e.to_string())
}

/// Move a `[start, end)` character range out of `id` into `target_note_id`
/// as its own block, leaving the trimmed remainder behind (or deleting the
/// block if nothing is left). Offsets are character offsets, not UTF-16
/// code units or bytes — the frontend must convert `Selection` offsets from
/// JS string indices to `Array.from(text)` char indices before calling this.
#[tauri::command]
#[specta::specta]
pub async fn split_and_move_block(
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
    start: i64,
    end: i64,
    target_note_id: i64,
) -> Result<NoteBlock, String> {
    if start < 0 || end < 0 {
        return Err("Selection offsets must be non-negative".to_string());
    }
    notes_manager
        .split_and_move_block(id, start as usize, end as usize, target_note_id)
        .map_err(|e| e.to_string())
}

/// Runs the block's content through the user's configured post-processing
/// provider/prompt (the same pipeline a live transcription uses) and
/// replaces the block's content with the result in place.
#[tauri::command]
#[specta::specta]
pub async fn post_process_block(
    app: AppHandle,
    notes_manager: State<'_, Arc<NotesManager>>,
    id: i64,
) -> Result<NoteBlock, String> {
    let block = notes_manager
        .get_block(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Block {} not found", id))?;

    let processed = process_transcription_output(&app, &block.content, true).await;

    notes_manager
        .apply_post_processed_content(id, processed.final_text)
        .map_err(|e| e.to_string())
}
