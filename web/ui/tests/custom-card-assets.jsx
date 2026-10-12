import React from "react";
import {createRoot} from "react-dom/client";
import {I18nProvider} from "../src/i18n/I18nContext";
import MiniatureCardFrame from "../src/components/cards/MiniatureCardFrame";
import {setCompiledCardNames} from "../src/lib/scryfall";
import {registerCustomCardFlavor} from "../src/lib/custom-card-flavor";
import {customCardImageUrl} from "../src/lib/custom-card-assets";
import pool from "../public/custommagic/pool.json";
import "../src/index.css";

const names = ["Cleveland", "Magikarp", "Tribute To...", "Winrar", "Syl of Many Deaths", "Thoughts"];
const cards = pool.cards.filter(card => names.includes(card.draft.faces[0].name));
setCompiledCardNames(names);
registerCustomCardFlavor(cards);
createRoot(document.getElementById("frames")).render(<I18nProvider>
  <div style={{display: "flex", flexWrap: "wrap", gap: 24}}>{cards.map((entry, i) => {
    const face = entry.draft.faces[0];
    const card = {id: i + 1, name: face.name, card_types: face.cardTypes,
      mana_cost: face.manaCost, oracle_text: face.oracleText, power_toughness: face.power ? face.power + "/" + face.toughness : null};
    return <section key={entry.id} data-custom-name={face.name} style={{display: "flex", gap: 8}}>
      <img alt={face.name + " full print"} src={customCardImageUrl(face.name)} style={{width: 244, height: 340, objectFit: "contain"}} />
      <div style={{position: "relative", width: 244, height: 340}}><MiniatureCardFrame card={card} imageUrl={customCardImageUrl(face.name)} /></div>
    </section>;
  })}</div>
</I18nProvider>);
