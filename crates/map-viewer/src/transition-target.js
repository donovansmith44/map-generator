(function (scope) {
  const retained = typeof module !== "undefined" && module.exports ? require("./retained-gpu.js") : scope;
  const labels = typeof module !== "undefined" && module.exports ? require("./retained-labels.js") : scope;

  function transitionTarget(gl, projection, paint, cold) {
    const target = { gl, projection, paint, cold, programs: new Map(), geometry: [], extras: [], frame: null, scenes: [], disposed: false, crossfades: 0,
      surfaces: [], glyphs: { canvas: null, ctx: null, tex: null, map: new Map(), x: 0, y: 0, rowH: 0, dirty: false } };
    try {
      programFor(target, projection());
      return target;
    } catch (error) {
      disposeTransition(target);
      throw error;
    }
  }

  function drawTransition(target, frame, shapes, fraction) {
    if (target.disposed) throw new Error("Transition target is disposed");
    if (!Number.isFinite(fraction) || fraction < 0 || fraction > 1) throw new Error("Invalid transition fraction");
    if (fraction === 1) return settleTransition(target, frame.after);
    const gl = target.gl;
    const state = saveState(gl);
    try {
      const input = target.projection();
      const program = programFor(target, input);
      if (target.frame !== frame || target.surfaces.some(surface => surface.width !== gl.drawingBufferWidth || surface.height !== gl.drawingBufferHeight)) {
        clearFrame(target);
        target.frame = frame;
        target.crossfades++;
        target.surfaces = [];
        target.surfaces.push(surface(target));
        target.surfaces.push(surface(target));
        target.screen = screenQuad(target, program.loc);
        target.scenes = [buildScene(target, program.loc, frame.before), buildScene(target, program.loc, frame.after)];
      }
      gl.useProgram(program.value);
      projectionUniforms(gl, program.loc, input.defaultProjectionData);
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
      gl.blendEquationSeparate(gl.FUNC_ADD, gl.FUNC_ADD);
      gl.colorMask(true, true, true, true);
      gl.disable(gl.DEPTH_TEST);
      gl.disable(gl.STENCIL_TEST);
      gl.stencilMask(0xff);
      gl.uniform1f(program.loc.uMorph, 1);
      gl.uniform1f(program.loc.uFrontMin, -1.1);
      gl.uniform2f(program.loc.uDash, 0, 0);
      gl.uniform2f(program.loc.uViewport, gl.drawingBufferWidth, gl.drawingBufferHeight);
      for (let scene = 0; scene < 2; scene++) {
        const surface = target.surfaces[scene];
        gl.bindFramebuffer(gl.FRAMEBUFFER, surface.framebuffer);
        gl.viewport(0, 0, surface.width, surface.height);
        gl.clearColor(0, 0, 0, 0);
        gl.clearStencil(0);
        gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
        drawScene(target, program.loc, target.scenes[scene], 1);
      }
      gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, state.DRAW_FRAMEBUFFER_BINDING);
      gl.viewport(...state.VIEWPORT);
      gl.uniform1f(program.loc.uScreen, 1);
      gl.uniform1f(program.loc.uLabel, 2);
      gl.uniform1f(program.loc.uMix, fraction);
      gl.uniform4f(program.loc.uUV, 0, 0, 1, 1);
      for (let scene = 0; scene < 2; scene++) {
        gl.activeTexture(gl.TEXTURE0 + scene);
        gl.bindTexture(gl.TEXTURE_2D, target.surfaces[scene].texture);
      }
      gl.uniform1i(program.loc.uTex, 0);
      gl.uniform1i(program.loc.uAfterTex, 1);
      gl.bindVertexArray(target.screen.vao);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      gl.uniform1f(program.loc.uScreen, 0);
    } catch (error) {
      clearFrame(target);
      throw error;
    } finally {
      restoreState(gl, state);
    }
  }

  function settleTransition(target, destination) {
    if (target.disposed) throw new Error("Transition target is disposed");
    clearFrame(target);
    return target.cold(destination);
  }

  function disposeTransition(target) {
    if (target.disposed) return;
    target.disposed = true;
    clearFrame(target);
    for (const program of target.programs.values()) target.gl.deleteProgram(program.value);
    target.programs.clear();
  }

  function clearFrame(target) {
    for (const resource of target.geometry) retained.disposeGeometry(target.gl, resource);
    for (const buffer of target.extras) target.gl.deleteBuffer(buffer);
    labels.disposeGlyphAtlas(target.glyphs, target.gl);
    for (const surface of target.surfaces) {
      target.gl.deleteFramebuffer(surface.framebuffer);
      target.gl.deleteRenderbuffer(surface.stencil);
      target.gl.deleteTexture(surface.texture);
    }
    target.surfaces = [];
    target.screen = null;
    target.geometry = [];
    target.extras = [];
    target.scenes = [];
    target.frame = null;
  }

  function shader(gl, type, source) {
    const value = gl.createShader(type);
    if (value === null) throw new Error("GPU shader allocation failed");
    try {
      gl.shaderSource(value, source);
      gl.compileShader(value);
      if (!gl.getShaderParameter(value, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(value));
      return value;
    } catch (error) {
      gl.deleteShader(value);
      throw error;
    }
  }

  function programFor(target, input) {
    const description = input.shaderData;
    const hit = target.programs.get(description.variantName);
    if (hit) return hit;
    const gl = target.gl;
    const vertex = shader(gl, gl.VERTEX_SHADER, `#version 300 es
${description.vertexShaderPrelude}
${description.define}
precision highp float;
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aPrev;
layout(location=2) in float aArc;
layout(location=3) in vec2 aCorner;
uniform float uMorph;
uniform float uPointSize;
uniform float uLabel;
uniform float uScreen;
uniform vec2 uViewport;
uniform vec2 uSize;
uniform vec4 uUV;
out float vArc;
out vec2 vUV;
void main() {
  vec3 p = normalize(mix(aPrev, aPos, uMorph));
  float latitude = asin(clamp(p.z, -1.0, 1.0));
  vec2 mercator = vec2(atan(p.y, p.x) / (2.0 * PI) + 0.5, 0.5 - log(tan(PI * 0.25 + latitude * 0.5)) / (2.0 * PI));
  gl_Position = uScreen > 0.5 ? vec4(aPos.xy, 0.0, 1.0) : projectTile(mercator);
  if (uLabel > 0.5 && uScreen < 0.5) gl_Position.xy += vec2(aCorner.x - 0.5, 0.5 - aCorner.y) * uSize / uViewport * gl_Position.w * 2.0;
  gl_PointSize = uPointSize;
  vArc = aArc;
  vUV = mix(uUV.xy, uUV.zw, aCorner);
}`);
    let fragment = null, value = null;
    try {
      fragment = shader(gl, gl.FRAGMENT_SHADER, `#version 300 es
precision highp float;
in float vArc;
in vec2 vUV;
uniform vec4 uColor;
uniform vec2 uDash;
uniform float uLabel;
uniform sampler2D uTex;
uniform sampler2D uAfterTex;
uniform float uMix;
uniform float uAlpha;
out vec4 o;
void main() {
  if (uLabel > 1.5) { o = mix(texture(uTex, vUV), texture(uAfterTex, vUV), uMix); return; }
  if (uLabel > 0.5) { o = texture(uTex, vUV) * uAlpha; return; }
  if (uDash.x > 0.0 && mod(vArc, uDash.x + uDash.y) > uDash.x) discard;
  o = vec4(uColor.rgb * uColor.a, uColor.a);
}`);
      value = gl.createProgram();
      if (value === null) throw new Error("GPU program allocation failed");
      gl.attachShader(value, vertex);
      gl.attachShader(value, fragment);
      gl.linkProgram(value);
      if (!gl.getProgramParameter(value, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(value));
      const loc = {};
      for (const name of ["aPos", "aPrev", "aArc", "aCorner"]) loc[name] = gl.getAttribLocation(value, name);
      for (const name of ["uMode", "uDash", "uColor", "uFrontMin", "uMorph", "uPointSize", "uLabel", "uViewport", "uSize", "uUV", "uTex", "uAfterTex", "uMix", "uScreen", "uAlpha", "u_projection_matrix", "u_projection_tile_mercator_coords", "u_projection_clipping_plane", "u_projection_transition", "u_projection_fallback_matrix"]) loc[name] = gl.getUniformLocation(value, name);
      const program = { value, loc };
      target.programs.set(description.variantName, program);
      return program;
    } catch (error) {
      if (value !== null) gl.deleteProgram(value);
      throw error;
    } finally {
      gl.deleteShader(vertex);
      if (fragment !== null) gl.deleteShader(fragment);
    }
  }

  function projectionUniforms(gl, loc, data) {
    gl.uniformMatrix4fv(loc.u_projection_matrix, false, data.mainMatrix);
    gl.uniform4fv(loc.u_projection_tile_mercator_coords, data.tileMercatorCoords);
    gl.uniform4fv(loc.u_projection_clipping_plane, data.clippingPlane);
    gl.uniform1f(loc.u_projection_transition, data.projectionTransition);
    gl.uniformMatrix4fv(loc.u_projection_fallback_matrix, false, data.fallbackMatrix);
  }

  function coordinates(point) {
    const latitude = point[1] * Math.PI / 180, longitude = point[0] * Math.PI / 180;
    return [Math.cos(latitude) * Math.cos(longitude), Math.cos(latitude) * Math.sin(longitude), Math.sin(latitude)];
  }

  function upload(target, loc, points) {
    const resource = retained.uploadGeometry(target.gl, loc, Float32Array.from(points.flatMap(coordinates)));
    target.geometry.push(resource);
    return resource;
  }

  function buildScene(target, loc, collection) {
    const draws = [];
    for (const feature of collection.features) {
      const [paint, stroke, marker, label] = target.paint(feature.properties);
      const geometry = feature.geometry;
      if (geometry.type === "MultiPolygon" && (paint || stroke)) {
        for (const polygon of geometry.coordinates) draws.push({ rings: polygon.map(ring => upload(target, loc, ring)), paint, stroke });
      } else if (geometry.type === "MultiLineString" && stroke) {
        for (const course of geometry.coordinates) draws.push({ line: upload(target, loc, course), stroke });
      } else if (geometry.type === "Point" && marker) {
        draws.push({ point: upload(target, loc, [geometry.coordinates]), marker });
      }
      const recorded = feature.properties.label;
      if (label && recorded && recorded.anchor.type === "Point") {
        labels.initializeGlyphAtlas(target.glyphs, target.gl);
        const [style, voice] = label;
        const entry = labels.retainedGlyphEntry(target.glyphs, voice.uppercase ? recorded.text.toUpperCase() : recorded.text,
          voice, style.size, style.color, style.halo, style.halo_width_em);
        if (entry) draws.push({ glyph: glyphQuad(target, loc, recorded.anchor.coordinates), entry });
      }
    }
    return draws;
  }

  function surface(target) {
    const gl = target.gl;
    const surface = { framebuffer: null, texture: null, stencil: null, width: gl.drawingBufferWidth, height: gl.drawingBufferHeight };
    try {
      surface.framebuffer = gl.createFramebuffer();
      if (surface.framebuffer === null) throw new Error("GPU framebuffer allocation failed");
      surface.texture = gl.createTexture();
      if (surface.texture === null) throw new Error("GPU scene texture allocation failed");
      gl.bindTexture(gl.TEXTURE_2D, surface.texture);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, surface.width, surface.height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      surface.stencil = gl.createRenderbuffer();
      if (surface.stencil === null) throw new Error("GPU stencil allocation failed");
      gl.bindRenderbuffer(gl.RENDERBUFFER, surface.stencil);
      gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH24_STENCIL8, surface.width, surface.height);
      gl.bindFramebuffer(gl.FRAMEBUFFER, surface.framebuffer);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, surface.texture, 0);
      gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_STENCIL_ATTACHMENT, gl.RENDERBUFFER, surface.stencil);
      if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error("GPU framebuffer is incomplete");
      return surface;
    } catch (error) {
      if (surface.framebuffer !== null) gl.deleteFramebuffer(surface.framebuffer);
      if (surface.stencil !== null) gl.deleteRenderbuffer(surface.stencil);
      if (surface.texture !== null) gl.deleteTexture(surface.texture);
      throw error;
    }
  }

  function screenQuad(target, loc) {
    const resource = retained.uploadLobe(target.gl, loc, Float32Array.from([-1, -1, 0, -1, 1, 0, 1, -1, 0, 1, 1, 0]));
    target.geometry.push(resource);
    corners(target, loc, resource);
    return resource;
  }

  function glyphQuad(target, loc, point) {
    const resource = upload(target, loc, [point, point, point, point]);
    corners(target, loc, resource);
    return resource;
  }

  function corners(target, loc, resource) {
    const gl = target.gl;
    const buffer = gl.createBuffer();
    if (buffer === null) throw new Error("GPU glyph quad allocation failed");
    target.extras.push(buffer);
    gl.bindVertexArray(resource.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, Float32Array.from([0, 0, 0, 1, 1, 0, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(loc.aCorner);
    gl.vertexAttribPointer(loc.aCorner, 2, gl.FLOAT, false, 0, 0);
  }

  function drawScene(target, loc, draws, alpha) {
    const gl = target.gl;
    for (const draw of draws) {
      gl.uniform1f(loc.uLabel, draw.glyph ? 1 : 0);
      gl.uniform2f(loc.uDash, 0, 0);
      if (draw.rings) {
        if (draw.paint) retained.drawStencilFill(gl, loc, draw.rings, { mode: 1, whole: false, color: opacity(draw.paint.fill, alpha), frontMin: -1.1 });
        if (draw.stroke) for (const ring of draw.rings) stroke(gl, loc, ring, draw.stroke, alpha);
      } else if (draw.line) stroke(gl, loc, draw.line, draw.stroke, alpha);
      else if (draw.point) {
        colour(gl, loc, draw.marker.color, alpha);
        gl.uniform1f(loc.uPointSize, draw.marker.size * pixelRatio(gl));
        gl.bindVertexArray(draw.point.vao);
        gl.drawArrays(gl.POINTS, 0, draw.point.count);
      } else if (draw.glyph) {
        const glyphs = target.glyphs;
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, glyphs.tex);
        if (glyphs.dirty) {
          gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
          gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
          gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, glyphs.canvas);
          glyphs.dirty = false;
        }
        gl.uniform1i(loc.uTex, 0);
        gl.uniform1f(loc.uAlpha, alpha);
        gl.uniform2f(loc.uSize, draw.entry.w * pixelRatio(gl), draw.entry.h * pixelRatio(gl));
        gl.uniform4f(loc.uUV, draw.entry.u0, draw.entry.v0, draw.entry.u1, draw.entry.v1);
        gl.bindVertexArray(draw.glyph.vao);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      }
    }
  }

  function pixelRatio(gl) { return gl.drawingBufferWidth / (gl.canvas.clientWidth || gl.drawingBufferWidth); }
  function opacity(color, alpha) { return [color[0], color[1], color[2], color[3] * alpha]; }
  function colour(gl, loc, color, alpha) { gl.uniform4f(loc.uColor, ...opacity(color, alpha).map(channel => channel / 255)); }
  function stroke(gl, loc, resource, style, alpha) {
    if (style.pattern !== "Solid") throw new Error("Transition stroke pattern is unsupported");
    colour(gl, loc, style.color, alpha);
    gl.lineWidth(style.width * pixelRatio(gl));
    gl.bindVertexArray(resource.vao);
    gl.drawArrays(gl.LINE_STRIP, 0, resource.count);
  }

  function saveState(gl) {
    const values = {};
    for (const name of ["CURRENT_PROGRAM", "DRAW_FRAMEBUFFER_BINDING", "READ_FRAMEBUFFER_BINDING", "RENDERBUFFER_BINDING", "VIEWPORT", "COLOR_CLEAR_VALUE", "VERTEX_ARRAY_BINDING", "ARRAY_BUFFER_BINDING", "ACTIVE_TEXTURE", "TEXTURE_BINDING_2D", "UNPACK_PREMULTIPLY_ALPHA_WEBGL", "UNPACK_FLIP_Y_WEBGL", "COLOR_WRITEMASK", "LINE_WIDTH", "STENCIL_CLEAR_VALUE", "STENCIL_WRITEMASK", "STENCIL_FUNC", "STENCIL_REF", "STENCIL_VALUE_MASK", "STENCIL_FAIL", "STENCIL_PASS_DEPTH_FAIL", "STENCIL_PASS_DEPTH_PASS", "STENCIL_BACK_WRITEMASK", "STENCIL_BACK_FUNC", "STENCIL_BACK_REF", "STENCIL_BACK_VALUE_MASK", "STENCIL_BACK_FAIL", "STENCIL_BACK_PASS_DEPTH_FAIL", "STENCIL_BACK_PASS_DEPTH_PASS", "BLEND_SRC_RGB", "BLEND_DST_RGB", "BLEND_SRC_ALPHA", "BLEND_DST_ALPHA", "BLEND_EQUATION_RGB", "BLEND_EQUATION_ALPHA"]) values[name] = gl.getParameter(gl[name]);
    values.textures = [gl.TEXTURE0, gl.TEXTURE1].map(unit => {
      gl.activeTexture(unit);
      return gl.getParameter(gl.TEXTURE_BINDING_2D);
    });
    gl.activeTexture(values.ACTIVE_TEXTURE);
    values.enabled = [gl.BLEND, gl.DEPTH_TEST, gl.STENCIL_TEST].map(capability => gl.isEnabled(capability));
    return values;
  }

  function restoreState(gl, state) {
    gl.useProgram(state.CURRENT_PROGRAM);
    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, state.DRAW_FRAMEBUFFER_BINDING);
    gl.bindFramebuffer(gl.READ_FRAMEBUFFER, state.READ_FRAMEBUFFER_BINDING);
    gl.bindRenderbuffer(gl.RENDERBUFFER, state.RENDERBUFFER_BINDING);
    gl.viewport(...state.VIEWPORT);
    gl.clearColor(...state.COLOR_CLEAR_VALUE);
    gl.bindVertexArray(state.VERTEX_ARRAY_BINDING);
    gl.bindBuffer(gl.ARRAY_BUFFER, state.ARRAY_BUFFER_BINDING);
    state.textures.forEach((texture, index) => { gl.activeTexture(gl.TEXTURE0 + index); gl.bindTexture(gl.TEXTURE_2D, texture); });
    gl.activeTexture(state.ACTIVE_TEXTURE);
    gl.bindTexture(gl.TEXTURE_2D, state.TEXTURE_BINDING_2D);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, state.UNPACK_PREMULTIPLY_ALPHA_WEBGL);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, state.UNPACK_FLIP_Y_WEBGL);
    gl.colorMask(...state.COLOR_WRITEMASK);
    gl.lineWidth(state.LINE_WIDTH);
    gl.clearStencil(state.STENCIL_CLEAR_VALUE);
    gl.stencilMaskSeparate(gl.FRONT, state.STENCIL_WRITEMASK);
    gl.stencilMaskSeparate(gl.BACK, state.STENCIL_BACK_WRITEMASK);
    gl.stencilFuncSeparate(gl.FRONT, state.STENCIL_FUNC, state.STENCIL_REF, state.STENCIL_VALUE_MASK);
    gl.stencilFuncSeparate(gl.BACK, state.STENCIL_BACK_FUNC, state.STENCIL_BACK_REF, state.STENCIL_BACK_VALUE_MASK);
    gl.stencilOpSeparate(gl.FRONT, state.STENCIL_FAIL, state.STENCIL_PASS_DEPTH_FAIL, state.STENCIL_PASS_DEPTH_PASS);
    gl.stencilOpSeparate(gl.BACK, state.STENCIL_BACK_FAIL, state.STENCIL_BACK_PASS_DEPTH_FAIL, state.STENCIL_BACK_PASS_DEPTH_PASS);
    gl.blendFuncSeparate(state.BLEND_SRC_RGB, state.BLEND_DST_RGB, state.BLEND_SRC_ALPHA, state.BLEND_DST_ALPHA);
    gl.blendEquationSeparate(state.BLEND_EQUATION_RGB, state.BLEND_EQUATION_ALPHA);
    [gl.BLEND, gl.DEPTH_TEST, gl.STENCIL_TEST].forEach((capability, index) => state.enabled[index] ? gl.enable(capability) : gl.disable(capability));
  }

  const api = { transitionTarget, drawTransition, settleTransition, disposeTransition };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else Object.assign(scope, api);
})(typeof globalThis !== "undefined" ? globalThis : this);
