import React, { Component } from 'react';
import ErrorScreen from './ErrorScreen/ErrorScreen';

interface Props {
  fallback?: JSX.Element;
  children: React.ReactNode;
}

type State = {
  error: Error | null;
};

class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      return <ErrorScreen error={this.state.error} />;
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
