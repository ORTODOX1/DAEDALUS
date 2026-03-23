import React from "react";

export default function App() {
  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100">
      {/* Sidebar */}
      <aside className="w-64 border-r border-zinc-800 p-4">
        <h1 className="text-xl font-bold mb-8">◊ Daedalus</h1>
        <p className="text-xs text-zinc-500 mb-6">Master the labyrinth</p>
        <nav className="space-y-2">
          {["Connection", "DTC", "Live Data", "Flash", "Editor", "AI Assistant"].map((item) => (
            <button key={item} className="w-full text-left px-3 py-2 rounded hover:bg-zinc-800 transition">
              {item}
            </button>
          ))}
        </nav>
      </aside>
      
      {/* Main */}
      <main className="flex-1 flex flex-col">
        {/* Top bar */}
        <header className="h-12 border-b border-zinc-800 flex items-center px-4 gap-4">
          <span className="h-2 w-2 rounded-full bg-red-500" />
          <span className="text-sm text-zinc-500">No ECU connected</span>
        </header>
        
        {/* Work area */}
        <div className="flex-1 p-6 flex items-center justify-center text-zinc-600">
          Connect an adapter to begin
        </div>
        
        {/* Status bar */}
        <footer className="h-8 border-t border-zinc-800 px-4 flex items-center text-xs text-zinc-600">
          Daedalus v0.1.0 — Master the labyrinth | No adapter detected
        </footer>
      </main>
    </div>
  );
}
