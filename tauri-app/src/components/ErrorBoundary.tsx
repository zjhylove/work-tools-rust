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
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error, errorInfo: null };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error("[ErrorBoundary] 捕获到错误:", error);
    console.error("[ErrorBoundary] 错误堆栈:", errorInfo.componentStack);
    this.setState({ error, errorInfo });
  }

  render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;

      return (
        <div
          style={{
            padding: 40,
            maxWidth: 600,
            margin: "0 auto",
            textAlign: "center",
            fontFamily: "var(--font-sans)",
          }}
        >
          <div
            style={{
              width: 56,
              height: 56,
              borderRadius: "50%",
              background: "var(--error-light, #fef2f2)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              margin: "0 auto 16px",
              fontSize: 24,
            }}
          >
            !
          </div>
          <h2
            style={{
              margin: "0 0 12px",
              fontSize: 17,
              fontWeight: 600,
              color: "var(--text-primary)",
            }}
          >
            组件渲染错误
          </h2>
          <pre
            style={{
              textAlign: "left",
              background: "var(--bg-secondary, #f8f9fa)",
              padding: 16,
              borderRadius: "var(--radius-md, 8px)",
              fontSize: 13,
              color: "var(--text-secondary, #6b7280)",
              whiteSpace: "pre-wrap",
              overflow: "auto",
              fontFamily: "var(--font-mono)",
              lineHeight: 1.6,
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
