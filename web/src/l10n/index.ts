import { writable, derived } from "svelte/store";
import l10nData from "./config";

export const langs = l10nData.langs;

export const locale = (() => {
  const { subscribe, set, update } = writable<string>(l10nData.default);
  return {
    subscribe,
    set: (value: string) => {
      if (value in l10nData.trans) {
        set(value);
      } else {
        console.warn(`Locale "${value}" is not supported.`);
      }
    },
    update: (updateFn: (current: string) => string) => {
      update((current) => {
        const newValue = updateFn(current);
        if (newValue in l10nData.trans) {
          return newValue;
        } else {
          console.warn(`Locale "${newValue}" is not supported.`);
          return current;
        }
      });
    },
  };
})();

export const l10nDerive = derived(locale, ($locale) => {
  const trans = l10nData.trans[$locale] ?? {};
  const indexMap = trans["l10n/index"] ?? {};
  return (path: string) => {
    let ns = path.replace(/\.[^.]+$/, "");
    const nsMap = trans[ns] ?? {};
    return (key: string, params?: Record<string, string | number>): string => {
      let text = key.startsWith(":")
        ? (indexMap[key.slice(1)] ?? key.slice(1))
        : (nsMap[key] ?? key);
      if (params) {
        for (const [k, v] of Object.entries(params)) {
          text = text.replace(`{${k}}`, String(v));
        }
      }
      return text;
    };
  };
});

const STORAGE_KEY = "mdoc-locale";

function detectLocale(): string {
  try {
    const lang = navigator.language.toLowerCase();
    if (lang in l10nData.trans) return lang;
    const base = lang.split("-")[0];
    for (const key of Object.keys(l10nData.trans)) {
      if (key.startsWith(base)) return key;
    }
  } catch {}
  return l10nData.default;
}

export function init() {
  const saved = window.localStorage?.getItem?.(STORAGE_KEY);
  if (saved && saved in l10nData.trans) {
    locale.set(saved);
  } else {
    locale.set(detectLocale());
  }
  locale.subscribe(($locale) => {
    try {
      window.localStorage.setItem(STORAGE_KEY, $locale);
    } catch (e) {
      console.warn("Failed to save locale to localStorage:", e);
    }
  });
}
