import { useEffect } from 'react'
import { useWalletStore } from '../state/wallet'
import { getWalletBalances } from '../lib/api'
import { Wallet } from 'lucide-react'

export function MultiCurrencyBalances() {
  const { profile, multiCurrencyBalances, setMultiCurrencyBalances } = useWalletStore()

  useEffect(() => {
    if (!profile) return

    const loadBalances = async () => {
      try {
        const balances = await getWalletBalances(profile.address)
        setMultiCurrencyBalances(balances)
      } catch (error) {
        console.error('Failed to load multi-currency balances:', error)
      }
    }

    loadBalances()
    const interval = setInterval(loadBalances, 5000)
    return () => clearInterval(interval)
  }, [profile, setMultiCurrencyBalances])

  if (!profile) return null

  const currencies = [
    { key: 'LAND', name: 'LAND', color: 'bg-purple-500/20 border-purple-500/30' },
    { key: 'BTC', name: 'Bitcoin', color: 'bg-orange-500/20 border-orange-500/30' },
    { key: 'BCH', name: 'Bitcoin Cash', color: 'bg-green-500/20 border-green-500/30' },
    { key: 'DOGE', name: 'Dogecoin', color: 'bg-yellow-500/20 border-yellow-500/30' }
  ] as const

  return (
    <div className="card">
      <div className="flex items-center gap-2 mb-4">
        <Wallet size={18} className="opacity-70" />
        <h3 className="text-lg font-semibold">Multi-Currency Wallet</h3>
      </div>
      
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {currencies.map(({ key, name, color }) => {
          const balance = multiCurrencyBalances[key] || { available: 0, locked: 0 }
          const total = balance.available + balance.locked
          
          return (
            <div key={key} className={`p-4 rounded-lg border ${color}`}>
              <div className="flex justify-between items-center mb-2">
                <span className="font-bold text-lg">{key}</span>
                <span className="text-sm opacity-70">{name}</span>
              </div>
              
              <div className="space-y-1 text-sm">
                <div className="flex justify-between">
                  <span className="opacity-70">Available:</span>
                  <span className="font-mono font-semibold">
                    {balance.available.toFixed(key === 'DOGE' ? 2 : 8)}
                  </span>
                </div>
                
                {balance.locked > 0 && (
                  <div className="flex justify-between">
                    <span className="opacity-70">Locked:</span>
                    <span className="font-mono text-yellow-400">
                      {balance.locked.toFixed(key === 'DOGE' ? 2 : 8)}
                    </span>
                  </div>
                )}
                
                <div className="flex justify-between pt-1 border-t border-white/10">
                  <span className="opacity-70">Total:</span>
                  <span className="font-mono font-bold">
                    {total.toFixed(key === 'DOGE' ? 2 : 8)}
                  </span>
                </div>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
