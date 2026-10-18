import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

let loadedI18n:
  | { changeLanguage: (language: string) => Promise<unknown> }
  | undefined;

beforeEach(() => {
  vi.resetModules();
  localStorage.clear();
});
afterEach(async () => {
  await loadedI18n?.changeLanguage("zh");
  loadedI18n = undefined;
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("first-launch language detection", () => {
  it.each(["zh", "zh-CN", "zh-SG", "zh-Hans-CN", "zh-TW", "zh-HK"])(
    "uses Simplified Chinese for %s",
    async (language) => {
      vi.stubGlobal("navigator", { language });
      const { default: i18n } = await import("@/i18n");
      loadedI18n = i18n;
      expect(i18n.language).toBe("zh");
    },
  );

  it.each(["en", "en-US", "ja", "ja-JP", "tr-TR", "de-DE", "fr", "ko-KR"])(
    "uses English for %s",
    async (language) => {
      vi.stubGlobal("navigator", { language });
      const { default: i18n } = await import("@/i18n");
      loadedI18n = i18n;
      expect(i18n.language).toBe("en");
    },
  );

  it("reads a language saved by an older version as the closest one we still ship", async () => {
    // 旧版上游存过中文繁体与日文、土耳其文：繁中读简中，另两个读英文
    localStorage.setItem("language", "zh-TW");
    vi.stubGlobal("navigator", { language: "en-US" });
    const zhTW = (await import("@/i18n")).default;
    loadedI18n = zhTW;
    expect(zhTW.language).toBe("zh");

    vi.resetModules();
    localStorage.setItem("language", "ja");
    const ja = (await import("@/i18n")).default;
    loadedI18n = ja;
    expect(ja.language).toBe("en");
  });

  it("keeps a saved language instead of overriding it with the system language", async () => {
    localStorage.setItem("language", "en");
    vi.stubGlobal("navigator", { language: "zh-CN" });
    const { default: i18n } = await import("@/i18n");
    loadedI18n = i18n;
    expect(i18n.language).toBe("en");
  });
});
