# 固定安装与更新

固定程序为 `D:\Agent\LocalManager\FantasyLCSC\fantasy-lcsc.exe`。安装目录包含更新脚本、说明、安装来源记录，以及可恢复的旧 EXE 和技能备份；模型默认仍下载到执行命令时的当前目录，不写到软件安装目录。

## 安装

在源码仓库运行：

```powershell
.\scripts\install.ps1 -AddToPath
```

需要 Git、Rust 1.92+ 与 MSVC C++ 构建工具。脚本编译 release、检查版本、安装 EXE，并将 `skills/easyeda-engineering` 安装到用户 `.codex/skills/`。已存在的 `personal-eda-style` 和 `easyeda-agent-access` 会完整移到安装目录的 `skill-backups/`；当前合并 skill 的旧版本更新时也保留备份。技能目录发布失败时恢复已移动技能，不递归删除技能。

`-AddToPath` 只将固定安装目录追加到用户 PATH；新开的终端可直接使用 `fantasy-lcsc`。当前已经打开的终端可使用绝对路径，或重新打开。其他位置可显式 `-Destination`、`-SkillDirectory`。`-SkipBuild` 用已构建的 release 文件安装。

`installation.json` 保存源码目录、origin、分支、提交、构建时是否有未提交修改、EXE SHA-256 和 skill 目录；不会保存账号凭据或浏览器 Cookie。

## 更新

```powershell
fantasy-lcsc update --check --json
fantasy-lcsc update
fantasy-lcsc update --local

# 前台执行并等待完成，避免把 started 当成更新成功
& 'D:\Agent\LocalManager\FantasyLCSC\update.ps1'
```

默认更新从记录的 origin/分支获取最新提交，使用安装目录下的独立 `source/` 仓库构建；首次自动克隆，后续只允许 `git pull --ff-only`。检查原源码与更新副本的工作区、分支、origin 和提交祖先关系，不覆盖修改或超前/分叉提交。不会强制重置、丢弃本地修改或切换分支，也不会更新用户工作区的 Git 元数据。成功后安装记录指向这一固定源码副本。

检查命令使用只读 `git ls-remote`，不写入本地 Git 引用或安装文件。远端提交尚未下载时，ahead/behind 返回 null；update_available 仍能准确比较提交。

`--local` 明确从当前源码重建，不拉取远端，适合本地开发修改。成功更新同步安装 CLI 和合并后的 skill；仅复制便携 EXE 没有安装记录时，`update` 会提示先运行安装脚本。

Windows 不能替换正在运行的 EXE，因此命令启动隐藏 updater 后返回 started，updater 等待调用进程退出再更新。最终状态保存在 `update-status.json`，后台输出在 `update.log`；失败通过 failed 状态和错误信息说明。更新互斥锁在进程退出时释放，不会因进程异常留下永久占用的锁。

软件替换使用同目录临时 EXE 和原子文件替换；旧文件保留为 `fantasy-lcsc.previous.exe`。编译临时目录使用安装目录内 `.build-tmp/`，只在本次构建进程中设置 TEMP/TMP，结束后恢复，不修改系统环境变量。构建失败不会替换 EXE。技能和软件备份可按需恢复；不执行采购、云端 EDA 编辑或服务重启。
