# Daedalus — Open-Source AI-Assisted ECU Tuning Platform

## Кодовое имя: `daedalus`

---

## ЧАСТЬ 1: ХАРДВАРНЫЙ СТЕК И СИГНАЛЬНАЯ ЦЕЛОСТНОСТЬ

### 1.1 Почему USB может быть проблемой

USB 2.0 добавляет **джиттер 1–5 мс** на уровне хост-контроллера (особенно на хабах).
Для CAN-шины при скорости 500 kbps длина бита = 2 мкс — USB-латенция не критична
для потоковой передачи, но **критична для тайминга Security Access** (seed/key) и
**Boot Mode activation** (замыкание пинов с точным таймингом).

**Решение — не в замене USB, а в правильной архитектуре:**

```
[ECU] ←CAN/K-Line→ [Микроконтроллер STM32 с real-time firmware]
                              ↕ USB (только data transfer, не timing)
                         [Ноутбук / PC]
```

STM32 держит все real-time тайминги локально; ПК получает уже готовые пакеты.
Это именно то, что делают candleLight, CANable, Macchina M2.

### 1.2 Рекомендуемый хардварный комплект

#### Tier 1: Минимальный старт (~$80–120)

```
┌─────────────────────────────────────────────────────────┐
│  CANable 2.0 (CAN-FD, $35)                             │
│  + USB isolator (ADUM3160, $8–12)                       │
│  + OBD2 breakout cable ($10–15)                         │
│  + Raspberry Pi 4/5 (опционально, $45–75)               │
│  + BDM Frame LED + probes ($25–40)                      │
└─────────────────────────────────────────────────────────┘
```

#### Tier 2: Профессиональный (~$250–400)

```
┌─────────────────────────────────────────────────────────┐
│  Macchina M2 ($79) — 2×CAN, 2×LIN/K-Line, J1850       │
│  + candleLight FD (€35) — второй CAN канал              │
│  + USB isolator гальванический (ADUM4160, $15)          │
│  + Raspberry Pi 5 + Waveshare 2-CH CAN FD HAT ($60)    │
│  + BDM Frame металл + 22 адаптера ($40–60)             │
│  + Bench power supply 12V/5A regulated ($30–50)         │
│  + EMI ferrite clamp-on filters ×4 ($8)                 │
│  + Twisted pair CAN cable ($10)                         │
└─────────────────────────────────────────────────────────┘
```

#### Tier 3: Полный стенд (~$500–800)

```
Всё из Tier 2 плюс:
┌─────────────────────────────────────────────────────────┐
│  Saleae Logic Pro 8 (или клон DSLogic Plus, $70–150)    │
│  — для отладки CAN/K-Line/SPI на уровне битов          │
│  + Olimexino-STM32F3 ($20) с OpenBLT для dev/test      │
│  + FTDI FT2232H breakout ($15) — K-Line + JTAG         │
│  + Промышленный USB хаб с изоляцией (Advantech, $80)    │
│  + EMI line filter для bench supply (Schaffner, $20)    │
│  + ESD protection board ($10)                           │
└─────────────────────────────────────────────────────────┘
```

### 1.3 Фильтрация сигналов и питания

#### Сетевое питание (bench supply → ECU)
```
[220V] → [EMI фильтр Schaffner FN2060] → [Bench PSU]
         → [LC фильтр: 100µH + 1000µF low-ESR]
         → [TVS диод P6KE18CA на выходе]
         → [ECU 12V input]

Зачем: ECU чувствителен к пульсациям >50mV на линии питания.
       Ripple от дешёвых БП может вызвать ложные ошибки коммуникации.
```

#### CAN-шина
```
[ECU CAN_H/CAN_L] → [120Ω терминатор если bench]
                   → [Common-mode choke WE-CNSW 744232601]
                   → [ESD: PESD2CAN (NXP)]
                   → [CAN transceiver на адаптере]

Зачем: На столе без штатного жгута проводов нет штатных
       фильтров автомобиля. Common-mode помехи от ноутбука
       через ground loop — основная причина CRC-ошибок.
```

#### USB изоляция
```
[Ноутбук USB] → [ADUM4160 гальванический изолятор]
              → [CAN адаптер]

Зачем: Разрыв ground loop между ноутбуком (заземлён через
       зарядку) и ECU (питание от bench supply). Без этого
       возможен common-mode noise 100–500mV на CAN.
```

### 1.4 Нужен ли специальный ноутбук?

**Нет.** Специальные разъёмы не нужны. Вот почему:

- CAN/K-Line/JTAG работают через внешние адаптеры (USB→CAN и т.д.)
- Весь real-time timing — на микроконтроллере адаптера
- USB 2.0 достаточен для пропускной способности CAN (до 1 Мбит/с)

