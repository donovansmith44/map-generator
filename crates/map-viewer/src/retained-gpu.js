function sphericalInterpolationSource() {
  return `vec3 sphericalInterpolate(vec3 from, vec3 to, float fraction) {
  vec3 a = normalize(from);
  vec3 b = normalize(to);
  if (fraction == 0.0) return a;
  if (fraction == 1.0) return b;
  vec3 normal = cross(a, b);
  float sine = length(normal);
  float cosine = clamp(dot(a, b), -1.0, 1.0);
  if (sine == 0.0 && cosine > 0.0) return a;
  float angle = atan(sine, cosine);
  vec3 tangent = cross(normal / sine, a);
  return normalize(a * cos(fraction * angle) + tangent * sin(fraction * angle));
}`;
}

function uploadGeometry(gl, loc, verts, ranges = [{ first: 0, count: verts.length / 3 }]) {
  const arcs = new Float32Array(verts.length / 3);
  for (const range of ranges) {
    let acc = 0;
    for (let i = range.first + 1; i < range.first + range.count; i++) {
      acc += Math.hypot(verts[i * 3] - verts[i * 3 - 3], verts[i * 3 + 1] - verts[i * 3 - 2], verts[i * 3 + 2] - verts[i * 3 - 1]);
      arcs[i] = acc;
    }
  }
  return uploadVertices(gl, loc, verts, arcs);
}

function uploadLobe(gl, loc, verts) {
  return uploadVertices(gl, loc, verts, null);
}

