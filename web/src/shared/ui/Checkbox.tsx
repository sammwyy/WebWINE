import type { InputHTMLAttributes, ReactNode } from 'react'

interface CheckProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: ReactNode
}

export function Checkbox({ label, className = '', ...props }: CheckProps) {
  return (
    <label className={`ww-check ${className}`}>
      <input type="checkbox" className="ww-checkbox" {...props} />
      {label}
    </label>
  )
}

export function Radio({ label, className = '', ...props }: CheckProps) {
  return (
    <label className={`ww-check ${className}`}>
      <input type="radio" className="ww-radio" {...props} />
      {label}
    </label>
  )
}
