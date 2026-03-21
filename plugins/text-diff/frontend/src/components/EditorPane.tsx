import { useRef, useMemo, useImperativeHandle, forwardRef, useEffect, useCallback } from 'react';
import type { ChangeEvent } from 'react';
import type { DiffLine } from '../hooks/useDiff';
import './EditorPane.css';

export interface EditorPaneProps {
  title: string;
  content: string;
  readOnly?: boolean;
  placeholder?: string;
  onChange?: (content: string) => void;
  onScroll?: (scrollTop: number) => void;
  className?: string;
  diffLines?: DiffLine[];  // 新增: 差异信息
}

export interface EditorPaneHandle {
  getScrollElement: () => HTMLDivElement | null;
}

export const EditorPane = forwardRef<EditorPaneHandle, EditorPaneProps>(function EditorPane({
  title,
  content,
  readOnly = false,
  placeholder = '请输入或粘贴文本...',
  onChange,
  onScroll,
  className = '',
  diffLines = []
}, ref) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const lineNumbersRef = useRef<HTMLDivElement>(null);
  const lineNumbersInnerRef = useRef<HTMLDivElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);

  // 暴露wrapper元素给父组件 (用于滚动同步)
  useImperativeHandle(ref, () => ({
    getScrollElement: () => wrapperRef.current
  }));

  // 渲染单行内容（支持字符级高亮）
  const renderLineContent = (line: typeof diffLines[number]) => {
    // 空行返回占位符
    if (!line.content) {
      return '\u00A0';
    }

    if (line.chars && line.chars.length > 0) {
      // 字符级高亮
      return (
        <span>
          {line.chars.map((char, index) => {
            let className = '';

            if (char.removed) {
              className = 'char-removed';
            } else if (char.added) {
              className = 'char-added';
            }

            return (
              <span key={index} className={className}>
                {char.value}
              </span>
            );
          })}
        </span>
      );
    }

    // 行级高亮或无高亮
    return line.content;
  };

  // 如果没有差异信息,使用原始内容
  const displayLines = useMemo(() => {
    if (diffLines.length === 0) {
      return content.split('\n').map((line, index) => ({
        content: line,
        type: 'equal' as const,
        lineNumber: index + 1
      }));
    }
    return diffLines;
  }, [content, diffLines]);

  // 计算行号区域宽度（根据行数自适应）
  const lineNumbersWidth = useMemo(() => {
    const lineCount = displayLines.length;
    // 默认2位数字宽度，超过99行时按实际数字位数计算
    const digits = lineCount > 99 ? String(lineCount).length : 2;
    // 每位数字约8px + 左padding 4px + 右padding 8px
    return digits * 8 + 12;
  }, [displayLines.length]);

  // 同步行号位置（只有行号需要transform，textarea和highlight-layer由wrapper自然滚动）
  const updateLineNumbersTransform = useCallback((scrollTop: number) => {
    if (lineNumbersInnerRef.current) {
      lineNumbersInnerRef.current.style.transform = `translateY(${-scrollTop}px)`;
    }
  }, []);

  // 从wrapper获取滚动位置并更新行号
  const syncScrollPosition = useCallback(() => {
    if (!wrapperRef.current) return;
    const { scrollTop } = wrapperRef.current;
    updateLineNumbersTransform(scrollTop);
  }, [updateLineNumbersTransform]);

  // 同步textarea和highlight-layer的尺寸
  const syncDimensions = useCallback(() => {
    if (!textareaRef.current || !highlightRef.current || !wrapperRef.current || !contentRef.current) return;

    // 保存当前滚动位置
    const savedScrollTop = wrapperRef.current.scrollTop;
    const savedScrollLeft = wrapperRef.current.scrollLeft;

    const textareaScrollHeight = textareaRef.current.scrollHeight;
    const scrollWidth = textareaRef.current.scrollWidth;
    const wrapperWidth = wrapperRef.current.clientWidth;

    // 设置内容容器、highlight-layer和textarea的高度
    const targetHeight = `${textareaScrollHeight}px`;
    const targetWidth = scrollWidth > wrapperWidth ? `${scrollWidth + 100}px` : '100%';

    contentRef.current.style.height = targetHeight;
    contentRef.current.style.width = targetWidth;

    highlightRef.current.style.height = targetHeight;
    highlightRef.current.style.width = targetWidth;

    textareaRef.current.style.height = targetHeight;
    textareaRef.current.style.width = targetWidth;

    // 同步行号容器的高度（与wrapper可视区域一致）
    if (lineNumbersRef.current) {
      lineNumbersRef.current.style.height = `${wrapperRef.current.clientHeight}px`;
    }

    // 恢复滚动位置
    wrapperRef.current.scrollTop = savedScrollTop;
    wrapperRef.current.scrollLeft = savedScrollLeft;

    // 同步滚动位置
    syncScrollPosition();
  }, [syncScrollPosition]);

  // 初始化时同步尺寸（仅一次）
  useEffect(() => {
    syncDimensions();
  }, []); // 空依赖，只在挂载时执行

  // 监听窗口大小变化，同步行号容器高度
  useEffect(() => {
    const handleResize = () => {
      if (lineNumbersRef.current && wrapperRef.current) {
        lineNumbersRef.current.style.height = `${wrapperRef.current.clientHeight}px`;
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // 内容变化时同步尺寸（处理文件导入等外部更新）
  useEffect(() => {
    // 使用 requestAnimationFrame 确保 DOM 已更新
    requestAnimationFrame(() => {
      syncDimensions();
    });
  }, [content, syncDimensions]);

  // 同步滚动：wrapper滚动时同步行号
  const handleWrapperScroll = useCallback(() => {
    if (!wrapperRef.current) return;

    const { scrollTop } = wrapperRef.current;

    // 只更新行号位置，textarea和highlight-layer由wrapper自然滚动
    updateLineNumbersTransform(scrollTop);

    onScroll?.(scrollTop);
  }, [onScroll, updateLineNumbersTransform]);

  // 内容变化时同步尺寸并自动滚动到光标位置
  const handleChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
    const newContent = e.target.value;
    onChange?.(newContent);

    // 在下一帧同步尺寸并滚动到光标位置
    requestAnimationFrame(() => {
      syncDimensions();

      // 计算光标位置并滚动wrapper使光标可见
      if (textareaRef.current && wrapperRef.current && lineNumbersInnerRef.current) {
        const { selectionStart } = textareaRef.current;
        const textBeforeCursor = newContent.substring(0, selectionStart);
        const lines = textBeforeCursor.split('\n');
        const cursorLine = lines.length;
        const lineHeight = 22;
        const padding = 8;
        const cursorTop = (cursorLine - 1) * lineHeight + padding;
        const cursorBottom = cursorTop + lineHeight;

        const wrapperHeight = wrapperRef.current.clientHeight;
        const currentScrollTop = wrapperRef.current.scrollTop;

        // 如果光标在可视区域下方，滚动使光标可见
        if (cursorBottom > currentScrollTop + wrapperHeight) {
          const newScrollTop = cursorBottom - wrapperHeight + padding;

          wrapperRef.current.scrollTop = newScrollTop;

          // 更新行号位置
          updateLineNumbersTransform(newScrollTop);
        }
      }
    });
  };

  // 监听粘贴事件，确保内容正确显示
  const handlePaste = () => {
    // 粘贴后同步尺寸
    setTimeout(syncDimensions, 50);
  };

  return (
    <div ref={containerRef} className={`editor-pane ${className}`}>
      <div className="pane-header">{title}</div>

      <div className="pane-content">
        <div className="editor-container">
          {/* 行号列 */}
          <div ref={lineNumbersRef} className="line-numbers" style={{ width: `${lineNumbersWidth}px` }}>
            <div ref={lineNumbersInnerRef} className="line-numbers-inner">
              {displayLines.map((line, index) => (
                <div key={index} className="line-number">
                  {line.lineNumber > 0 ? line.lineNumber : ''}
                </div>
              ))}
            </div>
          </div>

          {/* 编辑区域 - wrapper是滚动容器 */}
          <div ref={wrapperRef} className="editor-wrapper" onScroll={handleWrapperScroll}>
            {/* 内容容器 - 包裹textarea和highlight-layer，使它们跟随滚动 */}
            <div ref={contentRef} className="editor-content">
              {/* 高亮层 - 显示差异高亮 */}
              <div
                ref={highlightRef}
                className="highlight-layer"
                aria-hidden="true"
              >
                {displayLines.map((line, index) => (
                  <div
                    key={index}
                    data-line-index={index}
                    className={`highlight-line highlight-line-${line.type}`}
                  >
                    {renderLineContent(line)}
                  </div>
                ))}
              </div>

              {/* 文本输入层 */}
              <textarea
                ref={textareaRef}
                className="text-input"
                value={content}
                onChange={handleChange}
                onPaste={handlePaste}
                readOnly={readOnly}
                placeholder={placeholder}
                spellCheck={false}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});
