function initializeGlyphAtlas(glyphs, gl) {
  if (glyphs.canvas) return;
  glyphs.canvas = document.createElement("canvas");
  glyphs.canvas.width = 2048;
  glyphs.canvas.height = 2048;
  glyphs.ctx = glyphs.canvas.getContext("2d", { willReadFrequently: false });
  glyphs.tex = gl.createTexture();
  if (glyphs.tex === null) throw new Error("GPU glyph texture allocation failed");
  gl.bindTexture(gl.TEXTURE_2D, glyphs.tex);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
}

function resetGlyphAtlas(glyphs) {
  glyphs.ctx.clearRect(0, 0, 2048, 2048);
  glyphs.map.clear();
  glyphs.x = glyphs.y = glyphs.rowH = 0;
  glyphs.dirty = true;
}

function retainedGlyphEntry(glyphs, text, voice, sizeCss, color, halo, haloWidthEm, retried) {
  const dpr = window.devicePixelRatio || 1;
  const px = Math.max(2, Math.round(sizeCss * 2) / 2);
  const hw = haloWidthEm === undefined ? 0.24 : haloWidthEm;
  const key = [text, voice.family, voice.weight, voice.italic, (voice.tracking_em ?? voice.tracking), px,
               color.join(), halo.join(), hw].join("|");
  const hit = glyphs.map.get(key);
  if (hit) return hit;
  const ctx = glyphs.ctx;
  ctx.font = `${voice.italic ? "italic " : ""}${voice.weight} ${px * dpr}px ${voice.family}`;
  try { ctx.letterSpacing = ((voice.tracking_em ?? voice.tracking) * px * dpr) + "px"; } catch (e) {}
  const m = ctx.measureText(text);
  const pad = Math.ceil(px * dpr * 0.4);
  const w = Math.ceil(m.width) + pad * 2;
  const h = Math.ceil(px * dpr * 1.4) + pad;
  if (w > 2048) return null;
  if (glyphs.x + w > 2048) { glyphs.x = 0; glyphs.y += glyphs.rowH; glyphs.rowH = 0; }
  if (glyphs.y + h > 2048) {
    if (retried) return null;
    resetGlyphAtlas(glyphs);
    return retainedGlyphEntry(glyphs, text, voice, sizeCss, color, halo, haloWidthEm, true);
  }
  const bx = glyphs.x, by = glyphs.y;
  const baseline = by + pad + px * dpr;
  ctx.textAlign = "left";
  ctx.lineJoin = "round";
  ctx.strokeStyle = `rgba(${halo[0]},${halo[1]},${halo[2]},${halo[3] / 255})`;
  ctx.lineWidth = px * dpr * hw * 2;
  ctx.strokeText(text, bx + pad, baseline);
  ctx.fillStyle = `rgba(${color[0]},${color[1]},${color[2]},${color[3] / 255})`;
  ctx.fillText(text, bx + pad, baseline);
  const e = {
    u0: bx / 2048, v0: by / 2048, u1: (bx + w) / 2048, v1: (by + h) / 2048,
    w: w / dpr, h: h / dpr, px,
  };
  glyphs.map.set(key, e);
  glyphs.x += w;
  glyphs.rowH = Math.max(glyphs.rowH, h);
  glyphs.dirty = true;
  return e;
}

function disposeGlyphAtlas(glyphs, gl) {
  if (glyphs.tex !== null) gl.deleteTexture(glyphs.tex);
  glyphs.tex = null;
  glyphs.canvas = null;
  glyphs.ctx = null;
  glyphs.map.clear();
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { initializeGlyphAtlas, resetGlyphAtlas, retainedGlyphEntry, disposeGlyphAtlas };
}
