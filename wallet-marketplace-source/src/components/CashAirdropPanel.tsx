import React, { useState } from 'react';
import axios from 'axios';

interface CashAirdropPanelProps {
  onAirdropComplete?: () => void;
}

const CashAirdropPanel: React.FC<CashAirdropPanelProps> = ({ onAirdropComplete }) => {
  const [targetAddress, setTargetAddress] = useState('');
  const [amount, setAmount] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const handleAirdrop = async () => {
    if (!targetAddress || !amount) {
      setError('Please provide both address and amount');
      return;
    }

    const numAmount = parseFloat(amount);
    if (isNaN(numAmount) || numAmount <= 0) {
      setError('Invalid amount');
      return;
    }

    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      const res = await axios.post('/admin/airdrop/cash', {
        recipients: [{ address: targetAddress, amount: numAmount.toString() }],
        requested_by: 'Guardian UI',
        reason: 'Manual airdrop from Guardian Control Panel',
      });

      const message = res.data?.message || `Successfully airdropped ${amount} CASH to ${targetAddress}`;
      setSuccess(message);
      setTargetAddress('');
      setAmount('');

      if (onAirdropComplete) {
        onAirdropComplete();
      }

      setTimeout(() => setSuccess(null), 5000);
    } catch (err: any) {
      console.error('Airdrop failed', err);
      const msg = err?.response?.data?.error || 'Airdrop failed. You may not have permission.';
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="rounded-2xl border border-emerald-500/40 bg-emerald-900/20 p-6 shadow-[0_0_25px_rgba(16,185,129,0.3)]">
      <div className="mb-4">
        <div className="flex items-center gap-2 mb-2">
          <span className="inline-flex h-3 w-3 animate-pulse rounded-full bg-emerald-400" />
          <h3 className="text-lg font-bold text-emerald-100 uppercase tracking-wide">
            CASH Airdrop Control
          </h3>
        </div>
        <p className="text-sm text-emerald-100/70">
          Guardian-only power: Create CASH out of thin air and send it to any address. Use responsibly. 👑
        </p>
      </div>

      <div className="space-y-4">
        <div>
          <label className="block text-xs font-semibold text-emerald-200 mb-2 uppercase tracking-wide">
            Target Address
          </label>
          <input
            type="text"
            value={targetAddress}
            onChange={(e) => setTargetAddress(e.target.value)}
            placeholder="0x..."
            disabled={loading}
            className="w-full rounded-lg border border-emerald-500/60 bg-black/50 px-4 py-2 text-sm text-emerald-100 placeholder-emerald-600/50 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        <div>
          <label className="block text-xs font-semibold text-emerald-200 mb-2 uppercase tracking-wide">
            Amount (CASH)
          </label>
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="1000"
            disabled={loading}
            min="0"
            step="0.00000001"
            className="w-full rounded-lg border border-emerald-500/60 bg-black/50 px-4 py-2 text-sm text-emerald-100 placeholder-emerald-600/50 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        <button
          onClick={handleAirdrop}
          disabled={loading}
          className={`w-full relative overflow-hidden rounded-xl px-6 py-3 text-sm font-bold uppercase tracking-wide transition-all ${
            loading
              ? 'bg-emerald-600/70 cursor-wait'
              : 'bg-emerald-500 hover:bg-emerald-400 hover:scale-105'
          }`}
        >
          <span className="relative z-10">
            {loading ? 'Processing...' : 'Execute Airdrop'}
          </span>
          {!loading && (
            <span className="pointer-events-none absolute inset-0 animate-pulse bg-emerald-300/20" />
          )}
        </button>
      </div>

      {error && (
        <div className="mt-4 rounded-lg border border-red-500/30 bg-red-900/20 px-4 py-3 animate-in fade-in slide-in-from-top-2">
          <p className="text-sm text-red-300">❌ {error}</p>
        </div>
      )}

      {success && (
        <div className="mt-4 rounded-lg border border-emerald-500/30 bg-emerald-900/30 px-4 py-3 animate-in fade-in slide-in-from-top-2">
          <p className="text-sm text-emerald-300">✅ {success}</p>
        </div>
      )}
    </div>
  );
};

export default CashAirdropPanel;
