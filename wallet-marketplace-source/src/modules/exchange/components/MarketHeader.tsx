import { useExchange } from '../store'

export default function MarketHeader(){
  const { chain, lastPrice, change24h, volume24h, setChain, quote, setQuote } = useExchange();

  return (
    <div className="exchange-header">
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <select value={chain} onChange={(e)=>setChain(e.target.value as any)} style={{ padding: '6px 10px', borderRadius:8, background:'transparent', border:'1px solid rgba(255,255,255,0.04)' }}>
          <option>BTC</option><option>BCH</option><option>DOGE</option><option>LAND</option>
        </select>
        <div style={{ fontSize:18, fontWeight:700 }}>{chain}/{quote}</div>
      </div>

      <div style={{ display:'flex', gap:20, alignItems:'center' }}>
        <div style={{ fontSize:13, color:'var(--muted)' }}>Last <div style={{ fontWeight:700 }}>{lastPrice?.toFixed(2) ?? '—'}</div></div>
        <div style={{ fontSize:13, color:'var(--muted)' }}>24h <div style={{ fontWeight:700, color: (change24h??0)>=0 ? 'var(--green)' : 'var(--red)' }}>{change24h ?? '—'}%</div></div>
        <div style={{ fontSize:13, color:'var(--muted)' }}>Vol 24h <div style={{ fontWeight:700 }}>{volume24h ?? '—'}</div></div>
        {/* Move quote selector to the right side of the header as a compact pill with tooltip */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, position: 'relative' }}>
          <select
            aria-label="Quote asset"
            value={quote}
            onChange={(e)=>setQuote(e.target.value as any)}
            style={{ padding: '6px 10px', borderRadius:9999, background:'rgba(255,255,255,0.03)', border:'1px solid rgba(255,255,255,0.06)', fontSize:12, minWidth:82 }}
          >
            <option>CASH</option>
            <option>BTC</option>
            <option>GAME</option>
          </select>

          {/* info icon with hover popover */}
          <div style={{ position:'relative', display:'inline-flex', alignItems:'center' }}>
            <button aria-label="Quote info" title="Quote selection changes Home prefill behavior" style={{ background:'transparent', border:'none', padding:6, borderRadius:6, cursor:'pointer' }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="10" stroke="rgba(255,255,255,0.6)" strokeWidth="1.2" fill="rgba(0,0,0,0.0)" />
                <path d="M11 10h2v6h-2z" fill="rgba(255,255,255,0.85)" />
                <circle cx="12" cy="7" r="1" fill="rgba(255,255,255,0.85)" />
              </svg>
            </button>
            <div className="quote-popover" style={{ position:'absolute', right:0, top:'26px', minWidth:220, padding:10, background:'rgba(0,0,0,0.8)', color:'white', borderRadius:8, boxShadow:'0 8px 24px rgba(0,0,0,0.6)', opacity:0, transform:'translateY(-6px)', pointerEvents:'none', transition:'opacity 160ms ease, transform 160ms ease' }}>
              <div style={{ fontWeight:700, marginBottom:6 }}>Quote selection</div>
              <div style={{ fontSize:12, lineHeight:1.3 }}>Selecting the quote changes which token the Home send form will prefill when you click a market price. For example, selecting CASH will prefill the send form to spend CASH to buy the base asset.</div>
            </div>
          </div>
          <style>{`
            .quote-popover { opacity: 0; transform: translateY(-6px); }
            div[style] > div:hover + .quote-popover, div[style] > div:focus + .quote-popover, button[aria-label]:hover + .quote-popover, button[aria-label]:focus + .quote-popover { opacity: 1; transform: translateY(0); pointer-events: auto; }
          `}</style>
        </div>
      </div>
    </div>
  )
}
