import { Download } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";

// Downloads the audit transcript once a match is over or disputed; renders
// nothing otherwise.
export default function ExportMatchButton({ className = "" }) {
  const { state, multiplayer, setStatus, exportAuditTranscript } = useGame();
  const { t } = useI18n();
  const canExportMatch = Boolean(
    typeof exportAuditTranscript === "function"
    && (state?.game_over || multiplayer?.matchDisputed || multiplayer?.mode === "disputed")
  );

  const handleExportMatch = async () => {
    if (typeof exportAuditTranscript !== "function") {
      setStatus("No match transcript is available to export", true);
      return;
    }
    try {
      const transcript = await exportAuditTranscript();
      if (!transcript) {
        setStatus("No match transcript is available to export", true);
        return;
      }
      const matchId = String(transcript.matchId || transcript.match?.auditMatchId || "match")
        .replace(/[^a-z0-9._-]+/gi, "-")
        .replace(/^-+|-+$/g, "")
        || "match";
      const blob = new Blob([`${JSON.stringify(transcript, null, 2)}\n`], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `ironsmith-${matchId}-audit.json`;
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      setStatus("Exported match audit transcript");
    } catch (err) {
      setStatus(`Export match failed: ${err?.message || err}`, true);
    }
  };

  if (!canExportMatch) return null;
  return (
    <button type="button" className={className} onClick={handleExportMatch}>
      <Download className="size-3.5" aria-hidden="true" />
      {t("action.exportMatch")}
    </button>
  );
}
