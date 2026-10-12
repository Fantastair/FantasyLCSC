# 本机验证记录与依赖定位

以下路径与结果在 2026-10-09 的 Windows 环境中验证。换机器、更新版本后先确认路径和能力，不把它们作为安装约定。

| 资源 | 已验证位置或结果 |
| --- | --- |
| 专业版客户端 | `D:\Program Files\lceda-pro\lceda-pro.exe`，4.1.60 |
| 工作目录 | `D:\Agent\LocalManager\eda-agent` |
| 官方 API 包 | `gateway\official`，包版本 1.0.3，含 SKILL.md、references、scripts 和 package.json |
| Bridge 启动工具 | `gateway\start-bridge.ps1`；探测并复用现有服务，后台启动隐藏窗口 |
| Bridge 日志 | `gateway\bridge.stdout.log`、`gateway\bridge.stderr.log` |
| Bridge 进程记录 | `gateway\bridge-process.json`；PID 仅作线索，使用前验证进程身份 |
| 本机 Bridge 配置 | `official\scripts\bridge-server.mjs` 改为只绑定 `127.0.0.1`；协议未改 |
| 官方接口参考 | `gateway\official\references\classes\<类名>.md` |
| 客户端类型定义 | `D:\Program Files\lceda-pro\resources\app\assets\pro-api\0.3.38.25cc6d44\api-types.d.ts`；更新后目录版本可能变化 |
| Node | PATH 中 node.exe，或 Codex 工作区依赖运行时；工具需要 Node 18+ |
| 本地测试模板 | `D:\Agent\LocalManager\eda-agent\cli-probe.eprj2`；仅复制后用于隔离测试 |

先用 `rg --files` 定位类参考与 `api-types.d.ts`，再按方法名检索签名，避免读取整个大型编译脚本。API 包不存在时可从官方地址 `https://image.lceda.cn/files/easyeda-api.zip` 获取并检查当前包说明。本技能没有内置完整官方 API 包或 ws 依赖。

本机已完成官方包依赖安装；新环境需在可信官方包目录按 package.json/锁文件安装依赖，再运行 Bridge。启动前查看监听地址，保留本机环回绑定。需要重建服务时参考现有启动工具，不为一次任务添加系统自启动或重启正常服务。

已验证：全在线桌面客户端可通过 Gateway 读取工程、导出源码与图像；独立新窗口可单独连接；CLI 可以打开本地 headless 副本。4.1.60 的 CLI 返回 `MCP_NOT_SUPPORTED`；这是当时版本的实测结果，其他版本需重跑 help/doctor 确认。
