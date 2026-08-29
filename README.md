# Copilot Remap (Rust) 🚀

**Copilot Remap** is an ultra-lightweight native Windows 11 application written 100% in **Rust** that allows you to remap the hardware **Copilot key** or the `Windows + C` shortcut to launch any application, website, shortcut, or script you want.

The app registers as an official **Copilot Key Provider** with Windows 11 via the `com.microsoft.windows.copilotkeyprovider` App Extension (self-signed MSIX package), making it appear directly in Windows Settings without requiring background processes that consume RAM or continuous key polling.

---

## ⚡ Highlights & Features

- 🦀 **100% Native Rust**: Binary size is only **~343 KB**, and the entire MSIX package is only **~209 KB**.
- ⚡ **Instant Response**: Key trigger response latency under **5ms**, with no .NET/Electron runtime startup delays.
- 🍃 **Zero Background RAM**: When the Copilot key is pressed, Windows triggers the app, executes the action, and exits immediately — **0% background RAM usage**.
- 🛠️ **Versatile Actions**:
  - 🚀 **Launch Application (.exe / .lnk / .bat)** with custom arguments and working directory.
  - 🌐 **Open Websites / AI Assistants**: ChatGPT, Claude AI, Google Gemini, Perplexity, etc.
  - ⌨️ **Send Shortcut Keys (SendInput)**: PowerToys Run (`Alt + Space`), Snipping Tool (`Win + Shift + S`), Task Manager (`Ctrl + Shift + Esc`), etc.
  - 📜 **Execute Shell Commands**: Custom PowerShell / Command Prompt commands.
- 🎨 **Intuitive Settings GUI**: Modern Win32 interface with 1-click presets, `.exe` file picker, **"Test Action"** button, and direct link to Windows Settings.
- 📦 **1-Click Installation**: The `install.ps1` script automatically trusts the certificate, installs the MSIX package, and activates the Copilot key in the Registry.

---

## 📥 1-Click Installation

1. Open PowerShell as Administrator (or run normally; the script will request elevation):
   ```powershell
   .\install.ps1
   ```
2. The script will:
   - Automatically install the digital certificate into the `TrustedPeople` store
   - Install the `CopilotRemap.msix` package
   - Automatically configure CopilotRemap as the Copilot key handler in Windows
   - Open the Settings window for you to choose your desired action.

---

## ⚙️ Usage & Configuration

You can open the Settings interface at any time by:
- Launching `CopilotRemap.exe` from the Start Menu or project directory
- Or running `.\target\release\CopilotRemap.exe`

### Built-in Presets:
- 🤖 **ChatGPT (Web)**: Opens `https://chatgpt.com`
- 🧠 **Claude AI (Web)**: Opens `https://claude.ai`
- 🌐 **Google Gemini (Web)**: Opens `https://gemini.google.com`
- ⚡ **PowerToys Run**: Sends `Alt + Space`
- 💻 **Windows Terminal**: Launches `wt.exe`
- 📸 **Snipping Tool**: Sends `Win + Shift + S`
- ⚙️ **Task Manager**: Opens `Taskmgr.exe`
- 📝 **Notepad**: Opens `notepad.exe`
- 🛠️ **Custom Application**: Select any executable on your system.

The configuration file is stored at: `%LOCALAPPDATA%\CopilotRemap\config.json`.

---

## 🔨 Build from Source

Requirements:
- Rust toolchain (`rustup` / `cargo`)
- Visual Studio Build Tools (C++ Workload & Windows 10/11 SDK)

Run the build script:
```powershell
.\build.ps1
```
The signed MSIX package will be output to: `target\CopilotRemap.msix`.

---

## 🗑️ Uninstallation

Run the uninstall script:
```powershell
.\uninstall.ps1
```
