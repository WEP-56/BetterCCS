import type { TFunction } from "i18next";
import type { Provider } from "@/types";
import type { AppId } from "@/lib/api";
import {
  isOfficialAccount,
  providerFormatUnsupported,
} from "@/utils/providerCapabilities";

/**
 * 供应商卡片怎么画（v7）：当前那张的模式色、状态文字或按钮、徽标。由列表按应用算好交给
 * 卡片，卡片本身不再判断模式。
 */
export type CardTone = "direct" | "neutral";

export type ChipTone = "outline" | "direct" | "success" | "warning" | "danger";

export interface CardChip {
  key: string;
  label: string;
  tone: ChipTone;
  title?: string;
}

export interface CardMenuOption {
  key: string;
  label: string;
  detail?: string;
  onSelect: () => void;
}

/** 「更多」菜单里跟当前模式有关的一项。 */
export interface CardMenuItem {
  key: string;
  label: string;
  /** 不能点的原因：照常列出来，原因写在名字下面 */
  disabledReason?: string;
  onSelect: () => void;
}

export interface CardButton {
  key: string;
  label: string;
  onClick: () => void;
  /** 不能点的原因：按钮照常画出来，原因挂在按钮的说明卡上 */
  disabledReason?: string;
  /** 点开选一项（OpenClaw 的「设为默认 ▾」选默认模型） */
  menu?: { title: string; options: CardMenuOption[] };
}

export interface CardPresentation {
  /** 当前那张：模式色边框 + 淡底；neutral（灰底）只给共存式应用的默认那家（OpenClaw 默认、Hermes 当前） */
  tone?: CardTone;
  /** 主操作位换成状态文字（使用中 / 当前默认 / 已添加…） */
  status?: { label: string; dot: CardTone | "muted" };
  buttons: CardButton[];
  /** 「更多」菜单最前面、跟当前模式有关的操作 */
  menuItems?: CardMenuItem[];
  chips: CardChip[];
  /** 整卡淡一些（格式不支持、Hermes 托管） */
  dim?: boolean;
  /** 编辑 / 删除不可用的原因（当前那家不能删、Hermes 托管只能在 Web UI 改…） */
  editDisabledReason?: string;
  deleteDisabledReason?: string;
}

export interface ProviderSection {
  key: string;
  title?: string;
  help?: { title: string; body: string };
  emptyText?: string;
  items: { provider: Provider; presentation: CardPresentation }[];
}

// ─── 切换式应用（Claude Code / Codex / Gemini CLI / Grok Build）──────────────

export interface SwitchModeInput {
  app: AppId;
  t: TFunction;
  providers: Provider[];
  /** 当前正在用的那家（直连指针） */
  currentId: string | null;
  /** 这种格式 Agent 客户端直连用不了的原因 */
  unsupportedReason: (provider: Provider) => string;
  actions: {
    switchTo: (provider: Provider) => void;
  };
}

/**
 * 切换式应用的供应商列表：一条直连路径。
 *
 * 客户端只认它自己的协议格式，所以格式不兼容的供应商切过去也用不了——照常列出来并置灰，
 * 原因写在按钮的说明卡和卡片徽标上，用户能在表单里改回来。
 */
export function buildSwitchSections(input: SwitchModeInput): ProviderSection[] {
  const { app, t, providers, currentId, unsupportedReason, actions } = input;
  const official = (p: Provider) => isOfficialAccount(app, p);

  const chipsOf = (p: Provider): CardChip[] => {
    const chips: CardChip[] = [];
    if (official(p)) {
      chips.push({
        key: "official",
        label: t("providerCard.chip.official"),
        tone: "outline",
      });
    }
    if (providerFormatUnsupported(app, p)) {
      chips.push({
        key: "format",
        label: t("providerCard.chip.formatUnsupported"),
        tone: "warning",
        title: unsupportedReason(p),
      });
    }
    return chips;
  };

  return [
    {
      key: "all",
      items: providers.map((p) => {
        const chips = chipsOf(p);
        if (p.id === currentId) {
          return {
            provider: p,
            presentation: {
              tone: "direct" as const,
              status: {
                label: t("providerCard.status.inUse"),
                dot: "direct" as const,
              },
              chips,
              buttons: [],
              deleteDisabledReason: t("providerCard.reason.inUseCannotDelete"),
            },
          };
        }
        const unsupported = providerFormatUnsupported(app, p);
        return {
          provider: p,
          presentation: {
            chips,
            dim: unsupported,
            buttons: [
              {
                key: "switch",
                label: t("providerCard.action.switch"),
                onClick: () => actions.switchTo(p),
                disabledReason: unsupported ? unsupportedReason(p) : undefined,
              },
            ],
          },
        };
      }),
    },
  ];
}

// ─── 共存式应用（OpenCode / OpenClaw / Hermes / Pi / MiniMax Code）───────────

export interface AdditiveInput {
  app: AppId;
  t: TFunction;
  providers: Provider[];
  isInConfig: (provider: Provider) => boolean;
  /** OpenCode：当前启用的 OMO / OMO Slim */
  currentOmoId?: string | null;
  currentOmoSlimId?: string | null;
  /** OpenClaw：默认模型属于哪家、它的模型名 */
  openclawDefault?: { providerId: string; model: string } | null;
  /** OpenClaw：每家可选的模型 */
  openclawModels?: (provider: Provider) => { id: string; name?: string }[];
  /** Hermes：model.provider 指向的那家 */
  hermesCurrentId?: string | null;
  isHermesManaged?: (provider: Provider) => boolean;
  /** Pi：读不到当前配置时一律不能改 */
  piStateUnavailable?: boolean;
  actions: {
    add: (provider: Provider) => void;
    remove: (provider: Provider) => void;
    disableOmo: (provider: Provider) => void;
    setDefault: (provider: Provider, modelId?: string) => void;
  };
}

