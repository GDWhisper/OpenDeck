**English | [中文文档](README_zh.md)**

> [!CAUTION]
> **This is a Windows-focused fork** of [nekename/OpenDeck](https://github.com/nekename/OpenDeck).
> The upstream maintainer prefers universal, cross-platform solutions; the changes here are pragmatic
> Windows-specific workarounds that were not accepted upstream.
> They work on my machine but may not be suitable for general use.
>
> **What's different from upstream:**
>
> - **System sleep/wake detection** (`system_sleep_watchdog.rs`) — polls wall-clock time at 1 Hz;
>   if the gap between consecutive polls exceeds 3 seconds, assumes the system slept. After a 2-second
>   USB stabilization delay, all devices are re-enumerated and idle-slept devices are woken.
> - **WebView plugin recovery after wake** (`reload_webview_plugins()` in `plugins/mod.rs`) — uses
>   Tauri's native `window.reload()` (via WebView2 ICoreWebView2 controller, bypassing the potentially
>   degraded JS engine after sleep), then re-evaluates the connection init JS through a hidden iframe
>   that restores native `setInterval`/`setTimeout` (Elgato SDK's `timers.js` Web Worker dies during
>   sleep, killing all plugin timers).
> - **`systemDidWakeUp` event broadcast** (`events/outbound/misc.rs`) — a new outbound event sent to
>   all connected plugins after wake, allowing native plugins to reinitialize device connections.
> - **i18n multi-language UI** — built-in internationalization framework for the frontend (Svelte components) and
>   starterpack plugin property inspector pages. Chinese is the first supported language; more can be added by simply
>   creating new locale files. Switchable in Settings → Language.
>
> Everything else (architecture, UI, plugin SDK compatibility) is identical to upstream.

# OpenDeck

Linux software for your Elgato Stream Deck

![Main menu](.github/readme/mainmenu.png)
[More screenshots](#showcase)

OpenDeck is a desktop application for using stream controller devices like the Elgato Stream Deck on Linux, Windows, and macOS. OpenDeck supports plugins made for the original Stream Deck SDK, allowing many plugins made for the Elgato software ecosystem to be used, or the [OpenAction](https://openaction.amankhanna.me/) API.

Only Elgato hardware is officially supported, but plugins are available for support for other hardware vendors.

> [!TIP]
> No Stream Deck in front of you? Use OpenDeck with [Tacto](https://tacto.live/) to turn any smartphone into one!

Special thanks go to the developers of [Tauri](https://github.com/tauri-apps/tauri), the [elgato-streamdeck](https://github.com/OpenActionAPI/rust-elgato-streamdeck) Rust library, and [Phosphor Icons](https://phosphoricons.com/).

### Why use OpenDeck?

- **Stream Deck plugins**: OpenDeck supports the majority of the Stream Deck plugins that users of the Elgato ecosystem are already familiar with, unlike other third-party softwares which are much more limited (e.g. streamdeck-ui, StreamController, Boatswain etc).
- **Cross-platform**: OpenDeck supports Linux alongside Windows and macOS. macOS users also benefit from switching from the first-party Elgato software as OpenDeck can run plugins only built for Windows on Linux and macOS thanks to Wine. Additionally, profile files are easily moveable between platforms with no changes to them necessary.
- **Feature-packed**: From Multi Actions and Toggle Actions to switching profiles when you switch apps and brightness control, OpenDeck has all the features you'd expect from stream controller software.
- **Open source**: OpenDeck source code is licensed under the GNU General Public License, allowing anyone to view it and improve it for feature, stability, privacy or security reasons. [Most plugins are open-source, too.](https://marketplace.rivul.us/)
- **Written in Rust**: The Rust programming language, which OpenDeck is built with alongside TypeScript, is known for its performance, safety and resulting code quality.

## Installation

### Windows

- Download the latest release (`.exe` or `.msi`) from [GitHub Releases](https://github.com/GDWhisper/OpenDeck-Win/releases/latest).
- Double-click the downloaded file to run the installer.

### Linux & macOS

This fork focuses on Windows-specific optimizations (system sleep/wake recovery, WebView plugin resilience, etc.) and does not include targeted improvements for Linux or macOS. Users of these platforms should use the [upstream nekename/OpenDeck](https://github.com/nekename/OpenDeck) releases, which provide full cross-platform support including AUR packages, Flatpak, udev rules, and `.dmg` installers.

## Support

### How do I...?

To edit an action's settings, left-click on it to display its *property inspector*. To remove an action, right-click on it and choose "Delete" from the context menu.

To edit an action's appearance, right-click on it and select "Edit" from the context menu. You can then customise the image and text for each of its states. Left-click on the image to choose an image from your filesystem or right-click on the image to reset it to the plugin-provided default.

To select another device, or to switch profiles, use the dropdowns in the top right corner. You can organise profiles into folders by prefixing the profile name with the folder name and a forward slash. You can also configure automatically switching to a profile when a specific application's window is active.

To change other options, open Settings. From here, you can also view information about your version of OpenDeck or open the configuration and log directories. To add or remove plugins, visit the Plugins tab.

### Troubleshooting

- Ensure you are running the latest version of OpenDeck, as well as recent versions of related software (e.g. Spotify or OBS).
- Check the [FAQ](https://github.com/GDWhisper/OpenDeck-Win/wiki/0.-FAQ) and [GitHub Issues](https://github.com/GDWhisper/OpenDeck-Win/issues) to see if there's a fix for your problem already.
- Check the OpenDeck log file for any important messages. This file should be included with any support request.
	- You can also run OpenDeck from the terminal to see the logs directly if it's easier than finding the log file or if the log file is empty or missing details.
	- For issues with plugins, you can also check the plugin's logs (in the same folder, sometimes as well as a file named `plugin.log` or similar in the plugin's own folder).
	- The log directory can be opened from the settings page of OpenDeck, or alternatively located manually at: `%appdata%\opendeck\logs\`

### Support forums

- [Discord](https://discord.gg/26Nf8rHvaj)
- [Matrix](https://matrix.to/#/#opendeck:matrix.org)
- [GitHub Issues](https://github.com/GDWhisper/OpenDeck-Win/issues)

### Building from source / contributing

> [!TIP]
> The development guide for agents present in [AGENTS.md](AGENTS.md) also serves as a useful introduction to the codebase for humans.

You'll need to ensure that all of the [prerequisites for building a Tauri application](https://tauri.app/start/prerequisites) are satisfied to build OpenDeck, as well as making sure that [Deno](https://deno.com/) is installed. On Linux, you'll also need `libudev` and `libdbus` installed for your distribution. After running `deno install`, you can use `deno task tauri dev` and `deno task tauri build` to work with OpenDeck.

Before each commit, please ensure that all of the following are completed:
1. Rust code has been linted using `cargo clippy` and it discovers no violations
2. Rust code has been formatted using `cargo fmt`
3. TypeScript code has been checked using `deno check` and linted using `deno lint` and they discover no violations
4. Svelte code has been linted using `deno task check` and it discovers no violations
5. Frontend code has been formatted using `deno fmt --unstable-component`

When submitting contributions, please adhere to the [Conventional Commits specification](https://conventionalcommits.org/) for commit messages. You will also need to [sign your commits](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits). Feel free to reach out on the support channels above for guidance when contributing!

OpenDeck is licensed under the GNU General Public License version 3.0 or later. For more details, see the LICENSE.md file.

## Showcase

![Main menu](.github/readme/mainmenu.png)
![Multi action](.github/readme/multiaction.png)
![Plugins](.github/readme/plugins.png)
![Profiles](.github/readme/profiles.png)
