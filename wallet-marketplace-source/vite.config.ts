import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { createMockWalletMiddleware } from './src/dev/mockWallet'

// Plugin to patch the bundled output after minification
const patchAxiosOutput = () => ({
  name: 'patch-axios-output',
  generateBundle(_options: any, bundle: any) {
    for (const fileName in bundle) {
      const chunk = bundle[fileName];
      if (chunk.type === 'chunk' && chunk.code) {
        // Patch: Sz=(({Request:e,Response:t})=>...))(F.global)
        // The result is used directly, so we need to use || operator for fallback
        const original = chunk.code;
        chunk.code = chunk.code.replace(
          /\((\w+)\.global\),\{ReadableStream/g,
          '($1.global||globalThis),{ReadableStream'
        );
        // Also add fallback empty classes if global is undefined
        chunk.code = chunk.code.replace(
          /=\(\(\{Request:(\w+),Response:(\w+)\}\)=>\(\{Request:\1,Response:\2\}\)\)\(/g,
          '=(({Request:$1,Response:$2})=>({Request:$1||class{},Response:$2||class{}}))('
        );
        // Fix ReadableStream/TextEncoder destructuring from F.global
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

// https://vitejs.dev/config/
export default defineConfig(({ command, mode }) => {
  // Disable mock middleware by default so wallet talks to real node
  // Set VITE_ENABLE_MOCK=1 to enable mock mode for testing
  const enableMock = process.env.VITE_ENABLE_MOCK === '1'
  
  console.log('[vite.config] Mock middleware:', enableMock ? 'ENABLED' : 'DISABLED')

  return {
    plugins: [
      react(),
      patchAxiosOutput(),
    ],
    base: '/app/',  // Wallet is served at /app by the node
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
        'buffer': 'buffer/'
      }
    },
    define: {
      global: 'globalThis',
      'process.env': {},
      'process.env.NODE_ENV': JSON.stringify(mode),
      // Ensure these exist for axios adapter detection
      'globalThis.Request': 'globalThis.Request || class Request {}',
      'globalThis.Response': 'globalThis.Response || class Response {}',
      'globalThis.Headers': 'globalThis.Headers || class Headers {}'
    },
    build: {
      commonjsOptions: {
        transformMixedEsModules: true
      }
    },
    optimizeDeps: {
      include: ['buffer']
    },
    server: {
      host: '127.0.0.1',
      port: 4173,
      proxy: enableMock ? undefined : {
        // All /api/* requests go to Vision Node at port 7070
        '/api': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        // WebSocket endpoint for exchange stream
        '/api/exchange/stream': {
          target: 'ws://127.0.0.1:7070',
          ws: true,
          changeOrigin: true
        },
        // Legacy endpoints (without /api prefix) also go to Vision Node
        '/status': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        '/vault': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        '/keys': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        '/supply': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        '/receipts': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        '/wallet': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        '/tx': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        },
        '/balance': {
          target: 'http://127.0.0.1:7070',
          changeOrigin: true
        }
      }
    },
    configureServer(server) {
      if (enableMock) {
        // Inject the mock wallet middleware so requests to the dev server
        // can respond to /status, /vault, /keys and related wallet endpoints.
        server.middlewares.use(createMockWalletMiddleware())
        // eslint-disable-next-line no-console
        console.log('[vite] mock wallet middleware enabled')
      }
    }
  }
})