function uploadVertices(gl, loc, verts, arcs) {
  const resource = { vao: null, buf: null, abuf: null, count: verts.length / 3, verts, fillMeshes: new Set() };
  try {
    resource.vao = gl.createVertexArray();
    if (resource.vao === null) throw new Error("GPU vertex array allocation failed");
    gl.bindVertexArray(resource.vao);
    resource.buf = gl.createBuffer();
    if (resource.buf === null) throw new Error("GPU vertex buffer allocation failed");
    gl.bindBuffer(gl.ARRAY_BUFFER, resource.buf);
    gl.bufferData(gl.ARRAY_BUFFER, verts, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(loc.aPos);
    gl.vertexAttribPointer(loc.aPos, 3, gl.FLOAT, false, 0, 0);
    gl.enableVertexAttribArray(loc.aPrev);
    gl.vertexAttribPointer(loc.aPrev, 3, gl.FLOAT, false, 0, 0);
    if (arcs !== null) {
      resource.abuf = gl.createBuffer();
      if (resource.abuf === null) throw new Error("GPU arc buffer allocation failed");
      gl.bindBuffer(gl.ARRAY_BUFFER, resource.abuf);
      gl.bufferData(gl.ARRAY_BUFFER, arcs, gl.STATIC_DRAW);
      if (loc.aArc >= 0) {
        gl.enableVertexAttribArray(loc.aArc);
        gl.vertexAttribPointer(loc.aArc, 1, gl.FLOAT, false, 0, 0);
      }
    }
    return resource;
  } catch (error) {
    disposeGeometry(gl, resource);
    throw error;
  } finally {
    gl.bindVertexArray(null);
  }
}

function uploadPrevious(gl, loc, resource, parents) {
  const buffer = gl.createBuffer();
  if (buffer === null) throw new Error("GPU previous-position allocation failed");
  try {
    gl.bindVertexArray(resource.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, parents, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(loc.aPrev);
    gl.vertexAttribPointer(loc.aPrev, 3, gl.FLOAT, false, 0, 0);
    dropFillMeshes(gl, resource);
    return buffer;
  } catch (error) {
    endPrevious(gl, loc, resource, buffer);
    throw error;
  } finally {
    gl.bindVertexArray(null);
  }
}

function endPrevious(gl, loc, resource, buffer) {
  try {
    if (resource?.vao != null && resource.buf != null) {
      gl.bindVertexArray(resource.vao);
      gl.bindBuffer(gl.ARRAY_BUFFER, resource.buf);
      gl.enableVertexAttribArray(loc.aPrev);
      gl.vertexAttribPointer(loc.aPrev, 3, gl.FLOAT, false, 0, 0);
      dropFillMeshes(gl, resource);
    }
  } finally {
    gl.bindVertexArray(null);
    gl.deleteBuffer(buffer);
  }
}

// Borrow the ring's visible GPU ranges. The immutable ring owns every clipped
// lobe; callers invalidate/recreate it when its context, layout or range changes.
function visibleFillDraws(gl, loc, ring, frontHemisphereCenter) {
  if (frontHemisphereCenter === null) return [ring];
  const C = frontHemisphereCenter, key = C.join(",");
  if (ring.clipKey !== key) {
    dropClipScratch(gl, ring);
    const limb = typeof module !== "undefined" && module.exports ? require("./limb.js") : globalThis;
    try {
      const cap = ring.cap;
      const t = cap ? cap.ax * C[0] + cap.ay * C[1] + cap.az * C[2] : 0;
      const sinR = cap ? Math.sqrt(1 - cap.cosR * cap.cosR) : 0;
      ring.clipScratch = [];
      // Equality is tangent, so it must take the clipping path.
      if (cap && cap.cosR > 0 && t > sinR) {
        ring.clipSelf = true;
      } else if (!(cap && cap.cosR > 0 && t < -sinR)) {
        const { E, N } = limb.limbBasis(C);
        const first = ring.first ?? 0;
        const vertices = ring.verts.subarray(first * 3, (first + ring.count) * 3);
        const clipped = limb.clipRingFront(vertices, C, E, N);
        ring.clipSelf = clipped === vertices;
        if (!ring.clipSelf && clipped) {
          for (const points of clipped) ring.clipScratch.push(uploadLobe(gl, loc, points));
        }
      }
      ring.clipKey = key;
    } catch (error) {
      dropClipScratch(gl, ring);
      throw error;
    }
  }
  return ring.clipSelf ? [ring] : ring.clipScratch;
}

function dropClipScratch(gl, resource) {
  for (const lobe of resource.clipScratch || []) disposeGeometry(gl, lobe);
  resource.clipScratch = null;
  resource.clipKey = null;
  resource.clipSelf = false;
}

function dropFillMeshes(gl, resource) {
  for (const mesh of resource.fillMeshes || []) disposeGeometry(gl, mesh);
  resource.fillMeshes?.clear();
}

function disposeGeometry(gl, resource) {
  dropClipScratch(gl, resource);
  dropFillMeshes(gl, resource);
  if (resource.vao != null) gl.deleteVertexArray(resource.vao);
  if (resource.buf != null) gl.deleteBuffer(resource.buf);
  if (resource.abuf != null) gl.deleteBuffer(resource.abuf);
  resource.vao = null;
  resource.buf = null;
  resource.abuf = null;
}

function tessellateFill(contours, winding, boundaries = false) {
  const lib = typeof module !== "undefined" && module.exports ? require("./libtess.js") : globalThis.libtess;
  const tess = new lib.GluTesselator(), output = [], enums = lib.gluEnum;
  let loop;
  tess.gluTessCallback(enums.GLU_TESS_BEGIN, () => { if (boundaries) loop = []; });
  tess.gluTessCallback(enums.GLU_TESS_END, () => { if (boundaries) output.push(loop); });
  tess.gluTessCallback(enums.GLU_TESS_VERTEX, point => { if (boundaries) loop.push(point); else output.push(...point); });
  tess.gluTessCallback(enums.GLU_TESS_COMBINE, coordinates => coordinates);
  tess.gluTessCallback(enums.GLU_TESS_ERROR, error => { throw new Error(`Fill tessellation failed: ${error}`); });
  if (!boundaries) tess.gluTessCallback(enums.GLU_TESS_EDGE_FLAG, () => {});
  tess.gluTessProperty(enums.GLU_TESS_WINDING_RULE, winding);
  tess.gluTessProperty(enums.GLU_TESS_BOUNDARY_ONLY, boundaries);
  tess.gluTessNormal(0, 0, 1);
  try {
    tess.gluTessBeginPolygon(null);
    for (const ring of contours) {
      tess.gluTessBeginContour();
      for (const point of ring) tess.gluTessVertex(point, point);
      tess.gluTessEndContour();
    }
    tess.gluTessEndPolygon();
    return output;
  } finally { tess.gluDeleteTess(); }
}

function projectedFillContours(gl, loc, draws, options, width, height) {
  const prior = {
    feedback: gl.getParameter(gl.TRANSFORM_FEEDBACK_BINDING), buffer: gl.getParameter(gl.TRANSFORM_FEEDBACK_BUFFER_BINDING),
    vao: gl.getParameter(gl.VERTEX_ARRAY_BINDING), discard: gl.isEnabled(gl.RASTERIZER_DISCARD),
  };
  const feedback = gl.createTransformFeedback(), buffer = gl.createBuffer();
  let active = false;
  try {
    if (feedback === null || buffer === null) throw new Error("GPU fill projection allocation failed");
    gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, feedback);
    gl.bindBuffer(gl.TRANSFORM_FEEDBACK_BUFFER, buffer);
    const captures = draws.map(draw => ({ ...draw, mode: options.mode }));
    if (options.whole && options.mode === 0) captures.push({ vao: options.discVao, first: 1, count: options.discCount - 1, mode: 3 });
    const capacity = Math.max(0, ...captures.map(draw => draw.count));
    gl.bufferData(gl.TRANSFORM_FEEDBACK_BUFFER, capacity * 16, gl.STREAM_READ);
    gl.bindBufferBase(gl.TRANSFORM_FEEDBACK_BUFFER, 0, buffer);
    gl.enable(gl.RASTERIZER_DISCARD);
    const contours = [];
    for (const draw of captures) {
      if (draw.count < 3) continue;
      gl.uniform1i(loc.uMode, draw.mode);
      gl.bindVertexArray(draw.vao);
      gl.beginTransformFeedback(gl.POINTS); active = true;
      gl.drawArrays(gl.POINTS, draw.first ?? 0, draw.count);
      gl.endTransformFeedback(); active = false;
      const clip = new Float32Array(draw.count * 4), ring = [];
      gl.getBufferSubData(gl.TRANSFORM_FEEDBACK_BUFFER, 0, clip);
      for (let index = 0; index < clip.length; index += 4) {
        const x = (clip[index] / clip[index + 3] + 1) * width / 2;
        const y = (1 - clip[index + 1] / clip[index + 3]) * height / 2;
        if (!Number.isFinite(x) || !Number.isFinite(y)) throw new Error("Non-finite fill projection");
        ring.push([x, y, 0]);
      }
      contours.push(ring);
    }
    if (options.whole && options.mode !== 0) contours.push([[0, 0, 0], [width, 0, 0], [width, height, 0], [0, height, 0]]);
    return contours;
  } finally {
    if (active) gl.endTransformFeedback();
    gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, prior.feedback);
    gl.bindBuffer(gl.TRANSFORM_FEEDBACK_BUFFER, prior.buffer);
    gl.bindVertexArray(prior.vao);
    if (!prior.discard) gl.disable(gl.RASTERIZER_DISCARD);
    gl.uniform1i(loc.uMode, options.mode);
    gl.deleteTransformFeedback(feedback);
    gl.deleteBuffer(buffer);
  }
}

