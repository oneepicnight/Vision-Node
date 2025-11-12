import Pane from "../../../components/Pane";
import { useExchange } from "../store";

export default function TradesTapePro(){
  const { trades } = useExchange();
  return (
    <Pane title="Trades" className="h-full">
      <div className="h-[260px] overflow-auto">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-[var(--panel)]/90 backdrop-blur">
            <tr><th className="text-left p-2">Time</th><th className="text-right p-2">Price</th><th className="text-right p-2">Size</th><th className="p-2">Side</th></tr>
          </thead>
          <tbody>
            {trades.slice(0,80).map(t=>(
              <tr key={t.id} className="border-b border-white/5">
                <td className="p-2">{new Date(t.ts).toLocaleTimeString()}</td>
                <td className={`p-2 text-right ${t.side==="buy"?"text-emerald-400":"text-red-400"}`}>{t.price.toFixed(2)}</td>
                <td className="p-2 text-right">{t.size.toFixed(3)}</td>
                <td className="p-2 uppercase">{t.side}</td>
              </tr>
            ))}
            {!trades.length && <tr><td colSpan={4} className="p-6 text-center text-[var(--muted)]">No trades yet</td></tr>}
          </tbody>
        </table>
      </div>
    </Pane>
  );
}
