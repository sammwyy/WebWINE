import { useId } from 'react'

interface OrbPalette {
  /** sphere body: centre → middle → edge */
  body: [string, string, string]
  /** light coming from the bottom rim */
  rim: [string, string]
  /** extra glow shown on hover */
  hover: string
}

const PALETTES = {
  vista: { body: ['#21405f', '#0b1828', '#010307'], rim: ['#6fd2ff', '#1f7ee0'], hover: '#9be8ff' },
  win7: { body: ['#3d8fe0', '#0d4596', '#03183d'], rim: ['#9af3ff', '#2aa2f0'], hover: '#c2f6ff' },
} satisfies Record<string, OrbPalette>

/** Glossy glass sphere with the waving four-colour flag (Aero start orb). */
function AeroOrb({ palette }: { palette: OrbPalette }) {
  const id = useId().replace(/[^a-zA-Z0-9_-]/g, '')
  const g = (name: string) => `${id}-${name}`

  return (
    <svg className="aero-orb" viewBox="0 0 54 54" aria-hidden="true">
      <defs>
        <radialGradient id={g('body')} cx="50%" cy="42%" r="58%">
          <stop offset="0" stopColor={palette.body[0]} />
          <stop offset="0.55" stopColor={palette.body[1]} />
          <stop offset="1" stopColor={palette.body[2]} />
        </radialGradient>
        <radialGradient id={g('rim')} cx="50%" cy="100%" r="62%">
          <stop offset="0" stopColor={palette.rim[0]} stopOpacity="0.95" />
          <stop offset="0.45" stopColor={palette.rim[1]} stopOpacity="0.45" />
          <stop offset="1" stopColor={palette.rim[1]} stopOpacity="0" />
        </radialGradient>
        <radialGradient id={g('hover')} cx="50%" cy="62%" r="55%">
          <stop offset="0" stopColor={palette.hover} stopOpacity="0.85" />
          <stop offset="0.6" stopColor="#2b93ff" stopOpacity="0.35" />
          <stop offset="1" stopColor="#2b93ff" stopOpacity="0" />
        </radialGradient>
        <linearGradient id={g('gloss')} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#fff" stopOpacity="0.75" />
          <stop offset="1" stopColor="#fff" stopOpacity="0.06" />
        </linearGradient>
        <linearGradient id={g('red')} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ff9a6a" />
          <stop offset="1" stopColor="#e2380c" />
        </linearGradient>
        <linearGradient id={g('green')} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#c4ec5c" />
          <stop offset="1" stopColor="#4f9c00" />
        </linearGradient>
        <linearGradient id={g('blue')} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#7ad6ff" />
          <stop offset="1" stopColor="#0773d0" />
        </linearGradient>
        <linearGradient id={g('yellow')} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ffe97a" />
          <stop offset="1" stopColor="#f2a100" />
        </linearGradient>
      </defs>

      <circle cx="27" cy="27" r="25" fill={`url(#${g('body')})`} />
      <circle cx="27" cy="27" r="25" fill={`url(#${g('rim')})`} />
      <circle className="aero-orb-hover" cx="27" cy="27" r="25" fill={`url(#${g('hover')})`} />

      <g className="aero-orb-flag" transform="translate(15.4 15.8) scale(1.15)">
        <path d="M1.8 1.4C4.8 0 7.8 0.2 10.4 1.4L9.6 8.6C7 7.4 4 7.3 1 8.6Z" fill={`url(#${g('red')})`} />
        <path d="M11.6 1.8C14.3 3 17 3 20.2 1.4L19.4 8.6C16.4 10 13.6 10 10.8 9Z" fill={`url(#${g('green')})`} />
        <path d="M0.8 9.8C3.9 8.5 6.8 8.6 9.4 9.8L8.6 17C6 15.8 3.1 15.7 0 17Z" fill={`url(#${g('blue')})`} />
        <path d="M10.6 10.2C13.4 11.2 16.2 11.2 19.2 9.8L18.4 17C15.4 18.4 12.6 18.4 9.8 17.4Z" fill={`url(#${g('yellow')})`} />
      </g>

      <ellipse cx="27" cy="16.5" rx="19" ry="12" fill={`url(#${g('gloss')})`} />
      <circle cx="27" cy="27" r="24" fill="none" stroke="#fff" strokeOpacity="0.22" />
      <circle cx="27" cy="27" r="25.5" fill="none" stroke="#000" strokeOpacity="0.85" />
    </svg>
  )
}

/** Black glass orb (Windows Vista). */
export function VistaStartOrb() {
  return <AeroOrb palette={PALETTES.vista} />
}

/** Blue glass orb (Windows 7). */
export function Win7StartOrb() {
  return <AeroOrb palette={PALETTES.win7} />
}
