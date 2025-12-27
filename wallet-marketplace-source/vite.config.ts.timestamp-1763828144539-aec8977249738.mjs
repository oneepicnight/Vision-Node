// vite.config.ts
import { defineConfig } from "file:///C:/vision-node/wallet-marketplace-source/node_modules/vite/dist/node/index.js";
import react from "file:///C:/vision-node/wallet-marketplace-source/node_modules/@vitejs/plugin-react/dist/index.mjs";
import path from "path";

// src/dev/mockWallet.ts
var state = {
  receipts: [],
  balances: {},
  orderbooks: {},
  orders: [],
  trades: []
};
function sendJson(res, obj, code = 200) {
  res.statusCode = code;
  res.setHeader("Content-Type", "application/json");
  res.end(JSON.stringify(obj));
}
function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(Buffer.from(c)));
    req.on("end", () => {
      try {
        const s = Buffer.concat(chunks).toString() || "";
        if (!s)
          return resolve(null);
        resolve(JSON.parse(s));
      } catch (e) {
        reject(e);
      }
    });
    req.on("error", reject);
  });
}
function createMockWalletMiddleware() {
  return async function mockWallet(req, res, next) {
    try {
      const rawUrl = req.url || "/";
      const parsed = new URL(rawUrl, "http://localhost");
      const url = parsed.pathname;
      const method = (req.method || "GET").toUpperCase();
      if (method === "GET" && (url === "/status" || url === "/status/")) {
        return sendJson(res, { node: "mock", version: "dev", time: Date.now() });
      }
      if (method === "GET" && (url === "/keys" || url === "/keys/")) {
        return sendJson(res, { keys: [] });
      }
      if (method === "GET" && (url === "/vault" || url === "/vault/")) {
        return sendJson(res, { receipts: state.receipts.slice(0, 50), mocked: true });
      }
      if (method === "GET" && (url === "/supply" || url === "/supply/")) {
        return sendJson(res, { total: 1e6 });
      }
      if (method === "GET" && url.startsWith("/receipts/latest")) {
        return sendJson(res, state.receipts.slice(0, 50));
      }
      if (method === "GET" && url.startsWith("/balance/")) {
        const parts = url.split("/");
        const addr = parts[2] || "unknown";
        const bal = state.balances[addr] || { LAND: 1, GAME: 250, CASH: 500 };
        return sendJson(res, bal);
      }
      let basePath = url;
      if (basePath.startsWith("/api/market"))
        basePath = basePath.replace(/^\/api\/market/, "");
      if (basePath.startsWith("/market"))
        basePath = basePath.replace(/^\/market/, "");
      if (basePath.startsWith("/exchange") || basePath.startsWith("/market")) {
        if (method === "GET" && (parsed.pathname === "/exchange/book" || parsed.pathname === "/market/exchange/book" || basePath === "/exchange/book")) {
          const chain = parsed.searchParams.get("chain") || "BTC";
          const depth = Number(parsed.searchParams.get("depth") || 50);
          if (!state.orderbooks[chain]) {
            const mid = 5e4;
            const bids = [];
            const asks = [];
            for (let i = 0; i < Math.min(depth, 100); i++) {
              bids.push([mid - i * 10, Math.max(1, Math.round(50 - i))]);
              asks.push([mid + i * 10, Math.max(1, Math.round(10 + i))]);
            }
            state.orderbooks[chain] = { bids, asks };
          }
          return sendJson(res, state.orderbooks[chain]);
        }
        if (method === "GET" && (parsed.pathname === "/exchange/ticker" || basePath === "/exchange/ticker")) {
          const chain = parsed.searchParams.get("chain") || "BTC";
          const ob = state.orderbooks[chain];
          const last = ob ? (ob.asks[0][0] + ob.bids[0][0]) / 2 : 5e4;
          return sendJson(res, { chain, last, change24h: 1.2, volume24h: 12345 });
        }
        if (method === "GET" && (parsed.pathname === "/exchange/trades" || basePath === "/exchange/trades")) {
          const limit = Number(parsed.searchParams.get("limit") || 50);
          return sendJson(res, state.trades.slice(0, limit));
        }
        if (method === "GET" && (parsed.pathname === "/exchange/my/orders" || basePath === "/exchange/my/orders")) {
          const owner = parsed.searchParams.get("owner") || "";
          const list = state.orders.filter((o) => o.owner === owner);
          return sendJson(res, list);
        }
        if (method === "POST" && (parsed.pathname === "/exchange/order" || basePath === "/exchange/order")) {
          const body = await readBody(req);
          const order = body || {};
          const id = `ord-${Date.now()}-${Math.floor(Math.random() * 1e4)}`;
          const o = { id, ...order, status: "open", createdAt: Date.now() };
          state.orders.push(o);
          return sendJson(res, { ok: true, order: o });
        }
        if (method === "POST" && (parsed.pathname === "/exchange/buy" || basePath === "/exchange/buy")) {
          const body = await readBody(req);
          const { chain = "BTC", size = 1 } = body || {};
          const ob = state.orderbooks[chain] || { asks: [[5e4, 100]], bids: [[49900, 50]] };
          let remaining = Number(size);
          const trades = [];
          while (remaining > 0 && ob.asks.length > 0) {
            const [price, qty] = ob.asks.shift();
            const take = Math.min(remaining, qty);
            trades.push({ price, size: take, time: Date.now(), side: "buy" });
            remaining -= take;
            if (qty > take) {
              ob.asks.unshift([price, qty - take]);
              break;
            }
          }
          for (const t of trades)
            state.trades.unshift({ ...t, chain });
          return sendJson(res, { ok: true, trades });
        }
        if (method === "POST" && (parsed.pathname.startsWith("/exchange/order/") && parsed.pathname.endsWith("/cancel") || basePath.startsWith("/exchange/order/") && basePath.endsWith("/cancel"))) {
          const parts = parsed.pathname.split("/");
          const id = parts[3];
          const order = state.orders.find((o) => o.id === id);
          if (order) {
            order.status = "cancelled";
            return sendJson(res, { ok: true });
          }
          return sendJson(res, { ok: false, error: "not_found" }, 404);
        }
      }
      if (method === "POST" && (url === "/tx/submit" || url === "/tx/submit/")) {
        const body = await readBody(req);
        const payload = body || {};
        const tx = payload.tx || payload || {};
        const from = tx.from || "unknown";
        const to = tx.to || "unknown";
        const token = tx.token || "CASH";
        const amount = Number(tx.amount || 0);
        state.balances[from] = state.balances[from] || { LAND: 1, GAME: 250, CASH: 500 };
        state.balances[to] = state.balances[to] || { LAND: 0, GAME: 0, CASH: 0 };
        if (typeof state.balances[from][token] === "number") {
          state.balances[from][token] = Math.max(0, state.balances[from][token] - amount);
        }
        if (typeof state.balances[to][token] === "number") {
          state.balances[to][token] = (state.balances[to][token] || 0) + amount;
        }
        const txid = `mock-${Date.now()}-${Math.floor(Math.random() * 1e4)}`;
        const receipt = { txid, from, to, token, amount, time: Date.now(), status: "confirmed" };
        state.receipts.unshift(receipt);
        return sendJson(res, { ok: true, txid });
      }
      return next();
    } catch (e) {
      return next();
    }
  };
}

