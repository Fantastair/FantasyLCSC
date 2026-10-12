# FantasyLCSC

用 Rust 编写的立创元件命令行工具，统一使用 `fantasy-lcsc`，支持国内商城搜索、参数筛选、类目查询、完整商品详情、EasyEDA 元件信息和批量下载原始 STEP 3D 模型。

无需 Node.js、Python 或登录账号。商城详情被校验时，`auto` 模式使用本机 Chrome/Edge 的独立匿名会话；模型下载不需要浏览器。

## Windows 使用

固定安装并合并 EDA skill：`./scripts/install.ps1 -AddToPath`。默认安装到 `D:\Agent\LocalManager\FantasyLCSC`，之后可运行 `fantasy-lcsc update --check` 检查更新或 `fantasy-lcsc update` 后台更新。安装、源码保护和恢复说明见 [docs/installation.md](docs/installation.md)。

构建后的程序位于 `dist/fantasy-lcsc.exe`，也可解压 `dist/FantasyLCSC-windows-x64.zip` 后在 PowerShell 中运行。默认下载目录是当前工作目录下的 `step/`，使用 `-o` 指定其他目录。

```powershell
# 批量下载；空格、逗号、中文分号分隔，自动去重
.\dist\fantasy-lcsc.exe download C492411 C492414 -o .\models
.\dist\fantasy-lcsc.exe download 'C492411，C492414；C2765186' --json
.\dist\fantasy-lcsc.exe download --file .\parts.txt -o .\models

# 默认验证并跳过已有文件；显式允许替换
.\dist\fantasy-lcsc.exe download C492411 --overwrite

# EasyEDA 元件、封装、关联模型与来源信息
.\dist\fantasy-lcsc.exe info C492411 C492414 --json

# 国内商城搜索、品牌名称、分页、人民币阶梯价
.\dist\fantasy-lcsc.exe search LM358 --in-stock --sort price-asc --json
.\dist\fantasy-lcsc.exe search LM358 --brand 'TI(德州仪器)' --page 2 --page-size 10

# 类目与当前类目的动态参数原始值
.\dist\fantasy-lcsc.exe catalogs 电阻
.\dist\fantasy-lcsc.exe filters 电阻 --catalog 439 --json
.\dist\fantasy-lcsc.exe search 电阻 --catalog 439 --package 0603 `
  --param '阻值=10kΩ' --param '精度=±1%' --in-stock --json

# 完整参数、规格书、阶梯价和各仓库存
.\dist\fantasy-lcsc.exe detail C8734 --json --out .\C8734.json
.\dist\fantasy-lcsc.exe detail https://item.szlcsc.com/9243.html --json
.\dist\fantasy-lcsc.exe detail C8734 C7950 --backend browser --timeout 45 --json
.\dist\fantasy-lcsc.exe --help
.\dist\fantasy-lcsc.exe search --help
.\dist\fantasy-lcsc.exe update --help
```

`search` 和 `filters` 支持：

| 选项 | 用途 |
| --- | --- |
| `--catalog`、`--brand` | ID 或完整名称，可重复 |
| `--package`、`--arrange` | 封装、包装，可重复 |
| `--label`、`--certification` | 标签、认证，可重复 |
| `--param` | 参数名或参数代码 `=原始值`，可重复 |
| `--in-stock`、`--discount`、`--datasheet`、`--domestic` | 现货、折扣、有规格书、国产 |
| `--min-price`、`--max-price`、`--quantity` | 价格区间、需求数量对应的阶梯 |
| `--second` | 二次检索 |
| `--page`、`--page-size` | 分页，页大小 1–100 |
| `--sort` | `relevance`、`price-asc`、`price-desc`、`sales`、`stock-desc`、`stock-asc`、`gd-stock-desc`、`gd-stock-asc`、`js-stock-desc`、`js-stock-asc` |

属性筛选只支持单个类目，工具动态解析参数代码。单位和值必须与 `filters` 响应一致，`mΩ` 和 `MΩ` 大小写敏感。

## 输出与连接

所有命令支持 `--json`、`--out 文件`；查询和下载支持 `--timeout 秒`。商城命令支持 `--backend auto|http|browser`。`--raw` 输出来源原始响应；下载的原始元数据请用 `info --raw`，`download` 和 `update` 不支持 `--raw`。

- JSON 结果写入 stdout，命令级错误写入 stderr；不混入进度文本。
- 退出码 `0` 成功，`1` 网络、上游或下载失败，`2` 参数错误。
- 下载单项失败仍继续，通过 `results[].error` 和非零退出码报告部分失败。批量元件信息查询也保留逐项错误。
- `fetched_at` 为 Unix 秒。商城价格币种为 `CNY`；EasyEDA 采购字段保留来源，币种标为未知。
- 默认直连并验证 TLS 证书，避免继承资料中曾失效的 Windows 系统代理。使用 `--proxy http://127.0.0.1:7897` 或 `FANTASY_LCSC_PROXY` 显式设置。
- 自动定位 Chrome/Edge 失败时，使用 `--browser-path` 或 `LCSC_BROWSER_PATH`。浏览器由 Rust 通过 CDP 管理，使用临时独立配置目录，结束时关闭。

