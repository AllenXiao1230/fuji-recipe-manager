# Fuji Recipe Manager

[English](README.md) | 繁體中文

Fuji Recipe Manager 是一套支援 macOS 與 Windows 的本機優先桌面程式，用來管理 Fujifilm 的軟片模擬配方（Recipe）。技術基礎為 Tauri 2、React、TypeScript 與 Rust。

專案以安全為先：Recipe 可隨時在本機建立、比較、匯入與匯出；相機操作則必須先通過「機型＋韌體」的分階段硬體驗證。

> **目前狀態：已接上真實硬體的 Alpha 版本。** X-M5 韌體 1.30 的實驗性 Recipe 寫入會先建立可持久保存的本機快照，只寫入已驗證欄位、逐欄讀回，並在失敗時驗證還原；其他 Fujifilm 機型仍僅能探測。

## 設計方向

工作流程參考 [Latent](https://github.com/formray/latent) 的安全設計：保留本機 Recipe 資料庫、將相機備份與讀回驗證視為寫入前提、以「機型／韌體」記錄能力矩陣，並清楚處理 macOS 的 PTP 介面占用問題。本專案是獨立的原生桌面實作，未複製 Latent 的原始碼。

## 現階段已完成

- Tauri 2 桌面殼層與可調整版面的 React Recipe 工作區。
- 桌面版以本機 SQLite 作為 Recipe 資料庫唯一權威；瀏覽器開發模式才使用本機儲存。資料庫含 migration、WAL、樂觀版本與刪除 tombstone，可避免延遲儲存覆寫較新的資料。
- 本機 SQLite 能力矩陣資料庫會依 Fujifilm USB ID、機型與韌體保存可設定的欄位範圍、觀察到的 PTP 代碼、來源類型、安全出處網址、證據摘要與驗證時間，但不會授予相機寫入權限。
- 「相機能力總覽」會分開顯示可辨識的 Fujifilm 機型、受信任的機型／韌體記錄與本機研究矩陣；僅辨識到機型時仍維持唯讀探測。
- Recipe 建立、編輯、搜尋、標籤、收藏、JSON／FRecipe 匯入匯出，以及常見 Recipe 文字格式解析。
- Recipe 資料庫提供可組合篩選：軟片模擬、標籤（含未分類）、收藏、C1–C4 指派狀態與相機相容性；搜尋亦會比對 Recipe 名稱、描述、標籤、來源作者與相容機型。
- 桌面版的 Recipe 資料庫欄（搜尋、篩選與卡片）與 Recipe 編輯器採獨立捲動；較長的編輯內容不會擠走資料庫控制項。
- 使用自訂極簡「相機＋Recipe 卡片」App 圖示，已生成 macOS `.icns`、Windows `.ico` 與各平台 PNG 資產，並由 Tauri 明確納入安裝包。
- 每組本機 Recipe 可選填原始作者與來源網址；網址僅作出處紀錄，不會被程式用來爬取或複製 Recipe。
- 透過 Rust 與 `nusb` 偵測 Fujifilm X Series、X100、GFX、FinePix 與尚未列出的 Fujifilm 機身。
- X-M5 的四個自訂槽位能力定義，以及真實 USB 紀錄：`04CB:030C`、韌體 `1.30`。
- 唯讀 PTP DeviceInfo 探測，且刻意不記錄或輸出相機序號。
- 可明確選擇執行的 PTP 屬性描述探測：開啟並關閉標準 session，但不選取槽位、不送出寫入指令。
- 經真實硬體驗證的 X-M5 預設名稱寫入／讀回／還原 CLI 測試；firmware 1.30 目前只接受可列印 ASCII 名稱，Unicode 名稱因實機回傳 PTP `201C` 而會在 App 送出前阻止。實驗性 App 寫入也會將自訂檔名稱（`D18D`）與 Recipe 參數放進同一筆安全交易。
- 可還原的 X-M5 C4 寫入／讀回／還原測試，已涵蓋目前所有可寫入的 Recipe 欄位；列舉型選項逐一測試，數值欄位測試最小值、0 與最大值。
- X-M5 顆粒 Off 會先寫入保留的「小／大」顆粒大小，再送出 `01 00` 關閉命令；相機分別讀回 `06 00`（小）或 `07 00`（大），此復原順序已編碼並有單元測試。
- X-M5 firmware 1.30 對低於 `-2.0` 的高光與陰影會回覆 PTP `201C`，因此兩者在程式內限制為 `-2.0` 至 `+4.0`、步進 `0.5`。
- 僅限 X-M5 的實驗性 GUI 寫入：UI 必須先驗證已連接 DeviceInfo 的機型與韌體，才會保存自訂檔名稱（`D18D`）、Recipe 欄位備份與寫入日誌、逐欄讀回；失敗時執行可驗證的還原，每次操作後會恢復原本的作用中槽位。
- 可選的每組 Recipe 原始 C 槽保存：可讀取的 X-M5 自訂槽 PTP 原始位元組（包含相機拒絕寫入的欄位）可作為稽核／互通資料保存；它們是嚴格唯讀 metadata，絕不會被相機寫入或復原流程重送。
- 啟動時可偵測未完成寫入日誌、查看復原備份並執行明確、可驗證的 X-M5 還原操作；只有手動還原也讀回成功後，日誌才會被標記為已回復。
- 版本化且內建的能力紀錄位於 `data/capabilities/fujifilm-xm5-1.30.json`；匯入視窗會列出所有欄位的「已驗證可寫入／已探測尚未驗證／相機拒絕（`201C`）／未知或禁止寫入」狀態。
- `fuji-test ptp-audit-xm5-unverified` 固定範圍的唯讀稽核指令，會收集保留欄位 `D191`／`D1A5` 與未知廠商屬性的原始值及 descriptor metadata；不選擇槽位，也不會寫入相機。
- X-M5 1.30 額外完成平滑膚色效果、sRGB／Adobe RGB、色溫（白平衡為「色溫」時）與已特別驗證的 `S 3:2`／`S 16:9`／`S 1:1`／`M 3:2`／`M 16:9`／`M 1:1`／`L 3:2`／`L 16:9`／`L 1:1`／`RAW`／`FINE`／`NORMAL`／`FINE+RAW`／`NORMAL+RAW` 影像 payload 寫入／讀回／還原支援。
- 所有 20 種非 Auto 軟片模擬、動態範圍 Auto、白平衡白色優先／氛圍優先也已個別通過 X-M5 C4 寫入／讀回／還原驗證；軟片模擬 Auto 與白平衡 Custom 1–3 仍維持鎖定。
- 長時間曝光降噪與黑白暖冷／洋紅綠色調因實機回覆 `201C` 維持鎖定；已探測到的全域屬性也維持唯讀。
- 支援 X RAW Studio `.FP1`、`.FP2`、`.FP3` 設定檔匯入／匯出；能安全保留的未映射 XML 會跟隨本機 Recipe 保存，序號資料會被移除，並明確提示未映射欄位。
- 桌面版可由 JPEG／RAF metadata 建立本機 Recipe，並標記每個欄位為「已辨識／無法判定」。開發環境以 ExifTool 讀取 Fujifilm MakerNote；正式發行前必須在每個平台隨程式封裝並驗證該 metadata adapter。
- RAF 預覽目前提供桌面版離線前置檢查，以及 Rust 的「上傳 → 暫套 Recipe → 轉檔 → 下載 JPEG → 清理」可還原交易邊界。尚未有相機傳輸 adapter，因此前置檢查絕不開啟相機，也不會宣稱已轉出 JPEG。

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

需要 Node.js 22.12 以上、Rust stable 與 Tauri 對應平台的建置前置條件。

```bash
npm install
npm run tauri dev
```

建立僅含本機 macOS App 的開發版（建議用於本機交付測試）：

```bash
npm run tauri -- build --bundles app
```

產物為 `target/release/bundle/macos/Fuji Recipe Manager.app`。它尚未進行程式簽章與公證，僅適合本機開發。Windows 安裝檔應在 Windows runner 通過 USB／PTP 檢查後再建立。

未簽章的本機 App 可能需要在 macOS 的 Gatekeeper 中明確選擇「打開」。若要一般發行，仍須建立已簽章與已公證的 DMG。

### 建立 Windows `.exe`

在 Windows 執行 [`build-windows-exe.bat`](build-windows-exe.bat)。它需要 Node.js 22.12 以上、npm 與 Rust，會以 lockfile 安裝相依套件、執行 TypeScript 檢查與前端測試，最後以 Tauri 建立 `target\\release\\Fuji Recipe Manager.exe`。請先安裝 Rust MSVC toolchain、Microsoft C++ Build Tools 與 WebView2 Runtime；產出的 EXE 尚未簽章。

## Recipe 資料庫操作

篩選條件可以疊加使用。例如同時選擇**經典負片**、**收藏**與**未指派**，可找出已收藏、使用經典負片且尚未指派給 C1–C4 的 Recipe。按下**清除篩選**即可回到完整的本機資料庫。

在桌面版中，左側 Recipe 資料庫欄與右側 Recipe 編輯器各自捲動。搜尋與篩選控制項會隨資料庫保留，瀏覽或編輯很長的 Recipe 細節時不會遺失。

## 唯讀相機探測

先在相機中選擇說明書記載的 USB／PTP 相容模式，使用直連 USB 線，並關閉 Image Capture、Photos、X RAW Studio 與所有 tethering 軟體。

```bash
# 僅列出 USB 裝置
cargo run -p fuji-test

# 標準 PTP GetDeviceInfo；不開啟 PTP session，也不讀取相機設定
cargo run -p fuji-test -- ptp-info

# 任意 Fujifilm 機型的唯讀掃描：只記錄身分與標準可讀屬性值；
# 不會選取 C 槽，也不會寫入任何設定。
cargo run -p fuji-test -- ptp-scan-readonly

# 明確查詢一個屬性描述（十六進位）。它會開啟並關閉標準 PTP session，
# 但不會選取自訂槽位，也不會寫入任何設定。
cargo run -p fuji-test -- ptp-descriptor D18C

# 讀取一個屬性的原始值。未針對特定機型／韌體驗證編碼前，
# 程式只會保留原始位元組，不會把它解讀成 Recipe 設定。
cargo run -p fuji-test -- ptp-read D18C

# 對已在可拋棄 C 槽中手動選定的影像尺寸／品質值進行受控候選讀取。
# 它只讀取一個原始數值並還原原本作用中的槽位，不會寫入候選數值。
cargo run -p fuji-test -- ptp-capture-xm5-image-payload C4 image-size L_16_9

# 取得候選 raw 值後才進行可還原驗證；raw 參數使用十六進位。
# 只有顯示 PASS 且恢復作用中槽位後，該選項才能解鎖。
cargo run -p fuji-test -- ptp-verify-xm5-image-payload C4 image-size L_16_9 0x0008

# 在可承受測試的槽位執行實機驗證：逐項寫入／讀回／還原，最後恢復原先作用中槽位
cargo run -p fuji-test -- ptp-verify-xm5-recipe C4
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
data/capabilities/          依 USB ID＋機型＋韌體保存的版本化能力紀錄
crates/camera-fujifilm/     Fujifilm 系列／機型辨識與存取階段
crates/raf-preview/         可還原的 RAF 預覽交易邊界（尚無相機 adapter）
assets/branding/            App 圖示的來源設計素材
build-windows-exe.bat       Windows EXE 建置輔助腳本
src-tauri/                  原生 Tauri 殼層與本機 SQLite 資料庫
src-tauri/icons/            已生成的 macOS、Windows 與各平台圖示資產
src/                        React Recipe 工作區
src/CapabilityMatrixPanel.tsx  本機相機能力矩陣資料庫 UI
docs/                       硬體安全與發布文件
```

## 開發檢查

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run test
npm run build
```

請參考[開發說明](docs/development.md)與[發布狀態](docs/release-status.md)，了解真實硬體驗證門檻。
各機型支援狀態與擴充規則請參考[相機能力矩陣](docs/camera-capability-matrix.md)。

## 隱私與商標

Recipe 會留在你的裝置上，除非你主動匯出。程式不會上傳相機設定或 RAF 檔案。

Fuji Recipe Manager 與 Fujifilm 無隸屬或贊助關係。Fujifilm 與各底片模擬名稱為其各自權利人的商標。
