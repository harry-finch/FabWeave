"""Planche de variantes du logo FabWeave (skill fab-design-system, identite-et-logo.md).

Deuxième tour (24/10/2026) : deux cercles ondulés entrelacés, en bleu FabCAM
(les deux applications génèrent du G-code). Le premier tour (ondes, spirale,
treillis, buse ; quatre couleurs) est dans l'historique Git (commit 189f292).

Produit, dans ce dossier :
- `fabweave-<variante>.svg` : tuile commune de la famille + symbole d'une couleur ;
- `planche.html` : variantes à 128, 64, 48, 32, 24 et 16 px (rendu vectoriel natif
  à chaque taille), sur fond sombre et clair, en monochrome, et dans la barre
  d'application, avec la couleur et sa variante claire.

Un anneau ondulé suit r(θ) = R + A·sin(nθ + φ). Les bandes sont des polygones
denses aux coordonnées arrondies au centième ; aux croisements, le brin du
dessous est coupé net à distance `gap` du brin du dessus (dessus-dessous alternés).
Lancer :
    python3 branding/explorations/generate.py
"""

from __future__ import annotations

import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
TILE = (
    'M31 4 L97 4 C111.912 4 124 16.088 124 31 L124 97 C124 111.912 111.912 124 97 124 '
    'L31 124 C16.088 124 4 111.912 4 97 L4 31 C4 16.088 16.088 4 31 4 Z'
)
STEPS = 720  # points par anneau


def f(v: float) -> str:
    s = f"{v:.2f}".rstrip('0').rstrip('.')
    return '0' if s == '-0' else s


class Ring:
    """Anneau ondulé : centre, rayon moyen, amplitude, nombre d'ondes, phase (°)."""

    def __init__(self, cx: float, cy: float, r: float, amp: float, n: int, phase: float):
        self.cx, self.cy, self.r, self.amp, self.n = cx, cy, r, amp, n
        self.phase = math.radians(phase)

    def radius(self, th: float) -> float:
        return self.r + self.amp * math.sin(self.n * th + self.phase)

    def point(self, th: float) -> tuple[float, float]:
        rr = self.radius(th)
        return (self.cx + rr * math.cos(th), self.cy + rr * math.sin(th))

    def normal(self, th: float) -> tuple[float, float]:
        e = 1e-5
        (x0, y0), (x1, y1) = self.point(th - e), self.point(th + e)
        dx, dy = x1 - x0, y1 - y0
        n = math.hypot(dx, dy)
        return (dy / n, -dx / n)

    def samples(self, a: float, b: float) -> list[float]:
        k = max(2, int(abs(b - a) / (2 * math.pi) * STEPS))
        return [a + (b - a) * i / k for i in range(k + 1)]

    def polyline(self) -> list[tuple[float, float]]:
        return [self.point(th) for th in self.samples(0, 2 * math.pi)]


def offset(ring: Ring, ths: list[float], d: float) -> list[tuple[float, float]]:
    out = []
    for th in ths:
        (x, y), (nx, ny) = ring.point(th), ring.normal(th)
        out.append((x + nx * d, y + ny * d))
    return out


def poly(pts: list[tuple[float, float]]) -> str:
    return 'M ' + ' L '.join(f"{f(x)} {f(y)}" for x, y in pts) + ' Z'


def closed_band(ring: Ring, t: float) -> str:
    ths = ring.samples(0, 2 * math.pi)[:-1]
    # Contour extérieur et intérieur parcourus en sens opposés : l'intérieur est vide.
    return poly(offset(ring, ths, t / 2)) + ' ' + poly(list(reversed(offset(ring, ths, -t / 2))))


def open_band(ring: Ring, a: float, b: float, t: float) -> str:
    """Brin de θ = a à θ = b (a < b), bouts coupés net."""
    ths = ring.samples(a, b)
    return poly(offset(ring, ths, t / 2) + list(reversed(offset(ring, ths, -t / 2))))


