import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Mic,
  Sparkles,
  Pencil,
  Trash2,
  ArrowRightLeft,
  FileText,
  Plus,
} from "lucide-react";
import type { NoteBlock, NoteSummary } from "@/bindings";
import { utf16ToCharIndex } from "./utils";

interface SelectionState {
  start: number;
  end: number;
  top: number;
  left: number;
}

interface MoveMenuProps {
  notes: NoteSummary[];
  excludeNoteId: number;
  onPick: (noteId: number) => void;
  onCreateAndPick: () => void;
  className?: string;
}

const MoveMenu: React.FC<MoveMenuProps> = ({
  notes,
  excludeNoteId,
  onPick,
  onCreateAndPick,
  className,
}) => {
  const { t } = useTranslation();
  const targets = notes.filter((n) => n.id !== excludeNoteId);

  return (
    <div
      className={`z-20 w-52 rounded-lg border border-mid-gray/20 bg-background p-1 shadow-lg ${className ?? ""}`}
    >
      {targets.length === 0 ? (
        <div className="px-2 py-1.5 text-xs text-text/50">
          {t("notepad.noNotes")}
        </div>
      ) : (
        targets.map((n) => (
          <button
            key={n.id}
            onClick={() => onPick(n.id)}
            className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm text-text hover:bg-logo-primary/10 hover:text-logo-primary cursor-pointer"
          >
            <FileText width={14} height={14} className="shrink-0" />
            <span className="truncate">{n.title}</span>
          </button>
        ))
      )}
      <div className="my-1 h-px bg-mid-gray/20" />
      <button
        onClick={onCreateAndPick}
        className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm text-logo-primary hover:bg-logo-primary/10 cursor-pointer"
      >
        <Plus width={14} height={14} className="shrink-0" />
        <span>{t("notepad.moveToNew")}</span>
      </button>
    </div>
  );
};

interface BlockProps {
  block: NoteBlock;
  notes: NoteSummary[];
  postProcessing: boolean;
  onMove: (blockId: number, targetNoteId: number) => void;
  onMoveToNewNote: (blockId: number) => void;
  onSplitMove: (
    blockId: number,
    start: number,
    end: number,
    targetNoteId: number,
  ) => void;
  onSplitMoveToNewNote: (blockId: number, start: number, end: number) => void;
  onPostProcess: (blockId: number) => void;
  onDelete: (blockId: number) => void;
}

