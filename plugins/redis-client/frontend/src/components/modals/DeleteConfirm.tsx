interface Props {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DeleteConfirm({ message, onConfirm, onCancel }: Props) {
  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal-content modal-sm" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h3>确认删除</h3>
          <button className="btn-secondary" onClick={onCancel}>✕</button>
        </div>
        <div className="modal-body delete-confirm-body">
          <div className="delete-warning-icon">&#9888;</div>
          <p>{message}</p>
          <p className="delete-warning-hint">此操作不可撤销</p>
        </div>
        <div className="modal-footer">
          <button className="btn-secondary" onClick={onCancel}>取消</button>
          <button className="btn-danger" onClick={onConfirm}>确认删除</button>
        </div>
      </div>
    </div>
  );
}
