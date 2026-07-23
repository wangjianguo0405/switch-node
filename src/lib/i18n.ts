import zhCN from "../locales/zh-CN.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

const messages: Record<string, Record<string, string>> = {
  "zh-CN": zhCN as Record<string, string>,
  en: en as Record<string, string>,
  ja: ja as Record<string, string>,
};

let currentLang = "zh-CN";

export function setLanguage(lang: string): void {
  if (messages[lang]) {
    currentLang = lang;
  }
}

export function getLanguage(): string {
  return currentLang;
}

export function t(key: string, replacements?: Record<string, string | number>): string {
  const msg = messages[currentLang]?.[key] ?? messages["en"]?.[key] ?? key;
  if (!replacements) return msg;
  return msg.replace(/\{(\w+)\}/g, (_, k) =>
    String(replacements[k] ?? `{${k}}`)
  );
}
