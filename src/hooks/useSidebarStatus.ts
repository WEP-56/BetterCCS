import { useUsageSummary } from "@/lib/query/usage";
import { parseFiniteNumber } from "@/components/usage/format";
import type { UsageRangeSelection } from "@/types/usage";

const TODAY: UsageRangeSelection = { preset: "today" };

/**
 * 侧栏底部的今日花费与 Token。
 */
export function useSidebarStatus() {
  const { data: todaySummary } = useUsageSummary(TODAY, undefined, {
    refetchInterval: 60_000,
  });

  const todayCost = parseFiniteNumber(todaySummary?.totalCost) ?? 0;
  // 和用量页「真实消耗 Tokens」同口径
  const todayTokens = todaySummary?.realTotalTokens ?? 0;
  const todayRequests = todaySummary?.totalRequests ?? 0;
  const todayHitRate = todaySummary?.cacheHitRate ?? 0;

  return { todayCost, todayTokens, todayRequests, todayHitRate };
}
