<div align="center">

# BetterCCs

**cc-switch 的精简版 —— 只做「供应商切换」这一件事，把它做得更快、更清爽。**

</div>

---

## 这是什么

[cc-switch](https://github.com/farion1231/cc-switch) 是一个功能非常全的 AI 编码工具配置管理器，同时也长出了路由、聚合、代理这类「平台化」的能力。

BetterCCs 是它的一条精简分支：

| | cc-switch | BetterCCs |
|---|---|---|
| 供应商切换 | ✅ | ✅ |
| MCP / 提示词 / 技能管理 | ✅ | ✅ |
| 路由（协议转换） | ✅ | ❌ 不提供 |
| 聚合（高级路由） | ✅ | ❌ 不提供 |
| 前端表现 | 功能优先 | 重构，信息密度与响应速度优先 |
| 本地配置 | `~/.cc-switch` | **完全复用同一份配置** |

> 去掉的是「把 CC Switch 当成一个网关」的那部分；留下的是「把 CC Switch 当成一个切换器」的那部分。

## 和 cc-switch 平滑共存

BetterCCs 读写的是 `~/.cc-switch/` 下的同一套文件（`config.json`、`cc-switch.db`），不新建、不迁移、不改格式：

- 已经在用 cc-switch 的，直接启动就能看到全部供应商与设置；
- 两个应用不要同时运行，避免同时写同一份配置文件；
- 想回退，直接开回 cc-switch 即可，数据不用来回倒。

## 支持平台

| 平台 | 状态 |
|---|---|
| Windows 10+（x64 / ARM64） | 主目标平台 |
| macOS 12+（Universal） | 次要目标，随版本提供 |
| Linux | 不支持 —— 本分支不提供 Linux 构建与安装包 |

## 主要能力

- **一键切换供应商** —— Claude Code / Codex / Gemini CLI 等，切完即写回各工具自己的配置文件，不用手改 JSON / TOML / YAML
- **供应商管理** —— 预设模板、自定义图标、排序复制、用量查询
- **MCP 服务器** —— 一处维护，同步到多个工具
- **提示词与技能** —— 集中存放、按需启用
- **Deep Link** —— 用链接一键导入供应商 / MCP / 提示词
- **托盘常驻** —— 不占屏幕，随时切

![主界面](assets/screenshots/main-zh.png)

完整中文说明见 [用户手册](docs/user-manual/zh/README.md)。

## 开发

环境：Node 20+、pnpm、Rust（版本见 `rust-toolchain.toml`）、Tauri 2 的系统依赖。

```bash
pnpm install
pnpm tauri dev      # 开发（前端 + Rust）
pnpm typecheck      # TypeScript 类型检查
pnpm test:unit      # 单元测试
pnpm tauri build    # 打包
```

## 致谢与许可

上游项目：[farion1231/cc-switch](https://github.com/farion1231/cc-switch)，MIT License。

本仓库同样以 MIT License 发布，见 [LICENSE](LICENSE)。
