export default function BattlefieldGroupBadge({ count }) {
  const size = Number(count);
  if (!Number.isFinite(size) || size <= 1) return null;

  return <span className="battlefield-group-badge">x{size}</span>;
}
