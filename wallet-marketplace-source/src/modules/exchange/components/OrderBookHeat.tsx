import Pane from "../../../components/Pane";
import { useExchange } from "../store";

function HeatRow({price,size,side,max}:{price:number;size:number;side:"ask"|"bid";max:number}) {
  const pct = Math.min(100, (size / max) * 100);
  const bg = side==="ask" ? "bg-red-500/20" : "bg-emerald-500/20";
  const hover = side==="ask" ? "hover:bg-red-500/30" : "hover:bg-emerald-500/30";
  return (
    <tr className={`relative ${hover}`} data-p={price}>
      <td className="py-1 pr-2 z-10">{price.toFixed(2)}</td>
      <td className="py-1 text-right z-10">{size.toFixed(3)}</td>
      <td className="absolute left-0 top-0 h-full" style={{width:`${pct}%`}}>
        <div className={`${bg} w-full h-full`} />
      </td>
    </tr>
  );
}

export default function OrderBookHeat({ onPickPrice }:{ onPickPrice:(p:number)=>void }){
  const { asks, bids } = useExchange();
  const maxAsk = Math.max(0, ...asks.map(a=>a.size));
  const maxBid = Math.max(0, ...bids.map(b=>b.size));

  return (
    <Pane title="Order Book" className="h-full">
      <div className="grid grid-rows-2 gap-3 h-full">
        <div className="overflow-auto rounded-lg border border-white/5">
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-[var(--panel)]/90 backdrop-blur">
              <tr><th className="text-left p-2">Price</th><th className="text-right p-2">Size</th></tr>
            </thead>
            <tbody onClick={(e:any)=>{ const p = parseFloat(e.target.closest("tr")?.dataset?.p); if(!isNaN(p)) onPickPrice(p); }}>
              {asks.slice(0,20).map(l=>(
                <HeatRow key={`a-${l.price}`} price={l.price} size={l.size} side="ask" max={maxAsk} />
              ))}
            </tbody>
          </table>
        </div>
        <div className="overflow-auto rounded-lg border border-white/5">
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-[var(--panel)]/90 backdrop-blur">
              <tr><th className="text-left p-2">Price</th><th className="text-right p-2">Size</th></tr>
            </thead>
            <tbody onClick={(e:any)=>{ const p = parseFloat(e.target.closest("tr")?.dataset?.p); if(!isNaN(p)) onPickPrice(p); }}>
              {bids.slice(0,20).map(l=>(
                <HeatRow key={`b-${l.price}`} price={l.price} size={l.size} side="bid" max={maxBid} />
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </Pane>
  );
}
