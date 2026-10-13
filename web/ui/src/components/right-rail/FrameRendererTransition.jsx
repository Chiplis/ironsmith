import { Component, createRef } from "react";

// Capture before React replaces the renderer. A visual-only DOM snapshot keeps
// the already-fitted text intact without mounting a second interactive card.
export default class FrameRendererTransition extends Component {
  shell = createRef();
  outgoing = null;
  animation = null;
  motion = null;

  componentDidMount() {
    this.motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.motion.addEventListener("change", this.handleMotionChange);
  }

  handleMotionChange = () => {
    if (this.motion?.matches) this.clearSnapshot();
  };

  clearSnapshot = () => {
    this.animation?.cancel();
    this.animation = null;
    this.outgoing?.remove();
    this.outgoing = null;
  };

  getSnapshotBeforeUpdate(previous) {
    if (!this.props.enabled || this.motion?.matches
      || previous.identity !== this.props.identity
      || !previous.visible || previous.renderer === this.props.renderer) return null;
    const source = this.outgoing || this.shell.current?.firstElementChild;
    return source?.cloneNode(true) || null;
  }

  componentDidUpdate(previous, _state, snapshot) {
    if (!this.props.enabled || previous.identity !== this.props.identity || this.motion?.matches) {
      this.clearSnapshot();
      return;
    }
    if (snapshot) {
      this.clearSnapshot();
      snapshot.setAttribute("data-frame-transition-snapshot", "true");
      snapshot.setAttribute("aria-hidden", "true");
      snapshot.setAttribute("inert", "");
      // Snapshot labels must never collide with live rules helpers or controls.
      for (const node of [snapshot, ...snapshot.querySelectorAll("[id], [aria-labelledby], [aria-describedby]")]) {
        node.removeAttribute("id");
        node.removeAttribute("aria-labelledby");
        node.removeAttribute("aria-describedby");
      }
      snapshot.style.opacity = "1";
      snapshot.style.transition = "none";
      this.shell.current.appendChild(snapshot);
      this.outgoing = snapshot;
    }
    if (this.outgoing && this.props.ready && !this.animation) {
      if (!this.outgoing.animate) {
        this.clearSnapshot();
        return;
      }
      const animation = this.outgoing.animate([{ opacity: 1 }, { opacity: 0 }], {
        duration: 240,
        easing: "cubic-bezier(0.22, 1, 0.36, 1)",
        fill: "forwards",
      });
      this.animation = animation;
      animation.onfinish = () => {
        if (this.animation === animation) this.clearSnapshot();
      };
    }
  }

  componentWillUnmount() {
    this.motion?.removeEventListener("change", this.handleMotionChange);
    this.clearSnapshot();
  }

  render() {
    return <div ref={this.shell} className="card-frame-preview-shell">{this.props.children}</div>;
  }
}
