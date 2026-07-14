# 图吧工具箱 Rust 版

面向 Windows 的原生工具箱启动器，兼容既有的图吧工具箱资源包、加密 `EDB/EDT` 工具清单和 `.skin` 皮肤。项目使用 Rust 重建主界面与资源访问层，不依赖 WebView 或 .NET 运行时。

## 功能

- 原生 Win32 界面，支持高 DPI、窗口拖动、最小化和关闭。
- 从 `List/*.edb` 与配套的 `*.EDT` 读取工具分类、说明和图标。
- 支持 CPU、主板、内存、显卡、硬盘、屏幕、综合检测、外设、烤机、游戏及其他工具分类。
- 通过 WMI 收集操作系统、处理器、主板、内存、显卡、磁盘、显示器、网络、音频、电池和 USB 等硬件信息。
- 支持加载 `.skin` 皮肤、保存界面设置，以及内置新手指引、屏幕坏点/漏光测试和一键烤机。
- 对工具路径做规范化和根目录边界校验，并根据 PE 架构检查当前主机是否可运行目标程序。

## 目录结构

```text
.
├── crates/
│   ├── tbtool-app/    # Windows 图形应用
│   ├── tbtool-core/   # 配置、皮肤、硬件、工具目录与启动逻辑
│   └── tbtool-edb/    # EDB/EDT 加密与读写兼容库
├── Config.ini         # 用户设置与当前皮肤信息
├── List/              # 加密工具清单（.edb/.EDT）
├── skin/              # .skin 皮肤包与解压后的 skin/user/
├── tools/             # 被启动的工具及其工作目录
├── data/              # 硬件识别数据库
└── Cargo.toml
```

`Config.ini`、`List/`、`skin/`、`tools/` 和 `data/` 是运行时资源，不作为源码构建产物生成。它们目前被 `.gitignore` 排除；从仅含源码的克隆副本构建或运行时，需要从兼容的图吧工具箱资源包中准备这些目录和文件。

## 环境要求

- Windows 10/11。
- Rust 1.85 或更新版本，使用 Rust 2024 edition。
- 可用的 Windows WMI 服务，用于硬件信息页。

应用仅在 Windows 上提供图形界面；在其他平台可以编译核心数据处理代码，但不能运行图形应用或硬件检测。

## 构建与运行

在项目根目录执行：

```powershell
cargo build --release -p tbtool-app
```

生成的程序位于 `target\release\tbtool.exe`。将其放在运行时资源根目录中，或将整个资源目录复制到该可执行文件所在目录，然后运行：

```powershell
.\target\release\tbtool.exe
```

程序会从当前目录或自身所在目录向上查找包含 `List/`、`skin/` 和 `tools/` 的资源根目录，并从该目录加载 `Config.ini`。

## 测试

```powershell
cargo test --workspace
```

测试覆盖 EDB/EDT 加密数据的往返读写、GBK/UTF-8 文本与 INI 处理、皮肤包加载、工具目录解析和启动计划。部分测试以当前资源包为兼容性语料；缺少 `data/`、`List/`、`skin/` 或 `tools/` 时会失败。Windows 下还会执行一次本机 WMI 硬件检测。

## 开发说明

- `tbtool-edb` 可独立用于检查和编辑兼容的加密 `EDB/EDT` 数据文件。
- 工具清单中的可执行路径必须解析到 `tools/` 之内，启动器拒绝目录穿越和资源根目录以外的目标。
- 皮肤包必须含有 `Config.ini` 及七个必需图片资源；单个资源最大为 32 MiB。
- 选择工具时单击查看说明，双击启动。内置“一键烤机”会同时启动 Prime95 和 FurMark。

## 注意事项

烤机、压力测试和显卡负载工具可能快速提高设备功耗和温度。运行前请确认散热、电源和监控条件正常，并全程留意温度与系统稳定性。