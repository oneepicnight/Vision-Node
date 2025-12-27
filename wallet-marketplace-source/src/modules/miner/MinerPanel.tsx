/**
 * Mining Panel Component
 * Provides UI for solo mining, pool hosting, and joining pools
 */

import { useState, useEffect } from 'react';
import { useWalletStore } from '../../state/wallet';
import { useConstellationStatus } from '../../hooks/useConstellationStatus';

interface MiningStats {
  current_hashrate: number;
  average_hashrate: number;
  blocks_found: number;
  blocks_accepted: number;
  total_rewards: number;
  threads: number;
  enabled: boolean;
}

interface DiscordStatus {
  linked: boolean;
  discord_user_id?: string;
  discord_username?: string;
}

interface PoolStats {
  worker_count: number;
  total_shares: number;
  total_hashrate: number;
  blocks_found: number;
  workers: WorkerStats[];
}

interface WorkerStats {
  worker_id: string;
  wallet_address: string;
  total_shares: number;
  reported_hashrate: number;
  estimated_payout: string;
}

type MiningMode = 'solo' | 'host_pool' | 'join_pool';

export function MinerPanel() {
  const { profile } = useWalletStore();
  const constellation = useConstellationStatus();
  const [mode, setMode] = useState<MiningMode>('solo');
  const [stats, setStats] = useState<MiningStats | null>(null);
  const [poolStats, setPoolStats] = useState<PoolStats | null>(null);
  const [threads, setThreads] = useState(4);
  const [poolUrl, setPoolUrl] = useState('http://127.0.0.1:7070');
  const [poolFee, setPoolFee] = useState(1.5);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [discordStatus, setDiscordStatus] = useState<DiscordStatus | null>(null);
  const [checkingDiscord, setCheckingDiscord] = useState(false);
  const [anchorToggling, setAnchorToggling] = useState(false);

  // Fetch current mining stats
  useEffect(() => {
    const fetchStats = async () => {
      try {
        const response = await fetch('/miner/status');
        if (response.ok) {
          const data = await response.json();
          // Map the API response to our interface
          setStats({
            enabled: data.enabled,
            threads: data.threads,
            current_hashrate: data.hashrate || 0,
            average_hashrate: data.average_hashrate || 0,
            blocks_found: data.blocks_found || 0,
            blocks_accepted: data.blocks_accepted || 0,
            total_rewards: data.total_rewards || 0
          });
        }
      } catch (err) {
        console.error('Failed to fetch mining stats:', err);
      }
    };

    fetchStats();
    const interval = setInterval(fetchStats, 2000);
    return () => clearInterval(interval);
  }, []);

  // Fetch pool stats if in host mode
  useEffect(() => {
    if (mode !== 'host_pool') return;

    const fetchPoolStats = async () => {
      try {
        const response = await fetch('/pool/stats');
        if (response.ok) {
          const data = await response.json();
          setPoolStats(data);
        }
      } catch (err) {
        console.error('Failed to fetch pool stats:', err);
      }
    };

    fetchPoolStats();
    const interval = setInterval(fetchPoolStats, 3000);
    return () => clearInterval(interval);
  }, [mode]);

  // Check Discord link status
  useEffect(() => {
    if (!profile?.address) return;

    const checkDiscordStatus = async () => {
      try {
        const response = await fetch(`/api/discord/status?wallet_address=${profile.address}`);
        if (response.ok) {
          const data = await response.json();
          setDiscordStatus(data);
        }
      } catch (err) {
        console.error('Failed to check Discord status:', err);
      }
    };

    checkDiscordStatus();
  }, [profile?.address]);

  // Link Discord account
  const linkDiscord = async () => {
    if (!profile?.address) return;
    
    setCheckingDiscord(true);
    try {
      const response = await fetch(`/api/discord/login?wallet_address=${profile.address}`);
      if (response.ok) {
        const data = await response.json();
        window.location.href = data.url; // Redirect to Discord OAuth
      } else {
        setError('Failed to initiate Discord linking');
      }
    } catch (err) {
      console.error('Failed to link Discord:', err);
      setError('Failed to link Discord');
    } finally {
      setCheckingDiscord(false);
    }
  };

  // Start/stop mining
  const toggleMining = async () => {
    setIsLoading(true);
    setError(null);
    
    try {
      if (stats?.enabled) {
        // Stop mining
        const response = await fetch('/miner/stop', { method: 'POST' });
        if (!response.ok) {
          throw new Error('Failed to stop mining');
        }
      } else {
        // Start mining
        const response = await fetch('/miner/start', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ threads })
        });
        if (!response.ok) {
          throw new Error('Failed to start mining');
        }
      }
    } catch (err) {
      setError(`Failed to ${stats?.enabled ? 'stop' : 'start'} mining`);
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  // Change mining mode
  const changeMiningMode = async (newMode: MiningMode) => {
    setIsLoading(true);
    setError(null);

    try {
      // Stop current mining
      if (stats?.enabled) {
        await fetch('/miner/stop', { method: 'POST' });
      }

      // Stop pool if hosting
      if (mode === 'host_pool') {
        await fetch('/pool/stop', { method: 'POST' });
      }

      // Set new mode
      if (newMode === 'host_pool') {
        // Start hosting pool
        const response = await fetch('/pool/start', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ pool_fee: poolFee })
        });
        if (!response.ok) {
          throw new Error('Failed to start pool');
        }
      } else if (newMode === 'join_pool') {
        // Join pool - this would need additional logic
        // For now just set the mode
      }

      // Note: /pool/mode is GET only, mode is set by starting pool
      setMode(newMode);
    } catch (err) {
      setError(`Failed to change mode to ${newMode}`);
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const formatHashrate = (hr: number) => {
    if (hr >= 1_000_000) return `${(hr / 1_000_000).toFixed(2)} MH/s`;
    if (hr >= 1_000) return `${(hr / 1_000).toFixed(2)} KH/s`;
    return `${hr.toFixed(2)} H/s`;
  };

  // Toggle anchor mode
  const toggleAnchorMode = async () => {
    setAnchorToggling(true);
    setError(null);
    
    try {
      const newAnchorState = !constellation?.is_anchor;
      
      // Note: This requires restarting the node with P2P_IS_ANCHOR env var
      // For now, show instructions to user
      alert(
        newAnchorState
          ? 'To enable ANCHOR mode:\n\n1. Stop Vision Node\n2. Set environment variable: P2P_IS_ANCHOR=true\n3. Forward port 7070 on your router\n4. Restart Vision Node\n\nSee ANCHOR_LEAF_GUIDE.md for full instructions.'
          : 'To disable ANCHOR mode:\n\n1. Stop Vision Node\n2. Remove P2P_IS_ANCHOR environment variable\n3. Restart Vision Node'
      );
    } catch (err) {
      setError('Failed to toggle anchor mode');
      console.error(err);
    } finally {
      setAnchorToggling(false);
    }
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold">Mining Control</h2>
        <div className="flex items-center gap-4">
          {/* Node Identity Display */}
          {constellation?.node_id && (
            <div className="px-4 py-2 bg-blue-900/20 border border-blue-500/30 rounded-lg">
              <div className="text-xs text-gray-400">Node ID</div>
              <div className="text-sm font-mono text-blue-300" title={constellation.node_id}>
                {constellation.node_id.slice(0, 8)}...{constellation.node_id.slice(-6)}
              </div>
              {constellation.node_pubkey_fingerprint && (
                <div className="text-xs text-gray-500">
                  Key: {constellation.node_pubkey_fingerprint}
                </div>
              )}
            </div>
          )}
          {/* Farm Management Link */}
          <a
            href="/#/mining"
            className="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg font-semibold transition-colors flex items-center gap-2"
            title="Manage distributed farm rigs"
          >
            🚜 Farm Management
          </a>
          
          {/* Discord Link Status */}
          {discordStatus?.linked ? (
            <div className="flex items-center gap-2 px-3 py-2 bg-green-900/20 border border-green-500/30 rounded-lg">
              <span className="text-green-400">✓</span>
              <span className="text-sm text-green-300">
                Discord: {discordStatus.discord_username}
              </span>
            </div>
          ) : (
            <button
              onClick={linkDiscord}
              disabled={checkingDiscord}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-semibold transition-colors disabled:opacity-50"
            >
              {checkingDiscord ? 'Loading...' : '🔗 Link Discord'}
            </button>
          )}
          
          {/* Anchor Mode Status/Toggle */}
          {constellation && (
            <button
              onClick={toggleAnchorMode}
              disabled={anchorToggling}
              className={`px-4 py-2 rounded-lg font-semibold transition-colors flex items-center gap-2 ${
                constellation.is_anchor
                  ? 'bg-green-600 hover:bg-green-500 text-white'
                  : 'bg-gray-600 hover:bg-gray-500 text-white'
              } disabled:opacity-50`}
              title={constellation.is_anchor ? `Anchor Mode: ${constellation.mode}` : 'Enable Anchor Mode (requires restart)'}
            >
              {constellation.is_anchor ? (
                <>
                  <span className="text-lg">⚓</span>
                  <span>Anchor: {constellation.mode}</span>
                  {constellation.public_reachable && <span className="text-xs">✓</span>}
                </>
              ) : (
                <>
                  <span className="text-lg">🍃</span>
                  <span>Leaf Mode</span>
                </>
              )}
            </button>
          )}
          
          {/* Mining Status */}
          <div className="flex items-center gap-2">
            <span className={`w-3 h-3 rounded-full ${stats?.enabled ? 'bg-green-500 animate-pulse' : 'bg-gray-400'}`} />
            <span className="text-sm text-gray-400">
              {stats?.enabled ? 'Mining Active' : 'Mining Stopped'}
            </span>
          </div>
        </div>
      </div>

      {error && (
        <div className="p-4 bg-red-900/20 border border-red-500/30 rounded-lg text-red-400">
          {error}
        </div>
      )}

      {/* Mining Mode Selector */}
      <div className="bg-white/5 border border-white/10 rounded-lg p-6 space-y-4">
        <h3 className="text-xl font-semibold">Mining Mode</h3>
        <div className="grid grid-cols-3 gap-3">
          <button
            onClick={() => changeMiningMode('solo')}
            disabled={isLoading}
            className={`p-4 rounded-lg border transition-colors ${
              mode === 'solo'
                ? 'border-accent bg-accent/20 text-accent'
                : 'border-white/10 hover:border-white/20'
            } disabled:opacity-50`}
          >
            <div className="font-semibold">Solo Mining</div>
            <div className="text-sm text-gray-400 mt-1">
              Mine independently, keep all rewards
            </div>
          </button>

          <button
            onClick={() => changeMiningMode('host_pool')}
            disabled={isLoading}
            className={`p-4 rounded-lg border transition-colors ${
              mode === 'host_pool'
                ? 'border-accent bg-accent/20 text-accent'
                : 'border-white/10 hover:border-white/20'
            } disabled:opacity-50`}
          >
            <div className="font-semibold">Host Pool</div>
            <div className="text-sm text-gray-400 mt-1">
              Coordinate workers, distribute rewards
            </div>
          </button>

          <button
            onClick={() => changeMiningMode('join_pool')}
            disabled={isLoading}
            className={`p-4 rounded-lg border transition-colors ${
              mode === 'join_pool'
                ? 'border-accent bg-accent/20 text-accent'
                : 'border-white/10 hover:border-white/20'
            } disabled:opacity-50`}
          >
            <div className="font-semibold">Join Pool</div>
            <div className="text-sm text-gray-400 mt-1">
              Contribute hashpower, earn proportionally
            </div>
          </button>
        </div>

        {mode === 'host_pool' && (
          <div className="mt-4 space-y-3">
            <label className="block">
              <span className="text-sm text-gray-400">Pool Fee (%)</span>
              <input
                type="number"
                min="0"
                max="10"
                step="0.1"
                value={poolFee}
                onChange={(e) => setPoolFee(parseFloat(e.target.value))}
                className="w-full mt-1 px-3 py-2 bg-black/30 border border-white/10 rounded-lg"
              />
            </label>
            <div className="text-xs text-gray-400">
              Note: Foundation tithe (1%) is automatically added. This fee is split among pool operators.
            </div>
          </div>
        )}

        {mode === 'join_pool' && (
          <div className="mt-4">
            <label className="block">
              <span className="text-sm text-gray-400">Pool URL</span>
              <input
                type="text"
                value={poolUrl}
                onChange={(e) => setPoolUrl(e.target.value)}
                placeholder="http://pool.example.com:7070"
                className="w-full mt-1 px-3 py-2 bg-black/30 border border-white/10 rounded-lg"
              />
            </label>
          </div>
        )}
      </div>

      {/* Mining Controls */}
      <div className="bg-white/5 border border-white/10 rounded-lg p-6 space-y-4">
        <h3 className="text-xl font-semibold">Mining Controls</h3>
        
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm text-gray-400 mb-2">
              Number of Threads
            </label>
            <input
              type="number"
              min="1"
              max={navigator.hardwareConcurrency || 16}
              value={threads}
              onChange={(e) => setThreads(parseInt(e.target.value))}
              disabled={stats?.enabled}
              className="w-full px-3 py-2 bg-black/30 border border-white/10 rounded-lg disabled:opacity-50"
            />
            <div className="text-xs text-gray-400 mt-1">
              Available CPU threads: {navigator.hardwareConcurrency || 'Unknown'}
            </div>
          </div>

          <div className="flex items-end">
            <button
              onClick={toggleMining}
              disabled={isLoading}
              className={`w-full py-3 rounded-lg font-semibold transition-colors ${
                stats?.enabled
                  ? 'bg-red-600 hover:bg-red-500 text-white'
                  : 'bg-green-600 hover:bg-green-500 text-white'
              } disabled:opacity-50`}
            >
              {isLoading ? 'Processing...' : stats?.enabled ? 'Stop Mining' : 'Start Mining'}
            </button>
          </div>
        </div>
      </div>

      {/* Mining Statistics */}
      {stats && (
        <div className="bg-white/5 border border-white/10 rounded-lg p-6 space-y-4">
          <h3 className="text-xl font-semibold">Mining Statistics</h3>
          
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Current Hashrate</div>
              <div className="text-2xl font-bold text-accent mt-1">
                {formatHashrate(stats.current_hashrate)}
              </div>
            </div>

            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Average Hashrate</div>
              <div className="text-2xl font-bold text-accent mt-1">
                {formatHashrate(stats.average_hashrate)}
              </div>
            </div>

            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Blocks Found</div>
              <div className="text-2xl font-bold text-green-400 mt-1">
                {stats.blocks_accepted} / {stats.blocks_found}
              </div>
            </div>

            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Total Rewards</div>
              <div className="text-2xl font-bold text-yellow-400 mt-1">
                {(stats.total_rewards / 100_000_000).toFixed(2)} LAND
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Pool Statistics (Host Mode) */}
      {mode === 'host_pool' && poolStats && (
        <div className="bg-white/5 border border-white/10 rounded-lg p-6 space-y-4">
          <h3 className="text-xl font-semibold">Pool Statistics</h3>
          
          <div className="grid grid-cols-4 gap-4 mb-6">
            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Active Workers</div>
              <div className="text-2xl font-bold text-blue-400 mt-1">
                {poolStats.worker_count}
              </div>
            </div>

            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Total Shares</div>
              <div className="text-2xl font-bold text-purple-400 mt-1">
                {poolStats.total_shares.toLocaleString()}
              </div>
            </div>

            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Pool Hashrate</div>
              <div className="text-2xl font-bold text-accent mt-1">
                {formatHashrate(poolStats.total_hashrate)}
              </div>
            </div>

            <div className="p-4 bg-black/20 rounded-lg">
              <div className="text-sm text-gray-400">Blocks Found</div>
              <div className="text-2xl font-bold text-green-400 mt-1">
                {poolStats.blocks_found}
              </div>
            </div>
          </div>

          {/* Worker List */}
          <div>
            <h4 className="text-lg font-semibold mb-3">Connected Workers</h4>
            <div className="overflow-auto">
              <table className="w-full text-sm">
                <thead className="bg-black/30">
                  <tr>
                    <th className="text-left p-3">Worker ID</th>
                    <th className="text-left p-3">Wallet</th>
                    <th className="text-right p-3">Shares</th>
                    <th className="text-right p-3">Hashrate</th>
                    <th className="text-right p-3">Est. Payout</th>
                  </tr>
                </thead>
                <tbody>
                  {poolStats.workers.map((worker) => (
                    <tr key={worker.worker_id} className="border-t border-white/5">
                      <td className="p-3 font-mono text-xs">{worker.worker_id}</td>
                      <td className="p-3 font-mono text-xs">
                        {worker.wallet_address.slice(0, 10)}...{worker.wallet_address.slice(-6)}
                      </td>
                      <td className="p-3 text-right">{worker.total_shares.toLocaleString()}</td>
                      <td className="p-3 text-right">{formatHashrate(worker.reported_hashrate)}</td>
                      <td className="p-3 text-right text-yellow-400">{worker.estimated_payout}</td>
                    </tr>
                  ))}
                  {poolStats.workers.length === 0 && (
                    <tr>
                      <td colSpan={5} className="p-6 text-center text-gray-400">
                        No workers connected yet
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {/* Vision Foundation Tithe Notice */}
      {mode !== 'solo' && (
        <div className="bg-blue-900/20 border border-blue-500/30 rounded-lg p-4">
          <div className="flex items-start gap-3">
            <div className="text-2xl">ℹ️</div>
            <div>
              <div className="font-semibold text-blue-300">Vision Foundation Tithe</div>
              <div className="text-sm text-gray-300 mt-1">
                All pool mining includes a 1% foundation tithe to sustain the Vision ecosystem.
                This tithe is automatically deducted from pool rewards and supports network development,
                security audits, and community initiatives.
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
