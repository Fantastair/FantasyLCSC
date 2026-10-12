# 本次验证记录

验证日期：2026-10-12，北京时间。平台：Windows x64，Rust 1.96.1。

## 离线检查

- `cargo test --locked`：14 项回归测试全部通过（含更新参数与未安装版本检查）。
- `cargo clippy --all-targets --locked -- -D warnings`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo build --release --locked`：单一 CLI 入口编译成功。

## 使用 Rust 程序实测上游

| 查询 | 结果 |
| --- | --- |
| LM358、现货、价格升序、每页 3 条 | total=196，HTTP 模式成功 |
| 电阻、439、0603、10kΩ、±1%、现货 | total=68；首项返回阻值 10kΩ、精度 ±1% |
| 电阻、439 属性选项 | 7 种参数，状态 available |
| C8734 完整详情 | 商品 ID 9243，12 项参数；HTTP 成功 |
| C8734 强制 browser 模式 | 独立匿名浏览器成功，12 项参数 |
| 类目树匹配“电阻” | 50 个匹配节点 |
| C492411 EasyEDA info | 标题 PZ254R-11-03P，关联一个模型 |

## 原始 STEP 下载

| 编号 | 文件 | 字节数 |
| --- | --- | ---: |
| C492411 | C492411_PZ254R-11-03P.step | 1,423,728 |
| C492414 | C492414_PZ254R-11-06P.step | 1,674,611 |
| C2765186 | C2765186_TYPE-C_16PIN_2MD_073.step | 2,100,834 |

三个文件均通过结构完整性检查；实际字节数与参考资料记录相同，未声称已完成 CAD 几何或尺寸检查。

本地实际响应和 STEP 保存在 `artifacts/`，该目录被 Git 忽略。以上接口统计是此前使用相同 CLI 核心的实测值，不作为长期固定结果。

## 安装、更新与 skill 合并

在工作区隔离安装目录实测安装和 `update --local`：后台 updater 等待调用进程退出，Windows PowerShell 5.1 成功构建并原子替换 EXE，最终状态为 complete，安装文件 SHA-256 与构建结果一致。两个旧 skill 的测试备份内容完整保留，新的合并 skill 同步更新。源码未提交时普通更新拒绝执行并报告 failed，不改变已安装 EXE。

合并 skill 通过 skill-creator 的 quick_validate 校验；全部本地 Markdown 引用可解析。个人设计规则与原 skill 正文逐字比对一致（仅调整相对链接），接入脚本与证据原样保留，新增选型与更新使用说明。

固定 D 盘正式安装实测：只读远端检查成功；前台 updater 从 GitHub 获取最新提交，在 `D:/Agent/LocalManager/FantasyLCSC/source` 构建并同步更新程序和 skill，最终状态 complete。当前 Codex 沙箱的后台进程受到 skill 目录写保护，此限制已写入安装与技能说明；通过审批后的前台 updater 完成验证。用户 PATH 已追加固定目录，旧两项 skill 的真实完整备份已保留。
