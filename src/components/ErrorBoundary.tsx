import { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RefreshCw } from 'lucide-react';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallbackTitle?: string;
  onReset?: () => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  errorMessage: string | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  public state: ErrorBoundaryState = {
    hasError: false,
    errorMessage: null,
  };

  public static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return {
      hasError: true,
      errorMessage: error?.message || 'An unexpected rendering error occurred.',
    };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    // Structured diagnostic logging without exposing private keys or sensitive payloads
    console.error('[bazq-ui] Uncaught component render error:', {
      message: error.message,
      componentStack: errorInfo.componentStack,
    });
  }

  private handleReset = () => {
    this.setState({ hasError: false, errorMessage: null });
    if (this.props.onReset) {
      this.props.onReset();
    }
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div
          className="card animate-fade-in"
          style={{
            margin: '24px auto',
            maxWidth: '600px',
            padding: '24px',
            textAlign: 'center',
            border: '1px solid rgba(245, 101, 101, 0.4)',
            backgroundColor: 'var(--status-danger-bg)',
          }}
        >
          <div
            style={{
              width: '48px',
              height: '48px',
              borderRadius: '50%',
              backgroundColor: 'rgba(245, 101, 101, 0.15)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--status-danger)',
              margin: '0 auto 14px auto',
            }}
          >
            <AlertTriangle size={24} />
          </div>

          <h3 style={{ margin: '0 0 8px 0', fontSize: '16px', color: 'var(--text-primary)' }}>
            {this.props.fallbackTitle || 'Display Rendering Error'}
          </h3>

          <p
            style={{
              margin: '0 0 16px 0',
              fontSize: '13px',
              color: 'var(--text-secondary)',
              lineHeight: '1.5',
            }}
          >
            A component encounter an unexpected rendering failure. Your paired identity and saved data remain completely safe.
          </p>

          {this.state.errorMessage && (
            <div
              style={{
                padding: '8px 12px',
                borderRadius: 'var(--radius-sm)',
                backgroundColor: 'var(--bg-card)',
                border: '1px solid var(--border-subtle)',
                fontFamily: 'var(--font-mono)',
                fontSize: '11.5px',
                color: 'var(--status-danger)',
                marginBottom: '16px',
                wordBreak: 'break-word',
              }}
            >
              {this.state.errorMessage}
            </div>
          )}

          <button
            type="button"
            className="btn btn-secondary btn-sm"
            onClick={this.handleReset}
            style={{ margin: '0 auto' }}
          >
            <RefreshCw size={13} />
            <span>Retry View</span>
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
