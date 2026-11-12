/**
 * The light stream (web ADR 0141, shared with the launcher's Launchpad hero): additive orange →
 * amber → warm-yellow filaments, dust and bokeh on a canvas (full screen on the website). Its shape blends four presets (hero beam, vertical wave, soft aurora,
 * wide band) through `uW`; the host component writes the weights from the scroll position.
 * Loaded lazily (own chunk); low tier = fewer filaments, no dust/bokeh, DPR capped at 1.
 * Every filament is a ribbon along a smooth curve (finely tessellated, normals measured with the
 * screen's aspect ratio so the width is even in pixels); the render loop sleeps while paused,
 * hidden or faded out.
 */
import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  DoubleSide,
  InstancedBufferAttribute,
  InstancedBufferGeometry,
  Mesh,
  OrthographicCamera,
  Points,
  Scene,
  ShaderMaterial,
  Vector2,
  Vector4,
  WebGLRenderer,
} from 'three';

export interface LightStreamHandle {
  /**
   * Preset weights (hero, sources, soft, cloud), summing to 1, the overall intensity, and the
   * scroll position in viewport heights (the shapes move with it, eased).
   */
  setTarget(weights: [number, number, number, number], intensity: number, scroll: number): void;
  pause(paused: boolean): void;
  dispose(): void;
}

// Ashima Arts 3D simplex noise (MIT), compact form.
const NOISE = /* glsl */ `
vec3 mod289(vec3 x){return x-floor(x*(1.0/289.0))*289.0;}
vec4 mod289(vec4 x){return x-floor(x*(1.0/289.0))*289.0;}
vec4 permute(vec4 x){return mod289(((x*34.0)+1.0)*x);}
vec4 taylorInvSqrt(vec4 r){return 1.79284291400159-0.85373472095314*r;}
float snoise(vec3 v){
  const vec2 C=vec2(1.0/6.0,1.0/3.0); const vec4 D=vec4(0.0,0.5,1.0,2.0);
  vec3 i=floor(v+dot(v,C.yyy)); vec3 x0=v-i+dot(i,C.xxx);
  vec3 g=step(x0.yzx,x0.xyz); vec3 l=1.0-g; vec3 i1=min(g.xyz,l.zxy); vec3 i2=max(g.xyz,l.zxy);
  vec3 x1=x0-i1+C.xxx; vec3 x2=x0-i2+C.yyy; vec3 x3=x0-D.yyy;
  i=mod289(i);
  vec4 p=permute(permute(permute(i.z+vec4(0.0,i1.z,i2.z,1.0))+i.y+vec4(0.0,i1.y,i2.y,1.0))+i.x+vec4(0.0,i1.x,i2.x,1.0));
  float n_=0.142857142857; vec3 ns=n_*D.wyz-D.xzx;
  vec4 j=p-49.0*floor(p*ns.z*ns.z); vec4 x_=floor(j*ns.z); vec4 y_=floor(j-7.0*x_);
  vec4 x=x_*ns.x+ns.yyyy; vec4 y=y_*ns.x+ns.yyyy; vec4 h=1.0-abs(x)-abs(y);
  vec4 b0=vec4(x.xy,y.xy); vec4 b1=vec4(x.zw,y.zw);
  vec4 s0=floor(b0)*2.0+1.0; vec4 s1=floor(b1)*2.0+1.0; vec4 sh=-step(h,vec4(0.0));
  vec4 a0=b0.xzyw+s0.xzyw*sh.xxyy; vec4 a1=b1.xzyw+s1.xzyw*sh.zzww;
  vec3 p0=vec3(a0.xy,h.x); vec3 p1=vec3(a0.zw,h.y); vec3 p2=vec3(a1.xy,h.z); vec3 p3=vec3(a1.zw,h.w);
  vec4 norm=taylorInvSqrt(vec4(dot(p0,p0),dot(p1,p1),dot(p2,p2),dot(p3,p3)));
  p0*=norm.x; p1*=norm.y; p2*=norm.z; p3*=norm.w;
  vec4 m=max(0.6-vec4(dot(x0,x0),dot(x1,x1),dot(x2,x2),dot(x3,x3)),0.0); m=m*m;
  return 42.0*dot(m*m,vec4(dot(p0,x0),dot(p1,x1),dot(p2,x2),dot(p3,x3)));
}`;

