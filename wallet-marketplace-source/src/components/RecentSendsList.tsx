import React, { useEffect, useState } from 'react';

interface TxRecord {
  id: number;
  user_id: string;
  chain: string;
  to_address: string;
  amount: string;
  txid: string;
  status: string;
  created_at: string;
}

interface RecentSendsListProps {
  userId: string;
  limit?: number;
  refreshInterval?: number; // in milliseconds
}

export const RecentSendsList: React.FC<RecentSendsListProps> = ({
  userId,
  limit = 20,
  refreshInterval = 60000, // Default 60 seconds
}) => {
  const [sends, setSends] = useState<TxRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSends = async () => {
    try {
      const response = await fetch(`/wallet/sends?user_id=${encodeURIComponent(userId)}&limit=${limit}`);
      
      if (!response.ok) {
        throw new Error('Failed to fetch transaction history');
      }

      const data = await response.json();
      
      if (data.success) {
        setSends(data.sends || []);
        setError(null);
      } else {
        setError('Failed to load transaction history');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSends();

    // Set up periodic refresh
    const interval = setInterval(fetchSends, refreshInterval);

    return () => clearInterval(interval);
  }, [userId, limit, refreshInterval]);

  const formatDate = (isoString: string) => {
    const date = new Date(isoString);
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(date);
  };

  const shortenAddress = (address: string) => {
    if (address.length <= 16) return address;
    return `${address.slice(0, 8)}...${address.slice(-8)}`;
  };

  const getStatusBadge = (status: string) => {
    const styles = {
      broadcast: 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200',
      simulated: 'bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-200',
      failed: 'bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200',
      pending: 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-200',
    };

    const style = styles[status as keyof typeof styles] || styles.pending;

    return (
      <span className={`px-2 py-1 text-xs font-semibold rounded-full ${style}`}>
        {status}
      </span>
    );
  };

  if (loading) {
    return (
      <div className="text-center py-8">
        <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">Loading transaction history...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
        <p className="text-sm text-red-800 dark:text-red-200">{error}</p>
        <button
          onClick={fetchSends}
          className="mt-2 text-sm text-red-600 dark:text-red-400 hover:underline"
        >
          Retry
        </button>
      </div>
    );
  }

  if (sends.length === 0) {
    return (
      <div className="text-center py-8 bg-gray-50 dark:bg-gray-800/50 rounded-lg">
        <svg
          className="mx-auto h-12 w-12 text-gray-400"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
        <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">No transaction history yet</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Recent Sends</h3>
        <button
          onClick={fetchSends}
          className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
          disabled={loading}
        >
          Refresh
        </button>
      </div>

      <div className="space-y-2">
        {sends.map((tx) => (
          <div
            key={tx.id}
            className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 
                     rounded-lg hover:shadow-md transition-shadow"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center space-x-2 mb-1">
                  <span className="text-xs font-bold text-gray-500 dark:text-gray-400 uppercase">
                    {tx.chain}
                  </span>
                  {getStatusBadge(tx.status)}
                  <span className="text-xs text-gray-500 dark:text-gray-400">
                    {formatDate(tx.created_at)}
                  </span>
                </div>
                
                <div className="flex items-center space-x-2 mb-2">
                  <span className="text-sm font-semibold text-gray-900 dark:text-white">
                    {parseInt(tx.amount).toLocaleString()}
                  </span>
                  <span className="text-gray-400">→</span>
                  <span className="text-sm text-gray-600 dark:text-gray-300 font-mono">
                    {shortenAddress(tx.to_address)}
                  </span>
                </div>

                {tx.txid && tx.txid !== 'simulation-only' && (
                  <div className="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">
                    TXID: {tx.txid}
                  </div>
                )}
              </div>

              <button
                onClick={() => {
                  if (tx.txid && tx.txid !== 'simulation-only') {
                    navigator.clipboard.writeText(tx.txid);
                  }
                }}
                className="ml-2 p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                title="Copy TXID"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                  />
                </svg>
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

// Custom hook for using recent sends data
export const useRecentSends = (userId: string, limit: number = 20) => {
  const [sends, setSends] = useState<TxRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSends = async () => {
    try {
      setLoading(true);
      const response = await fetch(`/wallet/sends?user_id=${encodeURIComponent(userId)}&limit=${limit}`);
      
      if (!response.ok) {
        throw new Error('Failed to fetch transaction history');
      }

      const data = await response.json();
      
      if (data.success) {
        setSends(data.sends || []);
        setError(null);
      } else {
        setError('Failed to load transaction history');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSends();
  }, [userId, limit]);

  return { sends, loading, error, refresh: fetchSends };
};

export default RecentSendsList;
