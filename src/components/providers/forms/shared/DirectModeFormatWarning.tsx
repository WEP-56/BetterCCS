import { useTranslation } from "react-i18next";

/**
 * 上游格式要靠协议转换、而本地路由已经下线时，格式下面的后果提示（Claude Code、Codex 共用）。
 *
 * 没有路由 = 没有地方做转换：这种格式保存下去也发不出请求，客户端会直接报错。
 */
export function DirectModeFormatWarning() {
  const { t } = useTranslation();
  return (
    <p className="text-xs leading-relaxed text-warning-text">
      {t("providerForm.formatUnsupportedWarning", {
        defaultValue:
          "Agent 客户端只认自己的协议格式，选这种格式直连发不出请求。保存后这家用不了，请改用客户端支持的格式。",
      })}
    </p>
  );
}
