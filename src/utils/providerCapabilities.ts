import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import { isOAuthProviderType } from "@/config/constants";
import { resolveManagedAccountId } from "@/lib/authBinding";
import {
  extractCodexBaseUrl,
  extractCodexExperimentalBearerToken,
  extractCodexWireApi,
  hasExplicitNonOpenAiCodexModelProvider,
  isCodexAnthropicWireApi,
  isCodexChatWireApi,
} from "@/utils/providerConfigUtils";

export const CODEX_OFFICIAL_PROVIDER_ID = "codex-official";
export const GROKBUILD_OFFICIAL_PROVIDER_ID = "grokbuild-official";

export type CodexOfficialIdentity =
  | "native_login"
  | "managed_account"
  | "api_key";

const nonEmptyString = (value: unknown): boolean =>
  typeof value === "string" && value.trim().length > 0;

function hasExplicitCodexThirdPartyUpstream(
  settings: Record<string, unknown>,
): boolean {
  const config = typeof settings.config === "string" ? settings.config : "";

  return (
    nonEmptyString(settings.baseUrl) ||
    nonEmptyString(settings.baseURL) ||
    nonEmptyString(settings.base_url) ||
    Boolean(extractCodexExperimentalBearerToken(config)) ||
    Boolean(extractCodexBaseUrl(config)) ||
    hasExplicitNonOpenAiCodexModelProvider(config)
  );
}

function hasStoredCodexApiKey(settings: Record<string, unknown>): boolean {
  const auth = settings.auth as Record<string, unknown> | undefined;
  return nonEmptyString(auth?.OPENAI_API_KEY);
}

export function resolveCodexOfficialIdentity(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): CodexOfficialIdentity | null {
  if (appId !== "codex") return null;

  const managedAccountId = resolveManagedAccountId(
    provider.meta,
    "codex_oauth",
  )?.trim();
  const hasFixedOfficialId = provider.id === CODEX_OFFICIAL_PROVIDER_ID;
  if (hasFixedOfficialId && provider.category === "official") {
    return managedAccountId ? "managed_account" : "native_login";
  }

  const settings = provider.settingsConfig as Record<string, unknown>;
  const auth = settings?.auth;
  const config = settings?.config;
  if (
    !auth ||
    typeof auth !== "object" ||
    Array.isArray(auth) ||
    (config != null && typeof config !== "string")
  ) {
    return null;
  }

  if (hasExplicitCodexThirdPartyUpstream(settings)) {
    return null;
  }

  if (managedAccountId) {
    return "managed_account";
  }
  if (hasStoredCodexApiKey(settings)) {
    return provider.category === "official" ? "api_key" : null;
  }
  return hasFixedOfficialId || provider.category === "official"
    ? "native_login"
    : null;
}

/**
 * 官方账号卡（对应后端 `codex_direct::is_official`）：Codex 早期绑定托管账号的官方卡
 * 没有 category，按身份认。
 */
export function isOfficialAccount(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): boolean {
  return (
    provider.category === "official" ||
    resolveCodexOfficialIdentity(appId, provider) !== null
  );
}

/**
 * 这家供应商的格式，Agent 客户端直连能不能用。
 *
 * 客户端只认自己的协议：Claude Code 只吃 Anthropic Messages，Codex 只吃 Responses，
 * Grok Build 同 Codex。需要换协议、需要逐请求注入令牌（Copilot / xAI OAuth）、
 * 需要模型映射（Claude Desktop 的 proxy 模式）的供应商，没有本地路由就用不了——
 * 界面照常列出来，但切换按钮置灰并说明原因。
 */
export function providerFormatUnsupported(
  appId: AppId,
  provider: Provider,
): boolean {
  if (isOfficialAccount(appId, provider)) return false;

  const isManagedOAuth = isOAuthProviderType(provider.meta?.providerType);

  // Desktop 普通供应商由表单模式决定；托管 OAuth 的 token 只能由代理注入。
  if (appId === "claude-desktop") {
    return isManagedOAuth || provider.meta?.claudeDesktopMode === "proxy";
  }

  if (appId !== "claude" && appId !== "codex" && appId !== "grokbuild") {
    return false;
  }

  // 托管 OAuth：凭据由代理注入，与 apiFormat 无关，必须接管。
  if (isManagedOAuth) return true;

  if (appId === "claude") {
    const fmt = provider.meta?.apiFormat;
    // Claude 原生是 Anthropic 格式，任何非 anthropic 格式都需要代理转换。
    return provider.meta?.isFullUrl === true || (!!fmt && fmt !== "anthropic");
  }

  if (appId === "codex" || appId === "grokbuild") {
    const fmt = provider.meta?.apiFormat;
    // Codex 原生是 Responses，仅 Chat / Anthropic 需要转换（Responses 直连）。
    if (
      provider.meta?.isFullUrl === true ||
      fmt === "openai_chat" ||
      fmt === "anthropic"
    )
      return true;
    const config = (provider.settingsConfig as Record<string, unknown>)?.config;
    return (
      typeof config === "string" &&
      (isCodexChatWireApi(extractCodexWireApi(config)) ||
        isCodexAnthropicWireApi(extractCodexWireApi(config)))
    );
  }

  return false;
}
