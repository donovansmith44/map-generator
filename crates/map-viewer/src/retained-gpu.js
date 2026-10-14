function uploadGeometry(gl, loc, verts) {
  const arcs = new Float32Array(verts.length / 3);
  let acc = 0;
  for (let i = 1; i < arcs.length; i++) {
    acc += Math.hypot(verts[i * 3] - verts[i * 3 - 3], verts[i * 3 + 1] - verts[i * 3 - 2], verts[i * 3 + 2] - verts[i * 3 - 1]);
    arcs[i] = acc;
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

function dropClipScratch(gl, resource) {
  for (const lobe of resource.clipScratch || []) disposeGeometry(gl, lobe);
  resource.clipScratch = null;
  resource.clipKey = null;
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
      gl.drawArrays(gl.TRIANGLE_FAN, 0, draw.count);
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
    gl.clearStencil(0);
    gl.clear(gl.STENCIL_BUFFER_BIT);
    throw error;
  } finally {
    gl.colorMask(true, true, true, true);
    gl.disable(gl.STENCIL_TEST);
    gl.uniform1f(loc.uFrontMin, options.frontMin);
  }
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { uploadGeometry, uploadLobe, uploadPrevious, endPrevious, dropClipScratch, disposeGeometry, drawStencilFill };
}
