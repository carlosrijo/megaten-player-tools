'use strict';
// The assistant: a demon from the game standing in the window's corner (Black Frost).
// The host makes one JSON file out of the client's own model files (src\Build\DemonModel.cs): nodes, meshes, the
// texture, and every animation sampled at 30 frames a second. This file draws it. Plain WebGL, no library: the model
// is small (about 1,200 vertices), so the bones move the vertices here and the shader only textures and lights them.
//
//   const a = NMAssistant.mount(canvas, model);   // null when WebGL isn't there
//   a.play('NegoOK');  a.react('work' | 'ok' | 'fail' | 'rest');  a.poke();  a.dispose();
window.NMAssistant = (() => {
  // ---------- 3x4 matrices: rotation and scale, then translation, by rows (for column vectors); Z is up ----------
  function mul(a, ao, b, bo, o, oo) {   // o must not be a or b
    for (let i = 0; i < 12; i += 4) {
      const x = a[ao + i], y = a[ao + i + 1], z = a[ao + i + 2];
      o[oo + i] = x * b[bo] + y * b[bo + 4] + z * b[bo + 8];
      o[oo + i + 1] = x * b[bo + 1] + y * b[bo + 5] + z * b[bo + 9];
      o[oo + i + 2] = x * b[bo + 2] + y * b[bo + 6] + z * b[bo + 10];
      o[oo + i + 3] = x * b[bo + 3] + y * b[bo + 7] + z * b[bo + 11] + a[ao + i + 3];
    }
  }
  function invert(m, mo, o) {
    const a = m[mo], b = m[mo + 1], c = m[mo + 2], d = m[mo + 4], e = m[mo + 5], f = m[mo + 6], g = m[mo + 8], h = m[mo + 9], i = m[mo + 10];
    const A = e * i - f * h, B = f * g - d * i, C = d * h - e * g, det = a * A + b * B + c * C || 1;
    o[0] = A / det; o[1] = (c * h - b * i) / det; o[2] = (b * f - c * e) / det;
    o[4] = B / det; o[5] = (a * i - c * g) / det; o[6] = (c * d - a * f) / det;
    o[8] = C / det; o[9] = (b * g - a * h) / det; o[10] = (a * e - b * d) / det;
    for (let k = 0; k < 12; k += 4) o[k + 3] = -(o[k] * m[mo + 3] + o[k + 1] * m[mo + 7] + o[k + 2] * m[mo + 11]);
  }
  function compose(T, Q, S, n, o, oo) {   // node n's translation, rotation (x, y, z, w) and scale as a matrix
    const x = Q[n * 4], y = Q[n * 4 + 1], z = Q[n * 4 + 2], w = Q[n * 4 + 3], s = S[n];
    o[oo] = (1 - 2 * (y * y + z * z)) * s; o[oo + 1] = 2 * (x * y - w * z) * s; o[oo + 2] = 2 * (x * z + w * y) * s; o[oo + 3] = T[n * 3];
    o[oo + 4] = 2 * (x * y + w * z) * s; o[oo + 5] = (1 - 2 * (x * x + z * z)) * s; o[oo + 6] = 2 * (y * z - w * x) * s; o[oo + 7] = T[n * 3 + 1];
    o[oo + 8] = 2 * (x * z - w * y) * s; o[oo + 9] = 2 * (y * z + w * x) * s; o[oo + 10] = (1 - 2 * (x * x + y * y)) * s; o[oo + 11] = T[n * 3 + 2];
  }

  // ---------- the model file ----------
  function read(model) {
    const raw = atob(model.bin), bytes = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
    const buf = bytes.buffer, n = model.nodes.length;
    const M = { n, parent: new Int16Array(n), restT: new Float32Array(n * 3), restQ: new Float32Array(n * 4), restS: new Float32Array(n), textures: model.textures || [] };
    model.nodes.forEach((x, i) => { M.parent[i] = x.parent; M.restT.set(x.t, i * 3); M.restQ.set(x.q, i * 4); M.restS[i] = x.s; });
    M.meshes = model.meshes.filter((m) => !m.hidden && m.indices > 0).map((m) => ({
      node: m.node, verts: m.verts, count: m.indices, texture: m.texture, diffuse: m.diffuse,
      // the game's alpha settings: blended (drawn after the solid parts, see-through where its picture is), alpha test
      blend: !!m.blend, alpha: m.alpha != null ? m.alpha : 1, alphaNow: m.alpha != null ? m.alpha : 1, cut: m.alphaTest != null && m.alphaTest >= 0 ? m.alphaTest / 255 : -1,
      pos: new Float32Array(buf, m.pos, m.verts * 3), nrm: new Float32Array(buf, m.nrm, m.verts * 3),
      uv: m.uv >= 0 ? new Float32Array(buf, m.uv, m.verts * 2) : new Float32Array(m.verts * 2), idx: new Uint16Array(buf, m.idx, m.indices),
      outPos: new Float32Array(m.verts * 3), outNrm: new Float32Array(m.verts * 3),
      skin: m.skin && {
        root: m.skin.root, rootToSkin: Float64Array.from(m.skin.rootToSkin), bones: m.skin.bones, bind: m.skin.bind.map((b) => Float64Array.from(b)),
        joints: new Uint8Array(buf, m.skin.joints, m.verts * 4), weights: new Float32Array(buf, m.skin.weights, m.verts * 4), mats: new Float64Array(m.skin.bones.length * 12),
      },
    }));
    M.anims = new Map(model.anims.map((a) => [a.name, {
      name: a.name, seconds: a.seconds, frames: a.frames, fps: a.fps,
      tracks: a.tracks.map((t) => ({
        node: t.node, r: t.r, p: t.p, s: t.s,
        rot: t.rot >= 0 ? new Int16Array(buf, t.rot, a.frames * 4) : null,
        pos: t.pos >= 0 ? new Float32Array(buf, t.pos, a.frames * 3) : null,
        scl: t.scl >= 0 ? new Float32Array(buf, t.scl, a.frames) : null,
      })),
      // meshes fading in and out (their material's alpha every frame)
      fades: (a.fades || []).map((x) => ({ node: x.node, alpha: new Float32Array(buf, x.alpha, a.frames) })),
    }]));
    return M;
  }

  // One pose: every node's translation, rotation, scale. An animation only says something about the nodes it moves.
  const newPose = (M) => ({ T: new Float32Array(M.n * 3), Q: new Float32Array(M.n * 4), S: new Float32Array(M.n) });
  function sample(M, anim, time, P) {
    P.T.set(M.restT); P.Q.set(M.restQ); P.S.set(M.restS);
    if (!anim) return;
    const f = Math.min(Math.max(time, 0), anim.seconds) * anim.fps;
    const i0 = Math.min(Math.floor(f), anim.frames - 1), i1 = Math.min(i0 + 1, anim.frames - 1), a = f - i0, b = 1 - a;
    for (const t of anim.tracks) {
      const n = t.node;
      if (t.rot) {
        const r = t.rot, o0 = i0 * 4, o1 = i1 * 4;
        const sign = r[o0] * r[o1] + r[o0 + 1] * r[o1 + 1] + r[o0 + 2] * r[o1 + 2] + r[o0 + 3] * r[o1 + 3] < 0 ? -a : a;
        let x = r[o0] * b + r[o1] * sign, y = r[o0 + 1] * b + r[o1 + 1] * sign, z = r[o0 + 2] * b + r[o1 + 2] * sign, w = r[o0 + 3] * b + r[o1 + 3] * sign;
        const len = Math.hypot(x, y, z, w) || 1;
        P.Q[n * 4] = x / len; P.Q[n * 4 + 1] = y / len; P.Q[n * 4 + 2] = z / len; P.Q[n * 4 + 3] = w / len;
      } else if (t.r) P.Q.set(t.r, n * 4);
      if (t.pos) for (let k = 0; k < 3; k++) P.T[n * 3 + k] = t.pos[i0 * 3 + k] * b + t.pos[i1 * 3 + k] * a;
      else if (t.p) P.T.set(t.p, n * 3);
      if (t.scl) P.S[n] = t.scl[i0] * b + t.scl[i1] * a;
      else if (t.s != null) P.S[n] = t.s;
    }
  }
  function blend(from, to, w, out) {   // w = 0: all "from"
    const v = 1 - w;
    for (let i = 0; i < out.T.length; i++) out.T[i] = from.T[i] * v + to.T[i] * w;
    for (let i = 0; i < out.S.length; i++) out.S[i] = from.S[i] * v + to.S[i] * w;
    for (let i = 0; i < out.Q.length; i += 4) {
      const s = from.Q[i] * to.Q[i] + from.Q[i + 1] * to.Q[i + 1] + from.Q[i + 2] * to.Q[i + 2] + from.Q[i + 3] * to.Q[i + 3] < 0 ? -w : w;
      const x = from.Q[i] * v + to.Q[i] * s, y = from.Q[i + 1] * v + to.Q[i + 1] * s, z = from.Q[i + 2] * v + to.Q[i + 2] * s, q = from.Q[i + 3] * v + to.Q[i + 3] * s;
      const len = Math.hypot(x, y, z, q) || 1;
      out.Q[i] = x / len; out.Q[i + 1] = y / len; out.Q[i + 2] = z / len; out.Q[i + 3] = q / len;
    }
  }

  // Where every vertex is in that pose (outPos, outNrm of each mesh).
  function skin(M, P, world) {
    const local = new Float64Array(12), t1 = new Float64Array(12), t2 = new Float64Array(12), inv = new Float64Array(12), m = new Float64Array(12);
    for (let i = 0; i < M.n; i++) {
      compose(P.T, P.Q, P.S, i, local, 0);
      if (M.parent[i] >= 0) mul(world, M.parent[i] * 12, local, 0, world, i * 12); else world.set(local, i * 12);
    }
    for (const mesh of M.meshes) {
      const sk = mesh.skin, src = mesh.pos, sn = mesh.nrm, out = mesh.outPos, on = mesh.outNrm;
      if (sk) {
        // skinned: mesh node * rootToSkin * inverse(skeleton root) * bone * bind, weighted over up to 4 bones
        invert(world, sk.root * 12, inv);
        mul(world, mesh.node * 12, sk.rootToSkin, 0, t1, 0);
        mul(t1, 0, inv, 0, t2, 0);
        for (let b = 0; b < sk.bones.length; b++) { mul(t2, 0, world, sk.bones[b] * 12, t1, 0); mul(t1, 0, sk.bind[b], 0, sk.mats, b * 12); }
      }
      for (let v = 0; v < mesh.verts; v++) {
        if (sk) {
          m.fill(0);
          for (let k = 0; k < 4; k++) {
            const w = sk.weights[v * 4 + k];
            if (w === 0) continue;
            const o = sk.joints[v * 4 + k] * 12;
            for (let i = 0; i < 12; i++) m[i] += sk.mats[o + i] * w;
          }
        } else for (let i = 0; i < 12; i++) m[i] = world[mesh.node * 12 + i];
        const x = src[v * 3], y = src[v * 3 + 1], z = src[v * 3 + 2], nx = sn[v * 3], ny = sn[v * 3 + 1], nz = sn[v * 3 + 2];
        out[v * 3] = m[0] * x + m[1] * y + m[2] * z + m[3]; out[v * 3 + 1] = m[4] * x + m[5] * y + m[6] * z + m[7]; out[v * 3 + 2] = m[8] * x + m[9] * y + m[10] * z + m[11];
        on[v * 3] = m[0] * nx + m[1] * ny + m[2] * nz; on[v * 3 + 1] = m[4] * nx + m[5] * ny + m[6] * nz; on[v * 3 + 2] = m[8] * nx + m[9] * ny + m[10] * nz;
      }
    }
  }

  // ---------- drawing ----------
  const VS = `attribute vec3 aPos; attribute vec3 aNrm; attribute vec2 aUv;
uniform mat4 uViewProj; uniform vec3 uEye;
varying vec2 vUv; varying vec3 vN; varying vec3 vToEye;
void main() { vUv = aUv; vN = aNrm; vToEye = uEye - aPos; gl_Position = uViewProj * vec4(aPos, 1.0); }`;
  const FS = `precision mediump float;
uniform sampler2D uTex; uniform float uHasTex; uniform vec3 uTint; uniform vec3 uLight; uniform vec3 uRim;
uniform float uAlpha; uniform float uCut; uniform float uBlend;
varying vec2 vUv; varying vec3 vN; varying vec3 vToEye;
void main() {
  vec4 tex = texture2D(uTex, vUv);
  vec3 base = mix(vec3(1.0), tex.rgb, uHasTex) * uTint;
  float a = mix(1.0, tex.a, uHasTex) * uAlpha;
  if (uCut >= 0.0 && a < uCut) discard;
  vec3 n = normalize(vN); vec3 e = normalize(vToEye);
  if (dot(n, e) < 0.0) n = -n;
  float lit = dot(n, uLight) * 0.5 + 0.5;
  float rim = pow(1.0 - max(dot(n, e), 0.0), 3.0);
  // the light on its edges: a brighter touch of the surface's own colour with a little of the app's purple (a fixed
  // purple washed brown and grey demons lilac); none on added light
  vec3 col = base * (0.55 + 0.6 * lit * lit) + (base * 0.7 + uRim * 0.3) * rim * (uBlend > 1.5 ? 0.0 : 1.0);
  // blended parts: premultiplied, as the canvas is; solid parts stay solid
  // added light (uBlend 2) leaves the coverage as it is
  gl_FragColor = uBlend > 1.5 ? vec4(col * a, 0.0) : uBlend > 0.5 ? vec4(col * a, a) : vec4(col, 1.0);
}`;

  /** Black Frost standing idle (the default assistant), in model units: every demon is sized against it. */
  const REF = { tall: 111, wide: 74 };

  function mount(canvas, model, opts) {
    opts = opts || {};
    const M = read(model);
    const gl = canvas.getContext('webgl', { alpha: true, antialias: true, premultipliedAlpha: true });
    if (!gl || M.meshes.length === 0) return null;
    const idle = M.anims.has('Stay') ? 'Stay' : (M.anims.keys().next().value || null);
    // A layer the game fades in and out (Alilat's glyph shell over its stone) is light added on top: the game's own
    // picture of the demon shows its red glyphs orange over the brown stone and its black adding nothing. Drawn so, at
    // full strength (the pulsing looked wrong in the corner).
    for (const a of M.anims.values()) for (const fd of a.fades) for (const mesh of M.meshes) if (mesh.node === fd.node) mesh.add = true;
    const world = new Float64Array(M.n * 12);
    const pose = newPose(M), target = newPose(M), from = newPose(M);
    let G = null;   // the GL objects; made again if the browser drops the context

    // Frame the model as it stands in its idle animation (the file's rest pose has the arms out).
    sample(M, M.anims.get(idle), 0, pose); skin(M, pose, world);
    const lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
    for (const mesh of M.meshes) for (let i = 0; i < mesh.outPos.length; i++) { lo[i % 3] = Math.min(lo[i % 3], mesh.outPos[i]); hi[i % 3] = Math.max(hi[i % 3], mesh.outPos[i]); }
    const centre = [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2];
    const tall = hi[2] - lo[2];
    // One size for every demon: the box shows what Black Frost's height fills (REF), times the game's own size for the
    // model (CModelData scale, model.summon.scale), so a big demon stands bigger, a small one smaller, as in game. Capped:
    // no taller than 1.8 Black Frosts or wider than 3.4 (giants would cover the page), no smaller than 0.55 (still seen).
    // Its feet stay on the box's floor (the model's ground, z = 0): a demon that hovers in game hovers here too.
    const gameScale = model.summon && model.summon.scale > 0 ? model.summon.scale : 1;
    const top = Math.max(hi[2], 1), across = Math.max(hi[0] - lo[0], 1);
    let size = gameScale;
    if (top * size > REF.tall * 1.8) size = REF.tall * 1.8 / top;
    if (across * size > REF.wide * 3.4) size = REF.wide * 3.4 / across;
    if (top * size < REF.tall * 0.55) size = REF.tall * 0.55 / top;
    const span = REF.tall * 1.16 / size;   // model units from the box's bottom to its top
    const ground = Math.min(0, lo[2]);
    let atZ = ground + span * 0.457;   // the height the camera looks at (feet a little above the box's bottom); floor() may lower it

    function setup() {
      const sh = (type, text) => { const s = gl.createShader(type); gl.shaderSource(s, text); gl.compileShader(s); return s; };
      const prog = gl.createProgram();
      gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(prog);
      if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return null;
      const g = { prog, loc: {}, textures: [], meshes: [] };
      for (const a of ['aPos', 'aNrm', 'aUv']) g.loc[a] = gl.getAttribLocation(prog, a);
      for (const u of ['uViewProj', 'uEye', 'uTex', 'uHasTex', 'uTint', 'uLight', 'uRim', 'uAlpha', 'uCut', 'uBlend']) g.loc[u] = gl.getUniformLocation(prog, u);
      const white = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, white);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([255, 255, 255, 255]));
      M.textures.forEach((url, i) => {
        g.textures[i] = white;
        const img = new Image();
        img.onload = () => {
          if (G !== g) return;
          const t = gl.createTexture();
          gl.bindTexture(gl.TEXTURE_2D, t);
          gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
          const pot = (v) => (v & (v - 1)) === 0;
          if (pot(img.width) && pot(img.height)) { gl.generateMipmap(gl.TEXTURE_2D); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR); }
          else { gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE); }
          g.textures[i] = t; g.ready = true; dirty = true; kick();
        };
        img.src = url;
      });
      g.ready = M.textures.length === 0;
      for (const mesh of M.meshes) {
        const b = { pos: gl.createBuffer(), nrm: gl.createBuffer(), uv: gl.createBuffer(), idx: gl.createBuffer() };
        gl.bindBuffer(gl.ARRAY_BUFFER, b.pos); gl.bufferData(gl.ARRAY_BUFFER, mesh.outPos, gl.DYNAMIC_DRAW);
        gl.bindBuffer(gl.ARRAY_BUFFER, b.nrm); gl.bufferData(gl.ARRAY_BUFFER, mesh.outNrm, gl.DYNAMIC_DRAW);
        gl.bindBuffer(gl.ARRAY_BUFFER, b.uv); gl.bufferData(gl.ARRAY_BUFFER, mesh.uv, gl.STATIC_DRAW);
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, b.idx); gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, mesh.idx, gl.STATIC_DRAW);
        g.meshes.push(b);
      }
      return g;
    }

    // The camera: in front of the model (it faces -Y), turned a little so it looks into the window, slightly above.
    const viewProj = new Float32Array(16), eye = [0, 0, 0];
    const box = { w: opts.width || canvas.parentElement.clientWidth || 136, h: opts.height || canvas.parentElement.clientHeight || 150 };   // where it stands, in CSS pixels
    const pad = { l: 0, t: 0, r: 0, b: 0 };   // how much further the canvas goes on each side
    const home = { r: 0, b: 0, standR: 0 };   // the same for its right and bottom, facing the way it does at the start (standR: standing idle only)
    let camDist = 1;   // how far the camera stands from what it looks at (camera())
    let yawNow = opts.yaw != null ? opts.yaw : 22;   // degrees the camera stands to the side of its front; turn() changes it
    function camera() {
      const aspect = box.w / box.h, fov = 24 * Math.PI / 180, yaw = yawNow * Math.PI / 180, pitch = 7 * Math.PI / 180;
      const fit = span, dist = fit / 2 / Math.tan(fov / 2);
      camDist = dist;
      const dir = [Math.sin(yaw) * Math.cos(pitch), -Math.cos(yaw) * Math.cos(pitch), Math.sin(pitch)];
      // aimed above the model's origin, where the game stands a demon (the middle of its box can lie far behind a long one)
      const at = [0, 0, atZ];
      for (let i = 0; i < 3; i++) eye[i] = at[i] + dir[i] * dist;
      // view rows: right, up, back
      const back = dir, right = [-back[1], back[0], 0], rl = Math.hypot(right[0], right[1]) || 1;
      right[0] /= rl; right[1] /= rl;
      const up = [back[1] * right[2] - back[2] * right[1], back[2] * right[0] - back[0] * right[2], back[0] * right[1] - back[1] * right[0]];
      const f = 1 / Math.tan(fov / 2), near = dist * 0.1, far = dist * 4, rows = [right, up, back];
      const view = new Float64Array(16);   // by rows
      for (let r = 0; r < 3; r++) { for (let c = 0; c < 3; c++) view[r * 4 + c] = rows[r][c]; view[r * 4 + 3] = -(rows[r][0] * eye[0] + rows[r][1] * eye[1] + rows[r][2] * eye[2]); }
      view[15] = 1;
      const proj = [f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) / (near - far), 2 * far * near / (near - far), 0, 0, -1, 0];   // by rows
      for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
        let s = 0;
        for (let k = 0; k < 4; k++) s += proj[r * 4 + k] * view[k * 4 + c];
        viewProj[c * 4 + r] = s;   // WebGL wants columns
      }
      // That picture is the box; the canvas around it is bigger (pad), so shrink and shift the picture to its place in it.
      const W = box.w + pad.l + pad.r, H = box.h + pad.t + pad.b, sx = box.w / W, sy = box.h / H, ox = (pad.l - pad.r) / W, oy = (pad.b - pad.t) / H;
      for (let c = 0; c < 16; c += 4) { viewProj[c] = sx * viewProj[c] + ox * viewProj[c + 3]; viewProj[c + 1] = sy * viewProj[c + 1] + oy * viewProj[c + 3]; }
    }

    // The model is framed standing still, but its animations reach further: arms spread, jumps, lunges, a fall.
    // So the canvas is bigger than the box it stands in, by as much as any animation leaves the box on each side
    // (measured here once, every other frame of each animation), and nothing is cut off in mid-air.
    function measure() {
      // It can be turned around (turn), so from every side: the camera at 8 places around it, and last where it is now.
      const keep = yawNow, views = [];
      for (let k = 0; k < 8; k++) { yawNow = k * 45; camera(); views.push(viewProj.slice()); }
      yawNow = keep; camera(); views.push(viewProj.slice());
      let x0 = -1, x1 = 1, y0 = -1, y1 = 1, hx = 1, hy = -1, sx = 1;   // hx, hy: right and bottom as it faces now; sx: right standing idle
      for (const anim of M.anims.values()) {
        if (opts.only && !opts.only.includes(anim.name)) continue;
        for (let f = 0; f < anim.frames; f += 3) {
          sample(M, anim, f / anim.fps, pose); skin(M, pose, world);
          for (const mesh of M.meshes) {
            const p = mesh.outPos;
            for (const v of views)
              for (let i = 0; i < p.length; i += 3) {
                const w = v[3] * p[i] + v[7] * p[i + 1] + v[11] * p[i + 2] + v[15];
                // a demon rushing at the camera (Alilat's Rush, seen from some side) would fill the screen: not counted
                if (w <= camDist * 0.4) continue;
                const x = (v[0] * p[i] + v[4] * p[i + 1] + v[8] * p[i + 2] + v[12]) / w, y = (v[1] * p[i] + v[5] * p[i + 1] + v[9] * p[i + 2] + v[13]) / w;
                if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y;
                if (v === views[8]) { if (x > hx) hx = x; if (y < hy) hy = y; if (anim.name === idle && x > sx) sx = x; }
              }
          }
        }
      }
      const px = (over, size, most) => Math.min(most, over > 0 ? Math.ceil(over / 2 * size) + 8 : 0);
      home.r = px(hx - 1, box.w, box.w); home.b = px(-1 - hy, box.h, box.h); home.standR = px(sx - 1, box.w, box.w);
      // as far as its animations reach to the left and above (a rush, a jump), up to the screen's size: the window can
      // grow to that; to the right and below there is only the window's edge, close by
      const wideMost = Math.max(window.innerWidth || 0, (window.screen && screen.availWidth) || 0, box.w * 2), tallMost = Math.max(window.innerHeight || 0, (window.screen && screen.availHeight) || 0, box.h * 2);
      pad.l = px(-1 - x0, box.w, wideMost); pad.r = px(x1 - 1, box.w, box.w * 2); pad.t = px(y1 - 1, box.h, tallMost); pad.b = px(-1 - y0, box.h, box.h);
      Object.assign(canvas.style, { position: 'absolute', left: -pad.l + 'px', top: -pad.t + 'px', width: box.w + pad.l + pad.r + 'px', height: box.h + pad.t + pad.b + 'px' });
      sample(M, M.anims.get(idle), 0, pose); skin(M, pose, world);
    }

    // A demon reaching towards the camera (Cerberus's front paws) shows lower than its origin's ground: standing idle,
    // its lowest point on screen is lifted to just above the box's bottom. One that hovers above it keeps its height.
    function floor() {
      const anim = M.anims.get(idle), want = -1 + 2 * 0.043;
      for (let pass = 0; pass < 3; pass++) {
        camera();
        let low = 1e9;
        for (let f = 0; f < (anim ? anim.frames : 1); f += 3) {
          sample(M, anim, anim ? f / anim.fps : 0, pose); skin(M, pose, world);
          for (const mesh of M.meshes) {
            const p = mesh.outPos, v = viewProj;
            for (let i = 0; i < p.length; i += 3) {
              const w = v[3] * p[i] + v[7] * p[i + 1] + v[11] * p[i + 2] + v[15];
              if (w > 0) low = Math.min(low, (v[1] * p[i] + v[5] * p[i + 1] + v[9] * p[i + 2] + v[13]) / w);
            }
          }
        }
        if (!(low < want - 0.005)) break;
        atZ -= (want - low) / 2 * span;
      }
      sample(M, M.anims.get(idle), 0, pose); skin(M, pose, world);
    }

    function resize() {
      const dpr = Math.min(window.devicePixelRatio || 1, 3), w = Math.max(1, Math.round(canvas.clientWidth * dpr)), h = Math.max(1, Math.round(canvas.clientHeight * dpr));
      if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; dirty = true; }
    }

    const see = (mesh) => mesh.blend || mesh.add || mesh.alphaNow < 0.999;   // drawn see-through, after the solid parts
    function draw() {
      if (!G) return;
      resize(); camera();
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      if (!G.ready) return;   // nothing until the texture is in: no white flash
      gl.enable(gl.DEPTH_TEST); gl.disable(gl.CULL_FACE); gl.disable(gl.BLEND);
      gl.useProgram(G.prog);
      gl.uniformMatrix4fv(G.loc.uViewProj, false, viewProj);
      gl.uniform3f(G.loc.uEye, eye[0], eye[1], eye[2]);
      // light from the upper left of the picture, towards the viewer
      const l = [eye[0] - centre[0] - (eye[1] - centre[1]) * 0.6, eye[1] - centre[1] + (eye[0] - centre[0]) * 0.6, (hi[2] - lo[2]) * 2.2], ll = Math.hypot(l[0], l[1], l[2]);
      gl.uniform3f(G.loc.uLight, l[0] / ll, l[1] / ll, l[2] / ll);
      const rim = opts.rim || [0.36, 0.30, 0.52];
      gl.uniform3f(G.loc.uRim, rim[0], rim[1], rim[2]);
      gl.uniform1i(G.loc.uTex, 0); gl.activeTexture(gl.TEXTURE0);
      // the solid parts first, then the blended ones over them, added light last
      const drawMesh = (mesh, i) => {
        const b = G.meshes[i], tex = mesh.texture >= 0 ? G.textures[mesh.texture] : null;
        gl.uniform1f(G.loc.uAlpha, mesh.alphaNow); gl.uniform1f(G.loc.uCut, mesh.cut); gl.uniform1f(G.loc.uBlend, mesh.add ? 2 : see(mesh) ? 1 : 0);
        // a blended part still hides what is behind it (its own back faces too); added light hides nothing
        if (mesh.add) { gl.blendFunc(gl.ONE, gl.ONE); gl.depthMask(false); } else if (see(mesh)) { gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA); gl.depthMask(true); }
        gl.bindBuffer(gl.ARRAY_BUFFER, b.pos); gl.bufferSubData(gl.ARRAY_BUFFER, 0, mesh.outPos);
        gl.enableVertexAttribArray(G.loc.aPos); gl.vertexAttribPointer(G.loc.aPos, 3, gl.FLOAT, false, 0, 0);
        gl.bindBuffer(gl.ARRAY_BUFFER, b.nrm); gl.bufferSubData(gl.ARRAY_BUFFER, 0, mesh.outNrm);
        gl.enableVertexAttribArray(G.loc.aNrm); gl.vertexAttribPointer(G.loc.aNrm, 3, gl.FLOAT, false, 0, 0);
        gl.bindBuffer(gl.ARRAY_BUFFER, b.uv);
        gl.enableVertexAttribArray(G.loc.aUv); gl.vertexAttribPointer(G.loc.aUv, 2, gl.FLOAT, false, 0, 0);
        gl.bindTexture(gl.TEXTURE_2D, tex || G.textures[0] || null);
        gl.uniform1f(G.loc.uHasTex, tex ? 1 : 0);
        gl.uniform3f(G.loc.uTint, mesh.diffuse[0], mesh.diffuse[1], mesh.diffuse[2]);
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, b.idx);
        gl.drawElements(gl.TRIANGLES, mesh.count, gl.UNSIGNED_SHORT, 0);
      };
      M.meshes.forEach((mesh, i) => { if (!see(mesh)) drawMesh(mesh, i); });
      if (M.meshes.some(see)) {
        gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
        M.meshes.forEach((mesh, i) => { if (see(mesh) && !mesh.add && mesh.alphaNow > 0.002) drawMesh(mesh, i); });
        M.meshes.forEach((mesh, i) => { if (mesh.add && mesh.alphaNow > 0.002) drawMesh(mesh, i); });
        gl.depthMask(true); gl.disable(gl.BLEND);
      }
    }

    // ---------- what it is doing ----------
    const still = opts.still != null ? opts.still : matchMedia('(prefers-reduced-motion: reduce)').matches;
    const FADE = 0.22;
    let cur = { anim: M.anims.get(idle), time: 0, loop: true, hold: 0 }, fade = 1, dirty = true, raf = 0, last = 0, gone = false, working = false, frozen = false;
    let nextWhim = 0;
    const WHIMS = ['NegoOK', 'Bud', 'Spin', 'Eye', 'MagicAria'].filter((n) => M.anims.has(n));
    const POKES = ['NegoOK', 'Spin', 'Damage', 'Attack', 'Bud', 'MagicShot', 'Breath', 'Rush', 'Needle', 'Death'].filter((n) => M.anims.has(n));
    let pokeAt = 0;
    const planWhim = (now) => { nextWhim = now + 14000 + Math.random() * 22000; };

    /** Play one animation: { loop } keeps it going, { hold: seconds } stays on its last frame for a while; then back to idling. */
    function play(name, o) {
      const anim = M.anims.get(name);
      if (!anim) return false;
      from.T.set(pose.T); from.Q.set(pose.Q); from.S.set(pose.S);
      cur = { anim, time: 0, loop: !!(o && o.loop), hold: (o && o.hold) || 0 };
      fade = 0; frozen = false; dirty = true; kick();
      return true;
    }
    const rest = () => { working = false; if (cur.anim.name !== idle || !cur.loop) play(idle, { loop: true }); };

    function step(dt, now) {
      if (fade < 1) fade = Math.min(1, fade + dt / FADE);
      cur.time += dt;
      if (cur.time >= cur.anim.seconds) {
        if (cur.loop) cur.time %= cur.anim.seconds || 1;
        else if (cur.time >= cur.anim.seconds + cur.hold) { play(idle, { loop: true }); planWhim(now); }
      }
      if (!still && !working && cur.loop && cur.anim.name === idle && now > nextWhim && WHIMS.length) {
        if (nextWhim) play(WHIMS[Math.floor(Math.random() * WHIMS.length)]);
        planWhim(now);
      }
      sample(M, cur.anim, cur.time, target);
      if (fade < 1) blend(from, target, fade * fade * (3 - 2 * fade), pose); else { pose.T.set(target.T); pose.Q.set(target.Q); pose.S.set(target.S); }
      skin(M, pose, world);
    }

    function frame(now) {
      raf = 0;
      if (gone) return;
      const dt = last ? Math.min(0.5, (now - last) / 1000) : 0;   // a window in the background gets few frames: keep the pace
      // the animations are 30 frames a second: every other screen refresh is enough
      if (last && dt < 1 / 40 && !dirty) { raf = requestAnimationFrame(frame); return; }
      last = now;
      const moving = !still || !cur.loop || fade < 1;
      if (moving || dirty) { step(moving && !frozen ? dt : 0, now); draw(); dirty = false; }
      if (moving) raf = requestAnimationFrame(frame); else last = 0;
    }
    function kick() { if (!raf && !gone) raf = requestAnimationFrame(frame); }

    canvas.addEventListener('webglcontextlost', (e) => { e.preventDefault(); G = null; });
    canvas.addEventListener('webglcontextrestored', () => { G = setup(); dirty = true; kick(); });
    const onResize = () => { dirty = true; kick(); };
    window.addEventListener('resize', onResize);
    G = setup();
    if (!G) return null;
    floor();
    measure();
    kick();

    return {
      names: [...M.anims.keys()],
      /** For checks: the model's size standing idle, in its own units (lo, hi corners). */
      bounds: { lo: lo.slice(), hi: hi.slice() },
      /** How far the canvas goes beyond the box it was given, in CSS pixels: { l, t, r, b }. */
      reach: pad,
      /** The same to the right and below as it stood at the start: { r, b }. Keep the box that far from the window's edges. */
      home,
      /** Turn it around its own height by that many degrees (positive: the camera goes to its left side). */
      turn(degrees) { yawNow = ((yawNow + degrees + 180) % 360 + 360) % 360 - 180; dirty = true; kick(); },
      get yaw() { return yawNow; },
      play,
      /** A click: the next of its tricks. */
      poke() { if (POKES.length) { const n = POKES[pokeAt++ % POKES.length]; play(n, { hold: n === 'Death' ? 1.2 : 0 }); } },
      /** The app tells it what is going on: 'work' (reading the game), 'ok', 'fail', 'rest'. */
      react(what) {
        if (what === 'work') { working = true; if (!play('MagicAria', { loop: true })) working = false; }
        else if (what === 'ok') { working = false; if (!play('NegoOK')) rest(); }
        else if (what === 'fail') { working = false; if (!play('Damage')) rest(); }
        else rest();
      },
      get current() { return cur.anim.name; },
      /** For checks: jump to a time of an animation and hold there. */
      pose(name, time) { const a = M.anims.get(name); if (!a) return false; cur = { anim: a, time, loop: false, hold: 1e9 }; fade = 1; frozen = true; sample(M, a, time, target); pose.T.set(target.T); pose.Q.set(target.Q); pose.S.set(target.S); skin(M, pose, world); draw(); return true; },
      dispose() { gone = true; if (raf) cancelAnimationFrame(raf); window.removeEventListener('resize', onResize); const ext = gl.getExtension('WEBGL_lose_context'); if (ext) ext.loseContext(); },
    };
  }

  // ---------- the summoning effect (the assistant picker's change of demon) ----------
  // The pictures are the game's own (its summoning effect's models and particles, from the host's "assistantSummon"),
  // drawn added onto each other and tinted with the demon's summoning colour. How the game times its effect
  // (effect\eff) isn't decoded, so the timing here is the page's own: light comes down, the old demon fades into it,
  // the new one stands there from ARRIVE on, smoke spreads on the ground and sparkles rise.
  const SUMMON = { SECONDS: 1.9, ARRIVE: 0.55 };
  const fxCache = new Map();   // data URI -> loaded picture
  function fxImage(url) {
    if (!url) return Promise.resolve(null);
    if (!fxCache.has(url)) fxCache.set(url, new Promise((done) => { const i = new Image(); i.onload = () => done(i); i.onerror = () => done(null); i.src = url; }));
    return fxCache.get(url);
  }
  /**
   * The picture as light to add: how bright each pixel is becomes how much of it shows (the game draws these added,
   * so their dark backgrounds add nothing; here the near-black is cut so no box shows), in the colour `rgb`, going to
   * white where it is brightest. Without `rgb` it stays white.
   */
  function tinted(img, rgb) {
    const c = document.createElement('canvas'); c.width = img.width; c.height = img.height;
    const x = c.getContext('2d');
    x.drawImage(img, 0, 0);
    const d = x.getImageData(0, 0, c.width, c.height), p = d.data;
    const [r, g, b] = rgb || [255, 255, 255];
    for (let i = 0; i < p.length; i += 4) {
      let v = Math.max(p[i], p[i + 1], p[i + 2]) / 255 * (p[i + 3] / 255);
      v = v <= 0.07 ? 0 : (v - 0.07) / 0.93;
      const w = v * v * v * 0.6;   // the brightest part goes white
      p[i] = r + (255 - r) * w; p[i + 1] = g + (255 - g) * w; p[i + 2] = b + (255 - b) * w; p[i + 3] = v * 255;
    }
    x.putImageData(d, 0, 0);
    return c;
  }
  /**
   * Play the effect on `canvas` (2D), which stands over the assistant's box: `o.box` {w, h} is the box in CSS pixels,
   * `o.ground` the canvas pixel row (CSS) its feet stand on, `o.color` [r,g,b], `o.circle` the game's summoning circle
   * size, `o.arrive()` is called when the new demon should appear. Resolves when it is over (false: nothing to draw).
   */
  async function summon(canvas, fx, o) {
    const T = fx && fx.textures ? fx.textures : {};
    const names = ['pillar', 'lightning', 'glow', 'arc', 'sparks', 'smoke', 'sparkle'];
    const imgs = await Promise.all(names.map((n) => fxImage(T[n])));
    const raw = {}; names.forEach((n, i) => { raw[n] = imgs[i]; });
    if (!raw.glow && !raw.pillar && !raw.smoke) { o.arrive(); return false; }
    const rgb = o.color && o.color.length === 3 ? o.color : [162, 70, 243];
    const tint = {}; for (const n of names) if (raw[n]) tint[n] = tinted(raw[n], rgb);
    const white = {}; for (const n of ['pillar', 'glow', 'sparkle']) if (raw[n]) white[n] = tinted(raw[n], null);
    if (fx.sound && o.sound !== false) { try { const a = new Audio(fx.sound); a.volume = 0.45; a.play().catch(() => {}); } catch (e) { /* no sound */ } }

    const x = canvas.getContext('2d');
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const W = canvas.clientWidth, H = canvas.clientHeight;
    canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
    const gx = W / 2, gy = o.ground, bw = o.box.w, bh = o.box.h;
    const R = 46 * Math.min(1.5, Math.max(0.6, (o.circle || 1.5) / 1.5));   // the circle on the ground, CSS pixels
    // up a -> b, stays, down c -> d (seconds)
    const env = (t, a, b, c, d) => (t <= a || t >= d ? 0 : t < b ? (t - a) / (b - a) : t <= c ? 1 : (d - t) / (d - c));
    const ease = (v) => 1 - (1 - v) * (1 - v);
    const puffs = Array.from({ length: 8 }, (_, i) => ({ a: (i / 8) * Math.PI * 2 + Math.random() * 0.5, s: 0.8 + Math.random() * 0.5, r: Math.random() * Math.PI }));
    let bolt = { at: -1, col: 0, dx: 0, a: 0 }, arrived = false;
    const draw = (img, cx, cy, w, h, alpha, rot) => {
      if (!img || alpha <= 0.002) return;
      x.globalAlpha = Math.min(1, alpha);
      if (rot) { x.save(); x.translate(cx, cy); x.rotate(rot); x.drawImage(img, -w / 2, -h / 2, w, h); x.restore(); }
      else x.drawImage(img, cx - w / 2, cy - h / 2, w, h);
    };
    return new Promise((resolve) => {
      let start = 0, over = false;
      const done = (v) => { if (!over) { over = true; resolve(v); } };
      // a window in the background gets no frames: the new demon still arrives, and it ends
      setTimeout(() => { if (!arrived) { arrived = true; o.arrive(); } }, SUMMON.ARRIVE * 1000 + 400);
      setTimeout(() => { if (!over) { x.setTransform(1, 0, 0, 1, 0, 0); x.clearRect(0, 0, canvas.width, canvas.height); done(true); } }, SUMMON.SECONDS * 1000 + 1500);
      const frame = (now) => {
        if (over) return;
        if (!start) start = now;
        const t = (now - start) / 1000;
        if (!arrived && t >= SUMMON.ARRIVE) { arrived = true; o.arrive(); }
        x.setTransform(dpr, 0, 0, dpr, 0, 0);
        x.globalCompositeOperation = 'source-over'; x.globalAlpha = 1; x.clearRect(0, 0, W, H);
        x.globalCompositeOperation = 'lighter';
        // on the ground: a glow, and smoke spreading out in a ring
        draw(tint.glow, gx, gy, R * 2.8, R * 0.8, env(t, 0, 0.3, 1.1, 1.8));
        const spread = ease(Math.min(1, t / 1.6));
        for (const p of puffs) {
          const a = p.a + t * 0.5, rr = R * (0.45 + 0.75 * spread);
          draw(tint.smoke, gx + Math.cos(a) * rr, gy + Math.sin(a) * rr * 0.3, R * 0.95 * p.s * (0.7 + 0.5 * spread), R * 0.6 * p.s * (0.7 + 0.5 * spread), 0.5 * env(t, 0.05, 0.35, 0.9, 1.85), p.r + t * 0.4);
        }
        // the light coming down, and its white core
        const pa = env(t, 0.05, 0.35, 0.7, 1.25), pw = R * (1.6 + 0.6 * pa);
        draw(tint.pillar, gx, gy - bh * 0.62, pw, bh * 1.5, pa);
        draw(white.pillar, gx, gy - bh * 0.62, pw * 0.55, bh * 1.5, pa * 0.35);
        // lightning: one bolt of the picture's four at a time, changing every 70 ms
        const la = env(t, 0.12, 0.2, 0.65, 0.85);
        if (la > 0 && tint.lightning) {
          if (t - bolt.at > 0.07) bolt = { at: t, col: Math.floor(Math.random() * 4), dx: (Math.random() - 0.5) * R * 1.2, a: 0.55 + Math.random() * 0.45 };
          const L = tint.lightning, sw = L.width / 4;
          x.globalAlpha = la * bolt.a;
          x.drawImage(L, bolt.col * sw, 0, sw, L.height, gx + bolt.dx - R * 0.3, gy - bh * 1.05, R * 0.6, bh * 1.05);
        }
        // swirling arcs going up around it
        const aa = env(t, 0.25, 0.45, 0.95, 1.35);
        for (let k = 0; k < 3; k++) {
          const ang = t * 5 + k * 2.1, lift = ease(Math.min(1, Math.max(0, (t - 0.25) / 1.1)));
          draw(tint.arc, gx + Math.cos(ang) * R * 0.7, gy - bh * (0.15 + 0.55 * lift) + k * 6, R * 0.9, R * 0.9, aa * 0.9, ang + Math.PI / 2);
        }
        draw(tint.sparks, gx, gy - bh * 0.5, R * 1.1, bh * 0.9, env(t, 0.3, 0.45, 0.6, 0.9) * 0.8);
        // the flash it appears in
        const fa = env(t, SUMMON.ARRIVE - 0.12, SUMMON.ARRIVE, SUMMON.ARRIVE + 0.05, SUMMON.ARRIVE + 0.45);
        draw(tint.glow, gx, gy - bh * 0.45, R * 3.2, R * 3.6, fa);
        draw(white.glow, gx, gy - bh * 0.45, R * 1.6, R * 1.9, fa * 0.6);
        // sparkles rising after it
        const sa = env(t, 0.45, 0.7, 1.3, 1.9);
        draw(tint.sparkle, gx, gy - bh * 0.55 - t * 26, bw * 1.25, bh * 1.1, sa);
        draw(white.sparkle, gx + 6, gy - bh * 0.35 - t * 40, bw * 0.9, bh * 0.8, sa * 0.5);
        if (t < SUMMON.SECONDS) requestAnimationFrame(frame);
        else { x.setTransform(1, 0, 0, 1, 0, 0); x.clearRect(0, 0, canvas.width, canvas.height); done(true); }
      };
      requestAnimationFrame(frame);
    });
  }

  return { mount, summon, SUMMON };
})();
