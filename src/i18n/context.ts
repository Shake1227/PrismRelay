import { createContext } from "react";
import type { Language } from "../models";

export const LanguageContext = createContext<Language>("ja");
