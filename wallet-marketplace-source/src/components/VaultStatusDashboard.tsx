import { useEffect, useState } from 'react'
import { getVaultStatus, getVaultEpochStatus, type VaultStatus, type VaultEpochStatus } from '../lib/api'
import { Shield, TrendingUp, Users, Briefcase } from 'lucide-react'

function formatRelativeTime(ms: number): string {
  if (!ms || ms <= 0) return '—'
  const now = Date.now()
  const diff = Math.floor((now - ms) / 1000)
  
  if (diff < 0) return 'in the future'
  if (diff < 60) return `${diff}s ago`
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return `${Math.floor(diff / 86400)}d ago`
}

export function VaultStatusDashboard() {
  const [vaultStatus, setVaultStatus] = useState<VaultStatus | null>(null)
  const [epochStatus, setEpochStatus] = useState<VaultEpochStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string>('')

  useEffect(() => {
    const loadVaultStatus = async () => {
      try {
        const status = await getVaultStatus()
        setVaultStatus(status)
        setError('')
      } catch (err) {
        console.error('Failed to load vault status:', err)
        setError(err instanceof Error ? err.message : 'Failed to load vault status')
      } finally {
        setLoading(false)
      }
    }

    const loadEpochStatus = async () => {
      try {
        const status = await getVaultEpochStatus()
        setEpochStatus(status)
      } catch (err) {
        console.debug('Vault epoch fetch failed:', err)
      }
    }

    loadVaultStatus()
    loadEpochStatus()
    const interval1 = setInterval(loadVaultStatus, 10000) // Update every 10s
    const interval2 = setInterval(loadEpochStatus, 10000)
    return () => { clearInterval(interval1); clearInterval(interval2); }
  }, [])

  if (loading) {
    return (
      <div className="card">
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-accent"></div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="card">
        <div className="text-center py-8 text-red-400">
          <p className="font-semibold mb-2">Failed to load vault status</p>
          <p className="text-sm opacity-70">{error}</p>
        </div>
      </div>
    )
  }

  if (!vaultStatus) return null

  const currencies = [
    { key: 'LAND', name: 'LAND', color: 'bg-purple-500/20 border-purple-500/30', decimals: 2 },
    { key: 'BTC', name: 'Bitcoin', color: 'bg-orange-500/20 border-orange-500/30', decimals: 8 },
    { key: 'BCH', name: 'Bitcoin Cash', color: 'bg-green-500/20 border-green-500/30', decimals: 8 },
    { key: 'DOGE', name: 'Dogecoin', color: 'bg-yellow-500/20 border-yellow-500/30', decimals: 2 }
  ] as const

  const stakeholders = [
    { key: 'miners', name: 'Miners', icon: TrendingUp, percentage: 50, color: 'text-green-400' },
    { key: 'dev', name: 'Development', icon: Briefcase, percentage: 30, color: 'text-blue-400' },
    { key: 'founders', name: 'Founders', icon: Users, percentage: 20, color: 'text-purple-400' }
  ] as const

  return (
    <div className="card">
      <div className="flex items-center gap-3 mb-6">
        <Shield size={24} className="text-accent" />
        <h2 className="text-2xl font-bold">Vault Status Dashboard</h2>
      </div>

      {/* Epoch Summary */}
      {epochStatus && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-6 gap-4 mb-6">
          <div className="p-4 rounded-lg bg-black/30">
            <div className="text-xs opacity-70">Vault Balance</div>
            <div className="text-lg font-bold font-mono">{Number(epochStatus.vault_balance).toLocaleString()}</div>
          </div>
          <div className="p-4 rounded-lg bg-black/30">
            <div className="text-xs opacity-70">Fund Balance</div>
            <div className="text-lg font-bold font-mono">{Number(epochStatus.fund_balance).toLocaleString()}</div>
          </div>
          <div className="p-4 rounded-lg bg-black/30">
            <div className="text-xs opacity-70">Treasury Balance</div>
            <div className="text-lg font-bold font-mono">{Number(epochStatus.treasury_balance).toLocaleString()}</div>
          </div>
          <div className="p-4 rounded-lg bg-black/30">
            <div className="text-xs opacity-70">Epoch Index</div>
            <div className="text-lg font-bold">#{epochStatus.epoch_index}</div>
            <div className={`text-xs mt-1 ${epochStatus.due ? 'text-yellow-400' : 'text-green-400'}`}>
              {epochStatus.due ? '🟡 Payout due' : '🟢 Next payout'}
            </div>
          </div>
          <div className="p-4 rounded-lg bg-black/30">
            <div className="text-xs opacity-70">Last Payout Height</div>
            <div className="text-lg font-bold">{epochStatus.last_payout_height}</div>
            <div className="text-xs opacity-70 mt-1">{formatRelativeTime(epochStatus.last_payout_at_ms)}</div>
          </div>
          <div className="p-4 rounded-lg bg-black/30">
            <div className="text-xs opacity-70">Chain Height</div>
            <div className="text-lg font-bold">{epochStatus.height}</div>
            <div className="text-xs opacity-70 mt-1">Blocks: {Math.max(0, epochStatus.height - epochStatus.last_payout_height)}</div>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {currencies.map(({ key, name, color, decimals }) => {
          const balances = vaultStatus.balances[key as keyof typeof vaultStatus.balances]
          const total = balances.miners + balances.dev + balances.founders

          return (
            <div key={key} className={`p-6 rounded-lg border ${color}`}>
              <div className="flex justify-between items-center mb-4">
                <h3 className="text-xl font-bold">{key}</h3>
                <span className="text-sm opacity-70">{name}</span>
              </div>

              <div className="mb-4">
                <div className="text-sm opacity-70 mb-1">Total Balance</div>
                <div className="text-3xl font-bold font-mono">
                  {total.toFixed(decimals)}
                </div>
              </div>

              <div className="space-y-3">
                {stakeholders.map(({ key: stakeholderKey, name: stakeholderName, icon: Icon, percentage, color: textColor }) => {
                  const balance = balances[stakeholderKey]
                  const percent = total > 0 ? (balance / total * 100) : percentage

                  return (
                    <div key={stakeholderKey} className="bg-black/30 rounded-lg p-3">
                      <div className="flex items-center justify-between mb-2">
                        <div className="flex items-center gap-2">
                          <Icon size={16} className={textColor} />
                          <span className="font-semibold">{stakeholderName}</span>
                        </div>
                        <span className={`text-sm font-bold ${textColor}`}>
                          {percentage}%
                        </span>
                      </div>

                      <div className="flex justify-between items-center">
                        <div className="flex-1">
                          <div className="h-2 bg-black/50 rounded-full overflow-hidden">
                            <div
                              className={`h-full ${textColor.replace('text-', 'bg-')}`}
                              style={{ width: `${percent}%` }}
                            />
                          </div>
                        </div>
                        <div className="ml-3 font-mono text-sm">
                          {balance.toFixed(decimals)}
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>

              {/* Distribution Verification */}
              <div className="mt-4 pt-4 border-t border-white/10">
                <div className="flex justify-between text-xs opacity-70">
                  <span>Distribution</span>
                  <span>50% / 30% / 20%</span>
                </div>
              </div>
            </div>
          )
        })}
      </div>

      {/* Summary Stats */}
      <div className="mt-6 p-4 bg-black/30 rounded-lg">
        <div className="grid grid-cols-3 gap-4 text-center">
          <div>
            <div className="text-2xl font-bold text-green-400">
              {Object.values(vaultStatus.balances).reduce((sum, b) => sum + b.miners, 0).toFixed(2)}
            </div>
            <div className="text-xs opacity-70 mt-1">Total Miners</div>
          </div>
          <div>
            <div className="text-2xl font-bold text-blue-400">
              {Object.values(vaultStatus.balances).reduce((sum, b) => sum + b.dev, 0).toFixed(2)}
            </div>
            <div className="text-xs opacity-70 mt-1">Total Dev</div>
          </div>
          <div>
            <div className="text-2xl font-bold text-purple-400">
              {Object.values(vaultStatus.balances).reduce((sum, b) => sum + b.founders, 0).toFixed(2)}
            </div>
            <div className="text-xs opacity-70 mt-1">Total Founders</div>
          </div>
        </div>
      </div>
    </div>
  )
}
