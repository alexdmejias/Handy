import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { NotebookPen } from "lucide-react";
import { commands, type NoteSummary } from "@/bindings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { Button } from "../../ui/Button";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { useSettings } from "../../../hooks/useSettings";

export const NotepadSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [defaultNote, setDefaultNote] = useState<NoteSummary | null>(null);

  const enabled = getSetting("capture_to_notepad") ?? false;
  const autoOpen = getSetting("auto_open_notepad_on_capture") ?? false;

  useEffect(() => {
    let cancelled = false;
    commands.listNotes().then((result) => {
      if (cancelled || result.status !== "ok") return;
      setDefaultNote(result.data.find((n) => n.is_default) ?? null);
    });
    return () => {
      cancelled = true;
    };
  }, [enabled]);

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.notepad.title")}>
        <ToggleSwitch
          checked={enabled}
          onChange={(value) => updateSetting("capture_to_notepad", value)}
          isUpdating={isUpdating("capture_to_notepad")}
          label={t("settings.notepad.captureTitle")}
          description={t("settings.notepad.captureDescription")}
          grouped={true}
        />
        <ToggleSwitch
          checked={autoOpen}
          onChange={(value) =>
            updateSetting("auto_open_notepad_on_capture", value)
          }
          isUpdating={isUpdating("auto_open_notepad_on_capture")}
          disabled={!enabled}
          label={t("settings.notepad.autoOpenTitle")}
          description={t("settings.notepad.autoOpenDescription")}
          grouped={true}
        />
        <div className="flex items-center justify-between min-h-12 px-4 p-2">
          <div>
            <h3 className="text-sm font-medium">
              {t("settings.notepad.defaultNoteTitle")}
            </h3>
            <p className="text-sm text-text/60">
              {defaultNote
                ? defaultNote.title
                : t("settings.notepad.defaultNoteDescription")}
            </p>
          </div>
          <Button
            onClick={() => commands.openNotepadWindow()}
            variant="secondary"
            size="sm"
            className="flex items-center gap-2"
          >
            <NotebookPen className="w-4 h-4" />
            <span>{t("settings.notepad.openButton")}</span>
          </Button>
        </div>
      </SettingsGroup>
    </div>
  );
};
