import { useState, useMemo } from "react";
import {
  SlidersHorizontal,
  TrendingUp,
  TrendingDown,
  Flame,
  Droplets,
  Wind,
  Zap,
  ArrowRight,
} from "lucide-react";
import {
  RadarChart,
  PolarGrid,
  PolarAngleAxis,
  PolarRadiusAxis,
  Radar,
  ResponsiveContainer,
} from "recharts";

/* --- Types --- */

interface SliderConfig {
  id: string;
  label: string;
  unit: string;
  min: number;
  max: number;
  step: number;
  defaultValue: number;
  icon: typeof Zap;
}

interface PredictionResult {
  id: string;
  label: string;
  value: number;
  unit: string;
  positive: boolean; // whether higher = better
  icon: typeof TrendingUp;
}

/* --- Constants --- */

const SLIDERS: SliderConfig[] = [
  { id: "boost",    label: "Boost Delta",          unit: "bar",  min: -0.5, max: 1.0,   step: 0.05, defaultValue: 0, icon: Wind },
  { id: "timing",   label: "Injection Timing",     unit: "\u00B0",    min: -5,   max: 5,     step: 0.5,  defaultValue: 0, icon: Zap },
  { id: "rail",     label: "Rail Pressure Delta",  unit: "bar",  min: -200, max: 200,   step: 10,   defaultValue: 0, icon: Droplets },
  { id: "egr",      label: "EGR Rate",             unit: "%",    min: -30,  max: 30,    step: 1,    defaultValue: 0, icon: Flame },
];

/* --- Prediction formulas --- */

function computePredictions(values: Record<string, number>) {
  const boost = values.boost ?? 0;
  const timing = values.timing ?? 0;
  const rail = values.rail ?? 0;
  const egr = values.egr ?? 0;

  const powerDelta = boost * 25 + timing * 3 + rail * 0.015;
  const torqueDelta = boost * 40 + timing * 5 + rail * 0.02;
  const fuelDelta = -(boost * 2 + egr * 0.1) + Math.abs(timing) * 0.3;
  const egtDelta = boost * 60 + timing * 15 + Math.abs(rail) * 0.05;
  const knockRisk = Math.max(0, Math.min(100, boost * 30 + timing * 8 - 5 + Math.max(0, rail * 0.02)));
  const turboLifespan = -(boost * 8 + Math.abs(timing) * 1.5 + Math.max(0, egtDelta * 0.05));
  const noxDelta = -(egr * 1.5) + boost * 5 + timing * 2;

  const predictions: PredictionResult[] = [
    { id: "power",    label: "Power",            value: +powerDelta.toFixed(1),    unit: "HP",   positive: true,  icon: TrendingUp },
    { id: "torque",   label: "Torque",           value: +torqueDelta.toFixed(1),   unit: "Nm",   positive: true,  icon: TrendingUp },
    { id: "fuel",     label: "Fuel Consumption", value: +fuelDelta.toFixed(1),     unit: "%",    positive: false, icon: Droplets },
    { id: "egt",      label: "EGT Change",       value: +egtDelta.toFixed(0),      unit: "\u00B0C",   positive: false, icon: Flame },
    { id: "knock",    label: "Knock Risk",       value: +knockRisk.toFixed(0),     unit: "%",    positive: false, icon: Zap },
    { id: "turbo",    label: "Turbo Lifespan",   value: +turboLifespan.toFixed(1), unit: "%",    positive: true,  icon: Wind },
    { id: "nox",      label: "NOx Change",       value: +noxDelta.toFixed(1),      unit: "%",    positive: false, icon: Wind },
  ];

  return predictions;
}

function getValueColor(value: number, positive: boolean): string {
  if (value === 0) return "text-zinc-400";
  if (positive) {
    return value > 0 ? "text-green-400" : "text-red-400";
  }
  return value > 0 ? "text-red-400" : "text-green-400";
}

function formatDelta(value: number): string {
  if (value > 0) return `+${value}`;
  if (value === 0) return "0";
  return `${value}`;
}

/* --- Component --- */