**Что реально важно в ноутбуке:**
1. **Минимум 2 USB-A порта** (или хаб) — один для CAN, один для логического анализатора
2. **SSD, не HDD** — для быстрой работы с бинарниками и Ghidra
3. **16+ GB RAM** — Ghidra + LLM inference ест память
4. **Linux-совместимость** — SocketCAN, kernel CAN drivers
5. **RS-232 через USB-FTDI** — для K-Line/L-Line если нужны старые ECU

Твой текущий ПК (RTX 5080, 64 GB DDR5, AMD) — **идеален** для этого.
RTX 5080 позволяет запускать локальные ML-модели (llama.cpp, ONNX Runtime).

---

## ЧАСТЬ 2: АРХИТЕКТУРА ПРОГРАММЫ

### 2.1 Высокоуровневая архитектура

```
┌──────────────────────────────────────────────────────────────┐
│                    DAEDALUS                               │
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │   Frontend   │  │   Backend    │  │   AI Engine          │ │
│  │   (Tauri +   │  │   (Rust +    │  │   (Python +          │ │
│  │    React)    │←→│    Python)   │←→│    Claude API)       │ │
│  └──────┬──────┘  └──────┬───────┘  └──────────┬──────────┘ │
│         │                │                      │             │
│  ┌──────┴──────────────┴──────────────────────┴───────────┐ │
│  │                    Hardware Abstraction Layer            │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │ │
│  │  │SocketCAN │ │ J2534    │ │ K-Line   │ │ BDM/JTAG  │  │ │
│  │  │ driver   │ │ passthru │ │ serial   │ │ (OpenOCD) │  │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 Почему Tauri + React, а не Electron

| Критерий | Electron | Tauri |
|----------|----------|-------|
| Размер билда | 150–300 MB | 5–15 MB |
| RAM usage | 300+ MB | 30–80 MB |
| Backend | Node.js | Rust (native speed) |
| Доступ к serial/USB | Через node-serialport | Нативный через Rust |
| SocketCAN (Linux) | Через node addon | Нативный socketcan-rs |
| Cross-platform | Win/Mac/Linux | Win/Mac/Linux |
| Security | Chromium sandbox | Строже, нет Node в renderer |

Tauri + Rust backend = **прямой доступ к CAN/serial без overhead**.

### 2.3 Модульная структура

```
daedalus/
│
├── CLAUDE.md                    # Инструкции для Claude Code
├── README.md
├── LICENSE                      # GPLv3
├── Cargo.toml                   # Rust workspace
├── pyproject.toml               # Python AI/analysis components
│
├── docs/
│   ├── architecture.md
│   ├── hardware-setup.md
│   ├── protocol-specs/
│   │   ├── uds-iso14229.md
│   │   ├── kwp2000-iso14230.md
│   │   ├── can-iso11898.md
│   │   └── isotp-iso15765.md
│   └── ecu-profiles/
│       ├── bosch-med17.md
│       ├── bosch-edc17.md
│       ├── siemens-simos18.md
│       └── template.md
│
├── crates/                      # Rust crates (backend)
│   │
│   ├── forge-core/              # Ядро: типы данных, конфиг
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs        # Конфигурация приложения
│   │       ├── error.rs         # Единая система ошибок
│   │       ├── types.rs         # ECU types, протоколы, адреса
│   │       └── project.rs       # Проект: файлы, история, undo
│   │
│   ├── forge-hal/               # Hardware Abstraction Layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── can/
│   │       │   ├── mod.rs
│   │       │   ├── socketcan.rs     # Linux SocketCAN
│   │       │   ├── slcan.rs         # Serial Line CAN
│   │       │   ├── gs_usb.rs        # candleLight/CANable
│   │       │   └── virtual_can.rs   # Для тестов без железа
│   │       ├── kline/
│   │       │   ├── mod.rs
│   │       │   ├── ftdi.rs          # FT232/FT2232 через libftdi
│   │       │   └── serial.rs        # Обычный serial port
│   │       ├── j2534/
│   │       │   ├── mod.rs
│   │       │   └── passthru.rs      # J2534 DLL wrapper
│   │       ├── traits.rs            # Trait: TransportLayer
│   │       └── discovery.rs         # Автодетект подключённых адаптеров
│   │
│   ├── forge-proto/             # Протоколы: UDS, KWP2000, ISO-TP
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── isotp.rs             # ISO 15765-2 Transport Protocol
│   │       ├── uds/
│   │       │   ├── mod.rs
│   │       │   ├── services.rs      # DiagnosticSessionControl, SecurityAccess...
│   │       │   ├── dtc.rs           # ReadDTCInformation
│   │       │   ├── upload.rs        # RequestUpload/TransferData
│   │       │   ├── download.rs      # RequestDownload
│   │       │   └── routine.rs       # RoutineControl
│   │       ├── kwp2000/
│   │       │   ├── mod.rs
│   │       │   └── services.rs
│   │       ├── obd2/
│   │       │   ├── mod.rs
│   │       │   └── pids.rs          # Стандартные PID
│   │       └── seed_key/
│   │           ├── mod.rs
│   │           ├── algorithms.rs    # Известные алгоритмы seed/key
│   │           └── bruteforce.rs    # Перебор (с safety limits)
│   │
│   ├── forge-flash/             # Чтение/запись прошивок
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── reader.rs            # Unified read interface
│   │       ├── writer.rs            # Unified write interface
│   │       ├── strategies/
│   │       │   ├── mod.rs
│   │       │   ├── obd_flash.rs     # Flash через OBD2 (UDS)
│   │       │   ├── bench_flash.rs   # Flash на столе (Bench/Boot)
│   │       │   ├── bdm.rs           # BDM через OpenOCD
│   │       │   └── tricore_bsl.rs   # TC1791/1796 CAN Bootstrap
│   │       ├── checksum/
│   │       │   ├── mod.rs
│   │       │   ├── crc32.rs
│   │       │   ├── bosch_me7.rs     # ME7 multipoint checksum
│   │       │   ├── bosch_med17.rs   # MED17 CRC + RSA
│   │       │   ├── siemens.rs       # Siemens/Continental
│   │       │   └── auto_detect.rs   # Автоопределение алгоритма
│   │       ├── compression/
│   │       │   ├── mod.rs
│   │       │   └── lzss.rs          # LZSS для Simos
│   │       └── encryption/
│   │           ├── mod.rs
│   │           └── aes_simos.rs     # AES для Simos18
│   │
│   ├── forge-binary/            # Анализ бинарных файлов прошивок
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser/
│   │       │   ├── mod.rs
│   │       │   ├── intel_hex.rs     # Intel HEX формат
│   │       │   ├── motorola_s.rs    # Motorola S-Record
│   │       │   ├── raw_bin.rs       # Raw binary
│   │       │   └── odx_container.rs # VW ODX/PDX контейнеры
│   │       ├── identify/
│   │       │   ├── mod.rs
│   │       │   ├── ecu_detect.rs    # Определение типа ECU по сигнатурам
│   │       │   ├── signatures.rs    # База сигнатур (Bosch, Siemens...)
│   │       │   └── memory_map.rs    # Определение layout Flash
│   │       ├── maps/
│   │       │   ├── mod.rs
│   │       │   ├── finder.rs        # Автоматический поиск карт (ML)
│   │       │   ├── axis_detect.rs   # Определение осей (RPM, load...)
│   │       │   ├── types.rs         # Map1D, Map2D, Map3D, Scalar
│   │       │   ├── export.rs        # Экспорт в A2L, XDF
│   │       │   └── import.rs        # Импорт A2L, XDF, Damos, .kp
│   │       ├── diff.rs              # Сравнение двух прошивок
│   │       ├── patch.rs             # Применение патчей
│   │       └── hex_view.rs          # Hex view с аннотациями
│   │
│   ├── forge-dtc/               # Диагностические коды
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── database.rs          # База OBD2 + manufacturer DTC
│   │       ├── reader.rs            # Чтение DTC из ECU
│   │       ├── clear.rs             # Сброс DTC
│   │       ├── filter.rs            # Фильтрация и поиск
│   │       └── freeze_frame.rs      # Freeze frame data
│   │
│   ├── forge-live/              # Real-time данные
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── logger.rs            # Data logging
│   │       ├── gauges.rs            # Данные для виртуальных приборов
│   │       ├── recorder.rs          # Запись сессий
│   │       └── export.rs            # CSV, MDF4 экспорт
│   │
│   └── forge-app/               # Tauri application shell
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands/            # Tauri IPC commands
│       │   │   ├── mod.rs
│       │   │   ├── connection.rs    # connect, disconnect, scan
│       │   │   ├── flash.rs         # read, write, verify
│       │   │   ├── dtc.rs           # read_dtc, clear_dtc
│       │   │   ├── live.rs          # start_logging, get_pids
│       │   │   ├── binary.rs        # open_file, analyze, find_maps
│       │   │   ├── ai.rs            # classify_map, suggest_mod
│       │   │   └── project.rs       # save, load, undo, redo
│       │   ├── state.rs             # AppState (connection, project)
│       │   └── events.rs            # Tauri events (progress, logs)
│       └── icons/
│
├── frontend/                    # React + TypeScript UI
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── tailwind.config.ts
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   ├── styles/
│   │   │   └── global.css
│   │   │
│   │   ├── components/
│   │   │   ├── layout/
│   │   │   │   ├── Sidebar.tsx          # Навигация
│   │   │   │   ├── TopBar.tsx           # Статус подключения, ECU info
│   │   │   │   ├── StatusBar.tsx        # Прогресс, лог сообщений
│   │   │   │   └── WorkArea.tsx         # Основная рабочая область
│   │   │   │
│   │   │   ├── connection/
│   │   │   │   ├── AdapterSelector.tsx  # Выбор адаптера (auto-detect)
│   │   │   │   ├── ProtocolConfig.tsx   # CAN bitrate, адреса
│   │   │   │   └── ConnectionStatus.tsx # Статус с LED-индикатором
│   │   │   │
│   │   │   ├── flash/
│   │   │   │   ├── ReadPanel.tsx        # Чтение прошивки (progress bar)
│   │   │   │   ├── WritePanel.tsx       # Запись с подтверждением
│   │   │   │   ├── VerifyPanel.tsx      # Верификация после записи
│   │   │   │   └── BackupManager.tsx    # Бэкапы прошивок
│   │   │   │
│   │   │   ├── editor/
│   │   │   │   ├── HexEditor.tsx        # Hex view с подсветкой регионов
│   │   │   │   ├── MapEditor2D.tsx      # 2D график карты
│   │   │   │   ├── MapEditor3D.tsx      # 3D surface (Three.js)
│   │   │   │   ├── MapTable.tsx         # Таблица значений
│   │   │   │   ├── MapList.tsx          # Список найденных карт
│   │   │   │   ├── MapSearch.tsx        # Поиск карт по имени/типу
│   │   │   │   ├── MapDiff.tsx          # Сравнение stock vs modified
│   │   │   │   └── ScalarEditor.tsx     # Редактор одиночных значений
│   │   │   │
│   │   │   ├── dtc/
│   │   │   │   ├── DTCPanel.tsx         # Список ошибок
│   │   │   │   ├── DTCFilter.tsx        # Фильтр: active/stored/pending
│   │   │   │   ├── DTCDetail.tsx        # Описание + freeze frame
│   │   │   │   └── DTCSearch.tsx        # Поиск по коду/описанию
│   │   │   │
│   │   │   ├── live/
│   │   │   │   ├── Dashboard.tsx        # Виртуальные приборы
│   │   │   │   ├── GaugeWidget.tsx      # Один прибор (RPM, boost...)
│   │   │   │   ├── GraphWidget.tsx      # Time-series график
│   │   │   │   └── LogViewer.tsx        # Табличный просмотр лога
│   │   │   │
│   │   │   ├── ai/
│   │   │   │   ├── AIAssistant.tsx      # Chat-подобный интерфейс
│   │   │   │   ├── MapClassifier.tsx    # Результаты классификации
│   │   │   │   ├── SafetyReport.tsx     # Отчёт безопасности
│   │   │   │   └── SuggestionCard.tsx   # Карточка предложения AI
│   │   │   │
│   │   │   └── common/
│   │   │       ├── ProgressBar.tsx
│   │   │       ├── ConfirmDialog.tsx    # "Вы уверены?" для записи
│   │   │       ├── Toast.tsx
│   │   │       ├── FileDropZone.tsx     # Drag & drop бинарников
│   │   │       └── ThemeToggle.tsx
│   │   │
│   │   ├── hooks/
│   │   │   ├── useConnection.ts         # Хук для подключения к ECU
│   │   │   ├── useFlash.ts             # Хук для чтения/записи
│   │   │   ├── useLiveData.ts          # Хук для real-time данных
│   │   │   ├── useProject.ts           # Хук для проекта (undo/redo)
│   │   │   └── useAI.ts               # Хук для AI-функций
│   │   │
│   │   ├── stores/
│   │   │   ├── connectionStore.ts       # Zustand store: connection
│   │   │   ├── projectStore.ts          # Zustand store: open project
│   │   │   ├── editorStore.ts           # Zustand store: editor state
│   │   │   └── settingsStore.ts         # Zustand store: preferences
│   │   │
│   │   ├── lib/
│   │   │   ├── tauri.ts                 # Typed Tauri invoke wrappers
│   │   │   ├── binary.ts               # Frontend binary utils
│   │   │   └── formatters.ts           # Hex, engineering units
│   │   │
│   │   └── types/
│   │       ├── ecu.ts
│   │       ├── map.ts
│   │       ├── dtc.ts
│   │       └── protocol.ts
│   │
│   └── public/
│       └── assets/
│
├── python/                      # Python AI/ML компоненты
│   ├── pyproject.toml
│   ├── daedalus/
│   │   ├── __init__.py
│   │   │
│   │   ├── ai/
│   │   │   ├── __init__.py
│   │   │   ├── client.py            # Claude API client wrapper
│   │   │   ├── map_classifier.py    # Классификация карт через LLM
│   │   │   ├── safety_validator.py  # Проверка безопасности модификаций
│   │   │   ├── damos_generator.py   # Генерация A2L-описаний
│   │   │   ├── prompts/
│   │   │   │   ├── classify_map.txt
│   │   │   │   ├── validate_mod.txt
│   │   │   │   ├── identify_ecu.txt
│   │   │   │   └── explain_dtc.txt
│   │   │   └── cache.py             # Кеширование ответов API
│   │   │
│   │   ├── ml/
│   │   │   ├── __init__.py
│   │   │   ├── map_finder.py        # ML-based map detection
│   │   │   ├── features.py          # Feature extraction из бинарников
│   │   │   ├── training/
│   │   │   │   ├── dataset.py       # Создание датасета из A2L+binary
│   │   │   │   ├── train.py         # Обучение модели
│   │   │   │   └── evaluate.py
│   │   │   └── models/
│   │   │       └── map_detector_v1.onnx
│   │   │
│   │   ├── analysis/
│   │   │   ├── __init__.py
│   │   │   ├── binary_stats.py      # Статистический анализ бинарников
│   │   │   ├── entropy.py           # Энтропия регионов (code vs data)
│   │   │   ├── axis_finder.py       # Поиск монотонных последовательностей
│   │   │   └── diff_engine.py       # Умное сравнение прошивок
│   │   │
│   │   ├── parsers/
│   │   │   ├── __init__.py
│   │   │   ├── a2l_parser.py        # A2L/ASAP2 парсинг (через pyA2L)
│   │   │   ├── xdf_parser.py        # TunerPro XDF
│   │   │   ├── kp_parser.py         # WinOLS .kp projects
│   │   │   └── damos_parser.py      # Damos файлы
│   │   │
│   │   └── ghidra/
│   │       ├── __init__.py
│   │       ├── headless.py          # Ghidra headless mode runner
│   │       ├── scripts/
│   │       │   ├── analyze_ecu.py   # Ghidra script: полный анализ
│   │       │   ├── find_tables.py   # Ghidra script: поиск таблиц по xrefs
│   │       │   └── export_funcs.py  # Ghidra script: экспорт функций
│   │       └── mcp_bridge.py        # Bridge к ReVa MCP для Claude
│   │
│   ├── server.py                # OPTIONAL: dev-only test server
│   └── tests/
│       ├── test_map_finder.py
│       ├── test_classifier.py
│       └── fixtures/
│           ├── sample_edc17.bin
│           └── sample_edc17.a2l
│
├── data/                        # Статические данные
│   ├── dtc/
│   │   ├── obd2_standard.json       # P0xxx, C0xxx, B0xxx, U0xxx
│   │   ├── bosch_manufacturer.json  # Manufacturer-specific DTC
│   │   └── vag_specific.json        # VAG-specific
│   ├── ecu_signatures/
│   │   ├── bosch.json               # Сигнатуры Bosch ECU
│   │   ├── siemens.json
│   │   └── denso.json
│   ├── checksum_defs/
│   │   └── algorithms.json          # Описания алгоритмов чексум
│   └── protocols/
│       └── can_databases/           # DBC файлы
│
├── tests/                       # Интеграционные тесты
│   ├── rust/
│   │   ├── test_isotp.rs
│   │   ├── test_uds.rs
│   │   └── test_checksum.rs
│   └── e2e/
│       └── test_full_flow.rs
│
├── scripts/
│   ├── setup_dev.sh                 # Установка зависимостей
│   ├── setup_socketcan.sh           # Настройка vcan для тестов
│   └── build_release.sh
│
└── .github/
    └── workflows/
        ├── ci.yml
        └── release.yml
