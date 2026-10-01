import { createContext, useContext } from "react";
export const ManaPaymentEditorContext = createContext(null);
export function useManaPaymentEditor() { return useContext(ManaPaymentEditorContext); }
