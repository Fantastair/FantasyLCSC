# FantasyLCSC 元器件选型

固定位置为 `D:\Agent\LocalManager\FantasyLCSC\fantasy-lcsc.exe`，加入用户 PATH 后也可使用 `fantasy-lcsc`。源码仓库为 `https://github.com/Fantastair/FantasyLCSC`；安装记录在同目录 `installation.json`，记录实际源码目录和 Git 提交。更换机器时先确认程序存在；不存在时按仓库 README 安装，不把这台机器的路径视作所有环境的约定。

## 查询流程

```powershell
$lcsc = 'D:\Agent\LocalManager\FantasyLCSC\fantasy-lcsc.exe'
& $lcsc --version
& $lcsc catalogs 电阻 --json
& $lcsc filters 电阻 --catalog 439 --json
& $lcsc search 电阻 --catalog 439 --package 0603 `
  --param '阻值=10kΩ' --param '精度=±1%' --in-stock --quantity 100 --json
& $lcsc detail C8734 --json --out '<任务目录>\C8734.json'
& $lcsc info C492411 --json
& $lcsc download C492411 C492414 -o '<任务目录>\step' --json
```

- 属性值使用 `filters` 返回的原始值，参数代码随类目变化；`mΩ`/`MΩ` 大小写敏感。属性筛选只支持单类目，不假造数值范围参数。
- 搜索结果是摘要；`detail` 才返回完整参数、参数代码列表、规格书、人民币阶梯价与分来源库存。对需要核对的额定值阅读规格书，不把 PDF 未检查的内容写成结论。
- 不混用 C 编号、国内商品 ID、EDA 元件 UUID、封装 UUID 和模型 UUID。详情会先搜索精确 C 编号映射商品 ID，再核对页面身份。
- `info` 的 EasyEDA 采购字段与国内商城口径不同，币种未知；不能按国内实时报价使用。有模型仅用于结构参考。
- `download` 保存上游原始 STEP，不改尺寸、不生成网格转换文件。默认验证并跳过完整旧文件，只有用户任务需要替换时使用 `--overwrite`。结构检查不是 CAD 几何或尺寸检查。
- JSON stdout 与错误 stderr 分开；退出码 0 成功、1 上游/网络/部分下载失败、2 参数错误。批量下载检查 `results[].ok/error`，不要仅看已出现的成功文件。
- PowerShell 消费 JSON 用 `ConvertFrom-Json -AsHashtable`，避免 `MΩ`/`mΩ` 等大小写不同键的冲突；Windows PowerShell 5.1 没有此参数时将 JSON 保存给程序解析。
- `--backend auto` 在商城页受校验时使用独立匿名 Chrome/Edge；不读取用户日常账号/Cookie，不自动解验证码。`--timeout` 设请求秒数，默认直连；需要时显式 `--proxy`，不修改系统代理。

把候选的 C 编号、型号、封装、关键电气参数、数量对应价格、库存来源、规格书与获取时间保留在任务产物中。筛选与推荐不授权下单或修改 EDA 工程；实际放置器件时还须核对符号引脚、封装焊盘和网络关系。

## 更新

```powershell
& $lcsc update --check --json  # 获取远端提交信息，不替换程序
& $lcsc update               # 启动更新，退出当前 CLI 后由后台 updater 完成
& $lcsc update --local       # 明确用本地源码重建，不拉取远端

# 需要前台等待并查看最终结果时直接运行 updater
& 'D:\Agent\LocalManager\FantasyLCSC\update.ps1'
```

检查更新通过只读 ls-remote 查询，不写工作区 Git 元数据。默认更新要求记录的源码仓库位于预期分支、origin 未改变、工作区干净且提交在远端历史中；在固定安装目录的独立 `source/` 副本中克隆/fast-forward 拉取与构建，不修改用户工作区，也不丢弃副本的修改或强制重置。成功后安装记录指向固定源码副本。源码更新需要 Git、Rust 和 MSVC 构建工具；在当前机器它们已经安装。`--local` 明确从记录的当前源码重建，允许未提交修改。

`update` 返回 started 只代表 updater 已启动；最终结果查看安装目录 `update-status.json` 与 `update.log`。成功更新同时安装 CLI 和本仓库的 easyeda-engineering skill；不会新增采购或 EDA 编辑动作。配置记录了 skill 安装目录；两个原 skill 的完整备份保存在固定安装目录的 `skill-backups/`，不继续作为重复发现的入口。