```

---

## ЧАСТЬ 3: CLAUDE.md — ИНСТРУКЦИИ ДЛЯ CLAUDE CODE

```markdown
# CLAUDE.md — Daedalus Development Guide

## Project Overview
Daedalus is an open-source, AI-assisted ECU tuning platform.
Desktop app: Tauri (Rust backend) + React (TypeScript frontend).
AI: Cloud APIs + local LLM via Provider trait (Claude/OpenAI/Gemini/Ollama).

## Tech Stack
- **Backend**: Rust 1.78+, Tauri 2.x, tokio async runtime, reqwest for API calls
- **Frontend**: React 19, TypeScript 5.x, Vite, Tailwind CSS 4, Zustand
- **AI**: Provider trait in Rust — Claude API, OpenAI, Gemini, Ollama (local LLM)
- **Local ML** (optional, needs GPU): ONNX Runtime for map finder CNN
- **3D**: Three.js (React Three Fiber) for 3D map visualization
- **Charts**: Recharts for 2D maps and time-series
- **Hex editor**: Custom virtual-scroll hex view component
- **NO Python in production** — optional dev/training tool only

## Architecture Rules
1. All CAN/serial timing-critical code runs in Rust, never in JS
2. ZERO mandatory cloud dependency — app works fully offline with Ollama
3. AI Provider calls are pure Rust HTTP (reqwest) — no Python runtime
4. Frontend communicates with Rust via Tauri IPC (invoke)
5. All file I/O (binary read/write) happens in Rust
6. Every write operation requires explicit user confirmation (ConfirmDialog)
7. Always create backup before any ECU write operation
8. Undo/redo for all map edits (command pattern in projectStore)
9. Two profiles: LOCAL-FIRST (GPU laptop) and CLOUD-FIRST (any laptop)

