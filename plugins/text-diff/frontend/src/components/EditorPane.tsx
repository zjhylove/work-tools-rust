import { useRef, useEffect } from 'react';
import type { ChangeEvent } from 'react';
import './EditorPane.css';

export interface EditorPaneProps {
  title: string;
  content: string;
  readOnly?: boolean;
  placeholder?: string;
  onChange?: (content: string) => void;
  onScroll?: (scrollTop: number) => void;
  className?: string;
}

export function EditorPane({
  title,
  content,
  readOnly = false,
  placeholder = '请输入或粘贴文本...',
  onChange,
  onScroll,
  className = ''
}: EditorPaneProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);

  // 同步滚动
  const handleScroll = () => {
    if (!textareaRef.current) return;
    const scrollTop = textareaRef.current.scrollTop;
    if (highlightRef.current) {
      highlightRef.current.scrollTop = scrollTop;
    }
    onScroll?.(scrollTop);
  };

  // 内容变化
  const handleChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
    const newContent = e.target.value;
    onChange?.(newContent);
  };

  // 自动调整高度
  useEffect(() => {
    if (!textareaRef.current) return;

    const textarea = textareaRef.current;
    const adjustHeight = () => {
      textarea.style.height = 'auto';
      textarea.style.height = `${Math.max(textarea.scrollHeight, 400)}px`;
    };

    adjustHeight();
  }, [content]);

  return (
    <div className={`editor-pane ${className}`}>
      <div className="pane-header">{title}</div>

      <div className="pane-content">
        <div className="editor-container">
          {/* 行号列 */}
          <div className="line-numbers">
            {content.split('\n').map((_, index) => (
              <div key={index} className="line-number">
                {index + 1}
              </div>
            ))}
          </div>

          {/* 编辑区域 */}
          <div className="editor-wrapper">
            {/* 高亮层 (暂时为空,后续添加差异高亮) */}
            <div
              ref={highlightRef}
              className="highlight-layer"
              aria-hidden="true"
            >
              {content.split('\n').map((line, index) => (
                <div key={index} className="highlight-line">
                  {line || '\u00A0'}
                </div>
              ))}
            </div>

            {/* 文本输入层 */}
            <textarea
              ref={textareaRef}
              className="text-input"
              value={content}
              onChange={handleChange}
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
}
