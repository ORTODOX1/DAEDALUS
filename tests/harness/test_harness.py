"""
Daedalus — Test Harness
=============================
Запусти это ПРЯМО СЕЙЧАС на своём ПК чтобы увидеть как работают оба режима.
Не нужен ни ECU, ни CAN-адаптер, ни ноутбук.

Установка:
    pip install anthropic numpy rich pyyaml httpx

Запуск (облачный режим — нужен API ключ Claude):
    set ANTHROPIC_API_KEY=sk-ant-...
    python test_harness.py --mode cloud

Запуск (локальный режим — нужен Ollama):
    1. Установи Ollama: https://ollama.com/download
    2. ollama pull phi3:3.8b-mini-4k-instruct-q4_K_M
    3. python test_harness.py --mode local

Запуск (без API, без Ollama — демо на заглушках):
    python test_harness.py --mode demo
"""

import argparse
import asyncio
import json
import os
import struct
import sys
import time
from dataclasses import dataclass, field
from typing import Optional

import numpy as np

# Rich для красивого вывода
try:
    from rich.console import Console
    from rich.table import Table
    from rich.panel import Panel
    from rich.progress import Progress, SpinnerColumn, TextColumn
    from rich import print as rprint
    HAS_RICH = True
except ImportError:
    HAS_RICH = False
    def rprint(*a, **kw): print(*a)

console = Console() if HAS_RICH else None


# ═══════════════════════════════════════════════════════════
# ЧАСТЬ 1: Генерация тестового бинарника ECU
# ═══════════════════════════════════════════════════════════