function drawStencilFill(gl, loc, draws, options) {
  if (draws.length === 0 && !options.whole) return;
  const lib = typeof module !== "undefined" && module.exports ? require("./libtess.js") : globalThis.libtess;
  const viewport = Array.from(gl.getParameter(gl.VIEWPORT));
  const owner = options.owner ?? draws[0];
  if (!owner) return;
  const key = [gl.getParameter(gl.CURRENT_PROGRAM), ...viewport, options.whole, options.mode, ...(options.key ?? [])];
  const cached = owner.fillCache;
  let mesh;
  gl.uniform2f(loc.uDash, 0, 0);
  gl.uniform1f(loc.uFrontMin, -1.1);
  try {
    if (cached && cached.mesh.vao !== null && options.key && cached.key.length === key.length && cached.key.every((value, index) => value === key[index])
      && cached.draws.length === draws.length && cached.draws.every((draw, index) => draw === draws[index])) mesh = cached.mesh;
    else {
      if (cached) {
        owner.fillMeshes?.delete(cached.mesh);
        disposeGeometry(gl, cached.mesh);
        owner.fillCache = null;
      }
      const width = viewport[2], height = viewport[3];
      const contours = projectedFillContours(gl, loc, draws, options, width, height);
      const normalized = tessellateFill(contours, lib.windingRule.GLU_TESS_WINDING_ODD, true);
      normalized.push([[0, 0, 0], [width, 0, 0], [width, height, 0], [0, height, 0]]);
      const triangles = tessellateFill(normalized, lib.windingRule.GLU_TESS_WINDING_ABS_GEQ_TWO);
      for (let index = 0; index < triangles.length; index += 3) {
        triangles[index] = triangles[index] / width * 2 - 1;
        triangles[index + 1] = 1 - triangles[index + 1] / height * 2;
      }
      mesh = uploadLobe(gl, loc, Float32Array.from(triangles));
      owner.fillMeshes.add(mesh);
      owner.fillCache = { key, draws: [...draws], mesh };
    }
    gl.disable(gl.STENCIL_TEST);
    gl.uniform4f(loc.uColor, ...options.color.map(channel => channel / 255));
    if (loc.uScreen != null) gl.uniform1f(loc.uScreen, 1);
    gl.uniform1i(loc.uMode, loc.uScreen != null && !options.whole ? options.mode : 2);
    gl.bindVertexArray(mesh.vao);
    gl.drawArrays(gl.TRIANGLES, 0, mesh.count);
  } finally {
    if (loc.uScreen != null) gl.uniform1f(loc.uScreen, 0);
    gl.uniform1i(loc.uMode, options.mode);
    gl.uniform1f(loc.uFrontMin, options.frontMin);
  }
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { sphericalInterpolationSource, uploadGeometry, uploadLobe, uploadPrevious, endPrevious, visibleFillDraws, dropClipScratch, disposeGeometry, drawStencilFill };
}