def dist_to_polyline(p: tuple[float, float], line: list[tuple[float, float]]) -> float:
    best = math.inf
    px, py = p
    for (x0, y0), (x1, y1) in zip(line, line[1:]):
        if abs(x0 - px) > best + 2 and abs(x1 - px) > best + 2:
            continue
        dx, dy = x1 - x0, y1 - y0
        L = dx * dx + dy * dy
        u = 0.0 if L == 0 else max(0.0, min(1.0, ((px - x0) * dx + (py - y0) * dy) / L))
        best = min(best, math.hypot(px - x0 - u * dx, py - y0 - u * dy))
    return best


def crossings(a: Ring, b: Ring) -> list[float]:
    """Paramètres θ sur `a` où `a` traverse `b` (passe d'un côté à l'autre)."""

    def side(th: float) -> float:
        x, y = a.point(th)
        ang = math.atan2(y - b.cy, x - b.cx)
        return math.hypot(x - b.cx, y - b.cy) - b.radius(ang)

    # Décalé d'un demi-pas : un croisement ne tombe jamais pile sur un échantillon.
    half = math.pi / STEPS
    ths = [th + half for th in a.samples(0, 2 * math.pi)]
    out = []
    for t0, t1 in zip(ths, ths[1:]):
        s0, s1 = side(t0), side(t1)
        if s0 * s1 < 0:
            lo, hi = t0, t1
            for _ in range(40):
                mid = (lo + hi) / 2
                if side(lo) * side(mid) <= 0:
                    hi = mid
                else:
                    lo = mid
            out.append((lo + hi) / 2)
    return out


def theta_on(ring: Ring, p: tuple[float, float]) -> float:
    """θ du point de `ring` le plus proche de `p`."""
    th = math.atan2(p[1] - ring.cy, p[0] - ring.cx)
    cand = [th + (k - 300) * 0.001 for k in range(601)]
    return min(cand, key=lambda x: math.dist(ring.point(x), p))


def cut(under: Ring, th: float, over_line: list[tuple[float, float]], t: float, gap: float) -> tuple[float, float]:
    """Intervalle de θ retiré au brin du dessous autour du croisement `th` : ses
    bouts (bords et axe) restent à `gap` au moins du bord du brin du dessus."""
    need = t / 2 + gap

    def clear(x: float) -> bool:
        (px, py), (nx, ny) = under.point(x), under.normal(x)
        return all(dist_to_polyline((px + nx * s * t / 2, py + ny * s * t / 2), over_line) >= need for s in (-1, 0, 1))

    step = 0.004
    d0 = step
    while not clear(th - d0):
        d0 += step
    d1 = step
    while not clear(th + d1):
        d1 += step
    return (th - d0, th + d1)


def interlace(a: Ring, b: Ring, t: float, gap: float) -> list[tuple[str, str]]:
    """Deux anneaux entrelacés : dessus-dessous alternés le long de `a`."""
    ths_a = sorted(crossings(a, b))
    line_a, line_b = a.polyline(), b.polyline()
    cuts: dict[str, list[tuple[float, float]]] = {'a': [], 'b': []}
    for k, th in enumerate(ths_a):
        if k % 2 == 0:  # a dessus, b coupé
            cuts['b'].append(cut(b, theta_on(b, a.point(th)), line_a, t, gap))
        else:  # b dessus, a coupé
            cuts['a'].append(cut(a, th, line_b, t, gap))
    out = []
    for name, ring in (('a', a), ('b', b)):
        holes = sorted(cuts[name])
        if not holes:
            out.append((f"ring-{name}", closed_band(ring, t)))
            continue
        # Brins entre deux coupes, en faisant le tour (θ modulo 2π).
        for k, (_, e0) in enumerate(holes):
            s1 = holes[(k + 1) % len(holes)][0]
            if k + 1 == len(holes):
                s1 += 2 * math.pi
            out.append((f"ring-{name}-{k + 1}", open_band(ring, e0, s1, t)))
    return out


# --- Variantes ----------------------------------------------------------------


def v_maillons() -> list[tuple[str, str]]:
    """Deux anneaux ondulés passés l'un dans l'autre, comme deux maillons."""
    return interlace(Ring(47.5, 64, 25, 3.4, 6, 0), Ring(80.5, 64, 25, 3.4, 6, 180), t=8.5, gap=5)