def generate_fake_ecu_binary(size_kb: int = 512) -> bytes:
    """
    Генерирует фейковый бинарник ECU для тестирования.
    Имитирует структуру Bosch MED17: код + калибровки + пустые области.
    """
    rng = np.random.default_rng(42)
    data = bytearray(size_kb * 1024)
    
    regions = {}
    offset = 0
    
    # 1. Заголовок ECU (сигнатура)
    header = b"BOSCH_MED17.5.2\x00" + b"VW_06L906022FT\x00\x00"
    data[0:len(header)] = header
    offset = 0x100
    
    # 2. Регион кода (высокая энтропия, ~6.5)
    code_size = 200 * 1024
    code = bytes(rng.integers(0, 256, code_size, dtype=np.uint8))
    data[offset:offset + code_size] = code
    regions["code"] = {"offset": offset, "size": code_size, "type": "code"}
    offset += code_size
    
    # 3. Калибровочные карты (средняя энтропия, структурированные)
    maps_start = offset
    generated_maps = []
    
    # Карта 1: Boost target (16x16, RPM x Load → mbar)
    rpm_axis = np.linspace(800, 6800, 16).astype(np.uint16)
    load_axis = np.linspace(0, 1600, 16).astype(np.uint16)
    boost_data = np.outer(
        np.linspace(0.3, 1.0, 16),
        np.linspace(0.5, 1.35, 16)
    ) * 1800
    boost_data = boost_data.astype(np.uint16)
    
    # Записываем оси + данные
    map1_offset = offset
    data[offset:offset + 32] = rpm_axis.tobytes()
    offset += 32
    data[offset:offset + 32] = load_axis.tobytes()
    offset += 32
    data[offset:offset + 512] = boost_data.tobytes()
    offset += 512
    generated_maps.append({
        "name": "Boost target pressure",
        "offset": map1_offset,
        "size": 576,
        "dims": (16, 16),
        "x_range": (800, 6800),
        "y_range": (0, 1600),
        "data_range": (float(boost_data.min()), float(boost_data.max())),
        "data_mean": float(boost_data.mean()),
    })
    
    # Карта 2: Injection timing (12x16)
    rpm2 = np.linspace(600, 6400, 12).astype(np.uint16)
    load2 = np.linspace(0, 1400, 16).astype(np.uint16)
    timing = np.outer(
        np.linspace(5, 35, 12),
        np.linspace(0.8, 1.2, 16)
    ) * 10  # 0.1° resolution
    timing = timing.astype(np.int16)
    
    map2_offset = offset
    data[offset:offset + 24] = rpm2.tobytes()
    offset += 24
    data[offset:offset + 32] = load2.tobytes()
    offset += 32
    data[offset:offset + 384] = timing.tobytes()
    offset += 384
    generated_maps.append({
        "name": "Injection timing main",
        "offset": map2_offset,
        "size": 440,
        "dims": (12, 16),
        "x_range": (600, 6400),
        "y_range": (0, 1400),
        "data_range": (float(timing.min()), float(timing.max())),
        "data_mean": float(timing.mean()),
    })
    
    # Карта 3: Speed limiter (скаляр)
    map3_offset = offset
    data[offset:offset + 2] = struct.pack("<H", 250)  # 250 km/h
    offset += 2
    generated_maps.append({
        "name": "Speed limiter",
        "offset": map3_offset,
        "size": 2,
        "dims": (1, 1),
        "x_range": (0, 0),
        "y_range": (0, 0),
        "data_range": (250, 250),
        "data_mean": 250.0,
    })
    
    # Карта 4: Lambda target (16x16)
    map4_offset = offset
    lambda_data = np.ones((16, 16), dtype=np.uint16) * 1000  # Lambda 1.0
    # Rich zone under boost
    lambda_data[8:, 8:] = 850  # Lambda 0.85 (rich under boost)
    data[offset:offset + 512] = lambda_data.tobytes()
    offset += 512
    generated_maps.append({
        "name": "Lambda target",
        "offset": map4_offset,
        "size": 512,
        "dims": (16, 16),
        "x_range": (800, 6800),
        "y_range": (0, 1600),
        "data_range": (float(lambda_data.min()), float(lambda_data.max())),
        "data_mean": float(lambda_data.mean()),
    })
    
    regions["calibration"] = {
        "offset": maps_start,
        "size": offset - maps_start,
        "type": "calibration",
        "maps": generated_maps,
    }
    
    # 4. DTC area
    dtc_offset = offset
    dtc_codes = [
        (0x0420, 0x01),  # P0420 - Catalyst efficiency
        (0x2463, 0x08),  # P2463 - DPF soot
        (0x0401, 0x00),  # P0401 - EGR insufficient (inactive)
    ]
    for code, status in dtc_codes:
        data[offset:offset + 2] = struct.pack("<H", code)
        data[offset + 2] = status
        offset += 4
    regions["dtc"] = {"offset": dtc_offset, "size": offset - dtc_offset, "type": "dtc"}
    
    # 5. Пустая область (0xFF)
    empty_start = offset
    data[offset:offset + 50000] = b"\xFF" * 50000
    offset += 50000
    regions["empty"] = {"offset": empty_start, "size": 50000, "type": "empty"}
    
    return bytes(data), regions, generated_maps


# ═══════════════════════════════════════════════════════════
# ЧАСТЬ 2: Анализ бинарника (работает локально на CPU)
# ═══════════════════════════════════════════════════════════

def analyze_binary_regions(data: bytes, block_size: int = 4096) -> list[dict]:
    """Локальный анализ: энтропия, статистика, эвристики. Работает на ЛЮБОМ CPU."""
    regions = []
    arr = np.frombuffer(data, dtype=np.uint8)
    
    for i in range(0, len(arr), block_size):
        block = arr[i:i + block_size]
        if len(block) < 256:
            continue
        
        # Энтропия Шеннона
        counts = np.bincount(block, minlength=256)
        probs = counts[counts > 0] / len(block)
        entropy = -np.sum(probs * np.log2(probs))
        
        # Статистики
        mean_val = float(block.mean())
        std_val = float(block.std())
        zero_ratio = float(np.sum(block == 0) / len(block))
        ff_ratio = float(np.sum(block == 0xFF) / len(block))
        
        # Проверка на монотонные последовательности (оси карт)
        # Ищем uint16 значения, которые монотонно возрастают
        has_monotonic = False
        if len(block) >= 16:
            u16 = np.frombuffer(block.tobytes(), dtype=np.uint16)
            for start in range(0, min(len(u16) - 8, 50)):
                seq = u16[start:start + 8]
                if np.all(np.diff(seq.astype(np.int32)) > 0):
                    has_monotonic = True
                    break
        
        # Классификация региона
        if ff_ratio > 0.95:
            region_type = "empty"
        elif entropy > 6.5:
            region_type = "code"
        elif entropy < 1.0:
            region_type = "empty"
        elif 2.5 < entropy < 6.0 and has_monotonic:
            region_type = "calibration_likely"
        elif 2.5 < entropy < 6.0:
            region_type = "data"
        else:
            region_type = "unknown"
        
        regions.append({
            "offset": i,
            "size": len(block),
            "entropy": round(entropy, 2),
            "mean": round(mean_val, 1),
            "std": round(std_val, 1),
            "zero_ratio": round(zero_ratio, 3),
            "ff_ratio": round(ff_ratio, 3),
            "has_monotonic": has_monotonic,
            "type": region_type,
        })
    
    return regions


