import {cardPrintingProfile} from './card-printing-profile.js';
const family = name => `"${name}", Georgia, serif`;
export function cardTypography(printing = {}) {
  const profile = cardPrintingProfile(printing);
  const era = profile.era;
  const title = family(era === 'retro' ? 'Goudy Medieval' : era === 'beleren' ? 'Beleren' : 'Matrix');
  const rules = family('MPlantin');
  const type = era === 'retro' ? rules : title;
  const stats = era === 'beleren' ? family('Beleren Small Caps') : era === 'modern' ? family('Matrix Small Caps') : rules;
  const titleWeight = era === 'retro' ? 400 : 700;
  const conventionalFrame = profile.conventional;
  return { profile, era, title, rules, type, stats, titleWeight, conventionalFrame, style: {
    '--card-title-font': title, '--card-type-font': type, '--card-rules-font': rules,
    '--card-stats-font': stats, '--card-title-weight': titleWeight,
    '--card-type-weight': titleWeight, '--card-stats-weight': era === 'retro' || era === 'future' ? 400 : 700,
  }};
}