## Coding Standards
### Rust
- Use `thiserror` for error types, `anyhow` for application errors
- All async code with `tokio`
- `serde` for all serialization
- Clippy clean, `#![deny(clippy::all)]`
- Document public APIs with `///` doc comments

### TypeScript/React
- Functional components only, no class components
- Zustand for state management (no Redux)
- All Tauri calls wrapped in typed functions in `lib/tauri.ts`
- Use `React.lazy` for heavy components (HexEditor, MapEditor3D)
- Tailwind utility classes, no CSS modules

### Python
- Type hints everywhere (mypy strict)
- async/await for API calls
- Pydantic models for all data structures
- pytest for tests

## Key Design Decisions

### Map Editor
- HexEditor: Virtual scrolling, 64 bytes per row, selectable regions
- 2D: Recharts with zoom/pan, cursor shows current cell
- 3D: React Three Fiber, orbit controls, color gradient (blue→red)
- Table: Editable cells, highlight changes vs stock (green=decreased, red=increased)
- All views synchronized: selecting in one updates others

### DTC System
- Searchable by code (P0420), description text, or category
- Filters: Active, Stored, Pending, All
- Each DTC shows: code, description, freeze frame, status byte
- "Explain with AI" button → sends DTC + freeze frame to Claude API

### Safety
- NEVER allow write without backup
- NEVER allow AI to auto-apply changes
- Always show diff (stock vs modified) before write
- Checksum correction is MANDATORY and automatic
- Safety validator checks: lambda limits, boost limits, timing limits
- All safety violations shown as RED warnings, block write until acknowledged

