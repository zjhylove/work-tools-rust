import { useState } from 'react';
import './FilePickerButton.css';

export interface FilePickerButtonProps {
  label: string;
  fileName?: string;
  onFileSelected: (content: string, fileName: string) => void;
  disabled?: boolean;
  className?: string;
}

export function FilePickerButton({
  label,
  fileName,
  onFileSelected,
  disabled = false,
  className = ''
}: FilePickerButtonProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleClick = async () => {
    setError(null);
    setIsLoading(true);

    try {
      // 显示文件选择对话框
      // 注意: 在 iframe 环境中,我们需要使用 Tauri 的 IPC
      // 但目前先使用简单的文件路径输入,之后可以升级
      const input = prompt(
        `请输入${label}文件路径:\n\n` +
        `提示: 可以拖放文件到下方输入框\n` +
        `例如: /tmp/test.txt`,
        fileName || ''
      );

      if (!input || input.trim() === '') {
        setIsLoading(false);
        return;
      }

      const filePath = input.trim();

      // 调用后端加载文件
      const result = await window.pluginAPI.call('text-diff', 'load_text_file', {
        file_path: filePath
      });

      if (result.error) {
        throw new Error(result.error);
      }

      onFileSelected(result.content, filePath);
    } catch (err: any) {
      const errorMsg = err.message || '加载文件失败';
      setError(errorMsg);
      console.error(`[${label}] 加载文件失败:`, err);

      // 3秒后清除错误
      setTimeout(() => setError(null), 3000);
    } finally {
      setIsLoading(false);
    }
  };

  // 拖放支持
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const file = e.dataTransfer.files[0];
    if (!file) return;

    setError(null);
    setIsLoading(true);

    try {
      const reader = new FileReader();
      reader.onload = (event) => {
        const content = event.target?.result as string;
        onFileSelected(content, file.name);
        setIsLoading(false);
      };
      reader.onerror = () => {
        throw new Error('读取文件失败');
      };
      reader.readAsText(file);
    } catch (err: any) {
      setError(err.message || '读取文件失败');
      setIsLoading(false);
      setTimeout(() => setError(null), 3000);
    }
  };

  return (
    <div
      className={`file-picker-button ${className}`}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      <button
        onClick={handleClick}
        disabled={disabled || isLoading}
        className={isLoading ? 'loading' : ''}
        title={fileName || `点击选择${label}`}
      >
        {isLoading ? '⏳' : '📂'}
        {isLoading ? '加载中...' : fileName || label}
      </button>

      {error && (
        <div className="file-picker-error">
          ❌ {error}
        </div>
      )}
    </div>
  );
}
