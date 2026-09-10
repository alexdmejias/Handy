import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast, Toaster } from "sonner";
import {
  commands,
  events,
  type NoteSummary,
  type NoteWithBlocks,
} from "@/bindings";
import NoteList from "./NoteList";
import NoteEditor from "./NoteEditor";

const NotepadApp: React.FC = () => {
  const { t } = useTranslation();
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [activeNoteId, setActiveNoteId] = useState<number | null>(null);
  const [activeNote, setActiveNote] = useState<NoteWithBlocks | null>(null);
  const [postProcessingIds, setPostProcessingIds] = useState<Set<number>>(
    new Set(),
  );
  const activeNoteIdRef = useRef<number | null>(null);
  activeNoteIdRef.current = activeNoteId;

  const refreshNotes = useCallback(async () => {
    const result = await commands.listNotes();
    if (result.status !== "ok") {
      console.error("Failed to list notes:", result.error);
      return;
    }
    setNotes(result.data);

    const current = activeNoteIdRef.current;
    const stillExists =
      current !== null && result.data.some((n) => n.id === current);
    if (!stillExists) {
      const fallback =
        result.data.find((n) => n.is_default) ?? result.data[0] ?? null;
      setActiveNoteId(fallback ? fallback.id : null);
    }
  }, []);

  const refreshActiveNote = useCallback(async (id: number) => {
    const result = await commands.getNote(id);
    if (result.status !== "ok") {
      console.error("Failed to load note:", result.error);
      return;
    }
    if (activeNoteIdRef.current === id) {
      setActiveNote(result.data);
    }
  }, []);

  useEffect(() => {
    refreshNotes();
  }, [refreshNotes]);

  useEffect(() => {
    if (activeNoteId === null) {
      setActiveNote(null);
      return;
    }
    refreshActiveNote(activeNoteId);
  }, [activeNoteId, refreshActiveNote]);

  useEffect(() => {
    const unlisten = events.noteUpdatePayload.listen((event) => {
      const payload = event.payload;
      if (payload.action === "notes_changed") {
        refreshNotes();
      } else if (payload.action === "note_changed") {
        refreshNotes();
        if (payload.note_id === activeNoteIdRef.current) {
          refreshActiveNote(payload.note_id);
        }
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshNotes, refreshActiveNote]);

  const withPostProcessing = useCallback(
    async (blockId: number, run: () => Promise<void>) => {
      setPostProcessingIds((prev) => new Set(prev).add(blockId));
      try {
        await run();
      } finally {
        setPostProcessingIds((prev) => {
          const next = new Set(prev);
          next.delete(blockId);
          return next;
        });
      }
    },
    [],
  );

  const handleCreateNote = async () => {
    const result = await commands.createNote(t("notepad.untitledNote"));
    if (result.status !== "ok") {
      toast.error(String(result.error));
      return;
    }
    setActiveNoteId(result.data.id);
  };

  const handleRename = async (id: number, title: string) => {
    const result = await commands.renameNote(id, title);
    if (result.status !== "ok") toast.error(String(result.error));
  };

  const handleDeleteNote = async (id: number) => {
    const result = await commands.deleteNote(id);
    if (result.status !== "ok") toast.error(String(result.error));
  };

  const handleMakeDefault = async (id: number) => {
    const result = await commands.setDefaultNote(id);
    if (result.status !== "ok") toast.error(String(result.error));
  };

  const handleAddBlock = async (noteId: number, content: string) => {
    const result = await commands.createBlock(noteId, content);
    if (result.status !== "ok") toast.error(String(result.error));
  };

  const handleDeleteBlock = async (blockId: number) => {
    const result = await commands.deleteBlock(blockId);
    if (result.status !== "ok") toast.error(String(result.error));
  };

  const handleMoveBlock = async (blockId: number, targetNoteId: number) => {
    const result = await commands.moveBlock(blockId, targetNoteId);
    if (result.status !== "ok") toast.error(String(result.error));
  };

  const handleMoveBlockToNewNote = async (blockId: number) => {
    const created = await commands.createNote(t("notepad.untitledNote"));
    if (created.status !== "ok") {
      toast.error(String(created.error));
      return;
    }
    await handleMoveBlock(blockId, created.data.id);
  };

  const handleSplitMoveBlock = async (
    blockId: number,
    start: number,
    end: number,
    targetNoteId: number,
  ) => {
    const result = await commands.splitAndMoveBlock(
      blockId,
      start,
      end,
      targetNoteId,
    );
    if (result.status !== "ok") toast.error(String(result.error));
  };

  const handleSplitMoveBlockToNewNote = async (
    blockId: number,
    start: number,
    end: number,
  ) => {
    const created = await commands.createNote(t("notepad.untitledNote"));
    if (created.status !== "ok") {
      toast.error(String(created.error));
      return;
    }
    await handleSplitMoveBlock(blockId, start, end, created.data.id);
  };

  const handlePostProcessBlock = (blockId: number) => {
    withPostProcessing(blockId, async () => {
      const result = await commands.postProcessBlock(blockId);
      if (result.status !== "ok") toast.error(String(result.error));
    });
  };

  const handlePostProcessNote = (note: NoteWithBlocks) => {
    note.blocks.forEach((block) => handlePostProcessBlock(block.id));
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-text">
      <Toaster position="bottom-center" />
      <NoteList
        notes={notes}
        activeNoteId={activeNoteId}
        onSelect={setActiveNoteId}
        onCreate={handleCreateNote}
      />
      {activeNote ? (
        <NoteEditor
          note={activeNote}
          notes={notes}
          postProcessingIds={postProcessingIds}
          onRename={handleRename}
          onDeleteNote={handleDeleteNote}
          onMakeDefault={handleMakeDefault}
          onAddBlock={handleAddBlock}
          onMoveBlock={handleMoveBlock}
          onMoveBlockToNewNote={handleMoveBlockToNewNote}
          onSplitMoveBlock={handleSplitMoveBlock}
          onSplitMoveBlockToNewNote={handleSplitMoveBlockToNewNote}
          onPostProcessBlock={handlePostProcessBlock}
          onPostProcessNote={handlePostProcessNote}
          onDeleteBlock={handleDeleteBlock}
        />
      ) : (
        <div className="flex flex-1 items-center justify-center text-sm text-text/45">
          {t("notepad.noNotes")}
        </div>
      )}
    </div>
  );
};

export default NotepadApp;
