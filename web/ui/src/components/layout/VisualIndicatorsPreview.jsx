import GameCard from "@/components/cards/GameCard";
import "./visual-indicators-preview.css";

// A focused visual fixture: the values are snapshots, not changes to a match.
// In particular, power_toughness includes counters for rules purposes while
// power_toughness_without_counters is what the battlefield footer displays.
const cases = [
  {
    title: "Aura + contador + mareo",
    description: "Runeclaw Bear 2/2 · Rancor +2/+0 · un contador +1/+1. Total real: 5/3.",
    card: {
      id: 91001,
      name: "Runeclaw Bear",
      type_line: "Creature — Bear",
      power_toughness: "5/3",
      power_toughness_without_counters: "4/2",
      counters: [{ kind: "+1/+1", amount: 1 }],
      counter_signature: "+1/+1:1",
      has_active_aura: true,
      summoning_sick: true,
    },
  },
  {
    title: "Solo contador",
    description: "Runeclaw Bear 2/2 · un contador +1/+1. Total real: 3/3.",
    card: {
      id: 91002,
      name: "Runeclaw Bear",
      type_line: "Creature — Bear",
      power_toughness: "3/3",
      power_toughness_without_counters: "2/2",
      counters: [{ kind: "+1/+1", amount: 1 }],
      counter_signature: "+1/+1:1",
      summoning_sick: true,
    },
  },
  {
    title: "Solo aura",
    description: "Runeclaw Bear 2/2 · Rancor +2/+0. El 4/2 violeta ya incluye el aura.",
    card: {
      id: 91003,
      name: "Runeclaw Bear",
      type_line: "Creature — Bear",
      power_toughness: "4/2",
      power_toughness_without_counters: "4/2",
      has_active_aura: true,
      summoning_sick: false,
    },
  },
  {
    title: "Recién entrada",
    description: "Runeclaw Bear 2/2 · espiral violeta: todavía no puede atacar.",
    card: {
      id: 91004,
      name: "Runeclaw Bear",
      type_line: "Creature — Bear",
      power_toughness: "2/2",
      power_toughness_without_counters: "2/2",
      summoning_sick: true,
    },
  },
  {
    title: "Ya puede atacar",
    description: "Runeclaw Bear 2/2 · sin espiral después de superar el mareo.",
    card: {
      id: 91005,
      name: "Runeclaw Bear",
      type_line: "Creature — Bear",
      power_toughness: "2/2",
      power_toughness_without_counters: "2/2",
      summoning_sick: false,
    },
  },
];

export default function VisualIndicatorsPreview() {
  return (
    <main className="visual-indicators-preview">
      <header className="visual-indicators-preview__header">
        <a href="/?test=compile">← Volver a Compile Card</a>
        <h1>Prueba visual de criaturas</h1>
        <p>La pastilla del contador muestra solo +1/+1. El número inferior muestra la fuerza/resistencia sin sumar ese contador; si hay un aura activa, se vuelve violeta.</p>
      </header>
      <div className="visual-indicators-preview__grid">
        {cases.map(({ title, description, card }) => (
          <section className="visual-indicators-preview__case" key={card.id}>
            <h2>{title}</h2>
            <div className="visual-indicators-preview__card">
              <GameCard card={card} variant="battlefield" battlefieldVisualMode="portrait" suppressTooltip />
            </div>
            <p>{description}</p>
          </section>
        ))}
      </div>
      <p className="visual-indicators-preview__note">Esta escena usa el mismo componente de carta del battlefield, con datos de prueba aislados. No altera la partida.</p>
    </main>
  );
}
