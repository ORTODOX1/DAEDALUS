<p align="center">
  <img src="assets/logo.png" alt="Daedalus Logo" width="400"/>
</p>

<h1 align="center">D A E D A L U S</h1>

<p align="center">
  <em>master the labyrinth</em>
</p>

<p align="center">
  <strong>Open-source AI-assisted ECU tuning platform</strong><br/>
  Специализация: коммерческая техника, грузовики, оптимизация расхода топлива
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.78+-orange?logo=rust" alt="Rust"/>
  <img src="https://img.shields.io/badge/React-19-blue?logo=react" alt="React"/>
  <img src="https://img.shields.io/badge/Tauri-2.x-purple?logo=tauri" alt="Tauri"/>
  <img src="https://img.shields.io/badge/AI-Claude%20%7C%20GPT%20%7C%20Gemini-green" alt="AI"/>
  <img src="https://img.shields.io/badge/License-GPL--3.0-red" alt="License"/>
</p>

---

## Что это?

**Daedalus** — десктопное приложение для чтения, анализа, модификации и записи прошивок ECU автомобилей и коммерческой техники.

### Ключевые возможности

| Функция | Описание |
|---------|----------|
| **Чтение/запись ECU** | OBD2, BDM, JTAG, Boot Mode через CAN/K-Line |
| **Hex-редактор** | Виртуальный скролл, подсветка регионов, поиск паттернов |
| **Редактор карт** | 2D таблицы + 3D поверхности (Three.js) |
| **AI-анализ** | Автоматический поиск карт, классификация, проверка безопасности |
| **DTC диагностика** | Чтение/сброс кодов ошибок, freeze frame, J1939 SPN/FMI |
| **Live Data** | Мониторинг параметров в реальном времени |
| **Diff View** | Сравнение stock vs modified side-by-side |
| **Undo/Redo** | Полная история изменений с откатом |

### Фокус: коммерческая техника

```
  Грузовики          ТНВД              Оптимизация           Экология
  ┌──────────┐   ┌──────────────┐   ┌───────────────┐   ┌──────────────┐
  │ MAN      │   │ Common Rail  │   │ Расход топлива│   │ AdBlue/SCR   │
  │ DAF      │   │ давления     │   │ Крутящий      │   │ DPF/EGR      │
  │ Scania   │   │ IQ-адаптация │   │ момент        │   │ Сажевый      │
  │ Volvo    │   │ Тайминг      │   │ Режимы работы │   │ фильтр       │
  │ Mercedes │   │ впрыска      │   │ Boost/турбина │   │ NOx сенсоры  │
  │ Iveco    │   │ Pilot/Main/  │   │ Лимитеры      │   │ Lambda       │
  │ Камаз    │   │ Post inject  │   │ скорости      │   │              │
  └──────────┘   └──────────────┘   └───────────────┘   └──────────────┘
```

**Поддерживаемые ECU грузовиков:**
- Bosch EDC17 / MD1 (MAN, DAF, Iveco, КамАЗ)
- Bosch EDC16 (старые европейские грузовики)
- Delphi DCM3.7 / DCM6.x (DAF, Ford Cargo)
- Denso (Volvo, Renault Trucks)
- Cummins CM2350 / CM2450 (PACCAR, КамАЗ)
- Siemens/Continental SID309 (Volvo, Renault)

### Протоколы

| Протокол | Стандарт | Применение |
|----------|----------|------------|
| **J1939** | SAE J1939 | CAN 250 kbps для грузовиков |
| **UDS** | ISO 14229 | Диагностика, Security Access |
| **ISO-TP** | ISO 15765 | Транспортный уровень CAN |
| **KWP2000** | ISO 14230 | Старые ECU |
| **OBD2** | ISO 15031 | Стандартная диагностика |
| **J1708/J1587** | SAE | Старые американские грузовики |

---

## Архитектура

```
┌────────────────────────────────────────────────────────────────┐
│                         DAEDALUS                               │
│                                                                │
│  ┌─────────────┐  ┌───────────────┐  ┌──────────────────────┐ │
│  │  Frontend    │  │   Backend     │  │    AI Engine          │ │
│  │  React 19    │  │   Rust        │  │    Claude / GPT /    │ │
│  │  TypeScript  │←→│   Tauri 2.x   │←→│    Gemini / Ollama   │ │
│  │  Tailwind    │  │   tokio       │  │                      │ │
│  └──────┬──────┘  └───────┬───────┘  └──────────┬───────────┘ │
│         │                 │                      │             │
│  ┌──────┴─────────────────┴──────────────────────┴───────────┐ │
│  │              Hardware Abstraction Layer                     │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │ │
│  │  │SocketCAN │ │ J2534    │ │ K-Line   │ │ BDM / JTAG   │  │ │
│  │  │ + J1939  │ │ passthru │ │ serial   │ │ (OpenOCD)    │  │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

---

## Быстрый старт

### Требования

- **Rust** 1.78+
- **Node.js** 20+
- **Tauri CLI** 2.x

### Установка

```bash
# Клонирование
git clone https://github.com/ORTODOX1/DAEDALUS.git
cd DAEDALUS

