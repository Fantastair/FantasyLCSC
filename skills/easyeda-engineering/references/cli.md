# CLI 与本地隔离副本

适合已有本地工程、后台生成，以及云工程导出后的隔离分析。手动打开的全在线窗口不会自动成为 CLI session；线上互通使用 Gateway。

## 探测与调用

指定 `EASYEDA_EXE` 后，用 `eda-cli.cjs` 执行。Windows GUI 可执行文件输出由 Node spawnSync 捕获，隐藏启动窗口。

```powershell
$env:EASYEDA_EXE = 'D:\Program Files\lceda-pro\lceda-pro.exe' # 本机示例
node '<skill-dir>\scripts\eda-cli.cjs' doctor
node '<skill-dir>\scripts\eda-cli.cjs' --help
node '<skill-dir>\scripts\eda-cli.cjs' session list
node '<skill-dir>\scripts\eda-cli.cjs' open --path '<绝对路径>\review-copy.eprj2' --headless true
node '<skill-dir>\scripts\eda-cli.cjs' invoke --session '<sessionId>' --ext-uuid eda --code-file '<代码文件.js>'
node '<skill-dir>\scripts\eda-cli.cjs' session close --session '<任务创建的sessionId>' --destroy
```

输出有两层：外层 `status/error/stdout/stderr`；`stdout` 中为客户端 JSON。检查客户端 `ok`、`value`、`logs`，必要时记录 `durationMs`。helper 的 `--code-file` 转为原生 `--code`，不能直接传给客户端。非 JSON 输出保留诊断原文，不能视为成功。

本机曾在 agent 普通沙箱内调用 doctor 超时，而使用运行环境提供的提权执行后立即成功。遇到类似情况先区分沙箱跨进程限制与 EDA 连接故障；按运行环境的审批机制执行所需诊断，不据此重装客户端或修改系统权限。

4.1.60 的 headless open 需要本地 `--path`，`--mcp stdio` 返回 `MCP_NOT_SUPPORTED`；以后版本重新确认。helper 等待最多 70 秒；超时先检查会话和实际结果，不假设调用端终止就回滚了编辑。

## 云导出到隔离副本

`.epro2` 是便携 ZIP 导出，包含 project2.json、原生文档与库；`.eprj2` 是本地 SQLite 工程。改扩展名不是转换。

本机实测路径：

1. Gateway 按目标 UUID 导出 `.epro2`，保留原始导出与文档清单。
2. 复制已知可打开的 `.eprj2` 模板为任务目录的一次性副本，通过 CLI headless 打开。记录 sessionId 与路径，不编辑模板本身。
3. 等待 `getCurrentProjectInfo()` 有效树；确认 `isOnlineMode()` 为 false、是任务副本，再导入。模板若含旧文档，可用空模板，或仅在已验证的一次性副本清理，不清理线上工程。
4. 在该会话执行下面的导入方式，所有变量值实际获取：

```javascript
const p = await eda.dmt_Project.getCurrentProjectInfo();
if (eda.sys_Environment.isOnlineMode() || !p || p.uuid !== expectedCopyUuid)
  throw Error('Not the expected offline copy');
const f = await eda.sys_FileSystem.readFileFromFileSystem(exportAbsolutePath);
if (!f) throw Error('Export file unavailable');
const imported = await eda.sys_FileManager.importProjectByProjectFile(
  f, 'JLCEDA Pro', {importOption: 'ImportDocument'},
  {operation: 'Existing Project', existingProjectUuid: p.uuid}
);
return {imported, project: await eda.dmt_Project.getCurrentProjectInfo()};
```

5. 对比导出与实际导入的板、原理图页、PCB，使用导入后的 UUID。4.1.60 曾返回成功但漏个别原理图页；源档案与云工程仍有该页，不能判定为用户删除。此时 Gateway 在线只读导出该页。
6. 在副本按需导出 SVG/渲染图或编辑，并检查结果。只改本地副本不能宣称已更新线上工程。
7. `finally` 关闭本任务创建的 session；保留有用导出与副本，不关闭用户原有会话或窗口。

当时试过 `Offline Client Local Path` 导入未产生预期文件；这是版本观察，优先使用实测的 Existing Project 路径，不断言接口永不可用。导入可能重建 UUID、更新自动图签；审阅原始排版应对照在线原稿。

原生档案可能保留 `DELETE_DOC` 历史记录，不能全部计入当前文档。活动清单以实际工程树核对，再解析对应源码。漏页或显示异常如实说明，不用导入成功代替完整性检查。
