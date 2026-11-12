/** A printed lattice: alternating rounded strands, with an extrusion nozzle above it.
 * Picking a resting layer opens the stack above it; no hit area moves with the drawing. */
const {
  Cam, clamp, circ, facing, fit, hull, mk, open, poly, prism, proj,
  put, ringAt, rings, run, solid, tween, tset, tval, tdone, disposer, pointer, register,
} = HL;

function mount({ stage, svg, read }, value) {
  const bag = disposer(), N = 6, pitch = 7, thick = 4;
  let gap = value, active = -1;
  const C = Cam(45, 0.5, 2.05);
  fit(C, [[-43, -35, 0], [43, 35, 0], [-43, 35, 0], [43, -35, 0], [14, 4, 118]], 200, 166);
  const P = proj(C), front = facing(C), root = mk('g', {}, svg);
  const [base, inset] = rings(-43, -35, 43, 35, 5, 1.4);
  put(solid(root), prism(P, front, base, inset, 0, 3));
  const layers = [];
  for (let i = 0; i < N; i++) {
    const group = mk('g', {}, root), strands = [];
    for (let j = 0; j < 4; j++) {
      const q = -25 + j * 16.5;
      const rr = i % 2 ? rings(q - 3, -30, q + 3, 30, 3, 0.9)
        : rings(-38, q - 3, 38, q + 3, 3, 0.9);
      strands.push({ shape: rr, el: solid(group) });
    }
    layers.push({ group, strands, lift: tween(0), last: null });
  }
  // Nozzle: short cylindrical collar, rounded heater, taper and tiny flat tip.
  const nozzle = mk('g', {}, root), cap = solid(nozzle), heater = solid(nozzle), tip = solid(nozzle);
  const bore = mk('path', { class: 'nf lo' }, nozzle);
  const nz = tween(0);
  const shifted = (r, n) => circ(r, n).map(q => ({ ...q, u: q.u + 14, v: q.v + 4 }));
  const body = shifted(10, 48), bodyInner = shifted(8.8, 48);
  const [hex, hexInner] = rings(0, -10, 28, 18, 3, 1.5);
  const foot = shifted(2.5, 32), upper = shifted(11, 48), upperInner = shifted(9.8, 48);
  let nozzleLast = null;
  function drawNozzle(lift) {
    if (lift === nozzleLast) return;
    nozzleLast = lift;
    const z = 53 + lift;
    put(tip, { sil: poly(hull(ringAt(P, foot, z).concat(ringAt(P, upper, z + 19)))),
      crease: open(ringAt(P, run(upperInner, front), z + 18)) });
    put(heater, prism(P, front, hex, hexInner, z + 19, z + 29));
    put(cap, prism(P, front, body, bodyInner, z + 29, z + 41));
    // Painter order: taper behind heater behind collar, all above the printed part.
    nozzle.append(tip.g, heater.g, cap.g, bore);
    bore.setAttribute('d', open(ringAt(P, run(shifted(2, 24), front), z)));
  }
  const B = register(stage, (_dt, now) => {
    let moving = false;
    layers.forEach((layer, i) => {
      const lift = tval(layer.lift, now);
      if (layer.last !== lift) {
        layer.last = lift;
        const z = 4 + i * pitch + lift;
        layer.strands.forEach(s => put(s.el, prism(P, front, ...s.shape, z, z + thick)));
      }
      if (!tdone(layer.lift, now)) moving = true;
    });
    drawNozzle(tval(nz, now));
    return moving || !tdone(nz, now);
  });
  bag.add(B.unregister);
  function choose(a, force = false) {
    if (a === active && !force) return;
    const origin = a >= 0 ? a : Math.max(0, active), now = performance.now();
    active = a;
    layers.forEach((layer, i) => {
      tset(layer.lift, a < 0 || i < a ? 0 : gap * (1 + (i - a) * 0.13), now, Math.abs(i - origin) * 40);
      layer.strands.forEach(s => s.el.sil.classList.toggle('hi', i === (a < 0 ? N - 1 : a)));
    });
    tset(nz, a < 0 ? 0 : gap * (1 + (N - 1 - a) * 0.13), now, Math.abs(N - origin) * 40);
    read.textContent = a < 0 ? 'rest' : `couche ${a + 1}`;
    B.wake();
  }
  // Static front-edge bands: a raised layer never moves its own hit target.
  const edge = layers.map((_, i) => P(28, 22, 6 + i * pitch));
  function hit([x, y]) {
    if (Math.abs(x - edge[0][0]) > 92 || y > edge[0][1] + 12 || y < edge[N - 1][1] - 18) return -1;
    let best = 0;
    for (let i = 1; i < N; i++) if (Math.abs(y - edge[i][1]) < Math.abs(y - edge[best][1])) best = i;
    return best;
  }
  bag.add(pointer(stage, { move: p => choose(hit(p)), leave: () => choose(-1) }));
  bag.add(() => svg.replaceChildren());
  choose(-1, true);
  return { set: v => { gap = clamp(v, 8, 20); choose(active, true); }, destroy: bag.dispose };
}

hairline({
  name: 'woven-nozzle',
  means: 'Une buse au-dessus d’une pièce tissée ; le pointeur ouvre les couches pour montrer le parcours.',
  rules: [1, 2, 4, 8, 9],
  range: [8, 14, 20],
  tour: [[209, 262], [209, 237], [209, 199], null],
  mount,
});
