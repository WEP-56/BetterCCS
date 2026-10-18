import { afterEach, describe, expect, it } from "vitest";
import i18n, { syncLanguageFromSettings } from "@/i18n";

describe("syncLanguageFromSettings", () => {
  afterEach(async () => {
    localStorage.removeItem("language");
    await i18n.changeLanguage("en");
  });

  it("switches to the saved language and refills the cache when localStorage is empty", async () => {
    await i18n.changeLanguage("en");
    localStorage.removeItem("language");

    syncLanguageFromSettings("zh");

    expect(i18n.language).toBe("zh");
    expect(localStorage.getItem("language")).toBe("zh");
  });

  it("overrides a stale cached language", async () => {
    await i18n.changeLanguage("en");
    localStorage.setItem("language", "en");

    syncLanguageFromSettings("zh");

    expect(i18n.language).toBe("zh");
    expect(localStorage.getItem("language")).toBe("zh");
  });

  it("reads languages that no longer exist as the closest one we still ship", async () => {
    // 旧版上游可选的繁中 / 日文 / 土耳其文：繁中读简中，另两个读英文
    await i18n.changeLanguage("en");
    syncLanguageFromSettings("zh-TW");
    expect(i18n.language).toBe("zh");
    expect(localStorage.getItem("language")).toBe("zh");

    syncLanguageFromSettings("ja");
    expect(i18n.language).toBe("en");
    expect(localStorage.getItem("language")).toBe("en");

    syncLanguageFromSettings("tr");
    expect(i18n.language).toBe("en");
    expect(localStorage.getItem("language")).toBe("en");
  });

  it("leaves the current language alone when nothing is saved", async () => {
    await i18n.changeLanguage("en");

    syncLanguageFromSettings(undefined);
    syncLanguageFromSettings("fr");

    expect(i18n.language).toBe("en");
    expect(localStorage.getItem("language")).toBeNull();
  });
});
