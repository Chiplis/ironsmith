import { ManaSymbol } from '@/lib/mana-symbols';
import { LAND_MANA_COLORS } from '@/lib/land-mana-colors';
import useUiText from '@/i18n/useUiText';

// A land's lower border: one section per color of mana it makes. Clicking the
// land opens it to three times its height; each section then shows its mana
// symbol and taps the land for that color.
export default function LandManaStrip({ colors, choices = [], open = false, overlay = false, cardName, onChoose }) {
  const ui = useUiText();
  const sections = open && choices.length ? choices : (colors || []).map(color => ({ color }));
  if (!sections.length) return null;
  return <div className="arena-land-mana-strip" data-open={open && choices.length ? 'true' : undefined} data-overlay={overlay ? 'true' : undefined}
    role={open ? 'group' : undefined}
    aria-label={open ? ui('Tap {0} for mana', { 0: cardName || '' }) : `Produces ${sections.map(({ color }) => LAND_MANA_COLORS[color].name.toLowerCase()).join(', ')} mana`}>
    {sections.map(({ color, action }) => open && action
      ? <button key={color} type="button" data-mana-color={color} style={{ backgroundColor: LAND_MANA_COLORS[color].background }}
        aria-label={ui('Add {0} mana', { 0: ui(LAND_MANA_COLORS[color].name).toLowerCase() })}
        onPointerDown={event => event.stopPropagation()}
        onClick={event => { event.preventDefault(); event.stopPropagation(); onChoose?.(action, color); }}>
        <ManaSymbol sym={color} size={14} />
      </button>
      : <span key={color} data-mana-color={color} aria-label={LAND_MANA_COLORS[color].name} style={{ backgroundColor: LAND_MANA_COLORS[color].background }} />)}
  </div>;
}
