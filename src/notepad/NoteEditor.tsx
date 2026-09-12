import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Sparkles, MoreVertical, Pin, Plus, ArrowUpDown } from "lucide-react";
import type { NoteBlock, NoteSummary, NoteWithBlocks } from "@/bindings";
import { formatRelativeTime } from "@/utils/dateFormat";
import Block from "./Block";

interface NoteEditorProps {
  note: NoteWithBlocks;
  notes: NoteSummary[];
  postProcessingIds: Set<number>;
  onRename: (id: number, title: string) => void;
  onDeleteNote: (id: number) => void;
  onMakeDefault: (id: number) => void;
  onAddBlock: (noteId: number, content: string) => void;
  onMoveBlock: (blockId: number, targetNoteId: number) => void;
  onMoveBlockToNewNote: (blockId: number) => void;
  onSplitMoveBlock: (
    blockId: number,
    start: number,
    end: number,
    targetNoteId: number,
  ) => void;
  onSplitMoveBlockToNewNote: (
    blockId: number,
    start: number,
    end: number,
  ) => void;
  onPostProcessBlock: (blockId: number) => void;
  onPostProcessNote: (note: NoteWithBlocks) => void;
  onDeleteBlock: (blockId: number) => void;
}

const NoteEditor: React.FC<NoteEditorProps> = ({
  note,
  notes,
  postProcessingIds,
  onRename,
  onDeleteNote,
  onMakeDefault,
  onAddBlock,
  onMoveBlock,
  onMoveBlockToNewNote,
  onSplitMoveBlock,
  onSplitMoveBlockToNewNote,
  onPostProcessBlock,
  onPostProcessNote,
  onDeleteBlock,
}) => {
  const { t, i18n } = useTranslation();
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState(note.note.title);
  const [showMenu, setShowMenu] = useState(false);
  const [draft, setDraft] = useState("");
  // Blocks are stored (and returned by the backend) oldest-first, the order
  // moves/splits append into — newest-first is purely a display reversal.
  const [newestFirst, setNewestFirst] = useState(true);

  useEffect(() => {
    setTitleDraft(note.note.title);
    setEditingTitle(false);
    setShowMenu(false);
  }, [note.note.id, note.note.title]);

  const commitTitle = () => {
    setEditingTitle(false);
    const trimmed = titleDraft.trim();
    if (trimmed && trimmed !== note.note.title) {
      onRename(note.note.id, trimmed);
    } else {
      setTitleDraft(note.note.title);
    }
  };

  const submitDraft = () => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    onAddBlock(note.note.id, trimmed);
    setDraft("");
  };

  const anyPostProcessing = note.blocks.some((b) =>
    postProcessingIds.has(b.id),
  );

  const displayBlocks = useMemo(
    () => (newestFirst ? [...note.blocks].reverse() : note.blocks),
    [note.blocks, newestFirst],
  );

  return (
    <div className="flex h-full min-w-0 flex-1 flex-col">
      <div className="flex items-start justify-between border-b border-mid-gray/20 px-6 py-4">
        <div className="min-w-0">
          <div className="mb-1 flex items-center gap-2.5">
            {editingTitle ? (
              <input
                autoFocus
                value={titleDraft}
                onChange={(e) => setTitleDraft(e.target.value)}
                onBlur={commitTitle}
                onKeyDown={(e) => {
                  if (e.key === "Enter") commitTitle();
                  if (e.key === "Escape") {
                    setTitleDraft(note.note.title);
                    setEditingTitle(false);
                  }
                }}
                className="rounded-md border border-logo-primary/40 bg-background px-1.5 py-0.5 text-xl font-semibold outline-none"
              />
            ) : (
              <h2
                onClick={() => setEditingTitle(true)}
                title={t("notepad.renameNote")}
                className="cursor-text truncate text-xl font-semibold"
              >
                {note.note.title}
              </h2>
            )}
            {note.note.is_default && (
              <span className="inline-flex shrink-0 items-center gap-1 rounded-full bg-logo-primary/15 px-2 py-0.5 text-[11px] font-semibold">
                <Pin
                  width={11}
                  height={11}
                  className="text-logo-primary"
                  fill="currentColor"
                />
                {t("notepad.defaultNote")}
              </span>
            )}
          </div>
          <div className="text-[13px] text-text/45">
            {t("notepad.blockCount", { count: note.blocks.length })} &middot;{" "}
            {t("notepad.updated", {
              time: formatRelativeTime(
                String(note.note.updated_at),
                i18n.language,
              ),
            })}
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <button
            onClick={() => onPostProcessNote(note)}
            disabled={anyPostProcessing || note.blocks.length === 0}
            className="flex items-center gap-1.5 rounded-lg border border-mid-gray/20 px-3 py-1.5 text-[13px] font-medium hover:border-logo-primary/40 hover:text-logo-primary disabled:opacity-40 cursor-pointer"
          >
            <Sparkles width={15} height={15} />
            {t("notepad.postProcessNote")}
          </button>
          <button
            onClick={() => setNewestFirst((v) => !v)}
            title={
              newestFirst
                ? t("notepad.showOldestFirst")
                : t("notepad.showNewestFirst")
            }
            className="flex h-8 w-8 items-center justify-center rounded-lg text-text/50 hover:bg-mid-gray/10 hover:text-text cursor-pointer"
          >
            <ArrowUpDown width={15} height={15} />
          </button>
          <div className="relative">
            <button
              onClick={() => setShowMenu((v) => !v)}
              title={t("notepad.moreOptions")}
              className="flex h-8 w-8 items-center justify-center rounded-lg text-text/50 hover:bg-mid-gray/10 hover:text-text cursor-pointer"
            >
              <MoreVertical width={16} height={16} />
            </button>
            {showMenu && (
              <div className="absolute right-0 top-full z-20 mt-1 w-44 rounded-lg border border-mid-gray/20 bg-background p-1 shadow-lg">
                {!note.note.is_default && (
                  <button
                    onClick={() => {
                      setShowMenu(false);
                      onMakeDefault(note.note.id);
                    }}
                    className="w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-mid-gray/10 cursor-pointer"
                  >
                    {t("notepad.makeDefault")}
                  </button>
                )}
                <button
                  onClick={() => {
                    setShowMenu(false);
                    if (
                      window.confirm(
                        t("notepad.deleteNoteConfirm", {
                          title: note.note.title,
                        }),
                      )
                    ) {
                      onDeleteNote(note.note.id);
                    }
                  }}
                  className="w-full rounded-md px-2 py-1.5 text-left text-sm text-error hover:bg-error/10 cursor-pointer"
                >
                  {t("notepad.deleteNote")}
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        {note.blocks.length === 0 ? (
          <div className="py-8 text-center text-sm text-text/45">
            {t("notepad.emptyNote")}
          </div>
        ) : (
          <div className="flex flex-col gap-3">
            {displayBlocks.map((block: NoteBlock) => (
              <Block
                key={block.id}
                block={block}
                notes={notes}
                postProcessing={postProcessingIds.has(block.id)}
                onMove={onMoveBlock}
                onMoveToNewNote={onMoveBlockToNewNote}
                onSplitMove={onSplitMoveBlock}
                onSplitMoveToNewNote={onSplitMoveBlockToNewNote}
                onPostProcess={onPostProcessBlock}
                onDelete={onDeleteBlock}
              />
            ))}
          </div>
        )}

        <div className="mt-3 flex items-center gap-2">
          <input
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") submitDraft();
            }}
            placeholder={t("notepad.addBlockPlaceholder")}
            className="flex-1 rounded-lg border border-mid-gray/20 bg-background px-3 py-2 text-sm outline-none focus:border-logo-primary/40"
          />
          <button
            onClick={submitDraft}
            disabled={!draft.trim()}
            className="flex items-center gap-1.5 rounded-lg border border-mid-gray/20 px-3 py-2 text-sm font-medium hover:border-logo-primary/40 hover:text-logo-primary disabled:opacity-40 cursor-pointer"
          >
            <Plus width={15} height={15} />
            {t("notepad.add")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default NoteEditor;
