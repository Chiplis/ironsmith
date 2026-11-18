import { registerMageFixtureSource } from "./custom-fixtures.mjs";

// Planechase cards are absent from the normal-format card catalog.
// Oracle sources from Scryfall; the game runs them through the native compiler.
const planarSources = [
  {
    name: "Undercity Reaches",
    typeLine: "Plane — Ravnica",
    oracleText: "Whenever a creature deals combat damage to a player, its controller may draw a card.\nWhenever chaos ensues, you have no maximum hand size for the rest of the game.",
  },
  {
    name: "Hedron Fields of Agadeem",
    typeLine: "Plane — Zendikar",
    oracleText: "Creatures with power 7 or greater can't attack or block.\nWhenever chaos ensues, create a 7/7 colorless Eldrazi creature token with annihilator 1.",
  },
  {
    name: "Panopticon",
    typeLine: "Plane — Mirrodin",
    oracleText: "When you planeswalk to Panopticon, draw a card.\nAt the beginning of your draw step, draw an additional card.\nWhenever chaos ensues, draw a card.",
  },
  {
    name: "Tazeem",
    typeLine: "Plane — Zendikar",
    oracleText: "Creatures can't block.\nWhenever chaos ensues, draw a card for each land you control.",
  },
  {
    name: "Gavony",
    typeLine: "Plane — Innistrad",
    oracleText: "All creatures have vigilance.\nWhenever chaos ensues, creatures you control gain indestructible until end of turn.",
  },
  {
    name: "Krosa",
    typeLine: "Plane — Dominaria",
    oracleText: "All creatures get +2/+2.\nWhenever chaos ensues, you may add {W}{U}{B}{R}{G}.",
  },
  {
    name: "Naya",
    typeLine: "Plane — Alara",
    oracleText: "You may play any number of lands on each of your turns.\nWhenever chaos ensues, target red, green, or white creature you control gets +1/+1 until end of turn for each land you control.",
  },
  {
    name: "Llanowar",
    typeLine: "Plane — Dominaria",
    oracleText: "All creatures have \"{T}: Add {G}{G}.\"\nWhenever chaos ensues, untap all creatures you control.",
  },
  {
    name: "Goldmeadow",
    typeLine: "Plane — Lorwyn",
    oracleText: "Whenever a land enters, that land's controller creates three 0/1 white Goat creature tokens.\nWhenever chaos ensues, create a 0/1 white Goat creature token.",
  },
  {
    name: "Turri Island",
    typeLine: "Plane — Ir",
    oracleText: "Creature spells cost {2} less to cast.\nWhenever chaos ensues, reveal the top three cards of your library. Put all creature cards revealed this way into your hand and the rest into your graveyard.",
  },
];

export function registerPlanarFixtureSources(game, names) {
  for (const source of planarSources) {
    if (names.includes(source.name) && !game.isKnownCardName(source.name)) registerMageFixtureSource(game, source);
  }
}