def v_couronne4() -> list[tuple[str, str]]:
    """Deux anneaux ondulés concentriques en opposition de phase, tressés :
    deux couches successives du vase à ondulations vues de dessus (4 ondes)."""
    return interlace(Ring(64, 64, 30, 12, 4, 0), Ring(64, 64, 30, 12, 4, 180), t=8.5, gap=4.5)


def v_entrelacs3() -> list[tuple[str, str]]:
    """Même tresse à 3 ondes très marquées : un entrelacs en forme de nœud."""
    return interlace(Ring(64, 64, 29, 13, 3, 90), Ring(64, 64, 29, 13, 3, 270), t=9, gap=4.5)


VARIANTS = {
    'maillons': ('E — Maillons', 'Deux anneaux ondulés passés l’un dans l’autre.', v_maillons),
    'couronne-4': ('F — Couronne tressée, 4 ondes', 'Deux anneaux ondulés en opposition de phase, dessus-dessous : deux couches successives du vase vues de dessus.', v_couronne4),
    'entrelacs-3': ('G — Entrelacs, 3 ondes', 'La même tresse à 3 ondes très marquées : un nœud d’entrelacs.', v_entrelacs3),
}

BLUE = '#58B5F5'  # bleu de FabCAM : la famille des générateurs de G-code
ACCENT = '#4b9cf5'


def svg(variant: str, size: int | None = 128, mono: bool = False, shapes: list[tuple[str, str]] | None = None) -> str:
    tile_fill, tile_stroke, sym = ('#000000', '#ffffff', '#ffffff') if mono else ('#181f2a', '#354759', BLUE)
    attrs = f'width="{size}" height="{size}" ' if size else ''
    body = '\n    '.join(f'<path id="{i}" d="{d}"/>' for i, d in (shapes or VARIANTS[variant][2]()))
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" {attrs}viewBox="0 0 128 128">\n'
        f'  <title>FabWeave — variante {variant}</title>\n'
        f'  <path d="{TILE}" fill="{tile_fill}" stroke="{tile_stroke}" stroke-width="3.6"/>\n'
        f'  <g id="symbol" fill="{sym}">\n    {body}\n  </g>\n</svg>\n'
    )


# --- Couleur : contraste et variante claire -------------------------------------


def rgb(hexa: str) -> tuple[float, float, float]:
    h = hexa.lstrip('#')
    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))  # type: ignore[return-value]


def lum(hexa: str) -> float:
    def lin(c: float) -> float:
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(c) for c in rgb(hexa))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a: str, b: str) -> float:
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def darker(hexa: str) -> str:
    """Variante du thème clair : même teinte, assombrie par pas de 1 % jusqu'à
    un contraste de 3:1 sur la barre claire (#e6e6e6), et d'au moins 12 %."""
    r, g, b = rgb(hexa)
    k = 0.88
    c = hexa
    while k > 0.2:
        c = '#{:02X}{:02X}{:02X}'.format(round(r * k * 255), round(g * k * 255), round(b * k * 255))
        if contrast(c, '#e6e6e6') >= 3.0:
            break
        k -= 0.01
    return c


# --- Planche --------------------------------------------------------------------

SIZES = [128, 64, 48, 32, 24, 16]


