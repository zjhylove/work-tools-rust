import { Component } from 'solid-js';
import './Dialog.css';

interface LogViewerProps {
  onClose: () => void;
}

const LogViewer: Component<LogViewerProps> = (props) => {
  return (
    <div class="dialog-overlay" onClick={props.onClose}>
      <div class="dialog-content dialog-large" onClick={(e) => e.stopPropagation()}>
        <div class="dialog-header">
          <h2>系统日志</h2>
          <button class="dialog-close" onClick={props.onClose}>
            ✕
          </button>
        </div>
        <div class="dialog-body">
          <div class="log-viewer">
            <p>日志文件位于: ~/.worktools/logs/work-tools.log</p>
            <pre>{`暂无日志记录`}</pre>
          </div>
        </div>
      </div>
    </div>
  );
};

export default LogViewer;