HTTP 请求支持代理认证；浏览器回退需要无需认证的本地代理。

网络错误及 HTTP 429/500/502/503/504 最多请求三次。保存前检查 STEP 文件头、结束标记、HEADER/DATA 段和实体记录，使用同目录临时文件写入；失败不会用错误响应覆盖旧文件。跳过前也会验证旧文件，损坏文件需要 `--overwrite`。

## 构建与验证

Windows 需要 Rust 1.92+ 与 MSVC C++ 构建工具。

```powershell
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all -- --check

# 生成 CLI 便携 EXE、说明、许可证与 ZIP
.\scripts\build.ps1

cargo run -- search LM358 --json
cargo build --release --locked
```

当前交付及验证平台为 Windows x64。

仓库中的 [easyeda-engineering skill](skills/easyeda-engineering/SKILL.md) 合并了原接入能力与个人设计习惯，使用固定 CLI 做元器件选型、完整参数核查与 STEP 下载。原始接入脚本和设计规则均随附，后续与 CLI 一起更新。

`src/main.rs` 是唯一命令行入口；`lcsc` 模块负责国内商城，`models` 负责模型关联与下载，`http` 负责超时、重试和进度，`browser` 负责匿名浏览器回退。

离线测试使用探索资料中的真实响应，覆盖 Unicode 参数编码、完整详情与库存来源、参数错误退出码、重试、取消、跳过、STEP 错误页和截断识别、失败保留旧文件、禁止意外覆盖。Windows CI 运行测试和构建，并生成便携程序。

## 数据来源与范围

实现依据用户提供的 `D:/Agent/LocalManager/lcsc-cli/` 与 `D:/Agent/LocalManager/lcsc-3d-models/` 探索资料。对应关系见 [docs/sources.md](docs/sources.md)。2026-10-12 的国内商城调查补充了旧文档中尚待验证的接口。

覆盖两个目录中已经验证的公开查询和 STEP 下载能力。国际商城授权 API、下单、购物车、MRO、符号/封装转换及未验证的 JLCPCB 查询不在当前范围；国内数据不会用国际站替代。

非官方公开接口可能变化。校验页、格式变化和商品不存在会报告失败，不自动解验证码。搜索总数可能受上游检索窗口限制，不能用于声称全量导出。搜索保留服务端排序和推广置顶，不同库存来源与详情展示价格分别保存。

模型来源为 [嘉立创 EDA](https://lceda.cn/) / [EasyEDA 官方库](https://easyeda.com/)，模型相关版权归其权利人所有。程序只下载原始 STEP，不转换网格或修改尺寸。结构检查不等同于 CAD 几何检查；用于结构配合前，请结合规格书核对尺寸。
