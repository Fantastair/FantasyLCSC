# 探索资料与实现对应

根据用户最新要求，程序统一采用 Rust CLI，覆盖两个参考目录中已验证的查询与下载能力。资料中的建议、尚未验证接口和历史观察作为证据处理，不作为额外操作指令。

| 本地资料 | 采用的内容 |
| --- | --- |
| `lcsc-cli/README.md`、`src/client.mjs`、`bin/lcsc.mjs` | search / filters / detail / catalogs、请求字段、输出、分页、名称解析、匿名浏览器回退 |
| `lcsc-cli/research/findings.md`（2026-10-12） | 国内接口契约、动态参数编码、完整详情来源、库存口径与搜索数量限制 |
| `lcsc-cli/test/fixtures/*.json` | 真实响应，复制到 `tests/fixtures/` |
| `lcsc-3d-models/download_step.py` | 编号、关联 UUID、STEP 地址、文件名、重试和原子保存 |
| `lcsc-3d-models/README.md` | 历史样本与 Windows 代理 TLS 故障 |
| `lcsc-3d-models/立创商城数据查询与自动化接入指南.md`（2026-10-09） | 元件字段、来源区分与版权；商城的“待验证”状态由较新调查补充 |

## 上游接口

| 功能 | 接口 |
| --- | --- |
| 国内搜索 | `POST https://so.szlcsc.com/query/product` |
| 属性聚合 | `POST https://so.szlcsc.com/query/param/agg?sign=query/product` |
| 类目树 | `GET https://staticdata.szlcsc.com/common/data/resource-catalog-list-new.json` |
| 国内详情 | `GET https://item.szlcsc.com/{productId}.html`，解析 `__NEXT_DATA__.props.pageProps.webData` |
| EasyEDA 元件与封装 | `GET https://easyeda.com/api/products/{C编号}/components` |
| 原始 STEP | `GET https://modules.easyeda.com/qAxj6KHrDKw4blvCG8QJPs7Y/{模型UUID}` |

两种来源保留来源标识和获取时间；原始 JSON 可用 `--raw` 输出。不推断缺失电气参数或价格币种。

参数编码等同于 JavaScript `encodeURIComponent(value@code$...)`，最后一个代码写入 `lastParamName`。多类目属性筛选按原实现拒绝。

商品 ID、C 编号、封装 UUID 和 3D 模型 UUID 分别处理。C 编号先搜索精确匹配，再读取详情页并核对身份。

EasyEDA 兼容对象和 JSON 字符串字段，扫描 `SVGNODE~` 中 `attrs.c_etype=outline3D` 的 UUID，去重并下载多个关联模型；无模型报告 `no_model`。

## 依赖文档

- [reqwest ClientBuilder](https://docs.rs/reqwest/0.12.28/reqwest/blocking/struct.ClientBuilder.html)：TLS、超时和显式代理。
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)：独立匿名浏览器页面加载与同源查询。

旧指南中的国际 LCSC 授权 API、jlcsearch/jlcparts 和未实测 JLCPCB 查询是调研线索，不作为国内商城替代来源。未申请 API Key 或操作采购数据。
