import { useExchange } from "../store";

export default function TradesTape(){
  const { trades } = useExchange();
  return (
    <div className="card-exchange trades-tape">
      <div className="text-sm font-semibold mb-2">Trades</div>
      <div style={{ maxHeight: 240, overflow: 'auto' }}>
      <table className="w-full text-sm">
        <thead><tr><th className="text-left">Time</th><th className="text-right">Price</th><th className="text-right">Size</th><th>Side</th></tr></thead>
        <tbody>
          {trades.slice(0,60).map(t=>(
            <tr key={t.id} style={{ color: t.side==="buy"? 'var(--green)': 'var(--red)' }}>
              <td>{new Date(t.ts).toLocaleTimeString()}</td>
              <td className="text-right">{t.price.toFixed(2)}</td>
              <td className="text-right">{t.size.toFixed(3)}</td>
              <td className="uppercase">{t.side}</td>
            </tr>
          ))}
        </tbody>
      </table>
      </div>
    </div>
  );
}
