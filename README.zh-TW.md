# Fuji Recipe Manager

[English](README.md) | 繁體中文

Fuji Recipe Manager 是一套支援 macOS 與 Windows 的本機優先桌面程式，用來管理 Fujifilm 的底片模擬 Recipe。技術基礎為 Tauri 2、React、TypeScript 與 Rust。

專案以安全為先：Recipe 可隨時在本機建立、比較、匯入與匯出；相機操作則必須先通過「機型＋韌體」的分階段硬體驗證。

> **目前狀態：已接上真實硬體的 Alpha 版本。** X-M5 韌體 1.30 的實驗性 Recipe 寫入會先建立可持久保存的本機快照，只寫入已驗證欄位、逐欄讀回，並在失敗時驗證還原；其他 Fujifilm 機型仍僅能探測。

## 設計方向

工作流程參考 [Latent](https://github.com/formray/latent) 的安全設計：保留本機 Recipe 資料庫、將相機備份與讀回驗證視為寫入前提、以「機型／韌體」記錄能力矩陣，並清楚處理 macOS 的 PTP 介面占用問題。本專案是獨立的原生桌面實作，未複製 Latent 的原始碼。

## 現階段已完成

- Tauri 2 桌面殼層與可調整版面的 React Recipe 工作區。
- 桌面版使用本機 SQLite 保存 Recipe；瀏覽器開發模式則使用本機儲存作為替代。
- Recipe 建立、編輯、搜尋、標籤、收藏、JSON／FRecipe 匯入匯出，以及常見 Recipe 文字格式解析。
- 每組本機 Recipe 可選填原始作者與來源網址；網址僅作出處紀錄，不會被程式用來爬取或複製 Recipe。
- 透過 Rust 與 `nusb` 偵測 Fujifilm X Series、X100、GFX、FinePix 與尚未列出的 Fujifilm 機身。
- X-M5 的四個自訂槽位能力定義，以及真實 USB 紀錄：`04CB:030C`、韌體 `1.30`。
- 唯讀 PTP DeviceInfo 探測，且刻意不記錄或輸出相機序號。
- 可明確選擇執行的 PTP 屬性描述探測：開啟並關閉標準 session，但不選取槽位、不送出寫入指令。
- 經真實硬體驗證的 X-M5 預設名稱寫入／讀回／還原 CLI 測試；firmware 1.30 目前只接受可列印 ASCII 名稱，Unicode 名稱因實機回傳 PTP `201C` 而會在 App 送出前阻止。實驗性 App 寫入也會將自訂檔名稱（`D18D`）與 Recipe 參數放進同一筆安全交易。
- 可還原的 X-M5 C2 寫入／讀回／還原測試，已驗證動態範圍，以及高光、陰影、色彩、銳利度、清晰度等有符號欄位。
- 可還原的 X-M5 C2 測試也已驗證底片模擬、Color Chrome、Chrome FX Blue、白平衡、白平衡偏移、高 ISO 雜訊抑制與顆粒；X-M5 顆粒 Off 的命令／讀回正規化已編碼並有單元測試。
- 僅限 X-M5 的實驗性 GUI 寫入：寫入前會在 SQLite 保存自訂檔名稱（`D18D`）、Recipe 欄位備份與寫入日誌、逐欄讀回，失敗時執行可驗證的還原；每次操作後會恢復原本的作用中槽位。
- 可查看復原備份並執行明確、可驗證的 X-M5 還原操作。

## 相機安全界線

目前只有 USB ID `04CB:030C` 的 X-M5 會顯示**實驗性** Recipe 寫入操作。辨識到機型名稱或 Fujifilm USB 廠商 ID，不代表它和其他世代的相機使用相同 Recipe 屬性編碼；App 不會對其他機型開放寫入。

在開放某機型寫入前，該「機型／韌體」必須依序通過：

1. 偵測 USB／PTP 介面並記錄 DeviceInfo。
2. 讀取屬性描述資料，建立已支援欄位的能力紀錄。
3. 在送出任何欄位前，擷取目標槽位中已驗證的 Recipe 欄位，並保存成可還原的本機備份。
4. 只將 Recipe 轉換為該機型與韌體已確認的欄位。
5. 使用者明確確認後，僅在可犧牲的槽位進行寫入。
6. 讀回槽位、逐欄比對、保存寫入日誌，並提供可驗證的還原／中斷復原流程。

## 啟動桌面程式

```bash
npm install
npm run tauri dev
```

建立本機 macOS 開發版：

```bash
npm run tauri -- build
```

產物會放在 `target/release/bundle/macos/`。它尚未進行程式簽章與公證，僅適合本機開發。Windows 安裝檔應在 Windows runner 通過 USB／PTP 檢查後再建立。

## 唯讀相機探測

先在相機中選擇說明書記載的 USB／PTP 相容模式，使用直連 USB 線，並關閉 Image Capture、Photos、X RAW Studio 與所有 tethering 軟體。

```bash
# 僅列出 USB 裝置
cargo run -p fuji-test

# 標準 PTP GetDeviceInfo；不開啟 PTP session，也不讀取相機設定
cargo run -p fuji-test -- ptp-info

# 明確查詢一個屬性描述（十六進位）。它會開啟並關閉標準 PTP session，
# 但不會選取自訂槽位，也不會寫入任何設定。
cargo run -p fuji-test -- ptp-descriptor D18C

# 讀取一個屬性的原始值。未針對特定機型／韌體驗證編碼前，
# 程式只會保留原始位元組，不會把它解讀成 Recipe 設定。
cargo run -p fuji-test -- ptp-read D18C
```

`D18C` 是目前 X-M5 實驗性流程所記錄的 Fujifilm 自訂槽位選擇器識別碼。即使描述結果標示為可寫，也只表示相機能力；它不會自行解鎖程式內的寫入功能。X-M5 對這個屬性的標準描述查詢回傳通用錯誤，但直接唯讀取值可成功；這會被記錄為機型專屬的協議觀察，不是寫入許可。

## 專案結構

```text
apps/fuji-test/             唯讀 USB／PTP 驗證 CLI
crates/camera-core/         共用相機能力型別
crates/ptp-core/            標準 PTP 封包與安全描述資料解析
crates/fuji-ptp/            Fujifilm 廠商屬性識別碼
crates/usb-transport/       nusb 後端與平台 fallback 邊界
crates/camera-xm5/          X-M5 能力定義
crates/camera-fujifilm/     Fujifilm 系列／機型辨識與存取階段
src-tauri/                  原生 Tauri 殼層與本機 SQLite 資料庫
src/                        React Recipe 工作區
docs/                       硬體安全與發布文件
```

## 開發檢查

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run build
```

請參考[開發說明](docs/development.md)與[發布狀態](docs/release-status.md)，了解真實硬體驗證門檻。
各機型支援狀態與擴充規則請參考[相機能力矩陣](docs/camera-capability-matrix.md)。

## 隱私與商標

Recipe 會留在你的裝置上，除非你主動匯出。程式不會上傳相機設定或 RAF 檔案。

Fuji Recipe Manager 與 Fujifilm 無隸屬或贊助關係。Fujifilm 與各底片模擬名稱為其各自權利人的商標。
