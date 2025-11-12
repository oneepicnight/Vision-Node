import clsx from 'classnames'

export function Stat({label, value, className}:{label:string; value:any; className?:string}){
  return (
    <div className={clsx('flex flex-col', className)}>
      <div className="text-xs text-[var(--muted)]">{label}</div>
      <div className="text-lg font-semibold">{value}</div>
    </div>
  )
}

export function SmallLabel({children}:{children:any}){
  return <div className="text-xs text-[var(--muted)]">{children}</div>
}
