/** A hollow fluted vase under a nozzle. Pointer x controls the wave amplitude. */
const {
  Cam, clamp, circ, facing, fit, hull, mk, open, poly, prism, proj,
  put, ringAt, rings, run, solid, spring, stepS, disposer, pointer, register,
} = HL;

function mount({ stage, svg, read }, value) {
  const bag = disposer(), N = 13, H = 78;
  let reach = value, last = null, pointerX = null;
  const amplitude = spring(3), C = Cam(45, 0.5, 1.95);
  fit(C, [[-43, -43, 0], [43, 43, 0], [-43, 43, 0], [43, -43, 0], [25, 0, 123]], 200, 166);
  const P = proj(C), front = facing(C), root = mk('g', {}, svg);
  const bands = Array.from({ length: N }, () => solid(root));
  const mouth = mk('path', { class: 'sil hi' }, root);
  const inside = mk('path', { class: 'lo' }, root);
  const lip = mk('path', { class: 'nf lo' }, root);
  const nozzle = mk('g', {}, root);
  const tip = solid(nozzle), heater = solid(nozzle), collar = solid(nozzle);
  const shifted = r => circ(r, 64).map(q => ({ ...q, u: q.u + 25 }));
  put(tip, { sil: poly(hull(ringAt(P, shifted(1.8), 84).concat(ringAt(P, shifted(9), 100)))),
    crease: open(ringAt(P, run(shifted(8), front), 99)) });
  const [block, inset] = rings(14, -11, 36, 11, 3, 1.2);
  put(heater, prism(P, front, block, inset, 100, 109));
  put(collar, prism(P, front, shifted(7), shifted(6), 109, 123));

  function ring(t, amp, inset = 0) {
    const radius = 24 + 13 * Math.sin(Math.PI * t) - 5 * Math.sin(Math.PI * t / 2) + 6 * t ** 8;
    return Array.from({ length: 144 }, (_, k) => {
      const a = k * Math.PI * 2 / 144;
      const r = radius + amp * Math.sin(6 * a + t * 0.8) - inset;
      return { u: r * Math.cos(a), v: r * Math.sin(a), nu: Math.cos(a), nv: Math.sin(a) };
    });
  }
  function draw(amp) {
    if (amp === last) return;
    last = amp;
    // Opaque bands from the foot to the mouth hide lower rear edges.
    bands.forEach((band, i) => {
      const lo = ring(i / N, amp), hi = ring((i + 1) / N, amp);
      const z0 = i * H / N, z1 = (i + 1) * H / N;
      put(band, { sil: poly(hull(ringAt(P, lo, z0).concat(ringAt(P, hi, z1)))),
        crease: open(ringAt(P, run(ring((i + 1) / N, amp, 0.8), front), z1 - 0.6)) });
    });
    mouth.setAttribute('d', poly(ringAt(P, ring(1, amp), H)));
    inside.setAttribute('d', poly(ringAt(P, ring(1, amp, 2.2), H - 1.3)));
    lip.setAttribute('d', open(ringAt(P, run(ring(1, amp, 2.2), front), H - 1.3)));
  }
  const B = register(stage, dt => {
    const moving = stepS(amplitude, dt);
    draw(amplitude.x);
    return moving;
  });
  bag.add(B.unregister);
  function target() {
    amplitude.t = pointerX === null ? 3 : 1.2 + clamp((pointerX - 105) / 190, 0, 1) * reach;
    read.textContent = pointerX === null ? 'rest' : `relief ${Math.round(amplitude.t * 10) / 10}`;
    B.wake();
  }
  bag.add(pointer(stage, {
    move: ([x, y]) => { pointerX = x >= 100 && x <= 300 && y >= 95 && y <= 288 ? x : null; target(); },
    leave: () => { pointerX = null; target(); },
  }));
  bag.add(() => svg.replaceChildren());
  read.textContent = 'rest';
  B.wake();
  return { set: v => { reach = clamp(v, 4, 8); target(); }, destroy: bag.dispose };
}

hairline({
  name: 'ripple-vase',
  means: 'Un vase ondulé sous une buse : le pointeur règle le relief de sa paroi.',
  rules: [1, 3, 4, 5, 8, 9],
  range: [4, 6, 8],
  tour: [[125, 205], [200, 175], [275, 205], null],
  mount,
});
