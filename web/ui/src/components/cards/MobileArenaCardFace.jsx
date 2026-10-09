import { landManaColors, LAND_MANA_COLORS } from '@/lib/land-mana-colors';
import LoadingCardArt from './LoadingCardArt';
import { cardArtCropUrl } from '@/lib/card-image-variants';
import { cardArtColors } from '@/lib/card-art-colors';
import { arenaFrameColor, arenaPermanentKind } from '@/lib/mobile-arena';

export default function MobileArenaCardFace({ card, name, artUrl, pending, primary, secondary }) {
  const kind = arenaPermanentKind(card);
  const manaColors = landManaColors(card);
  const statKind = kind === 'planeswalker' ? 'loyalty' : kind === 'battle' ? 'defense' : null;
  const liveCounter = statKind && (card.counters || []).find?.(counter => String(counter.kind).toLowerCase() === statKind);
  const statistic = statKind ? { label: card[statKind] ?? liveCounter?.amount ?? 0, title: statKind } : primary;
  const extraCounter = secondary && !(statKind && Number(secondary.label) === Number(statistic.label)) ? secondary : null;
  const token = Boolean(card.is_token || card.token);
  const chapters = [...new Set(String(card.oracle_text || '').match(/^(?:I|II|III|IV|V|VI)(?=\s*[—,])/gm) || [])];
  return <div className="arena-permanent" data-kind={kind} data-mana-strip={manaColors.length ? "true" : undefined} data-token={token || undefined}
    style={{ '--arena-frame': arenaFrameColor(card) }}>
    <div className="arena-permanent-art">
      <LoadingCardArt src={cardArtCropUrl(artUrl)} sourceKey={artUrl} pending={pending}
        colors={cardArtColors(card)} variant="battlefield" alt="" draggable={false} />
    </div>
    <div className="arena-permanent-name" title={name || card.name}>{name || card.name}</div>
    {statistic && <span className={kind === 'creature'
      ? ['arena-permanent-pt', 'battlefield-pt-badge',
        card.pt_modified_by_effect && 'battlefield-pt-badge--modified',
        card.summoning_sick && 'battlefield-pt-badge--summoning-sick'].filter(Boolean).join(' ')
      : 'arena-permanent-stat'}
      data-summoning-sick={kind === 'creature' && card.summoning_sick === true ? 'true' : undefined}
      data-modified={kind === 'creature' && card.pt_modified_by_effect === true ? 'true' : undefined}
      title={kind === 'creature' && card.summoning_sick ? 'Summoning sickness' : statistic.title}
      aria-label={kind === 'creature' && card.summoning_sick ? `${statistic.title || 'Power/toughness'} — summoning sickness` : statistic.title}>{statistic.label}</span>}
    {extraCounter && <span className="arena-permanent-counter" aria-label={extraCounter.title}>{extraCounter.label}</span>}
    {kind === 'saga' && chapters.length > 0 && <span className="arena-saga-chapters" aria-label="Saga chapters">{chapters.map(chapter => <span key={chapter} style={{display:'block'}}>{chapter}</span>)}</span>}
    {manaColors.length > 0 && <div className="arena-land-mana-strip" aria-label={`Produces ${manaColors.map(color => LAND_MANA_COLORS[color].name.toLowerCase()).join(', ')} mana`}>
      {manaColors.map(color => <span key={color} data-mana-color={color} aria-label={LAND_MANA_COLORS[color].name} style={{backgroundColor: LAND_MANA_COLORS[color].background}} />)}
    </div>}
    {card.tapped && <span className="arena-tapped-mark" aria-label="Tapped">↷</span>}
  </div>;
}
