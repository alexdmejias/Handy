use anyhow::{anyhow, Result};
use chrono::Utc;
use log::{debug, error};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_specta::Event;

/// How many characters of a note's most recent block are kept for the note
/// list's preview snippet.
const SNIPPET_CHARS: usize = 140;

static MIGRATIONS: &[M] = &[M::up(
    "CREATE TABLE IF NOT EXISTS notes (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        is_default BOOLEAN NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS note_blocks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        note_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        content TEXT NOT NULL,
        source TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_note_blocks_note_id ON note_blocks(note_id, position);",
)];

/// Origin of a note block's content. Kept as a plain string column (rather
/// than a SQL enum) so a future source doesn't need a migration.
pub mod block_source {
    pub const DICTATION: &str = "dictation";
    pub const POST_PROCESSED: &str = "post_processed";
    pub const MANUAL: &str = "manual";
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub is_default: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct NoteBlock {
    pub id: i64,
    pub note_id: i64,
    pub position: i64,
    pub content: String,
    pub source: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct NoteSummary {
    pub id: i64,
    pub title: String,
    pub is_default: bool,
    pub updated_at: i64,
    pub block_count: i64,
    pub snippet: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct NoteWithBlocks {
    pub note: Note,
    pub blocks: Vec<NoteBlock>,
}

/// Coarse-grained change notification for the notepad window(s). Callers
/// refetch (`list_notes` / `get_note`) on receipt rather than patching state
/// from the payload — the notepad is low-frequency compared to history's
/// live transcription stream, so there's no need to mirror full rows here.
#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(tag = "action")]
pub enum NoteUpdatePayload {
    /// A note was created, renamed, deleted, or the default note changed.
    #[serde(rename = "notes_changed")]
    NotesChanged,
    /// A block within `note_id` was added, edited, moved, or deleted.
    #[serde(rename = "note_changed")]
    NoteChanged { note_id: i64 },
}

pub struct NotesManager {
    app_handle: AppHandle,
    db_path: PathBuf,
}

impl NotesManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_data_dir = crate::portable::app_data_dir(app_handle)?;
        let db_path = app_data_dir.join("notes.db");

        let manager = Self {
            app_handle: app_handle.clone(),
            db_path,
        };

        manager.init_database()?;
        manager.ensure_default_note()?;

        Ok(manager)
    }

    fn init_database(&self) -> Result<()> {
        let mut conn = Connection::open(&self.db_path)?;
        let migrations = Migrations::new(MIGRATIONS.to_vec());

        #[cfg(debug_assertions)]
        migrations.validate().expect("Invalid notes migrations");

        migrations.to_latest(&mut conn)?;
        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn notify_notes_changed(&self) {
        if let Err(e) = (NoteUpdatePayload::NotesChanged).emit(&self.app_handle) {
            error!("Failed to emit notes-updated event: {}", e);
        }
    }

    fn notify_note_changed(&self, note_id: i64) {
        if let Err(e) = (NoteUpdatePayload::NoteChanged { note_id }).emit(&self.app_handle) {
            error!("Failed to emit notes-updated event: {}", e);
        }
    }

    fn map_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
        Ok(Note {
            id: row.get("id")?,
            title: row.get("title")?,
            is_default: row.get("is_default")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn map_block(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteBlock> {
        Ok(NoteBlock {
            id: row.get("id")?,
            note_id: row.get("note_id")?,
            position: row.get("position")?,
            content: row.get("content")?,
            source: row.get("source")?,
            created_at: row.get("created_at")?,
        })
    }

    /// Guarantees exactly one note is marked default, creating a starter
    /// "Scratchpad" note the first time this runs. `delete_note` refuses to
    /// remove the default note, so the self-heal path below only matters for
    /// a fresh database or one left without a default some other way.
    fn ensure_default_note(&self) -> Result<i64> {
        let conn = self.get_connection()?;

        if let Some(id) = conn
            .query_row(
                "SELECT id FROM notes WHERE is_default = 1 LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            return Ok(id);
        }

        if let Some(id) = conn
            .query_row(
                "SELECT id FROM notes ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            conn.execute("UPDATE notes SET is_default = 1 WHERE id = ?1", params![id])?;
            return Ok(id);
        }

        let now = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO notes (title, is_default, created_at, updated_at) VALUES (?1, 1, ?2, ?2)",
            params!["Scratchpad", now],
        )?;
        let id = conn.last_insert_rowid();
        debug!("Created starter default note (id {})", id);
        Ok(id)
    }

    pub fn list_notes(&self) -> Result<Vec<NoteSummary>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT
                n.id, n.title, n.is_default, n.updated_at,
                (SELECT COUNT(*) FROM note_blocks b WHERE b.note_id = n.id) AS block_count,
                (SELECT content FROM note_blocks b WHERE b.note_id = n.id ORDER BY position DESC LIMIT 1) AS last_content
             FROM notes n
             ORDER BY n.is_default DESC, n.updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let last_content: Option<String> = row.get("last_content")?;
            Ok(NoteSummary {
                id: row.get("id")?,
                title: row.get("title")?,
                is_default: row.get("is_default")?,
                updated_at: row.get("updated_at")?,
                block_count: row.get("block_count")?,
                snippet: truncate_snippet(last_content.as_deref().unwrap_or("")),
            })
        })?;

        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Notes whose title or any block's content contains `query`
    /// (case-insensitive, SQLite's default for ASCII `LIKE`). The snippet
    /// shows the matching block when one matched on content, falling back to
    /// the note's most recent block when only the title matched. An empty
    /// (after trimming) query is equivalent to `list_notes`.
    pub fn search(&self, query: &str) -> Result<Vec<NoteSummary>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.list_notes();
        }

        let conn = self.get_connection()?;
        let pattern = format!("%{}%", escape_like(trimmed));

        let mut stmt = conn.prepare(
            "SELECT
                n.id, n.title, n.is_default, n.updated_at,
                (SELECT COUNT(*) FROM note_blocks b WHERE b.note_id = n.id) AS block_count,
                COALESCE(
                    (SELECT content FROM note_blocks b WHERE b.note_id = n.id
                     AND b.content LIKE ?1 ESCAPE '\\' ORDER BY b.position DESC LIMIT 1),
                    (SELECT content FROM note_blocks b WHERE b.note_id = n.id
                     ORDER BY b.position DESC LIMIT 1)
                ) AS snippet_source
             FROM notes n
             WHERE n.title LIKE ?1 ESCAPE '\\'
                OR EXISTS (
                    SELECT 1 FROM note_blocks b
                    WHERE b.note_id = n.id AND b.content LIKE ?1 ESCAPE '\\'
                )
             ORDER BY n.is_default DESC, n.updated_at DESC",
        )?;

        let rows = stmt.query_map(params![pattern], |row| {
            let snippet_source: Option<String> = row.get("snippet_source")?;
            Ok(NoteSummary {
                id: row.get("id")?,
                title: row.get("title")?,
                is_default: row.get("is_default")?,
                updated_at: row.get("updated_at")?,
                block_count: row.get("block_count")?,
                snippet: truncate_snippet(snippet_source.as_deref().unwrap_or("")),
            })
        })?;

        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_note(&self, note_id: i64) -> Result<Option<NoteWithBlocks>> {
        let conn = self.get_connection()?;
        let note = conn
            .query_row(
                "SELECT id, title, is_default, created_at, updated_at FROM notes WHERE id = ?1",
                params![note_id],
                Self::map_note,
            )
            .optional()?;

        let Some(note) = note else {
            return Ok(None);
        };

        let mut stmt = conn.prepare(
            "SELECT id, note_id, position, content, source, created_at
             FROM note_blocks WHERE note_id = ?1 ORDER BY position ASC",
        )?;
        let blocks = stmt
            .query_map(params![note_id], Self::map_block)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Some(NoteWithBlocks { note, blocks }))
    }

