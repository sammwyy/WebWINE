import type { SVGProps } from 'react'

export { SystemIcon } from './SystemIcon'
export type { SystemIconName } from './SystemIcon'

type IconProps = SVGProps<SVGSVGElement>

/* monochrome glyphs (use currentColor) */

export function ChevronRightGlyph(props: IconProps) {
  return (
    <svg viewBox="0 0 8 8" width="8" height="8" {...props}>
      <path d="M2 0.5 5.5 4 2 7.5z" fill="currentColor" />
    </svg>
  )
}

export function ChevronDownGlyph(props: IconProps) {
  return (
    <svg viewBox="0 0 8 8" width="8" height="8" {...props}>
      <path d="M0.5 2 4 5.5 7.5 2z" fill="currentColor" />
    </svg>
  )
}

export function ChevronUpGlyph(props: IconProps) {
  return (
    <svg viewBox="0 0 10 10" width="10" height="10" {...props}>
      <path d="M1.5 6.5 5 3l3.5 3.5" fill="none" stroke="currentColor" strokeWidth="1.4" />
    </svg>
  )
}

export function ArrowGlyph({ direction = 'left', ...props }: IconProps & { direction?: 'left' | 'right' }) {
  return (
    <svg viewBox="0 0 14 14" {...props}>
      <path
        d="M7.5 2 2.5 7l5 5M3 7h9"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
        strokeLinejoin="round"
        transform={direction === 'right' ? 'matrix(-1 0 0 1 14 0)' : undefined}
      />
    </svg>
  )
}

export function SearchGlyph(props: IconProps) {
  return (
    <svg viewBox="0 0 14 14" {...props}>
      <circle cx="5.5" cy="5.5" r="4" fill="rgba(255,255,255,0.6)" stroke="currentColor" strokeWidth="1.5" />
      <path d="M8.5 8.5 13 13" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
    </svg>
  )
}
