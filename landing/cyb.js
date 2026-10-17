/* cyb.ai — the robot · the deck. seven chapters: robot · own · worlds · anatomy · wire · numbers · get it
   marks: ● measured · ○ target · ◌ mock. every number carries one. */
(() => {
const $ = (s, r = document) => r.querySelector(s);
const el = (h) => { const t = document.createElement('template'); t.innerHTML = h.trim(); return t.content.firstChild; };
const fmt = (n) => n.toLocaleString('en-US');
const esc = (t) => String(t).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const M = (k) => `<i class="mk ${k}"></i>`;
const MONO = 'JetBrains Mono,Menlo,monospace';
const COLS = ['#ff1a00', '#ff7a00', '#ffc400', '#00ff41', '#00b8ff', '#1e00ff', '#7a00ff'];
const REL = 'https://github.com/cyberia-to/cyb/releases/download/v0.16.0/';

/* ------------------------------------------------------------------ sound · synthesized, no files */
const sfx = (() => {
  let ac = null, master = null, on = localStorage.getItem('cyber.sound') !== 'off', amb = null;
  const ensure = () => { if (ac) return true; try { ac = new (window.AudioContext || window.webkitAudioContext)(); master = ac.createGain(); master.gain.value = .22; master.connect(ac.destination); } catch { return false; } return true; };
  const resume = () => { if (ac && ac.state === 'suspended') ac.resume(); };
  const tone = (f0, f1, dur, type = 'sine', g = .5, at = 0) => { if (!on || !ensure()) return; resume(); const t = ac.currentTime + at; const o = ac.createOscillator(), v = ac.createGain(); o.type = type; o.frequency.setValueAtTime(f0, t); if (f1) o.frequency.exponentialRampToValueAtTime(f1, t + dur); v.gain.setValueAtTime(0, t); v.gain.linearRampToValueAtTime(g, t + .006); v.gain.exponentialRampToValueAtTime(.0005, t + dur); o.connect(v); v.connect(master); o.start(t); o.stop(t + dur + .02); };
  const noise = (dur, g = .3, hp = 2000) => { if (!on || !ensure()) return; resume(); const t = ac.currentTime; const n = ac.sampleRate * dur | 0, b = ac.createBuffer(1, n, ac.sampleRate), d = b.getChannelData(0); for (let i = 0; i < n; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / n); const src = ac.createBufferSource(); src.buffer = b; const f = ac.createBiquadFilter(); f.type = 'highpass'; f.frequency.value = hp; const v = ac.createGain(); v.gain.value = g; src.connect(f); f.connect(v); v.connect(master); src.start(t); };
  const ambient = (start) => { if (!ensure()) return; if (start && !amb && on) { const v = ac.createGain(); v.gain.value = 0; v.connect(master); const f = ac.createBiquadFilter(); f.type = 'lowpass'; f.frequency.value = 220; f.connect(v); const os = [55, 55.4, 82.5].map((fr) => { const o = ac.createOscillator(); o.type = 'triangle'; o.frequency.value = fr; o.connect(f); o.start(); return o; }); v.gain.linearRampToValueAtTime(.06, ac.currentTime + 4); amb = { v, os }; } if (!start && amb) { amb.v.gain.linearRampToValueAtTime(0, ac.currentTime + 1); const a = amb; amb = null; setTimeout(() => a.os.forEach((o) => o.stop()), 1200); } };
  const NOTES = [220, 246.9, 277.2, 329.6, 370, 440, 493.9];
  return {
    get on() { return on; },
    toggle() { on = !on; localStorage.setItem('cyber.sound', on ? 'on' : 'off'); if (on) { ensure(); resume(); ambient(true); tone(660, 990, .12, 'sine', .4); } else ambient(false); return on; },
    wake() { if (on && ensure()) { resume(); ambient(true); } },
    tick() { noise(.02, .25, 3000); tone(1800, 1200, .03, 'square', .08); },
    move(dir, ch) { noise(.03, .2, 1500); tone(dir > 0 ? 440 : 520, dir > 0 ? 560 : 400, .08, 'sine', .25); if (ch != null) { const r = NOTES[ch]; tone(r, null, .5, 'sine', .18, .04); tone(r * 1.5, null, .45, 'sine', .1, .08); } },
    link() { tone(880, 1320, .16, 'sine', .35); tone(1320, 1760, .22, 'triangle', .18, .1); },
    hash() { noise(.012, .12, 4000); },
    ticket() { tone(1760, 1750, .35, 'sine', .3); tone(2640, 2630, .25, 'sine', .12, .02); },
    converge(d) { const f = 220 + 1200 * Math.min(1, Math.max(0, (-Math.log10(Math.max(d, 1e-9)) - 1) / 7)); tone(f, f * 1.02, .07, 'triangle', .22); },
    stake(v) { tone(v ? 160 : 110, 60, .25, 'sine', .45); noise(.05, .15, 800); },
    mentor() { [0, 2, 4, 6].forEach((k, i) => tone(NOTES[k] * 2, null, .3, 'sine', .22, i * .09)); },
    chord(ch) { const r = NOTES[ch]; [1, 1.25, 1.5, 2].forEach((m, i) => tone(r * m, null, .9, 'sine', .14, i * .05)); },
  };
})();


const CH = [
  { id: 'robot', name: 'the robot', c: 'var(--c1)' }, { id: 'own', name: 'own', c: 'var(--c2)' }, { id: 'worlds', name: 'worlds', c: 'var(--c3)' }, { id: 'anatomy', name: 'anatomy', c: 'var(--c4)' },
  { id: 'wire', name: 'the wire', c: 'var(--c5)' }, { id: 'numbers', name: 'numbers', c: 'var(--c6)' }, { id: 'get', name: 'get it', c: 'var(--c7)' },
];
const chIdx = (id) => CH.findIndex((c) => c.id === id);
const S = []; const push = (ch, html, cls = '', extra = {}) => S.push({ ch: chIdx(ch), cls, html, ...extra });
const shot = (file, cls, cap) => `<figure class="shot ${cls}"><img src="shots/${file}" alt="${esc(cap)}" loading="lazy">${cap ? `<figcaption>${esc(cap)}</figcaption>` : ''}</figure>`;

/* the twenty-one parts: name · layer · line · today (anatomy.md / cy/specs/anatomy.md) */
const PARTS = [
  ['name', 'identity', 'a unique identity nobody can take — a name that resolves to your robot on a network that cannot be turned off', 'ports with the soft3 genesis'],
  ['avatar', 'identity', 'the robot\'s model — the rendered being others see and recognize', 'to grow'],
  ['soul', 'identity', 'one file that defines how your robot thinks when asked — which model, which manners, what it may do unasked', 'to define'],
  ['ward', 'identity', 'the enforcer: every act any code wants passes its gate — allowed, refused, or asked of you', 'live · effect router'],
  ['soma', 'mind', 'mind and model: local inference on your own silicon. nobody reads your questions', 'live'],
  ['brain', 'mind', 'the knowledge graph, rendered live — everything it knows, as a world you fly through', 'live · 37 fps on pixel, 60 Hz on mac'],
  ['memory', 'mind', 'every particle as a file — the same brain ranked by focus, viewed as your library', 'live'],
  ['com', 'senses', 'the commander: one line where you type — the robot\'s mouth, always present', 'live'],
  ['sense', 'senses', 'the messenger: how it talks to other neurons, particles and robots', 'seed'],
  ['voice', 'senses', 'it hears and speaks', 'to grow'],
  ['vision', 'senses', 'it sees — camera, screen, world', 'to grow'],
  ['state', 'senses', 'reads other networks and proves what it reads; every answer wears a tier — verified, anchored, or unproven', 'doctrine'],
  ['sigma', 'value', 'not a wallet: the sum of everything it owns, and the neurons it commands', 'live · last-known balance'],
  ['vault', 'value', 'secrets under seal — keys, mnemonics, sleeping identities. sigma spends; vault signs', 'live · XChaCha20 under mnemonic + TOTP'],
  ['log', 'time', 'everything that ever happened, on a durable tape — the robot forgets nothing', 'live · 43,613 signals on a phone'],
  ['now', 'time', 'the context: the particle it stands on — what commands act on, what the model sees. attention is a position', 'partial'],
  ['plan', 'time', 'standing orders and schedules — it acts at 3am so you don\'t have to', 'seed'],
  ['time', 'time', 'one screen: the past flowing into now flowing into the plan. your life, one axis', 'to build · the flagship view'],
  ['body', 'flesh', 'the physical body: silicon, sensors, energy, mining — telemetry of the machine it lives on', 'live · the default world'],
  ['cell', 'flesh', 'organs it can grow: live-loaded programs that add abilities while it runs', 'folded into neuron'],
  ['radio', 'flesh', 'the physical layer: sovereign transport to other networks — the wire obeys the graph', 'live as cy wire · shell wiring not started'],
];
const partsFig = (layer) => `<div class="parts">${PARTS.filter((p) => !layer || p[1] === layer).map(([n, l, d, t]) => `<div class="part"><b>${n}<i>${l}</i></b><span>${esc(d)}</span><span class="st">${esc(t)}</span></div>`).join('')}</div>`;

/* 1 · the robot */
push('robot', `<div><p class="kick">cyb · the robot · v0.16.0</p><p class="hero-word">cyb</p>
  <h2>intelligence <em>you own.</em></h2>
  <p class="lead">the immortal robot — your personal interface to superintelligence. a digital being that works for you: thinks while you think, earns while you sleep, remembers what you forget. yours forever.</p>
  <div class="tiles"><div class="tile"><b>not rented</b>not a subscription, not an account that gets banned</div><div class="tile"><b>one binary</b>macOS · android · linux — the same robot, the same worlds</div><div class="tile"><b>local mind</b>inference on your own silicon; nobody reads your questions</div><div class="tile"><b>mines and pays</b>from your pocket, on real chains</div></div>
  <p class="kbd">→ or space to move · <span id="nslides"></span> slides</p></div>`, 'one');
push('robot', `<div><p class="kick">01 · the robot · the body</p><h2>the robot is cy<br><em>with a body.</em></h2><p class="lead">cy is the agent — the mind over the book, headless anywhere. cyb gives it eyes, ears, pictures and a pocket. every cyb is born unique and grows with its owner: it learns from every cyberlink you create, remembers everything you ever linked, and reasons over a living cybergraph no corporation can erase.</p><p class="mute">the robot belongs to its owner, runs on any surface, and answers to no one else.</p></div>${shot('mac.png', 'mac', 'the brain world on macOS · v0.16.0 · 1,437 particles · 5,603 axons')}`, '', { bridge: 'why own a mind at all? because every other one is rented.' });

/* 2 · own */
push('own', `<div><p class="kick">02 · own · the thesis</p><h2>all intelligence sold today<br><em>is rented.</em></h2><p class="lead">the provider owns it, it dies with the account, it cannot be transferred. cyb is a robot you own outright: it earns, acts, remembers, has standing, and does not die. a capital asset that compounds.</p><p class="mute">the wedge: own, don't rent. ownership is the founding principle — no keys, no robot.</p></div>
  <div class="steps"><div class="step"><b>it earns</b><span>mines settlement tickets and proof-of-work on your machine; sigma holds what it owns</span></div><div class="step"><b>it acts</b><span>every act passes ward's gate — allowed, refused, or asked of you; nothing acts on its own</span></div><div class="step"><b>it remembers</b><span>a durable log and a ranked memory; the robot forgets nothing</span></div><div class="step"><b>it does not die</b><span>name + soul + vault seed + log — carry those four to any machine and the robot resurrects</span></div></div>`, 'ink');
push('own', `<div><p class="kick">02 · own · immortality</p><h2>carry four things<br><em>and it resurrects.</em></h2><p class="lead">the robot's immortality needs no extra organ. it is the backup: name, soul, vault seed, log. move them to any machine and the same robot wakes up with the same memory, the same keys, the same standing.</p></div>
  <div class="nums"><div class="n"><big>name</big><small>the identity that resolves through the graph · ports with the soft3 genesis</small></div><div class="n"><big>soul</big><small>one file: which model, which manners, what it may do unasked</small></div><div class="n"><big>seed</big><small>the vault: keys and mnemonics under seal</small></div><div class="n"><big>log</big><small>everything that ever happened, on a durable tape</small></div></div>`, '', { bridge: 'what it looks like, today, on real devices.' });

/* 3 · worlds */
push('worlds', `<div class="full"><p class="kick">03 · worlds · the same robot on a phone</p><h2>ten worlds. <em>one binary.</em></h2><div class="shots">${shot('android.png', 'phone', 'body · telemetry, miners, networks')}${shot('android-brain.png', 'phone', 'brain · the graph you fly through')}${shot('android-log.png', 'phone', 'log · 43,613 signals')}${shot('android-sigma.png', 'phone', 'sigma · what it owns')}</div><p class="fine">pixel 10 · v0.16.0 · all shots are the shipped build ${M('m')}</p></div>`, 'one');
push('worlds', `<div><p class="kick">03 · worlds · the body</p><h2>the default world<br><em>is the body.</em></h2><p class="lead">cpu, memory, network, energy — and the miners: zheng settlement tickets and the seer, with the day's yield. the robot opens on what it already knows and never says "loading": never wait on the network to paint.</p><p class="mute">worlds in the shipped build: body · brain · log · chat · robot · sigma · models · vault · memory · oracle ${M('m')}</p></div>${shot('android.png', 'phone', 'body world · pixel 10')}`);
push('worlds', `<div><p class="kick">03 · worlds · the brain</p><h2>everything it knows,<br><em>as a world you fly through.</em></h2><p class="lead">the knowledge graph rendered live as topology — particles sized by focus, axons as light. one compute kernel per frame; on the phone it paints over Vulkan with zero-copy memory. tap a particle to read it.</p><p class="mute">measured: 37 fps on pixel 10 ${M('m')}, 60 Hz pinned on mac ${M('m')}; the 100+ fps on the old page is a target ${M('t')}</p></div>${shot('android-brain.png', 'phone', 'brain · pixel 10')}`, '', { bridge: 'under the worlds, twenty-one parts in six layers.' });

/* 4 · anatomy */
push('anatomy', `<div class="full"><p class="kick">04 · anatomy · twenty-one parts, six layers</p><h2>a robot is not an account.<br><em>it is an organism.</em></h2>${partsFig()}<p class="fine">identity · mind · senses · value · time · flesh — each part a power that is only yours. status words are from the anatomy page: live, seed, to grow, to build ${M('m')}</p></div>`, 'one ink');
push('anatomy', `<div><p class="kick">04 · anatomy · what is live today</p><h2>soma, brain, memory,<br><em>com, sigma, vault, log, body.</em></h2><p class="lead">eight organs carry real traffic in v0.16.0: the local model streams tokens, the brain paints the graph, memory ranks it by focus, com is the mouth, sigma holds the balance, vault signs under seal, the log keeps every signal, the body reports the machine.</p><p class="mute">to grow: avatar, voice, vision, plan, sense. to build: time, the flagship view — log ← now → plan, your life on one axis. radio is live as cy wire; wiring it into the shell is the named next milestone.</p></div>
  <div class="nums"><div class="n"><big>8</big><small>organs live in v0.16.0 ${M('m')}</small></div><div class="n"><big>5</big><small>to grow · avatar, voice, vision, plan, sense</small></div><div class="n"><big>1</big><small>to build · time, the flagship view ${M('t')}</small></div><div class="n"><big>32</big><small>releases since 0.2.0 · sept–oct 2026 ${M('m')}</small></div></div>`, '', { bridge: 'the robot is never alone: the wire obeys the graph.' });

/* 5 · the wire */
push('wire', `<div><p class="kick">05 · the wire · networks</p><h2>the wire<br><em>obeys the graph.</em></h2><p class="lead">follow, antenna and socket are cyberlinks, so who talks to whom is readable from the graph itself. sovereign transport, no rented relays. proven with three nodes: the third reached the first through the second by reading the address out of the graph.</p><p class="mute">networks are per chain: net add · net set · net rm, with per-chain sync; prover tickets bind to the last-block beacon. the fleet gate runs three isolated bodies and a mock chain on every commit.</p></div>
  <div class="nums"><div class="n"><big>3</big><small>nodes · graph-routed, 2026-09-01 ${M('m')}</small></div><div class="n"><big>21 / 21</big><small>fleet gate asserts green · 3 bodies + mockchain ${M('m')}</small></div><div class="n"><big>h 58 → 60</big><small>spacepussy-test advanced by the robot's own proving ${M('m')}</small></div><div class="n"><big>0</big><small>rented relays</small></div></div>`, 'ink');
push('wire', `<div><p class="kick">05 · the wire · mining from the pocket</p><h2>it earns<br><em>while you sleep.</em></h2><p class="lead">the body runs two miners: zheng settlement tickets over the local graph — every ticket is a Shapley sample with its own proof — and the seer. an erga child mines autolykos on the same machine. yield is read from the ledger, never derived from work counters.</p></div>
  <div class="nums"><div class="n"><big>20,067</big><small>tickets per minute on 16 workers · ~12.2 ms per ticket ${M('m')}</small></div><div class="n"><big>4.4×</big><small>over a single worker ${M('m')}</small></div><div class="n"><big>40.67<span class="u">MH/s</span></big><small>erga child, autolykos ${M('m')}</small></div><div class="n"><big>172,287</big><small>PUSSY/day estimate at that rate ${M('k')}</small></div></div>`, '', { bridge: 'the numbers, all of them marked.' });

/* 6 · numbers */
push('numbers', `<div><p class="kick">06 · numbers · measured</p><h2>what the robot<br><em>does today.</em></h2></div>
  <div class="nums"><div class="n"><big>230<span class="u">tok/s</span></big><small>qwen3-0.6B on M4 Max · 57 for 1.7B · 1.5 for 14B ${M('m')}</small></div><div class="n"><big>21<span class="u">tok/s</span></big><small>on the phone's CPU ${M('m')}</small></div><div class="n"><big>37<span class="u">fps</span></big><small>brain on pixel 10 · 60 Hz pinned on mac ${M('m')}</small></div><div class="n"><big>0.1<span class="u">ms</span></big><small>paint per frame on metal ${M('m')}</small></div><div class="n"><big>19.2<span class="u">k</span></big><small>lines · 76 tests · the whole robot ${M('m')}</small></div><div class="n"><big>39<span class="u">MB</span></big><small>the binary · dmg, apk ${M('m')}</small></div></div>`, 'ink');
push('numbers', `<div><p class="kick">06 · numbers · honest</p><h2>what is target,<br><em>what is shipped.</em></h2><p class="lead">every number on this site is marked. a few that are easy to get wrong:</p></div>
  <div class="steps"><div class="step"><b>100+ fps brain</b><span>target ${M('t')} — measured 37 fps on pixel 10, 60 Hz pinned on mac ${M('m')}</span></div><div class="step"><b>windows and iOS</b><span>builds are attached; windows was red in the 0.16.0 cut and iOS is an unsigned TrollStore ipa — macOS, android and linux are the green platforms ${M('m')}</span></div><div class="step"><b>notarization</b><span>not yet — the install script clears the quarantine flag for you</span></div><div class="step"><b>pussy chaosnet in 0.16.0</b><span>not expected to work in this release; the node line is being redeployed</span></div></div>`, '', { bridge: 'get it.' });

/* 7 · get it */
push('get', `<div><p class="kick">07 · get it · one binary, every machine</p><h2>your robot.<br><em>on your machine.</em></h2><pre class="code">curl -fsSL https://cyb.ai/install.sh | sh   # macOS
cargo install cyb                            # runtime + cy, crates.io
adb install -r cyb-0.16.0.apk                # android</pre><p class="fine">v0.16.0 · open source · sha256sums on the release</p></div>
  <div class="dl"><a href="${REL}cyb-0.16.0.dmg"><b>macOS</b><span>apple silicon · macOS 14+ · 39 MB · after copy: xattr -cr /Applications/cyb.app</span></a><a href="${REL}cyb-0.16.0.apk"><b>android</b><span>arm64-v8a · 39 MB · the identical binary with hardware senses</span></a><a href="${REL}cyb-0.16.0-linux-x86_64.tar.gz"><b>linux</b><span>x86_64 · ubuntu 20.04+ · 37 MB · unpack and run ./cyb</span></a><a href="https://github.com/cyberia-to/cyb"><b>source</b><span>git clone … && make build · make dmg · make android · make fleet</span></a></div>`, 'ink', { id: 'get' });
push('get', `<div><p class="kick">07 · get it · 2026-11-05</p><h2>the chains come back.<br><em>cyb reads them.</em></h2><p class="lead">on 2026-11-05 bostrom and space pussy relaunch on soft3 — and the robot is the client. content visible in cyb is a core requirement of that day: every referenced particle resolves, media and PDFs render, from any link to its two files in one tap. a graph nobody can read is a graph nobody links.</p><p class="mute">claim your old $BOOT and $PUSSY by key, in cyb. the same binary, the same worlds — on real chains.</p></div>
  <div class="ctas"><a class="btn" href="#get">download</a><a class="btn ghost" href="https://cyber.page">cyber.page</a><a class="btn ghost" href="https://soft3.org">soft3.org</a><a class="btn ghost" href="https://cyberia.to">cyberia.to</a><a class="btn ghost" href="https://cyber.page/cyb/">docs</a><a class="btn ghost" href="#0" id="restart">start again</a></div>
  <p class="fine" style="margin-top:18px">cyb · don't trust · don't fear · don't beg</p>`, 'one');

/* build */
const deck = $('#deck'), segs = $('#segs');
S.forEach((s, i) => deck.appendChild(el(`<section class="slide ch${s.ch} ${s.cls || ''}" data-i="${i}" data-ch="${String(s.ch + 1).padStart(2, '0')}" ${s.id ? `id="${s.id}"` : ''} style="--c:${CH[s.ch].c}"><div class="body">${s.html}</div>${s.bridge ? `<p class="bridge">→ ${esc(s.bridge)}</p>` : ''}</section>`)));
CH.forEach((c, i) => { const d = el(`<div class="seg" style="--sc:${c.c}" data-ch="${i}"><div class="fill"></div><span class="lab">${String(i + 1).padStart(2, '0')} ${c.name}</span></div>`); d.addEventListener('click', () => go(S.findIndex((s) => s.ch === i))); segs.appendChild(d); });
$('#nslides').textContent = S.length;
let cur = -1;
const fit = () => {
  const sl = document.querySelectorAll('.slide')[cur]; if (!sl) return;
  const body = sl.querySelector('.body');
  Object.assign(body.style, { transform: '', width: '', height: '', maxHeight: '', margin: '' }); sl.classList.remove('tight', 'tighter', 'scaled');
  const availW = body.clientWidth, availH = body.clientHeight;
  const over = () => body.scrollHeight > availH + 1 || body.scrollWidth > availW + 1;
  if (!over()) return;
  sl.classList.add('tight'); if (!over()) return;
  sl.classList.add('tighter'); if (!over()) return;
  let z = Math.min(availH / body.scrollHeight, availW / body.scrollWidth);
  sl.classList.add('scaled');
  for (let k = 0; k < 5; k++) {
    z = Math.max(.4, Math.floor(z * 100) / 100);
    Object.assign(body.style, { width: availW / z + 'px', height: availH / z + 'px', maxHeight: 'none', margin: '0', transform: `scale(${z})` });
    if (body.scrollHeight <= availH / z + 1 && body.scrollWidth <= availW / z + 1) break;
    z *= .96;
  }
};

let fitT = 0; const refit = () => { clearTimeout(fitT); fitT = setTimeout(fit, 40); };
addEventListener('resize', refit); addEventListener('orientationchange', refit); document.fonts?.ready.then(refit);
const go = (i) => {
  i = Math.max(0, Math.min(S.length - 1, i)); if (i === cur) return;
  const dir = i > cur ? 1 : -1, prevCh = cur >= 0 ? S[cur].ch : -1; cur = i; deck.dataset.dir = dir;
  document.querySelectorAll('.slide').forEach((s, k) => { s.classList.toggle('on', k === i); s.classList.toggle('left', k < i); });
  document.body.classList.toggle('ink', (S[i].cls || '').includes('ink')); document.body.className = document.body.className.replace(/\bch\d+\b/g, '').trim() + ' ch' + S[i].ch;
  const ch = S[i].ch, idx = S.filter((s, k) => s.ch === ch && k <= i).length, n = S.filter((s) => s.ch === ch).length;
  document.querySelectorAll('.seg').forEach((sg, k) => { sg.classList.toggle('cur', k === ch); sg.classList.toggle('done', k < ch); $('.fill', sg).style.width = k < ch ? '100%' : k === ch ? (idx / n * 100) + '%' : '0'; });
  sfx.move(dir, ch !== prevCh ? ch : null); if (navigator.vibrate) navigator.vibrate(8);
  graph.chapter(ch); graph.pulse();
  $('#pos').textContent = `${String(i + 1).padStart(2, '0')} / ${S.length}`;
  history.replaceState(null, '', '#' + (S[i].id || i));
  requestAnimationFrame(() => requestAnimationFrame(fit));
};
window.deckGo = go; window.deckCount = () => S.length; window.deckFit = fit;
$('#prev').onclick = () => go(cur - 1); $('#next').onclick = () => go(cur + 1);
document.addEventListener('keydown', (e) => { if (e.target.tagName === 'INPUT' || (e.target.tagName === 'BUTTON' && [' ', 'Enter'].includes(e.key))) return; if (['ArrowRight', ' ', 'PageDown', 'ArrowDown'].includes(e.key)) { e.preventDefault(); go(cur + 1); } if (['ArrowLeft', 'PageUp', 'ArrowUp'].includes(e.key)) { e.preventDefault(); go(cur - 1); } if (e.key === 'Home') go(0); if (e.key === 'End') go(S.length - 1); });
let wheelLock = 0; document.addEventListener('wheel', (e) => { const t = Date.now(); if (t - wheelLock < 700 || Math.abs(e.deltaY) < 24) return; wheelLock = t; go(cur + (e.deltaY > 0 ? 1 : -1)); }, { passive: true });
let tx = null, ty = 0; document.addEventListener('touchstart', (e) => { tx = e.touches[0].clientX; ty = e.touches[0].clientY; }, { passive: true });
document.addEventListener('touchend', (e) => { if (tx == null) return; const dx = e.changedTouches[0].clientX - tx, dy = e.changedTouches[0].clientY - ty; if (Math.abs(dx) > 60 && Math.abs(dx) > Math.abs(dy)) go(cur + (dx < 0 ? 1 : -1)); tx = null; });
$('#restart')?.addEventListener('click', (e) => { e.preventDefault(); go(0); });
document.addEventListener('pointerdown', (e) => { sfx.wake(); if (e.target.closest('button, a, .seg')) sfx.tick(); }, { passive: true });
document.addEventListener('keydown', () => sfx.wake(), { once: true });
/* music: WE ARE THE CODE · jooyyrocket — loops, starts on the first gesture if it was on last time */
const music = (() => { const a = new Audio('music/we-are-the-code.mp3'); a.loop = true; a.preload = 'metadata'; a.volume = .55; let on = localStorage.getItem('cyb.music') === 'on', armed = false; const btn = $('#music');
  const paint = () => { btn.textContent = (on && !a.paused ? '❚❚' : '▶') + ' we are the code'; btn.classList.toggle('off', !on); btn.title = 'WE ARE THE CODE · jooyyrocket · ' + (on ? 'playing' : 'play'); };
  const play = () => a.play().then(paint).catch(() => { on = false; paint(); });
  btn.onclick = (e) => { e.stopPropagation(); on = !on; localStorage.setItem('cyb.music', on ? 'on' : 'off'); if (on) play(); else { a.pause(); paint(); } };
  document.addEventListener('pointerdown', () => { if (on && a.paused && !armed) { armed = true; play(); } }, { passive: true });
  document.addEventListener('keydown', () => { if (on && a.paused && !armed) { armed = true; play(); } });
  a.addEventListener('play', paint); a.addEventListener('pause', paint); paint(); return { get on() { return on; } }; })();
const sndBtn = $('#snd'); const sndPaint = () => { sndBtn.textContent = sfx.on ? '◉ sound' : '○ sound'; sndBtn.classList.toggle('off', !sfx.on); }; sndBtn.onclick = () => { sfx.toggle(); sndPaint(); }; sndPaint();
if (matchMedia('(pointer:fine)').matches) { const dot = el('<div class="cur-dot"></div>'), ring = el('<div class="cur-ring"></div>'); document.body.append(dot, ring); let rx = 0, ry = 0, px = 0, py = 0; addEventListener('mousemove', (e) => { px = e.clientX; py = e.clientY; dot.style.transform = `translate(${px}px,${py}px)`; ring.classList.toggle('hot', !!e.target.closest('button, a, .seg, .shot')); }, { passive: true }); const lag = () => { rx += (px - rx) * .18; ry += (py - ry) * .18; ring.style.transform = `translate(${rx}px,${ry}px)`; requestAnimationFrame(lag); }; lag(); document.body.classList.add('has-cursor'); }

/* background: the robot's own graph — particles joined by axons, the chapter colour */
const graph = (() => {
  const cv = $('#graph'), ctx = cv.getContext('2d'); let W = 0, H = 0, hue = COLS[0], pulse = 0, nodes = [], edges = [];
  const resize = () => { W = cv.width = innerWidth * devicePixelRatio; H = cv.height = innerHeight * devicePixelRatio; }; addEventListener('resize', resize); resize();
  const rnd = (a, b) => a + Math.random() * (b - a);
  for (let i = 0; i < 160; i++) { const p = { x: rnd(0, W), y: rnd(0, H), vx: rnd(-.12, .12), vy: rnd(-.12, .12), r: rnd(1, 2.4) * devicePixelRatio, phi: Math.pow(Math.random(), 2) }; nodes.push(p); if (i > 0 && Math.random() < .9) edges.push([p, nodes[Math.floor(Math.pow(Math.random(), 2) * i)]]); }
  const tick = () => { ctx.clearRect(0, 0, W, H); for (const p of nodes) { p.x += p.vx; p.y += p.vy; if (p.x < 0 || p.x > W) p.vx *= -1; if (p.y < 0 || p.y > H) p.vy *= -1; }
    ctx.lineWidth = .8 * devicePixelRatio; ctx.strokeStyle = hue; ctx.globalAlpha = .18 + pulse * .3; ctx.beginPath(); for (const [a, b] of edges) { ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); } ctx.stroke();
    for (const p of nodes) { ctx.globalAlpha = .4 + .6 * p.phi; ctx.fillStyle = p.phi > .6 ? '#fff' : hue; ctx.beginPath(); ctx.arc(p.x, p.y, p.r * (1 + 2 * p.phi) * (1 + pulse * .8), 0, Math.PI * 2); ctx.fill(); }
    ctx.globalAlpha = 1; pulse *= .95; requestAnimationFrame(tick); };
  tick(); return { chapter(c) { hue = COLS[c]; }, pulse() { pulse = Math.max(pulse, .8); } };
})();

const fromHash = () => { const h = location.hash.slice(1); if (!h) return 0; const k = S.findIndex((x) => x.id === h); return k >= 0 ? k : (parseInt(h) || 0); };
addEventListener('hashchange', () => go(fromHash()));
go(fromHash());
})();
