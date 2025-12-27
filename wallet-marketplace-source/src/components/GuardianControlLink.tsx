import React from 'react';
import { Link } from 'react-router-dom';
import { useGuardianAuth } from '../hooks/useGuardianAuth';

/**
 * Guardian Control Room Link
 * 
 * Only visible to the creator wallet.
 * Conditionally renders a styled link to /guardian.
 * 
 * Add this to your navbar or side menu.
 */
export const GuardianControlLink: React.FC = () => {
  const { isGuardianCreator, loading } = useGuardianAuth();

  // Don't show anything while loading
  if (loading) {
    return null;
  }

  // Only show if user is the creator
  if (!isGuardianCreator) {
    return null;
  }

  return (
    <Link
      to="/guardian"
      className="group relative flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-900/10 px-4 py-2 text-sm font-semibold text-emerald-300 transition-all hover:border-emerald-500/60 hover:bg-emerald-900/20 hover:shadow-[0_0_20px_rgba(16,185,129,0.3)]"
    >
      <span className="text-lg">🛡️</span>
      <span>Guardian Control Room</span>
      <div className="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-emerald-400 animate-pulse"></div>
    </Link>
  );
};

/**
 * Guardian Badge
 * 
 * Small badge to show in the header when creator is logged in.
 */
export const GuardianBadge: React.FC = () => {
  const { isGuardianCreator, loading } = useGuardianAuth();

  if (loading || !isGuardianCreator) {
    return null;
  }

  return (
    <div className="inline-flex items-center gap-2 rounded-full border border-emerald-500/40 bg-emerald-900/20 px-3 py-1 text-xs font-semibold text-emerald-200">
      <span className="relative flex h-2 w-2">
        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75"></span>
        <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-500"></span>
      </span>
      <span>Creator Mode</span>
    </div>
  );
};
