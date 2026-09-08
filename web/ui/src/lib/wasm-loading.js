// The bundler gives production assets content-addressed URLs. Keep that identity
// intact for HTTP/compiled-code caches and consume progress on the compile stream.
export async function compileWasmWithProgress(url, onProgress = () => {}, {
  fetchModule = fetch, wasm = WebAssembly, estimatedSize = 40_000_000,
} = {}) {
  const response = await fetchModule(url);
  if (!response.ok) throw new Error(`WASM fetch failed: HTTP ${response.status}`);
  const total = Number(response.headers.get('content-length')) || estimatedSize;
  let received = 0;
  let reported = 0;
  const report = chunk => {
    received += chunk.byteLength;
    const progress = Math.min(received / total, 1);
    if (progress - reported >= 0.005) { reported = progress; onProgress(progress); }
  };
  if (response.body && typeof wasm.compileStreaming === 'function') {
    const stream = response.body.pipeThrough(new TransformStream({
      transform(chunk, controller) { report(chunk); controller.enqueue(chunk); },
    }));
    // Some static hosts omit the WASM MIME type. We know the asset type here.
    const headers = new Headers(response.headers);
    headers.set('content-type', 'application/wasm');
    try {
      const module = await wasm.compileStreaming(new Response(stream, { headers }));
      onProgress(1);
      return module;
    } catch (error) {
      // Invalid code/CSP failures must remain visible; only streaming API
      // incompatibility gets an array-buffer fallback, fetched from HTTP cache.
      if (!(error instanceof TypeError)) throw error;
      try { await stream.cancel(); } catch { /* compileStreaming owns the reader */ }
      const retry = await fetchModule(url);
      if (!retry.ok) throw new Error(`WASM fetch failed: HTTP ${retry.status}`);
      const module = await wasm.compile(await retry.arrayBuffer());
      onProgress(1);
      return module;
    }
  }
  const module = await wasm.compile(await response.arrayBuffer());
  onProgress(1);
  return module;
}
