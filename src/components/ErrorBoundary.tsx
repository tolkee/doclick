import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Top-level error boundary. Catches render-time exceptions in the React
 * tree and shows a minimal recovery UI in the overlay's chrome style.
 * Asynchronous errors (event handlers, promises) bypass this — those are
 * surfaced via the store's `lastError` and the PanicIndicator.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("doclick: render error", error, info.componentStack);
  }

  private handleReload = (): void => {
    window.location.reload();
  };

  render(): ReactNode {
    if (!this.state.error) {
      return this.props.children;
    }

    return (
      <div className="flex h-screen w-screen items-center justify-center rounded-xl border border-border/50 bg-background p-4 text-foreground shadow-2xl">
        <div className="max-w-sm space-y-3 text-center">
          <h1 className="text-sm font-semibold text-destructive">Doclick crashed</h1>
          <p className="text-xs text-muted-foreground">
            {this.state.error.message || "Unknown render error"}
          </p>
          <button
            onClick={this.handleReload}
            className="rounded-md bg-secondary px-3 py-1 text-xs font-medium hover:bg-secondary/80"
          >
            Reload overlay
          </button>
        </div>
      </div>
    );
  }
}
