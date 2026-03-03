import React from 'react';
import './Dialog.css';

interface LogViewerProps {
  onClose: () => void;
}

const LogViewer: React.FC<LogViewerProps> = ({ onClose }) => {
  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog-content dialog-large" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <h2>系统日志</h2>
          <button className="dialog-close" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="dialog-body">
          <div className="log-viewer">
            <p>日志文件位于: ~/.worktools/logs/work-tools.log</p>
            <pre>{`暂无日志记录`}</pre>
          </div>
        </div>
      </div>
    </div>
  );
};

export default LogViewer;
