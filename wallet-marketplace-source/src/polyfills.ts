// Essential polyfills for browser environment
import { Buffer } from 'buffer'

// Make Buffer globally available BEFORE any other imports
if (typeof window !== 'undefined') {
  ;(window as any).Buffer = Buffer
  ;(window as any).global = window
  
  if (!window.process) {
    ;(window as any).process = { 
      env: {},
      version: '',
      versions: {},
      browser: true
    }
  }
  
  // Add crypto polyfill if needed
  if (!window.crypto) {
    ;(window as any).crypto = {
      getRandomValues: function(arr: Uint8Array) {
        for (let i = 0; i < arr.length; i++) {
          arr[i] = Math.floor(Math.random() * 256)
        }
        return arr
      }
    }
  }
  
  // Ensure Fetch API classes exist for axios adapter detection
  // Axios checks for these at initialization but we don't need them since we use XHR
  if (typeof (window as any).Request === 'undefined') {
    ;(window as any).Request = class Request {}
  }
  if (typeof (window as any).Response === 'undefined') {
    ;(window as any).Response = class Response {}
  }
  if (typeof (window as any).Headers === 'undefined') {
    ;(window as any).Headers = class Headers {}
  }
}

// Export Buffer so other modules can use it
export { Buffer }
