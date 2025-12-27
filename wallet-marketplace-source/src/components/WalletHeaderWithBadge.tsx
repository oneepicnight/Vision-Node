import React, { useEffect, useState } from "react";
import axios from "axios";
import BelieverBadge from "./BelieverBadge";
import TipButton from "./TipButton";

type TipStatus = {
  has_tipped: boolean;
  coin?: string;
  amount?: string;
  last_tip_at?: number;
  badge_label?: string;
};

interface WalletHeaderWithBadgeProps {
  walletAddress?: string;
  className?: string;
}

/**
 * Example component showing how to integrate the BelieverBadge
 * into your wallet dashboard.
 * 
 * Usage:
 * ```tsx
 * <WalletHeaderWithBadge walletAddress={myAddress} />
 * <TipButton onTipped={handleTipped} />
 * ```
 */
const WalletHeaderWithBadge: React.FC<WalletHeaderWithBadgeProps> = ({
  walletAddress = "0x0000000000000000000000000000000000000000",
  className = "",
}) => {
  const [tipStatus, setTipStatus] = useState<TipStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const loadTipStatus = async () => {
    try {
      setLoading(true);
      // Get wallet address from props or localStorage
      const address = walletAddress || localStorage.getItem("walletAddress") || "0xdefault";
      
      const res = await axios.post<TipStatus>("/wallet/tip/status", {
        wallet_address: address,
      });
      
      setTipStatus(res.data);
    } catch (err) {
      console.error("Failed to load tip status", err);
      // On error, assume not tipped
      setTipStatus({ has_tipped: false });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTipStatus();
  }, [walletAddress]);

  const handleTipped = () => {
    // Immediately update state to show badge
    setTipStatus({ has_tipped: true });
    
    // Optionally re-fetch to get full data
    setTimeout(() => loadTipStatus(), 500);
  };

  const hasTipped = tipStatus?.has_tipped === true;

  return (
    <div className={`space-y-6 ${className}`}>
      {/* Wallet Header */}
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-slate-700/50 bg-slate-900/50 p-6">
        <div className="flex-1">
          <div className="text-xs uppercase tracking-wide text-slate-400 mb-1">
            Your Wallet
          </div>
          <div className="text-sm font-mono text-slate-100 break-all">
            {walletAddress}
          </div>
        </div>

        <div className="flex items-center gap-3">
          {loading && (
            <div className="text-xs text-slate-400 animate-pulse">
              Loading status...
            </div>
          )}
          
          {!loading && hasTipped && <BelieverBadge />}
        </div>
      </div>

      {/* Tip Button - only shows if not tipped */}
      <TipButton onTipped={handleTipped} />

      {/* Optional: Show believer stats */}
      {hasTipped && tipStatus && (
        <div className="rounded-lg border border-emerald-500/20 bg-emerald-950/30 p-4 text-xs text-emerald-200">
          <div className="font-semibold mb-2">🌟 Believer Status</div>
          <div className="space-y-1 text-emerald-300/80">
            {tipStatus.coin && (
              <div>Tipped with: <span className="font-mono">{tipStatus.coin}</span></div>
            )}
            {tipStatus.amount && (
              <div>Amount: <span className="font-mono">{tipStatus.amount}</span> sats</div>
            )}
            {tipStatus.last_tip_at && (
              <div>Date: {new Date(tipStatus.last_tip_at * 1000).toLocaleDateString()}</div>
            )}
            <div className="mt-2 text-[10px] text-emerald-400/60">
              You were here early. You believed when it was still just stars and code.
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default WalletHeaderWithBadge;
