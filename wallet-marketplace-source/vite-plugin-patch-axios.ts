// Vite plugin to patch axios to not rely on global.Request/Response
export function patchAxios() {
  return {
    name: 'patch-axios',
    transform(code: string, id: string) {
      // Only patch axios adapter files
      if (id.includes('node_modules/axios')) {
        // Replace destructuring of Request/Response from global with safe fallback
        if (code.includes('Request:') && code.includes('Response:') && code.includes('.global')) {
          // Add safe fallback for global object
          code = code.replace(
            /\(\{Request:(\w+),Response:(\w+)\}\)=>\(\{Request:\1,Response:\2\}\)\)\((\w+)\.global\)/g,
            '(({Request:$1,Response:$2})=>({Request:$1||(globalThis.Request||class Request{}),Response:$2||(globalThis.Response||class Response{})}))(($3.global)||globalThis)'
          );
          
          console.log('[patch-axios] Patched axios adapter detection');
        }
      }
      return null;
    }
  };
}
