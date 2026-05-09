interface Props {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DeleteConfirm({ message, onConfirm, onCancel }: Props) {
  return (
    <div className="modal-overlay">
      <div className="modal-content modal-sm">
        <div className="modal-body"><p>{message}</p></div>
        <div className="modal-footer">
          <button className="btn-danger" onClick={onConfirm}>确认删除</button>
          <button onClick={onCancel}>取消</button>
        </div>
      </div>
    </div>
  );
}