const Block: React.FC<BlockProps> = ({
  block,
  notes,
  postProcessing,
  onMove,
  onMoveToNewNote,
  onSplitMove,
  onSplitMoveToNewNote,
  onPostProcess,
  onDelete,
}) => {
  const { t } = useTranslation();
  const bodyRef = useRef<HTMLParagraphElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const [showBlockMenu, setShowBlockMenu] = useState(false);
  const [selection, setSelection] = useState<SelectionState | null>(null);
  const [showSelectionMenu, setShowSelectionMenu] = useState(false);

  const sourceMeta =
    block.source === "post_processed"
      ? {
          label: t("notepad.postProcessed"),
          Icon: Sparkles,
          tone: "text-logo-primary",
        }
      : block.source === "manual"
        ? { label: t("notepad.manual"), Icon: Pencil, tone: "text-text/55" }
        : { label: t("notepad.dictation"), Icon: Mic, tone: "text-text/55" };

  const handleMouseUp = () => {
    const sel = window.getSelection();
    const container = wrapRef.current;
    if (!sel || sel.isCollapsed || !container || sel.rangeCount === 0) {
      setSelection(null);
      return;
    }
    const range = sel.getRangeAt(0);
    if (!container.contains(range.commonAncestorContainer)) {
      setSelection(null);
      return;
    }
    const text = sel.toString();
    if (!text.trim() || range.startContainer !== range.endContainer) {
      // Selection spans more than the single text node we render — skip
      // rather than compute a misleading offset.
      setSelection(null);
      return;
    }

    const rect = range.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    setSelection({
      start: utf16ToCharIndex(block.content, range.startOffset),
      end: utf16ToCharIndex(block.content, range.endOffset),
      top: rect.top - containerRect.top,
      left: rect.left - containerRect.left,
    });
    setShowSelectionMenu(false);
  };

  const clearSelection = () => {
    setSelection(null);
    setShowSelectionMenu(false);
    window.getSelection()?.removeAllRanges();
  };

  return (
    <div className="group relative rounded-lg border border-mid-gray/20 p-4">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center">
          <span
            className={`inline-flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wide ${sourceMeta.tone}`}
          >
            <sourceMeta.Icon width={12} height={12} />
            {sourceMeta.label}
          </span>
        </div>

        <div className="relative flex items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
          <div className="relative">
            <button
              onClick={() => setShowBlockMenu((v) => !v)}
              title={t("notepad.moveTo")}
              className="flex h-7 w-7 items-center justify-center rounded-md text-text/50 hover:bg-logo-primary/10 hover:text-logo-primary cursor-pointer"
            >
              <ArrowRightLeft width={14} height={14} />
            </button>
            {showBlockMenu && (
              <MoveMenu
                notes={notes}
                excludeNoteId={block.note_id}
                className="absolute right-0 top-full mt-1"
                onPick={(targetId) => {
                  setShowBlockMenu(false);
                  onMove(block.id, targetId);
                }}
                onCreateAndPick={() => {
                  setShowBlockMenu(false);
                  onMoveToNewNote(block.id);
                }}
              />
            )}
          </div>
          <button
            onClick={() => onPostProcess(block.id)}
            disabled={postProcessing}
            title={t("notepad.postProcessBlock")}
            className="flex h-7 w-7 items-center justify-center rounded-md text-text/50 hover:bg-logo-primary/10 hover:text-logo-primary disabled:opacity-40 cursor-pointer"
          >
            <Sparkles
              width={14}
              height={14}
              style={
                postProcessing
                  ? { animation: "spin 1.2s linear infinite" }
                  : undefined
              }
            />
          </button>
          <button
            onClick={() => onDelete(block.id)}
            title={t("notepad.deleteBlock")}
            className="flex h-7 w-7 items-center justify-center rounded-md text-text/50 hover:bg-logo-primary/10 hover:text-logo-primary cursor-pointer"
          >
            <Trash2 width={14} height={14} />
          </button>
        </div>
      </div>

      <div ref={wrapRef} className="relative">
        <p
          ref={bodyRef}
          onMouseUp={handleMouseUp}
          className="whitespace-pre-wrap text-sm leading-6 text-text/90 select-text cursor-text"
        >
          {block.content}
        </p>

        {selection && (
          <div
            className="absolute z-10 -translate-y-full"
            style={{ top: selection.top - 8, left: selection.left }}
          >
            <button
              onClick={() => setShowSelectionMenu((v) => !v)}
              className="flex items-center gap-1.5 whitespace-nowrap rounded-lg border border-mid-gray/20 bg-background px-2.5 py-1.5 text-xs font-medium text-text shadow-lg hover:text-logo-primary cursor-pointer"
            >
              <ArrowRightLeft
                width={13}
                height={13}
                className="text-logo-primary"
              />
              {t("notepad.moveSelection")}
            </button>
            {showSelectionMenu && (
              <MoveMenu
                notes={notes}
                excludeNoteId={block.note_id}
                className="mt-1"
                onPick={(targetId) => {
                  onSplitMove(
                    block.id,
                    selection.start,
                    selection.end,
                    targetId,
                  );
                  clearSelection();
                }}
                onCreateAndPick={() => {
                  onSplitMoveToNewNote(
                    block.id,
                    selection.start,
                    selection.end,
                  );
                  clearSelection();
                }}
              />
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default Block;
