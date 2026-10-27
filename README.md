# KeyEcho

[中文](README.md) | [English](README.en.md)

Windows 托盘音效工具：将全局组合键绑定到本地音频，按下快捷键即可播放。

## 功能

- 支持多个按键同时按下触发，例如 `CTRL+S+1`。
- 新触发的音频会中断当前播放。
- 托盘菜单提供「重启」（重新加载配置）和「退出」。
- 单实例运行，重复启动会自动退出。
- 便携式使用，配置和音频可随应用目录一起移动。
- EXE 与托盘共用内嵌图标，运行时无需外部 ICO 文件。

目前仅支持 Windows 10/11，没有配置编辑界面。

## 快速开始

1. 从 [Releases](https://github.com/NGTsjc/KeyEcho/releases) 下载 `KeyEcho_<版本>_win64.zip` 并解压。
2. 将自己的音频文件放入 `asset/`；如果该目录不存在，手动创建。
3. 编辑 `config.json`，填写快捷键和实际存在的音频路径。
4. 运行 `KeyEcho.exe`，在系统托盘找到图标，按下配置的快捷键播放音频。

运行目录示例：

```text
KeyEcho/
├── KeyEcho.exe
├── config.json
├── asset/
│   └── hello.wav       # 自行添加，不随安装包提供
└── keyecho.log         # 运行时生成
```

修改配置后，右键托盘图标，点击「重启」使其生效。此操作只重载配置；更新 EXE 后，需要先「退出」旧进程，再运行新版。

## 配置

`config.json` 示例：

```json
{
  "bindings": [
    {
      "id": "hello",
      "hotkey": "CTRL+S+1",
      "audio_path": "asset/hello.wav"
    },
    {
      "id": "warning",
      "hotkey": "CTRL+SHIFT+W",
      "audio_path": "D:/Audio/warning.mp3"
    }
  ]
}
```

| 字段 | 说明 |
| --- | --- |
| `id` | 非空且唯一的绑定标识，重复项会被跳过 |
| `hotkey` | 用 `+` 连接按键，不区分字母大小写；组合中的所有键同时按下时触发 |
| `audio_path` | 音频的绝对路径，或相对于 `config.json` 所在目录的路径 |

JSON 中的 Windows 路径建议使用 `/`；使用反斜杠时需要写成 `\\`。

常用按键：

| 类型 | 写法 |
| --- | --- |
| 修饰键 | `CTRL`、`ALT`、`SHIFT`、`WIN` |
| 字母、数字、功能键 | `A`–`Z`、`0`–`9`、`F1`–`F24` |
| 常用控制键 | `SPACE`、`ENTER`、`TAB`、`ESC`、`BACKSPACE`、`DELETE`、`INSERT` |
| 导航键 | `UP`、`DOWN`、`LEFT`、`RIGHT`、`HOME`、`END`、`PAGEUP`、`PAGEDOWN` |
| 标点键 | `MINUS`、`EQUAL`、`COMMA`、`PERIOD`、`SLASH`、`BACKSLASH`、`SEMICOLON`、`QUOTE`、`BACKQUOTE` |

快捷键不会拦截按键，其他应用仍会收到这些按键。建议避免与日常操作冲突的组合。

程序优先读取 EXE 同目录的 `config.json`；不存在时尝试当前工作目录。加载失败后程序仍驻留托盘，但不会保留原有绑定。字段齐全但包含空值、重复 ID 或不支持按键的条目会被跳过；JSON 格式错误或条目缺少必填字段会导致整份配置加载失败。音频文件是否可读取、可解码，在触发播放时检查。

## 从源码运行与打包

在 Windows 上安装支持 Rust 2024 edition 的 Rust 工具链，以及 MSVC 构建工具和 Windows SDK。在项目根目录执行：

```powershell
cargo test

# 首次开发运行前准备配置，并在 asset/ 中添加对应音频
Copy-Item .\build\windows\config.template.json .\config.json
cargo run
```

已有 `config.json` 时直接编辑即可，避免再次复制模板覆盖自己的配置。模板引用 `asset/sample.wav`，需要自行提供音频或修改路径。

构建和打包：

```powershell
cargo build --release
powershell -ExecutionPolicy Bypass -File .\build\windows\package.ps1
```

| 输出 | 路径 |
| --- | --- |
| 调试构建 | `TempFile/target/debug/KeyEcho.exe` |
| Release 构建 | `TempFile/target/release/KeyEcho.exe` |
| 便携式打包目录 | `TempFile/dist/windows/` |

打包脚本会构建 Release、复制 EXE 和配置模板、创建音频目录，并清理旧版遗留的 `asset/logo.ico`。它会覆盖打包目录的 `config.json`，音频文件会保留；打包前请另存自定义配置。该脚本只生成目录，ZIP 由发布工作流生成。

源码中的 `asset/logo.ico` 是构建输入，需保留。修改它后重新构建，EXE 和托盘图标会一起更新；发布目录不再需要 ICO，遗留的外部 ICO 也不会被程序读取。

## 版本发布

[发布工作流](.github/workflows/release.yml) 在 GitHub 收到 `v*` 标签时触发，构建的是**标签指向的提交**。推送 `master` 不会触发发布，也不会移动已有标签。

先将 `Cargo.toml` 中的版本更新为准备发布的版本，再提交修改。以下命令在项目根目录执行，`github` 为本项目的 GitHub remote 名称；示例版本需替换为尚未发布的版本。

```powershell
# 1. 检查并提交本次发布内容
git status
git add .
git commit -m "Release v1.0.2"

# 2. 测试并推送代码；测试通过后再继续
cargo test
git push github master

# 3. 提交完成后，再创建和核对标签
git tag v1.0.2
git log -1 --oneline
git show -s --oneline v1.0.2

# 4. 确认标签指向要发布的提交，再仅推送这个标签
git push github refs/tags/v1.0.2
```

在 GitHub 仓库的 Actions 页面确认构建成功，再到 Releases 下载 `KeyEcho_1.0.2_win64.zip`。CI 会在构建工作区内将 Cargo 版本同步为标签版本，不会修改仓库中的提交。

如果产物仍是旧版，先用 `git show -s --oneline <标签>` 检查标签指向。`git add` 只暂存文件，只有 `git commit` 才创建提交。若旧标签已经发布，建议创建新的补丁版本标签；仅重新运行旧标签的工作流仍会构建旧提交。

## 故障排查

| 现象 | 检查方式 |
| --- | --- |
| 运行后没有窗口 | 属于正常行为，在系统托盘或隐藏图标区域查找 KeyEcho |
| 再次启动没有反应 | 程序只允许一个实例，先退出已有实例 |
| 快捷键没有声音 | 检查配置、音频路径、输出设备，以及 `keyecho.log` 中的错误 |
| 修改配置未生效 | 在托盘菜单点击「重启」，查看日志中的配置路径及有效绑定数量 |
| 更新后托盘仍是旧图标 | 退出旧进程并运行新版 EXE，确认运行路径及发布标签对应的提交 |

日志 `keyecho.log` 位于选中的配置文件同目录，正常启动时会覆盖上次日志；需要排查时可先保存旧日志。

## 项目结构

```text
src/
├── main.rs             # 可执行程序入口
├── lib.rs              # 库入口
├── app/                # 启动、日志、配置重载和事件调度
├── config/             # 配置校验、快捷键解析、路径处理
├── audio/              # rodio 音频播放
└── platform/           # Windows 键盘钩子、托盘、单实例管理
asset/logo.ico          # EXE 和托盘共用的构建资源
build.rs                # 嵌入 Windows 图标资源
build/windows/          # 配置模板和打包脚本
.github/workflows/      # 标签发布工作流
```
