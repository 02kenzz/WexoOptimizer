import { describe, expect, it } from "vitest";

import ar from "./locales/ar.json";
import de from "./locales/de.json";
import en from "./locales/en.json";
import es from "./locales/es.json";
import fr from "./locales/fr.json";
import ru from "./locales/ru.json";
import zh from "./locales/zh.json";

const LOCALES: Record<string, unknown> = { es, zh, de, fr, ar, ru };

/** Flattens a nested translation object into `path -> string` pairs. */
function flatten(value: unknown, prefix = "", out = new Map<string, string>()) {
  if (!value || typeof value !== "object") return out;
  for (const [key, child] of Object.entries(value)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (child && typeof child === "object") flatten(child, path, out);
    else out.set(path, String(child));
  }
  return out;
}

/** Extracts the `{{name}}` placeholders a string interpolates, sorted. */
function placeholders(value: string): string[] {
  return (value.match(/\{\{[^}]+\}\}/g) ?? []).sort();
}

const source = flatten(en);

describe("locales", () => {
  it("ships the same keys as the English source", () => {
    const expected = [...source.keys()];

    for (const [language, locale] of Object.entries(LOCALES)) {
      const keys = [...flatten(locale).keys()];
      expect(keys, `${language} key order`).toEqual(expected);
    }
  });

  it("keeps every interpolation placeholder", () => {
    for (const [language, locale] of Object.entries(LOCALES)) {
      for (const [path, value] of flatten(locale)) {
        const reference = source.get(path);
        expect(reference, `${language}.${path} exists in English`).toBeDefined();
        expect(placeholders(value), `${language}.${path}`).toEqual(placeholders(reference ?? ""));
      }
    }
  });

  it("has no empty translations", () => {
    for (const [language, locale] of Object.entries(LOCALES)) {
      for (const [path, value] of flatten(locale)) {
        expect(value.trim(), `${language}.${path}`).not.toBe("");
      }
    }
  });
});