export function buildAdditiveSections(input: AdditiveInput): ProviderSection[] {
  const { app, t, providers, actions } = input;
  const added: ProviderSection["items"] = [];
  const available: ProviderSection["items"] = [];
  const piBlocked =
    app === "pi" && input.piStateUnavailable
      ? t("pi.current.stateUnavailableHint")
      : undefined;

  for (const p of providers) {
    const chips: CardChip[] = [];
    const isOmo = p.category === "omo";
    const isOmoSlim = p.category === "omo-slim";
    if (isOmo) chips.push({ key: "omo", label: "OMO", tone: "outline" });
    if (isOmoSlim) chips.push({ key: "slim", label: "Slim", tone: "outline" });

    // OMO / Slim：插件配置，同一时间只启用一个
    if (isOmo || isOmoSlim) {
      const enabled = isOmo
        ? p.id === input.currentOmoId
        : p.id === input.currentOmoSlimId;
      if (enabled) {
        // 分组标题已经写了「已添加」，按钮是「停用」，卡上不再重复写状态
        added.push({
          provider: p,
          presentation: {
            chips,
            buttons: [
              {
                key: "disable",
                label: t("providerCard.action.disable"),
                onClick: () => actions.disableOmo(p),
              },
            ],
          },
        });
      } else {
        available.push({
          provider: p,
          presentation: {
            chips,
            buttons: [
              {
                key: "enable",
                label: t("providerCard.action.enable"),
                onClick: () => actions.add(p),
              },
            ],
          },
        });
      }
      continue;
    }

    const managed = app === "hermes" && (input.isHermesManaged?.(p) ?? false);
    if (managed) {
      chips.push({
        key: "managed",
        label: t("providerCard.chip.hermesManaged"),
        tone: "outline",
        title: t("provider.managedByHermesHint"),
      });
    }
    const readOnlyReason = managed
      ? t("provider.managedByHermesHint")
      : undefined;

    if (!input.isInConfig(p)) {
      available.push({
        provider: p,
        presentation: {
          chips,
          dim: managed,
          editDisabledReason: readOnlyReason,
          deleteDisabledReason: readOnlyReason ?? piBlocked,
          buttons: [
            {
              key: "add",
              label:
                app === "pi"
                  ? t("providerCard.action.enable")
                  : t("providerCard.action.add"),
              onClick: () => actions.add(p),
              disabledReason: piBlocked,
            },
          ],
        },
      });
      continue;
    }

    // 已添加：白底、不写「● 已添加」（分组标题和「移除」已经说清楚）；灰底只给默认那一家
    const buttons: CardButton[] = [];
    let status: CardPresentation["status"];
    let isDefault = false;
    let removeReason: string | undefined = piBlocked;

    if (app === "openclaw") {
      isDefault = input.openclawDefault?.providerId === p.id;
      if (isDefault) {
        status = { label: t("providerCard.status.default"), dot: "muted" };
        chips.push({
          key: "defaultModel",
          label: t("providerCard.chip.defaultModel", {
            model: input.openclawDefault?.model ?? "",
          }),
          tone: "outline",
        });
        removeReason = t("providerCard.reason.defaultCannotRemove");
      } else {
        const models = input.openclawModels?.(p) ?? [];
        buttons.push({
          key: "setDefault",
          label: t("providerCard.action.setDefault"),
          onClick: () => actions.setDefault(p, models[0]?.id),
          menu:
            models.length > 1
              ? {
                  title: t("openclaw.selectDefaultModel"),
                  options: models.map((model) => ({
                    key: model.id,
                    label: model.name?.trim() || model.id,
                    detail:
                      model.name?.trim() && model.name.trim() !== model.id
                        ? model.id
                        : undefined,
                    onSelect: () => actions.setDefault(p, model.id),
                  })),
                }
              : undefined,
        });
      }
    }

    if (app === "hermes") {
      if (p.id === input.hermesCurrentId) {
        isDefault = true;
        status = { label: t("providerCard.status.current"), dot: "muted" };
        removeReason = t("providerCard.reason.currentCannotRemove");
      } else {
        buttons.push({
          key: "use",
          label: t("providerCard.action.enable"),
          onClick: () => actions.setDefault(p),
          disabledReason: readOnlyReason,
        });
      }
    }

    buttons.push({
      key: "remove",
      label: t("providerCard.action.remove"),
      onClick: () => actions.remove(p),
      disabledReason: removeReason ?? readOnlyReason,
    });

    added.push({
      provider: p,
      presentation: {
        tone: isDefault ? "neutral" : undefined,
        status,
        chips,
        dim: managed,
        editDisabledReason: readOnlyReason,
        deleteDisabledReason: readOnlyReason ?? piBlocked,
        buttons,
      },
    });
  }

  return [
    {
      key: "added",
      title: t("providerCard.section.added", { count: added.length }),
      help: {
        title: t("providerCard.section.addedHelpTitle"),
        body: t("providerCard.section.addedHelp", {
          app: t(`apps.${app}`),
        }),
      },
      emptyText: t("providerCard.section.addedEmpty"),
      items: added,
    },
    {
      key: "available",
      title: t("providerCard.section.available", { count: available.length }),
      items: available,
    },
  ];
}
