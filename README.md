# 命令管理器

Windows 上的命令速取面板。常驻托盘，热键唤出，检索后单击即复制，窗口自动收起。

- 单个可执行文件，静态链接 CRT，不依赖运行时
- Rust + GPUI，GPU 即时模式渲染
- 深色 / 浅色双主题，设置中切换并持久化
- 无边框窗口；拖至屏幕左右边缘收为 6 像素条，指针掠过滑出
- 命令归入分组，支持多分组、标签、多行备注与多选连续复制
- 检索覆盖名称、标签、备注与正文；多词任意顺序，未命中时按编辑距离近似

## 安装

| | |
|---|---|
| 安装包 | [CommandManager-0.3.0-windows-x64-setup.exe](https://github.com/ORyong/CommandManager/releases/download/v0.3.0/CommandManager-0.3.0-windows-x64-setup.exe) |
| 免安装版 | [CommandManager-0.3.0-windows-x64-portable.zip](https://github.com/ORyong/CommandManager/releases/download/v0.3.0/CommandManager-0.3.0-windows-x64-portable.zip) |
| 校验和 | [SHA256SUMS-0.3.0.txt](https://github.com/ORyong/CommandManager/releases/download/v0.3.0/SHA256SUMS-0.3.0.txt) |
| 默认路径 | `%LOCALAPPDATA%\Programs\CommandManager` |
| 系统要求 | Windows 10 1809 及以上，x64 |
| 覆盖升级 | 从托盘退出旧版本，装到同一目录，`data` 保留 |

免安装版解压到可写目录，运行 `command-manager.exe`。静默安装：`setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART`。旧版本见 [Releases](https://github.com/ORyong/CommandManager/releases)。

## 使用

| 操作 | 结果 |
|---|---|
| `Ctrl+Alt+X` | 显示 / 隐藏；热键可在设置中改，冲突会提示 |
| 托盘左键、托盘“打开”、再次启动 | 显示窗口 |
| 单击命令 | 复制正文，显示反馈后约 800 毫秒收起 |
| 搜索区留白或工具栏空白处拖动 | 移动窗口 |
| 拖到屏幕边缘松手 | 收为 6 像素，指针移入滑出，移开约 0.7 秒收回 |
| 工具栏 ✕ | 收进托盘，不退出 |
| 多选 → 勾选 → 复制 N 条 | 按勾选顺序换行拼接 |

命令的正文只进剪贴板，不执行；备注、标签与选择序号不进入剪贴板。

数据位于安装目录 `data/`：`saved_commands.json` 存命令库，`settings.json` 存设置。升级与卸载保留整个目录。内置命令可改备注与分组，改正文需先另存为副本。

## 资源占用

隐藏态释放工作集，托盘常驻约 1–2 MB；可见态取决于窗口尺寸与字形图集，实测约 20 MB 稳态、73 MB 首帧峰值。后台定时器可见时 16 毫秒、隐藏时 100 毫秒，隐藏态 CPU 约 0.4%。

## 构建

需要 Rust 1.97.1、MSVC Build Tools 与 Windows SDK。

```powershell
cargo test --locked --no-default-features
cargo test --locked --features ui-tests --test list_scroll
cargo build --release --locked
```

`.cargo/config.toml` 启用静态 CRT，并把编译并行度限制为 4：单个 rustc 峰值可达 1–2 GB。GPUI 与组件库固定在 `Cargo.toml` 的 Git 提交上。

开发参数：`--data-dir <目录>` 指定独立数据目录；`--background` 静默启动；`--stay-open` 关闭“失焦收起”，便于调试。

打包（需 Inno Setup 6）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
```

产物在 `dist/`：安装包、免安装 ZIP、源码 ZIP 与 `SHA256SUMS.txt`。

## 结构

| 路径 | 内容 |
|---|---|
| `src/style.rs` | 主题、色板、尺寸常量 |
| `src/ui.rs` | 界面状态机：页面栈、选择、拖拽、贴边、定时 |
| `src/ui/views.rs` | 各页面布局 |
| `src/model.rs` | 命令库模型与内置模板 |
| `src/command_search.rs` | 检索与打分 |
| `src/atomic_file.rs` | 原子写入与跨进程锁 |
| `src/platform.rs` | 托盘、热键、单实例、窗口移动 |

设计约定见 [assets/STYLE.md](assets/STYLE.md)，需求与验证记录见 [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md)、[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)，版本变更见 [docs/CHANGELOG.md](docs/CHANGELOG.md)。

## 许可

MIT，全文见 `LICENSE`。内嵌 Geist 字体为 OFL，见 `assets/LICENSE-GEIST`；全部依赖的许可原文汇总在 `assets/THIRD-PARTY-NOTICES.txt`。
