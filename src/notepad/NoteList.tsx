import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Search, Pin } from "lucide-react";
import type { NoteSummary } from "@/bindings";
import { formatRelativeTime } from "@/utils/dateFormat";

interface NoteListProps {
  notes: NoteSummary[];
  activeNoteId: number | null;
  onSelect: (id: number) => void;
  onCreate: () => void;
}

const NoteList: React.FC<NoteListProps> = ({
  notes,
  activeNoteId,
  onSelect,
  onCreate,
}) => {
  const { t, i18n } = useTranslation();
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return notes;
    return notes.filter(
      (n) =>
        n.title.toLowerCase().includes(q) ||
        n.snippet.toLowerCase().includes(q),
    );
  }, [notes, query]);

  return (
    <div className="flex h-full w-[280px] shrink-0 flex-col border-e border-mid-gray/20">
      <div className="flex items-center justify-between px-4 pb-1 pt-4">
        <h1 className="text-lg font-semibold">{t("notepad.title")}</h1>
        <button
          onClick={onCreate}
          title={t("notepad.newNote")}
          className="flex h-7 w-7 items-center justify-center rounded-md bg-logo-primary/10 text-logo-primary hover:bg-logo-primary/20 cursor-pointer"
        >
          <Plus width={16} height={16} />
        </button>
      </div>

      <div className="relative mx-4 my-3">
        <Search
          width={14}
          height={14}
          className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-mid-gray"
        />
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("notepad.searchPlaceholder")}
          className="w-full rounded-lg border border-mid-gray/20 bg-background py-2 pe-3 ps-8 text-[13px] text-text outline-none focus:border-logo-primary/40"
        />
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-2">
        {filtered.length === 0 ? (
          <div className="px-3 py-3 text-center text-xs text-text/50">
            {query ? t("notepad.noNotesMatch") : t("notepad.noNotes")}
          </div>
        ) : (
          filtered.map((note) => {
            const isActive = note.id === activeNoteId;
            return (
              <div
                key={note.id}
                onClick={() => onSelect(note.id)}
                className={`mb-0.5 flex cursor-pointer flex-col gap-1 rounded-lg px-3 py-2.5 transition-colors ${
                  isActive
                    ? "bg-logo-primary/15 hover:bg-logo-primary/20"
                    : "hover:bg-mid-gray/10"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="truncate text-sm font-medium">
                    {note.title}
                  </span>
                  {note.is_default && (
                    <Pin
                      width={13}
                      height={13}
                      className="shrink-0 text-logo-primary"
                      fill="currentColor"
                    />
                  )}
                </div>
                {note.snippet && (
                  <div className="truncate text-xs text-text/45">
                    {note.snippet}
                  </div>
                )}
                <div className="text-[11px] text-text/35">
                  {t("notepad.updated", {
                    time: formatRelativeTime(
                      String(note.updated_at),
                      i18n.language,
                    ),
                  })}{" "}
                  &middot;{" "}
                  {t("notepad.blockCount", { count: note.block_count })}
                </div>
              </div>
            );
          })
        )}
      </div>

      <div className="flex items-center gap-1.5 border-t border-mid-gray/20 px-4 py-3 text-xs text-text/50">
        <Pin width={12} height={12} className="shrink-0" />
        <span>{t("notepad.captureHint")}</span>
      </div>
    </div>
  );
};

export default NoteList;
