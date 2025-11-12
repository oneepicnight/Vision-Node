import Pane from '../../../components/Pane'
import { useExchange } from '../store'

export default function MarketHeaderHero(){
  const { chain, lastPrice, change24h, volume24h } = useExchange()

  return (
    <div className="w-full">
      <div className="rounded-2xl p-6 bg-gradient-to-r from-black/30 to-black/20 border border-white/5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="text-xs text-[var(--muted)]">Market</div>
            <div className="text-2xl font-bold">{chain} / CASH</div>
            <div className="text-sm text-[var(--muted)]">Decentralized market</div>
          </div>
          <div className="flex gap-4">
            <Pane className="px-4 py-2" title="Last">
              <div className="text-lg font-semibold">{lastPrice ?? '—'}</div>
              <div className="text-xs text-[var(--muted)]">Last</div>
            </Pane>
            <Pane className="px-4 py-2" title="24h Vol">
              <div className="text-lg font-semibold">{volume24h ?? '—'}</div>
              <div className="text-xs text-[var(--muted)]">quote</div>
            </Pane>
            <Pane className="px-4 py-2" title="Change">
              <div className={`text-lg font-semibold ${change24h && change24h>0 ? 'text-emerald-400' : 'text-red-400'}`}>{change24h ? (change24h>0?`+${change24h}%`:`${change24h}%`) : '—'}</div>
              <div className="text-xs text-[var(--muted)]">24h</div>
            </Pane>
          </div>
        </div>
      </div>
    </div>
  )
}
