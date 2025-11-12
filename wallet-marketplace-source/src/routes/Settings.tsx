import { useState, useEffect } from 'react'
import { useWalletStore } from '../state/wallet'
import { getBaseUrl, setBaseUrl, pingStatus } from '../lib/api'
import { loadAndDecrypt } from '../lib/keystore'
import { del } from 'idb-keyval'

export default function Settings() {
  const { reset: resetWallet } = useWalletStore()
  const [nodeUrl, setNodeUrl] = useState('')
  const [nodeTestStatus, setNodeTestStatus] = useState('')
  const [showMnemonic, setShowMnemonic] = useState(false)
  const [mnemonic, setMnemonic] = useState<string[]>([])
  const [wipeConfirm, setWipeConfirm] = useState('')
  const [darkMode, setDarkMode] = useState(true)

  useEffect(() => {
    getBaseUrl().then(url => setNodeUrl(url));
    
    // Load theme preference
    const savedTheme = localStorage.getItem('vision.theme')
    if (savedTheme === 'light') {
      setDarkMode(false)
      document.documentElement.classList.remove('dark')
    } else {
      setDarkMode(true)
      document.documentElement.classList.add('dark')
    }
  }, [])

  const handleTestNode = async () => {
    setNodeTestStatus('Testing...')
    
    try {
      // Temporarily update base URL for test
      const originalUrl = await getBaseUrl()
      setBaseUrl(nodeUrl)
      
      const result = await pingStatus()
      
      if (result.up) {
        setNodeTestStatus('✓ Connected successfully')
        // Keep the new URL
      } else {
        setNodeTestStatus('✗ Connection failed')
        // Restore original URL
        setBaseUrl(originalUrl)
        setNodeUrl(originalUrl)
      }
    } catch (error) {
      setNodeTestStatus('✗ Network error')
      // Restore original URL on error
      const originalUrl = await getBaseUrl()
      setBaseUrl(originalUrl)
      setNodeUrl(originalUrl)
    }
    
    setTimeout(() => setNodeTestStatus(''), 3000)
  }

  const handleExportBackup = async () => {
    if (!confirm('This will display your recovery words. Make sure no one is watching your screen.')) {
      return
    }

    try {
      const keystore = await loadAndDecrypt()
      if (!keystore) {
        window.pushToast?.('Could not decrypt keystore. You may need to re-create your wallet.', 'error')
        return
      }
      
      setMnemonic(keystore.mnemonic)
      setShowMnemonic(true)
    } catch (error) {
      console.error('Export failed:', error)
      window.pushToast?.('Failed to export backup. Please try again.', 'error')
    }
  }

  const handleWipeWallet = async () => {
    if (wipeConfirm !== 'VISION') {
      window.pushToast?.('Type "VISION" to confirm wallet wipe.', 'info')
      return
    }

    if (!confirm('This will permanently delete your wallet. Are you absolutely sure?')) {
      return
    }

    try {
      // Clear IndexedDB
      await del('vision.keystore')
      await del('vision.device.secret')
      
      // Clear localStorage
      localStorage.removeItem('vision-wallet')
      localStorage.removeItem('vision.node.url')
      localStorage.removeItem('vision.theme')
      
      // Reset wallet state
      resetWallet()
      
  window.pushToast?.('Wallet wiped successfully. Redirecting...', 'success')
  window.location.href = '/'
    } catch (error) {
      console.error('Wipe failed:', error)
      window.pushToast?.('Failed to wipe wallet. Please try again.', 'error')
    }
  }

  const handleThemeToggle = () => {
    const newDarkMode = !darkMode
    setDarkMode(newDarkMode)
    
    if (newDarkMode) {
      document.documentElement.classList.add('dark')
      localStorage.setItem('vision.theme', 'dark')
    } else {
      document.documentElement.classList.remove('dark')
      localStorage.setItem('vision.theme', 'light')
    }
  }

  return (
    <div className="min-h-screen p-6">
      <div className="max-w-2xl mx-auto space-y-6">
        <h2 className="text-3xl font-bold">Settings</h2>

        {/* Node Configuration */}
        <div className="bg-white/5 border border-white/10 rounded-lg p-6 space-y-4">
          <h3 className="text-xl font-semibold">Node Configuration</h3>
          <div className="space-y-3">
            <label className="block">
              <span className="text-sm text-slate-400 mb-1 block">Node URL</span>
              <div className="flex gap-2">
                <input 
                  value={nodeUrl}
                  onChange={(e) => setNodeUrl(e.target.value)}
                  className="flex-1 bg-black/60 border border-white/20 rounded p-2 text-white"
                  placeholder="http://127.0.0.1:7070"
                />
                <button 
                  onClick={handleTestNode}
                  className="px-4 py-2 bg-accent/20 text-accent border border-accent/30 rounded hover:bg-accent/30 transition-colors"
                >
                  Test & Save
                </button>
              </div>
            </label>
            {nodeTestStatus && (
              <div className={`text-sm ${nodeTestStatus.includes('✓') ? 'text-green-400' : 'text-red-400'}`}>
                {nodeTestStatus}
              </div>
            )}
          </div>
        </div>

        {/* Security */}
        <div className="bg-white/5 border border-white/10 rounded-lg p-6 space-y-4">
          <h3 className="text-xl font-semibold">Security</h3>
          <div className="space-y-3">
            <button 
              onClick={handleExportBackup}
              className="w-full py-3 bg-yellow-600/20 text-yellow-400 border border-yellow-500/30 rounded font-semibold hover:bg-yellow-600/30 transition-colors"
            >
              Export Backup
            </button>
            <p className="text-sm text-slate-400">
              Display recovery words after confirmation dialog
            </p>
            
            {showMnemonic && (
              <div className="mt-4 p-4 bg-yellow-900/20 border border-yellow-500/30 rounded">
                <h4 className="font-semibold text-yellow-400 mb-3">Recovery Words</h4>
                <div className="grid grid-cols-3 gap-2 mb-3">
                  {mnemonic.map((word, index) => (
                    <div key={index} className="bg-black/40 p-2 rounded text-center text-sm">
                      <span className="text-slate-400">{index + 1}.</span> {word}
                    </div>
                  ))}
                </div>
                <button 
                  onClick={() => setShowMnemonic(false)}
                  className="text-sm text-slate-400 hover:text-white"
                >
                  Hide words
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Danger Zone */}
        <div className="bg-red-950/20 border border-red-500/30 rounded-lg p-6 space-y-4">
          <h3 className="text-xl font-semibold text-red-400">Danger Zone</h3>
          <div className="space-y-3">
            <input
              type="text"
              placeholder="Type VISION to confirm"
              value={wipeConfirm}
              onChange={(e) => setWipeConfirm(e.target.value)}
              className="w-full bg-black/60 border border-red-500/30 rounded p-2 text-white placeholder-slate-500"
            />
            <button 
              onClick={handleWipeWallet}
              disabled={wipeConfirm !== 'VISION'}
              className="w-full py-3 bg-red-600/20 text-red-400 border border-red-500/30 rounded font-semibold hover:bg-red-600/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Wipe Local Wallet
            </button>
            <p className="text-sm text-slate-400">
              Clears all local data. Type 'VISION' to confirm.
            </p>
          </div>
        </div>

        {/* Theme */}
        <div className="bg-white/5 border border-white/10 rounded-lg p-6 space-y-4">
          <h3 className="text-xl font-semibold">Appearance</h3>
          <div className="flex items-center justify-between">
            <span>Dark mode</span>
            <label className="relative inline-flex items-center cursor-pointer">
              <input 
                type="checkbox" 
                checked={darkMode}
                onChange={handleThemeToggle}
                className="sr-only peer" 
              />
              <div className="w-11 h-6 bg-gray-600 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-accent"></div>
            </label>
          </div>
        </div>
      </div>
    </div>
  )
}