def find_maps_heuristic(data: bytes) -> list[dict]:
    """Эвристический поиск карт без ML. Медленнее, но работает на CPU."""
    arr = np.frombuffer(data, dtype=np.uint8)
    u16 = np.frombuffer(data, dtype=np.uint16)
    candidates = []
    
    # Ищем монотонные последовательности uint16 (оси карт)
    for i in range(len(u16) - 16):
        seq = u16[i:i + 16].astype(np.int32)
        diffs = np.diff(seq)
        
        # Ось: все разности > 0, разумный диапазон
        if np.all(diffs > 0) and seq[0] > 100 and seq[-1] < 10000:
            byte_offset = i * 2
            axis_len = 16
            
            # Проверяем: есть ли вторая ось рядом?
            for j in range(i + 16, min(i + 48, len(u16) - 8)):
                seq2 = u16[j:j + 16].astype(np.int32)
                diffs2 = np.diff(seq2[:8])
                if np.all(diffs2 > 0):
                    # Нашли две оси! Между ними или после — данные карты
                    data_start = (j + 16) * 2
                    data_size = axis_len * 16 * 2  # 16x16 uint16
                    candidates.append({
                        "offset": byte_offset,
                        "x_axis_offset": byte_offset,
                        "y_axis_offset": j * 2,
                        "data_offset": data_start,
                        "likely_dims": (16, 16),
                        "x_range": (int(seq[0]), int(seq[-1])),
                        "y_range": (int(seq2[0]), int(seq2[min(15, len(seq2)-1)])),
                        "confidence": 0.6,
                    })
                    break
    
    return candidates


# ═══════════════════════════════════════════════════════════
# ЧАСТЬ 3: AI Providers
# ═══════════════════════════════════════════════════════════

class MockProvider:
    """Заглушка — работает без API и без Ollama."""
    name = "Mock (demo)"
    
    async def classify_map(self, map_info: dict) -> dict:
        await asyncio.sleep(0.1)  # Имитация задержки
        
        x_min, x_max = map_info.get("x_range", (0, 0))
        data_mean = map_info.get("data_mean", 0)
        dims = map_info.get("dims", (1, 1))
        
        # Простая эвристика
        if dims == (1, 1):
            return {"name": "Scalar value", "category": "limiters", "confidence": 0.5}
        if x_max > 6000:
            if data_mean > 1000:
                return {"name": "Boost target", "category": "boost", "confidence": 0.7}
            elif data_mean > 100:
                return {"name": "Injection timing", "category": "fuel", "confidence": 0.6}
        if 800 < data_mean < 1100:
            return {"name": "Lambda target", "category": "fuel", "confidence": 0.65}
        return {"name": "Unknown map", "category": "other", "confidence": 0.3}
    
    async def explain_dtc(self, code: str) -> str:
        await asyncio.sleep(0.1)
        explanations = {
            "P0420": "Эффективность катализатора ниже порога. Частые причины: изношенный катализатор, проблемы с лямбда-зондами, подсос воздуха.",
            "P2463": "Чрезмерное накопление сажи в DPF. Частые причины: частые короткие поездки, неисправность датчика дифференциального давления, засорённый фильтр.",
            "P0401": "Недостаточный поток EGR. Частые причины: засорён клапан EGR, забит канал рециркуляции, неисправность вакуумного привода.",
        }
        return explanations.get(code, f"Код {code}: описание недоступно в демо-режиме.")
    
    async def chat(self, message: str) -> str:
        await asyncio.sleep(0.1)
        return f"[DEMO] Это демо-режим. В реальной работе здесь будет ответ от LLM. Ваш вопрос: '{message[:50]}...'"


