import { ChevronDown, TrendingUp, Activity } from "lucide-react";
import { useExchange } from "../store";

export default function MarketHeaderPro() {
  const { chain, lastPrice, change24h, volume24h } = useExchange();
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-3 bg-[var(--panel-2)]/80 rounded-xl border border-white/5">
      <div className="flex items-center gap-3">
        <button className="inline-flex items-center gap-2 px-3 py-1.5 rounded-lg bg-black/30 border border-white/5 hover:border-white/10 transition">
          <span className="text-[var(--text)] font-semibold">{chain}/CASH</span>
          <ChevronDown size={16} className="opacity-70" />
        </button>
        <span className="text-sm text-[var(--muted)]">Quick market view</span>
      </div>
      <div className="flex items-center gap-2">
        <div className="px-3 py-1.5 rounded-lg bg-black/30 border border-white/5 text-sm">
          Last <span className="font-semibold text-[var(--text)]">{lastPrice?.toFixed(2) ?? "—"}</span>
        </div>
        <div className={`px-3 py-1.5 rounded-lg border text-sm ${ (change24h??0)>=0 ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400":"border-red-500/30 bg-red-500/10 text-red-400"}`}>
          <TrendingUp size={14} className="inline mr-1" /> {change24h ?? "—"}%
        </div>
        <div className="px-3 py-1.5 rounded-lg bg-black/30 border border-white/5 text-sm text-[var(--muted)]">
          <Activity size={14} className="inline mr-1 opacity-70" /> Vol 24h <span className="text-[var(--text)]">{volume24h ?? "—"}</span>
        </div>
      </div>
    </div>
  );
}
