import { describe, expect, it } from 'vitest';
import {
  buildUiCapabilitiesForWidth,
  detectViewportClass,
  estimateMaxVisibleItems,
} from '../uiCapabilities.ts';

describe('uiCapabilities', () => {
  it('derives viewport classes from width breakpoints', () => {
    expect(detectViewportClass(480)).toBe('mobile');
    expect(detectViewportClass(900)).toBe('tablet');
    expect(detectViewportClass(1440)).toBe('desktop');
  });

  it('computes max_visible_items from viewport capacity', () => {
    expect(estimateMaxVisibleItems(600, 'mobile')).toBe(1);
    expect(estimateMaxVisibleItems(900, 'tablet')).toBe(2);
    expect(estimateMaxVisibleItems(1280, 'desktop')).toBe(3);
    expect(estimateMaxVisibleItems(1760, 'desktop')).toBe(5);
  });

  it('changes requested batch size when viewport capacity changes', () => {
    const mobile = buildUiCapabilitiesForWidth(540);
    const desktop = buildUiCapabilitiesForWidth(1400);

    expect(mobile.viewport_class).toBe('mobile');
    expect(desktop.viewport_class).toBe('desktop');
    expect(desktop.max_visible_items).toBeGreaterThan(mobile.max_visible_items);
  });
});
