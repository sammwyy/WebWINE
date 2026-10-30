import type { CaptionGlyphKind } from '@/shared/lib/theme-registry'

/** Bold white Aero caption glyphs (the dark outline comes from aero.css). */
export function AeroCaptionGlyph({ kind }: { kind: CaptionGlyphKind }) {
  switch (kind) {
    case 'minimize':
      return (
        <svg width="11" height="9" viewBox="0 0 11 9">
          <rect x="1" y="5.5" width="9" height="2.5" fill="currentColor" />
        </svg>
      )
    case 'maximize':
      return (
        <svg width="11" height="9" viewBox="0 0 11 9">
          <rect x="1.5" y="1.5" width="8" height="6.5" fill="none" stroke="currentColor" strokeWidth="1.2" />
          <rect x="1" y="1" width="9" height="2.4" fill="currentColor" />
        </svg>
      )
    case 'restore':
      return (
        <svg width="11" height="10" viewBox="0 0 11 10">
          <path d="M3.5 0.5h7v5.5h-1.5" fill="none" stroke="currentColor" strokeWidth="1" />
          <rect x="3" y="0" width="8" height="2" fill="currentColor" />
          <rect x="0.5" y="3.5" width="7" height="6" fill="none" stroke="currentColor" strokeWidth="1" />
          <rect x="0" y="3" width="8" height="2" fill="currentColor" />
        </svg>
      )
    case 'close':
      return (
        <svg width="12" height="10" viewBox="0 0 12 10">
          <path d="M2 1.2 10 8.8M10 1.2 2 8.8" stroke="currentColor" strokeWidth="2.2" />
        </svg>
      )
  }
}
