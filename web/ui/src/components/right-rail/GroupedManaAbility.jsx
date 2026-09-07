import { SymbolText } from '@/lib/mana-symbols';
import './grouped-mana-ability.css';

export default function GroupedManaAbility({ group, onActivate, name, className, style }) {
  return <span className={`inspector-mana-line ${className || ''}`} style={style}>
    <SymbolText text={group.prefix} />
    {group.parts.map((part, index) => {
      if (part.type === 'literal') return <span key={index}>{part.value}</span>;
      const option = group.options.find(option => option.output === part.value);
      const action = option.actions.find(action => !action.payment_pending && action.mana_payment_available !== false);
      const available = Boolean(action && onActivate);
      return <button key={index} type="button" className="inspector-mana-choice"
        data-available={available ? 'true' : 'false'} disabled={!available}
        aria-label={`${name || 'Card'}: ${group.prefix}${option.output}${group.suffix}`}
        onPointerDown={event => event.stopPropagation()}
        onClick={event => {
          event.preventDefault();
          event.stopPropagation();
          if (available) onActivate(action);
        }}>
        <SymbolText text={option.output} />
      </button>;
    })}
    <SymbolText text={group.suffix} />
  </span>;
}
