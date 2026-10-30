import type { CaptionGlyphKind } from '@/shared/lib/theme-registry'

/** Chunky white Luna caption glyphs. */
export function XpCaptionGlyph({ kind }: { kind: CaptionGlyphKind }) {
  switch (kind) {
    case 'minimize':
      return (
        <svg width="13" height="13" viewBox="0 0 13 13">
          <rect x="2" y="9" width="6" height="3" fill="currentColor" />
        </svg>
      )
    case 'maximize':
      return (
        <svg width="13" height="13" viewBox="0 0 13 13">
          <rect x="1.5" y="1.5" width="10" height="10" fill="none" stroke="currentColor" />
          <rect x="1" y="1" width="11" height="3" fill="currentColor" />
        </svg>
      )
    case 'restore':
      return (
        <svg width="13" height="13" viewBox="0 0 13 13">
          <path d="M4.5 1.5h7v7h-2" fill="none" stroke="currentColor" />
          <rect x="4" y="1" width="8" height="2.5" fill="currentColor" />
          <rect x="1.5" y="4.5" width="7" height="7" fill="none" stroke="currentColor" />
          <rect x="1" y="4" width="8" height="2.5" fill="currentColor" />
        </svg>
      )
    case 'close':
      return (
        <svg width="13" height="13" viewBox="0 0 13 13">
          <path d="M2.5 2.5 10.5 10.5M10.5 2.5 2.5 10.5" stroke="currentColor" strokeWidth="2.2" />
        </svg>
      )
  }
}

/** Green "start" pill: waving flag + italic label. */
export function XpStartButton() {
  return (
    <span className="xp-start">
      <svg className="xp-start-flag" viewBox="0 0 21 19" aria-hidden="true">
        <path d="M1.8 1.4C4.8 0 7.8 0.2 10.4 1.4L9.6 8.6C7 7.4 4 7.3 1 8.6Z" fill="#f65314" />
        <path d="M11.6 1.8C14.3 3 17 3 20.2 1.4L19.4 8.6C16.4 10 13.6 10 10.8 9Z" fill="#7cbb00" />
        <path d="M0.8 9.8C3.9 8.5 6.8 8.6 9.4 9.8L8.6 17C6 15.8 3.1 15.7 0 17Z" fill="#00a1f1" />
        <path d="M10.6 10.2C13.4 11.2 16.2 11.2 19.2 9.8L18.4 17C15.4 18.4 12.6 18.4 9.8 17.4Z" fill="#ffbb00" />
      </svg>
      <span>start</span>
    </span>
  )
}
