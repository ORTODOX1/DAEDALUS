import { useState, useRef, useEffect } from "react";
import { Send, Bot, User, Trash2 } from "lucide-react";
import type { ChatMessage } from "../../types";

export function AIChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: "1",
      role: "system",
      content: "Daedalus AI Assistant ready. I can help with ECU analysis, map classification, DTC explanation, and safety validation. Ask me anything about your firmware.",
      timestamp: Date.now(),
    },
  ]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || loading) return;

    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      role: "user",
      content: input,
      timestamp: Date.now(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setInput("");
    setLoading(true);

    // Mock AI response (replace with actual Tauri invoke)
    setTimeout(() => {
      const reply: ChatMessage = {
        id: crypto.randomUUID(),
        role: "assistant",
        content: getMockReply(userMsg.content),
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, reply]);
      setLoading(false);
    }, 1500);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Messages */}
      <div ref={scrollRef} className="flex-1 overflow-auto p-4 space-y-4">
        {messages.map((msg) => (
          <div key={msg.id} className={`flex gap-3 ${msg.role === "user" ? "justify-end" : ""}`}>
            {msg.role !== "user" && (
              <div className="w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center flex-shrink-0">
                <Bot size={16} className="text-amber-400" />
              </div>
            )}
            <div
              className={`max-w-[70%] rounded-lg px-4 py-3 text-sm ${
                msg.role === "user"
                  ? "bg-blue-500/20 text-blue-100 border border-blue-500/20"
                  : msg.role === "system"
                    ? "bg-zinc-800/50 text-zinc-400 border border-zinc-700/50"
                    : "bg-zinc-900 text-zinc-200 border border-zinc-800"
              }`}
            >
              <p className="whitespace-pre-wrap">{msg.content}</p>
              <span className="text-[10px] text-zinc-600 mt-1 block">
                {new Date(msg.timestamp).toLocaleTimeString()}
              </span>
            </div>
            {msg.role === "user" && (
              <div className="w-8 h-8 rounded-lg bg-blue-500/20 flex items-center justify-center flex-shrink-0">
                <User size={16} className="text-blue-400" />
              </div>
            )}
          </div>
        ))}
        {loading && (
          <div className="flex gap-3">
            <div className="w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center">
              <Bot size={16} className="text-amber-400 animate-pulse" />
            </div>
            <div className="bg-zinc-900 rounded-lg px-4 py-3 border border-zinc-800">
              <div className="flex gap-1">
                <div className="w-2 h-2 bg-zinc-600 rounded-full animate-bounce" />
                <div className="w-2 h-2 bg-zinc-600 rounded-full animate-bounce" style={{ animationDelay: "0.15s" }} />
                <div className="w-2 h-2 bg-zinc-600 rounded-full animate-bounce" style={{ animationDelay: "0.3s" }} />
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Input */}
      <div className="p-4 border-t border-zinc-800">
        <div className="flex gap-2">
          <button
            onClick={() => setMessages([])}
            className="px-3 py-2 bg-zinc-800 text-zinc-500 rounded-lg hover:text-zinc-300 transition-colors"
          >
            <Trash2 size={16} />
          </button>
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSend()}
            placeholder="Ask about ECU maps, DTC codes, safety checks..."
            className="flex-1 px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-zinc-600"
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || loading}
            className="px-4 py-2 bg-amber-500/20 text-amber-400 rounded-lg hover:bg-amber-500/30 transition-colors disabled:opacity-30"
          >
            <Send size={16} />
          </button>
        </div>
      </div>
    </div>
  );
}

function getMockReply(question: string): string {
  const q = question.toLowerCase();
  if (q.includes("rail") || q.includes("тнвд") || q.includes("давлен"))
    return "Rail Pressure (SPN 157) — ключевая карта для ТНВД Common Rail.\n\nОсновные параметры:\n- Ось X: обороты двигателя (RPM)\n- Ось Y: объём впрыска (mg/stroke)\n- Значения: давление в рампе (bar)\n\nДля EDC17C46 типичный диапазон: 250-1800 bar.\nОптимизация расхода: снижение давления на частичных нагрузках на 5-10% уменьшает расход на 2-4%.";
  if (q.includes("dpf") || q.includes("сажев"))
    return "DPF (SPN 3226) — сажевый фильтр.\n\nКлючевые карты:\n- DPF soot load threshold (порог регенерации)\n- Post injection quantity (впрыск для прожига)\n- Exhaust temperature target (целевая температура)\n\nВНИМАНИЕ: Удаление DPF запрещено в большинстве стран для грузовиков Euro 5/6.";
  if (q.includes("egr"))
    return "EGR (SPN 3216) — рециркуляция выхлопных газов.\n\nОптимизация для грузовиков:\n- Снижение EGR rate на 20-30% улучшает расход топлива\n- Но увеличивает NOx выбросы\n- Для Euro 6 SCR компенсирует NOx через AdBlue";
  return "Для анализа мне нужно больше контекста. Укажите:\n1. Тип ECU (EDC17, MD1, CM2350...)\n2. Марку и модель грузовика\n3. Что именно нужно оптимизировать (расход, мощность, AdBlue)\n\nЯ могу помочь с анализом карт, расшифровкой DTC кодов, проверкой безопасности модификаций.";
}
