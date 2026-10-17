import { createContext, useContext, type ReactNode } from "react";
import { DRAG_REGION_ATTR, DRAG_REGION_STYLE } from "@/lib/platform";
import { cn } from "@/lib/utils";

/** 由外壳提供：Linux 自绘窗口按钮要接在每一屏页头的最右端。 */
export const WindowControlsContext = createContext<ReactNode>(null);

const NO_DRAG = { WebkitAppRegion: "no-drag" } as React.CSSProperties;

interface AppPageHeaderProps {
  /** 标题前的可操作元素（二级页的返回按钮）；和 icon 二选一 */
  leading?: ReactNode;
  icon?: ReactNode;
  /** 不传就只有图标 + 副标题（供应商页：加粗大字属于过度强调） */
  title?: ReactNode;
  /** 应用页用 16px 的名字 + 13px 的页面名；全局页和设置用 18px 的页面标题 */
  variant?: "app" | "page";
  /** 应用页的页面名（如「供应商」），灰字跟在应用名后面；没有 title 时它就是这一屏的标题 */
  subtitle?: ReactNode;
  /** 标题后面的「?」、Beta 徽标、计数等 */
  titleExtra?: ReactNode;
  /** 标题右侧的工具区（主操作、搜索）：贴着标题往左靠，不跟右边的 ⋯ 挤在一起 */
  toolbar?: ReactNode;
  /** 右侧操作区：主按钮放最后一个、⋯ 放它后面 */
  actions?: ReactNode;
  /** 让 actions 与窗口按钮挨在一起（⋯ 和 最小化/最大化/关闭 算一组） */
  groupActionsWithControls?: boolean;
  /** 标题可能很长（如会话阅读页的会话名）：标题占满剩余宽度，放不下就截断 */
  truncateTitle?: boolean;
  className?: string;
}

/**
 * 内容区页头：高 44（与侧栏顶条同高），整条可拖动窗口，左边是图标 + 标题，右边是操作。
 * 不画下边框：页头和内容靠留白分开，避免把窗口切成上下两块。
 */
export function AppPageHeader({
  leading,
  icon,
  title,
  variant = "page",
  subtitle,
  titleExtra,
  toolbar,
  actions,
  groupActionsWithControls = false,
  truncateTitle = false,
  className,
}: AppPageHeaderProps) {
  const windowControls = useContext(WindowControlsContext);
  const hasTitle = title != null && title !== "";
  return (
    <header
      className={cn(
        "flex h-11 shrink-0 items-center gap-5 pe-3 ps-5",
        className,
      )}
      {...DRAG_REGION_ATTR}
      style={DRAG_REGION_STYLE as React.CSSProperties}
    >
      {/* Tauri 只认按下的元素自身带拖动属性：标题区、标题、占位条都要带，整条才能拖窗口 */}
      <div
        {...DRAG_REGION_ATTR}
        className={cn(
          "flex min-w-0 items-center gap-2.5",
          truncateTitle ? "flex-1" : "shrink-0",
        )}
      >
        {leading && (
          <span className="-ms-2 flex shrink-0 items-center" style={NO_DRAG}>
            {leading}
          </span>
        )}
        {icon && (
          <span
            aria-hidden="true"
            className="pointer-events-none flex h-[22px] w-[22px] shrink-0 items-center justify-center text-fg-1"
          >
            {icon}
          </span>
        )}
        {hasTitle && (
          <h1
            {...DRAG_REGION_ATTR}
            className={cn(
              "m-0 whitespace-nowrap",
              variant === "app" ? "text-title" : "text-page",
              truncateTitle && "min-w-0 truncate",
            )}
          >
            {title}
          </h1>
        )}
        {(subtitle || titleExtra) && (
          <div
            className={cn("flex items-center gap-1", !hasTitle && "min-w-0")}
            style={NO_DRAG}
          >
            {subtitle &&
              (hasTitle ? (
                <span className="whitespace-nowrap text-body text-fg-3">
                  {subtitle}
                </span>
              ) : (
                // 没有大标题时，副标题就是这一屏的标题：13px 常规字重、第二级灰
                <h1
                  className={cn(
                    "m-0 whitespace-nowrap text-body font-medium text-fg-2",
                    truncateTitle && "min-w-0 truncate",
                  )}
                >
                  {subtitle}
                </h1>
              ))}
            {titleExtra}
          </div>
        )}
      </div>

      {toolbar && (
        <div
          className="flex min-w-0 shrink items-center justify-start gap-2"
          style={NO_DRAG}
        >
          {toolbar}
        </div>
      )}

      {!truncateTitle && (
        <div {...DRAG_REGION_ATTR} className="flex-1" />
      )}

      <div
        className={cn(
          "flex min-w-0 shrink-0 items-center justify-end",
          // 不合并时和以前一样留 20px，别的页面观感不变
          groupActionsWithControls && windowControls ? "gap-1" : "gap-5",
        )}
        style={NO_DRAG}
      >
        {actions && (
          <div className="flex min-w-0 shrink items-center justify-end gap-2">
            {actions}
          </div>
        )}
        {windowControls && (
          <div className="flex shrink-0 items-center gap-1">
            {windowControls}
          </div>
        )}
      </div>
    </header>
  );
}
