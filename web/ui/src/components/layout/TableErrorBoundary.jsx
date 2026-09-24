import { Component } from "react";
import TableErrorFallback from "./TableErrorFallback";

// Keeps a render exception inside the table from white-screening the app. The
// engine, the peer session, and the game state all live above this boundary in
// GameProvider, so the match keeps running while the table is replaced by the
// fallback; "Try again" remounts the subtree against the same state.
export default class TableErrorBoundary extends Component {
  state = { error: null, resetKey: this.props.resetKey };

  static getDerivedStateFromError(error) {
    return { error };
  }

  // A fresh snapshot (an opponent's action, a resolved decision) may no longer
  // hit the failing render path, so clear the error when the reset key moves.
  static getDerivedStateFromProps(props, state) {
    if (props.resetKey === state.resetKey) return null;
    return { error: null, resetKey: props.resetKey };
  }

  componentDidCatch(error, info) {
    console.error("[ironsmith] table render error", error, info?.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <TableErrorFallback
          error={this.state.error}
          onRetry={() => this.setState({ error: null })}
        />
      );
    }
    return this.props.children;
  }
}
