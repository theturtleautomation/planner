import { useEffect, useMemo } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { RefObject } from 'react';
import SocraticDocumentSection from './SocraticDocumentSection.tsx';
import type { SocraticDocumentCategoryView } from '../stores/socraticDocumentStore.ts';
import type { PromptAnswer, PromptEnvelope, SocraticWorkspaceGroup } from '../types.ts';

interface VirtualizedCategoryDocumentProps {
  scrollElementRef: RefObject<HTMLDivElement | null>;
  categories: SocraticDocumentCategoryView[];
  currentPrompt: PromptEnvelope | null;
  pendingCategoryId: string | null;
  branchNotice: string | null;
  focusedCategoryId: string | null;
  groupMap: Map<string, SocraticWorkspaceGroup>;
  jumpTargetCategoryId: string | null;
  focusTargetCategoryId: string | null;
  disabled?: boolean;
  onVisibleCategoryChange?: (categoryId: string | null) => void;
  onSubmitAnswers: (answers: PromptAnswer[]) => void;
  onDone?: () => void;
  onShowAll: () => void;
  onFocusTargetHandled?: (categoryId: string) => void;
  onAnswerFocus?: (categoryId: string) => void;
}

function categorySelector(categoryId: string): string {
  return `[data-category-id="${categoryId.replaceAll('"', '\\"')}"]`;
}

function scrollStaticCategoryIntoView(
  scrollElement: HTMLDivElement | null,
  categoryId: string,
): void {
  if (!scrollElement) return;
  const section = scrollElement.querySelector<HTMLElement>(categorySelector(categoryId));
  section?.scrollIntoView({ block: 'start' });
}

function focusFirstAnswerableInCategory(
  scrollElement: HTMLDivElement | null,
  categoryId: string,
): boolean {
  if (!scrollElement) return false;
  const section = scrollElement.querySelector<HTMLElement>(categorySelector(categoryId));
  if (!section) return false;

  const answerable = section.querySelector<HTMLElement>(
    'textarea:not([disabled]), button[role="radio"]:not([disabled]), input:not([disabled])',
  );
  if (!answerable) return false;

  answerable.focus();
  return document.activeElement === answerable;
}

export default function VirtualizedCategoryDocument({
  scrollElementRef,
  categories,
  currentPrompt,
  pendingCategoryId,
  branchNotice,
  focusedCategoryId,
  groupMap,
  jumpTargetCategoryId,
  focusTargetCategoryId,
  disabled = false,
  onVisibleCategoryChange,
  onSubmitAnswers,
  onDone,
  onShowAll,
  onFocusTargetHandled,
  onAnswerFocus,
}: VirtualizedCategoryDocumentProps) {
  const categoryIndexMap = useMemo(
    () => new Map(categories.map((category, index) => [category.categoryId, index])),
    [categories],
  );

  const virtualizer = useVirtualizer({
    count: categories.length,
    getScrollElement: () => scrollElementRef.current,
    estimateSize: () => 360,
    overscan: 3,
    initialRect: { width: 1024, height: 900 },
  });

  const virtualItems = virtualizer.getVirtualItems();
  const useStaticLayout = !scrollElementRef.current || virtualItems.length === 0;

  useEffect(() => {
    const targetCategoryId = focusTargetCategoryId ?? jumpTargetCategoryId ?? focusedCategoryId;
    if (!targetCategoryId) return;
    if (useStaticLayout) {
      scrollStaticCategoryIntoView(scrollElementRef.current, targetCategoryId);
      return;
    }
    const targetIndex = categoryIndexMap.get(targetCategoryId);
    if (targetIndex === undefined) return;
    virtualizer.scrollToIndex(targetIndex, { align: 'start' });
  }, [
    categoryIndexMap,
    focusTargetCategoryId,
    focusedCategoryId,
    jumpTargetCategoryId,
    scrollElementRef,
    useStaticLayout,
    virtualizer,
  ]);

  useEffect(() => {
    if (!focusTargetCategoryId) return;
    if (!focusFirstAnswerableInCategory(scrollElementRef.current, focusTargetCategoryId)) return;
    onFocusTargetHandled?.(focusTargetCategoryId);
  }, [categories, currentPrompt, focusTargetCategoryId, onFocusTargetHandled, scrollElementRef, useStaticLayout]);

  useEffect(() => {
    if (!onVisibleCategoryChange) return;
    if (useStaticLayout) {
      onVisibleCategoryChange(focusedCategoryId ?? categories[0]?.categoryId ?? null);
      return;
    }
    const firstVirtualItem = virtualItems[0];
    if (!firstVirtualItem) {
      onVisibleCategoryChange(null);
      return;
    }
    const category = categories[firstVirtualItem.index];
    onVisibleCategoryChange(category?.categoryId ?? null);
  }, [categories, focusedCategoryId, onVisibleCategoryChange, useStaticLayout, virtualItems]);

  if (useStaticLayout) {
    return (
      <div className="socratic-document-virtualizer" aria-label="Socratic document desk">
        <div className="socratic-document-virtualizer__spacer">
          {categories.map((category) => (
            <div key={category.categoryId} className="socratic-document-virtual-item is-static">
              <SocraticDocumentSection
                category={category}
                currentPrompt={currentPrompt}
                pendingCategoryId={pendingCategoryId}
                branchNotice={branchNotice && category.categoryId === focusedCategoryId ? branchNotice : null}
                group={groupMap.get(category.categoryId) ?? null}
                disabled={disabled}
                onSubmitAnswers={onSubmitAnswers}
                onDone={onDone}
                onShowAll={onShowAll}
                onAnswerFocus={onAnswerFocus}
              />
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="socratic-document-virtualizer" aria-label="Socratic document desk">
      <div
        className="socratic-document-virtualizer__spacer"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualItems.map((virtualItem) => {
          const category = categories[virtualItem.index];
          if (!category) return null;

          return (
            <div
              key={category.categoryId}
              ref={virtualizer.measureElement}
              data-index={virtualItem.index}
              className="socratic-document-virtual-item"
              style={{ transform: `translateY(${virtualItem.start}px)` }}
            >
              <SocraticDocumentSection
                category={category}
                currentPrompt={currentPrompt}
                pendingCategoryId={pendingCategoryId}
                branchNotice={branchNotice && category.categoryId === focusedCategoryId ? branchNotice : null}
                group={groupMap.get(category.categoryId) ?? null}
                disabled={disabled}
                onSubmitAnswers={onSubmitAnswers}
                onDone={onDone}
                onShowAll={onShowAll}
                onAnswerFocus={onAnswerFocus}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
