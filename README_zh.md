**[English](README.md) | 中文**

> [!CAUTION]
> **这是 [nekename/OpenDeck](https://github.com/nekename/OpenDeck) 的 Windows 优化分支。**
> 上游作者倾向于通用型跨平台方案；本分支中的改动是针对 Windows 的务实补丁，
> [未被上游接受](https://github.com/nekename/OpenDeck/pull/357)。
> 这些改动在我的机器上运行良好，但不一定适合通用场景。
>
> **与上游的差异：**
>
> - **系统休眠/唤醒检测** (`system_sleep_watchdog.rs`) — 以 1 Hz 频率轮询系统挂钟时间，
>   若两次轮询间隔超过 3 秒则判定系统经历了休眠。等待 2 秒 USB 稳定后，重新枚举所有设备并唤醒处于空闲休眠的设备。
> - **唤醒后 WebView 插件恢复** (`plugins/mod.rs` 中的 `reload_webview_plugins()`) — 使用
>   Tauri 原生 `window.reload()`（通过 WebView2 ICoreWebView2 控制器，绕过休眠后可能退化的 JS 引擎），
>   然后通过隐藏 iframe 恢复原生 `setInterval`/`setTimeout` 并重新执行连接初始化 JS
>   （Elgato SDK 的 `timers.js` Web Worker 在休眠期间会死亡，导致所有插件定时器停摆）。
> - **`systemDidWakeUp` 事件广播** (`events/outbound/misc.rs`) — 唤醒后向所有已连接插件发送新事件，
>   允许原生插件重新初始化设备连接。
>
> 其余部分（架构、UI、插件 SDK 兼容性）与上游完全一致。

# OpenDeck

适用于 Elgato Stream Deck 的桌面软件

![主界面](.github/readme/mainmenu.png)
[更多截图](#展示)

OpenDeck 是一款桌面应用，用于在 Linux、Windows 和 macOS 上使用 Elgato Stream Deck 等流控制器设备。OpenDeck 支持为原版 Stream Deck SDK 开发的插件，可以兼容 Elgato 软件生态中的大量插件，也支持 [OpenAction](https://openaction.amankhanna.me/) API。

官方仅支持 Elgato 硬件，但可通过插件支持其他硬件厂商的设备。

> [!TIP]
> 手边没有 Stream Deck？使用 OpenDeck 搭配 [Tacto](https://tacto.live/) 可以把任意智能手机变成 Stream Deck！

如果你想支持 OpenDeck 的开发，可以在 [GitHub Sponsors](https://github.com/sponsors/nekename)、[Ko-fi](https://ko-fi.com/nekename) 或 [Liberapay](https://liberapay.com/nekename) 上赞助我！Stream Deck 的强大离不开软件的支持，只需 $5（仅占 Stream Deck+ 价格的 2.5%）就能帮上大忙。

特别感谢 [Tauri](https://github.com/tauri-apps/tauri)、[elgato-streamdeck](https://github.com/OpenActionAPI/rust-elgato-streamdeck) Rust 库以及 [Phosphor Icons](https://phosphoricons.com/) 的开发者们。

### 为什么选择 OpenDeck？

- **Stream Deck 插件**：OpenDeck 支持 Elgato 生态中用户熟悉的大部分 Stream Deck 插件，比其他第三方软件（如 streamdeck-ui、StreamController、Boatswain 等）的兼容性好得多。
- **跨平台**：OpenDeck 同时支持 Linux、Windows 和 macOS。macOS 用户还能通过 Wine 运行仅支持 Windows 的插件，这是 Elgato 官方软件做不到的。此外，配置文件可以在不同平台间无缝迁移。
- **功能丰富**：从多动作、切换动作，到应用切换时自动切换配置文件、亮度控制，OpenDeck 具备你对流控制器软件的所有期望功能。
- **开源**：OpenDeck 源代码采用 GNU 通用公共许可证，任何人都可以查看并改进其功能、稳定性、隐私或安全性。[大多数插件也是开源的。](https://marketplace.rivul.us/)
- **Rust 编写**：OpenDeck 使用 Rust 和 TypeScript 构建，Rust 以其高性能、安全性和代码质量著称。

## 安装

### Linux

> [!TIP]
> 如果你使用的是 Debian、Ubuntu、Fedora、Fedora Atomic、openSUSE 或 Arch 系发行版，可以尝试自动安装脚本：
> ```bash
> curl -sSL https://raw.githubusercontent.com/nekename/OpenDeck/main/install_opendeck.sh | bash
> ```
> 该脚本会从 .deb 或 .rpm 发布文件、AUR 或 Flathub 安装 OpenDeck，并安装和重载相应的 udev 子系统规则。安装过程中还可以选择安装 Wine 和/或 Node.js。

- 从 [GitHub Releases](https://github.com/nekename/OpenDeck/releases/latest) 下载最新版本。
	- 建议避免使用 AppImage 版本，AppImage 通常存在一些问题（一般也建议避免使用 AppImage）。
	- Arch 系发行版用户可以使用 `opendeck` 和 `opendeck-bin` AUR 包获取最新发布版，以及 `opendeck-git` AUR 包获取 `main` 分支的最新提交。
- 使用你选择的包管理器安装 OpenDeck。
- 从[这里](https://raw.githubusercontent.com/OpenActionAPI/rust-elgato-streamdeck/main/40-streamdeck.rules)安装 udev 子系统规则：
	- 如果使用 `.deb` 或 `.rpm` 发布包，此文件应已自动安装。
	- 否则，下载并复制到正确位置：`sudo cp 40-streamdeck.rules /etc/udev/rules.d/`。
	- 两种情况下都需要重载 udev 规则：`sudo udevadm control --reload-rules && sudo udevadm trigger`。
- 如果需要运行未针对 Linux 编译的插件，需要安装 [Wine](https://www.winehq.org/)。部分插件可能还依赖 Wine Mono（某些发行版的 Wine 包已包含，但不一定都有）。

> [!NOTE]
> 如果 Flatpak 是你唯一的选择，OpenDeck 也[可以在 Flathub 获取](https://flathub.org/apps/me.amankhanna.opendeck)。请注意你仍然需要按上述方式安装 udev 子系统规则。要使用 Windows 和 Node.js 插件，需要在系统中原生安装 Wine 或 Node.js（不支持 Wine 和 Node.js 的 Flatpak 版本）。

### Windows

- 从 [GitHub Releases](https://github.com/nekename/OpenDeck/releases/latest) 下载最新版本（`.exe` 或 `.msi`）。
- 双击下载的文件运行安装程序。

### macOS

- 从 [GitHub Releases](https://github.com/nekename/OpenDeck/releases/latest) 下载最新版本。
- 如果下载的是 `.dmg`，打开磁盘镜像并将应用拖入"应用程序"文件夹；如果下载的是 `.tar.gz`，解压到"应用程序"文件夹。
- 打开已安装的应用。注意：如果收到"来自身份不明的开发者"的警告，*在 Finder 中右键点击应用然后选择"打开"*即可跳过警告。
- 如果需要运行仅编译了 Windows 版本的插件，需要在系统上安装 [Wine](https://www.winehq.org/)。

## 支持

### 如何操作……？

要编辑动作的设置，左键点击它以显示其*属性检查器*。要删除动作，右键点击并从上下文菜单中选择"删除"。

要编辑动作的外观，右键点击并从上下文菜单中选择"编辑"。然后可以为每个状态自定义图片和文字。左键点击图片可从文件系统选择图片，右键点击图片可重置为插件提供的默认图片。

要选择其他设备或切换配置文件，使用右上角的下拉菜单。你可以通过在配置文件名前添加文件夹名和正斜杠来将配置文件组织到文件夹中。还可以配置在特定应用窗口激活时自动切换到对应配置文件。

要更改其他选项，打开"设置"。在这里你还可以查看当前 OpenDeck 版本信息或打开配置和日志目录。要添加或移除插件，访问"插件"标签页。

### 故障排除

- 确保你运行的是最新版本的 OpenDeck，以及相关软件（如 Spotify 或 OBS）的较新版本。
- 查看 [FAQ](https://github.com/nekename/OpenDeck/wiki/0.-FAQ) 和 [GitHub Issues](https://github.com/nekename/OpenDeck/issues) 看看是否已有解决方案。
- 检查 OpenDeck 日志文件中的重要信息。提交支持请求时应附上此文件。
	- 你也可以从终端运行 OpenDeck 直接查看日志，这比找日志文件更方便，尤其是日志文件为空或缺少细节时。
	- 对于插件问题，还可以检查插件日志（位于同一文件夹，有时在插件自己的文件夹中也有 `plugin.log` 或类似文件）。
	- 日志目录可从 OpenDeck 设置页面打开，或手动定位到以下路径：
		- Linux：`~/.local/share/opendeck/logs/`
		- Flatpak：`~/.var/app/me.amankhanna.opendeck/data/opendeck/logs/`
		- Windows：`%appdata%\opendeck\logs\`
		- macOS：`~/Library/Logs/opendeck/`
- 在 Linux 或 macOS 上运行仅支持 Windows 的编译插件时，请确保系统安装了最新版本的 Wine（以及 Wine Mono）。
- 如果设备未显示，请确保你有正确的访问权限（例如在 Linux 上安装 udev 子系统规则并重启系统），且连接设备后已重启 OpenDeck。

### 支持论坛

- [Discord](https://discord.gg/26Nf8rHvaj)
- [Matrix](https://matrix.to/#/#opendeck:matrix.org)
- [GitHub Issues](https://github.com/nekename/OpenDeck/issues)

### 从源码构建 / 贡献

> [!TIP]
> [AGENTS.md](AGENTS.md) 中的代理开发指南同样适合作为代码库入门参考。

你需要确保满足[构建 Tauri 应用的所有先决条件](https://tauri.app/start/prerequisites)，并安装 [Deno](https://deno.com/)。在 Linux 上还需要安装对应发行版的 `libudev` 和 `libdbus`。运行 `deno install` 后，可以使用 `deno task tauri dev` 和 `deno task tauri build` 来开发 OpenDeck。

每次提交前，请确保完成以下步骤：
1. 使用 `cargo clippy` 检查 Rust 代码，确保无违规
2. 使用 `cargo fmt` 格式化 Rust 代码
3. 使用 `deno check` 检查 TypeScript 代码并使用 `deno lint` 检查，确保无违规
4. 使用 `deno task check` 检查 Svelte 代码，确保无违规
5. 使用 `deno fmt --unstable-component` 格式化前端代码

提交贡献时，请遵循 [Conventional Commits 规范](https://conventionalcommits.org/)编写提交信息。你还需要[签署你的提交](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits)。如有疑问欢迎在上述支持渠道寻求帮助！

OpenDeck 采用 GNU 通用公共许可证 3.0 版或更高版本授权。详情请见 LICENSE.md 文件。

## 展示

![主界面](.github/readme/mainmenu.png)
![多动作](.github/readme/multiaction.png)
![插件](.github/readme/plugins.png)
![配置文件](.github/readme/profiles.png)
