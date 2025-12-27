import { useState, useEffect } from 'react'

export type P2PHealthState = 'stable' | 'weak' | 'broken'

export interface ConstellationStatus {
  local_ebid: string
  total_known_peers: number
  connected_peers: number
  hot_peers: number
  warm_peers: number
  cold_peers: number
  last_successful_guardian_check: number | null
  guardian_reachable: boolean
  guardian_ebid: string | null
  avg_peer_latency_ms: number | null
  max_peer_latency_ms: number | null
  sync_height: number
  network_estimated_height: number
  is_syncing: boolean
  last_peer_event_at: number | null
  p2p_debug_mode: boolean
  is_anchor: boolean
  public_reachable: boolean
  mode: string
  node_id: string | null
  node_pubkey: string | null
  node_pubkey_fingerprint: string | null
  mining_blocked_reason?: string | null
  p2p_health?: string
}

export function computeP2PHealth(status: ConstellationStatus | null): P2PHealthState {
  if (!status) return 'broken'

  const peers = status.connected_peers ?? 0
  const guardian = status.guardian_reachable
  const avgLatency = status.avg_peer_latency_ms ?? null

  // Broken: no peers and guardian not reachable
  if (peers === 0 && !guardian) {
    return 'broken'
  }

  // Stable: good peers + reasonable latency + guardian reachable
  if (
    peers >= 3 &&
    guardian &&
    (avgLatency === null || avgLatency <= 200)
  ) {
    return 'stable'
  }

  // Everything else is "weak"
  return 'weak'
}

export function useConstellationStatus(pollInterval = 10000) {
  const [constellation, setConstellation] = useState<ConstellationStatus | null>(null)

  useEffect(() => {
    let cancelled = false

    const load = async () => {
      try {
        const res = await fetch('http://127.0.0.1:7070/constellation/status')
        const data = await res.json()
        if (!cancelled) setConstellation(data)
      } catch (err) {
        console.debug('Failed to load constellation status:', err)
        if (!cancelled) setConstellation(null)
      }
    }

    load()
    const id = setInterval(load, pollInterval)
    return () => {
      cancelled = true
      clearInterval(id)
    }
  }, [pollInterval])

  return constellation
}
