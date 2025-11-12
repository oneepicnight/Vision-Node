import { useEffect, useState } from "react";
import { useExchange } from "./store";
import MarketHeaderHero from "./components/MarketHeaderHero";
import BalancesBar from "./components/BalancesBar";
import OrderBookHeat from "./components/OrderBookHeat";
import DepthChartPro from "./components/DepthChartPro";
import TradesTapePro from "./components/TradesTapePro";
import OrderTicketPro from "./components/OrderTicketPro";

export default function ExchangePage({ onPickPrice }:{ onPickPrice?: (p:number)=>void }){
  const { boot } = useExchange();
  const [prefill,setPrefill] = useState<number|undefined>(undefined);

  useEffect(()=>{ boot(); }, [boot]);

  return (
    <div className="flex flex-col gap-4 p-4 text-[var(--text)] bg-[var(--bg)] min-h-screen">
  <MarketHeaderHero />
      <BalancesBar />
      <div className="grid grid-cols-12 gap-4">
  <div className="col-span-3"><OrderBookHeat onPickPrice={(p)=>{ setPrefill(p); if(onPickPrice) onPickPrice(p); }} /></div>
        <div className="col-span-6"><DepthChartPro /></div>
        <div className="col-span-3"><OrderTicketPro prefill={prefill} /></div>
        <div className="col-span-8"><TradesTapePro /></div>
        <div className="col-span-4">{/* MyOrders stays as you had; progress bars already added earlier */}</div>
      </div>
    </div>
  );
}
