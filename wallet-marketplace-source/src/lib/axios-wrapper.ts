// Wrapper to ensure Request/Response exist before axios loads
// This MUST be imported before axios in any file

// Ensure global has Request/Response before axios checks
if (typeof globalThis !== 'undefined') {
  if (!globalThis.Request) {
    (globalThis as any).Request = class Request {
      constructor(input: any, init?: any) {
        this.url = typeof input === 'string' ? input : input?.url;
        this.method = init?.method || 'GET';
      }
      url: string;
      method: string;
    };
  }
  if (!globalThis.Response) {
    (globalThis as any).Response = class Response {
      constructor(body?: any, init?: any) {
        this.body = body;
        this.status = init?.status || 200;
      }
      body: any;
      status: number;
    };
  }
  if (!globalThis.Headers) {
    (globalThis as any).Headers = class Headers {
      private map = new Map<string, string>();
      constructor(init?: any) {
        if (init) {
          Object.keys(init).forEach(key => {
            this.map.set(key.toLowerCase(), init[key]);
          });
        }
      }
      get(name: string) { return this.map.get(name.toLowerCase()); }
      set(name: string, value: string) { this.map.set(name.toLowerCase(), value); }
      has(name: string) { return this.map.has(name.toLowerCase()); }
    };
  }
}

// Now import and export axios
import axios from 'axios'

// Force XHR adapter
axios.defaults.adapter = 'xhr' as any

export default axios
export { axios }
