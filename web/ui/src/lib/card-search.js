// Fold search keys only; keep the original spelling for display and identity.
export function cardSearchKey(value) {
  return String(value || '').trim().normalize('NFD').replace(/\p{M}/gu, '').toLocaleLowerCase('en-US');
}
