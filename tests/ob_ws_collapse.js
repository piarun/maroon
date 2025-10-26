#!/usr/bin/env node
// Collapse/execute outstanding order book levels by issuing aggressive orders.
// Strategy per cycle:
// 1) Fetch top-N depth via WS endpoint
// 2) If asks exist: place one buy at >= max ask price for total ask qty
// 3) Refresh depth; if bids exist: place one sell at <= best bid price for total bid qty
// 4) Repeat until empty (or until stopped)
//
// Requires: Node 18+; ws installed (locally `npm i ws` or globally `npm i -g ws`).

const GATEWAY_URL = process.env.GATEWAY_URL || 'http://localhost:5000';
const WS_BASE = GATEWAY_URL.replace('http://', 'ws://').replace('https://', 'wss://');
const WS_DEPTH = WS_BASE + '/ws/order_book/top_n_depth';
const WS_ADD_BUY = WS_BASE + '/ws/order_book/add_buy';
const WS_ADD_SELL = WS_BASE + '/ws/order_book/add_sell';

const PERIOD_MS = Number(process.env.PERIOD_MS || 500);
const N_LEVELS = Number(process.env.N_LEVELS || 100);
const EMPTY_WAIT_MIN_MS = Number(process.env.EMPTY_WAIT_MIN_MS || 5000);
const EMPTY_WAIT_MAX_MS = Number(process.env.EMPTY_WAIT_MAX_MS || 7000);

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
    console.error('ws module not found. Install locally: `npm i ws`, or globally: `npm i -g ws` (set NODE_PATH if needed).');
    process.exit(1);
  }
}

function requestWsTo(url, payload) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    ws.on('open', () => {
      try { ws.send(JSON.stringify(payload)); } catch (e) { reject(e); try { ws.close(); } catch {} }
    });
    ws.on('message', (buf) => {
      let msg; try { msg = JSON.parse(String(buf)); } catch { return; }
      const status = msg?.meta?.status?.type || msg?.meta?.status || '';
      if (status === 'Finished') { resolve(msg?.result ?? null); try { ws.close(); } catch {} }
    });
    ws.on('error', (e) => reject(e));
  });
}

const getSnapshot = (v) => (v && typeof v === 'object' && 'BookSnapshot' in v ? v.BookSnapshot : { bids: [], asks: [] });
const getTrades = (v) => (v && typeof v === 'object' && 'ArrayTrade' in v ? v.ArrayTrade : []);

let nextOrderId = BigInt(process.env.START_ORDER_ID || 1000000);
function allocOrderId() { const id = nextOrderId; nextOrderId += 1n; return Number(id); }

async function fetchDepth(n) { return getSnapshot(await requestWsTo(WS_DEPTH, { n })); }
async function buyAgg(price, qty) { return getTrades(await requestWsTo(WS_ADD_BUY, { id: allocOrderId(), price, qty })); }
async function sellAgg(price, qty) { return getTrades(await requestWsTo(WS_ADD_SELL, { id: allocOrderId(), price, qty })); }

function sumQty(levels) { return levels.reduce((acc, l) => acc + (l?.qty || 0), 0); }

function fmtLevels(levels) { return levels.map((l) => `${l.price}@${l.qty}`).join(', ') || '-'; }

(async () => {
  console.log(`Gateway ${GATEWAY_URL}. Collapse using WS endpoints:`);
  console.log(`  depth: ${WS_DEPTH}`);
  console.log(`  add_buy: ${WS_ADD_BUY}`);
  console.log(`  add_sell: ${WS_ADD_SELL}`);
  console.log(`N_LEVELS=${N_LEVELS} PERIOD_MS=${PERIOD_MS} EMPTY_WAIT=${EMPTY_WAIT_MIN_MS}-${EMPTY_WAIT_MAX_MS}ms`);

  let tick = 0;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    tick += 1;
    try {
      let snap = await fetchDepth(N_LEVELS);
      const asks = snap.asks || [];
      const bids = snap.bids || [];
      const askQty = sumQty(asks);
      const bidQty = sumQty(bids);

      console.log(`tick ${tick}: asks[${asks.length}]=${fmtLevels(asks)} | bids[${bids.length}]=${fmtLevels(bids)}`);

      if (askQty > 0) {
        const maxAsk = asks.reduce((m, l) => Math.max(m, l.price), 0);
        const trades = await buyAgg(maxAsk, askQty);
        console.log(`  buyAgg qty=${askQty} @>=${maxAsk} -> trades=${trades.length}`);
      }

      // Refresh depth before selling to avoid self-cross
      snap = await fetchDepth(N_LEVELS);
      const bids2 = snap.bids || [];
      const bidQty2 = sumQty(bids2);
      if (bidQty2 > 0) {
        const bestBid = bids2[0]?.price || 0;
        const trades = await sellAgg(bestBid, bidQty2);
        console.log(`  sellAgg qty=${bidQty2} @<=${bestBid} -> trades=${trades.length}`);
      }

      const finalSnap = await fetchDepth(N_LEVELS);
      const remaining = (finalSnap.asks?.length || 0) + (finalSnap.bids?.length || 0);
      if (remaining === 0) {
        const waitMs = Math.floor(Math.random() * (EMPTY_WAIT_MAX_MS - EMPTY_WAIT_MIN_MS + 1)) + EMPTY_WAIT_MIN_MS;
        console.log(`Book is empty. Waiting ${waitMs}ms before re-check...`);
        await new Promise((r) => setTimeout(r, waitMs));
        continue; // start next loop
      }
    } catch (e) {
      console.error('tick error:', e?.message || e);
    }
    await new Promise((r) => setTimeout(r, PERIOD_MS));
  }
})();
