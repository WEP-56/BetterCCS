import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";
import zh from "./locales/zh.json";

type Language = "zh" | "en";

const DEFAULT_LANGUAGE: Language = "zh";

const isLanguage = (value: unknown): value is Language =>
  value === "zh" || value === "en";

/**
 * 旧版本（上游 cc-switch）可能把语言存成 `zh-TW` / `ja` / `tr`：这些语言本分支
 * 已经不提供，落到能读的那一档上，别让界面掉回系统语言。繁中读简中、日文与
 * 土耳其文读英文，都是各自最接近的选择。
 */
const normalizeLegacyLanguage = (value: unknown): Language | undefined => {
  if (typeof value !== "string") return undefined;
  const normalized = value.toLowerCase().replace(/_/g, "-");
  if (isLanguage(normalized)) return normalized;
  if (normalized.startsWith("zh")) return "zh";
  if (normalized === "ja" || normalized === "tr") return "en";
  return undefined;
};

const getInitialLanguage = (): Language => {
  if (typeof window !== "undefined") {
    try {
      const stored = window.localStorage.getItem("language");
      const legacy = normalizeLegacyLanguage(stored);
      if (legacy) {
        return legacy;
      }
    } catch (error) {
      console.warn("[i18n] Failed to read stored language preference", error);
    }
  }

  const navigatorLang =
    typeof navigator !== "undefined"
      ? (navigator.language?.toLowerCase() ??
        navigator.languages?.[0]?.toLowerCase())
      : undefined;

  if (navigatorLang?.startsWith("zh")) {
    return "zh";
  }

  if (navigatorLang?.startsWith("en")) {
    return "en";
  }

  // 认得出系统语言、但界面没有这种语言（德、法、韩…）时用英文：英文大多数人能读，简中几乎不能
  return navigatorLang ? "en" : DEFAULT_LANGUAGE;
};

const resources = {
  en: {
    translation: en,
  },
  zh: {
    translation: zh,
  },
};

i18n.use(initReactI18next).init({
  resources,
  lng: getInitialLanguage(), // 根据本地存储或系统语言选择默认语言
  fallbackLng: "en", // 如果缺少中文翻译则退回英文

  interpolation: {
    escapeValue: false, // React 已经默认转义
  },

  // 开发模式下显示调试信息
  debug: false,
});

/**
 * 用后端 settings.json 里保存的语言校正界面语言。
 *
 * 首屏语言取自 localStorage（同步可读），但它只是缓存：开发版与正式版的
 * WebView 来源不同、WebView 数据被清空时都会读不到或读到旧值，界面就落到
 * 系统语言。启动后拿到后端设置再对一次，不一致时切换并回写缓存。
 */
export const syncLanguageFromSettings = (language?: string | null): void => {
  const normalized = normalizeLegacyLanguage(language);
  if (!normalized) return;

  if (i18n.language !== normalized) {
    void i18n.changeLanguage(normalized);
  }

  try {
    if (window.localStorage.getItem("language") !== normalized) {
      window.localStorage.setItem("language", normalized);
    }
  } catch (error) {
    console.warn("[i18n] Failed to persist language preference", error);
  }
};

export default i18n;