/**
 * The hero beam's spine: a cubic Bézier in normalised screen coordinates (x and y in -1..1, x
 * scaled by the aspect ratio before any distance is measured). One smooth curve, no corners:
 * the earlier polyline through a "node" drew a visible kink. Exported for the tessellation test.
 */
export const HERO_PATH: readonly [number, number][] = [
  [-0.32, -1.38],
  [0.12, -0.5],
  [0.42, 0.22],
  [1.42, 0.98],
];
/** Where along the hero spine the filaments gather (and fan out on both sides). */
export const HERO_NODE = 0.5;

/** A point of a cubic Bézier (same formula as the shader's `bez`). */
export function bezierPoint(p: readonly [number, number][], t: number): [number, number] {
  const u = 1 - t;
  const a = u * u * u;
  const b = 3 * u * u * t;
  const c = 3 * u * t * t;
  const d = t * t * t;
  return [
    a * p[0]![0] + b * p[1]![0] + c * p[2]![0] + d * p[3]![0],
    a * p[0]![1] + b * p[1]![1] + c * p[2]![1] + d * p[3]![1],
  ];
}

/**
 * The largest turn (degrees) between two consecutive segments when the spine is cut into `seg`
 * straight pieces on a screen of the given aspect ratio: what the eye reads as a corner.
 */
export function maxTurnDegrees(seg: number, aspect = 16 / 9, p = HERO_PATH): number {
  let max = 0;
  let prev: [number, number] | null = null;
  let last = bezierPoint(p, 0);
  for (let i = 1; i <= seg; i++) {
    const q = bezierPoint(p, i / seg);
    const d: [number, number] = [(q[0] - last[0]) * aspect, q[1] - last[1]];
    if (prev) {
      const dot = (prev[0] * d[0] + prev[1] * d[1]) / (Math.hypot(...prev) * Math.hypot(...d));
      max = Math.max(max, (Math.acos(Math.min(1, Math.max(-1, dot))) * 180) / Math.PI);
    }
    prev = d;
    last = q;
  }
  return max;
}

/**
 * Segments per filament (desktop / low tier). The spine is gentle: 72 pieces already turn less
 * than 0.5° per joint, invisible on a glowing ribbon. More only costs vertex work (each vertex
 * evaluates the shape, noise included, twice), which software GL and the launcher's hero card pay
 * on the main thread: 240 made the launcher's recorded launch journey stall.
 */
export const SEGMENTS = { high: 120, low: 72 } as const;

