import useUiText from '@/i18n/useUiText';
import { cardSimpleTapManaColors } from '@/lib/land-mana-choices';
import { landManaColors } from '@/lib/land-mana-colors';
import LandManaStrip from './LandManaStrip';
import LoadingCardArt from './LoadingCardArt';
import BattlefieldGroupBadge from './BattlefieldGroupBadge';
import { cardArtCropUrl } from '@/lib/card-image-variants';
import { cardArtColors } from '@/lib/card-art-colors';
import { arenaFrameColor, arenaPermanentKind } from '@/lib/mobile-arena';

export default function MobileArenaCardFace({ card, name, artUrl, pending, primary, manaChoices, manaStripOpen, onManaChoice }) {
  const ui = useUiText();
  const kind = arenaPermanentKind(card);
  const sagaArt = kind === 'saga' || card.layout === 'saga' || /\bSaga\b/i.test(card.type_line || '');
  const passiveManaColors = landManaColors(card);
  const manaColors = manaStripOpen && manaChoices?.length
    ? manaChoices.map(choice => choice.color) : passiveManaColors.length ? passiveManaColors : cardSimpleTapManaColors(card);
  const statKind = kind === 'planeswalker' ? 'loyalty' : kind === 'battle' ? 'defense' : null;
  const liveCounter = statKind && (card.counters || []).find?.(counter => String(counter.kind).toLowerCase() === statKind);
  const statistic = statKind ? { label: card[statKind] ?? liveCounter?.amount ?? 0, title: statKind } : primary;
  const token = Boolean(card.is_token || card.token);
  const chapters = [...new Set(String(card.oracle_text || '').match(/^(?:I|II|III|IV|V|VI)(?=\s*[—,])/gm) || [])];
  return <div className="arena-permanent" data-kind={kind} data-mana-strip={manaColors.length ? "true" : undefined} data-token={token || undefined}
    style={{ '--arena-frame': arenaFrameColor(card) }}>
    <div className="arena-permanent-art" data-saga-art={sagaArt || undefined}>
      <LoadingCardArt src={cardArtCropUrl(artUrl)} sourceKey={artUrl} pending={pending}
        colors={cardArtColors(card)} variant="battlefield" alt="" draggable={false} />
    </div>
    {!token && <div className="arena-permanent-name" title={name || card.name}>{name || card.name}</div>}
    {statistic && <span className={kind === 'creature'
      ? ['arena-permanent-pt', 'battlefield-pt-badge',
        card.pt_modified_by_effect && 'battlefield-pt-badge--modified',
        card.summoning_sick && 'battlefield-pt-badge--summoning-sick'].filter(Boolean).join(' ')
      : 'arena-permanent-stat'}
      data-summoning-sick={kind === 'creature' && card.summoning_sick === true ? 'true' : undefined}
      data-modified={kind === 'creature' && card.pt_modified_by_effect === true ? 'true' : undefined}
      title={kind === 'creature' && card.summoning_sick ? 'Summoning sickness' : statistic.title}
      aria-label={kind === 'creature' && card.summoning_sick ? `${statistic.title || 'Power/toughness'} — summoning sickness` : statistic.title}>{statistic.label}</span>}
    <BattlefieldGroupBadge count={card.count} />
    {kind === 'saga' && chapters.length > 0 && <span className="arena-saga-chapters" aria-label={ui('Saga chapters')}>{chapters.map(chapter => <span key={chapter} style={{display:'block'}}>{chapter}</span>)}</span>}
    {manaColors.length > 0 && <LandManaStrip colors={manaColors} choices={manaChoices} open={manaStripOpen}
      cardName={name || card.name} onChoose={onManaChoice} />}
    {card.tapped && <span className="arena-tapped-mark" aria-label={ui('Tapped')}>↷</span>}
  </div>;
}
