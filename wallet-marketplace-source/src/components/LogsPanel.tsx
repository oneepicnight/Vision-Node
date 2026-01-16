import { useState, useMemo, useRef, useEffect } from 'react'
import { useLogStream } from '../hooks/useLogStream'
import '../styles/logs-panel.css'

export default function LogsPanel() {
  const { logs, connected, reconnecting, clearLogs } = useLogStream()
  const [filter, setFilter] = useState<string>('all')
  const [search, setSearch] = useState<string>('')
  const [autoScroll, setAutoScroll] = useState<boolean>(true)
  const logStreamRef = useRef<HTMLDivElement>(null)

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (autoScroll && logStreamRef.current) {
      logStreamRef.current.scrollTop = logStreamRef.current.scrollHeight
    }
  }, [logs, autoScroll])

  // Filter logs
  const filteredLogs = useMemo(() => {
    return logs.filter(log => {
      // Category filter
      if (filter !== 'all' && log.category !== filter) return false
      
      // Search filter
      if (search && !log.message.toLowerCase().includes(search.toLowerCase())) {
        return false
      }
      
      return true
    })
  }, [logs, filter, search])

  // Get log level color
  const getLogColor = (level?: string) => {
    switch (level) {
      case 'error': return 'log-error'
      case 'warn': return 'log-warn'
      case 'info': return 'log-info'
      default: return 'log-default'
    }
  }

  // Get category badge color
  const getCategoryColor = (category?: string) => {
    switch (category) {
      case 'payout': return 'badge-payout'
      case 'canon': return 'badge-canon'
      case 'orphan': return 'badge-orphan'
      case 'reject': return 'badge-reject'
      case 'accept': return 'badge-accept'
      case 'strike': return 'badge-strike'
      case 'p2p': return 'badge-p2p'
      case 'sync': return 'badge-sync'
      case 'compat': return 'badge-compat'
      case 'miner_error': return 'badge-miner-error'
      default: return 'badge-general'
    }
  }

  // Format timestamp
  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000)
    return date.toLocaleTimeString()
  }

  // Export logs
  const exportLogs = () => {
    const content = filteredLogs
      .map(log => `[${formatTime(log.timestamp)}] ${log.message}`)
      .join('\n')
    
    const blob = new Blob([content], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `vision-logs-${Date.now()}.txt`
    a.click()
    URL.revokeObjectURL(url)
  }

  // Stats
  const stats = useMemo(() => {
    return {
      total: filteredLogs.length,
      payouts: logs.filter(l => l.category === 'payout').length,
      errors: logs.filter(l => l.level === 'error').length,
      warnings: logs.filter(l => l.level === 'warn').length
    }
  }, [filteredLogs, logs])

  return (
    <div className="logs-panel">
      {/* Header */}
      <div className="logs-header">
        <div className="logs-title">
          <h3>Live Logs</h3>
          <div className="logs-status">
            <div className={`status-indicator ${connected ? 'connected' : reconnecting ? 'reconnecting' : 'disconnected'}`} />
            <span>
              {connected ? 'Live' : reconnecting ? 'Reconnecting...' : 'Disconnected'}
            </span>
          </div>
        </div>
      </div>

      {/* Controls */}
      <div className="logs-controls">
        {/* Filter by category */}
        <select 
          value={filter} 
          onChange={(e) => setFilter(e.target.value)}
          className="logs-select"
        >
          <option value="all">All Categories</option>
          <option value="payout">💰 Payouts</option>
          <option value="canon">✅ Canonical</option>
          <option value="orphan">🔶 Orphans</option>
          <option value="reject">❌ Rejects</option>
          <option value="accept">✔️ Accepts</option>
          <option value="p2p">🌐 P2P</option>
          <option value="sync">🔄 Sync</option>
          <option value="strike">⚡ Strikes</option>
          <option value="miner_error">🚨 Miner Errors</option>
        </select>

        {/* Search */}
        <input
          type="text"
          placeholder="Search logs..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="logs-search"
        />

        {/* Auto-scroll toggle */}
        <label className="logs-checkbox">
          <input
            type="checkbox"
            checked={autoScroll}
            onChange={(e) => setAutoScroll(e.target.checked)}
          />
          Auto-scroll
        </label>

        {/* Actions */}
        <button onClick={clearLogs} className="logs-btn logs-btn-secondary">
          Clear
        </button>
        <button onClick={exportLogs} className="logs-btn logs-btn-primary">
          Export
        </button>
      </div>

      {/* Log stream */}
      <div className="logs-stream" ref={logStreamRef}>
        {filteredLogs.length === 0 ? (
          <div className="logs-empty">
            {logs.length === 0 ? (
              <>
                <p>Waiting for logs...</p>
                <small>{connected ? 'Connected to node' : 'Connecting...'}</small>
              </>
            ) : (
              <p>No logs match current filter</p>
            )}
          </div>
        ) : (
          filteredLogs.map((log, idx) => (
            <div key={idx} className={`log-entry ${getLogColor(log.level)}`}>
              <span className="log-timestamp">{formatTime(log.timestamp)}</span>
              {log.category && (
                <span className={`log-badge ${getCategoryColor(log.category)}`}>
                  {log.category.toUpperCase().replace('_', ' ')}
                </span>
              )}
              <span className="log-message">{log.message}</span>
              {log.height && (
                <span className="log-meta">#{log.height}</span>
              )}
              {log.peer && (
                <span className="log-meta">{log.peer}</span>
              )}
            </div>
          ))
        )}
      </div>

      {/* Stats */}
      <div className="logs-stats">
        <span>{stats.total} logs</span>
        <span>•</span>
        <span>{stats.payouts} payouts</span>
        <span>•</span>
        <span className="stat-errors">{stats.errors} errors</span>
        <span>•</span>
        <span className="stat-warnings">{stats.warnings} warnings</span>
      </div>
    </div>
  )
}
