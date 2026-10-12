const { spawnSync } = require('node:child_process');
const { readFileSync } = require('node:fs');
try {
  const exe = process.env.EASYEDA_EXE;
  if (!exe) throw Error('Set EASYEDA_EXE to the installed lceda-pro.exe absolute path');
  const args = process.argv.slice(2);
  const index = args.indexOf('--code-file');
  if (index !== -1) {
    if (!args[index + 1]) throw Error('--code-file requires a JavaScript file path');
    args.splice(index, 2, '--code', readFileSync(args[index + 1], 'utf8'));
  }
  const result = spawnSync(exe, args, {
    windowsHide: true, timeout: 70000, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024,
  });
  let payload;
  try { payload = JSON.parse(result.stdout); } catch {}
  process.stdout.write(JSON.stringify({
    status: result.status, signal: result.signal, error: result.error?.message,
    stdout: result.stdout, stderr: result.stderr,
  }, null, 2) + '\n');
  if (result.error || result.status !== 0 || payload?.ok === false) process.exitCode = 1;
} catch (error) { console.error(error.message); process.exitCode = 1; }