def main() -> None:
    shapes = {v: fn() for v, (_, _, fn) in VARIANTS.items()}
    for old in HERE.glob('fabweave-*.svg'):
        old.unlink()
    for v in VARIANTS:
        (HERE / f'fabweave-{v}.svg').write_text(svg(v, None, shapes=shapes[v]))
    light = darker(BLUE)

    def tile_row(v: str, bg: str, mono: bool = False) -> str:
        cells = ''.join(f'<div class="cell"><div>{svg(v, s, mono, shapes[v])}</div><span>{s}</span></div>' for s in SIZES)
        return f'<div class="row" style="background:{bg}">{cells}</div>'

    def appbar(v: str, is_light: bool) -> str:
        bar, text, muted, edge = ('#e6e6e6', '#1e1e1e', '#636363', light) if is_light else ('#181818', '#e8e8e8', '#7d7d7d', BLUE)
        button = '#1473e6' if is_light else ACCENT
        return (
            f'<div class="appbar" style="background:{bar};color:{text}"><i style="background:{edge}"></i>'
            f'{svg(v, 24, shapes=shapes[v])}<b>FabWeave</b><span style="color:{muted}">G-code direct, modèles paramétriques</span>'
            f'<em style="background:{button}">Exporter le G-code</em></div>'
        )

    sections = []
    for v, (title, desc, _) in VARIANTS.items():
        sections.append(
            f'<section><h2>{title}</h2><p>{desc}</p>'
            f'{tile_row(v, "#232323")}{tile_row(v, "#f8f8f8")}{tile_row(v, "#808080", True)}'
            f'<div class="bars">{appbar(v, False)}{appbar(v, True)}</div></section>'
        )
    sections.append(
        '<section><h2>Couleur : bleu FabCAM</h2>'
        f'<p><span class="sw" style="background:{BLUE}"></span><code>{BLUE}</code> (thème sombre, symbole et liseré) · '
        f'<span class="sw" style="background:{light}"></span><code>{light}</code> (liseré du thème clair). '
        f'Contraste du symbole sur la tuile : {contrast(BLUE, "#181f2a"):.1f}:1 ; liseré clair sur la barre claire : '
        f'{contrast(light, "#e6e6e6"):.1f}:1. Proche du bleu d’action <span class="sw" style="background:{ACCENT}"></span>'
        f'<code>{ACCENT}</code> (bouton « Exporter » des barres ci-dessus) : acceptable car l’identité ne sert '
        'qu’au logo et au liseré, jamais à un état ni à une action.</p></section>'
    )
    html = f'''<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>FabWeave — planche de logos</title>
<style>
  body {{ margin: 0; padding: 24px; background: #1c1c1c; color: #e8e8e8; font: 14px/1.45 -apple-system, "Inter", system-ui, sans-serif; }}
  h1 {{ font-size: 20px; margin: 0 0 4px; }} h2 {{ font-size: 15px; margin: 0 0 2px; }}
  p {{ color: #ababab; margin: 0 0 10px; max-width: 780px; }}
  section {{ margin: 0 0 28px; padding: 16px; background: #232323; border: 1px solid #343434; border-radius: 6px; }}
  .row {{ display: flex; align-items: flex-end; gap: 22px; padding: 12px 14px; border-radius: 4px; margin-bottom: 6px; overflow-x: auto; }}
  .cell {{ display: flex; flex-direction: column; align-items: center; gap: 4px; }}
  .cell svg {{ display: block; }} .cell span {{ font-size: 11px; color: #7d7d7d; }}
  .bars {{ display: grid; gap: 6px; margin-top: 8px; }}
  .appbar {{ position: relative; display: flex; align-items: center; gap: 8px; height: 42px; padding: 0 12px; border-radius: 4px; overflow: hidden; }}
  .appbar i {{ position: absolute; inset: 0 0 auto 0; height: 2px; opacity: .85; }}
  .appbar b {{ font-size: 15px; letter-spacing: -0.01em; }} .appbar span {{ font-size: 12px; }}
  .appbar em {{ margin-left: auto; font-style: normal; color: #fff; font-size: 12px; font-weight: 500; padding: 5px 12px; border-radius: 4px; }}
  .sw {{ display: inline-block; width: 14px; height: 14px; border-radius: 3px; margin: 0 4px 0 0; vertical-align: -2px; }}
  code {{ font-size: 12px; color: #d0d0d0; }}
</style></head><body>
<h1>FabWeave — planche de logos (2<sup>e</sup> tour)</h1>
<p>Deux cercles ondulés entrelacés, bleu FabCAM. Tuile graphite commune à la famille, symbole d’une seule couleur.
Chaque variante de 128 à 16 px (rendu vectoriel natif à chaque taille) sur panneau sombre, panneau clair, en monochrome,
puis dans la barre d’application (logo 24 px, liseré 2 px). Généré par <code>branding/explorations/generate.py</code>.</p>
{''.join(sections)}
</body></html>
'''
    (HERE / 'planche.html').write_text(html)
    print(f"bleu {BLUE} / {light} ; tuile {contrast(BLUE, '#181f2a'):.1f}:1 ; clair {contrast(light, '#e6e6e6'):.1f}:1")
    for v, s in shapes.items():
        print(v, len(s), 'formes')


if __name__ == '__main__':
    main()
