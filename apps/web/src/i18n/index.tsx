// Interface languages (R94): every text the interface shows comes from en.json, zh-CN.json or ja.json.
//
// English is the reference. The other files must hold exactly the same keys: `typecheck` catches a
// missing one, and `npm run check:i18n` also catches extra or empty texts and mismatched {placeholders}.
// Rust sends problems as codes; their words are the `problem.<code>` texts (codes.json lists them).

import { createContext, Fragment, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import en from "./en.json";
import ja from "./ja.json";
import zhCN from "./zh-CN.json";

export type Key = keyof typeof en;
type Params = Record<string, string | number>;

/** A text to show later, kept as a key so it changes when the language does. */
export type Message = { key: Key; params?: Params };

export const LANGUAGES = ["en", "zh-CN", "ja"] as const;
export type Language = (typeof LANGUAGES)[number];

const MESSAGES: Record<Language, Record<Key, string>> = { en, "zh-CN": zhCN, ja };

/** Each language's name in that language, so anyone can find their own. */
export const LANGUAGE_NAMES: Record<Language, string> = { en: "English", "zh-CN": "简体中文", ja: "日本語" };

/** The first of the computer's preferred languages that Allomos speaks; English otherwise. */
export function pickLanguage(preferred: readonly string[]): Language {
  for (const tag of preferred) {
    const t = tag.toLowerCase();
    if (t === "en" || t.startsWith("en-")) return "en";
    if (t === "ja" || t.startsWith("ja-")) return "ja";
    // Traditional Chinese has no file of its own yet; Simplified is closer than English.
    if (t === "zh" || t.startsWith("zh-")) return "zh-CN";
  }
  return "en";
}

function fill(text: string, params: Params = {}): string {
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => (name in params ? String(params[name]) : whole));
}

const CHOICE_KEY = "allomos.language";

function readChoice(): Language | null {
  try {
    const v = localStorage.getItem(CHOICE_KEY);
    return (LANGUAGES as readonly string[]).includes(v ?? "") ? (v as Language) : null;
  } catch {
    return null;
  }
}

interface I18n {
  language: Language;
  /** The user's choice, or null to follow the computer. */
  choice: Language | null;
  setChoice(choice: Language | null): void;
  t(key: Key, params?: Params): string;
  /** Like `t`, but placeholders may be filled with elements, for example `<code>`. */
  rich(key: Key, parts: Record<string, ReactNode>): ReactNode;
  /** Words for a problem code sent by Rust. */
  problem(p: { code: string; params?: Params }): string;
}

const I18nContext = createContext<I18n | null>(null);

export function I18nProvider({ systemLanguages, children }: { systemLanguages: () => Promise<string[]>; children: ReactNode }) {
  const [system, setSystem] = useState<Language>(() => pickLanguage(navigator.languages ?? []));
  const [choice, setChoiceState] = useState<Language | null>(readChoice);

  useEffect(() => {
    systemLanguages()
      .then((tags) => tags.length > 0 && setSystem(pickLanguage(tags)))
      .catch(() => {
        /* keep the window's own guess */
      });
  }, [systemLanguages]);

  const language = choice ?? system;

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const setChoice = useCallback((c: Language | null) => {
    setChoiceState(c);
    try {
      if (c) localStorage.setItem(CHOICE_KEY, c);
      else localStorage.removeItem(CHOICE_KEY);
    } catch {
      /* storage unavailable: the choice lasts until the window closes */
    }
  }, []);

  const value = useMemo<I18n>(() => {
    const words = MESSAGES[language];
    // A key no language has comes from a value this Allomos does not know, such as a task state a
    // newer Allomos wrote (`state.parked`): show the value itself (Task-A01-piano403).
    const t = (key: Key, params?: Params) => fill(words[key] ?? en[key] ?? key.slice(key.indexOf(".") + 1), params);
    return {
      language,
      choice,
      setChoice,
      t,
      rich(key, parts) {
        const pieces = (words[key] ?? en[key]).split(/\{(\w+)\}/);
        // split() with a group alternates text, placeholder name, text, …
        return pieces.map((piece, n) => <Fragment key={n}>{n % 2 === 1 ? (parts[piece] ?? `{${piece}}`) : piece}</Fragment>);
      },
      problem(p) {
        const key = `problem.${p.code}`;
        return key in words ? t(key as Key, p.params) : p.code;
      },
    };
  }, [language, choice, setChoice]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18n {
  const v = useContext(I18nContext);
  if (!v) throw new Error("useI18n outside I18nProvider");
  return v;
}