class ClaudeProvider:
    """Облачный провайдер — Claude API."""
    name = "Claude (Anthropic)"
    
    def __init__(self):
        import anthropic
        self.client = anthropic.Anthropic()
        self.model = "claude-sonnet-4-20250514"
    
    async def classify_map(self, map_info: dict) -> dict:
        prompt = f"""You are an ECU calibration expert. Classify this map.

ECU: Bosch MED17.5.2 (gasoline DI turbo)
Dimensions: {map_info['dims']}
X-axis range: {map_info.get('x_range', 'unknown')}
Y-axis range: {map_info.get('y_range', 'unknown')}
Data range: {map_info.get('data_range', 'unknown')}
Data mean: {map_info.get('data_mean', 'unknown')}

Respond ONLY with JSON: {{"name": "...", "category": "fuel|boost|timing|torque|emissions|limiters|other", "confidence": 0.0-1.0, "description": "one sentence"}}"""
        
        response = self.client.messages.create(
            model=self.model,
            max_tokens=300,
            messages=[{"role": "user", "content": prompt}],
        )
        text = response.content[0].text.strip()
        text = text.strip("`").removeprefix("json").strip()
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return {"name": "Parse error", "category": "other", "confidence": 0, "raw": text}
    
    async def explain_dtc(self, code: str) -> str:
        response = self.client.messages.create(
            model=self.model,
            max_tokens=500,
            messages=[{"role": "user", "content": f"Объясни DTC код {code} для Bosch MED17 (бензин TSI). Кратко по-русски: что значит, причины, что делать."}],
        )
        return response.content[0].text
    
    async def chat(self, message: str) -> str:
        response = self.client.messages.create(
            model=self.model,
            max_tokens=1000,
            system="Ты AI-ассистент в программе чип-тюнинга Daedalus. Помогаешь с ECU калибровками, диагностикой и тюнингом. Отвечай по-русски, кратко и по делу.",
            messages=[{"role": "user", "content": message}],
        )
        return response.content[0].text


class OllamaProvider:
    """Локальный провайдер — Ollama."""
    name = "Ollama (local)"
    
    def __init__(self, model: str = "phi3:3.8b-mini-4k-instruct-q4_K_M"):
        import httpx
        self.client = httpx.AsyncClient(timeout=120)
        self.endpoint = "http://localhost:11434"
        self.model = model
    
    async def _generate(self, prompt: str, system: str = "") -> str:
        resp = await self.client.post(
            f"{self.endpoint}/api/generate",
            json={
                "model": self.model,
                "prompt": prompt,
                "system": system,
                "stream": False,
                "options": {"num_ctx": 4096, "temperature": 0.3},
            },
        )
        resp.raise_for_status()
        return resp.json()["response"]
    
    async def classify_map(self, map_info: dict) -> dict:
        prompt = f"""Classify this ECU calibration map. ECU: Bosch MED17.5.2 gasoline turbo.
Dims: {map_info['dims']}, X: {map_info.get('x_range')}, Y: {map_info.get('y_range')}, Mean: {map_info.get('data_mean')}
Reply ONLY JSON: {{"name":"...","category":"fuel|boost|timing|limiters|other","confidence":0.0-1.0}}"""
        
        text = await self._generate(prompt, "You are an ECU calibration expert. Reply ONLY with valid JSON.")
        try:
            # Пытаемся извлечь JSON из ответа
            start = text.find("{")
            end = text.rfind("}") + 1
            if start >= 0 and end > start:
                return json.loads(text[start:end])
        except (json.JSONDecodeError, ValueError):
            pass
        return {"name": "Parse error", "category": "other", "confidence": 0, "raw": text[:200]}
    
    async def explain_dtc(self, code: str) -> str:
        return await self._generate(
            f"Объясни DTC код {code} для Bosch MED17 (бензиновый TSI мотор). Кратко по-русски.",
            "Ты автомобильный диагност-эксперт."
        )
    
    async def chat(self, message: str) -> str:
        return await self._generate(message, "Ты AI-ассистент чип-тюнинга. Отвечай по-русски.")


