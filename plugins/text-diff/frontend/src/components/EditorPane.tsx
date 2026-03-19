import { useRef, useMemo, useImperativeHandle, forwardRef, useEffect } from 'react';
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

  // 暴露容器元素给父组件 (用于滚动同步)
  useImperativeHandle(ref, () => ({
    getScrollElement: () => containerRef.current
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

  // 同步两个层的高度和宽度
  useEffect(() => {
    const syncDimensions = () => {
      if (!textareaRef.current || !highlightRef.current) return;

      // 同步scrollHeight
      const scrollHeight = textareaRef.current.scrollHeight;
      highlightRef.current.style.height = `${scrollHeight}px`;
      highlightRef.current.style.minHeight = `${scrollHeight}px`;

      // 同步scrollWidth（确保横向足够宽）
      const scrollWidth = textareaRef.current.scrollWidth;
      highlightRef.current.style.width = `${scrollWidth}px`;
    };

    // 立即同步
    syncDimensions();

    // 下一个事件循环同步
    setTimeout(syncDimensions, 0);

    // 渲染帧前同步
    requestAnimationFrame(syncDimensions);

    // 延迟同步（确保DOM完全更新）
    setTimeout(syncDimensions, 50);
  }, [displayLines, content]);

  // 同步滚动（包括横向滚动）
  const handleScroll = () => {
    if (!textareaRef.current) return;
    const scrollTop = textareaRef.current.scrollTop;
    const scrollLeft = textareaRef.current.scrollLeft;

    // 同步highlight-layer的滚动位置
    if (highlightRef.current) {
      highlightRef.current.scrollTop = scrollTop;
      highlightRef.current.scrollLeft = scrollLeft;
    }

    onScroll?.(scrollTop);
  };

  // 内容变化
  const handleChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
    const newContent = e.target.value;
    onChange?.(newContent);

    // 立即同步尺寸（最重要！）
    if (textareaRef.current && highlightRef.current) {
      const scrollHeight = textareaRef.current.scrollHeight;
      const scrollWidth = textareaRef.current.scrollWidth;
      highlightRef.current.style.height = `${scrollHeight}px`;
      highlightRef.current.style.minHeight = `${scrollHeight}px`;
      highlightRef.current.style.width = `${scrollWidth}px`;
    }
  };

  // 监听粘贴事件，确保内容正确显示
  const handlePaste = () => {
    setTimeout(() => {
      // 粘贴后强制更新显示
      if (textareaRef.current && highlightRef.current) {
        // 同步滚动位置到顶部
        textareaRef.current.scrollTop = 0;
        highlightRef.current.scrollTop = 0;
      }
    }, 10);
  };

  return (
    <div ref={containerRef} className={`editor-pane ${className}`}>
      <div className="pane-header">{title}</div>

      <div className="pane-content">
        <div className="editor-container">
          {/* 行号列 */}
          <div className="line-numbers">
            {displayLines.map((line, index) => (
              <div key={index} className="line-number">
                {line.lineNumber > 0 ? line.lineNumber : ''}
              </div>
            ))}
          </div>

          {/* 编辑区域 */}
          <div className="editor-wrapper">
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
              onScroll={handleScroll}
              readOnly={readOnly}
              placeholder={placeholder}
              spellCheck={false}
            />
          </div>
        </div>
      </div>
    </div>
  );
});
