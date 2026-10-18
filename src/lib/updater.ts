import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";

export type UpdateChannel = "stable" | "beta";

export interface UpdateInfo {
  currentVersion: string;
  availableVersion: string;
  notes?: string;
  pubDate?: string;
}

export interface CheckOptions {
  timeout?: number;
  channel?: UpdateChannel;
}

export async function getCurrentVersion(): Promise<string> {
  try {
    return await getVersion();
  } catch {
    return "";
  }
}

/**
 * 检查应用自身有没有新版本。
 *
 * 走后端命令查本仓库的 GitHub Releases，不用 Tauri 的更新插件：本仓库的产物不做
 * 代码签名（见 .github/workflows/release.yml），插件要求的 minisign 签名和
 * latest.json 都不存在；装新版本的方式是去 Releases 页下载覆盖安装。
 */
export async function checkForUpdate(): Promise<
  { status: "up-to-date" } | { status: "available"; info: UpdateInfo }
> {
  const currentVersion = await getCurrentVersion();
  const availableVersion = await invoke<string | null>(
    "check_app_update_available",
  );

  if (!availableVersion) {
    return { status: "up-to-date" };
  }

  return {
    status: "available",
    info: { currentVersion, availableVersion },
  };
}
