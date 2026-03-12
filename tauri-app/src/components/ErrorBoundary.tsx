import { Component, ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): State {
    return {
      hasError: true,
      error,
      errorInfo: null,
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error("[ErrorBoundary] 捕获到错误:", error);
    console.error("[ErrorBoundary] 错误堆栈:", errorInfo.componentStack);

    this.setState({
      error,
      errorInfo,
    });
  }

  render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div
          style={{
            padding: "40px",
            maxWidth: "800px",
            margin: "0 auto",
            textAlign: "center",
          }}
        >
          <div style={{ fontSize: "64px", marginBottom: "20px" }}>💥</div>
          <h2>组件渲染错误</h2>
          <pre
            style={{
              textAlign: "left",
              background: "#f8f9fa",
              padding: "20px",
              borderRadius: "8px",
              fontSize: "14px",
              color: "#495057",
              whiteSpace: "pre-wrap",
              marginTop: "20px",
              overflow: "auto",
            }}
          >
            <strong>错误信息:</strong>
            {this.state.error?.toString()}
            {"\n\n"}
            <strong>组件堆栈:</strong>
            {this.state.errorInfo?.componentStack}
          </pre>
        </div>
      );
    }

    return this.props.children;
  }
}