    pub fn create_note(&self, title: String) -> Result<Note> {
        let conn = self.get_connection()?;
        let now = Utc::now().timestamp();
        let title = if title.trim().is_empty() {
            "Untitled note".to_string()
        } else {
            title
        };

        conn.execute(
            "INSERT INTO notes (title, is_default, created_at, updated_at) VALUES (?1, 0, ?2, ?2)",
            params![title, now],
        )?;
        let id = conn.last_insert_rowid();

        self.notify_notes_changed();
        Ok(Note {
            id,
            title,
            is_default: false,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn rename_note(&self, note_id: i64, title: String) -> Result<()> {
        let conn = self.get_connection()?;
        let title = if title.trim().is_empty() {
            "Untitled note".to_string()
        } else {
            title
        };
        let updated = conn.execute(
            "UPDATE notes SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, Utc::now().timestamp(), note_id],
        )?;
        if updated == 0 {
            return Err(anyhow!("Note {} not found", note_id));
        }
        self.notify_notes_changed();
        Ok(())
    }

    /// Deletes a note and its blocks. The default note can't be deleted —
    /// it's the fixed landing spot for captured dictation, so removing it
    /// would silently conjure a fresh "Scratchpad" via `ensure_default_note`
    /// and lose the user's reference to where their captures go. Callers
    /// must reassign default to another note first (`set_default_note`).
    pub fn delete_note(&self, note_id: i64) -> Result<()> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        let is_default: bool = tx
            .query_row(
                "SELECT is_default FROM notes WHERE id = ?1",
                params![note_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Note {} not found", note_id))?;

        if is_default {
            return Err(anyhow!("The default note can't be deleted"));
        }

        tx.execute(
            "DELETE FROM note_blocks WHERE note_id = ?1",
            params![note_id],
        )?;
        tx.execute("DELETE FROM notes WHERE id = ?1", params![note_id])?;
        tx.commit()?;

        self.notify_notes_changed();
        Ok(())
    }

    pub fn set_default_note(&self, note_id: i64) -> Result<()> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM notes WHERE id = ?1",
                params![note_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !exists {
            return Err(anyhow!("Note {} not found", note_id));
        }

        tx.execute("UPDATE notes SET is_default = 0 WHERE is_default = 1", [])?;
        tx.execute(
            "UPDATE notes SET is_default = 1 WHERE id = ?1",
            params![note_id],
        )?;
        tx.commit()?;

        self.notify_notes_changed();
        Ok(())
    }

    fn next_position(conn: &Connection, note_id: i64) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM note_blocks WHERE note_id = ?1",
            params![note_id],
            |row| row.get(0),
        )
    }

    fn touch_note(conn: &Connection, note_id: i64) -> rusqlite::Result<()> {
        conn.execute(
            "UPDATE notes SET updated_at = ?1 WHERE id = ?2",
            params![Utc::now().timestamp(), note_id],
        )?;
        Ok(())
    }

    /// Append a block to an arbitrary note (used for manual entry from the
    /// notepad UI). See `append_to_default` for the dictation capture path.
    pub fn append_block(&self, note_id: i64, content: String, source: &str) -> Result<NoteBlock> {
        let conn = self.get_connection()?;
        let position = Self::next_position(&conn, note_id)?;
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO note_blocks (note_id, position, content, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![note_id, position, content, source, now],
        )?;
        let id = conn.last_insert_rowid();
        Self::touch_note(&conn, note_id)?;

        self.notify_note_changed(note_id);
        Ok(NoteBlock {
            id,
            note_id,
            position,
            content,
            source: source.to_string(),
            created_at: now,
        })
    }

    /// Capture-pipeline entry point: appends `content` to the default note.
    /// Returns `Ok(None)` for blank content rather than creating an empty
    /// block. `post_processed` selects the block's source tag — it does not
    /// run post-processing itself (that already happened upstream in
    /// `process_transcription_output`).
    pub fn append_to_default(
        &self,
        content: &str,
        post_processed: bool,
    ) -> Result<Option<NoteBlock>> {
        if content.trim().is_empty() {
            return Ok(None);
        }
        let note_id = self.ensure_default_note()?;
        let source = if post_processed {
            block_source::POST_PROCESSED
        } else {
            block_source::DICTATION
        };
        Ok(Some(self.append_block(
            note_id,
            content.to_string(),
            source,
        )?))
    }

    pub fn get_block(&self, block_id: i64) -> Result<Option<NoteBlock>> {
        let conn = self.get_connection()?;
        let block = conn
            .query_row(
                "SELECT id, note_id, position, content, source, created_at FROM note_blocks WHERE id = ?1",
                params![block_id],
                Self::map_block,
            )
            .optional()?;
        Ok(block)
    }

    pub fn update_block(&self, block_id: i64, content: String) -> Result<NoteBlock> {
        let conn = self.get_connection()?;
        let note_id: i64 = conn
            .query_row(
                "SELECT note_id FROM note_blocks WHERE id = ?1",
                params![block_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Block {} not found", block_id))?;

        conn.execute(
            "UPDATE note_blocks SET content = ?1 WHERE id = ?2",
            params![content, block_id],
        )?;
        Self::touch_note(&conn, note_id)?;

        let block = conn.query_row(
            "SELECT id, note_id, position, content, source, created_at FROM note_blocks WHERE id = ?1",
            params![block_id],
            Self::map_block,
        )?;

        self.notify_note_changed(note_id);
        Ok(block)
    }

    /// Replace a block's content with its post-processed result and mark it
    /// as such. Used by the "post-process this block" action.
    pub fn apply_post_processed_content(
        &self,
        block_id: i64,
        content: String,
    ) -> Result<NoteBlock> {
        let conn = self.get_connection()?;
        let note_id: i64 = conn
            .query_row(
                "SELECT note_id FROM note_blocks WHERE id = ?1",
                params![block_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Block {} not found", block_id))?;

        conn.execute(
            "UPDATE note_blocks SET content = ?1, source = ?2 WHERE id = ?3",
            params![content, block_source::POST_PROCESSED, block_id],
        )?;
        Self::touch_note(&conn, note_id)?;

        let block = conn.query_row(
            "SELECT id, note_id, position, content, source, created_at FROM note_blocks WHERE id = ?1",
            params![block_id],
            Self::map_block,
        )?;

        self.notify_note_changed(note_id);
        Ok(block)
    }

    pub fn delete_block(&self, block_id: i64) -> Result<()> {
        let conn = self.get_connection()?;
        let note_id: i64 = conn
            .query_row(
                "SELECT note_id FROM note_blocks WHERE id = ?1",
                params![block_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Block {} not found", block_id))?;

        conn.execute("DELETE FROM note_blocks WHERE id = ?1", params![block_id])?;
        Self::touch_note(&conn, note_id)?;

        self.notify_note_changed(note_id);
        Ok(())
    }

    /// Move an entire block to another note, appended at the end.
    pub fn move_block(&self, block_id: i64, target_note_id: i64) -> Result<NoteBlock> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        let source_note_id: i64 = tx
            .query_row(
                "SELECT note_id FROM note_blocks WHERE id = ?1",
                params![block_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Block {} not found", block_id))?;

        let target_exists: bool = tx
            .query_row(
                "SELECT 1 FROM notes WHERE id = ?1",
                params![target_note_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !target_exists {
            return Err(anyhow!("Target note {} not found", target_note_id));
        }

        let position = Self::next_position(&tx, target_note_id)?;
        tx.execute(
            "UPDATE note_blocks SET note_id = ?1, position = ?2 WHERE id = ?3",
            params![target_note_id, position, block_id],
        )?;
        Self::touch_note(&tx, source_note_id)?;
        Self::touch_note(&tx, target_note_id)?;

        let block = tx.query_row(
            "SELECT id, note_id, position, content, source, created_at FROM note_blocks WHERE id = ?1",
            params![block_id],
            Self::map_block,
        )?;
        tx.commit()?;

        self.notify_note_changed(source_note_id);
        if target_note_id != source_note_id {
            self.notify_note_changed(target_note_id);
        }
        Ok(block)
    }

    /// Split a block at a character range `[start, end)` and move only that
    /// slice to another note (appended at the end), leaving the remainder
    /// (trimmed) in the original block. If nothing is left behind, the
    /// original block is deleted rather than left empty. Offsets are in
    /// `char`s, not UTF-16 code units or bytes.
    pub fn split_and_move_block(
        &self,
        block_id: i64,
        start: usize,
        end: usize,
        target_note_id: i64,
    ) -> Result<NoteBlock> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        let (source_note_id, content, source): (i64, String, String) = tx
            .query_row(
                "SELECT note_id, content, source FROM note_blocks WHERE id = ?1",
                params![block_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Block {} not found", block_id))?;

        let target_exists: bool = tx
            .query_row(
                "SELECT 1 FROM notes WHERE id = ?1",
                params![target_note_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !target_exists {
            return Err(anyhow!("Target note {} not found", target_note_id));
        }

        let chars: Vec<char> = content.chars().collect();
        let start = start.min(chars.len());
        let end = end.clamp(start, chars.len());
        if start == end {
            return Err(anyhow!("Empty selection"));
        }

        let selected: String = chars[start..end].iter().collect();
        let before: String = chars[..start].iter().collect();
        let after: String = chars[end..].iter().collect();
        let remainder = format!("{}{}", before.trim_end(), {
            let a = after.trim_start();
            if before.trim_end().is_empty() || a.is_empty() {
                a.to_string()
            } else {
                format!(" {}", a)
            }
        })
        .trim()
        .to_string();

        if remainder.is_empty() {
            tx.execute("DELETE FROM note_blocks WHERE id = ?1", params![block_id])?;
        } else {
            tx.execute(
                "UPDATE note_blocks SET content = ?1 WHERE id = ?2",
                params![remainder, block_id],
            )?;
        }
        Self::touch_note(&tx, source_note_id)?;

        let position = Self::next_position(&tx, target_note_id)?;
        let now = Utc::now().timestamp();
        tx.execute(
            "INSERT INTO note_blocks (note_id, position, content, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![target_note_id, position, selected, source, now],
        )?;
        let new_block_id = tx.last_insert_rowid();
        Self::touch_note(&tx, target_note_id)?;

        let new_block = tx.query_row(
            "SELECT id, note_id, position, content, source, created_at FROM note_blocks WHERE id = ?1",
            params![new_block_id],
            Self::map_block,
        )?;
        tx.commit()?;

        self.notify_note_changed(source_note_id);
        if target_note_id != source_note_id {
            self.notify_note_changed(target_note_id);
        }
        Ok(new_block)
    }
}

/// Escapes SQLite `LIKE` wildcards (`%`, `_`) and the escape character
/// itself so a search query is matched literally rather than as a pattern.
/// Pair with `ESCAPE '\\'` in the query.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn truncate_snippet(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= SNIPPET_CHARS {
        return collapsed;
    }
    let truncated: String = collapsed.chars().take(SNIPPET_CHARS).collect();
    format!("{}\u{2026}", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                is_default BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE note_blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                note_id INTEGER NOT NULL,
                position INTEGER NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );",
        )
        .expect("create schema");
        conn
    }

    fn insert_note(conn: &Connection, title: &str, is_default: bool) -> i64 {
        conn.execute(
            "INSERT INTO notes (title, is_default, created_at, updated_at) VALUES (?1, ?2, 0, 0)",
            params![title, is_default],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_block(conn: &Connection, note_id: i64, position: i64, content: &str) -> i64 {
        conn.execute(
            "INSERT INTO note_blocks (note_id, position, content, source, created_at)
             VALUES (?1, ?2, ?3, 'dictation', 0)",
            params![note_id, position, content],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn next_position_starts_at_zero() {
        let conn = setup_conn();
        let note_id = insert_note(&conn, "A", true);
        assert_eq!(NotesManager::next_position(&conn, note_id).unwrap(), 0);
        insert_block(&conn, note_id, 0, "first");
        assert_eq!(NotesManager::next_position(&conn, note_id).unwrap(), 1);
    }

    #[test]
    fn truncate_snippet_collapses_whitespace_and_caps_length() {
        assert_eq!(truncate_snippet("  hello   world  "), "hello world");
        let long = "a".repeat(200);
        let snippet = truncate_snippet(&long);
        assert_eq!(snippet.chars().count(), SNIPPET_CHARS + 1); // + ellipsis
        assert!(snippet.ends_with('\u{2026}'));
    }

    #[test]
    fn split_logic_rejoins_before_and_after_with_a_single_space() {
        // Exercises the same trim/join logic split_and_move_block uses,
        // without needing a full NotesManager (which requires an AppHandle).
        let content = "Follow-up items: finalize the thing, confirm tokens.";
        let chars: Vec<char> = content.chars().collect();
        let start = content.find("finalize the thing").unwrap();
        let end = start + "finalize the thing".len();

        let before: String = chars[..start].iter().collect();
        let after: String = chars[end..].iter().collect();
        let a = after.trim_start();
        let remainder = if before.trim_end().is_empty() || a.is_empty() {
            format!("{}{}", before.trim_end(), a)
        } else {
            format!("{} {}", before.trim_end(), a)
        }
        .trim()
        .to_string();

        assert_eq!(
            remainder,
            "Follow-up items: , confirm tokens.".replace("  ", " ")
        );
    }
}