export function WhatIfSimulator() {
  const [values, setValues] = useState<Record<string, number>>(() => {
    const init: Record<string, number> = {};
    for (const s of SLIDERS) init[s.id] = s.defaultValue;
    return init;
  });

  const predictions = useMemo(() => computePredictions(values), [values]);

  const radarData = useMemo(() => {
    const power = predictions.find((p) => p.id === "power")!;
    const torque = predictions.find((p) => p.id === "torque")!;
    const fuel = predictions.find((p) => p.id === "fuel")!;
    const egt = predictions.find((p) => p.id === "egt")!;
    const knock = predictions.find((p) => p.id === "knock")!;
    const turbo = predictions.find((p) => p.id === "turbo")!;
    const nox = predictions.find((p) => p.id === "nox")!;

    // Normalize to 0-100 scale for radar
    return [
      { subject: "Power",    value: Math.min(100, Math.max(0, power.value * 2 + 50)) },
      { subject: "Torque",   value: Math.min(100, Math.max(0, torque.value + 50)) },
      { subject: "Economy",  value: Math.min(100, Math.max(0, 50 - fuel.value * 5)) },
      { subject: "Thermal",  value: Math.min(100, Math.max(0, 80 - egt.value * 0.1)) },
      { subject: "Safety",   value: Math.min(100, Math.max(0, 100 - knock.value)) },
      { subject: "Durability", value: Math.min(100, Math.max(0, 70 + turbo.value)) },
      { subject: "Emissions", value: Math.min(100, Math.max(0, 60 - nox.value * 0.5)) },
    ];
  }, [predictions]);

  const handleSliderChange = (id: string, value: number) => {
    setValues((prev) => ({ ...prev, [id]: value }));
  };

  const handleReset = () => {
    const init: Record<string, number> = {};
    for (const s of SLIDERS) init[s.id] = s.defaultValue;
    setValues(init);
  };

  const hasChanges = SLIDERS.some((s) => values[s.id] !== s.defaultValue);

  const knockRisk = predictions.find((p) => p.id === "knock")?.value ?? 0;
  const overallRisk = knockRisk > 60 ? "high" : knockRisk > 30 ? "medium" : "low";

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-amber-500/10 flex items-center justify-center">
            <SlidersHorizontal size={16} className="text-amber-400" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-zinc-200">What-If Simulator</h2>
            <p className="text-xs text-zinc-500">Predict outcomes of parameter changes</p>
          </div>
        </div>
        <div className="flex gap-2">
          {hasChanges && (
            <button
              onClick={handleReset}
              className="px-3 py-1.5 bg-zinc-800 text-zinc-400 rounded-lg text-xs hover:text-zinc-200 transition-colors border border-zinc-700"
            >
              Reset
            </button>
          )}
          <button
            disabled={!hasChanges}
            className="flex items-center gap-2 px-4 py-1.5 bg-amber-500/10 text-amber-400 border border-amber-500/20 rounded-lg text-xs hover:bg-amber-500/20 transition-colors disabled:opacity-30"
          >
            <ArrowRight size={12} />
            Apply to Safety Debate
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4">
        <div className="grid grid-cols-2 gap-6">
          {/* Left: Sliders */}
          <div className="space-y-5">
            <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider">Parameters</h3>

            {SLIDERS.map((slider) => {
              const val = values[slider.id];
              const Icon = slider.icon;
              const pct = ((val - slider.min) / (slider.max - slider.min)) * 100;

              return (
                <div key={slider.id} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Icon size={14} className="text-zinc-500" />
                      <span className="text-sm text-zinc-300">{slider.label}</span>
                    </div>
                    <span className={`text-sm font-mono font-bold ${
                      val === 0 ? "text-zinc-500" : val > 0 ? "text-amber-400" : "text-blue-400"
                    }`}>
                      {formatDelta(val)} {slider.unit}
                    </span>
                  </div>

                  <div className="relative">
                    <input
                      type="range"
                      min={slider.min}
                      max={slider.max}
                      step={slider.step}
                      value={val}
                      onChange={(e) => handleSliderChange(slider.id, +e.target.value)}
                      className="w-full h-2 bg-zinc-800 rounded-full appearance-none cursor-pointer
                        [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4
                        [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-amber-400
                        [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:shadow-amber-500/20
                        [&::-webkit-slider-thumb]:cursor-pointer"
                    />
                    {/* Zero marker */}
                    <div
                      className="absolute top-1/2 -translate-y-1/2 w-0.5 h-4 bg-zinc-600 pointer-events-none"
                      style={{ left: `${((0 - slider.min) / (slider.max - slider.min)) * 100}%` }}
                    />
                  </div>

                  <div className="flex justify-between text-[10px] text-zinc-600">
                    <span>{slider.min} {slider.unit}</span>
                    <span>{slider.max} {slider.unit}</span>
                  </div>
                </div>
              );
            })}

            {/* Risk indicator */}
            <div className={`mt-4 p-3 rounded-lg border ${
              overallRisk === "high" ? "bg-red-500/5 border-red-500/20"
              : overallRisk === "medium" ? "bg-yellow-500/5 border-yellow-500/20"
              : "bg-green-500/5 border-green-500/20"
            }`}>
              <div className="flex items-center gap-2 mb-2">
                <div className={`w-2 h-2 rounded-full ${
                  overallRisk === "high" ? "bg-red-400" : overallRisk === "medium" ? "bg-yellow-400" : "bg-green-400"
                }`} />
                <span className={`text-xs font-semibold uppercase ${
                  overallRisk === "high" ? "text-red-400" : overallRisk === "medium" ? "text-yellow-400" : "text-green-400"
                }`}>
                  {overallRisk} risk
                </span>
              </div>
              <div className="w-full h-2 bg-zinc-800 rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all duration-500 ${
                    overallRisk === "high" ? "bg-red-500" : overallRisk === "medium" ? "bg-yellow-500" : "bg-green-500"
                  }`}
                  style={{ width: `${Math.min(100, knockRisk)}%` }}
                />
              </div>
            </div>
          </div>

          {/* Right: Predictions + Radar */}
          <div className="space-y-5">
            <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider">Predicted Outcomes</h3>

            {/* Prediction cards */}
            <div className="grid grid-cols-2 gap-3">
              {predictions.map((pred) => {
                const Icon = pred.icon;
                const colorClass = getValueColor(pred.value, pred.positive);

                return (
                  <div
                    key={pred.id}
                    className="bg-zinc-900/60 border border-zinc-800 rounded-lg p-3 space-y-1"
                  >
                    <div className="flex items-center gap-2">
                      <Icon size={12} className="text-zinc-500" />
                      <span className="text-[11px] text-zinc-500">{pred.label}</span>
                    </div>
                    <div className="flex items-baseline gap-1">
                      <span className={`text-xl font-bold font-mono ${colorClass}`}>
                        {formatDelta(pred.value)}
                      </span>
                      <span className="text-xs text-zinc-600">{pred.unit}</span>
                    </div>
                    {/* Mini bar */}
                    {pred.id === "knock" && (
                      <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden mt-1">
                        <div
                          className={`h-full rounded-full transition-all duration-300 ${
                            pred.value > 60 ? "bg-red-500" : pred.value > 30 ? "bg-yellow-500" : "bg-green-500"
                          }`}
                          style={{ width: `${Math.min(100, Math.max(0, pred.value))}%` }}
                        />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>

            {/* Radar chart */}
            <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-4">
              <h4 className="text-xs text-zinc-500 font-semibold mb-2">Impact Profile</h4>
              <div className="h-64">
                <ResponsiveContainer width="100%" height="100%">
                  <RadarChart data={radarData} cx="50%" cy="50%" outerRadius="75%">
                    <PolarGrid stroke="#27272a" />
                    <PolarAngleAxis
                      dataKey="subject"
                      tick={{ fill: "#71717a", fontSize: 11 }}
                    />
                    <PolarRadiusAxis
                      angle={90}
                      domain={[0, 100]}
                      tick={false}
                      axisLine={false}
                    />
                    <Radar
                      name="Impact"
                      dataKey="value"
                      stroke="#f59e0b"
                      fill="#f59e0b"
                      fillOpacity={0.15}
                      strokeWidth={2}
                    />
                  </RadarChart>
                </ResponsiveContainer>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
