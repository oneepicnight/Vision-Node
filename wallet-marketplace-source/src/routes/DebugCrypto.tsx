/**
 * DebugCrypto
 *  - Diagnostic route used during development to exercise the
 *    WebCrypto Subtle API (importKey / encrypt / decrypt) and collect
 *    error stacks when browsers report "UnknownError: Internal error.".
 *
 * Usage: open `/debug/crypto` while running the dev server. The page
 * prints environment info and a timestamped log of each crypto step.
 */
import { useEffect, useState } from 'react'
import Pane from '../components/Pane'

function InfoRow({k, v}:{k:string; v:any}){
  return (
    <div className="flex justify-between text-sm text-[var(--muted)] py-0.5">
      <div className="font-medium">{k}</div>
      <div className="select-all">{String(v)}</div>
    </div>
  )
}

export default function DebugCrypto(){
  const [logs, setLogs] = useState<string[]>([])
  const [result, setResult] = useState<string>('idle')

  const push = (s:string)=> setLogs(l=>[...l, `${new Date().toISOString()} ${s}`])

  useEffect(()=>{
    const run = async()=>{
      push('Starting WebCrypto diagnostic')
      try{
        if (!('crypto' in window) || !('subtle' in (window as any).crypto)){
          push('WebCrypto Subtle API not available')
          setResult('no-subtle')
          return
        }

        const subtle = (window as any).crypto.subtle

        // Basic symmetric AES-GCM test
        const keyData = crypto.getRandomValues(new Uint8Array(32))
        push('Generated random 256-bit key material')

        const imported = await subtle.importKey('raw', keyData, {name:'AES-GCM'}, false, ['encrypt','decrypt'])
        push('importKey succeeded')

        const plain = new TextEncoder().encode('hello-webcrypto-test')
        const iv = crypto.getRandomValues(new Uint8Array(12))
        push('Encrypting payload...')
        const ct = await subtle.encrypt({name:'AES-GCM', iv}, imported, plain)
        push('Encrypt succeeded: ' + (ct instanceof ArrayBuffer ? `${(ct as ArrayBuffer).byteLength} bytes` : typeof ct))

        push('Decrypting payload...')
        const pt = await subtle.decrypt({name:'AES-GCM', iv}, imported, ct)
        const decoded = new TextDecoder().decode(new Uint8Array(pt))
        push('Decrypt succeeded: ' + decoded)

        setResult('ok')
      }catch(err:any){
        push('ERROR: ' + (err && err.message ? err.message : String(err)))
        push('STACK: ' + (err && err.stack ? err.stack : 'no-stack'))
        setResult('error')
      }
    }

    run()
  }, [])

  return (
    <div className="p-4 min-h-screen bg-[var(--bg)] text-[var(--text)]">
      <h2 className="text-2xl mb-2">WebCrypto Diagnostic</h2>
      <div className="grid grid-cols-3 gap-4">
        <div className="col-span-2">
          <Pane title="Environment">
            <InfoRow k="isSecureContext" v={window.isSecureContext} />
            <InfoRow k="userAgent" v={navigator.userAgent} />
            <InfoRow k="platform" v={navigator.platform} />
            <InfoRow k="language" v={navigator.language} />
          </Pane>

          <Pane title="Result">
            <div className="text-sm">Status: <strong>{result}</strong></div>
            <div className="mt-2 text-xs text-[var(--muted)]">Logs:</div>
            <div className="mt-2 bg-black/20 p-2 rounded max-h-64 overflow-auto">
              {logs.map((l,i)=>(<div key={i} className="text-xs font-mono">{l}</div>))}
            </div>
          </Pane>
        </div>
        <div>
          <Pane title="Instructions">
            <div className="text-sm text-[var(--muted)]">
              Use this page to reproduce WebCrypto failures. Open the browser console and note any extension errors.
              If you see an "UnknownError: Internal error", copy the logs and the full console output and share them.
            </div>
          </Pane>
        </div>
      </div>
    </div>
  )
}