# ═══════════════════════════════════════════════════════════
# ЧАСТЬ 4: Тестовый пайплайн
# ═══════════════════════════════════════════════════════════

async def run_test(mode: str):
    """Полный тестовый пайплайн: генерация → анализ → классификация → DTC."""
    
    # Выбираем провайдер
    if mode == "cloud":
        if not os.environ.get("ANTHROPIC_API_KEY"):
            rprint("[red]Ошибка: установи ANTHROPIC_API_KEY[/red]")
            rprint("  Windows: set ANTHROPIC_API_KEY=sk-ant-...")
            rprint("  Linux:   export ANTHROPIC_API_KEY=sk-ant-...")
            return
        provider = ClaudeProvider()
    elif mode == "local":
        try:
            import httpx
            resp = httpx.get("http://localhost:11434/api/tags", timeout=5)
            resp.raise_for_status()
            provider = OllamaProvider()
        except Exception:
            rprint("[red]Ошибка: Ollama не запущена на localhost:11434[/red]")
            rprint("  1. Установи: https://ollama.com/download")
            rprint("  2. Запусти: ollama serve")
            rprint("  3. Скачай модель: ollama pull phi3:3.8b-mini-4k-instruct-q4_K_M")
            return
    else:
        provider = MockProvider()
    
    rprint(f"\n[bold]═══ Daedalus Test Harness ═══[/bold]")
    rprint(f"[bold]Режим:[/bold] {mode.upper()} | [bold]Provider:[/bold] {provider.name}\n")
    
    # ── Шаг 1: Генерация бинарника ──
    rprint("[bold cyan]▸ Шаг 1: Генерация тестового бинарника ECU[/bold cyan]")
    binary, regions, ground_truth_maps = generate_fake_ecu_binary(512)
    rprint(f"  Размер: {len(binary) // 1024} КБ")
    rprint(f"  Регионы: code ({regions['code']['size']//1024} КБ), "
           f"calibration ({regions['calibration']['size']} байт, {len(ground_truth_maps)} карт), "
           f"DTC, empty")
    rprint(f"  ECU: Bosch MED17.5.2, VW 06L906022FT\n")
    
    # ── Шаг 2: Анализ регионов (всегда локально) ──
    rprint("[bold cyan]▸ Шаг 2: Локальный анализ бинарника (CPU)[/bold cyan]")
    t0 = time.time()
    analysis = analyze_binary_regions(binary)
    t1 = time.time()
    
    type_counts = {}
    for r in analysis:
        type_counts[r["type"]] = type_counts.get(r["type"], 0) + 1
    
    rprint(f"  Время: {(t1-t0)*1000:.0f} мс")
    rprint(f"  Блоков: {len(analysis)}")
    for rtype, count in sorted(type_counts.items()):
        rprint(f"    {rtype}: {count} блоков")
    
    cal_blocks = [r for r in analysis if r["type"] == "calibration_likely"]
    rprint(f"  Найдено калибровочных регионов: {len(cal_blocks)}\n")
    
    # ── Шаг 3: Эвристический поиск карт (всегда локально) ──
    rprint("[bold cyan]▸ Шаг 3: Эвристический поиск карт (CPU)[/bold cyan]")
    t0 = time.time()
    found_maps = find_maps_heuristic(binary)
    t1 = time.time()
    rprint(f"  Время: {(t1-t0)*1000:.0f} мс")
    rprint(f"  Кандидатов: {len(found_maps)}")
    for fm in found_maps:
        rprint(f"    offset=0x{fm['offset']:06X}, dims={fm['likely_dims']}, "
               f"x={fm['x_range']}, confidence={fm['confidence']}\n")
    
    # ── Шаг 4: AI-классификация карт ──
    rprint(f"[bold cyan]▸ Шаг 4: Классификация карт через {provider.name}[/bold cyan]")
    
    if HAS_RICH:
        table = Table(title="Результаты классификации")
        table.add_column("Ground Truth", style="dim")
        table.add_column("AI Result", style="bold")
        table.add_column("Category")
        table.add_column("Confidence")
        table.add_column("Время")
    
    api_calls = 0
    for gt_map in ground_truth_maps:
        t0 = time.time()
        result = await provider.classify_map(gt_map)
        t1 = time.time()
        api_calls += 1
        
        ai_name = result.get("name", "?")
        category = result.get("category", "?")
        conf = result.get("confidence", 0)
        elapsed = f"{(t1-t0)*1000:.0f}ms"
        
        if HAS_RICH:
            conf_style = "green" if conf > 0.7 else "yellow" if conf > 0.4 else "red"
            table.add_row(
                gt_map["name"],
                ai_name,
                category,
                f"[{conf_style}]{conf:.0%}[/{conf_style}]",
                elapsed,
            )
        else:
            rprint(f"  {gt_map['name']:30s} → {ai_name:25s} [{category}] conf={conf:.0%} ({elapsed})")
    
    if HAS_RICH:
        console.print(table)
    rprint(f"  API запросов: {api_calls}\n")
    
    # ── Шаг 5: DTC объяснение ──
    rprint(f"[bold cyan]▸ Шаг 5: Объяснение DTC через {provider.name}[/bold cyan]")
    
    dtc_codes = ["P0420", "P2463", "P0401"]
    for code in dtc_codes:
        t0 = time.time()
        explanation = await provider.explain_dtc(code)
        t1 = time.time()
        api_calls += 1
        
        rprint(f"\n  [bold]{code}[/bold] ({(t1-t0)*1000:.0f}ms):")
        # Обрезаем длинные ответы
        lines = explanation.strip().split("\n")
        for line in lines[:5]:
            rprint(f"    {line}")
        if len(lines) > 5:
            rprint(f"    ... (+{len(lines)-5} строк)")
    
    # ── Шаг 6: Чат ──
    rprint(f"\n[bold cyan]▸ Шаг 6: Чат-ассистент через {provider.name}[/bold cyan]")
    question = "Что нужно изменить для Stage 1 на Golf 7 2.0 TSI EA888 gen3?"
    rprint(f"  Вопрос: {question}")
    
    t0 = time.time()
    answer = await provider.chat(question)
    t1 = time.time()
    api_calls += 1
    
    rprint(f"  Время: {(t1-t0)*1000:.0f}ms")
    lines = answer.strip().split("\n")
    for line in lines[:8]:
        rprint(f"    {line}")
    if len(lines) > 8:
        rprint(f"    ... (+{len(lines)-8} строк)")
    
    # ── Итоги ──
    rprint(f"\n[bold]═══ Итоги ═══[/bold]")
    rprint(f"  Режим: {mode.upper()}")
    rprint(f"  Provider: {provider.name}")
    rprint(f"  Всего API/LLM запросов: {api_calls}")
    rprint(f"  Локальных вычислений: анализ бинарника + поиск карт (0 запросов)")
    
    if mode == "local":
        rprint(f"  [green]Всё работало ОФЛАЙН — 0 обращений в интернет[/green]")
    elif mode == "cloud":
        rprint(f"  [yellow]Все AI-задачи через API — нужен интернет[/yellow]")
    else:
        rprint(f"  [dim]Демо-режим — заглушки вместо AI[/dim]")
    
    rprint(f"\n[bold]Что дальше:[/bold]")
    if mode == "demo":
        rprint("  Попробуй --mode cloud (с API ключом) или --mode local (с Ollama)")
    rprint("  Этот же пайплайн будет внутри Tauri-приложения с GUI")
    rprint("  CAN-адаптер заменит generate_fake_ecu_binary() на реальное чтение ECU\n")


# ═══════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Daedalus Test Harness")
    parser.add_argument(
        "--mode",
        choices=["demo", "local", "cloud"],
        default="demo",
        help="demo=заглушки, local=Ollama, cloud=Claude API",
    )
    args = parser.parse_args()
    
    asyncio.run(run_test(args.mode))