const FILAMENT_VS = /* glsl */ `
uniform float uTime; uniform vec4 uW; uniform float uIntensity; uniform float uScroll;
uniform float uAspect; uniform float uPx; uniform vec2 uHero[4];
attribute vec4 aSeed;
varying float vSide; varying float vAlpha; varying float vHeat;
${NOISE}
const float NODE = ${HERO_NODE.toFixed(3)};
vec2 bez(float t){
  float u = 1.0 - t;
  return u*u*u*uHero[0] + 3.0*u*u*t*uHero[1] + 3.0*u*t*t*uHero[2] + t*t*t*uHero[3];
}
vec2 bezD(float t){
  float u = 1.0 - t;
  return 3.0*u*u*(uHero[1]-uHero[0]) + 6.0*u*t*(uHero[2]-uHero[1]) + 3.0*t*t*(uHero[3]-uHero[2]);
}
// Every shape is a smooth function of t (no piecewise parts), so any blend of them is smooth too.
// Positions are in normalised screen units; uScroll is the scroll position in viewport heights.
vec2 hero(float t, vec4 s){
  vec2 p = bez(t);
  vec2 d = bezD(t); d.x *= uAspect;
  vec2 n = normalize(vec2(-d.y, d.x)); n.x /= uAspect;
  float k = t - NODE;
  float spread = 0.03 + 1.25 * k * k;
  p += n * (s.x - 0.5) * spread;
  p += n * snoise(vec3(t * 2.0 + s.y * 9.0, uTime * 0.07 + uScroll * 0.35, s.z * 3.0)) * (0.012 + 0.08 * k * k);
  // The beam drifts up with the page (parallax) as the hero scrolls away.
  p.y += uScroll * 0.9;
  return p;
}
vec2 wave(float t, vec4 s){
  float y = mix(-1.3, 1.3, t);
  // Anchored to the page: the wave travels with the content (0.7 parallax), and breathes slowly.
  float ph = (y - uScroll * 1.4) * 2.6;
  float x = -0.04 + sin(ph + uTime * 0.35 + s.y * 0.9) * 0.1 + sin(ph * 0.43 + 1.7) * 0.045;
  x += (s.x - 0.5) * (0.07 + 0.05 * sin(ph * 0.7 + s.z * 6.28));
  x += snoise(vec3(y * 1.2, uTime * 0.05 + uScroll * 0.2, s.z * 5.0)) * 0.015;
  return vec2(x, y);
}
vec2 aurora(float t, vec4 s){
  float x = mix(-1.45, 1.45, t);
  float ph = uScroll * 0.9;
  float y = -0.05 + sin(t * 2.6 + uTime * 0.1 + ph + s.y * 6.28) * 0.2 + (s.x - 0.5) * 0.95;
  y += snoise(vec3(t * 1.3, uTime * 0.04 + ph * 0.3, s.z * 4.0)) * 0.2;
  return vec2(x, y);
}
vec2 band(float t, vec4 s){
  float x = mix(-1.45, 1.45, t);
  float y = -0.02 + (s.x - 0.5) * 0.11 + sin(t * 5.0 + uTime * 0.3 + uScroll * 0.8 + s.y * 6.28) * 0.014;
  return vec2(x, y);
}
vec2 shape(float t, vec4 s){
  // Uniform branches: presets with no weight cost nothing.
  vec2 p = vec2(0.0);
  if (uW.x > 0.001) p += uW.x * hero(t, s);
  if (uW.y > 0.001) p += uW.y * wave(t, s);
  if (uW.z > 0.001) p += uW.z * aurora(t, s);
  if (uW.w > 0.001) p += uW.w * band(t, s);
  return p;
}
void main(){
  float t = position.x; float side = position.y;
  vec2 p = shape(t, aSeed);
  // Forward difference in aspect-correct space: the ribbon's normal, so its width is the same
  // in every direction (and in pixels, uPx = one CSS pixel in normalised units). Two shape
  // evaluations per vertex, not three: the vertex shader is the light's whole cost.
  vec2 b = shape(t + 0.004, aSeed);
  vec2 tang = (b - p); tang.x *= uAspect;
  tang = normalize(tang + vec2(1e-6));
  vec2 nrm = vec2(-tang.y, tang.x);
  float px = mix(2.2, 6.0, aSeed.w) * (1.0 + uW.z * 2.5);
  vec2 off = nrm * side * px * uPx;
  off.x /= uAspect;
  gl_Position = vec4(p + off, 0.0, 1.0);
  vSide = side;
  // smoothstep needs edge0 < edge1 (reversed edges are undefined in GLSL: 0 on some drivers).
  float ends = smoothstep(0.0, 0.12, t) * (1.0 - smoothstep(0.88, 1.0, t));
  // Where the hero filaments gather they overlap: dim each one there so the sum stays even.
  float gather = uW.x * exp(-pow((t - NODE) * 3.2, 2.0));
  vAlpha = ends * uIntensity * mix(0.14, 0.5, aSeed.z) * (1.0 - uW.z * 0.45) * (1.0 - 0.6 * gather);
  vHeat = aSeed.y * 0.85 + gather * 0.55 + uW.w * 0.25;
}`;

const FILAMENT_FS = /* glsl */ `
varying float vSide; varying float vAlpha; varying float vHeat;
void main(){
  // A soft gaussian across the ribbon (zero at its edges) plus a thin hot core.
  float d = vSide * vSide;
  float glow = max(0.0, exp(-d * 3.6) - 0.027);
  float core = exp(-d * 28.0);
  vec3 orange = vec3(1.0, 0.416, 0.0);
  vec3 amber = vec3(1.0, 0.69, 0.125);
  vec3 warm = vec3(1.0, 0.9, 0.6);
  vec3 c = mix(orange, amber, clamp(vHeat, 0.0, 1.0));
  c = mix(c, warm, clamp(vHeat - 1.0, 0.0, 1.0) + core * 0.25);
  gl_FragColor = vec4(c, (glow * 0.34 + core * 0.62) * vAlpha);
}`;

