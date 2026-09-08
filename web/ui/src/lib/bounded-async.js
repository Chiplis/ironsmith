export function createAsyncLimiter(concurrency = 8) {
  let running = 0, head = 0;
  let queue = [];
  const drain = () => {
    while (running < concurrency && head < queue.length) {
      const { task, resolve, reject } = queue[head++];
      running++;
      Promise.resolve().then(task).then(resolve, reject).finally(() => { running--; drain(); });
    }
    if (head === queue.length) { queue = []; head = 0; }
  };
  return task => new Promise((resolve, reject) => { queue.push({ task, resolve, reject }); drain(); });
}
