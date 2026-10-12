---
name: easyeda-engineering
description: 使用 FantasyLCSC CLI 做国内立创元器件选型和 STEP 下载，通过在线 Gateway 或专业版本地 CLI 读取、编辑嘉立创 EDA 工程，并沿用本用户的原理图与 PCB 设计习惯。适用于选型、接入诊断、原理图/PCB 创建整理和审阅。
---

# 立创 EDA 选型、接入与设计

根据用户指定的工程和工作模式操作。当前功能、电气、尺寸和制造约束优先于个人风格；接入诊断与选型本身不授权修改工程。

## 按任务选择资料与工具

| 任务 | 使用方式 |
| --- | --- |
| 国内商城选型、完整参数、规格书、库存价格、3D 模型 | 固定安装的 FantasyLCSC CLI；读 [component-selection.md](references/component-selection.md) |
| 读取或编辑全在线工程，与用户桌面编辑器互通 | Run API Gateway → 本机 Bridge → `eda` API；读 [gateway.md](references/gateway.md) |
| 后台分析、渲染或编辑本地副本 | 专业版桌面 CLI 的 headless 会话；读 [cli.md](references/cli.md) |
| 创建、整理或审阅原理图与 PCB | 操作前读 [personal-design.md](references/personal-design.md)，需要依据时再读 [design-evidence.md](references/design-evidence.md) |
| 定位客户端、Bridge 和安装版本的官方签名 | 读 [local-environment.md](references/local-environment.md) |

FantasyLCSC CLI 与专业版桌面 CLI 是两个工具：前者查询元件与模型，后者操作本地 EDA 工程。Gateway 使用 EDA 内用户已登录的会话；本地 CLI 不会自动连接手动打开的全在线窗口。Bridge 的存在不等于已配置 MCP。

## 元器件选型

固定程序：`D:\Agent\LocalManager\FantasyLCSC\fantasy-lcsc.exe`。优先调用此 Rust CLI，不使用旧 Node/Python 实验入口替代。先执行 `--version`、所需子命令 `--help` 确认当前能力；安装与更新见 component-selection.md。

按电路的额定值、裕量、封装、制造方式和数量确定筛选条件；用 `catalogs`、`filters` 获取当前类目及参数原始值，再用 `search` 组合筛选，最后用 `detail` 核对候选的完整参数、规格书和采购信息。以 C 编号关联 `info` 与 `download` 获取 EDA 元件/封装信息及原始 STEP。不要把搜索摘要当成完整参数，也不要把有 3D 模型当成电气适用证明。

结果保留来源、获取时间、币种和不同库存口径。只报告网站实际公布的信息；缺失或未验证的参数如实列出。选型结果给出 C 编号、型号、关键适用依据和需要进一步核实的条件。

## 工程接入约束

- 复用现有 Bridge，先查 `/health`、`/eda-windows`，再读取窗口版本、在线状态、工程 UUID 和文档树。连接成功与目标正确分别核对。
- 多窗口显式传 `windowId`，重连后重新发现；不把 Bridge 的 `active` 当成用户前台或任务目标。只有一个窗口时也检查工程身份。
- 用户要求另开窗口时使用独立窗口或本地副本。`openProject()` 会切换执行窗口，可能影响未保存工作；沿用已有编辑授权，不额外增加确认步骤。
- 修改前在代码中核对工程 UUID、文档 UUID 和模式。在线编辑或保存可能同步云端；只读审阅优先导出，视图/图层操作也影响对应窗口。
- 使用安装版本的官方 API 签名，在 `eda` 环境运行；异步操作 `await`，结果显式 `return`。分批检查实际文档、连接和图像，不只依据返回成功。
- Bridge 超时不会取消 EDA 内代码。写入超时先查实际状态，确认是否已生效或仍执行；状态不明时停止重复写入。
- 导入可能重建 UUID 或漏页；核对板、原理图页、PCB 数量与身份，缺失页回到在线只读导出核查。

## 设计与完成条件

完整个人规则在 personal-design.md 中，创建或修改相关设计时应用：中文功能分区、模块内部直连、原生图纸明细栏；GND 优先铺铜直连、就近接地过孔、可读丝印、圆角走线与板框、按电流和回路选择线宽。

原理图导入 PCB 前清除并重新分配位号、检查映射并通过原理图 DRC。PCB 整理完成后按个人规则依次生成泪滴、重新铺铜、补 GND 缝合孔、合并重复过孔、更新受影响泪滴与铺铜，最后实际运行并通过 PCB DRC。两项 DRC 的最终结果都确认通过，才报告整套设计完成；无法执行或确认时说明验证缺口，不用关闭规则或忽略报错替代通过。

整理现有设计时保留器件、引脚与网络关系；重画连线核对网表，检查整页、关键局部与 PCB 双面，不扩大用户指定范围。

## 接入辅助脚本

脚本以本 skill 实际安装目录为准，需要 Node.js 18+：

```powershell
node '<skill-dir>\scripts\bridge-client.cjs' status
node '<skill-dir>\scripts\bridge-client.cjs' execute '<skill-dir>\scripts\inspect-eda.js' '<windowId>'
$env:EASYEDA_EXE = '<专业版安装路径>\lceda-pro.exe'
node '<skill-dir>\scripts\eda-cli.cjs' doctor
```

Bridge helper 只查询状态和提交代码，不安装或重启服务；桌面 CLI helper 的 `--code-file` 转成原生 `--code`。较大结果重定向到任务目录，避免输出整份源码或图像 base64。
