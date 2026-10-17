import { describe, expect, it, vi } from "vitest";
import type { TFunction } from "i18next";
import type { Provider } from "@/types";
import {
  buildAdditiveSections,
  buildSwitchSections,
  type ProviderSection,
  type SwitchModeInput,
} from "@/components/providers/presentation";

const t = ((key: string, options?: Record<string, unknown>) =>
  options && "count" in options
    ? `${key}:${String(options.count)}`
    : key) as unknown as TFunction;

const provider = (id: string, overrides: Partial<Provider> = {}): Provider => ({
  id,
  name: id,
  settingsConfig: {},
  ...overrides,
});

const official = provider("official", { category: "official" });
const relay = provider("relay");
const backup = provider("backup");
/** Claude 下是 OpenAI 格式：客户端直连发不出去 */
const converted = provider("converted", { meta: { apiFormat: "openai_chat" } });

function build(overrides: Partial<SwitchModeInput> = {}) {
  const input: SwitchModeInput = {
    app: "claude",
    t,
    providers: [official, relay, backup, converted],
    currentId: "relay",
    unsupportedReason: () => "reason",
    actions: { switchTo: vi.fn() },
    ...overrides,
  };
  return { sections: buildSwitchSections(input), input };
}

const item = (sections: ProviderSection[], id: string) => {
  for (const section of sections) {
    const found = section.items.find((entry) => entry.provider.id === id);
    if (found) return { section: section.key, ...found.presentation };
  }
  throw new Error(`没有 ${id}`);
};

const button = (sections: ProviderSection[], id: string, key: string) => {
  const found = item(sections, id).buttons.find((b) => b.key === key);
  if (!found) throw new Error(`no ${key} button on ${id}`);
  return found;
};

describe("buildSwitchSections", () => {
  it("puts every provider in one list: the current one is in use, the rest offer a switch", () => {
    const { sections, input } = build();

    expect(sections).toHaveLength(1);
    expect(sections[0].key).toBe("all");
    expect(sections[0].items).toHaveLength(4);

    const current = item(sections, "relay");
    expect(current.tone).toBe("direct");
    expect(current.status).toEqual({
      label: "providerCard.status.inUse",
      dot: "direct",
    });
    expect(current.buttons).toHaveLength(0);
    expect(current.deleteDisabledReason).toBe(
      "providerCard.reason.inUseCannotDelete",
    );

    const other = item(sections, "backup");
    expect(other.buttons).toHaveLength(1);
    expect(other.buttons[0].key).toBe("switch");
    expect(other.buttons[0].label).toBe("providerCard.action.switch");
    expect(other.buttons[0].disabledReason).toBeUndefined();

    other.buttons[0].onClick();
    expect(input.actions.switchTo).toHaveBeenCalledWith(backup);
  });

  it("keeps providers whose format the client cannot speak, greyed out with the reason", () => {
    const { sections } = build();

    const convertedItem = item(sections, "converted");
    expect(convertedItem.dim).toBe(true);
    expect(convertedItem.chips.map((chip) => chip.key)).toEqual(["format"]);
    expect(convertedItem.chips[0].tone).toBe("warning");
    expect(convertedItem.chips[0].title).toBe("reason");
    expect(convertedItem.buttons[0].disabledReason).toBe("reason");
  });

  it("marks official cards with the official chip only", () => {
    const { sections } = build();

    const officialItem = item(sections, "official");
    expect(officialItem.chips.map((chip) => chip.key)).toEqual(["official"]);
    expect(officialItem.dim).toBeUndefined();
  });

  it("counts no current provider as everything being switchable", () => {
    const { sections } = build({ currentId: null });

    expect(sections[0].items.every((entry) => !entry.presentation.status)).toBe(
      true,
    );
  });
});

