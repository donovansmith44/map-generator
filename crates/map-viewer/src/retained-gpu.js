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
  const resource = { vao: null, buf: null, abuf: null, count: verts.length / 3, verts };
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

function disposeGeometry(gl, resource) {
  dropClipScratch(gl, resource);
  if (resource.vao != null) gl.deleteVertexArray(resource.vao);
  if (resource.buf != null) gl.deleteBuffer(resource.buf);
  if (resource.abuf != null) gl.deleteBuffer(resource.abuf);
  resource.vao = null;
  resource.buf = null;
  resource.abuf = null;
}

function drawStencilFill(gl, loc, draws, options) {
  const pagePass = () => {
    if (!options.whole) return;
    if (options.mode === 0) {
      gl.uniform1i(loc.uMode, 3);
      gl.bindVertexArray(options.discVao);
      gl.drawArrays(gl.TRIANGLE_FAN, 0, options.discCount);
    } else {
      gl.uniform1i(loc.uMode, 2);
      gl.bindVertexArray(options.pageVao);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }
    gl.uniform1i(loc.uMode, options.mode);
  };
  const rings = () => {
    pagePass();
    for (const draw of draws) {
      gl.bindVertexArray(draw.vao);
      gl.drawArrays(gl.TRIANGLE_FAN, draw.first ?? 0, draw.count);
    }
  };
  gl.uniform2f(loc.uDash, 0, 0);
  gl.uniform1f(loc.uFrontMin, -1.1);
  gl.enable(gl.STENCIL_TEST);
  try {
    gl.colorMask(false, false, false, false);
    gl.stencilFunc(gl.ALWAYS, 0, 0xff);
    gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);
    rings();
    gl.colorMask(true, true, true, true);
    gl.stencilFunc(gl.NOTEQUAL, 0, 0xff);
    gl.stencilOp(gl.KEEP, gl.KEEP, gl.ZERO);
    gl.uniform4f(loc.uColor, ...options.color.map(channel => channel / 255));
    rings();
  } catch (error) {
    gl.clearBufferiv(gl.STENCIL, 0, [0]);
    throw error;
  } finally {
    gl.colorMask(true, true, true, true);
    gl.disable(gl.STENCIL_TEST);
    gl.uniform1f(loc.uFrontMin, options.frontMin);
  }
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { sphericalInterpolationSource, uploadGeometry, uploadLobe, uploadPrevious, endPrevious, visibleFillDraws, dropClipScratch, disposeGeometry, drawStencilFill };
}
