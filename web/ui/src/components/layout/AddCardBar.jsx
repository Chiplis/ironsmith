import { useGame } from "@/context/GameContext";
import { Checkbox } from "@/components/ui/checkbox";
import { useI18n } from "@/i18n/I18nContext";


export default function AddCardBar({
  compact = false,
  utilityControls,
}) {
  const {
    autoResolveEnabled,
    setAutoResolveEnabled,
  } = useGame();
  const { t } = useI18n();

  return (
    <div className={`add-card-toolbar table-toolbar table-toolbar--secondary rounded-none px-3 py-2${compact ? " add-card-toolbar--compact" : ""}`}>
      <div className="add-card-toolbar-zone-group">
        {utilityControls}
      </div>

      <span className="add-card-toolbar-separator add-card-toolbar-control-separator" aria-hidden="true" />

      <div className="add-card-toolbar-control-group">

        <label className="toolbar-checkbox add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap cursor-pointer">
          <Checkbox
            checked={autoResolveEnabled}
            onCheckedChange={(value) => setAutoResolveEnabled(!!value)}
            className="h-3.5 w-3.5"
          />
          {t("action.autoPass")}
        </label>
      </div>
    </div>
  );
}
