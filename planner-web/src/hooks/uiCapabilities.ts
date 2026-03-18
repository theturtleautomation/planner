import type { UiCapabilities, ViewportClass } from '../types.ts';

const MOBILE_BREAKPOINT = 768;
const TABLET_BREAKPOINT = 1200;

export function detectViewportClass(width: number): ViewportClass {
  if (width < MOBILE_BREAKPOINT) return 'mobile';
  if (width < TABLET_BREAKPOINT) return 'tablet';
  return 'desktop';
}

export function estimateMaxVisibleItems(width: number, viewportClass: ViewportClass): number {
  if (viewportClass === 'mobile') {
    return 1;
  }

  if (viewportClass === 'tablet') {
    return Math.max(1, Math.min(2, Math.floor((width - 80) / 320)));
  }

  return Math.max(2, Math.min(5, Math.floor((width - 120) / 320)));
}

export function buildUiCapabilitiesForWidth(width: number): UiCapabilities {
  const viewportClass = detectViewportClass(width);

  return {
    viewport_class: viewportClass,
    max_visible_items: estimateMaxVisibleItems(width, viewportClass),
    supports_split_draft_view: viewportClass !== 'mobile',
  };
}

export function buildUiCapabilities(): UiCapabilities {
  return buildUiCapabilitiesForWidth(window.innerWidth);
}

export function sameUiCapabilities(a: UiCapabilities | null, b: UiCapabilities): boolean {
  if (!a) return false;

  return a.viewport_class === b.viewport_class
    && a.max_visible_items === b.max_visible_items
    && a.supports_split_draft_view === b.supports_split_draft_view;
}
