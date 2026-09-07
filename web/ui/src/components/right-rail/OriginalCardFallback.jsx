import { SymbolText } from '@/lib/mana-symbols';
import GroupedManaAbility from './GroupedManaAbility';
import './original-card-fallback.css';

// When text regions cannot be masked, preserve the printing. Live rules and
// actions remain available in an explicit details panel, not a fake card frame.
export default function OriginalCardFallback({ imageUrl, name, rulesView, onActivate, highlighted, flavorText, stats, counters, detailsLabel }) {
  return <article className="original-card-fallback" aria-label={name || 'Card details'}>
    {imageUrl && <img src={imageUrl} alt={name || 'Card'} referrerPolicy="no-referrer" decoding="async" />}
    <details className="original-card-details" open={!imageUrl || undefined}>
      <summary>{detailsLabel}</summary>
      <div className="original-card-details__body">
        <strong>{name}</strong>
        {(stats || counters) && <p>{[stats, counters].filter(Boolean).join(' · ')}</p>}
        {rulesView.lines.map((line, index) => {
          const actions = rulesView.actions.get(index) || [];
          const action = actions.find(action => !action.payment_pending && action.mana_payment_available !== false);
          const available = Boolean(action && onActivate);
          return <div key={index} className="inspector-ability-section" data-stack-highlighted={highlighted.has(index) ? 'true' : undefined}>
            {rulesView.manaGroups.has(index) ? <GroupedManaAbility group={rulesView.manaGroups.get(index)} name={name} onActivate={onActivate} />
              : actions.length || /[:：]/u.test(line) ? <button type="button" className="inspector-oracle-line-action"
                data-available={available ? 'true' : 'false'} disabled={!available}
                aria-label={`${name || 'Card'}: ${line}`}
                onPointerDown={event => event.stopPropagation()}
                onClick={event => { event.stopPropagation(); if (available) onActivate(action); }}>
                <SymbolText text={line} />
              </button> : <SymbolText text={line} />}
          </div>;
        })}
        {flavorText && <p aria-label="Flavor text"><em>{flavorText}</em></p>}
      </div>
    </details>
  </article>;
}
