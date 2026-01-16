import { useState, useEffect, useCallback, useRef } from 'react'
import { env } from '../utils/env'

export interface LogEntry {
  type: 'log' | 'connected'
  timestamp: number
  level?: 'info' | 'warn' | 'error'
  target?: string
  message: string
  category?: string
  chain_id?: string
  pow_fp?: string
  block_hash?: string
  height?: string
  miner?: string
  peer?: string
  fields?: Record<string, string>
}

export function useLogStream(nodeUrl: string = env.NODE_URL) {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [connected, setConnected] = useState(false)
  const [reconnecting, setReconnecting] = useState(false)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  const connect = useCallback(() => {
    try {
      // Convert HTTP to WS protocol
      const wsUrl = nodeUrl.replace(/^http/, 'ws')
      const socket = new WebSocket(`${wsUrl}/api/ws/logs`)

      socket.onopen = () => {
        console.log('[LogStream] Connected')
        setConnected(true)
        setReconnecting(false)
      }

      socket.onmessage = (event) => {
        try {
          const log: LogEntry = JSON.parse(event.data)
          
          // Skip connection messages
          if (log.type === 'connected') {
            console.log('[LogStream] Connection confirmed:', log.message)
            return
          }
          
          // Add to logs (keep last 500)
          setLogs(prev => [...prev.slice(-499), log])
        } catch (err) {
          console.error('[LogStream] Failed to parse message:', err)
        }
      }

      socket.onerror = (error) => {
        console.error('[LogStream] WebSocket error:', error)
        setConnected(false)
      }

      socket.onclose = () => {
        console.log('[LogStream] Disconnected')
        setConnected(false)
        wsRef.current = null
        
        // Auto-reconnect after 3 seconds
        setReconnecting(true)
        reconnectTimeoutRef.current = setTimeout(() => {
          console.log('[LogStream] Reconnecting...')
          connect()
        }, 3000)
      }

      wsRef.current = socket
    } catch (err) {
      console.error('[LogStream] Failed to connect:', err)
      setConnected(false)
      
      // Retry after 3 seconds
      setReconnecting(true)
      reconnectTimeoutRef.current = setTimeout(() => {
        connect()
      }, 3000)
    }
  }, [nodeUrl])

  useEffect(() => {
    connect()

    return () => {
      // Cleanup on unmount
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
      if (wsRef.current) {
        wsRef.current.close()
      }
    }
  }, [connect])

  const clearLogs = useCallback(() => {
    setLogs([])
  }, [])

  return { logs, connected, reconnecting, clearLogs }
}
