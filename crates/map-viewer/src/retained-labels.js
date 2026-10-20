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

function reserveGlyphSpace(glyphs, w, h) {
  if (w > 2048 || h > 2048) return null;
  const wraps = glyphs.x + w > 2048;
  const x = wraps ? 0 : glyphs.x;
  const y = wraps ? glyphs.y + glyphs.rowH : glyphs.y;
  if (y + h > 2048) return null;
  glyphs.x = x + w;
  glyphs.y = y;
  glyphs.rowH = Math.max(wraps ? 0 : glyphs.rowH, h);
  return { x, y };
}

function retainedImageEntry(glyphs, marker) {
  const hit = glyphs.map.get(marker);
  if (hit) return hit;
  const { image, pixelRatio } = marker;
  const position = reserveGlyphSpace(glyphs, image.width, image.height);
  if (position === null) return null;
  const data = glyphs.ctx.createImageData(image.width, image.height);
  data.data.set(image.data);
  glyphs.ctx.putImageData(data, position.x, position.y);
  const entry = {
    u0: position.x / 2048, v0: position.y / 2048,
    u1: (position.x + image.width) / 2048, v1: (position.y + image.height) / 2048,
    w: image.width / pixelRatio, h: image.height / pixelRatio,
  };
  glyphs.map.set(marker, entry);
  glyphs.dirty = true;
  return entry;
}

function retainedGlyphEntry(glyphs, text, voice, sizeCss, color, halo, haloWidthEm) {
  const dpr = window.devicePixelRatio || 1;
  const px = Math.max(2, Math.round(sizeCss * 2) / 2);
  const hw = haloWidthEm === undefined ? 0.24 : haloWidthEm;
  const key = [dpr, text, voice.family, voice.weight, voice.italic, (voice.tracking_em ?? voice.tracking), px,
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
  const position = reserveGlyphSpace(glyphs, w, h);
  if (position === null) return null;
  const { x: bx, y: by } = position;
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
  glyphs.dirty = true;
  return e;
}

function disposeGlyphAtlas(glyphs, gl) {
  if (glyphs.tex !== null) gl.deleteTexture(glyphs.tex);
  glyphs.tex = null;
  glyphs.canvas = null;
  glyphs.ctx = null;
  glyphs.map.clear();
  glyphs.x = glyphs.y = glyphs.rowH = 0;
  glyphs.dirty = false;
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { initializeGlyphAtlas, resetGlyphAtlas, retainedGlyphEntry, retainedImageEntry, disposeGlyphAtlas };
}
