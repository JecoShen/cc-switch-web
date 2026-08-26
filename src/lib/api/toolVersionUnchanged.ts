import { compareVersions } from "@/lib/version";
import type {
  ToolInstallation,
  ToolInstallationReport,
} from "@/lib/api/settings";

/**
 * 升级后是否"真正没改版本"的判定 —— 必须看完整安装列表,不能只比
 * `tool.version` (它来自 `try_get_version`,会被 npm 残留旧版遮蔽)。
 *
 * 判定为"已升级"(返回 `false`,不要误报失败)的条件满足任一即返回:
 *   A. 任意一处可运行安装的版本严格高于 previousVersion
 *   B. 升级目标 (`is_path_default`) 那处的版本严格高于 previousVersion
 * 其它情况返回 `true` —— 升级确实没生效。
 *
 * previousVersion 为空(原本就没装)时直接返回 `true`,让前端走 notRunnable
 * 分支,而不是误报"版本没变"。
 */
export function isVersionStillUnchanged(
  previousVersion: string | null,
  installs: readonly ToolInstallation[],
): boolean {
  if (!previousVersion) return true;

  // A: 任意一处可运行安装 > previousVersion
  const higherAnywhere = installs.some(
    (i) =>
      i.runnable &&
      typeof i.version === "string" &&
      compareVersions(i.version, previousVersion) > 0,
  );
  if (higherAnywhere) return false;

  // B: path_default 那处(若可运行) > previousVersion
  const def = installs.find((i) => i.is_path_default);
  if (
    def?.runnable &&
    typeof def.version === "string" &&
    compareVersions(def.version, previousVersion) > 0
  ) {
    return false;
  }

  return true;
}

/**
 * 升级实际作用到的最具体那处的版本号。优先 `is_path_default` (命令行实际命中的),
 * 否则首个 runnable 有版本号的安装;都拿不到时返回 `null`。
 * 供 toast 文案展示用,避免把 npm 残留旧版误当作"升级后的版本"。
 */
export function pickedUpgradedVersion(
  installs: readonly ToolInstallation[],
): string | null {
  const def = installs.find((i) => i.is_path_default);
  if (def?.runnable && typeof def.version === "string") return def.version;
  const any = installs.find(
    (i) => i.runnable && typeof i.version === "string",
  );
  return any?.version ?? null;
}

/** 从 `ToolInstallationReport[]` 抽单工具报告。 */
export function pickReport(
  reports: readonly ToolInstallationReport[],
  toolName: string,
): ToolInstallationReport | undefined {
  return reports.find((r) => r.tool === toolName);
}
