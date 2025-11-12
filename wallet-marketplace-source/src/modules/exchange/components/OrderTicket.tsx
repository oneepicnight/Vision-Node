import { useState, useEffect } from "react";
import { useExchange } from "../store";

export default function OrderTicket(){
  const { placeSell, placeBuy, balances, chain, prefillPrice, setPrefillPrice } = useExchange();
  const [tab, setTab] = useState<"sell"|"buy">("sell");
  const [price, setPrice] = useState<number>(0);
  const [size, setSize] = useState<number>(0);
  const [spend, setSpend] = useState<number>(0);
  const [postOnly, setPostOnly] = useState(true);
  const [tif, setTif] = useState<"GTC"|"IOC"|"GTT">("GTC");
  const [gtTime, setGtTime] = useState<string>("");
  const bal = balances[chain] || { available: 0, locked: 0 };

  // apply prefillPrice from store when set
  useEffect(()=>{
    if(typeof prefillPrice === 'number'){
      setPrice(prefillPrice)
      // optionally clear after using
      setPrefillPrice(undefined)
  // show a toast
  window.pushToast?.(`Prefilled price ${prefillPrice.toFixed(2)}`,'info')
    }
  },[prefillPrice, setPrefillPrice])

  return (
    <div className="card-exchange order-ticket">
      <div className="tabs">
        <button className={`btn ${tab==="sell"?"sell":""}`} onClick={()=>setTab("sell")}>Sell (List)</button>
        <button className={`btn ${tab==="buy"?"buy":""}`} onClick={()=>setTab("buy")}>Buy (Take)</button>
      </div>

      {tab==="sell" ? (
        <div style={{ display:'flex', flexDirection:'column', gap:8 }}>
          <label className="text-sm">Price</label>
          <input type="number" value={price||""} onChange={e=>setPrice(+e.target.value)} />
          <label className="text-sm">Amount</label>
          <input type="number" value={size||""} onChange={e=>setSize(+e.target.value)} />
          <div className="text-xs text-gray-600">Available: {bal.available.toFixed(6)} • Locked: {bal.locked.toFixed(6)}</div>
          <label className="text-sm flex items-center gap-2">
            <input type="checkbox" checked={postOnly} onChange={e=>setPostOnly(e.target.checked)} />
            Post-only
          </label>
          <div className="flex items-center gap-2">
            <label className="text-sm">TIF</label>
            <select className="border rounded px-2 py-1" value={tif} onChange={e=>setTif(e.target.value as any)}>
              <option>GTC</option><option>IOC</option><option>GTT</option>
            </select>
            {tif==="GTT" && (
              <input className="border rounded px-2 py-1" type="datetime-local" value={gtTime} onChange={e=>setGtTime(e.target.value)} />
            )}
          </div>
          <button
            className="btn sell"
            disabled={!(price>0 && size>0 && size<=bal.available)}
            onClick={async ()=>{
              try { await placeSell({ price, size, post_only: postOnly, tif }); setSize(0); window.pushToast?.('Sell placed (locked)', 'success') } catch(e:any){ window.pushToast?.(e?.message||"Failed", 'error'); }
            }}
          >Place Sell (Lock)</button>
        </div>
      ) : (
        <div style={{ display:'flex', flexDirection:'column', gap:8 }}>
          <label className="text-sm">Spend (quote)</label>
          <input type="number" value={spend||""} onChange={e=>setSpend(+e.target.value)} />
          <label className="text-sm">or Size</label>
          <input type="number" value={size||""} onChange={e=>setSize(+e.target.value)} />
          <button
            className="btn buy"
            disabled={!((spend>0) || (size>0))}
            onClick={async ()=>{
              try { await placeBuy({ spend, size }); setSize(0); setSpend(0); window.pushToast?.('Buy executed', 'success') } catch(e:any){ window.pushToast?.(e?.message||"Failed", 'error'); }
            }}
          >Buy (Market)</button>
        </div>
      )}
    </div>
  );
}