# Установка зависимостей
./scripts/setup_dev.sh

# Запуск в dev-режиме
cd frontend && npm install && npm run dev &
cd crates/forge-app && cargo tauri dev
```

### Тестирование без оборудования

```bash
# Виртуальный CAN (Linux)
sudo modprobe vcan
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0

# Запуск симулятора ECU
python -m daedalus.test.ecu_sim --interface vcan0 --ecu-type edc17

# Запуск приложения
DAEDALUS_CAN_INTERFACE=vcan0 cargo tauri dev
```

---

## Структура проекта

```
daedalus/
├── crates/                  # Rust backend (Cargo workspace)
│   ├── forge-core/          # Типы, конфиг, ошибки, проекты
│   ├── forge-hal/           # Драйверы: SocketCAN, SLCAN, J2534, K-Line
│   ├── forge-proto/         # Протоколы: ISO-TP, UDS, KWP2000, J1939, OBD2
│   ├── forge-flash/         # Чтение/запись ECU, контрольные суммы
│   ├── forge-binary/        # Парсинг бинарников, поиск карт, hex view
│   ├── forge-dtc/           # База DTC (OBD2 + J1939 SPN/FMI)
│   ├── forge-live/          # Real-time логгирование, датчики
│   ├── forge-ai/            # AI провайдеры (Claude/OpenAI/Gemini/Ollama)
│   └── forge-app/           # Tauri app, IPC команды, состояние
├── frontend/                # React UI
│   └── src/
│       ├── components/      # UI компоненты по категориям
│       ├── hooks/           # React hooks для Tauri IPC
│       ├── stores/          # Zustand stores
│       └── types/           # TypeScript типы
├── data/                    # Базы DTC, сигнатуры ECU, контрольные суммы
├── docs/                    # Документация, спецификации протоколов
├── assets/                  # Логотип, иконки
└── scripts/                 # Скрипты установки и настройки
```

---

## Оптимизация ТНВД (Common Rail)

Ключевые карты для работы с коммерческой техникой:

| Карта | Описание | Единицы |
|-------|----------|---------|
| **IQ → Rail Pressure** | Давление рампы от объема впрыска | bar / mg/stroke |
| **Pilot Quantity** | Предварительный впрыск | mg/stroke |
| **Main Injection Timing** | Угол основного впрыска | °BTDC |
| **Post Injection** | Пост-впрыск для DPF регенерации | mg/stroke |
| **Torque Limiter** | Ограничение крутящего момента | Nm |
| **Boost Pressure** | Целевое давление наддува | mbar |
| **EGR Rate** | Рециркуляция выхлопных газов | % |
| **Speed Limiter** | Ограничитель скорости | km/h |
| **Fuel Temperature Comp** | Компенсация по температуре топлива | mg/°C |

---

## AI-ассистент

Daedalus использует облачные AI API для анализа прошивок:

- **Поиск карт**: автоматическое обнаружение таблиц в бинарнике по энтропии и паттернам
- **Классификация**: определение типа карты (впрыск, давление, тайминг)
- **Проверка безопасности**: блокировка опасных модификаций
- **Объяснение DTC**: расшифровка кодов ошибок с рекомендациями

В облако уходят только статистические признаки (~2-5 КБ), **не полный бинарник**.

---

## Безопасность

- Запись в ECU невозможна без создания резервной копии
- AI не может автоматически модифицировать прошивку — только предлагает
- Обязательный diff-просмотр перед записью
- Коррекция контрольных сумм перед записью
- Жесткие лимиты безопасности по типу ECU (не настраиваемые)
- Lambda < 0.78 под наддувом = **БЛОКИРОВКА ЗАПИСИ**
- Тайминг за пределами детонации = **БЛОКИРОВКА ЗАПИСИ**

---

## Лицензия

[GPL-3.0](LICENSE)

---

<p align="center">
  <img src="assets/logo.png" alt="Daedalus" width="120"/>
  <br/>
  <em>Daedalus — master the labyrinth</em>
</p>
