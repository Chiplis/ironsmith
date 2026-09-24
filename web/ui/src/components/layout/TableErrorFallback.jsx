import useUiText from "@/i18n/useUiText";
import { Button } from "@/components/ui/button";

export default function TableErrorFallback({ error, onRetry }) {
  const ui = useUiText();
  return (
    <div
      role="alert"
      className="flex h-full min-h-0 flex-col items-center justify-center gap-3 px-6 text-center"
    >
      <span className="text-[18px] font-bold uppercase tracking-wider text-destructive">
        {ui("The table hit a rendering error")}
      </span>
      <span className="max-w-2xl text-sm text-muted-foreground">
        {ui("Your game is still running. Try again to redraw the table, or reload the page if the error keeps happening.")}
      </span>
      {error?.message ? (
        <code className="max-w-2xl break-words text-xs text-muted-foreground">{error.message}</code>
      ) : null}
      <div className="flex gap-2">
        <Button size="sm" onClick={onRetry}>{ui("Try again")}</Button>
        <Button size="sm" variant="outline" onClick={() => globalThis.location?.reload()}>
          {ui("Reload page")}
        </Button>
      </div>
    </div>
  );
}
