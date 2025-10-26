#!/usr/bin/env node
// Places buy/sell orders over WebSocket per request and logs trades.
// Requires: Node 18+; ws installed (locally `npm i ws` or globally `npm i -g ws`).

const GATEWAY_URL = process.env.GATEWAY_URL || 'http://localhost:5000';
const WS_BASE = GATEWAY_URL.replace('http://', 'ws://').replace('https://', 'wss://');
const WS_ADD_BUY = WS_BASE + '/ws/order_book/add_buy';
const WS_ADD_SELL = WS_BASE + '/ws/order_book/add_sell';
const PERIOD_MS = Number(process.env.PERIOD_MS || 1000);
const BATCH = Number(process.env.BATCH || 1); // orders per side per tick
const BASE_PRICE = Number(process.env.BASE_PRICE || 1000);
const PRICE_SPREAD = Number(process.env.PRICE_SPREAD || 20); // +/- around base
const QTY_MIN = Number(process.env.QTY_MIN || 1);
const QTY_MAX = Number(process.env.QTY_MAX || 5);

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

function randInt(min, max) { return Math.floor(Math.random() * (max - min + 1)) + min; }

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

let nextOrderId = BigInt(process.env.START_ORDER_ID || 1);
function allocOrderId() { const id = nextOrderId; nextOrderId += 1n; return Number(id); }

async function placeBuy(price, qty) {
  const id = allocOrderId();
  const res = await requestWsTo(WS_ADD_BUY, { id, price, qty });
  return { side: 'buy', id, price, qty, trades: Array.isArray(res?.ArrayTrade) ? res.ArrayTrade : [] };
}

async function placeSell(price, qty) {
  const id = allocOrderId();
  const res = await requestWsTo(WS_ADD_SELL, { id, price, qty });
  return { side: 'sell', id, price, qty, trades: Array.isArray(res?.ArrayTrade) ? res.ArrayTrade : [] };
}

function fmtTrades(trades) { return trades.map((t) => `${t.qty}@${t.price}(taker:${t.takerId},maker:${t.makerId})`).join(', ') || '-'; }

(async () => {
  console.log(`Gateway ${GATEWAY_URL}. Using WS endpoints:`);
  console.log(`  buy:  ${WS_ADD_BUY}`);
  console.log(`  sell: ${WS_ADD_SELL}`);
  console.log(`Period=${PERIOD_MS}ms; batch=${BATCH}`);
  console.log(`Price ~ ${BASE_PRICE} +/- ${PRICE_SPREAD}; qty ${QTY_MIN}-${QTY_MAX}`);

  let tick = 0;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    tick += 1;
    try {
      const buys = Array.from({ length: BATCH }, () => ({
        price: BASE_PRICE - randInt(0, PRICE_SPREAD),
        qty: randInt(QTY_MIN, QTY_MAX),
      }));
      const sells = Array.from({ length: BATCH }, () => ({
        price: BASE_PRICE + randInt(0, PRICE_SPREAD),
        qty: randInt(QTY_MIN, QTY_MAX),
      }));

      const buyPromises = buys.map((o) => placeBuy(o.price, o.qty));
      const sellPromises = sells.map((o) => placeSell(o.price, o.qty));
      const results = await Promise.all([...buyPromises, ...sellPromises]);

      // Summarize
      const tradesCount = results.reduce((acc, r) => acc + r.trades.length, 0);
      console.log(`tick ${tick}: placed ${results.length} orders, trades=${tradesCount}`);
      for (const r of results) {
        console.log(`  ${r.side} id=${r.id} ${r.qty}@${r.price} -> trades: ${fmtTrades(r.trades)}`);
      }
    } catch (e) {
      console.error('tick error:', e?.message || e);
    }

    await new Promise((r) => setTimeout(r, PERIOD_MS));
  }
})();
