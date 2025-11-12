import React from 'react'
import { listOrders, replayMint, type CashOrder } from './ordersApi'

function cents(n:number){ return (n/100).toFixed(2); }
function ts(s:number){ return new Date(s*1000).toLocaleString(); }

export default function Orders(){
  const [rows,setRows]=React.useState<CashOrder[]>([]);
  const [buyer,setBuyer]=React.useState("");
  const [limit,setLimit]=React.useState(50);
  const [err,setErr]=React.useState<string|null>(null);
  const [loading,setLoading]=React.useState(false);
  const [cursor, setCursor] = React.useState<string|undefined>(undefined);
  const [nextCursor, setNextCursor] = React.useState<string|undefined>(undefined);

  const load = React.useCallback(async ()=>{
    try{
      setLoading(true);
      const data = await listOrders(limit, buyer || undefined, cursor);
      // server returns newest-first; update rows depending on page
      if (!cursor) {
        // first page replaces
        data.items.sort((a,b)=>b.updated_at - a.updated_at);
        setRows(data.items);
      } else {
        // append older page, de-dupe
        const merged = [...rows];
        for (const it of data.items) {
          if (!merged.find(r=>r.id===it.id)) merged.push(it);
        }
        merged.sort((a,b)=>b.updated_at - a.updated_at);
        setRows(merged);
      }
      setNextCursor(data.next_cursor ?? undefined);
      setErr(null);
    }catch(e:any){ setErr(String(e?.message ?? e)); }
    finally{ setLoading(false); }
  },[buyer,limit,cursor,rows]);

  // initial load + polling only when on first page (cursor undefined)
  React.useEffect(()=>{
    let alive=true;
    load();
    const id = setInterval(()=> { if (alive && cursor===undefined) load(); }, 3000);
    return ()=>{ alive=false; clearInterval(id); };
  },[load, cursor]);

  async function onReplay(id:string){
    try{
      await replayMint(id);
      await load();
    }catch(e:any){ window.pushToast?.("Replay failed: "+String(e?.message ?? e), 'error'); }
  }

  return (
    <div className="p-4 space-y-3">
      <div className="flex items-end gap-2">
        <div>
          <label className="block text-xs uppercase opacity-70">Buyer Addr</label>
          <input value={buyer} onChange={e=>setBuyer(e.target.value)} placeholder="V1..." className="border rounded px-2 py-1"/>
        </div>
        <div>
          <label className="block text-xs uppercase opacity-70">Limit</label>
          <input type="number" value={limit} onChange={e=>setLimit(parseInt(e.target.value||"50"))} className="border rounded px-2 py-1 w-24"/>
        </div>
  <button onClick={load} className="px-3 py-1 rounded bg-black text-white">Refresh</button>
        {loading && <span className="text-sm opacity-60">Loading…</span>}
        {err && <span className="text-sm text-red-600">Error: {err}</span>}
      </div>

      <div className="overflow-auto border rounded">
        <table className="min-w-full text-sm">
          <thead className="bg-black/5">
            <tr>
              <th className="text-left p-2">ID</th>
              <th className="text-left p-2">Buyer</th>
              <th className="text-right p-2">USD</th>
              <th className="text-right p-2">CASH</th>
              <th className="text-left p-2">Status</th>
              <th className="text-left p-2">Stripe</th>
              <th className="text-left p-2">Updated</th>
              <th className="text-left p-2"></th>
            </tr>
          </thead>
          <tbody>
            {rows.map(r=>(
              <tr key={r.id} className="border-t">
                <td className="p-2">{r.id.slice(0,8)}…</td>
                <td className="p-2">{r.buyer_addr.slice(0,10)}…</td>
                <td className="p-2 text-right">${cents(r.usd_amount_cents)}</td>
                <td className="p-2 text-right">{r.cash_amount.toLocaleString()}</td>
                <td className="p-2">
                  <span className={`px-2 py-0.5 rounded text-xs ${
                    r.status==="minted"?"bg-green-100 text-green-700":
                    r.status==="failed"?"bg-red-100 text-red-700":
                    r.status==="paid"?"bg-amber-100 text-amber-700":"bg-gray-100 text-gray-700"
                  }`}>{r.status}</span>
                </td>
                <td className="p-2">{r.stripe_session_id ? `sess:${r.stripe_session_id.slice(0,6)}…` : "-"}</td>
                <td className="p-2">{ts(r.updated_at)}</td>
                <td className="p-2">
                  {r.status!=="minted" && (
                    <button onClick={()=>onReplay(r.id)} className="px-2 py-1 border rounded hover:bg-black/5">Replay Mint</button>
                  )}
                </td>
              </tr>
            ))}
            {rows.length===0 && <tr><td className="p-4" colSpan={8}>No orders yet.</td></tr>}
          </tbody>
        </table>
      </div>
      <div className="flex items-center gap-2">
        {nextCursor && (
          <button
            className="px-3 py-1 rounded border hover:bg-black/5"
            onClick={() => { setCursor(nextCursor); }}
          >
            Load more
          </button>
        )}
        {cursor && (
          <button className="px-3 py-1 rounded border hover:bg-black/5" onClick={()=>{ setCursor(undefined); setNextCursor(undefined); }}>Back to newest</button>
        )}
      </div>
    </div>
  );
}
