import React, { useEffect, useState } from "react";
import axios from "axios";
import { useWalletStore } from "../state/wallet";

type TipStatus = {
  has_tipped: boolean;
  coin?: string;
  amount?: string;
  last_tip_at?: number;
  badge_label?: string;
};

interface TipButtonProps {
  onTipped?: () => void;
}

const TipButton: React.FC<TipButtonProps> = ({ onTipped }) => {
  const { profile } = useWalletStore();
  const [status, setStatus] = useState<TipStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [coin, setCoin] = useState<"BTC" | "BCH" | "DOGE">("BTC");
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  console.log("[TipButton] Rendered, profile:", profile, "status:", status);

  // Get wallet address from Zustand store
  const getWalletAddress = () => {
    return profile?.address || "0xdefault";
  };

  useEffect(() => {
    console.log("[TipButton useEffect] profile:", profile);
    if (!profile) {
      // Set initial state even if no profile
      console.log("[TipButton] No profile, setting default status");
      setStatus({ has_tipped: false });
      return;
    }
    
    let cancelled = false;
    const loadStatus = async () => {
      try {
        const walletAddress = getWalletAddress();
        console.log("[TipButton] Loading status for wallet:", walletAddress);
        const res = await axios.post<TipStatus>("/wallet/tip/status", {
          wallet_address: walletAddress,
        });
        console.log("[TipButton] Status loaded:", res.data);
        if (!cancelled) setStatus(res.data);
      } catch (err) {
        console.error("[TipButton] Failed to load tip status", err);
        // If error, assume not tipped and show button
        if (!cancelled) {
          console.log("[TipButton] Setting fallback status after error");
          setStatus({ has_tipped: false });
        }
      }
    };
    loadStatus();
    return () => {
      cancelled = true;
    };
  }, [profile]);

  // Don't show while loading or if already tipped
  console.log("[TipButton render check] status:", status, "has_tipped:", status?.has_tipped);
  if (!status || status.has_tipped) {
    console.log("[TipButton] Returning null - status:", status);
    return null;
  }
  console.log("[TipButton] Rendering button");

  const handleTip = async () => {
    setLoading(true);
    setError(null);
    try {
      const walletAddress = getWalletAddress();
      const res = await axios.post("/wallet/tip", {
        wallet_address: walletAddress,
        coin,
      });
      
      const message = res.data?.message || "Thanks for the drink. You are officially not an asshole.";
      setSuccessMessage(message);
      setStatus({ has_tipped: true });
      
      // Notify parent component so badge can appear
      if (onTipped) {
        onTipped();
      }
      
      // Hide success message after 5 seconds
      setTimeout(() => setSuccessMessage(null), 5000);
    } catch (err: any) {
      console.error("Tip failed", err);
      const msg =
        err?.response?.data?.error ||
        "Tip failed. Either you are broke or the node is grumpy.";
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="mt-4 rounded-2xl border border-yellow-500/40 bg-yellow-900/20 p-4 text-sm text-yellow-100 shadow-[0_0_25px_rgba(253,224,71,0.3)]">
      <div className="flex items-center justify-between gap-4">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <span className="inline-flex h-3 w-3 animate-pulse rounded-full bg-yellow-400" />
            <span className="text-xs uppercase tracking-wide text-yellow-300 font-semibold">
              Buy the madman a drink
            </span>
          </div>
          <p className="mt-1 text-xs text-yellow-100/80 leading-relaxed">
            Toss me the equivalent of <strong>$3</strong> in your favorite coin (not LAND) for
            building this insanity. Totally optional… but this button will keep
            flashing at you like a bartender who just got stiffed. 💀
          </p>
        </div>
        <div className="flex flex-col items-end gap-2">
          <select
            disabled={loading}
            value={coin}
            onChange={(e) =>
              setCoin(e.target.value as "BTC" | "BCH" | "DOGE")
            }
            className="rounded-lg border border-yellow-500/60 bg-black px-2 py-1 text-xs text-yellow-100 focus:outline-none focus:ring-2 focus:ring-yellow-500/50 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <option value="BTC">BTC</option>
            <option value="BCH">BCH</option>
            <option value="DOGE">DOGE</option>
          </select>
          <button
            disabled={loading}
            onClick={handleTip}
            className={`relative overflow-hidden rounded-xl px-4 py-2 text-xs font-semibold uppercase tracking-wide transition-all ${
              loading
                ? "bg-yellow-600/70 cursor-wait"
                : "bg-yellow-500 hover:bg-yellow-400 hover:scale-105"
            }`}
          >
            <span className="relative z-10">
              {loading ? "Sending..." : "Tip $3"}
            </span>
            {!loading && (
              <span className="pointer-events-none absolute inset-0 animate-pulse bg-yellow-300/20" />
            )}
          </button>
        </div>
      </div>
      {error && (
        <div className="mt-3 rounded-lg border border-red-500/30 bg-red-900/20 px-3 py-2">
          <p className="text-xs text-red-300">
            ❌ {error}
          </p>
        </div>
      )}
      {successMessage && (
        <div className="mt-3 rounded-lg border border-emerald-500/30 bg-emerald-900/20 px-3 py-2 animate-in fade-in slide-in-from-top-2">
          <p className="text-xs text-emerald-300">
            ✅ {successMessage}
          </p>
          <p className="text-[10px] text-emerald-400/60 mt-1">
            This button will now disappear forever. You are officially on the "not a jerk" list.
          </p>
        </div>
      )}
    </div>
  );
};

export default TipButton;
