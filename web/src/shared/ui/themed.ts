import type { ComponentType } from 'react'
import { useCurrentTheme } from '@/shared/lib/theme-registry'

/**
 * Resolve a themeable UI component: returns the active theme's
 * `components.ui[name]` override if any, otherwise `Default`.
 *
 *   export function NavButtons(props: NavButtonsProps) {
 *     return createElement(useThemed('NavButtons', DefaultNavButtons), props)
 *   }
 */
export function useThemed<P extends object>(name: string, Default: ComponentType<P>): ComponentType<P> {
  return (useCurrentTheme()?.components?.ui?.[name] as ComponentType<P> | undefined) ?? Default
}