describe("buildAdditiveSections", () => {
  const additiveActions = () => ({
    add: vi.fn(),
    remove: vi.fn(),
    disableOmo: vi.fn(),
    setDefault: vi.fn(),
  });

  it("splits added and available providers and keeps OMO to one at a time", () => {
    const omo = provider("omo", { category: "omo" });
    const otherOmo = provider("omo-2", { category: "omo" });
    const actions = additiveActions();
    const sections = buildAdditiveSections({
      app: "opencode",
      t,
      providers: [relay, backup, omo, otherOmo],
      isInConfig: (p) => p.id === "relay",
      currentOmoId: "omo",
      actions,
    });

    expect(sections.map((s) => [s.key, s.items.length])).toEqual([
      ["added", 2],
      ["available", 2],
    ]);
    button(sections, "relay", "remove").onClick();
    expect(actions.remove).toHaveBeenCalledWith(relay);
    // 已添加的卡白底、不再写「● 已添加 / 已启用」：灰底只给默认那一家
    for (const id of ["relay", "omo"]) {
      expect(item(sections, id).tone).toBeUndefined();
      expect(item(sections, id).status).toBeUndefined();
    }
    button(sections, "backup", "add").onClick();
    expect(actions.add).toHaveBeenCalledWith(backup);
    button(sections, "omo", "disable").onClick();
    expect(actions.disableOmo).toHaveBeenCalledWith(omo);
    expect(item(sections, "omo-2").buttons.map((b) => b.key)).toEqual([
      "enable",
    ]);
  });

  it("lets OpenClaw pick the default model and protects the current default", () => {
    const multi = provider("multi", {
      settingsConfig: {
        models: [{ id: "m-1", name: "Model 1" }, { id: "m-2" }],
      },
    });
    const actions = additiveActions();
    const sections = buildAdditiveSections({
      app: "openclaw",
      t,
      providers: [relay, multi],
      isInConfig: () => true,
      openclawDefault: { providerId: "relay", model: "gpt" },
      openclawModels: (p) =>
        (p.settingsConfig as { models?: { id: string; name?: string }[] })
          .models ?? [],
      actions,
    });

    expect(item(sections, "relay")).toMatchObject({
      tone: "neutral",
      status: { label: "providerCard.status.default" },
    });
    expect(item(sections, "multi").tone).toBeUndefined();
    expect(item(sections, "multi").status).toBeUndefined();
    expect(button(sections, "relay", "remove").disabledReason).toBe(
      "providerCard.reason.defaultCannotRemove",
    );
    const setDefault = button(sections, "multi", "setDefault");
    expect(setDefault.menu?.options.map((o) => [o.label, o.detail])).toEqual([
      ["Model 1", "m-1"],
      ["m-2", undefined],
    ]);
    setDefault.menu?.options[1].onSelect();
    expect(actions.setDefault).toHaveBeenCalledWith(multi, "m-2");
  });

  it("keeps Hermes-managed providers read-only", () => {
    const managed = provider("managed");
    const sections = buildAdditiveSections({
      app: "hermes",
      t,
      providers: [relay, managed],
      isInConfig: () => true,
      hermesCurrentId: "relay",
      isHermesManaged: (p) => p.id === "managed",
      actions: additiveActions(),
    });

    expect(button(sections, "relay", "remove").disabledReason).toBe(
      "providerCard.reason.currentCannotRemove",
    );
    expect(item(sections, "relay").tone).toBe("neutral");
    expect(item(sections, "managed").tone).toBeUndefined();
    expect(item(sections, "managed")).toMatchObject({
      dim: true,
      editDisabledReason: "provider.managedByHermesHint",
      deleteDisabledReason: "provider.managedByHermesHint",
    });
    expect(button(sections, "managed", "use").disabledReason).toBe(
      "provider.managedByHermesHint",
    );
  });

  it("blocks every Pi change while Pi's current state is unavailable", () => {
    const sections = buildAdditiveSections({
      app: "pi",
      t,
      providers: [relay],
      isInConfig: () => false,
      piStateUnavailable: true,
      actions: additiveActions(),
    });

    expect(button(sections, "relay", "add")).toMatchObject({
      label: "providerCard.action.enable",
      disabledReason: "pi.current.stateUnavailableHint",
    });
    expect(item(sections, "relay").deleteDisabledReason).toBe(
      "pi.current.stateUnavailableHint",
    );
  });
});

