import { useState, useCallback, useMemo } from 'react';
import type { DiffLine } from './useDiff';

/**
 * 差异导航 Hook
 * 提供跳转到上一个/下一个差异的功能
 */
export function useDiffNavigation(
  originalLines: DiffLine[],
  modifiedLines: DiffLine[]
) {
  const [currentIndex, setCurrentIndex] = useState<number>(-1);

  // 找出所有差异行的索引
  const diffIndices = useMemo(() => {
    const indices: number[] = [];

    // 使用较长的数组来确定索引
    const maxLength = Math.max(originalLines.length, modifiedLines.length);

    for (let i = 0; i < maxLength; i++) {
      const origLine = originalLines[i];
      const modLine = modifiedLines[i];

      // 如果任一面板的行是差异行,则记录索引
      if (origLine?.type !== 'equal' || modLine?.type !== 'equal') {
        indices.push(i);
      }
    }

    return indices;
  }, [originalLines, modifiedLines]);

  // 跳转到下一个差异
  const goToNext = useCallback(() => {
    if (diffIndices.length === 0) {
      return null;
    }

    const nextIndex = currentIndex < diffIndices.length - 1
      ? currentIndex + 1
      : 0; // 循环到第一个

    setCurrentIndex(nextIndex);
    return diffIndices[nextIndex];
  }, [currentIndex, diffIndices]);

  // 跳转到上一个差异
  const goToPrevious = useCallback(() => {
    if (diffIndices.length === 0) {
      return null;
    }

    const prevIndex = currentIndex > 0
      ? currentIndex - 1
      : diffIndices.length - 1; // 循环到最后一个

    setCurrentIndex(prevIndex);
    return diffIndices[prevIndex];
  }, [currentIndex, diffIndices]);

  // 滚动到指定行
  const scrollToLine = useCallback((lineIndex: number) => {
    const element = document.querySelector(
      `[data-line-index="${lineIndex}"]`
    ) as HTMLElement;

    if (element) {
      element.scrollIntoView({
        behavior: 'smooth',
        block: 'center'
      });

      // 添加临时高亮效果
      element.classList.add('current-diff');
      setTimeout(() => {
        element.classList.remove('current-diff');
      }, 2000);
    }
  }, []);

  // 跳转到下一个差异并滚动
  const goToNextAndScroll = useCallback(() => {
    const lineIndex = goToNext();
    if (lineIndex !== null) {
      scrollToLine(lineIndex);
    }
    return lineIndex;
  }, [goToNext, scrollToLine]);

  // 跳转到上一个差异并滚动
  const goToPreviousAndScroll = useCallback(() => {
    const lineIndex = goToPrevious();
    if (lineIndex !== null) {
      scrollToLine(lineIndex);
    }
    return lineIndex;
  }, [goToPrevious, scrollToLine]);

  return {
    currentIndex,
    totalDiffs: diffIndices.length,
    goToNext,
    goToPrevious,
    goToNextAndScroll,
    goToPreviousAndScroll,
    scrollToLine,
    hasDiffs: diffIndices.length > 0
  };
}