const DUST_VS = /* glsl */ `
uniform float uTime; uniform float uIntensity; uniform float uPixel; uniform float uScroll;
attribute vec4 aSeed;
varying float vAlpha; varying float vBokeh;
${NOISE}
void main(){
  vec2 p = position.xy;
  // Drifts up slowly and with the scroll (a little parallax), wrapping off screen.
  p.y = mod(p.y + 1.2 + uTime * (0.006 + aSeed.x * 0.01) + uScroll * (0.25 + aSeed.x * 0.3), 2.4) - 1.2;
  p += vec2(snoise(vec3(p * 0.8, uTime * 0.05 + aSeed.z)), snoise(vec3(p * 0.8 + 7.0, uTime * 0.05))) * 0.03;
  gl_Position = vec4(p, 0.0, 1.0);
  vBokeh = step(0.985, aSeed.w);
  gl_PointSize = mix(mix(1.0, 2.6, aSeed.y), mix(28.0, 70.0, aSeed.y), vBokeh) * uPixel;
  vAlpha = uIntensity * mix(mix(0.12, 0.45, aSeed.z), 0.07, vBokeh);
}`;

const DUST_FS = /* glsl */ `
varying float vAlpha; varying float vBokeh;
void main(){
  vec2 d = gl_PointCoord - 0.5;
  float r = length(d);
  float soft = 1.0 - smoothstep(0.0, 0.5, r);
  float disc = 1.0 - smoothstep(0.38, 0.5, r);
  float a = mix(soft, disc * 0.8 + soft * 0.2, vBokeh);
  gl_FragColor = vec4(mix(vec3(1.0, 0.72, 0.35), vec3(1.0, 0.58, 0.2), vBokeh), a * vAlpha);
}`;

/** Software GL (SwiftShader, llvmpipe) or few cores: the low tier. */
export function isLowTier(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
  narrow: boolean,
): boolean {
  const info = gl.getExtension('WEBGL_debug_renderer_info');
  const renderer = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : '';
  return (
    narrow ||
    /swiftshader|llvmpipe|software/i.test(renderer) ||
    (navigator.hardwareConcurrency ?? 8) <= 4
  );
}

export interface LightStreamOptions {
  /** Keeps the drawing buffer and exposes `window.__rusticLight` (dev tools, screenshots). */
  debug?: boolean;
}

