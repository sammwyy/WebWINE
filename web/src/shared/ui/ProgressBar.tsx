interface ProgressBarProps {
  value?: number
  max?: number
  /** green (default), blue (e.g. disk usage), yellow (paused), red (error) */
  color?: 'green' | 'blue' | 'yellow' | 'red'
  /** marquee animation, ignores `value` */
  indeterminate?: boolean
  /** disable the moving highlight */
  static?: boolean
  thin?: boolean
  className?: string
}

export function ProgressBar({
  value = 0,
  max = 100,
  color = 'green',
  indeterminate = false,
  static: isStatic = false,
  thin = false,
  className = '',
}: ProgressBarProps) {
  const pct = Math.min(100, Math.max(0, (value / max) * 100))
  const classes = [
    'ww-progress',
    color,
    indeterminate && 'indeterminate',
    isStatic && 'static',
    thin && 'thin',
    className,
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <div
      className={classes}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={max}
      aria-valuenow={indeterminate ? undefined : value}
    >
      <div className="ww-progress-fill" style={{ width: `${pct}%` }} />
    </div>
  )
}
