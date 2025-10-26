#!/usr/bin/env node
// Minimal WS+HTTP cycle:
// - Connects to ws://.../monitor once
// - In each cycle: POST 3 tasks (best_bid, best_ask, top_n_depth)
// - Waits for their Finished updates via the monitor and prints results
//
// Requires: Node 18+ and `npm i ws`

const GATEWAY_URL = process.env.GATEWAY_URL || 'http://localhost:5000';
const MONITOR_WS_URL = GATEWAY_URL.replace('http://', 'ws://').replace('https://', 'wss://') + '/monitor';
const DEPTH_LEVELS = Number(process.env.DEPTH_LEVELS || 5);
const PERIOD_MS = Number(process.env.PERIOD_MS || 1000);

let WebSocket;
try {
  WebSocket = require('ws');
} catch (_) {
  try {
    const Module = require('module');
    const cp = require('child_process');
    const globalRoot = cp.execSync('npm root -g').toString().trim();
    const greq = Module.createRequire(globalRoot + '/');
    WebSocket = greq('ws');
  } catch (_) {
    console.error('ws module not found. Install locally: `npm i ws`, or globally: `npm i -g ws` and run with `NODE_PATH=$(npm root -g)`');
    process.exit(1);
  }
}

function jstr(x) { try { return JSON.stringify(x); } catch { return String(x); } }

// Correlation helpers
const pending = []; // queued requests awaiting id (by NewRequest)
const waiters = new Map(); // id -> { resolve, reject }

function deepEq(a, b) { return jstr(a) === jstr(b); }

function connectMonitor() {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(MONITOR_WS_URL);
    ws.once('open', () => resolve(ws));
    ws.once('error', (e) => reject(e));
  });
}

function fiberStr(ft) {
  // FiberType is a tuple struct; serde may serialize as {"0":"order_book"} or as string
  if (typeof ft === 'string') return ft;
  if (ft && typeof ft === 'object' && '0' in ft) return ft['0'];
  return String(ft);
}

function idVal(id) {
  if (typeof id === 'number') return id;
  if (id && typeof id === 'object' && '0' in id) return id['0'];
  return id;
}

function startMonitor(ws) {
  ws.on('message', (buf) => {
    let msg; try { msg = JSON.parse(String(buf)); } catch { return; }

    if (msg.NewRequest) {
      const { id, fiber_type, function_key, init_values } = msg.NewRequest;
      const idx = pending.findIndex(
        (p) => p.function_key === function_key && p.fiber_type === fiberStr(fiber_type) && deepEq(p.init_values, init_values || [])
      );
      if (idx !== -1) {
        const { resolve, reject } = pending.splice(idx, 1)[0];
        waiters.set(idVal(id), { resolve, reject });
      }
      return;
    }

    if (msg.TxUpdate) {
      const { meta, result } = msg.TxUpdate;
      const wid = idVal(meta?.id);
      const w = waiters.get(wid);
      if (!w) return;
      const status = meta?.status?.type || meta?.status || '';
      if (status === 'Finished') {
        try { w.resolve(result ?? null); } finally { waiters.delete(wid); }
      }
    }
  });

  ws.on('close', () => {
    for (const [, w] of waiters) { try { w.reject(new Error('monitor closed')); } catch {} }
    waiters.clear();
  });
}

async function postNewRequest(bp) {
  const res = await fetch(`${GATEWAY_URL}/new_request`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(bp),
  });
  if (!res.ok) throw new Error(`POST /new_request ${res.status}`);
}

function request(function_key, init_values) {
  const bp = { fiber_type: 'order_book', function_key, init_values };
  return new Promise(async (resolve, reject) => {
    pending.push({ fiber_type: 'order_book', function_key, init_values, resolve, reject });
    try { await postNewRequest(bp); } catch (e) {
      const i = pending.findIndex((p) => p.function_key === function_key && deepEq(p.init_values, init_values));
      if (i !== -1) pending.splice(i, 1);
      reject(e);
    }
  });
}

const getOptU64 = (v) => (v && typeof v === 'object' && 'OptionU64' in v ? v.OptionU64 : null);
const getSnapshot = (v) => (v && typeof v === 'object' && 'BookSnapshot' in v ? v.BookSnapshot : { bids: [], asks: [] });

function printCycle(i, bb, ba, snap) {
  const bids = (snap.bids || []).map((l) => `${l.price}@${l.qty}`).join(', ') || '-';
  const asks = (snap.asks || []).map((l) => `${l.price}@${l.qty}`).join(', ') || '-';
  console.log(`tick ${i}: best_bid=${bb ?? '-'} best_ask=${ba ?? '-'} | bids[${snap.bids?.length||0}]=${bids} | asks[${snap.asks?.length||0}]=${asks}`);
}

(async () => {
  console.log(`Connecting to monitor ${MONITOR_WS_URL} ...`);
  const ws = await connectMonitor();
  startMonitor(ws);
  console.log(`Connected. Gateway ${GATEWAY_URL}. Depth=${DEPTH_LEVELS}. Period=${PERIOD_MS}ms`);

  let i = 0;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    i += 1;
    try {
      const [bb, ba, depth] = await Promise.all([
        request('best_bid', []),
        request('best_ask', []),
        request('top_n_depth', [{ U64: DEPTH_LEVELS }]),
      ]);
      printCycle(i, getOptU64(bb), getOptU64(ba), getSnapshot(depth));
    } catch (e) {
      console.error('cycle error:', e?.message || e);
    }
    await new Promise((r) => setTimeout(r, PERIOD_MS));
  }
})();
