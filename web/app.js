let selectedWorkspaceId = null;
let selectedFile = null;
let socket = null;
let termBuffers = new Map();

const gridEl = document.getElementById('grid');
const filesEl = document.getElementById('files');
const diffEl = document.getElementById('diff');
const filesTitleEl = document.getElementById('files-title');
const diffTitleEl = document.getElementById('diff-title');
const addBtn = document.getElementById('add');
const pathInput = document.getElementById('path');
const termOutput = document.getElementById('term-output');
const termInput = document.getElementById('term-input');
const termKindSel = document.getElementById('term-kind');

addBtn.addEventListener('click', addWorkspace);
pathInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') addWorkspace();
});

document.getElementById('agent-start').onclick = () => terminalCmd('StartTerminal', 'Agent');
document.getElementById('agent-stop').onclick = () => terminalCmd('StopTerminal', 'Agent');
document.getElementById('shell-start').onclick = () => terminalCmd('StartTerminal', 'Shell');
document.getElementById('shell-stop').onclick = () => terminalCmd('StopTerminal', 'Shell');
termInput.addEventListener('keydown', (e) => {
  if (e.key !== 'Enter' || !selectedWorkspaceId) return;
  const text = termInput.value;
  termInput.value = '';
  if (!text) return;
  sendWs({
    SendTerminalInput: {
      id: selectedWorkspaceId,
      kind: termKindSel.value,
      data_b64: btoa(text + '\n')
    }
  });
});

function connectWs() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  socket = new WebSocket(`${proto}://${location.host}/ws`);
  socket.onmessage = (ev) => {
    let evt;
    try {
      evt = JSON.parse(ev.data);
    } catch {
      return;
    }
    handleEvent(evt);
  };
  socket.onclose = () => setTimeout(connectWs, 1000);
}

function sendWs(obj) {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  socket.send(JSON.stringify(obj));
}

function handleEvent(evt) {
  if (evt.WorkspaceList) {
    renderWorkspaces(evt.WorkspaceList.items);
    return;
  }
  if (evt.TerminalOutput) {
    const t = evt.TerminalOutput;
    const key = `${t.id}:${t.kind}`;
    const chunk = atob(t.data_b64 || '');
    const prev = termBuffers.get(key) || '';
    const next = (prev + chunk).slice(-120000);
    termBuffers.set(key, next);
    renderTerminal();
    return;
  }
  if (evt.TerminalStarted) {
    appendTerminalLine(evt.TerminalStarted.id, evt.TerminalStarted.kind, '[terminal started]');
    return;
  }
  if (evt.TerminalExited) {
    appendTerminalLine(evt.TerminalExited.id, evt.TerminalExited.kind, '[terminal exited]');
  }
}

function appendTerminalLine(id, kind, line) {
  const key = `${id}:${kind}`;
  const prev = termBuffers.get(key) || '';
  termBuffers.set(key, (prev + '\n' + line + '\n').slice(-120000));
  renderTerminal();
}

function renderTerminal() {
  if (!selectedWorkspaceId) {
    termOutput.textContent = '';
    return;
  }
  const key = `${selectedWorkspaceId}:${termKindSel.value}`;
  termOutput.textContent = termBuffers.get(key) || '';
  termOutput.scrollTop = termOutput.scrollHeight;
}

termKindSel.addEventListener('change', renderTerminal);

async function addWorkspace() {
  const path = pathInput.value.trim();
  if (!path) return;
  await fetch('/api/workspaces', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ path })
  });
  pathInput.value = '';
}

function terminalCmd(kind, term) {
  if (!selectedWorkspaceId) return;
  if (kind === 'StartTerminal') {
    sendWs({ StartTerminal: { id: selectedWorkspaceId, kind: term, cmd: [] } });
  } else {
    sendWs({ StopTerminal: { id: selectedWorkspaceId, kind: term } });
  }
}

function renderWorkspaces(items) {
  if (items.length > 0 && !selectedWorkspaceId) {
    selectedWorkspaceId = items[0].id;
  }
  if (!items.find((i) => i.id === selectedWorkspaceId)) {
    selectedWorkspaceId = items[0] ? items[0].id : null;
  }

  gridEl.innerHTML = '';
  for (const ws of items) {
    const el = document.createElement('div');
    el.className = 'tile' + (ws.id === selectedWorkspaceId ? ' active' : '');
    el.innerHTML = `
      <div><strong>${escapeHtml(ws.name)}</strong></div>
      <div class="path">${escapeHtml(ws.path)}</div>
      <div>branch: ${escapeHtml(ws.branch || '-')}</div>
      <div>dirty: ${ws.dirty_files}</div>
    `;
    el.onclick = async () => {
      selectedWorkspaceId = ws.id;
      selectedFile = null;
      renderWorkspaces(items);
      await refreshGit();
      renderTerminal();
    };
    gridEl.appendChild(el);
  }

  refreshGit();
}

async function refreshGit() {
  filesEl.innerHTML = '';
  diffEl.textContent = '';
  if (!selectedWorkspaceId) {
    filesTitleEl.textContent = 'Changed Files';
    diffTitleEl.textContent = 'Diff';
    return;
  }

  const res = await fetch(`/api/workspace/${selectedWorkspaceId}/git`);
  const git = await res.json();
  filesTitleEl.textContent = `Changed Files (${git.changed.length})`;

  for (const ch of git.changed) {
    const btn = document.createElement('button');
    btn.textContent = `${(ch.status || '').padStart(2, ' ')} ${ch.path}`;
    btn.onclick = async () => {
      selectedFile = ch.path;
      await refreshDiff();
    };
    filesEl.appendChild(btn);
  }

  if (!selectedFile && git.changed.length > 0) {
    selectedFile = git.changed[0].path;
    await refreshDiff();
  }
}

async function refreshDiff() {
  if (!selectedWorkspaceId || !selectedFile) {
    diffTitleEl.textContent = 'Diff';
    diffEl.textContent = '';
    return;
  }
  diffTitleEl.textContent = `Diff: ${selectedFile}`;
  const url = `/api/workspace/${selectedWorkspaceId}/diff?file=${encodeURIComponent(selectedFile)}`;
  const res = await fetch(url);
  diffEl.textContent = await res.text();
}

function escapeHtml(s) {
  return String(s)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

connectWs();
refreshGit();
setInterval(refreshGit, 1500);
