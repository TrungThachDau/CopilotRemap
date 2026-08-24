# Copilot Remap (Rust) 🚀

**Copilot Remap** là ứng dụng native Windows 11 siêu nhẹ được viết 100% bằng **Rust**, cho phép bạn gán lại (remap) phím cứng **Copilot** hoặc tổ hợp phím `Windows + C` để mở bất kỳ ứng dụng, website, phím tắt hoặc script nào bạn muốn.

Ứng dụng đăng ký làm **Copilot Key Provider** chính quy với Windows 11 thông qua App Extension `com.microsoft.windows.copilotkeyprovider` (đóng gói MSIX tự ký), giúp xuất hiện trực tiếp trong Windows Settings mà không cần chạy ngầm tốn RAM hay quét phím liên tục.

---

## ⚡ Tính năng nổi bật

- 🦀 **100% Native Rust**: Kích thước binary chỉ **~343 KB**, toàn bộ gói MSIX chỉ **~209 KB**.
- ⚡ **Tốc độ tức thì**: Phản hồi kích hoạt phím dưới **5ms**, không trễ khởi động của runtime .NET/Electron.
- 🍃 **Không tốn RAM nền**: Khi bấm phím Copilot, Windows kích hoạt app chạy hành động rồi tự đóng ngay, **0% RAM tiêu thụ nền**.
- 🛠️ **Đa dạng hành động**:
  - 🚀 **Khởi chạy ứng dụng (.exe / .lnk / .bat)** kèm tham số và thư mục làm việc.
  - 🌐 **Mở Website / AI Assistants**: ChatGPT, Claude AI, Google Gemini, Perplexity, v.v.
  - ⌨️ **Gửi phím tắt (SendInput)**: PowerToys Run (`Alt + Space`), Snipping Tool (`Win + Shift + S`), Task Manager (`Ctrl + Shift + Esc`), v.v.
  - 📟 **Chạy lệnh Shell**: PowerShell / Command Prompt tùy chỉnh.
- 🎨 **Settings GUI trực quan**: Giao diện Win32 hiện đại, hỗ trợ preset 1-click, hộp thoại chọn file `.exe`, nút **"Test Action"** và liên kết trực tiếp tới Windows Settings.
- 📦 **Cài đặt 1-click**: Script `install.ps1` tự động trust chứng chỉ, cài MSIX và kích hoạt phím Copilot trong Registry.

---

## 📥 Cài đặt 1-Click

1. Mở PowerShell với quyền Administrator (hoặc chạy bình thường, script sẽ tự xin quyền):
   ```powershell
   .\install.ps1
   ```
2. Script sẽ:
   - Tự động thêm chứng chỉ số vào kho `TrustedPeople`
   - Cài đặt gói `CopilotRemap.msix`
   - Tự động kích hoạt CopilotRemap làm ứng dụng xử lý phím Copilot trong Windows
   - Mở cửa sổ Cài đặt để bạn chọn hành động mong muốn.

---

## ⚙️ Sử dụng & Cấu hình

Bạn có thể mở giao diện Cài đặt bất cứ lúc nào bằng cách:
- Chạy `CopilotRemap.exe` từ Start Menu hoặc thư mục dự án
- Hoặc chạy `.\target\release\CopilotRemap.exe`

### Preset có sẵn:
- 🤖 **ChatGPT (Web)**: Mở `https://chatgpt.com`
- 🧠 **Claude AI (Web)**: Mở `https://claude.ai`
- 🌐 **Google Gemini (Web)**: Mở `https://gemini.google.com`
- ⚡ **PowerToys Run**: Gửi phím `Alt + Space`
- 💻 **Windows Terminal**: Chạy `wt.exe`
- 📸 **Snipping Tool**: Gửi phím `Win + Shift + S`
- ⚙️ **Task Manager**: Mở `Taskmgr.exe`
- 📝 **Notepad**: Mở `notepad.exe`
- 🛠️ **Custom Application**: Chọn bất kỳ phần mềm nào trên máy.

File cấu hình được lưu tại: `%LOCALAPPDATA%\CopilotRemap\config.json`.

---

## 🔨 Hướng dẫn Build từ Source

Yêu cầu:
- Rust toolchain (`rustup` / `cargo`)
- Visual Studio Build Tools (C++ Workload & Windows 10/11 SDK)

Chạy script build:
```powershell
.\build.ps1
```
Kết quả gói MSIX đã ký sẽ nằm tại: `target\CopilotRemap.msix`.

---

## 🗑️ Gỡ cài đặt

Chạy script:
```powershell
.\uninstall.ps1
```
