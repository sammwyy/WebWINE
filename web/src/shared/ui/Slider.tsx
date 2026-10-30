import type { InputHTMLAttributes } from 'react'

export function Slider({ className = '', ...props }: Omit<InputHTMLAttributes<HTMLInputElement>, 'type'>) {
  return <input type="range" className={`ww-slider ${className}`} {...props} />
}
