// Submit work immediately; only inspect link status after non-blocking completion.
// Without the extension, defer the potentially blocking check to the next frame.
export function prepareShaderProgram(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  const shaders = [];
  let frame = null, disposed = false, settle;
  const extension = gl.getExtension('KHR_parallel_shader_compile');
  const ready = new Promise((resolve, reject) => {
    settle = resolve;
    try {
      for (const [type, source] of [[gl.VERTEX_SHADER, vertexSource], [gl.FRAGMENT_SHADER, fragmentSource]]) {
        const shader = gl.createShader(type);
        shaders.push(shader);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        gl.attachShader(program, shader);
      }
      gl.linkProgram(program);
      const poll = () => {
        frame = null;
        if (disposed || gl.isContextLost()) { resolve(false); return; }
        if (extension && !gl.getProgramParameter(program, extension.COMPLETION_STATUS_KHR)) {
          frame = requestAnimationFrame(poll);
          return;
        }
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
          const info = [gl.getProgramInfoLog(program), ...shaders.map(shader => gl.getShaderInfoLog(shader))].filter(Boolean).join('\n');
          reject(new Error(info || 'Shader link failed'));
          return;
        }
        resolve(true);
      };
      frame = requestAnimationFrame(poll);
    } catch (error) { reject(error); }
  });
  return {
    program, ready,
    dispose() {
      if (disposed) return;
      disposed = true;
      if (frame !== null) cancelAnimationFrame(frame);
      settle(false);
      for (const shader of shaders) gl.deleteShader(shader);
      gl.deleteProgram(program);
    },
  };
}
