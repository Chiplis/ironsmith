import { useEffect, useState } from "react";
import CardFramePresentation from "@/components/cards/CardFramePresentation";
import { useScryfallImage } from "@/hooks/useScryfallImageUrl";
import { fetchCardFaceSource } from "@/lib/scryfall";
import useUiText from "@/i18n/useUiText";

// The front face of a face-down permanent whose identity this viewer knows
// (its controller, or a player who looked at it). Only the identity reaches the
// client, so the printed characteristics are loaded by name.
export default function FaceDownIdentityFrame({ name }) {
  const ui = useUiText();
  const [source, setSource] = useState(null);
  useEffect(() => {
    let active = true;
    setSource(null);
    fetchCardFaceSource(name).then(card => { if (active) setSource(card || null); }).catch(() => {});
    return () => { active = false; };
  }, [name]);
  const image = useScryfallImage(name, "art_crop");
  return <div className="face-down-identity-frame" data-face-down-identity={name}>
    <span className="face-down-identity-frame__label">{ui("Face-down card")}</span>
    <CardFramePresentation card={source || { name }} imageUrl={image.url} />
  </div>;
}