## Dependencies to Integrate from Open Source

### From VW_Flash (MIT license):
- Simos18 checksum algorithms
- AES encryption/decryption routines
- LZSS compression
- UDS flash sequence for Simos

### From python-udsoncan (MIT):
- UDS service definitions
- ISO-TP layer (reference implementation)

### From python-can (LGPL-3.0):
- SocketCAN interface wrapper
- Bus abstraction

### From pyA2L (GPL-2.0 — careful with licensing):
- A2L parser (may need clean-room reimplementation for GPLv3)

### From candleLight_fw (GPL-2.0):
- gs_usb protocol documentation

### From RomRaider (GPL-2.0):
- XML-based map definition format (for reference)
- Logger protocol implementation (for reference)

### From LinOLS (license TBD):
- Map visualization approach (for reference, not code)

### From Atlas (AGPL-3.0 — careful):
- Table-matching algorithm concept (for reference)

## Build & Run
```bash
# Setup
./scripts/setup_dev.sh

# Development
cd frontend && npm run dev  # Vite dev server
cd crates/forge-app && cargo tauri dev  # Tauri dev mode

# Optional: local LLM
ollama pull phi3:3.8b-mini-4k-instruct-q4_K_M

# Test with virtual CAN
./scripts/setup_socketcan.sh  # creates vcan0
cargo test

# Test harness (standalone)
cd tests/harness && python test_harness.py --mode demo
```

