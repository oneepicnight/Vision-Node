import React from 'react'
import { getVault } from '../modules/wallet'

function friendlyErrorMessage(raw: string | null) {
  if (!raw) return null
  const s = raw.toLowerCase()
  if (s.includes('404')) {
    return `Server responded 404 — the process listening on the node port appears to be the market server, not the Vision node API. Start the Vision node (vision-node.exe) or point the Node URL in Settings to a node that implements /status and /vault.`
  }
  if (s.includes('failed to fetch') || s.includes('networkerror') || s.includes('ec o n n') || s.includes('refused') || s.includes('unable to connect')) {
    return `Cannot reach Vision node at the configured address (connection refused). Start the node or update the Node URL in Settings.`
  }
  return raw
}

export function VaultCard() {
  const [vault, setVault] = React.useState<any>(null)
  const [err, setErr] = React.useState<string | null>(null)

  React.useEffect(() => {
    let alive = true
    async function tick() {
      try {
        const v = await getVault()
        if (alive) {
          setErr(null)
          setVault(v)
        }
      } catch (e:any) {
        if (alive) {
          const raw = String(e?.message ?? e)
          setErr(raw)
        }
      }
    }
    tick()
    const id = setInterval(tick, 3000)
    return () => { alive = false; clearInterval(id) }
  }, [])

  if (err) {
    const friendly = friendlyErrorMessage(err)
    return (
      <div className="p-4 border rounded">
        <div className="font-semibold mb-2">Vision Vault</div>
        <div className="text-sm text-red-600">{friendly ?? `Vault error: ${err}`}</div>
        <div className="mt-3 text-xs opacity-80">Quick actions:</div>
        <ul className="mt-2 text-xs list-disc pl-5">
          <li>Open <a className="underline" href="/settings">Settings</a> and verify the Node URL.</li>
          <li>Start the Vision node locally: <code>.\\vision-node.exe</code> (or run your node service).</li>
        </ul>
      </div>
    )
  }

  if (!vault) return <div className="p-4 border rounded">Loading vault…</div>

  return (
    <div className="p-4 border rounded shadow-sm">
      <div className="flex items-center justify-between mb-1">
        <div className="font-semibold">Vision Vault</div>
        <div>
          <a href="/exchange" className="px-2 py-1 text-sm border rounded">Exchange</a>
        </div>
      </div>
      <div className="text-sm opacity-80">Auto-updates every 3s</div>
      <pre className="text-xs mt-2 bg-black/5 p-2 rounded overflow-auto">{JSON.stringify(vault, null, 2)}</pre>
    </div>
  )
}
