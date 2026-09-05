import { createContext } from "react";

// Shared with isolated UI fixtures so visual checks never mutate a real match.
export const GameContext = createContext(null);