## Priority Implementation Order
1. forge-core + forge-hal (CAN connection)
2. forge-proto (UDS basics: session, security, DTC read)
3. Frontend: connection panel + DTC viewer
4. forge-binary (file parser + hex view)
5. Frontend: hex editor + basic map view
6. forge-flash (OBD read for one ECU family)
7. Python: AI map classifier
8. Frontend: AI assistant panel
9. forge-flash (write + checksum)
10. Python: ML map finder
```

---

## ЧАСТЬ 4: ЛОГИКА РАБОТЫ ПРОГРАММЫ

### 4.1 User Flow: Подключение к ECU

```
[Запуск приложения]
    ↓
[Auto-detect: сканирование USB для CAN/serial адаптеров]
    ↓ (найдены: CANable на /dev/ttyACM0, vcan0)
[Пользователь выбирает адаптер]
    ↓
[Выбор протокола: CAN 500kbps / CAN 250kbps / K-Line]
    ↓
[ECU Scan: отправка UDS TesterPresent на стандартные адреса]
    ↓ (ответ от 0x7E0 → ECU engine)
[Автоопределение ECU: ReadDataByIdentifier → HW/SW version]
    ↓
[Отображение: "Bosch MED17.5.2, VW Golf 7 2.0 TSI, SW: 06L906022FT"]
    ↓
[Главный экран с вкладками: DTC | Live | Flash | Editor | AI]
```

### 4.2 User Flow: Чтение ошибок (DTC)

```
[Вкладка DTC → кнопка "Считать"]
    ↓
[UDS: ReadDTCInformation (subfunc 0x02 = reportDTCByStatusMask)]
    ↓
[Получен список: P0420, P2463, P0401...]
    ↓
