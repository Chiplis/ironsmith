import { useGame } from "@/context/GameContext";
import { Checkbox } from "@/components/ui/checkbox";
import ZoneViewer from "@/components/board/ZoneViewer";
import { useI18n } from "@/i18n/I18nContext";

const selectPill = "stone-select rounded-none px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer";

export default function AddCardBar({
  compact = false,
  zoneViews = ["battlefield"],
  setZoneViews,
}) {
  const {
    autoPassEnabled,
    setAutoPassEnabled,
    holdRule,
    setHoldRule,
  } = useGame();
  const { t } = useI18n();

  return (
    <div className={`add-card-toolbar table-toolbar table-toolbar--secondary rounded-none px-3 py-2${compact ? " add-card-toolbar--compact" : ""}`}>
      <div className="add-card-toolbar-zone-group">
        <ZoneViewer zoneViews={zoneViews} setZoneViews={setZoneViews} embedded />
      </div>

      <span className="add-card-toolbar-separator add-card-toolbar-control-separator" aria-hidden="true" />

      <div className="add-card-toolbar-control-group">
        <select
          className={selectPill}
          value={holdRule}
          onChange={(event) => setHoldRule(event.target.value)}
          aria-label={t("settings.autoPassHold")}
        >
          <option value="never">{t("hold.never")}</option>
          <option value="if_actions">{t("hold.ifActions")}</option>
          <option value="stack">{t("hold.stack")}</option>
          <option value="main">{t("hold.main")}</option>
          <option value="combat">{t("hold.combat")}</option>
          <option value="ending">{t("hold.ending")}</option>
          <option value="always">{t("hold.always")}</option>
        </select>
        <label className="toolbar-checkbox add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap cursor-pointer">
          <Checkbox
            checked={autoPassEnabled}
            onCheckedChange={(value) => setAutoPassEnabled(!!value)}
            className="h-3.5 w-3.5"
          />
          {t("action.autoPass")}
        </label>
      </div>
    </div>
  );
}
