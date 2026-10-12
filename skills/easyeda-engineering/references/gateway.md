# 在线 Gateway 工作流

## 连接与目标定位

链路为 agent → localhost HTTP Bridge → WebSocket → EDA 的 Run API Gateway 扩展 → `eda` API。无需 agent 登录云账户；用户在 EDA 内正常登录。不要从本机文件提取密码或令牌来替代这条链路。

1. 运行 `bridge-client.cjs status`，在 49620–49629 探测 `/health`，验证 `service === 'easyeda-bridge'`，再查 `/eda-windows`。服务存在但 `edaConnected` 为 false，表示扩展尚未连接。
2. 对候选窗口执行 `inspect-eda.js`，以工程 UUID/名称和树确认目标。完整 JSON 存任务目录，只输出必要摘要。首次加载可能返回 null 或空树，可短暂等待后再查。
3. 后续 `/execute` 请求传确定的 `windowId`；代码明确 `await` 与 `return`。修改前先检查，变量值从实际目标获得：

```javascript
const p = await eda.dmt_Project.getCurrentProjectInfo();
if (!p || p.uuid !== expectedProjectUuid) throw Error('Target project mismatch');
// 根据 p.data 确认目标文档 UUID，再打开或修改。
return {projectUuid: p.uuid};
```

请求为 `POST /execute`，JSON `{code, windowId}`；检查 HTTP 状态及返回的 `success`、`result`、`windowId`。普通连接检查不扫描所有团队、全部工程；用户要求查找工程时再调用团队/文件夹/工程列表接口。

## 独立窗口

桌面全在线模式已验证以下方式可新开原生 EDA 窗口：

```javascript
eda.sys_Window.open('https://pro.lceda.cn/editor#id=' + projectUuid, '_blank');
return {opened: projectUuid};
```

记录打开前后 `/eda-windows` 集合差。新窗口不一定自动运行 Gateway，需要用户在该窗口运行 Run API Gateway/重新连接，并按产品提示允许外部交互。说明这是该窗口尚未接入的必要步骤，原窗口无需重新操作。加入后再次检查工程身份，不能仅凭集合差断定正确。

`dmt_Project.openProject(uuid)` 切换执行请求所在窗口；用户要求另开窗口时不可用它代替。连接 ID 可因重连改变，不保存用户窗口 ID 到技能配置。

## 读取、导出与显示

- 详细树：`await eda.dmt_Project.getCurrentProjectInfo()`；`getProjectInfo(uuid)` 只给简略信息。
- 不切换窗口导出：`await eda.sys_FileManager.getProjectFileByProjectUuid(uuid, fileName, undefined, 'epro2')`。
- 打开已确认文档：`await eda.dmt_EditorControl.openDocument(documentUuid)`，返回 tabId。当前源码：`await eda.sys_FileManager.getDocumentSource()`。
- 原理图 SVG：`await eda.sch_ManufactureData.getSvgFile(fileName)`。
- 当前渲染图：`await eda.dmt_EditorControl.getCurrentRenderedAreaImage(tabId)`；空白/黑图或裁切图不能作为完整审阅证据，需等渲染或改用 SVG/本地副本。
- 缩放：`zoomToAllPrimitives(tabId)`；`zoomToRegion(left, right, top, bottom, tabId)` 为五个位置参数，不能传一个坐标对象。
- PCB 顶底视图：先 `selectLayer(1)` 或 `selectLayer(2)`，再配合 `setLayerVisible`；顶层常用 `[1,3,11,12]`，底层 `[2,4,11,12]`，实际层定义先用 `getAllLayers()` 核对。只改可见层而不选活动层，可能仍看到错误一侧。审阅优先在副本中调整。

File/Blob 不能直接依赖 JSON 序列化传输；使用 `arrayBuffer()` 后编码或官方保存接口。base64 编码分块处理，不将大数组一次 spread；结果重定向本地 JSON 再解码，不在工具输出打印大图。每个导出请求控制在 Bridge 30 秒超时内，按文档拆分任务。

## 故障处理

| 现象 | 检查与处理 |
| --- | --- |
| “Bridge 未找到”且 health 不通 | 查现有进程、端口、日志；确认官方依赖、环回地址和服务启动，必要时启动一次 |
| health 正常但没有 EDA 窗口 | 当前窗口运行扩展/重新连接；确认用户按产品提示允许外部交互 |
| 新窗口不在列表 | 新窗口独立运行扩展，不重启正常 Bridge |
| windowId 不存在 | 重新发现窗口并核对工程 |
| 请求超时 | EDA 代码可能继续执行；只读任务缩小批次，写入先读状态，未确认前停止重复写入 |
| 返回成功但画面异常 | 检查文档身份、实际对象、活动图层、渲染及导出内容 |

不要因扩展未连接而反复重装客户端、关闭防火墙或重启全部窗口。连接和权限确需用户动作时只请求该动作；其余可完成的检查继续执行。