[Каждый DTC отображается в таблице:]
   Код      | Описание              | Статус    | Freeze Frame
   P0420    | Catalyst efficiency   | Active    | RPM: 2100, MAP: 45kPa
   P2463    | DPF soot accum.       | Stored    | —
    ↓
[Фильтры: 🔴Active  🟡Stored  🟢Pending  📋All]
[Поиск: 🔍 "DPF" → фильтрует P2463]
    ↓
[Клик на DTC → детальная панель справа]
    ↓
[Кнопка "🤖 Объяснить (AI)"]
    ↓
[Claude API: "P2463 на MED17 Golf — что означает, причины, решения"]
    ↓
[AI ответ в chat-панели с конкретными рекомендациями]
```

### 4.3 User Flow: Чтение и редактирование прошивки

```
[Вкладка Flash → "Считать прошивку"]
    ↓
[Диалог: метод чтения — OBD / Bench / Boot / Файл]
    ↓ (OBD)
[SecurityAccess → seed/key exchange]
    ↓
[Progress bar: чтение Flash 2MB... 15% ... 73% ... 100%]
    ↓ (автоматически)
[Backup сохранён: ~/daedalus/backups/MED17_06L906022FT_2026-03-23.bin]
    ↓
[Автоанализ запускается параллельно:]
   1. ECU identification → "Bosch MED17.5.2, Tricore TC1791"
   2. Memory map → Code: 0x80000000-0x801FFFFF, Cal: 0x80100000-0x8017FFFF
   3. Map finder (ML) → обнаружено 247 потенциальных карт
   4. Claude API → классификация: "Fuel injection main map (confidence 0.94)"
    ↓
[Переход на вкладку Editor]
    ↓
[Слева: дерево карт (как в WinOLS)]
   📂 Fuel
     📊 Main injection timing (16×16)
     📊 Rail pressure target (12×16)
   📂 Boost
     📊 Target boost pressure (16×16)
     📊 Wastegate duty cycle (16×16)
   📂 Torque
     📊 Max torque limiter (16×1)
   📂 Limiters
     📊 Speed limiter (scalar: 250 km/h)
   📂 Emissions
     📊 DPF regeneration threshold
     📊 EGR target position
   📂 Unclassified (47 maps)
    ↓
[Клик на карту → правая панель: 2D/3D/Table/Hex]
[Инструменты: +%, -%, smooth, interpolate, copy/paste region]
    ↓
[Все изменения подсвечены: 🟢 уменьшено, 🔴 увеличено]
[Внизу: diff summary "Modified: 3 maps, 127 bytes changed"]
```

### 4.4 User Flow: AI-ассистент

```
[Вкладка AI или боковая панель справа]
    ↓
[Chat interface:]

User: "Хочу Stage 1 для Golf 7 2.0 TSI, что изменить?"
    ↓
Claude: "Для MQB EA888 gen3 Stage 1 обычно затрагивает:
  1. Boost target: +200-300 mbar (с 1350 до 1550-1650 mbar)
  2. Timing advance: +2-4° в зоне 2000-5000 RPM, 80-160 kPa
  3. Torque limiter: увеличить на 20-30%
  4. Lambda target: оставить stock (безопасность!)
  5. Speed limiter: по желанию

  ⚠️ Я могу показать конкретные карты и предложить значения,
  но финальное решение — за вами. Рекомендую проверку на стенде."
    ↓
[Кнопки: "Покажи boost target" → открывает карту в Editor]
         "Предложи значения" → Claude генерирует таблицу с изменениями
         "Проверь безопасность" → Safety Validator
```

### 4.5 Safety Validator Pipeline

```
[Пользователь нажимает "Записать в ECU"]
    ↓
[Step 1: Checksum correction]
   Автокоррекция CRC32, CVN, multipoint checksum
    ↓
[Step 2: Diff analysis]
   Показать ВСЕ изменения: "127 bytes in 3 maps changed"
    ↓
[Step 3: Safety checks (hard-coded rules)]
   ✅ Lambda target ≥ 0.78 under boost — OK
   ✅ Timing advance ≤ knock-limited MBT — OK
   ⚠️ Boost target 1650 mbar — WARNING: выше stock 22%
   ✅ EGT limit maintained — OK
   ✅ Speed limiter ≥ tire rating — OK
    ↓
[Step 4: AI Safety Review (optional)]
   Claude API: "Проверь эти изменения для EA888 gen3..."
   → "Изменения в допустимых пределах для Stage 1.
      Рекомендация: после прошивки проверить knock count
      и lambda correlation на стенде."
    ↓
[Step 5: User confirmation]
   ⚠️ ВНИМАНИЕ: Запись изменённой прошивки.
   Backup создан: [путь]
   Изменено: 3 карты (127 байт)
   Предупреждения: 1

   [❌ Отмена]  [✅ Записать]
    ↓
