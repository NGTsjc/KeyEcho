# KeyEcho

[中文](README.md) | [English](README.en.md)

托盘常驻的 Windows 工具：把全局快捷键绑定到本地音频文件，按下快捷键即播放对应音频。

## 功能特性
- 和弦式全局热键：任意多个键同时按下触发（如 `CTRL+S+1`），按住不重复播放
- 系统托盘常驻，托盘菜单支持「重启」（重新加载配置）和「退出」
- 单实例运行：重复启动会自动退出
- 单音频播放：新触发会中断当前播放
- 便携式设计：整个目录复制到任意位置即可运行

## 环境要求
- Windows 10/11，仅支持 Windows（macOS 暂未实现）

## 快速开始
1. 从 Releases 页面下载 `KeyEcho_<版本>_win64.zip` 并解压
2. 把音频文件放入 `asset/` 目录
3. 编辑 `config.json`，配置快捷键与音频
4. 运行 `KeyEcho.exe`，按下配置的快捷键即可播放

## config.json 配置
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
- `id`：唯一标识，不能重复，重复会被跳过
- `hotkey`：用 `+` 连接多个键，所有键**同时按下**触发。支持 `CTRL` / `ALT` / `SHIFT` / `WIN` 等修饰键，以及字母、数字、`F1-F24`、方向键、`SPACE` / `ENTER` 等常见按键
- `audio_path`：绝对路径，或相对 `config.json` 所在目录的相对路径

修改配置后，在托盘菜单点击「重启」即可生效。

## 从源码构建
```powershell
cargo test                     # 单元测试
cargo run                      # 调试运行
powershell -ExecutionPolicy Bypass -File build/windows/package.ps1   # 一键打包
```
打包产物位于 `TempFile/dist/windows/`。

## 发布
推送 `v*` 格式的 tag 到 GitHub 会自动触发 [GitHub Actions](.github/workflows/release.yml) 构建并创建 Release：
```powershell
git tag v1.0.0
git push github master --tags
```
