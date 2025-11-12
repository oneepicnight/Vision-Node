import { create } from "zustand";
import { openStream, getBook, getTicker, getMyOrders, postOrder, postBuy, cancelOrderReq } from "./api.client";

export type Chain = "BTC" | "BCH" | "DOGE" | "LAND";
export type Quote = "CASH" | "BTC" | "GAME";

export type BookLevel = { price: number; size: number };
export type Trade = { id: string; ts: number; price: number; size: number; side: "buy" | "sell"; chain: Chain };
export type Order = {
  id: string;
  chain: Chain;
  side: "sell";
  price: number;
  sizeTotal: number;
  sizeFilled: number;
  status: "open" | "partial" | "filled" | "cancelled";
  tif?: "GTC" | "IOC" | "GTT";
  post_only?: boolean;
};

export type Balances = Record<Chain, { available: number; locked: number }>;

type State = {
  owner: string;
  chain: Chain;
  asks: BookLevel[];
  bids: BookLevel[];
  trades: Trade[];
  myOrders: Order[];
  balances: Balances;
  lastPrice?: number;
  change24h?: number;
  volume24h?: number;
  ws?: WebSocket | null;
  quote: Quote;
  setQuote: (q: Quote) => void;
  setChain: (c: Chain) => void;
  boot: () => Promise<void>;
  placeSell: (o: { price: number; size: number; post_only: boolean; tif: "GTC" | "IOC" | "GTT" }) => Promise<void>;
  placeBuy: (o: { spend?: number; size?: number }) => Promise<void>;
  cancelOrder: (id: string) => Promise<void>;
  prefillPrice?: number;
  setPrefillPrice: (p?: number) => void;
};

export const useExchange = create<State>((set, get) => ({
  owner: "demo-user-1",
  chain: "LAND" as Chain, // Will be loaded from config at runtime
  quote: "CASH",
  asks: [],
  bids: [],
  trades: [],
  myOrders: [],
  balances: {
    BTC: { available: 100, locked: 0 },
    BCH: { available: 100, locked: 0 },
    DOGE: { available: 10000, locked: 0 },
    LAND: { available: 1, locked: 0 },
  },
  lastPrice: undefined,
  change24h: undefined,
  volume24h: undefined,
  ws: null,

  setChain: (c) => {
    set({ chain: c });
    // re-run boot to fetch book/ticker for new chain
    get().boot().catch((e) => console.warn("boot failed on setChain", e));
  },

  setQuote: (q) => {
    set({ quote: q });
  },

  prefillPrice: undefined,
  setPrefillPrice: (p) => set({ prefillPrice: p }),

  boot: async () => {
    const { chain, owner } = get();
    try {
      const [book, tick, mine] = await Promise.all([
        getBook(chain as any, 200),
        getTicker(chain as any),
        getMyOrders(owner),
      ]);
      set({
        asks: book?.asks || [],
        bids: book?.bids || [],
        lastPrice: tick?.last,
        change24h: tick?.change24h,
        volume24h: tick?.vol24h,
        myOrders: (mine || []).map((o: any) => ({
          id: o.id,
          chain: o.chain,
          side: "sell",
          price: o.price,
          sizeTotal: o.size_total,
          sizeFilled: o.size_filled,
          status: o.status,
          tif: o.tif,
          post_only: o.post_only,
        })),
      });
    } catch (e) {
      console.warn("exchange boot failed (REST)", e);
    }

    // open websocket stream (graceful)
    try {
      const nws = await openStream((ev) => {
        if (!ev) return;
        const type = (ev.type || "").toString().toLowerCase();
        if (type === "book") {
          const p = ev.payload || ev;
          set({ asks: p.asks || ev.asks || [], bids: p.bids || ev.bids || [] });
        } else if (type === "ticker") {
          const t = ev.payload || ev;
          set({ lastPrice: t.last, change24h: t.change24h, volume24h: t.vol24h });
        } else if (type === "trade") {
          const t = ev.payload || ev;
          set((s) => ({ trades: [t, ...s.trades].slice(0, 200) }));
        } else if (type === "order") {
          const o = ev.payload || ev;
          set((s) => {
            const mine = s.owner === o.owner;
            if (!mine) return {} as any;
            const existing = s.myOrders.find((x) => x.id === o.id);
            const normalized: Order = {
              id: o.id,
              chain: o.chain,
              side: "sell",
              price: o.price,
              sizeTotal: o.size_total,
              sizeFilled: o.size_filled,
              status: o.status,
              tif: o.tif,
              post_only: o.post_only,
            };
            return existing ? { myOrders: s.myOrders.map((x) => (x.id === o.id ? normalized : x)) } : { myOrders: [normalized, ...s.myOrders] };
          });
        } else if (type === "balance") {
          const b = ev.payload || ev;
          set((s) => ({ balances: { ...s.balances, [b.chain]: { available: b.available, locked: b.locked } } }));
        }
      });
      set({ ws: nws });
    } catch (e) {
      console.warn("ws failed", e);
    }
  },

  placeSell: async ({ price, size, post_only, tif }) => {
    const { owner, chain } = get();
    try {
      await postOrder({ owner, chain, price, size, post_only, tif });
    } catch (e) {
      console.error("postOrder failed", e);
      throw e;
    }
  },

  placeBuy: async ({ spend, size }) => {
    const { owner, chain } = get();
    try {
      await postBuy({ owner, chain, spend, size });
    } catch (e) {
      console.error("postBuy failed", e);
      throw e;
    }
  },

  cancelOrder: async (id) => {
    const { owner } = get();
    try {
      await cancelOrderReq(id, owner);
    } catch (e) {
      console.error("cancel failed", e);
      throw e;
    }
  },
}));

