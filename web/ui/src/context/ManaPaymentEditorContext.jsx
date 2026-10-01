import { ManaPaymentEditorContext } from "./ManaPaymentEditorContext.shared";
import usePaymentDraft from "@/hooks/usePaymentDraft";
export function ManaPaymentEditorProvider({ state, dispatch, cancelBackgroundDispatch, children }) {
  const editor = usePaymentDraft({ payment: state?.mana_payment, dispatch, cancelBackgroundDispatch });
  return <ManaPaymentEditorContext.Provider value={editor}>{children}</ManaPaymentEditorContext.Provider>;
}
