import { useState } from 'react';
import useUiText from '@/i18n/useUiText';
import { filterCatalogRows } from '@/lib/build-catalog';

export default function BuildCatalogList({ rows, actions, showSource = false, label = 'Search set contents' }) {
  const ui = useUiText();
  const [query, setQuery] = useState('');
  const [limit, setLimit] = useState(60);
  const filtered = filterCatalogRows(rows, query);
  return <div className="grid min-w-0 gap-3">
    <input type="search" className="fantasy-field w-full min-w-0 px-3 py-2" aria-label={ui(label)} placeholder={ui(label)} value={query} onChange={event => { setQuery(event.target.value); setLimit(60); }} />
    <p className="text-xs text-muted-foreground" role="status">{filtered.length} / {rows.length} {ui('entries')}</p>
    <ul className="grid gap-2" data-testid="build-catalog-list">
      {filtered.slice(0, limit).map(row => <li key={`${row.source}:${row.index}`} className="min-w-0 border-b border-white/10 py-2">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <details className="min-w-0 flex-1 break-words">
            <summary className="cursor-pointer font-medium"><span>{row.name}</span>{showSource && <span className="mt-1 block text-xs font-normal text-muted-foreground">{row.source}</span>}</summary>
            <p className="mt-2 whitespace-pre-wrap text-sm text-muted-foreground">{row.detail || ui('No rules text.')}</p>
            {row.cards && <div className="mt-2 text-sm"><span className="text-muted-foreground">{ui('Used by cards')} · {row.cards.length}</span><ul>{row.cards.map((name, index) => <li key={index}>{name}</li>)}</ul></div>}
          </details>
          {actions?.(row)}
        </div>
      </li>)}
    </ul>
    {!filtered.length && <p className="text-sm text-muted-foreground">{ui('No matching entries.')}</p>}
    {filtered.length > limit && <button className="stone-pill px-3 py-2 text-sm" onClick={() => setLimit(current => current + 60)}>{ui('Show more')}</button>}
  </div>;
}