export function createLightStream(
  canvas: HTMLCanvasElement,
  narrow: boolean,
  options: LightStreamOptions = {},
): LightStreamHandle {
  const renderer = new WebGLRenderer({
    canvas,
    alpha: true,
    antialias: false,
    powerPreference: 'high-performance',
    preserveDrawingBuffer: options.debug === true,
  });
  const low = isLowTier(renderer.getContext(), narrow);
  const dpr = Math.min(window.devicePixelRatio || 1, low ? 1 : 1.5);
  renderer.setPixelRatio(dpr);
  renderer.setClearColor(0x000000, 0);
  const scene = new Scene();
  const camera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);

  const weights = new Vector4(1, 0, 0, 0);
  const target = { w: new Vector4(1, 0, 0, 0), intensity: 1, scroll: 0 };
  const uniforms = {
    uTime: { value: 0 },
    uW: { value: weights },
    uIntensity: { value: 0 },
    uScroll: { value: 0 },
    uPixel: { value: dpr },
    uAspect: { value: 16 / 9 },
    uPx: { value: 2 / 900 },
    uHero: { value: HERO_PATH.map(([x, y]) => new Vector2(x, y)) },
  };

  // Filaments: one instanced ribbon strip, finely cut so the curves never show a corner.
  const SEG = low ? SEGMENTS.low : SEGMENTS.high;
  const N = low ? 54 : 110;
  const strip = new InstancedBufferGeometry();
  const pos = new Float32Array((SEG + 1) * 2 * 3);
  const idx: number[] = [];
  for (let i = 0; i <= SEG; i++) {
    const t = i / SEG;
    pos.set([t, -1, 0, t, 1, 0], i * 6);
    if (i < SEG) {
      const a = i * 2;
      idx.push(a, a + 1, a + 2, a + 1, a + 3, a + 2);
    }
  }
  strip.setAttribute('position', new BufferAttribute(pos, 3));
  strip.setIndex(idx);
  const seeds = new Float32Array(N * 4);
  for (let i = 0; i < seeds.length; i++) seeds[i] = Math.random();
  strip.setAttribute('aSeed', new InstancedBufferAttribute(seeds, 4));
  strip.instanceCount = N;
  const filaments = new Mesh(
    strip,
    new ShaderMaterial({
      uniforms,
      vertexShader: FILAMENT_VS,
      fragmentShader: FILAMENT_FS,
      // The ribbons' winding follows the curve: never cull them.
      side: DoubleSide,
      transparent: true,
      depthWrite: false,
      depthTest: false,
      blending: AdditiveBlending,
    }),
  );
  filaments.frustumCulled = false;
  scene.add(filaments);

  if (!low) {
    const D = 420;
    const g = new BufferGeometry();
    const p = new Float32Array(D * 3);
    const s = new Float32Array(D * 4);
    for (let i = 0; i < D; i++) {
      p.set([Math.random() * 2.4 - 1.2, Math.random() * 2.4 - 1.2, 0], i * 3);
      s.set([Math.random(), Math.random(), Math.random(), Math.random()], i * 4);
    }
    g.setAttribute('position', new BufferAttribute(p, 3));
    g.setAttribute('aSeed', new BufferAttribute(s, 4));
    const dust = new Points(
      g,
      new ShaderMaterial({
        uniforms,
        vertexShader: DUST_VS,
        fragmentShader: DUST_FS,
        transparent: true,
        depthWrite: false,
        depthTest: false,
        blending: AdditiveBlending,
      }),
    );
    dust.frustumCulled = false;
    scene.add(dust);
  }

  if (options.debug) {
    (window as unknown as { __rusticLight?: unknown }).__rusticLight = {
      renderer,
      uniforms,
      target,
      low,
    };
  }

  // The canvas' own box (full screen on the website, the hero card in the launcher).
  const resize = () => {
    const w = canvas.clientWidth || window.innerWidth;
    const h = canvas.clientHeight || window.innerHeight;
    renderer.setSize(w, h, false);
    uniforms.uAspect.value = w / Math.max(1, h);
    uniforms.uPx.value = 2 / Math.max(1, h);
    // A resize clears the drawing buffer: draw again even if the loop is asleep.
    if (!running) draw();
  };

  let raf = 0;
  let paused = false;
  let running = false;
  let last = performance.now();
  const draw = () => renderer.render(scene, camera);
  // The loop sleeps (no requestAnimationFrame at all) while paused, while the tab is hidden and
  // once the light has faded out; `wake()` restarts it.
  const frame = () => {
    if (paused || document.hidden) {
      running = false;
      return;
    }
    // performance.now(), not the rAF timestamp (it can lag behind and go negative), clamped.
    const now = performance.now();
    const dt = Math.min(0.05, Math.max(0, (now - last) / 1000));
    last = now;
    uniforms.uTime.value += dt;
    // Frame-rate independent easing; the scroll follows faster (Lenis already smooths it).
    const k = 1 - Math.pow(0.0015, dt);
    const ks = 1 - Math.pow(0.000002, dt);
    weights.lerp(target.w, k);
    uniforms.uIntensity.value += (target.intensity - uniforms.uIntensity.value) * k;
    uniforms.uScroll.value += (target.scroll - uniforms.uScroll.value) * ks;
    if (uniforms.uIntensity.value < 0.003 && target.intensity === 0) {
      uniforms.uIntensity.value = 0;
      renderer.clear();
      running = false;
      return;
    }
    draw();
    raf = requestAnimationFrame(frame);
  };
  const wake = () => {
    if (running || paused || document.hidden) return;
    running = true;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  };
  resize();
  window.addEventListener('resize', resize);
  const observer = typeof ResizeObserver === 'function' ? new ResizeObserver(resize) : null;
  observer?.observe(canvas);
  wake();
  const onVisibility = () => wake();
  document.addEventListener('visibilitychange', onVisibility);

  return {
    setTarget(w, intensity, scroll) {
      target.w.set(w[0], w[1], w[2], w[3]);
      target.intensity = intensity;
      target.scroll = scroll;
      wake();
    },
    pause(p) {
      paused = p;
      wake();
    },
    dispose() {
      cancelAnimationFrame(raf);
      window.removeEventListener('resize', resize);
      observer?.disconnect();
      document.removeEventListener('visibilitychange', onVisibility);
      scene.traverse((o) => {
        const m = o as Mesh;
        m.geometry?.dispose();
        (m.material as ShaderMaterial | undefined)?.dispose?.();
      });
      renderer.dispose();
    },
  };
}
