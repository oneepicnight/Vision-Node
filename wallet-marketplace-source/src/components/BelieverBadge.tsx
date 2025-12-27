import React from "react";

interface BelieverBadgeProps {
  className?: string;
  tooltipText?: string;
}

const BelieverBadge: React.FC<BelieverBadgeProps> = ({
  className = "",
  tooltipText = "This wallet helped buy the creator a drink while Vision was still being forged.",
}) => {
  return (
    <div
      className={
        "group relative inline-flex items-center gap-1.5 rounded-full border border-emerald-400/60 bg-emerald-900/30 px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wide text-emerald-200 shadow-[0_0_15px_rgba(16,185,129,0.5)] transition-all hover:shadow-[0_0_25px_rgba(16,185,129,0.7)] hover:scale-105 " +
        className
      }
      title={tooltipText}
    >
      <span className="text-xs animate-pulse">🌟</span>
      <span>Thank you for believing in my dream</span>
      
      {/* Tooltip on hover */}
      <div className="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden w-64 rounded-lg border border-emerald-500/30 bg-emerald-950/95 px-3 py-2 text-[10px] normal-case tracking-normal text-emerald-100 shadow-xl group-hover:block">
        <div className="text-center leading-relaxed">
          {tooltipText}
        </div>
        <div className="absolute top-full left-1/2 -translate-x-1/2 -mt-1 h-0 w-0 border-l-4 border-r-4 border-t-4 border-l-transparent border-r-transparent border-t-emerald-500/30" />
      </div>
    </div>
  );
};

export default BelieverBadge;
