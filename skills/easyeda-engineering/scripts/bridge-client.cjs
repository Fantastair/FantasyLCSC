const fs = require('node:fs');
async function discover() {
  const candidates = [];
  for (let port = 49620; port <= 49629; port++) {
    try {
      const url = `http://127.0.0.1:${port}`;
      const response = await fetch(`${url}/health`, { signal: AbortSignal.timeout(600) });
      const health = await response.json();
      if (response.ok && health.service === 'easyeda-bridge') candidates.push({ url, health });
    } catch {}
  }
  if (!candidates.length) throw Error('No easyeda-bridge found on localhost ports 49620-49629');
  if (candidates.length > 1) throw Error('Multiple bridges found; inspect services before choosing a target');
  return candidates[0];
}
(async () => {
  const action = process.argv[2] || 'status';
  if (!['status', 'execute'].includes(action)) throw Error('Use status or execute <code-file> [window-id]');
  const { url, health } = await discover();
  const responseWindows = await fetch(`${url}/eda-windows`, { signal: AbortSignal.timeout(2000) });
  if (!responseWindows.ok) throw Error(`Window discovery failed: HTTP ${responseWindows.status}`);
  const windows = await responseWindows.json();
  if (action === 'status') { console.log(JSON.stringify({ url, health, windows }, null, 2)); return; }
  const windowId = process.argv[4] || (windows.count === 1 ? windows.windows[0].windowId : null);
  if (!windowId) throw Error('Explicit window ID required when zero or multiple windows are connected');
  if (!windows.windows.some(window => window.windowId === windowId)) throw Error('Window ID is stale or not connected');
  if (!process.argv[3]) throw Error('A JavaScript code file is required');
  const code = fs.readFileSync(process.argv[3], 'utf8');
  const response = await fetch(`${url}/execute`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code, windowId }), signal: AbortSignal.timeout(35000),
  });
  const result = await response.json();
  console.log(JSON.stringify(result, null, 2));
  if (!response.ok || !result.success) process.exitCode = 1;
})().catch(error => { console.error(error.message); process.exitCode = 1; });