[Step 6: Flash write + verify]
   Writing... [▓▓▓▓▓▓▓▓░░] 80%
   Verifying... [▓▓▓▓▓▓▓▓▓▓] 100% ✅
   
   "Прошивка записана успешно. Выключите и включите зажигание."
```

---

## ЧАСТЬ 5: ЧТО БЕРЁМ ИЗ OPEN-SOURCE, ЧЕГО НЕ ХВАТАЕТ

### Берём (с адаптацией):

| Что | Откуда | Лицензия | Как используем |
|-----|--------|----------|----------------|
| CAN transport layer | python-can → socketcan-rs | LGPL/MIT | Порт на Rust |
| UDS services | python-udsoncan | MIT | Референс для Rust |
| ISO-TP | python-can-isotp | MIT | Референс для Rust |
| Simos18 flash | VW_Flash (bri3d) | MIT | Интеграция checksum/crypto |
| TC1791 BSL | TC1791_CAN_BSL | MIT | Порт boot-mode логики |
| A2L parsing | pyA2L | GPL-2.0 | Используем как Python dep |
| Map definitions | RomRaider XML | GPL-2.0 | Формат для референса |
| CAN analysis | SavvyCAN | MIT | Не интегрируем, параллельный инструмент |
| Ghidra integration | ReVa MCP | Apache-2.0 | Прямая интеграция через MCP |
| DTC database | OBD2 standards (free) | Public | JSON-база |
| ECU simulation | ecu-simulator | MIT | Для тестов |

### Чего не хватает (нужно создать):

| Компонент | Сложность | Приоритет | Описание |
|-----------|-----------|-----------|----------|
| **Rust UDS stack** | Высокая | P0 | Полный UDS на Rust (нет зрелых crate) |
| **ML Map Finder** | Высокая | P1 | CNN/transformer для поиска карт в бинарниках |
| **Claude Map Classifier** | Средняя | P1 | Prompt engineering + structured output |
| **Universal Checksum Engine** | Высокая | P0 | Bosch ME7/MED17/EDC16/EDC17 |
| **Hex Editor Component** | Средняя | P0 | Virtual-scroll React hex editor |
| **3D Map Surface** | Средняя | P1 | React Three Fiber + color mapping |
| **Safety Constraint Engine** | Средняя | P0 | Жёсткие лимиты per-ECU-type |
| **ECU Signature DB** | Средняя | P1 | Сигнатуры для автоопределения |
| **Bench Power Control** | Низкая | P2 | USB-relay для управления питанием ECU |

---

## ЧАСТЬ 6: ROADMAP РАЗРАБОТКИ

### Phase 1: Foundation (2-3 недели)
```
[x] Инициализация Tauri + React + Rust workspace
[ ] forge-core: типы, конфиг, error handling
[ ] forge-hal: SocketCAN + virtual CAN
[ ] forge-proto: ISO-TP + базовые UDS сервисы
[ ] Frontend: layout shell, connection panel
[ ] Тесты: vcan0, echo UDS
```

### Phase 2: Read & Diagnose (2-3 недели)
```
[ ] forge-dtc: чтение DTC, база кодов
[ ] forge-live: OBD2 PIDs, простой logger
[ ] Frontend: DTC panel с фильтрацией
[ ] Frontend: live dashboard с gauges
[ ] forge-ai: Provider trait + Claude implementation
[ ] AI: "Explain DTC" feature (cloud + local fallback)
```

### Phase 3: Binary Analysis (3-4 недели)
```
[ ] forge-binary: парсеры форматов, ECU identification
[ ] forge-binary: автоматический map finder (v1, heuristic)
[ ] Frontend: hex editor (virtual scroll)
[ ] Frontend: map editor (2D table + Recharts)
[ ] Frontend: map tree + search
[ ] Python: A2L/XDF import
```

### Phase 4: AI Integration (2-3 недели)
```
[ ] forge-ai: Claude provider + map classifier prompts
[ ] forge-ai: Safety validator (hardcoded rules + AI review)
[ ] forge-ai: Ollama provider for local-first mode
[ ] Frontend: AI chat panel
[ ] Frontend: 3D map surface (Three.js)
[ ] config: profiles.yaml (local-first / cloud-first switching)
[ ] Integration: Ghidra headless → ReVa MCP → Claude (optional)
```

### Phase 5: Flash & Write (3-4 недели)
```
[ ] forge-flash: OBD flash для одного ECU (MED17/Simos18)
[ ] forge-flash: checksum correction engine
[ ] forge-flash: backup manager
[ ] Frontend: flash wizard с прогрессом
[ ] Frontend: diff view (stock vs modified)
[ ] Safety: full validation pipeline перед записью
```

### Phase 6: Polish & Expand (ongoing)
```
[ ] Больше ECU families
[ ] Bench/Boot mode
[ ] K-Line протокол
[ ] Map pack community sharing
[ ] Plugin system
[ ] Локализация (RU, DE, EN)
```
