const family = name => `"${name}", Georgia, serif`;
export function cardTypography(printing = {}) {
  const frameEra = {1993: 'retro', 1997: 'retro', 2003: 'modern', 2015: 'beleren', future: 'future'}[printing.frame];
  const date = /^\d{4}-\d{2}-\d{2}$/.test(printing.released_at || '') ? printing.released_at : null;
  const era = frameEra || (date ? date < '2003-07-28' ? 'retro' : date < '2014-07-18' ? 'modern' : 'beleren' : 'beleren');
  const title = family(era === 'retro' ? 'Goudy Medieval' : era === 'beleren' ? 'Beleren' : 'Matrix');
  const rules = family('MPlantin');
  const type = era === 'retro' ? rules : title;
  const stats = era === 'beleren' ? family('Beleren Small Caps') : era === 'modern' ? family('Matrix Small Caps') : rules;
  const titleWeight = era === 'retro' ? 400 : 700;
  return { era, title, rules, type, stats, titleWeight, style: {
    '--card-title-font': title, '--card-type-font': type, '--card-rules-font': rules,
    '--card-stats-font': stats, '--card-title-weight': titleWeight,
    '--card-type-weight': titleWeight, '--card-stats-weight': era === 'retro' || era === 'future' ? 400 : 700,
  }};
}
