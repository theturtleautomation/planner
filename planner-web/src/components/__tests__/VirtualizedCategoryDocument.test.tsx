import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { SocraticDocumentCategoryView } from '../../stores/socraticDocumentStore.ts';

const scrollToIndex = vi.fn();
const measureElement = vi.fn();
let visibleIndices: number[] = [0];

vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: ({ count }: { count: number }) => ({
    getVirtualItems: () => visibleIndices
      .filter((index) => index < count)
      .map((index) => ({ index, start: index * 320 })),
    getTotalSize: () => count * 320,
    measureElement,
    scrollToIndex,
  }),
}));

vi.mock('../SocraticDocumentSection.tsx', () => ({
  default: ({ category }: { category: SocraticDocumentCategoryView }) => (
    <div data-testid={`section-${category.categoryId}`}>{category.title}</div>
  ),
}));

import VirtualizedCategoryDocument from '../VirtualizedCategoryDocument.tsx';

function makeCategory(categoryId: string, title: string): SocraticDocumentCategoryView {
  return {
    categoryId,
    parentCategoryId: null,
    title,
    summary: `${title} summary`,
    status: 'ready',
    depth: 0,
    mappedDimensions: [title],
    hasChildren: false,
    hasPromptReady: true,
    itemCountHint: 1,
    isNewlyAvailable: false,
    questionIds: [],
    latestPromptId: null,
    latestPromptTitle: null,
    latestPromptInstructions: null,
    answeredCount: 0,
    totalCount: 1,
  };
}

describe('VirtualizedCategoryDocument', () => {
  beforeEach(() => {
    scrollToIndex.mockReset();
    measureElement.mockReset();
    visibleIndices = [0];
  });

  it('retargets jump-to-section anchoring after a live category insertion', () => {
    const scrollElement = document.createElement('div');
    const scrollElementRef = { current: scrollElement };
    const onVisibleCategoryChange = vi.fn();
    const baseProps = {
      scrollElementRef,
      currentPrompt: null,
      pendingCategoryId: null,
      branchNotice: null,
      focusedCategoryId: 'category-c',
      groupMap: new Map(),
      jumpTargetCategoryId: 'category-c',
      focusTargetCategoryId: null,
      disabled: false,
      onVisibleCategoryChange,
      onSubmitAnswers: vi.fn(),
      onDone: undefined,
      onShowAll: vi.fn(),
      onFocusTargetHandled: vi.fn(),
      onAnswerFocus: vi.fn(),
    };

    visibleIndices = [2];
    const { rerender } = render(
      <VirtualizedCategoryDocument
        {...baseProps}
        categories={[
          makeCategory('category-a', 'A'),
          makeCategory('category-b', 'B'),
          makeCategory('category-c', 'C'),
        ]}
      />,
    );

    expect(scrollToIndex).toHaveBeenLastCalledWith(2, { align: 'start' });
    expect(onVisibleCategoryChange).toHaveBeenLastCalledWith('category-c');

    scrollToIndex.mockClear();
    onVisibleCategoryChange.mockClear();
    visibleIndices = [3];

    rerender(
      <VirtualizedCategoryDocument
        {...baseProps}
        categories={[
          makeCategory('category-new', 'New'),
          makeCategory('category-a', 'A'),
          makeCategory('category-b', 'B'),
          makeCategory('category-c', 'C'),
        ]}
      />,
    );

    expect(scrollToIndex).toHaveBeenLastCalledWith(3, { align: 'start' });
    expect(onVisibleCategoryChange).toHaveBeenLastCalledWith('category-c');
  });
});
