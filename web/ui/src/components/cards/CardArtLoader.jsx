import { MANA_SYMBOL_SVGS } from "@/lib/mana-symbol-svg";
import { cardArtSymbolLayout } from "@/lib/card-art-colors";
import useUiText from "@/i18n/useUiText";

/** Art-only placeholder: never takes pointer input or changes card geometry. */
export default function CardArtLoader({ variant = "hand", failed = false, colors = [] }) {
  const ui = useUiText();
  return <span className={`card-art-loader card-art-loader--${variant}`}
    data-card-art-loader={failed ? "unavailable" : "loading"} data-variant={variant}
    role="img" aria-label={ui(failed ? "Card art unavailable" : "Loading card art")}>
    <span className="card-art-loader__ring" aria-hidden="true" />
    <svg className="card-art-loader__sigil" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid meet" aria-hidden="true" focusable="false">
      {cardArtSymbolLayout(colors).map(({ color, size, x, y }) => (
        <svg key={color} data-mana-color={color} x={x} y={y} width={size} height={size}
          viewBox={MANA_SYMBOL_SVGS[color].vb}
          dangerouslySetInnerHTML={{ __html: MANA_SYMBOL_SVGS[color].html }} />
      ))}
    </svg>
    <span className="card-art-loader__sweep" aria-hidden="true" />
  </span>;
}
