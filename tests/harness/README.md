# Daedalus — Тестовый стенд

## Быстрый старт (3 минуты)

### Вариант 1: Демо (без ключей, без Ollama)
```bash
pip install numpy rich pyyaml httpx
python test_harness.py --mode demo
```
Работает сразу. AI-ответы — заглушки, но весь пайплайн виден.

### Вариант 2: Облачный режим (Claude API)
```bash
pip install numpy rich pyyaml httpx anthropic

# Windows:
set ANTHROPIC_API_KEY=sk-ant-api03-ТВОЙ_КЛЮЧ

# Linux:
export ANTHROPIC_API_KEY=sk-ant-api03-ТВОЙ_КЛЮЧ

python test_harness.py --mode cloud
```
Реальные ответы от Claude. Классификация карт, объяснение DTC, чат — всё через API.

### Вариант 3: Локальный режим (Ollama + GPU)
```bash
# 1. Установи Ollama
#    Windows: https://ollama.com/download
#    Linux: curl -fsSL https://ollama.com/install.sh | sh

# 2. Скачай модель (~2.5 ГБ, займёт ~2 мин)
ollama pull phi3:3.8b-mini-4k-instruct-q4_K_M

# 3. Запусти (Ollama обычно уже запущена как сервис)
# Если нет: ollama serve

# 4. Тест
pip install numpy rich pyyaml httpx
python test_harness.py --mode local
```
Полный офлайн. Все AI-ответы от локальной модели на твоём GPU.

## Что тестируется

```
Шаг 1: Генерация фейкового бинарника ECU (512 КБ)
       ├── Код (200 КБ, энтропия ~6.5)
       ├── 4 калибровочные карты (boost, timing, lambda, speed limiter)
       ├── 3 DTC кода (P0420, P2463, P0401)
       └── Пустая область (0xFF)

Шаг 2: Локальный анализ регионов (CPU, ВСЕГДА)
       ├── Энтропия Шеннона каждого 4 КБ блока
       ├── Классификация: code / data / calibration / empty
       └── Время: ~80мс на 512 КБ

Шаг 3: Эвристический поиск карт (CPU, ВСЕГДА)
       ├── Поиск монотонных uint16 последовательностей (оси)
       ├── Детекция пар осей (X + Y)
       └── Время: ~1.5 сек на 512 КБ

Шаг 4: AI классификация карт → зависит от режима
Шаг 5: AI объяснение DTC     → зависит от режима
Шаг 6: AI чат-ассистент      → зависит от режима
```

## Два профиля

### LOCAL-FIRST (config/profiles.yaml → local)
- ONNX map finder на GPU → 1-3 сек
- Phi-3 3.8B → классификация карт (2.5 ГБ VRAM)
- Llama 3.2 8B → DTC и чат (5 ГБ VRAM)
- Claude API → только fallback для сложных задач
- **Итого VRAM: ~6-8 ГБ**
- **API запросов: 0-5 за сессию**

### CLOUD-FIRST (config/profiles.yaml → cloud)
- CPU-эвристика → поиск карт (30-60 сек)
- Claude → классификация, DTC, чат
- Gemini → простые задачи (дешевле)
- OpenAI → fallback
- **Итого VRAM: 0**
- **API запросов: ~250 за сессию**

## Файлы
```
ecu-test/
├── test_harness.py          ← Главный тест (запускай его)
├── config/
│   └── profiles.yaml        ← Два профиля конфигурации
└── README.md                ← Ты здесь
```
