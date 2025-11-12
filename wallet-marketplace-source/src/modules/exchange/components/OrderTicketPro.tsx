import Pane from "../../../components/Pane";
import { useState, useEffect } from "react";
import { useExchange } from "../store";

export default function OrderTicketPro({prefill}:{prefill?:number}) {
  const { placeSell, placeBuy, balances, chain } = useExchange();
  const [tab,setTab]=useState<"sell"|"buy">("sell");
  const [price,setPrice]=useState<number>(0);
  const [size,setSize]=useState<number>(0);
  const [postOnly,setPostOnly]=useState(true);
  const [tif,setTif]=useState<"GTC"|"IOC"|"GTT">("GTC");
  const [gtt,setGtt]=useState<string>("");

  useEffect(()=>{ if(prefill) setPrice(prefill); },[prefill]);
  const bal = balances[chain] || { available: 0, locked: 0 };

  return (
    <Pane title="Order Ticket">
      <div className="flex gap-2 mb-3">
        <button onClick={()=>setTab("sell")} className={`px-3 py-1.5 rounded-lg border ${tab==="sell"?"border-red-500/50 bg-red-500/15 text-red-300":"border-white/5 bg-black/20"}`}>Sell (List)</button>
        <button onClick={()=>setTab("buy")}  className={`px-3 py-1.5 rounded-lg border ${tab==="buy" ?"border-emerald-500/50 bg-emerald-500/15 text-emerald-300":"border-white/5 bg-black/20"}`}>Buy (Take)</button>
      </div>

      {tab==="sell" ? (
        <div className="flex flex-col gap-2">
          <label className="text-sm text-[var(--muted)]">Price</label>
          <input className="px-3 py-2 rounded-lg bg-black/30 border border-white/5 focus:outline-none focus:ring-2 focus:ring-[var(--ring)]" type="number" value={price||""} onChange={e=>setPrice(+e.target.value)} />
          <label className="text-sm text-[var(--muted)]">Amount</label>
          <input className="px-3 py-2 rounded-lg bg-black/30 border border-white/5 focus:outline-none focus:ring-2 focus:ring-[var(--ring)]" type="number" value={size||""} onChange={e=>setSize(+e.target.value)} />
          <div className="flex items-center gap-3">
            <label className="text-sm flex items-center gap-2"><input type="checkbox" checked={postOnly} onChange={e=>setPostOnly(e.target.checked)} /> Post-only</label>
            <label className="text-sm">TIF</label>
            <select className="px-2 py-1 rounded bg-black/30 border border-white/5" value={tif} onChange={e=>setTif(e.target.value as any)}>
              <option>GTC</option><option>IOC</option><option>GTT</option>
            </select>
            {tif==="GTT" && <input className="px-2 py-1 rounded bg-black/30 border border-white/5" type="datetime-local" value={gtt} onChange={e=>setGtt(e.target.value)} />}
          </div>
          <div className="text-xs text-[var(--muted)] mt-1">Available: {bal.available.toFixed(6)} • Locked: {bal.locked.toFixed(6)}</div>
          <button
            className="mt-2 w-full py-2 rounded-lg bg-red-600 text-white hover:bg-red-500 disabled:opacity-50"
            disabled={!(price>0 && size>0 && size<=bal.available)}
            onClick={async()=>{ await placeSell({price, size, post_only:postOnly, tif}); setSize(0); }}
          >Place Sell & Lock</button>
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          <label className="text-sm text-[var(--muted)]">Amount</label>
          <input className="px-3 py-2 rounded-lg bg-black/30 border border-white/5 focus:outline-none focus:ring-2 focus:ring-[var(--ring)]" type="number" value={size||""} onChange={e=>setSize(+e.target.value)} />
          <button
            className="mt-2 w-full py-2 rounded-lg bg-emerald-600 text-white hover:bg-emerald-500 disabled:opacity-50"
            disabled={!(size>0)}
            onClick={async()=>{ await placeBuy({ size }); setSize(0); }}
          >Buy (Market)</button>
        </div>
      )}
    </Pane>
  );
}
