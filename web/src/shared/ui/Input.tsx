import { createElement } from 'react'
import type { ComponentProps, InputHTMLAttributes } from 'react'
import { SearchGlyph } from '../icons'
import { useThemed } from './themed'

export function Input({ className = '', ...props }: ComponentProps<'input'>) {
  return <input className={`ww-input ${className}`} {...props} />
}

export function TextArea({ className = '', ...props }: ComponentProps<'textarea'>) {
  return <textarea className={`ww-input ${className}`} {...props} />
}

export type SearchBoxProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'type'>

/** Search box with a cue banner and a magnifier glyph. */
function DefaultSearchBox({ className = '', placeholder = 'Search', ...props }: SearchBoxProps) {
  return (
    <div className={`ww-search ${className}`}>
      <input type="search" className="ww-input" placeholder={placeholder} {...props} />
      <SearchGlyph className="ww-search-icon" />
    </div>
  )
}

/** Themeable: themes can replace it via `components.ui.SearchBox`. */
export function SearchBox(props: SearchBoxProps) {
  return createElement(useThemed('SearchBox', DefaultSearchBox), props)
}
