import { useState } from 'react'
import { useExchange } from '../store'

export default function OrderBook({ onPickPrice }:{ onPickPrice:(p:number)=>void }) {
  const { asks, bids, setPrefillPrice } = useExchange();
  const [filterMin, setFilterMin] = useState<number|undefined>();
  const [filterMax, setFilterMax] = useState<number|undefined>();
  const filteredAsks = asks.filter(l => (filterMin==null || l.price>=filterMin) && (filterMax==null || l.price<=filterMax));
  const filteredBids = bids.filter(l => (filterMin==null || l.price>=filterMin) && (filterMax==null || l.price<=filterMax));
  return (
    <div className="grid grid-rows-2 gap-2 h-full">
      <div>
        <div className="flex items-center gap-2 mb-2">
          <input className="border rounded px-2 py-1 w-24" placeholder="Min" type="number" value={filterMin??""} onChange={e=>setFilterMin(e.target.value?+e.target.value:undefined)} />
          <input className="border rounded px-2 py-1 w-24" placeholder="Max" type="number" value={filterMax??""} onChange={e=>setFilterMax(e.target.value?+e.target.value:undefined)} />
        </div>
        <div className="overflow-auto border rounded p-2">
          <div className="text-sm font-semibold mb-1">Asks</div>
          <table className="w-full text-sm">
            <thead><tr><th className="text-left">Price</th><th className="text-right">Size</th></tr></thead>
            <tbody>
              {filteredAsks.slice(0,20).map((l)=>(
                <tr key={`a-${l.price}`} className="cursor-pointer ask" onClick={()=>{ setPrefillPrice(l.price); onPickPrice(l.price) }}>
                  <td>{l.price.toFixed(2)}</td><td className="text-right">{l.size.toFixed(3)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
      <div className="overflow-auto border rounded p-2">
        <div className="text-sm font-semibold mb-1">Bids</div>
        <table className="w-full text-sm">
          <thead><tr><th className="text-left">Price</th><th className="text-right">Size</th></tr></thead>
          <tbody>
            {filteredBids.slice(0,20).map((l)=>(
              <tr key={`b-${l.price}`} className="cursor-pointer bid" onClick={()=>{ setPrefillPrice(l.price); onPickPrice(l.price) }}>
                <td>{l.price.toFixed(2)}</td><td className="text-right">{l.size.toFixed(3)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
