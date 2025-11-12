import clsx from "classnames";
export default function Pane({title, children, className}:{title?:string; children:any; className?:string}) {
  return (
    <div className={clsx("rounded-xl border border-white/5 bg-[var(--panel)]/80 backdrop-blur p-3", className)}>
      {title && <div className="mb-2 text-sm font-semibold text-[var(--muted)]">{title}</div>}
      {children}
    </div>
  );
}
