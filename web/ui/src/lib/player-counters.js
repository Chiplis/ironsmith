export function playerCounters(player) {
  const counters = Array.isArray(player?.counters) ? player.counters : [
    { kind: "poison", amount: player?.poison_counters },
    { kind: "energy", amount: player?.energy_counters },
    { kind: "experience", amount: player?.experience_counters },
  ];
  return counters.filter(({ amount }) => Number.isFinite(Number(amount)) && Number(amount) > 0);
}
