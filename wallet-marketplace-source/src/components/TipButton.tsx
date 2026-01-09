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

const TipButton: React.FC<TipButtonProps> = () => {
  const { profile } = useWalletStore();
  const [status, setStatus] = useState<TipStatus | null>(null);
  const [coin, setCoin] = useState<"BTC" | "BCH" | "DOGE">("BTC");
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [showAddresses, setShowAddresses] = useState(false);

  // Real donation addresses
  const addresses = {
    BTC: "bc1qfxpwq5x8g5el2q9jhg38yy9vzg0n5zxhd7lfr9",
    BCH: "bitcoincash:qqlz5xn7ytn763xsgjrk95sggsqdhkp9w5926w0jsa",
    DOGE: "Coming Soon"
  };

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

  const handleCopyAddress = async () => {
    const address = addresses[coin];
    if (address === "Coming Soon") return;
    
    try {
      await navigator.clipboard.writeText(address);
      setSuccessMessage(`✅ ${coin} address copied to clipboard!`);
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError("Failed to copy address");
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
            value={coin}
            onChange={(e) =>
              setCoin(e.target.value as "BTC" | "BCH" | "DOGE")
            }
            className="rounded-lg border border-yellow-500/60 bg-black px-2 py-1 text-xs text-yellow-100 focus:outline-none focus:ring-2 focus:ring-yellow-500/50 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <option value="BTC">Bitcoin (BTC)</option>
            <option value="BCH">Bitcoin Cash (BCH)</option>
            <option value="DOGE" disabled style={{ opacity: 0.5 }}>DOGE (Coming Soon)</option>
          </select>
          <button
            onClick={() => setShowAddresses(!showAddresses)}
            className="rounded-xl px-4 py-2 text-xs font-semibold uppercase tracking-wide transition-all bg-yellow-500 hover:bg-yellow-400 hover:scale-105"
          >
            {showAddresses ? "Hide Address" : "Show Address"}
          </button>
        </div>
      </div>
      
      {/* Address Display */}
      {showAddresses && (
        <div className="mt-3 rounded-lg border border-yellow-500/30 bg-black/40 p-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex-1 min-w-0">
              <div className="text-[10px] text-yellow-300/60 mb-1 uppercase tracking-wide">
                {coin} Address
              </div>
              <div className={`font-mono text-xs break-all ${coin === "DOGE" ? "opacity-40 cursor-not-allowed" : "text-yellow-100"}`}>
                {addresses[coin]}
              </div>
              {coin === "DOGE" && (
                <div className="text-[10px] text-yellow-400/60 mt-1">
                  🚧 DOGE support coming soon
                </div>
              )}
            </div>
            {coin !== "DOGE" && (
              <button
                onClick={handleCopyAddress}
                className="px-3 py-1.5 rounded-lg bg-yellow-500/20 hover:bg-yellow-500/30 border border-yellow-500/40 transition-all flex-shrink-0"
                title="Copy address"
              >
                <span className="text-xs">📋 Copy</span>
              </button>
            )}
          </div>
        </div>
      )}
      
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
