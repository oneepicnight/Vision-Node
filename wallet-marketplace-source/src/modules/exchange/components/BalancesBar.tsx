import { Wallet } from "lucide-react";
import Pane from "../../../components/Pane";
import { useExchange } from "../store";

export default function BalancesBar(){
  const { balances } = useExchange();
  return (
    <Pane>
      <div className="flex items-center gap-4 text-sm">
        <Wallet size={16} className="opacity-70" />
        {Object.entries(balances).map(([c,b])=>(
          <div key={c} className="px-3 py-1.5 rounded-lg bg-black/30 border border-white/5">
            <span className="font-semibold">{c}</span>
            <span className="mx-2 text-[var(--muted)]">Avail</span>
            <span className="font-semibold text-[var(--text)]">{(b.available||0).toFixed(6)}</span>
            <span className="mx-2 text-[var(--muted)]">Locked</span>
            <span className="font-semibold">{(b.locked||0).toFixed(6)}</span>
          </div>
        ))}
      </div>
    </Pane>
  );
}
