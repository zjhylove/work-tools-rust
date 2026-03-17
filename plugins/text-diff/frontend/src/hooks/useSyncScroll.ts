import { useRef, useCallback } from 'react';

interface ScrollSyncOptions {
  enabled?: boolean;
  threshold?: number; // 像素阈值,避免微小滚动触发同步
}

/**
 * 同步滚动 Hook
 * 用于协调两个可滚动容器的滚动位置
 */
export function useSyncScroll(options: ScrollSyncOptions = {}) {
  const {
    enabled = true,
    threshold = 10
  } = options;

  const leftPaneRef = useRef<HTMLDivElement>(null);
  const rightPaneRef = useRef<HTMLDivElement>(null);
  const isScrollingRef = useRef(false); // 防止循环触发
  const lastScrollTopRef = useRef({ left: 0, right: 0 });

  // 左面板滚动事件
  const handleLeftScroll = useCallback((scrollTop: number) => {
    if (!enabled || !rightPaneRef.current || isScrollingRef.current) {
      return;
    }

    // 检查是否超过阈值
    const delta = Math.abs(scrollTop - lastScrollTopRef.current.left);
    if (delta < threshold) {
      return;
    }

    isScrollingRef.current = true;

    // 计算滚动比例
    const leftHeight = leftPaneRef.current?.scrollHeight || 1;
    const rightHeight = rightPaneRef.current.scrollHeight || 1;
    const ratio = rightHeight / leftHeight;

    // 同步右面板
    rightPaneRef.current.scrollTop = scrollTop * ratio;

    lastScrollTopRef.current.left = scrollTop;

    // 解除锁定
    setTimeout(() => {
      isScrollingRef.current = false;
    }, 50);
  }, [enabled, threshold]);

  // 右面板滚动事件
  const handleRightScroll = useCallback((scrollTop: number) => {
    if (!enabled || !leftPaneRef.current || isScrollingRef.current) {
      return;
    }

    // 检查是否超过阈值
    const delta = Math.abs(scrollTop - lastScrollTopRef.current.right);
    if (delta < threshold) {
      return;
    }

    isScrollingRef.current = true;

    // 计算滚动比例
    const rightHeight = rightPaneRef.current?.scrollHeight || 1;
    const leftHeight = leftPaneRef.current.scrollHeight || 1;
    const ratio = leftHeight / rightHeight;

    // 同步左面板
    leftPaneRef.current.scrollTop = scrollTop * ratio;

    lastScrollTopRef.current.right = scrollTop;

    // 解除锁定
    setTimeout(() => {
      isScrollingRef.current = false;
    }, 50);
  }, [enabled, threshold]);

  return {
    leftPaneRef,
    rightPaneRef,
    handleLeftScroll,
    handleRightScroll
  };
}
