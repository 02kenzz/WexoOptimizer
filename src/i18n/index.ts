import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import ar from "./locales/ar.json";
import de from "./locales/de.json";
import en from "./locales/en.json";
import es from "./locales/es.json";
import fr from "./locales/fr.json";
import ru from "./locales/ru.json";
import zh from "./locales/zh.json";

/** Languages written right to left. */
const RTL_LANGUAGES = new Set(["ar"]);

/** Mirrors the active language onto `<html>` so the layout follows the text. */
function applyLanguageAttributes(language: string) {
  if (typeof document === "undefined") return;
  document.documentElement.lang = language;
  document.documentElement.dir = RTL_LANGUAGES.has(language) ? "rtl" : "ltr";
}

i18n.on("languageChanged", applyLanguageAttributes);

void i18n.use(initReactI18next).init({
  resources: {
    es: { translation: es },
    en: { translation: en },
    zh: { translation: zh },
    de: { translation: de },
    fr: { translation: fr },
    ar: { translation: ar },
    ru: { translation: ru },
  },
  lng: "es",
  fallbackLng: "es",
  interpolation: { escapeValue: false },
  returnNull: false,
});

applyLanguageAttributes(i18n.language || "es");

export default i18n;