// vite.config.ts
var __vite_injected_original_dirname = "C:\\vision-node\\wallet-marketplace-source";
var patchAxiosOutput = () => ({
  name: "patch-axios-output",
  generateBundle(_options, bundle) {
    for (const fileName in bundle) {
      const chunk = bundle[fileName];
      if (chunk.type === "chunk" && chunk.code) {
        const original = chunk.code;
        chunk.code = chunk.code.replace(
          /\((\w+)\.global\),\{ReadableStream/g,
          "($1.global||globalThis),{ReadableStream"
        );
        chunk.code = chunk.code.replace(
          /=\(\(\{Request:(\w+),Response:(\w+)\}\)=>\(\{Request:\1,Response:\2\}\)\)\(/g,
          "=(({Request:$1,Response:$2})=>({Request:$1||class{},Response:$2||class{}}))("
        );
        chunk.code = chunk.code.replace(
          /,\{ReadableStream:(\w+),TextEncoder:(\w+)\}=(\w+)\.global,/g,
          ',{ReadableStream:$1=$1||class{},TextEncoder:$2=$2||(typeof TextEncoder!=="undefined"?TextEncoder:class{})}=($3.global||globalThis||{}),'
        );
        if (chunk.code !== original) {
          console.log(`[patch-axios-output] Patched ${fileName}`);
        }
      }
    }
  }
});
var vite_config_default = defineConfig(({ command, mode }) => {
  const enableMock = process.env.VITE_ENABLE_MOCK === "1";
  console.log("[vite.config] Mock middleware:", enableMock ? "ENABLED" : "DISABLED");
  return {
    plugins: [
      react(),
      patchAxiosOutput()
    ],
    base: "/app/",
    // Wallet is served at /app by the node
    resolve: {
      alias: {
        "@": path.resolve(__vite_injected_original_dirname, "./src"),
        "buffer": "buffer/"
      }
    },
    define: {
      global: "globalThis",
      "process.env": {},
      "process.env.NODE_ENV": JSON.stringify(mode),
      // Ensure these exist for axios adapter detection
      "globalThis.Request": "globalThis.Request || class Request {}",
      "globalThis.Response": "globalThis.Response || class Response {}",
      "globalThis.Headers": "globalThis.Headers || class Headers {}"
    },
    build: {
      commonjsOptions: {
        transformMixedEsModules: true
      }
    },
    optimizeDeps: {
      include: ["buffer"]
    },
    server: {
      host: "127.0.0.1",
      port: 4173,
      proxy: enableMock ? void 0 : {
        // All /api/* requests go to Vision Node at port 7070
        "/api": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        // WebSocket endpoint for exchange stream
        "/api/exchange/stream": {
          target: "ws://127.0.0.1:7070",
          ws: true,
          changeOrigin: true
        },
        // Legacy endpoints (without /api prefix) also go to Vision Node
        "/status": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        "/vault": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        "/keys": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        "/supply": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        "/receipts": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        "/wallet": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        "/tx": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        },
        "/balance": {
          target: "http://127.0.0.1:7070",
          changeOrigin: true
        }
      }
    },
    configureServer(server) {
      if (enableMock) {
        server.middlewares.use(createMockWalletMiddleware());
        console.log("[vite] mock wallet middleware enabled");
      }
    }
  };
});
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiLCAic3JjL2Rldi9tb2NrV2FsbGV0LnRzIl0sCiAgInNvdXJjZXNDb250ZW50IjogWyJjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZGlybmFtZSA9IFwiQzpcXFxcdmlzaW9uLW5vZGVcXFxcd2FsbGV0LW1hcmtldHBsYWNlLXNvdXJjZVwiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9maWxlbmFtZSA9IFwiQzpcXFxcdmlzaW9uLW5vZGVcXFxcd2FsbGV0LW1hcmtldHBsYWNlLXNvdXJjZVxcXFx2aXRlLmNvbmZpZy50c1wiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9pbXBvcnRfbWV0YV91cmwgPSBcImZpbGU6Ly8vQzovdmlzaW9uLW5vZGUvd2FsbGV0LW1hcmtldHBsYWNlLXNvdXJjZS92aXRlLmNvbmZpZy50c1wiO2ltcG9ydCB7IGRlZmluZUNvbmZpZyB9IGZyb20gJ3ZpdGUnXHJcbmltcG9ydCByZWFjdCBmcm9tICdAdml0ZWpzL3BsdWdpbi1yZWFjdCdcclxuaW1wb3J0IHBhdGggZnJvbSAncGF0aCdcclxuaW1wb3J0IHsgY3JlYXRlTW9ja1dhbGxldE1pZGRsZXdhcmUgfSBmcm9tICcuL3NyYy9kZXYvbW9ja1dhbGxldCdcclxuXHJcbi8vIFBsdWdpbiB0byBwYXRjaCB0aGUgYnVuZGxlZCBvdXRwdXQgYWZ0ZXIgbWluaWZpY2F0aW9uXHJcbmNvbnN0IHBhdGNoQXhpb3NPdXRwdXQgPSAoKSA9PiAoe1xyXG4gIG5hbWU6ICdwYXRjaC1heGlvcy1vdXRwdXQnLFxyXG4gIGdlbmVyYXRlQnVuZGxlKF9vcHRpb25zOiBhbnksIGJ1bmRsZTogYW55KSB7XHJcbiAgICBmb3IgKGNvbnN0IGZpbGVOYW1lIGluIGJ1bmRsZSkge1xyXG4gICAgICBjb25zdCBjaHVuayA9IGJ1bmRsZVtmaWxlTmFtZV07XHJcbiAgICAgIGlmIChjaHVuay50eXBlID09PSAnY2h1bmsnICYmIGNodW5rLmNvZGUpIHtcclxuICAgICAgICAvLyBQYXRjaDogU3o9KCh7UmVxdWVzdDplLFJlc3BvbnNlOnR9KT0+Li4uKSkoRi5nbG9iYWwpXHJcbiAgICAgICAgLy8gVGhlIHJlc3VsdCBpcyB1c2VkIGRpcmVjdGx5LCBzbyB3ZSBuZWVkIHRvIHVzZSB8fCBvcGVyYXRvciBmb3IgZmFsbGJhY2tcclxuICAgICAgICBjb25zdCBvcmlnaW5hbCA9IGNodW5rLmNvZGU7XHJcbiAgICAgICAgY2h1bmsuY29kZSA9IGNodW5rLmNvZGUucmVwbGFjZShcclxuICAgICAgICAgIC9cXCgoXFx3KylcXC5nbG9iYWxcXCksXFx7UmVhZGFibGVTdHJlYW0vZyxcclxuICAgICAgICAgICcoJDEuZ2xvYmFsfHxnbG9iYWxUaGlzKSx7UmVhZGFibGVTdHJlYW0nXHJcbiAgICAgICAgKTtcclxuICAgICAgICAvLyBBbHNvIGFkZCBmYWxsYmFjayBlbXB0eSBjbGFzc2VzIGlmIGdsb2JhbCBpcyB1bmRlZmluZWRcclxuICAgICAgICBjaHVuay5jb2RlID0gY2h1bmsuY29kZS5yZXBsYWNlKFxyXG4gICAgICAgICAgLz1cXChcXChcXHtSZXF1ZXN0OihcXHcrKSxSZXNwb25zZTooXFx3KylcXH1cXCk9PlxcKFxce1JlcXVlc3Q6XFwxLFJlc3BvbnNlOlxcMlxcfVxcKVxcKVxcKC9nLFxyXG4gICAgICAgICAgJz0oKHtSZXF1ZXN0OiQxLFJlc3BvbnNlOiQyfSk9Pih7UmVxdWVzdDokMXx8Y2xhc3N7fSxSZXNwb25zZTokMnx8Y2xhc3N7fX0pKSgnXHJcbiAgICAgICAgKTtcclxuICAgICAgICAvLyBGaXggUmVhZGFibGVTdHJlYW0vVGV4dEVuY29kZXIgZGVzdHJ1Y3R1cmluZyBmcm9tIEYuZ2xvYmFsXHJcbiAgICAgICAgY2h1bmsuY29kZSA9IGNodW5rLmNvZGUucmVwbGFjZShcclxuICAgICAgICAgIC8sXFx7UmVhZGFibGVTdHJlYW06KFxcdyspLFRleHRFbmNvZGVyOihcXHcrKVxcfT0oXFx3KylcXC5nbG9iYWwsL2csXHJcbiAgICAgICAgICAnLHtSZWFkYWJsZVN0cmVhbTokMT0kMXx8Y2xhc3N7fSxUZXh0RW5jb2RlcjokMj0kMnx8KHR5cGVvZiBUZXh0RW5jb2RlciE9PVwidW5kZWZpbmVkXCI/VGV4dEVuY29kZXI6Y2xhc3N7fSl9PSgkMy5nbG9iYWx8fGdsb2JhbFRoaXN8fHt9KSwnXHJcbiAgICAgICAgKTtcclxuICAgICAgICBpZiAoY2h1bmsuY29kZSAhPT0gb3JpZ2luYWwpIHtcclxuICAgICAgICAgIGNvbnNvbGUubG9nKGBbcGF0Y2gtYXhpb3Mtb3V0cHV0XSBQYXRjaGVkICR7ZmlsZU5hbWV9YCk7XHJcbiAgICAgICAgfVxyXG4gICAgICB9XHJcbiAgICB9XHJcbiAgfVxyXG59KTtcclxuXHJcbi8vIGh0dHBzOi8vdml0ZWpzLmRldi9jb25maWcvXHJcbmV4cG9ydCBkZWZhdWx0IGRlZmluZUNvbmZpZygoeyBjb21tYW5kLCBtb2RlIH0pID0+IHtcclxuICAvLyBEaXNhYmxlIG1vY2sgbWlkZGxld2FyZSBieSBkZWZhdWx0IHNvIHdhbGxldCB0YWxrcyB0byByZWFsIG5vZGVcclxuICAvLyBTZXQgVklURV9FTkFCTEVfTU9DSz0xIHRvIGVuYWJsZSBtb2NrIG1vZGUgZm9yIHRlc3RpbmdcclxuICBjb25zdCBlbmFibGVNb2NrID0gcHJvY2Vzcy5lbnYuVklURV9FTkFCTEVfTU9DSyA9PT0gJzEnXHJcbiAgXHJcbiAgY29uc29sZS5sb2coJ1t2aXRlLmNvbmZpZ10gTW9jayBtaWRkbGV3YXJlOicsIGVuYWJsZU1vY2sgPyAnRU5BQkxFRCcgOiAnRElTQUJMRUQnKVxyXG5cclxuICByZXR1cm4ge1xyXG4gICAgcGx1Z2luczogW1xyXG4gICAgICByZWFjdCgpLFxyXG4gICAgICBwYXRjaEF4aW9zT3V0cHV0KCksXHJcbiAgICBdLFxyXG4gICAgYmFzZTogJy9hcHAvJywgIC8vIFdhbGxldCBpcyBzZXJ2ZWQgYXQgL2FwcCBieSB0aGUgbm9kZVxyXG4gICAgcmVzb2x2ZToge1xyXG4gICAgICBhbGlhczoge1xyXG4gICAgICAgICdAJzogcGF0aC5yZXNvbHZlKF9fZGlybmFtZSwgJy4vc3JjJyksXHJcbiAgICAgICAgJ2J1ZmZlcic6ICdidWZmZXIvJ1xyXG4gICAgICB9XHJcbiAgICB9LFxyXG4gICAgZGVmaW5lOiB7XHJcbiAgICAgIGdsb2JhbDogJ2dsb2JhbFRoaXMnLFxyXG4gICAgICAncHJvY2Vzcy5lbnYnOiB7fSxcclxuICAgICAgJ3Byb2Nlc3MuZW52Lk5PREVfRU5WJzogSlNPTi5zdHJpbmdpZnkobW9kZSksXHJcbiAgICAgIC8vIEVuc3VyZSB0aGVzZSBleGlzdCBmb3IgYXhpb3MgYWRhcHRlciBkZXRlY3Rpb25cclxuICAgICAgJ2dsb2JhbFRoaXMuUmVxdWVzdCc6ICdnbG9iYWxUaGlzLlJlcXVlc3QgfHwgY2xhc3MgUmVxdWVzdCB7fScsXHJcbiAgICAgICdnbG9iYWxUaGlzLlJlc3BvbnNlJzogJ2dsb2JhbFRoaXMuUmVzcG9uc2UgfHwgY2xhc3MgUmVzcG9uc2Uge30nLFxyXG4gICAgICAnZ2xvYmFsVGhpcy5IZWFkZXJzJzogJ2dsb2JhbFRoaXMuSGVhZGVycyB8fCBjbGFzcyBIZWFkZXJzIHt9J1xyXG4gICAgfSxcclxuICAgIGJ1aWxkOiB7XHJcbiAgICAgIGNvbW1vbmpzT3B0aW9uczoge1xyXG4gICAgICAgIHRyYW5zZm9ybU1peGVkRXNNb2R1bGVzOiB0cnVlXHJcbiAgICAgIH1cclxuICAgIH0sXHJcbiAgICBvcHRpbWl6ZURlcHM6IHtcclxuICAgICAgaW5jbHVkZTogWydidWZmZXInXVxyXG4gICAgfSxcclxuICAgIHNlcnZlcjoge1xyXG4gICAgICBob3N0OiAnMTI3LjAuMC4xJyxcclxuICAgICAgcG9ydDogNDE3MyxcclxuICAgICAgcHJveHk6IGVuYWJsZU1vY2sgPyB1bmRlZmluZWQgOiB7XHJcbiAgICAgICAgLy8gQWxsIC9hcGkvKiByZXF1ZXN0cyBnbyB0byBWaXNpb24gTm9kZSBhdCBwb3J0IDcwNzBcclxuICAgICAgICAnL2FwaSc6IHtcclxuICAgICAgICAgIHRhcmdldDogJ2h0dHA6Ly8xMjcuMC4wLjE6NzA3MCcsXHJcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWVcclxuICAgICAgICB9LFxyXG4gICAgICAgIC8vIFdlYlNvY2tldCBlbmRwb2ludCBmb3IgZXhjaGFuZ2Ugc3RyZWFtXHJcbiAgICAgICAgJy9hcGkvZXhjaGFuZ2Uvc3RyZWFtJzoge1xyXG4gICAgICAgICAgdGFyZ2V0OiAnd3M6Ly8xMjcuMC4wLjE6NzA3MCcsXHJcbiAgICAgICAgICB3czogdHJ1ZSxcclxuICAgICAgICAgIGNoYW5nZU9yaWdpbjogdHJ1ZVxyXG4gICAgICAgIH0sXHJcbiAgICAgICAgLy8gTGVnYWN5IGVuZHBvaW50cyAod2l0aG91dCAvYXBpIHByZWZpeCkgYWxzbyBnbyB0byBWaXNpb24gTm9kZVxyXG4gICAgICAgICcvc3RhdHVzJzoge1xyXG4gICAgICAgICAgdGFyZ2V0OiAnaHR0cDovLzEyNy4wLjAuMTo3MDcwJyxcclxuICAgICAgICAgIGNoYW5nZU9yaWdpbjogdHJ1ZVxyXG4gICAgICAgIH0sXHJcbiAgICAgICAgJy92YXVsdCc6IHtcclxuICAgICAgICAgIHRhcmdldDogJ2h0dHA6Ly8xMjcuMC4wLjE6NzA3MCcsXHJcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWVcclxuICAgICAgICB9LFxyXG4gICAgICAgICcva2V5cyc6IHtcclxuICAgICAgICAgIHRhcmdldDogJ2h0dHA6Ly8xMjcuMC4wLjE6NzA3MCcsXHJcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWVcclxuICAgICAgICB9LFxyXG4gICAgICAgICcvc3VwcGx5Jzoge1xyXG4gICAgICAgICAgdGFyZ2V0OiAnaHR0cDovLzEyNy4wLjAuMTo3MDcwJyxcclxuICAgICAgICAgIGNoYW5nZU9yaWdpbjogdHJ1ZVxyXG4gICAgICAgIH0sXHJcbiAgICAgICAgJy9yZWNlaXB0cyc6IHtcclxuICAgICAgICAgIHRhcmdldDogJ2h0dHA6Ly8xMjcuMC4wLjE6NzA3MCcsXHJcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWVcclxuICAgICAgICB9LFxyXG4gICAgICAgICcvd2FsbGV0Jzoge1xyXG4gICAgICAgICAgdGFyZ2V0OiAnaHR0cDovLzEyNy4wLjAuMTo3MDcwJyxcclxuICAgICAgICAgIGNoYW5nZU9yaWdpbjogdHJ1ZVxyXG4gICAgICAgIH0sXHJcbiAgICAgICAgJy90eCc6IHtcclxuICAgICAgICAgIHRhcmdldDogJ2h0dHA6Ly8xMjcuMC4wLjE6NzA3MCcsXHJcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWVcclxuICAgICAgICB9LFxyXG4gICAgICAgICcvYmFsYW5jZSc6IHtcclxuICAgICAgICAgIHRhcmdldDogJ2h0dHA6Ly8xMjcuMC4wLjE6NzA3MCcsXHJcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWVcclxuICAgICAgICB9XHJcbiAgICAgIH1cclxuICAgIH0sXHJcbiAgICBjb25maWd1cmVTZXJ2ZXIoc2VydmVyKSB7XHJcbiAgICAgIGlmIChlbmFibGVNb2NrKSB7XHJcbiAgICAgICAgLy8gSW5qZWN0IHRoZSBtb2NrIHdhbGxldCBtaWRkbGV3YXJlIHNvIHJlcXVlc3RzIHRvIHRoZSBkZXYgc2VydmVyXHJcbiAgICAgICAgLy8gY2FuIHJlc3BvbmQgdG8gL3N0YXR1cywgL3ZhdWx0LCAva2V5cyBhbmQgcmVsYXRlZCB3YWxsZXQgZW5kcG9pbnRzLlxyXG4gICAgICAgIHNlcnZlci5taWRkbGV3YXJlcy51c2UoY3JlYXRlTW9ja1dhbGxldE1pZGRsZXdhcmUoKSlcclxuICAgICAgICAvLyBlc2xpbnQtZGlzYWJsZS1uZXh0LWxpbmUgbm8tY29uc29sZVxyXG4gICAgICAgIGNvbnNvbGUubG9nKCdbdml0ZV0gbW9jayB3YWxsZXQgbWlkZGxld2FyZSBlbmFibGVkJylcclxuICAgICAgfVxyXG4gICAgfVxyXG4gIH1cclxufSlcclxuIiwgImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCJDOlxcXFx2aXNpb24tbm9kZVxcXFx3YWxsZXQtbWFya2V0cGxhY2Utc291cmNlXFxcXHNyY1xcXFxkZXZcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZmlsZW5hbWUgPSBcIkM6XFxcXHZpc2lvbi1ub2RlXFxcXHdhbGxldC1tYXJrZXRwbGFjZS1zb3VyY2VcXFxcc3JjXFxcXGRldlxcXFxtb2NrV2FsbGV0LnRzXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ltcG9ydF9tZXRhX3VybCA9IFwiZmlsZTovLy9DOi92aXNpb24tbm9kZS93YWxsZXQtbWFya2V0cGxhY2Utc291cmNlL3NyYy9kZXYvbW9ja1dhbGxldC50c1wiO2ltcG9ydCB7IEluY29taW5nTWVzc2FnZSwgU2VydmVyUmVzcG9uc2UgfSBmcm9tICdodHRwJ1xyXG5cclxudHlwZSBCYWxhbmNlcyA9IHsgW3Rva2VuOiBzdHJpbmddOiBudW1iZXIgfVxyXG5cclxuLy8gSW4tbWVtb3J5IG1vY2sgc3RhdGUgZm9yIGRldiBzZXNzaW9uc1xyXG5jb25zdCBzdGF0ZToge1xyXG4gIHJlY2VpcHRzOiBhbnlbXVxyXG4gIGJhbGFuY2VzOiBSZWNvcmQ8c3RyaW5nLCBCYWxhbmNlcz5cclxuICAvLyBleGNoYW5nZVxyXG4gIG9yZGVyYm9va3M6IFJlY29yZDxzdHJpbmcsIHsgYmlkczogbnVtYmVyW11bXTsgYXNrczogbnVtYmVyW11bXSB9PiAvLyBwcmljZSwgc2l6ZVxyXG4gIG9yZGVyczogYW55W11cclxuICB0cmFkZXM6IGFueVtdXHJcbn0gPSB7XHJcbiAgcmVjZWlwdHM6IFtdLFxyXG4gIGJhbGFuY2VzOiB7fSxcclxuICBvcmRlcmJvb2tzOiB7fSxcclxuICBvcmRlcnM6IFtdLFxyXG4gIHRyYWRlczogW11cclxufVxyXG5cclxuZnVuY3Rpb24gc2VuZEpzb24ocmVzOiBTZXJ2ZXJSZXNwb25zZSwgb2JqOiBhbnksIGNvZGUgPSAyMDApIHtcclxuICByZXMuc3RhdHVzQ29kZSA9IGNvZGVcclxuICByZXMuc2V0SGVhZGVyKCdDb250ZW50LVR5cGUnLCAnYXBwbGljYXRpb24vanNvbicpXHJcbiAgcmVzLmVuZChKU09OLnN0cmluZ2lmeShvYmopKVxyXG59XHJcblxyXG5mdW5jdGlvbiByZWFkQm9keShyZXE6IEluY29taW5nTWVzc2FnZSk6IFByb21pc2U8YW55PiB7XHJcbiAgcmV0dXJuIG5ldyBQcm9taXNlKChyZXNvbHZlLCByZWplY3QpID0+IHtcclxuICAgIGNvbnN0IGNodW5rczogQnVmZmVyW10gPSBbXVxyXG4gICAgcmVxLm9uKCdkYXRhJywgKGMpID0+IGNodW5rcy5wdXNoKEJ1ZmZlci5mcm9tKGMpKSlcclxuICAgIHJlcS5vbignZW5kJywgKCkgPT4ge1xyXG4gICAgICB0cnkge1xyXG4gICAgICAgIGNvbnN0IHMgPSBCdWZmZXIuY29uY2F0KGNodW5rcykudG9TdHJpbmcoKSB8fCAnJ1xyXG4gICAgICAgIGlmICghcykgcmV0dXJuIHJlc29sdmUobnVsbClcclxuICAgICAgICByZXNvbHZlKEpTT04ucGFyc2UocykpXHJcbiAgICAgIH0gY2F0Y2ggKGUpIHtcclxuICAgICAgICByZWplY3QoZSlcclxuICAgICAgfVxyXG4gICAgfSlcclxuICAgIHJlcS5vbignZXJyb3InLCByZWplY3QpXHJcbiAgfSlcclxufVxyXG5cclxuZXhwb3J0IGZ1bmN0aW9uIGNyZWF0ZU1vY2tXYWxsZXRNaWRkbGV3YXJlKCkge1xyXG4gIHJldHVybiBhc3luYyBmdW5jdGlvbiBtb2NrV2FsbGV0KHJlcTogSW5jb21pbmdNZXNzYWdlLCByZXM6IFNlcnZlclJlc3BvbnNlLCBuZXh0OiAoLi4uYXJnczogYW55W10pID0+IHZvaWQpIHtcclxuICAgIHRyeSB7XHJcbiAgICAgIGNvbnN0IHJhd1VybCA9IHJlcS51cmwgfHwgJy8nXHJcbiAgICAgIC8vIHBhcnNlIHBhdGggYW5kIHF1ZXJ5IHNhZmVseVxyXG4gICAgICBjb25zdCBwYXJzZWQgPSBuZXcgVVJMKHJhd1VybCwgJ2h0dHA6Ly9sb2NhbGhvc3QnKVxyXG4gICAgICBjb25zdCB1cmwgPSBwYXJzZWQucGF0aG5hbWVcclxuICAgICAgY29uc3QgbWV0aG9kID0gKHJlcS5tZXRob2QgfHwgJ0dFVCcpLnRvVXBwZXJDYXNlKClcclxuXHJcbiAgICAgIC8vIFNpbXBsZSByb3V0aW5nIGZvciB3YWxsZXQgZW5kcG9pbnRzIHVzZWQgYnkgdGhlIGZyb250ZW5kXHJcbiAgICAgIGlmIChtZXRob2QgPT09ICdHRVQnICYmICh1cmwgPT09ICcvc3RhdHVzJyB8fCB1cmwgPT09ICcvc3RhdHVzLycpKSB7XHJcbiAgICAgICAgcmV0dXJuIHNlbmRKc29uKHJlcywgeyBub2RlOiAnbW9jaycsIHZlcnNpb246ICdkZXYnLCB0aW1lOiBEYXRlLm5vdygpIH0pXHJcbiAgICAgIH1cclxuXHJcbiAgICAgIGlmIChtZXRob2QgPT09ICdHRVQnICYmICh1cmwgPT09ICcva2V5cycgfHwgdXJsID09PSAnL2tleXMvJykpIHtcclxuICAgICAgICAvLyBSZXR1cm4gYSBzaW1wbGUga2V5cyByZXNwb25zZSBzbyB0cnlLZXlzVGhlblZhdWx0IHN1Y2NlZWRzIGluIGRldlxyXG4gICAgICAgIHJldHVybiBzZW5kSnNvbihyZXMsIHsga2V5czogW10gfSlcclxuICAgICAgfVxyXG5cclxuICAgICAgaWYgKG1ldGhvZCA9PT0gJ0dFVCcgJiYgKHVybCA9PT0gJy92YXVsdCcgfHwgdXJsID09PSAnL3ZhdWx0LycpKSB7XHJcbiAgICAgICAgcmV0dXJuIHNlbmRKc29uKHJlcywgeyByZWNlaXB0czogc3RhdGUucmVjZWlwdHMuc2xpY2UoMCwgNTApLCBtb2NrZWQ6IHRydWUgfSlcclxuICAgICAgfVxyXG5cclxuICAgICAgaWYgKG1ldGhvZCA9PT0gJ0dFVCcgJiYgKHVybCA9PT0gJy9zdXBwbHknIHx8IHVybCA9PT0gJy9zdXBwbHkvJykpIHtcclxuICAgICAgICByZXR1cm4gc2VuZEpzb24ocmVzLCB7IHRvdGFsOiAxMDAwMDAwIH0pXHJcbiAgICAgIH1cclxuXHJcbiAgICAgIGlmIChtZXRob2QgPT09ICdHRVQnICYmIHVybC5zdGFydHNXaXRoKCcvcmVjZWlwdHMvbGF0ZXN0JykpIHtcclxuICAgICAgICByZXR1cm4gc2VuZEpzb24ocmVzLCBzdGF0ZS5yZWNlaXB0cy5zbGljZSgwLCA1MCkpXHJcbiAgICAgIH1cclxuXHJcbiAgICAgIGlmIChtZXRob2QgPT09ICdHRVQnICYmIHVybC5zdGFydHNXaXRoKCcvYmFsYW5jZS8nKSkge1xyXG4gICAgICAgIGNvbnN0IHBhcnRzID0gdXJsLnNwbGl0KCcvJylcclxuICAgICAgICBjb25zdCBhZGRyID0gcGFydHNbMl0gfHwgJ3Vua25vd24nXHJcbiAgICAgICAgY29uc3QgYmFsID0gc3RhdGUuYmFsYW5jZXNbYWRkcl0gfHwgeyBMQU5EOiAxLCBHQU1FOiAyNTAsIENBU0g6IDUwMCB9XHJcbiAgICAgICAgcmV0dXJuIHNlbmRKc29uKHJlcywgYmFsKVxyXG4gICAgICB9XHJcblxyXG4gIC8vIEV4Y2hhbmdlIGVuZHBvaW50cyAoc2ltcGxlIG1vY2tzKVxyXG4gIC8vIFN1cHBvcnQgYm90aCBkaXJlY3QgYC9leGNoYW5nZS8uLi5gIGFuZCBwcm94aWVkIGAvYXBpL21hcmtldC9leGNoYW5nZS8uLi5gXHJcbiAgbGV0IGJhc2VQYXRoID0gdXJsXHJcbiAgaWYgKGJhc2VQYXRoLnN0YXJ0c1dpdGgoJy9hcGkvbWFya2V0JykpIGJhc2VQYXRoID0gYmFzZVBhdGgucmVwbGFjZSgvXlxcL2FwaVxcL21hcmtldC8sICcnKVxyXG4gIGlmIChiYXNlUGF0aC5zdGFydHNXaXRoKCcvbWFya2V0JykpIGJhc2VQYXRoID0gYmFzZVBhdGgucmVwbGFjZSgvXlxcL21hcmtldC8sICcnKVxyXG5cclxuICBpZiAoYmFzZVBhdGguc3RhcnRzV2l0aCgnL2V4Y2hhbmdlJykgfHwgYmFzZVBhdGguc3RhcnRzV2l0aCgnL21hcmtldCcpKSB7XHJcbiAgICAgICAgLy8gR0VUIC9leGNoYW5nZS9ib29rP2NoYWluPUJUQyZkZXB0aD0yMDBcclxuICAgICAgICBpZiAobWV0aG9kID09PSAnR0VUJyAmJiAocGFyc2VkLnBhdGhuYW1lID09PSAnL2V4Y2hhbmdlL2Jvb2snIHx8IHBhcnNlZC5wYXRobmFtZSA9PT0gJy9tYXJrZXQvZXhjaGFuZ2UvYm9vaycgfHwgYmFzZVBhdGggPT09ICcvZXhjaGFuZ2UvYm9vaycpKSB7XHJcbiAgICAgICAgICBjb25zdCBjaGFpbiA9IHBhcnNlZC5zZWFyY2hQYXJhbXMuZ2V0KCdjaGFpbicpIHx8ICdCVEMnXHJcbiAgICAgICAgICBjb25zdCBkZXB0aCA9IE51bWJlcihwYXJzZWQuc2VhcmNoUGFyYW1zLmdldCgnZGVwdGgnKSB8fCA1MClcclxuICAgICAgICAgIC8vIHNlZWQgb3JkZXJib29rIGlmIG1pc3NpbmdcclxuICAgICAgICAgIGlmICghc3RhdGUub3JkZXJib29rc1tjaGFpbl0pIHtcclxuICAgICAgICAgICAgY29uc3QgbWlkID0gNTAwMDBcclxuICAgICAgICAgICAgY29uc3QgYmlkczogbnVtYmVyW11bXSA9IFtdXHJcbiAgICAgICAgICAgIGNvbnN0IGFza3M6IG51bWJlcltdW10gPSBbXVxyXG4gICAgICAgICAgICBmb3IgKGxldCBpID0gMDsgaSA8IE1hdGgubWluKGRlcHRoLCAxMDApOyBpKyspIHtcclxuICAgICAgICAgICAgICBiaWRzLnB1c2goW21pZCAtIGkgKiAxMCwgTWF0aC5tYXgoMSwgTWF0aC5yb3VuZCg1MCAtIGkpKV0pXHJcbiAgICAgICAgICAgICAgYXNrcy5wdXNoKFttaWQgKyBpICogMTAsIE1hdGgubWF4KDEsIE1hdGgucm91bmQoMTAgKyBpKSldKVxyXG4gICAgICAgICAgICB9XHJcbiAgICAgICAgICAgIHN0YXRlLm9yZGVyYm9va3NbY2hhaW5dID0geyBiaWRzLCBhc2tzIH1cclxuICAgICAgICAgIH1cclxuICAgICAgICAgIHJldHVybiBzZW5kSnNvbihyZXMsIHN0YXRlLm9yZGVyYm9va3NbY2hhaW5dKVxyXG4gICAgICAgIH1cclxuXHJcbiAgICAgICAgLy8gR0VUIC9leGNoYW5nZS90aWNrZXI/Y2hhaW49QlRDXHJcbiAgICAgICAgaWYgKG1ldGhvZCA9PT0gJ0dFVCcgJiYgKHBhcnNlZC5wYXRobmFtZSA9PT0gJy9leGNoYW5nZS90aWNrZXInIHx8IGJhc2VQYXRoID09PSAnL2V4Y2hhbmdlL3RpY2tlcicpKSB7XHJcbiAgICAgICAgICBjb25zdCBjaGFpbiA9IHBhcnNlZC5zZWFyY2hQYXJhbXMuZ2V0KCdjaGFpbicpIHx8ICdCVEMnXHJcbiAgICAgICAgICBjb25zdCBvYiA9IHN0YXRlLm9yZGVyYm9va3NbY2hhaW5dXHJcbiAgICAgICAgICBjb25zdCBsYXN0ID0gb2IgPyAob2IuYXNrc1swXVswXSArIG9iLmJpZHNbMF1bMF0pIC8gMiA6IDUwMDAwXHJcbiAgICAgICAgICByZXR1cm4gc2VuZEpzb24ocmVzLCB7IGNoYWluLCBsYXN0LCBjaGFuZ2UyNGg6IDEuMiwgdm9sdW1lMjRoOiAxMjM0NSB9KVxyXG4gICAgICAgIH1cclxuXHJcbiAgICAgICAgLy8gR0VUIC9leGNoYW5nZS90cmFkZXM/Y2hhaW49QlRDJmxpbWl0PTUwXHJcbiAgICAgICAgaWYgKG1ldGhvZCA9PT0gJ0dFVCcgJiYgKHBhcnNlZC5wYXRobmFtZSA9PT0gJy9leGNoYW5nZS90cmFkZXMnIHx8IGJhc2VQYXRoID09PSAnL2V4Y2hhbmdlL3RyYWRlcycpKSB7XHJcbiAgICAgICAgICBjb25zdCBsaW1pdCA9IE51bWJlcihwYXJzZWQuc2VhcmNoUGFyYW1zLmdldCgnbGltaXQnKSB8fCA1MClcclxuICAgICAgICAgIHJldHVybiBzZW5kSnNvbihyZXMsIHN0YXRlLnRyYWRlcy5zbGljZSgwLCBsaW1pdCkpXHJcbiAgICAgICAgfVxyXG5cclxuICAgICAgICAvLyBHRVQgL2V4Y2hhbmdlL215L29yZGVycz9vd25lcj0uLi5cclxuICAgICAgICBpZiAobWV0aG9kID09PSAnR0VUJyAmJiAocGFyc2VkLnBhdGhuYW1lID09PSAnL2V4Y2hhbmdlL215L29yZGVycycgfHwgYmFzZVBhdGggPT09ICcvZXhjaGFuZ2UvbXkvb3JkZXJzJykpIHtcclxuICAgICAgICAgIGNvbnN0IG93bmVyID0gcGFyc2VkLnNlYXJjaFBhcmFtcy5nZXQoJ293bmVyJykgfHwgJydcclxuICAgICAgICAgIGNvbnN0IGxpc3QgPSBzdGF0ZS5vcmRlcnMuZmlsdGVyKChvKSA9PiBvLm93bmVyID09PSBvd25lcilcclxuICAgICAgICAgIHJldHVybiBzZW5kSnNvbihyZXMsIGxpc3QpXHJcbiAgICAgICAgfVxyXG5cclxuICAgICAgICAvLyBQT1NUIC9leGNoYW5nZS9vcmRlciAtPiBjcmVhdGUgbGltaXQgb3JkZXJcclxuICAgICAgICBpZiAobWV0aG9kID09PSAnUE9TVCcgJiYgKHBhcnNlZC5wYXRobmFtZSA9PT0gJy9leGNoYW5nZS9vcmRlcicgfHwgYmFzZVBhdGggPT09ICcvZXhjaGFuZ2Uvb3JkZXInKSkge1xyXG4gICAgICAgICAgY29uc3QgYm9keSA9IGF3YWl0IHJlYWRCb2R5KHJlcSlcclxuICAgICAgICAgIGNvbnN0IG9yZGVyID0gYm9keSB8fCB7fVxyXG4gICAgICAgICAgY29uc3QgaWQgPSBgb3JkLSR7RGF0ZS5ub3coKX0tJHtNYXRoLmZsb29yKE1hdGgucmFuZG9tKCkgKiAxMDAwMCl9YFxyXG4gICAgICAgICAgY29uc3QgbyA9IHsgaWQsIC4uLm9yZGVyLCBzdGF0dXM6ICdvcGVuJywgY3JlYXRlZEF0OiBEYXRlLm5vdygpIH1cclxuICAgICAgICAgIHN0YXRlLm9yZGVycy5wdXNoKG8pXHJcbiAgICAgICAgICByZXR1cm4gc2VuZEpzb24ocmVzLCB7IG9rOiB0cnVlLCBvcmRlcjogbyB9KVxyXG4gICAgICAgIH1cclxuXHJcbiAgICAgICAgLy8gUE9TVCAvZXhjaGFuZ2UvYnV5IC0+IG1hcmtldCBidXkgc2ltdWxhdGlvblxyXG4gICAgICAgIGlmIChtZXRob2QgPT09ICdQT1NUJyAmJiAocGFyc2VkLnBhdGhuYW1lID09PSAnL2V4Y2hhbmdlL2J1eScgfHwgYmFzZVBhdGggPT09ICcvZXhjaGFuZ2UvYnV5JykpIHtcclxuICAgICAgICAgIGNvbnN0IGJvZHkgPSBhd2FpdCByZWFkQm9keShyZXEpXHJcbiAgICAgICAgICBjb25zdCB7IGNoYWluID0gJ0JUQycsIHNpemUgPSAxIH0gPSBib2R5IHx8IHt9XHJcbiAgICAgICAgICAvLyBjb25zdW1lIGFza3NcclxuICAgICAgICAgIGNvbnN0IG9iID0gc3RhdGUub3JkZXJib29rc1tjaGFpbl0gfHwgeyBhc2tzOiBbWzUwMDAwLCAxMDBdXSwgYmlkczogW1s0OTkwMCwgNTBdXSB9XHJcbiAgICAgICAgICBsZXQgcmVtYWluaW5nID0gTnVtYmVyKHNpemUpXHJcbiAgICAgICAgICBjb25zdCB0cmFkZXM6IGFueVtdID0gW11cclxuICAgICAgICAgIHdoaWxlIChyZW1haW5pbmcgPiAwICYmIG9iLmFza3MubGVuZ3RoID4gMCkge1xyXG4gICAgICAgICAgICBjb25zdCBbcHJpY2UsIHF0eV0gPSBvYi5hc2tzLnNoaWZ0KCkhXHJcbiAgICAgICAgICAgIGNvbnN0IHRha2UgPSBNYXRoLm1pbihyZW1haW5pbmcsIHF0eSlcclxuICAgICAgICAgICAgdHJhZGVzLnB1c2goeyBwcmljZSwgc2l6ZTogdGFrZSwgdGltZTogRGF0ZS5ub3coKSwgc2lkZTogJ2J1eScgfSlcclxuICAgICAgICAgICAgcmVtYWluaW5nIC09IHRha2VcclxuICAgICAgICAgICAgaWYgKHF0eSA+IHRha2UpIHtcclxuICAgICAgICAgICAgICBvYi5hc2tzLnVuc2hpZnQoW3ByaWNlLCBxdHkgLSB0YWtlXSlcclxuICAgICAgICAgICAgICBicmVha1xyXG4gICAgICAgICAgICB9XHJcbiAgICAgICAgICB9XHJcbiAgICAgICAgICAvLyByZWNvcmQgdHJhZGVzXHJcbiAgICAgICAgICBmb3IgKGNvbnN0IHQgb2YgdHJhZGVzKSBzdGF0ZS50cmFkZXMudW5zaGlmdCh7IC4uLnQsIGNoYWluIH0pXHJcbiAgICAgICAgICByZXR1cm4gc2VuZEpzb24ocmVzLCB7IG9rOiB0cnVlLCB0cmFkZXMgfSlcclxuICAgICAgICB9XHJcblxyXG4gICAgICAgIC8vIFBPU1QgL2V4Y2hhbmdlL29yZGVyLzppZC9jYW5jZWxcclxuICAgICAgICBpZiAobWV0aG9kID09PSAnUE9TVCcgJiYgKChwYXJzZWQucGF0aG5hbWUuc3RhcnRzV2l0aCgnL2V4Y2hhbmdlL29yZGVyLycpICYmIHBhcnNlZC5wYXRobmFtZS5lbmRzV2l0aCgnL2NhbmNlbCcpKSB8fCBiYXNlUGF0aC5zdGFydHNXaXRoKCcvZXhjaGFuZ2Uvb3JkZXIvJykgJiYgYmFzZVBhdGguZW5kc1dpdGgoJy9jYW5jZWwnKSkpIHtcclxuICAgICAgICAgIGNvbnN0IHBhcnRzID0gcGFyc2VkLnBhdGhuYW1lLnNwbGl0KCcvJylcclxuICAgICAgICAgIGNvbnN0IGlkID0gcGFydHNbM11cclxuICAgICAgICAgIGNvbnN0IG9yZGVyID0gc3RhdGUub3JkZXJzLmZpbmQoKG8pID0+IG8uaWQgPT09IGlkKVxyXG4gICAgICAgICAgaWYgKG9yZGVyKSB7XHJcbiAgICAgICAgICAgIG9yZGVyLnN0YXR1cyA9ICdjYW5jZWxsZWQnXHJcbiAgICAgICAgICAgIHJldHVybiBzZW5kSnNvbihyZXMsIHsgb2s6IHRydWUgfSlcclxuICAgICAgICAgIH1cclxuICAgICAgICAgIHJldHVybiBzZW5kSnNvbihyZXMsIHsgb2s6IGZhbHNlLCBlcnJvcjogJ25vdF9mb3VuZCcgfSwgNDA0KVxyXG4gICAgICAgIH1cclxuICAgICAgfVxyXG5cclxuICAgICAgaWYgKG1ldGhvZCA9PT0gJ1BPU1QnICYmICh1cmwgPT09ICcvdHgvc3VibWl0JyB8fCB1cmwgPT09ICcvdHgvc3VibWl0LycpKSB7XHJcbiAgICAgICAgY29uc3QgYm9keSA9IGF3YWl0IHJlYWRCb2R5KHJlcSlcclxuICAgICAgICBjb25zdCBwYXlsb2FkID0gYm9keSB8fCB7fVxyXG4gICAgICAgIGNvbnN0IHR4ID0gcGF5bG9hZC50eCB8fCBwYXlsb2FkIHx8IHt9XHJcbiAgICAgICAgY29uc3QgZnJvbSA9IHR4LmZyb20gfHwgJ3Vua25vd24nXHJcbiAgICAgICAgY29uc3QgdG8gPSB0eC50byB8fCAndW5rbm93bidcclxuICAgICAgICBjb25zdCB0b2tlbiA9IHR4LnRva2VuIHx8ICdDQVNIJ1xyXG4gICAgICAgIGNvbnN0IGFtb3VudCA9IE51bWJlcih0eC5hbW91bnQgfHwgMClcclxuXHJcbiAgICAgICAgLy8gVXBkYXRlIGJhbGFuY2VzIChiZXN0LWVmZm9ydClcclxuICAgICAgICBzdGF0ZS5iYWxhbmNlc1tmcm9tXSA9IHN0YXRlLmJhbGFuY2VzW2Zyb21dIHx8IHsgTEFORDogMSwgR0FNRTogMjUwLCBDQVNIOiA1MDAgfVxyXG4gICAgICAgIHN0YXRlLmJhbGFuY2VzW3RvXSA9IHN0YXRlLmJhbGFuY2VzW3RvXSB8fCB7IExBTkQ6IDAsIEdBTUU6IDAsIENBU0g6IDAgfVxyXG4gICAgICAgIGlmICh0eXBlb2Ygc3RhdGUuYmFsYW5jZXNbZnJvbV1bdG9rZW5dID09PSAnbnVtYmVyJykge1xyXG4gICAgICAgICAgc3RhdGUuYmFsYW5jZXNbZnJvbV1bdG9rZW5dID0gTWF0aC5tYXgoMCwgc3RhdGUuYmFsYW5jZXNbZnJvbV1bdG9rZW5dIC0gYW1vdW50KVxyXG4gICAgICAgIH1cclxuICAgICAgICBpZiAodHlwZW9mIHN0YXRlLmJhbGFuY2VzW3RvXVt0b2tlbl0gPT09ICdudW1iZXInKSB7XHJcbiAgICAgICAgICBzdGF0ZS5iYWxhbmNlc1t0b11bdG9rZW5dID0gKHN0YXRlLmJhbGFuY2VzW3RvXVt0b2tlbl0gfHwgMCkgKyBhbW91bnRcclxuICAgICAgICB9XHJcblxyXG4gICAgICAgIGNvbnN0IHR4aWQgPSBgbW9jay0ke0RhdGUubm93KCl9LSR7TWF0aC5mbG9vcihNYXRoLnJhbmRvbSgpICogMTAwMDApfWBcclxuICAgICAgICBjb25zdCByZWNlaXB0ID0geyB0eGlkLCBmcm9tLCB0bywgdG9rZW4sIGFtb3VudCwgdGltZTogRGF0ZS5ub3coKSwgc3RhdHVzOiAnY29uZmlybWVkJyB9XHJcbiAgICAgICAgc3RhdGUucmVjZWlwdHMudW5zaGlmdChyZWNlaXB0KVxyXG5cclxuICAgICAgICByZXR1cm4gc2VuZEpzb24ocmVzLCB7IG9rOiB0cnVlLCB0eGlkIH0pXHJcbiAgICAgIH1cclxuXHJcbiAgICAgIC8vIE5vdCBvbmUgb2Ygb3VyIG1vY2sgcm91dGVzIFx1MjAxNCBjb250aW51ZSB0byBuZXh0IG1pZGRsZXdhcmVcclxuICAgICAgcmV0dXJuIG5leHQoKVxyXG4gICAgfSBjYXRjaCAoZSkge1xyXG4gICAgICAvLyBJZiBhbnl0aGluZyBnb2VzIHdyb25nLCBwYXNzIHRocm91Z2ggc28gdGhlIGRldiBzZXJ2ZXIgY2FuIGhhbmRsZSBpdFxyXG4gICAgICByZXR1cm4gbmV4dCgpXHJcbiAgICB9XHJcbiAgfVxyXG59XHJcblxyXG5leHBvcnQgZGVmYXVsdCBjcmVhdGVNb2NrV2FsbGV0TWlkZGxld2FyZVxyXG4iXSwKICAibWFwcGluZ3MiOiAiO0FBQWdULFNBQVMsb0JBQW9CO0FBQzdVLE9BQU8sV0FBVztBQUNsQixPQUFPLFVBQVU7OztBQ0dqQixJQUFNLFFBT0Y7QUFBQSxFQUNGLFVBQVUsQ0FBQztBQUFBLEVBQ1gsVUFBVSxDQUFDO0FBQUEsRUFDWCxZQUFZLENBQUM7QUFBQSxFQUNiLFFBQVEsQ0FBQztBQUFBLEVBQ1QsUUFBUSxDQUFDO0FBQ1g7QUFFQSxTQUFTLFNBQVMsS0FBcUIsS0FBVSxPQUFPLEtBQUs7QUFDM0QsTUFBSSxhQUFhO0FBQ2pCLE1BQUksVUFBVSxnQkFBZ0Isa0JBQWtCO0FBQ2hELE1BQUksSUFBSSxLQUFLLFVBQVUsR0FBRyxDQUFDO0FBQzdCO0FBRUEsU0FBUyxTQUFTLEtBQW9DO0FBQ3BELFNBQU8sSUFBSSxRQUFRLENBQUMsU0FBUyxXQUFXO0FBQ3RDLFVBQU0sU0FBbUIsQ0FBQztBQUMxQixRQUFJLEdBQUcsUUFBUSxDQUFDLE1BQU0sT0FBTyxLQUFLLE9BQU8sS0FBSyxDQUFDLENBQUMsQ0FBQztBQUNqRCxRQUFJLEdBQUcsT0FBTyxNQUFNO0FBQ2xCLFVBQUk7QUFDRixjQUFNLElBQUksT0FBTyxPQUFPLE1B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
