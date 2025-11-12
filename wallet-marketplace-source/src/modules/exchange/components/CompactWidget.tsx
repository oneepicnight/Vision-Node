import { useState } from 'react'
import { useExchange } from "../store";

export default function CompactWidget({ onPickPrice }:{ onPickPrice?:(p:number)=>void }){
  const { asks, bids, lastPrice, placeSell, placeBuy } = useExchange();

  const topAsks = asks.slice(0,6);
  const topBids = bids.slice(0,6);

  const [selected, setSelected] = useState<number | null>(null)
  const [tab, setTab] = useState<'sell'|'buy'>('buy')
  const [size, setSize] = useState<number>(0)
  const [spend, setSpend] = useState<number>(0)
  const [busy, setBusy] = useState(false)

  const pick = (p:number)=>{
    setSelected(p); onPickPrice?.(p)
  }

  return (
    <div className="card-exchange compact-widget" style={{ display:'grid', gridTemplateColumns:'1fr 1fr', gap:8 }}>
      <div style={{ gridColumn: '1 / span 2', display:'flex', justifyContent:'space-between', alignItems:'center' }}>
        <div style={{ fontWeight:700 }}>Market</div>
        <div style={{ fontSize:12, color:'var(--muted)' }}>Last: {lastPrice?.toFixed(2) ?? '—'}</div>
      </div>

      <div style={{ maxHeight:140, overflow:'auto' }}>
        <div style={{ fontSize:12, color:'var(--muted)', marginBottom:6 }}>Asks</div>
        <table style={{ width:'100%', fontSize:12 }}>
          <tbody>
            {topAsks.map(a=> (
              <tr key={`ca-${a.price}`} className="ask" style={{ cursor:'pointer' }} onClick={()=>pick(a.price)}>
                <td style={{ color:'var(--red)' }}>{a.price.toFixed(2)}</td>
                <td style={{ textAlign:'right' }}>{a.size.toFixed(3)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div style={{ maxHeight:140, overflow:'auto' }}>
        <div style={{ fontSize:12, color:'var(--muted)', marginBottom:6 }}>Bids</div>
        <table style={{ width:'100%', fontSize:12 }}>
          <tbody>
            {topBids.map(b=> (
              <tr key={`cb-${b.price}`} className="bid" style={{ cursor:'pointer' }} onClick={()=>pick(b.price)}>
                <td style={{ color:'var(--green)' }}>{b.price.toFixed(2)}</td>
                <td style={{ textAlign:'right' }}>{b.size.toFixed(3)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Quick trade panel */}
      <div style={{ gridColumn: '1 / span 2', marginTop:6 }}>
        <div style={{ display:'flex', gap:8, marginBottom:8 }}>
          <button className={`btn ${tab==='sell'?'sell':''}`} onClick={()=>setTab('sell')}>Sell</button>
          <button className={`btn ${tab==='buy'?'buy':''}`} onClick={()=>setTab('buy')}>Buy</button>
          <div style={{ marginLeft:'auto', fontSize:12, color:'var(--muted)' }}>{selected ? `Price ${selected.toFixed(2)}` : 'Select price'}</div>
        </div>

        <div style={{ display:'flex', gap:8, alignItems:'center' }}>
          {tab==='sell' ? (
            <>
              <input type="number" placeholder="amount" value={size||''} onChange={e=>setSize(+e.target.value)} />
              <button className="btn sell" disabled={busy || !selected || size<=0} onClick={async ()=>{
                if(!selected) return; setBusy(true);
                try{ await placeSell({ price: selected, size, post_only:false, tif: 'GTC' }); window.pushToast?.('Sell placed','success'); setSize(0) }catch(e:any){ window.pushToast?.(String(e?.message || 'failed'),'error') } finally{ setBusy(false) }
              }}>Place Sell</button>
            </>
          ) : (
            <>
              <input type="number" placeholder="spend" value={spend||''} onChange={e=>setSpend(+e.target.value)} />
              <button className="btn buy" disabled={busy || !selected || spend<=0} onClick={async ()=>{
                if(!selected) return; setBusy(true);
                try{ await placeBuy({ spend, size: undefined }); window.pushToast?.('Buy executed','success'); setSpend(0) }catch(e:any){ window.pushToast?.(String(e?.message || 'failed'),'error') } finally{ setBusy(false) }
              }}>Buy (Market)</button>
            </>
          )}
        </div>
      </div>

    </div>
  )
}
