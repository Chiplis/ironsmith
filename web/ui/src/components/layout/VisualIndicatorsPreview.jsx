import GameCard from "@/components/cards/GameCard";
import useUiText from "@/i18n/useUiText";
import "./visual-indicators-preview.css";

// A focused visual fixture: the values are snapshots, not changes to a match.
// In particular, power_toughness includes counters for rules purposes while
// power_toughness_without_counters is what the battlefield footer displays.
const cases = [
  {
    title: "Aura + counter + summoning sickness",
    description: "Runeclaw Bear 2/2 · Rancor +2/+0 · one +1/+1 counter. Actual total: 5/3.",
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
    title: "Counter only",
    description: "Runeclaw Bear 2/2 · one +1/+1 counter. Actual total: 3/3.",
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
    title: "Aura only",
    description: "Runeclaw Bear 2/2 · Rancor +2/+0. The violet 4/2 already includes the aura.",
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
    title: "Just arrived",
    description: "Runeclaw Bear 2/2 · violet spiral: it can't attack yet.",
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
    title: "Ready to attack",
    description: "Runeclaw Bear 2/2 · no spiral once summoning sickness wears off.",
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
  const ui = useUiText();
  return (
    <main className="visual-indicators-preview">
      <header className="visual-indicators-preview__header">
        <a href="/?test=compile">{ui("← Back to Compile Card")}</a>
        <h1>{ui("Creature visual test")}</h1>
        <p>{ui("The counter pill shows only the +1/+1. The number below shows power/toughness without that counter; with an active aura it turns violet.")}</p>
      </header>
      <div className="visual-indicators-preview__grid">
        {cases.map(({ title, description, card }) => (
          <section className="visual-indicators-preview__case" key={card.id}>
            <h2>{ui(title)}</h2>
            <div className="visual-indicators-preview__card">
              <GameCard card={card} variant="battlefield" battlefieldVisualMode="portrait" suppressTooltip />
            </div>
            <p>{ui(description)}</p>
          </section>
        ))}
      </div>
      <p className="visual-indicators-preview__note">{ui("This scene uses the same battlefield card component with isolated test data. It does not change the match.")}</p>
    </main>
  );
}
