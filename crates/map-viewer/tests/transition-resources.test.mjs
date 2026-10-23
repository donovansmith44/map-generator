import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const limb = require('../src/limb.js');

function sourceTarget(label = null, realAtlas = false, realGpu = false) {
    const served = JSON.parse(fs.readFileSync(process.env.MG_TRANSITION_PAGE_FIXTURE, 'utf8')).body.transition.transition;
    // The archived transport fixture predates native xyz; adapt its coordinates
    // once at this test boundary, preserving the original shapes and references.
    const native = coordinate => Array.isArray(coordinate) && typeof coordinate[0] === 'number'
        ? { x: Math.cos(coordinate[1] * Math.PI / 180) * Math.cos(coordinate[0] * Math.PI / 180),
            y: Math.cos(coordinate[1] * Math.PI / 180) * Math.sin(coordinate[0] * Math.PI / 180), z: Math.sin(coordinate[1] * Math.PI / 180) }
        : coordinate.map(native);
    for (const shape of served.shapes) shape.geometry.coordinates = native(shape.geometry.coordinates);
    const original = served.origin.features[0];
    const geometry = servedGeometry(original.geometry, served.shapes);
    const collection = { ...served.origin, features: [{ ...original, geometry }], waters: [] };
    const live = new Set();
    const disposed = new Set();
    const counts = { geometry: 0, framebuffer: 0, texture: 0, stencil: 0 };
    const draws = [], images = [], painted = [], uploads = [];
    let boundBuffer;
    const clears = [], bufferClears = [], queries = [], uniforms = [], fills = [];
    let morphFraction, labelMode;
    let drawFramebuffer = 0, readFramebuffer = 0, canvasDraws = 0, finishes = 0;
    const resource = kind => { const value = { kind }; live.add(value); return value; };
    const gl = new Proxy({ drawingBufferWidth: 800, drawingBufferHeight: 600, canvas: { clientWidth: 800 } }, {
        get(object, name) {
            if (name in object) return object[name];
            if (name === 'getShaderParameter' || name === 'getProgramParameter') return () => true;
            if (name === 'getAttribLocation') return () => 0;
            if (name === 'getUniformLocation') return (_program, name) => ({ name });
            if (name === 'uniform1f') return (loc, value) => { uniforms.push({ name: loc.name, value }); if (loc.name === 'uMorph') morphFraction = value; if (loc.name === 'uLabel') labelMode = value; };
            if (name === 'getParameter') return parameter => { queries.push(parameter); return parameter === 'DRAW_FRAMEBUFFER_BINDING' ? drawFramebuffer
                : parameter === 'READ_FRAMEBUFFER_BINDING' ? readFramebuffer
                : ['VIEWPORT', 'COLOR_CLEAR_VALUE', 'COLOR_WRITEMASK'].includes(parameter) ? [0, 0, 800, 600] : 0; };
            if (name === 'bindBuffer') return (_binding, buffer) => { boundBuffer = buffer; };
            if (name === 'bufferData') return (_binding, data) => uploads.push({ buffer: boundBuffer, values: Array.from(data) });
            if (name === 'clear') return () => clears.push(drawFramebuffer);
            if (name === 'clearBufferfv' || name === 'clearBufferiv') return (buffer, index, values) => {
                bufferClears.push({ buffer, index, values: Array.from(values) });
                if (buffer === 'COLOR') clears.push(drawFramebuffer);
            };
            if (name === 'bindFramebuffer') return (binding, framebuffer) => {
                if (binding !== 'READ_FRAMEBUFFER') drawFramebuffer = framebuffer;
                if (binding !== 'DRAW_FRAMEBUFFER') readFramebuffer = framebuffer;
            };
            if (name === 'drawArrays') return (mode, first, count) => { draws.push({ mode, first, count }); painted.push({ mode, first, count, label: labelMode, framebuffer: drawFramebuffer }); if (drawFramebuffer === 0) canvasDraws++; };
            if (name === 'finish') return () => { finishes++; };
            if (name === 'checkFramebufferStatus') return () => 'FRAMEBUFFER_COMPLETE';
            if (name.startsWith('create')) return () => {
                if (name === 'createFramebuffer') counts.framebuffer++;
                if (name === 'createTexture') counts.texture++;
                if (name === 'createRenderbuffer') counts.stencil++;
                return resource(name);
            };
            if (name.startsWith('delete')) return value => live.delete(value);
            if (name === name.toUpperCase()) return name;
            return () => {};
        },
    });
    const gpuContext = { module: { exports: {} }, require: name => name === './limb.js' ? limb : require(name) };
    if (realGpu) vm.runInNewContext(fs.readFileSync(new URL('../src/retained-gpu.js', import.meta.url), 'utf8'), gpuContext);
    const retained = realGpu ? gpuContext.module.exports : {
        sphericalInterpolationSource() { return ''; },
        visibleFillDraws(_gl, _loc, ring) { return [ring]; },
        dropClipScratch() {},
        uploadGeometry(_gl, _loc, verts) { counts.geometry++; return { vao: resource('geometry'), count: verts.length / 3, verts }; },
        uploadLobe(_gl, _loc, verts) { counts.geometry++; return { vao: resource('screen'), count: verts.length / 3, verts }; },
        uploadPrevious() { return gl.createBuffer(); },
        drawStencilFill(_gl, _loc, _rings, options) { fills.push({ framebuffer: drawFramebuffer, fraction: morphFraction, whole: options.whole, color: options.color }); },
        disposeGeometry(_gl, geometry) { assert.equal(disposed.has(geometry), false, 'Each owned GPU geometry is disposed once.'); disposed.add(geometry); live.delete(geometry.vao); },
    };
    const atlas = realAtlas ? glyphAtlas(gl) : null;
    const labels = atlas?.api ?? {
        initializeGlyphAtlas(glyphs) { glyphs.tex ??= gl.createTexture(); },
        retainedGlyphEntry(glyphs, ...style) {
            const key = JSON.stringify(style);
            if (!glyphs.map.has(key)) { glyphs.map.set(key, { w: 1, h: 1, u0: 0, v0: 0, u1: 1, v1: 1 }); glyphs.dirty = true; }
            return glyphs.map.get(key);
        },
        retainedImageEntry(glyphs, marker) {
            images.push(marker);
            return { w: marker.image.width / marker.pixelRatio, h: marker.image.height / marker.pixelRatio, u0: 0, v0: 0, u1: 1, v1: 1 };
        },
        resetGlyphAtlas(glyphs) { glyphs.map.clear(); glyphs.x = glyphs.y = glyphs.rowH = 0; glyphs.dirty = true; },
        disposeGlyphAtlas(glyphs) { gl.deleteTexture(glyphs.tex); glyphs.tex = null; glyphs.map.clear(); },
    };
    const context = { module: { exports: {} }, require: name => name === './retained-gpu.js' ? retained : name === './limb.js' ? limb : labels };
    vm.runInNewContext(fs.readFileSync(new URL('../src/transition-target.js', import.meta.url), 'utf8'), context);
    const api = context.module.exports;
    const target = api.transitionTarget(gl, () => ({ shaderData: { variantName: 'test', vertexShaderPrelude: '', define: '' }, defaultProjectionData: {} }), () => [null, null, null, label], () => [], () => {}, reference => reference.id);
    return { api, target, gl, atlas, limb, counts, draws, painted, images, uploads, clears, bufferClears, queries, uniforms, fills, live, collection, waters: served.origin.waters, frames: served.keyframes, shapes: served.shapes, frame: { before: collection, after: collection },
        observed() { return { drawFramebuffer, readFramebuffer, canvasDraws, finishes }; } };
}

function glyphAtlas(gl) {
    const context2d = {
        clearRect() {},
        createImageData(width, height) { return { width, height, data: new Uint8ClampedArray(width * height * 4) }; },
        putImageData() {},
        measureText(text) { return { width: text.length * 8 }; },
        strokeText() {}, fillText() {},
    };
    const context = { module: { exports: {} }, window: { devicePixelRatio: 2 },
        document: { createElement() { return { getContext() { return context2d; } }; } } };
    vm.runInNewContext(fs.readFileSync(new URL('../src/retained-labels.js', import.meta.url), 'utf8'), context);
    return { api: context.module.exports, window: context.window };
}

function foreignState(gl) {
    const values = new Map(), textures = new Map(), enabled = new Set();
    const originalQuery = gl.getParameter;
    const setter = (name, change) => {
        const original = gl[name];
        gl[name] = (...args) => { original(...args); change(...args); };
    };
    const vector = (name, field) => setter(name, (...args) => values.set(field, args));
    gl.TEXTURE0 = 1000; gl.TEXTURE1 = 1001;
    gl.getParameter = parameter => {
        const original = originalQuery(parameter);
        return parameter === 'TEXTURE_BINDING_2D' ? textures.get(values.get('ACTIVE_TEXTURE'))
            : values.has(parameter) ? values.get(parameter) : original;
    };
    setter('useProgram', value => values.set('CURRENT_PROGRAM', value));
    setter('bindRenderbuffer', (_target, value) => values.set('RENDERBUFFER_BINDING', value));
    setter('bindVertexArray', value => values.set('VERTEX_ARRAY_BINDING', value));
    setter('bindBuffer', (_target, value) => values.set('ARRAY_BUFFER_BINDING', value));
    setter('activeTexture', value => values.set('ACTIVE_TEXTURE', value));
    setter('bindTexture', (_target, value) => textures.set(values.get('ACTIVE_TEXTURE'), value));
    setter('pixelStorei', (parameter, value) => values.set(parameter, value));
    setter('lineWidth', value => values.set('LINE_WIDTH', value));
    setter('clearStencil', value => values.set('STENCIL_CLEAR_VALUE', value));
    vector('viewport', 'VIEWPORT'); vector('clearColor', 'COLOR_CLEAR_VALUE'); vector('colorMask', 'COLOR_WRITEMASK');
    const faces = face => face === 'FRONT' ? ['STENCIL_'] : face === 'BACK' ? ['STENCIL_BACK_'] : ['STENCIL_', 'STENCIL_BACK_'];
    setter('stencilMaskSeparate', (face, mask) => { for (const prefix of faces(face)) values.set(prefix + 'WRITEMASK', mask); });
    setter('stencilMask', mask => { for (const prefix of faces()) values.set(prefix + 'WRITEMASK', mask); });
    const stencil = (face, fields, args) => { for (const prefix of faces(face)) fields.forEach((field, index) => values.set(prefix + field, args[index])); };
    setter('stencilFuncSeparate', (face, ...args) => stencil(face, ['FUNC', 'REF', 'VALUE_MASK'], args));
    setter('stencilFunc', (...args) => stencil(null, ['FUNC', 'REF', 'VALUE_MASK'], args));
    setter('stencilOpSeparate', (face, ...args) => stencil(face, ['FAIL', 'PASS_DEPTH_FAIL', 'PASS_DEPTH_PASS'], args));
    setter('stencilOp', (...args) => stencil(null, ['FAIL', 'PASS_DEPTH_FAIL', 'PASS_DEPTH_PASS'], args));
    const blend = (...args) => ['BLEND_SRC_RGB', 'BLEND_DST_RGB', 'BLEND_SRC_ALPHA', 'BLEND_DST_ALPHA'].forEach((field, index) => values.set(field, args[index]));
    setter('blendFuncSeparate', blend); setter('blendFunc', (source, destination) => blend(source, destination, source, destination));
    setter('blendEquationSeparate', (rgb, alpha) => { values.set('BLEND_EQUATION_RGB', rgb); values.set('BLEND_EQUATION_ALPHA', alpha); });
    setter('enable', capability => enabled.add(capability)); setter('disable', capability => enabled.delete(capability));
    gl.isEnabled = capability => enabled.has(capability);
    gl.useProgram({ foreign: 'program' }); gl.bindVertexArray({ foreign: 'vao' }); gl.bindBuffer(gl.ARRAY_BUFFER, { foreign: 'buffer' });
    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, { foreign: 'draw framebuffer' }); gl.bindFramebuffer(gl.READ_FRAMEBUFFER, { foreign: 'read framebuffer' });
    gl.bindRenderbuffer(gl.RENDERBUFFER, { foreign: 'renderbuffer' }); gl.viewport(3, 5, 701, 509);
    gl.clearColor(0.125, 0.25, 0.5, 0.75); gl.clearStencil(113); gl.colorMask(false, true, false, true); gl.lineWidth(2);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true); gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
    for (const [index, face] of ['FRONT', 'BACK'].entries()) {
        gl.stencilMaskSeparate(face, index ? 0x2a : 0x55); gl.stencilFuncSeparate(face, index ? 'LESS' : 'GREATER', index + 2, 0x7f);
        gl.stencilOpSeparate(face, 'INCR', 'DECR', 'REPLACE');
    }
    gl.blendFuncSeparate('SRC_ALPHA', 'ONE_MINUS_SRC_ALPHA', 'ONE', 'ZERO'); gl.blendEquationSeparate('FUNC_SUBTRACT', 'FUNC_REVERSE_SUBTRACT');
    for (const capability of [gl.BLEND, gl.SCISSOR_TEST, gl.CULL_FACE]) gl.enable(capability);
    for (const unit of [gl.TEXTURE0, gl.TEXTURE1, gl.TEXTURE0 + 4]) { gl.activeTexture(unit); gl.bindTexture(gl.TEXTURE_2D, { foreign: unit }); }
    return () => ({ values: [...values], textures: [...textures], enabled: [...enabled].sort(),
        draw: gl.getParameter(gl.DRAW_FRAMEBUFFER_BINDING), read: gl.getParameter(gl.READ_FRAMEBUFFER_BINDING) });
}

test('draw, preparation, resizing and failures preserve every foreign GL state value', () => {
    // Arrange
    for (const scenario of ['draw', 'prepare', 'resize', 'allocation', 'draw error']) {
        const fixture = sourceTarget(null, false, true);
        const snapshot = foreignState(fixture.gl), before = snapshot();
        const draw = () => fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.25);
        // Act
        if (scenario === 'prepare') fixture.api.prepareTransition(fixture.target, fixture.collection);
        else if (scenario === 'allocation') {
            fixture.gl.renderbufferStorage = () => { throw new Error('foreign allocation failure'); };
            assert.throws(draw, /foreign allocation failure/);
        } else if (scenario === 'draw error') {
            fixture.gl.drawArrays = () => { throw new Error('foreign draw failure'); };
            assert.throws(draw, /foreign draw failure/);
        } else {
            draw();
            if (scenario === 'resize') { fixture.gl.drawingBufferWidth++; draw(); }
        }
        // Assert
        assert.deepEqual(snapshot(), before, scenario + ' restores all incoming caller state.');
        fixture.api.disposeTransition(fixture.target);
        assert.equal(fixture.live.size, 0, scenario + ' releases all owned resources.');
    }
});

test('unchanged clear values need no queries and renderbuffer state is read only during surface allocation', () => {
    // Arrange
    const fixture = sourceTarget();
    const draw = fraction => fixture.api.drawTransition(fixture.target, fixture.frame, [], fraction);
    // Act
    draw(0);
    const allocated = fixture.queries.filter(value => value === 'RENDERBUFFER_BINDING').length;
    const start = fixture.queries.length;
    draw(0.25);
    // Assert
    assert.equal(allocated, 2, 'Each newly allocated surface owns its binding preservation.');
    assert.ok(!fixture.queries.includes('COLOR_CLEAR_VALUE') && !fixture.queries.includes('STENCIL_CLEAR_VALUE'), 'Explicit buffer clears never mutate these foreign values.');
    assert.ok(!fixture.queries.slice(start).includes('RENDERBUFFER_BINDING'), 'A retained surface does not change the renderbuffer binding.');
    assert.ok(fixture.queries.slice(start).includes('COLOR_WRITEMASK'), 'Every still-mutated mask retains its real incoming state query.');
    assert.deepEqual(fixture.bufferClears, Array.from({ length: 1 }, () => [
        { buffer: 'COLOR', index: 0, values: [0, 0, 0, 0] }, { buffer: 'STENCIL', index: 0, values: [0] },
    ]).flat(), 'Identical endpoint commands share one transparent color and zero stencil initialization.');
    fixture.gl.drawingBufferWidth++;
    draw(0.5);
    assert.equal(fixture.queries.filter(value => value === 'RENDERBUFFER_BINDING').length, allocated + 2, 'Resize preserves the binding for both replacement allocations.');
    fixture.api.disposeTransition(fixture.target);
});

function atlasStyle() {
    return [{ size: 12, color: [1, 2, 3, 255], halo: [255, 255, 255, 255] },
        { family: 'sans-serif', weight: 400, italic: false, tracking: 0, uppercase: false }];
}

function atlasMarker() {
    return { image: { width: 2, height: 2, data: new Uint8ClampedArray(16).fill(255) }, pixelRatio: 2 };
}

test('all original frames share one image-only atlas while releasing transient quads', () => {
    // Arrange
    const marker = atlasMarker(), fixture = sourceTarget(null, true);
    fixture.target.paint = () => [null, null, marker, null];
    const textures = [], markerEntries = [], frames = [];
    // Act
    for (const frame of fixture.frames) {
        const previous = fixture.target.geometry.map(resource => resource.vao);
        fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0);
        assert.ok(previous.every(resource => !fixture.live.has(resource)), 'Original frame changes release their transient geometry.');
        textures.push(fixture.target.glyphs.tex);
        markerEntries.push(fixture.target.glyphs.map.get(marker));
        frames.push(fixture.target.frame);
        fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0.5);
    }
    // Assert
    assert.ok(frames.length > 0);
    assert.deepEqual(frames, fixture.frames, 'All original frame objects draw in order with intermediate fractions.');
    assert.ok(textures[0] && textures.every(texture => texture === textures[0]), 'A transition retains its one original atlas texture.');
    assert.ok(markerEntries[0] && markerEntries.every(entry => entry === markerEntries[0]), 'Stable marker keys retain their exact raster entry.');
    fixture.api.resetTransition(fixture.target);
    assert.equal(fixture.live.has(textures[0]), false, 'Reset releases the atlas texture.');
    assert.equal(fixture.target.glyphs.map.size, 0);
    assert.equal(fixture.live.size, 1, 'Only the reusable shader program survives.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('a later text frame reuses atlas storage without retaining historical label capacity', () => {
    // Arrange
    const label = atlasStyle(), fixture = sourceTarget(label, true);
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const texture = fixture.target.glyphs.tex;
    let historical = 0;
    while (historical < 3000 && fixture.atlas.api.retainedGlyphEntry(fixture.target.glyphs,
        'prior-' + historical + '-'.repeat(100), label[1], label[0].size, label[0].color, label[0].halo)) historical++;
    assert.ok(historical > 0 && historical < 3000, 'Earlier frame labels fill the bounded atlas.');
    const feature = fixture.collection.features[0];
    const collection = { ...fixture.collection, features: [{ ...feature, properties: {
        ...feature.properties, label: { ...feature.properties.label, text: 'Current frame text' },
    } }] };
    const before = fixture.painted.filter(draw => draw.label === 1).length;
    // Act
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, [], 0.5);
    // Assert
    assert.ok(fixture.painted.filter(draw => draw.label === 1).length > before, 'The later label reaches a real glyph draw even after historical capacity was exhausted.');
    assert.equal(fixture.target.glyphs.tex, texture, 'Text frame changes retain the existing bounded texture allocation.');
    assert.equal(fixture.target.glyphs.map.size, 1, 'Only the current frame text occupies the rebuilt atlas.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('failed frame releases the retained marker atlas and all frame resources', () => {
    // Arrange
    const fixture = sourceTarget(null, true), marker = atlasMarker();
    fixture.target.paint = () => [null, null, marker, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const texture = fixture.target.glyphs.tex;
    const invalid = { before: fixture.collection, after: { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry: fixture.shapes[0].shape }] } };
    // Act
    assert.throws(() => fixture.api.drawTransition(fixture.target, invalid, [], 0), /Missing source geometry/);
    // Assert
    assert.equal(fixture.live.has(texture), false);
    assert.equal(fixture.target.glyphs.map.size, 0);
    assert.equal(fixture.live.size, 1, 'Only the reusable program remains after failure.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('repeated atlas disposal and initialization restart packing at the origin', () => {
    // Arrange
    const fixture = sourceTarget(null, true), glyphs = fixture.target.glyphs, marker = atlasMarker();
    // Act
    for (let cycle = 0; cycle < 3; cycle++) {
        fixture.atlas.api.initializeGlyphAtlas(glyphs, fixture.gl);
        const entry = fixture.atlas.api.retainedImageEntry(glyphs, marker);
        // Assert
        assert.equal(entry.u0, 0, 'Each new atlas starts at its original left edge.');
        assert.equal(entry.v0, 0, 'Each new atlas starts at its original top edge.');
        fixture.atlas.api.disposeGlyphAtlas(glyphs, fixture.gl);
        assert.deepEqual([glyphs.x, glyphs.y, glyphs.rowH, glyphs.dirty], [0, 0, 0, false]);
    }
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('text atlas keys include the pixel density used for the raster', () => {
    // Arrange
    const fixture = sourceTarget(null, true), glyphs = fixture.target.glyphs;
    const [style, voice] = atlasStyle();
    fixture.atlas.api.initializeGlyphAtlas(glyphs, fixture.gl);
    const entry = () => fixture.atlas.api.retainedGlyphEntry(glyphs, 'Text', voice, style.size, style.color, style.halo);
    // Act
    const doubleDensity = entry();
    fixture.atlas.window.devicePixelRatio = 1;
    const singleDensity = entry();
    // Assert
    assert.notEqual(singleDensity, doubleDensity, 'Changed DPR gets an independently rasterized entry.');
    assert.equal(entry(), singleDensity, 'The same consumed DPR reuses its raster.');
    fixture.atlas.window.devicePixelRatio = 2;
    assert.equal(entry(), doubleDensity, 'Returning to the original DPR reuses its correct raster.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('accepted morphs pack every original course and previous position with independent arc lengths', () => {
    // Arrange
    const fixture = sourceTarget(null, false, true), frame = fixture.frames.at(-1);
    const original = JSON.stringify(frame);
    // Act
    fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0.25);
    // Assert
    assert.ok(fixture.target.morphs.length > 0, 'The original frame exercises accepted source morphs.');
    assert.equal(fixture.target.geometry.length, fixture.target.morphs.length, 'Each accepted morph owns one shared geometry resource.');
    for (const [index, draw] of fixture.target.morphs.entries()) {
        const [row, ...companions] = draw.rows;
        const courses = [{ from: row.step.from_pts, to: row.step.to_pts }, ...companions.map(({ step }) => ({
            from: step.from ?? step.course, to: step.to ?? step.course,
        }))];
        const resource = fixture.target.geometry[index];
        const positions = Array.from(Float32Array.from(courses.flatMap(course => course.to.flatMap(point => [point.x, point.y, point.z]))));
        const previous = Array.from(Float32Array.from(courses.flatMap(course => course.from.flatMap(point => [point.x, point.y, point.z]))));
        assert.deepEqual(fixture.uploads.find(upload => upload.buffer === resource.buf).values, positions, 'Packed destination vertices exactly preserve every original component.');
        assert.deepEqual(fixture.uploads.find(upload => upload.buffer === fixture.target.extras[index]).values, previous, 'Packed previous positions match the same original course ordering.');
        const views = [draw.items[0].rings[0], ...draw.items.filter(item => item.line?.vao === resource.vao).map(item => item.line)];
        const ranges = [], arcs = [];
        let first = 0;
        for (const course of courses) {
            ranges.push([first, course.to.length]);
            let length = 0;
            for (let at = 0; at < course.to.length; at++) {
                if (at) length += Math.hypot(...[0, 1, 2].map(axis => positions[(first + at) * 3 + axis] - positions[(first + at - 1) * 3 + axis]));
                arcs.push(length);
            }
            first += course.to.length;
        }
        assert.deepEqual(views.map(view => [view.first, view.count]), ranges, 'Every original course retains a separate draw range.');
        assert.deepEqual(fixture.uploads.find(upload => upload.buffer === resource.abuf).values, Array.from(Float32Array.from(arcs)), 'Distance restarts independently at every course boundary.');
        const fades = Array.from(draw.items.filter(item => item.line?.vao === resource.vao), item => item.appears);
        assert.deepEqual(fades, companions.map(row => row.step.appears), 'Companion fade flags retain the original row order.');
    }
    assert.equal(JSON.stringify(frame), original, 'Batching changes no original frame, course, or metadata.');
    fixture.api.resetTransition(fixture.target);
    assert.equal(fixture.live.size, 1, 'Reset releases all packed buffers and keeps only the reusable shader.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('failed packed previous-position upload releases every partially created morph buffer', () => {
    // Arrange
    const fixture = sourceTarget(null, false, true), allocate = fixture.gl.createBuffer;
    let calls = 0;
    fixture.gl.createBuffer = () => ++calls === 3 ? null : allocate();
    // Act
    assert.throws(() => fixture.api.drawTransition(fixture.target, fixture.frames.at(-1), fixture.shapes, 0), /GPU previous-position allocation failed/);
    // Assert
    assert.equal(fixture.target.geometry.length, 0);
    assert.equal(fixture.target.extras.length, 0);
    assert.equal(fixture.live.size, 1, 'Failure releases both geometry buffers and its vertex array.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('unchanged endpoint textures render once while every original fraction composites and morphs', () => {
    // Arrange
    const fixture = sourceTarget();
    const frame = fixture.frames.at(-1);
    const fractions = [0, 0.25, 0.75];
    fixture.target.paint = () => [{ fill: [1, 2, 3, 255] }, { color: [4, 5, 6, 255], width: 1, pattern: 'Solid' }, null, null];
    // Act
    for (const fraction of fractions) fixture.api.drawTransition(fixture.target, frame, fixture.shapes, fraction);
    // Assert
    assert.equal(fixture.clears.length, 2, 'Only the two unchanged endpoint textures are rendered once.');
    assert.deepEqual(fixture.uniforms.filter(value => value.name === 'uMix').map(value => value.value), fractions, 'The composite still uses every original fraction.');
    for (const fraction of fractions) assert.equal(fixture.fills.filter(draw => draw.framebuffer === 0 && draw.fraction === fraction).length, fixture.target.morphs.length, 'Every original morph draws at every fraction.');
    assert.ok(fixture.target.morphs.length > 0, 'The original final frame exercises its actual morphs.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Cached textures and their resources are released normally.');
});

test('endpoint reuse compares exact consumed projection values and preserves GL state reads', () => {
    // Arrange
    const fixture = sourceTarget();
    const original = fixture.target.projection();
    const data = { mainMatrix: new Float32Array(16), tileMercatorCoords: new Float32Array(4), clippingPlane: new Float32Array(4), fallbackMatrix: new Float32Array(16), projectionTransition: 0 };
    fixture.target.projection = () => ({ ...original, defaultProjectionData: { ...data, mainMatrix: Float32Array.from(data.mainMatrix), fallbackMatrix: Float32Array.from(data.fallbackMatrix), tileMercatorCoords: Float32Array.from(data.tileMercatorCoords), clippingPlane: Float32Array.from(data.clippingPlane) } });
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const reads = fixture.queries.length;
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.1);
    // Assert
    assert.equal(fixture.clears.length, 1, 'New projection arrays with the same values reuse the original endpoint textures.');
    assert.deepEqual(fixture.queries.slice(reads), fixture.queries.slice(0, reads).filter(value => value !== 'RENDERBUFFER_BINDING'), 'Every fraction still saves all mutated incoming GL state without repeating allocation-only reads.');
    let expected = 1;
    for (const [index, field] of ['mainMatrix', 'tileMercatorCoords', 'clippingPlane', 'projectionTransition', 'fallbackMatrix'].entries()) {
        if (field === 'projectionTransition') data[field]++;
        else data[field][0]++;
        fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.2 + index * 0.1);
        assert.equal(fixture.clears.length, expected += 1, 'Every consumed projection uniform value invalidates the textures.');
    }
    fixture.api.resetTransition(fixture.target);
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    assert.equal(fixture.clears.length, expected + 1, 'Reset never reuses textures from an earlier transition.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Projection and reset invalidation leak no resources.');
});

test('mutated paint and water styles invalidate cached endpoint pixels by value', () => {
    // Arrange
    const fixture = sourceTarget();
    const water = fixture.waters[0];
    const collection = { ...fixture.collection, features: [], waters: [water] };
    const frame = { before: collection, after: collection };
    const paint = { fill: [1, 2, 3, 255] }, stroke = { color: [4, 5, 6, 255], width: 1, pattern: 'Solid' };
    fixture.target.water = () => [paint, stroke];
    // Act
    fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0);
    fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0.1);
    // Assert
    assert.equal(fixture.clears.length, 1, 'Stable prepared water presentation reuses the textures.');
    let expected = 1;
    for (const change of [() => paint.fill[0]++, () => stroke.color[1]++, () => stroke.width++, () => { fixture.target.water = () => [null, stroke]; }]) {
        change();
        fixture.api.drawTransition(fixture.target, frame, fixture.shapes, expected / 20);
        assert.equal(fixture.clears.length, expected += 1, 'Mutating an existing fill/stroke value changes the actual endpoint pixels.');
    }
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Water presentation invalidation keeps normal ownership.');
});

test('marker size color and pixel ratio invalidate endpoint textures', () => {
    // Arrange
    const fixture = sourceTarget();
    const marker = { color: [1, 2, 3, 255], size: 4 };
    fixture.target.paint = () => [null, null, marker, null];
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.1);
    // Assert
    assert.equal(fixture.clears.length, 1, 'Stable marker presentation reuses endpoint textures.');
    let expected = 1;
    for (const change of [() => marker.size++, () => marker.color[2]++, () => { fixture.gl.canvas.clientWidth /= 2; }]) {
        change();
        fixture.api.drawTransition(fixture.target, fixture.frame, [], expected / 20);
        assert.equal(fixture.clears.length, expected += 1, 'Each effective marker pixel input invalidates the textures.');
    }
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Marker invalidation keeps normal cleanup.');
});

test('glyph atlas content changes invalidate textures even with identical glyph bounds', () => {
    // Arrange
    const style = { size: 12, color: [1, 2, 3, 255], halo: [4, 5, 6, 255], halo_width_em: 0.1 };
    const fixture = sourceTarget([style, { uppercase: false }]);
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.1);
    // Assert
    assert.equal(fixture.clears.length, 1, 'Identical effective glyph content reuses the identical endpoint pixels.');
    style.color[0]++;
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.2);
    assert.equal(fixture.clears.length, 2, 'A glyph color mutation rerenders the shared endpoint pixels despite unchanged UV bounds.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Glyph invalidation releases the atlas and cached endpoint resources.');
});

function retainedTarget() {
    const live = new Set(), uploads = [], draws = [];
    const gl = new Proxy({}, {
        get(object, name) {
            if (name in object) return object[name];
            if (name.startsWith('create')) return () => { const resource = {}; live.add(resource); return resource; };
            if (name.startsWith('delete')) return resource => live.delete(resource);
            if (name === 'bufferData') return (_target, data) => uploads.push(Array.from(data));
            if (name === 'drawArrays') return (mode, first, count) => draws.push({ mode, first, count });
            if (name === name.toUpperCase()) return name;
            return () => {};
        },
    });
    const context = { module: { exports: {} } };
    vm.runInNewContext(fs.readFileSync(new URL('../src/retained-gpu.js', import.meta.url), 'utf8'), context);
    return { api: context.module.exports, gl, live, uploads, draws, loc: { aPos: 0, aPrev: 1, aArc: 2 } };
}

test('one geometry upload preserves independent course ranges and arclength resets', () => {
    // Arrange
    const fixture = retainedTarget();
    const vertices = Float32Array.from([0, 0, 0, 3, 0, 0, 3, 4, 0, 10, 0, 0, 10, 0, 5]);
    const ranges = [{ first: 0, count: 3 }, { first: 3, count: 2 }];
    // Act
    const resource = fixture.api.uploadGeometry(fixture.gl, fixture.loc, vertices, ranges);
    // Assert
    assert.deepEqual(fixture.uploads, [Array.from(vertices), [0, 3, 7, 0, 5]], 'Each original course resets distance without a connector between ranges.');
    assert.equal(fixture.live.size, 3, 'All ranges share one vertex array and two buffers.');
    fixture.api.disposeGeometry(fixture.gl, resource);
    assert.equal(fixture.live.size, 0, 'Only the shared resource owns its GPU handles.');
});

test('stencil fill preserves polygon and hole ranges through both passes', () => {
    // Arrange
    const fixture = retainedTarget();
    const vertices = new Float32Array(24);
    const resource = fixture.api.uploadGeometry(fixture.gl, fixture.loc, vertices);
    const rings = [{ ...resource, first: 0, count: 4 }, { ...resource, first: 4, count: 4 }];
    const expected = [0, 4, 0, 4].map(first => ({ mode: 'TRIANGLE_FAN', first, count: 4 }));
    // Act
    fixture.api.drawStencilFill(fixture.gl, fixture.loc, rings, { whole: false, mode: 1, color: [1, 2, 3, 255], frontMin: -1.1 });
    // Assert
    assert.deepEqual(fixture.draws, expected, 'Original outer and hole rings retain their offsets, order and separate stencil passes.');
    fixture.api.disposeGeometry(fixture.gl, resource);
    assert.equal(fixture.live.size, 0, 'Non-owning ring views leave no extra resources.');
});

test('ordinary and paired uploads preserve the original contiguous adapter API', () => {
    // Arrange
    const fixture = retainedTarget();
    const vertices = Float32Array.from([0, 0, 0, 3, 4, 0]);
    const previous = Float32Array.from([0, 0, 0, 0, 0, 5]);
    // Act
    const resource = fixture.api.uploadGeometry(fixture.gl, fixture.loc, vertices);
    const buffer = fixture.api.uploadPrevious(fixture.gl, fixture.loc, resource, previous);
    // Assert
    assert.deepEqual(fixture.uploads, [Array.from(vertices), [0, 5], Array.from(previous)], 'Existing callers retain their contiguous arclengths and complete previous-position upload.');
    fixture.api.endPrevious(fixture.gl, fixture.loc, resource, buffer);
    fixture.api.disposeGeometry(fixture.gl, resource);
    assert.equal(fixture.live.size, 0, 'The old paired-resource lifecycle remains unchanged.');
});

test('an incomplete shared upload releases every partially allocated handle', () => {
    // Arrange
    const fixture = retainedTarget();
    const create = fixture.gl.createBuffer;
    let count = 0;
    fixture.gl.createBuffer = () => ++count === 2 ? null : create();
    // Act
    const upload = () => fixture.api.uploadGeometry(fixture.gl, fixture.loc, new Float32Array(12), [{ first: 0, count: 2 }, { first: 2, count: 2 }]);
    // Assert
    assert.throws(upload, /GPU arc buffer allocation failed/, 'The adapter exposes the original allocation failure.');
    assert.equal(fixture.live.size, 0, 'The shared upload releases both its vertex array and its earlier buffer.');
});

test('multiline courses share one upload while preserving original stroke draw ranges', () => {
    // Arrange
    const fixture = sourceTarget();
    const shape = fixture.shapes.find(shape => shape.geometry.type === 'MultiLineString' && shape.geometry.coordinates.length > 1);
    const collection = { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry: shape.shape }] };
    const expected = [];
    let first = 0;
    for (const course of shape.geometry.coordinates) { expected.push({ mode: 'LINE_STRIP', first, count: course.length }); first += course.length; }
    fixture.target.paint = () => [null, { color: [1, 2, 3, 255], width: 1, pattern: 'Solid' }, null, null];
    // Act
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, fixture.shapes, 0);
    // Assert
    assert.equal(fixture.counts.geometry, 2, 'All original line courses share one static resource plus the screen quad.');
    assert.deepEqual(fixture.draws.filter(draw => draw.mode === 'LINE_STRIP'), expected, 'The shared endpoint raster draws every original course in order without connecting adjacent courses.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Shared course views dispose their common GPU resource exactly once.');
});

test('multipolygon rings share one resource while retaining every original boundary', () => {
    // Arrange
    const fixture = sourceTarget();
    const shape = fixture.shapes.find(shape => shape.geometry.type === 'MultiPolygon' && shape.geometry.coordinates.flat().length > 1);
    const collection = { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry: shape.shape }] };
    // Act
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, fixture.shapes, 0);
    // Assert
    const items = fixture.target.scenes[0][0].items;
    assert.equal(fixture.counts.geometry, 2, 'Every original polygon/ring shares one upload plus the screen quad.');
    assert.deepEqual(Array.from(items, item => item.rings.length), shape.geometry.coordinates.map(polygon => polygon.length), 'Each original polygon retains its outer ring and every hole.');
    const rings = items.flatMap(item => item.rings);
    assert.ok(rings.every(ring => ring.vao === rings[0].vao), 'Ring views share the one geometry-owned vertex array.');
    assert.deepEqual(Array.from(rings, ring => ring.count), shape.geometry.coordinates.flat().map(ring => ring.length), 'Every original ring retains its exact vertex count.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'All ring views release their common resource once.');
});

test('hidden and image-only points never upload unused point geometry', () => {
    // Arrange
    const fixture = sourceTarget(null, true);
    const marker = atlasMarker();
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    // Assert
    assert.equal(fixture.target.cachedGeometry.length, 0, 'A hidden point has no static GPU resource.');
    fixture.target.paint = () => [null, null, marker, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.5);
    assert.equal(fixture.target.cachedGeometry.length, 0, 'The image marker uses only its own anchored quad.');
    assert.ok(fixture.painted.some(draw => draw.label === 1), 'Skipping unused point geometry still paints the marker image.');
    assert.equal(fixture.draws.filter(draw => draw.mode === 'POINTS').length, 0);
    fixture.api.resetTransition(fixture.target);
    assert.equal(fixture.live.size, 1, 'Reset releases every image resource and keeps only its shader.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('a newly visible plain marker uploads once and reuses its original point resource', () => {
    // Arrange
    const fixture = sourceTarget();
    const marker = { color: [1, 2, 3, 255], size: 4 };
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const before = fixture.counts.geometry;
    fixture.target.paint = () => [null, null, marker, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.25);
    const resource = fixture.target.cachedGeometry[0];
    // Assert
    assert.equal(fixture.counts.geometry, before + 1, 'The first visible plain marker uploads its original point once.');
    assert.ok(fixture.draws.some(draw => draw.mode === 'POINTS'), 'The newly visible point reaches its existing GPU primitive.');
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.5);
    assert.equal(fixture.counts.geometry, before + 1, 'Later fractions reuse the same point resource.');
    assert.equal(fixture.target.scenes[0][0].items[0].point.vao, resource.vao);
    const pointDraws = fixture.draws.filter(draw => draw.mode === 'POINTS').length;
    fixture.target.paint = () => [null, null, null, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.75);
    assert.equal(fixture.draws.filter(draw => draw.mode === 'POINTS').length, pointDraws, 'A later hidden style stops drawing the retained point.');
    fixture.api.resetTransition(fixture.target);
    assert.equal(fixture.live.has(resource.vao), false, 'Reset releases the lazily allocated resource.');
    assert.equal(fixture.live.size, 1);
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('all original served frames retain shared static GPU geometry until reset', () => {
    // Arrange
    const fixture = sourceTarget();
    fixture.target.paint = () => [null, null, { color: [1, 2, 3, 255], size: 4 }, null];
    const original = JSON.stringify(fixture.frames);
    const first = fixture.collection.features[0];
    const resources = [];
    const order = [];
    // Act
    for (const frame of fixture.frames) {
        const previous = fixture.target.geometry.map(resource => resource.vao);
        fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0);
        assert.ok(previous.every(resource => !fixture.live.has(resource)), 'Transient morph and glyph buffers are released on each frame change.');
        order.push(fixture.target.frame);
        const draw = fixture.target.scenes[0].find(draw => draw.feature?.id === first.id);
        resources.push(draw.items[0].point);
        fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0.5);
    }
    // Assert
    assert.equal(order.length, fixture.frames.length, 'All original served frames draw and interpolate.');
    assert.deepEqual(order, fixture.frames, 'Original frame objects retain their exact served ordering.');
    assert.ok(resources.every(resource => resource === resources[0]), 'Repeated static geometry keeps its original GPU resource across every frame.');
    assert.ok(fixture.live.has(resources[0].vao), 'A frame change retains the static buffer.');
    assert.equal(JSON.stringify(fixture.frames), original, 'Resource reuse changes no frame or geometry value.');
    const previous = resources[0].vao;
    fixture.api.resetTransition(fixture.target);
    assert.equal(fixture.live.has(previous), false, 'Reset releases the whole transition cache.');
    assert.equal(fixture.live.size, 1, 'Only the warm shader program survives reset.');
    fixture.api.drawTransition(fixture.target, fixture.frames[0], fixture.shapes, 0);
    assert.notEqual(fixture.target.scenes[0].find(draw => draw.feature?.id === first.id).items[0].point.vao, previous, 'Another transition starts with newly owned buffers.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Disposal releases the next transition too.');
});

test('shared GPU buffers retain separate paint wrappers for each original scene', () => {
    // Arrange
    const fixture = sourceTarget();
    const before = fixture.collection;
    const after = { ...before, features: [{ ...before.features[0], properties: { ...before.features[0].properties } }] };
    const first = { fill: [1, 2, 3, 255] }, second = { fill: [4, 5, 6, 255] };
    fixture.target.paint = properties => [properties === before.features[0].properties ? first : second, null, { color: [1, 2, 3, 255], size: 4 }, null];
    // Act
    fixture.api.drawTransition(fixture.target, { before, after }, [], 0.5);
    // Assert
    const left = fixture.target.scenes[0][0].items[0], right = fixture.target.scenes[1][0].items[0];
    assert.equal(left.point, right.point, 'Both original occurrences reuse the same GPU geometry.');
    assert.notEqual(left, right, 'Paint state is private to each occurrence.');
    assert.equal(left.paint, first, 'The before scene keeps its original paint.');
    assert.equal(right.paint, second, 'The after scene keeps its own original paint.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Aliased geometry is released exactly once.');
});

test('shader program changes retain independently bound GPU resources for the same geometry', () => {
    // Arrange
    const fixture = sourceTarget();
    fixture.target.paint = () => [null, null, { color: [1, 2, 3, 255], size: 4 }, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const first = fixture.target.scenes[0][0].items[0].point;
    const projection = fixture.target.projection;
    // Act
    fixture.target.projection = () => ({ ...projection(), shaderData: { ...projection().shaderData, variantName: 'other' } });
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.5);
    const second = fixture.target.scenes[0][0].items[0].point;
    fixture.target.projection = projection;
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.5);
    // Assert
    assert.notEqual(second, first, 'Another compiled program receives its own attribute bindings.');
    assert.equal(fixture.target.scenes[0][0].items[0].point, first, 'Returning to a program reuses its original bound geometry.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Disposal releases all program variants and their buffers.');
});

test('a failed frame releases geometry cached by an earlier frame', () => {
    // Arrange
    const fixture = sourceTarget();
    fixture.target.paint = () => [null, null, { color: [1, 2, 3, 255], size: 4 }, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const previous = fixture.target.scenes[0][0].items[0].point.vao;
    const invalid = { before: fixture.collection, after: { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry: fixture.shapes[0].shape }] } };
    // Act
    assert.throws(() => fixture.api.drawTransition(fixture.target, invalid, [], 0), /Missing source geometry/);
    // Assert
    assert.equal(fixture.live.has(previous), false, 'An error releases geometry from previous frames as well.');
    assert.equal(fixture.live.size, 1, 'Only the reusable shader program remains after failure.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Final disposal releases the remaining shader.');
});

test('shared feature and water references draw the original dictionary geometry without changing the frame', () => {
    // Arrange
    const fixture = sourceTarget();
    const shape = fixture.shapes[0];
    const feature = { ...fixture.collection.features[0], geometry: shape.shape };
    const water = { geometry: shape.shape };
    const collection = { ...fixture.collection, features: [feature], waters: [water] };
    const frame = { before: collection, after: collection };
    const original = JSON.stringify({ frame, shapes: fixture.shapes });
    const coordinates = shape.geometry.coordinates;
    let consumed = 0;
    Object.defineProperty(shape.geometry, 'coordinates', { configurable: true, enumerable: true, get() { consumed++; return coordinates; } });
    // Act
    fixture.api.drawTransition(fixture.target, frame, fixture.shapes, 0);
    // Assert
    assert.equal(fixture.target.frame, frame, 'Drawing retains the original served frame.');
    assert.equal(consumed, 1, 'Every feature and water occurrence shares one upload of the original geometry object.');
    assert.equal(fixture.target.scenes[0][0].water, water, 'Water metadata keeps its original object identity.');
    assert.equal(fixture.target.scenes[0][1].feature, feature, 'Feature metadata keeps its original object identity.');
    assert.equal(JSON.stringify({ frame, shapes: fixture.shapes }), original, 'Reference resolution changes no served value.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Shared drawing releases all GPU resources.');
});

test('a missing shared geometry refuses the draw and releases its frame resources', () => {
    // Arrange
    const fixture = sourceTarget();
    const frame = { before: { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry: fixture.shapes[0].shape }] }, after: fixture.collection };
    // Act
    const draw = () => fixture.api.drawTransition(fixture.target, frame, [], 0);
    // Assert
    assert.throws(draw, /Missing source geometry/, 'A missing served reference is a visible renderer failure.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Malformed reference drawing leaks no GPU resources.');
});

test('preparing an original scene completes its GPU work offscreen and retains only its shader program', () => {
    // Arrange
    const fixture = sourceTarget();
    const program = fixture.target.programs.get('test');
    const expected = { drawFramebuffer: 0, readFramebuffer: 0, canvasDraws: 0, finishes: 1 };
    // Act
    fixture.api.prepareTransition(fixture.target, fixture.collection);
    // Assert
    assert.deepEqual(fixture.observed(), expected, 'Preparation finishes its real draw without touching the canvas or changing framebuffer bindings.');
    assert.equal(fixture.target.programs.get('test'), program, 'Preparation retains the exact compiled program for the first interaction.');
    assert.equal(fixture.live.size, 1, 'Preparation releases every frame, texture and scratch resource while retaining one program.');
    assert.equal(fixture.target.frame, null, 'Preparation cannot count as a played served frame.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Map disposal releases the retained warm program.');
});

test('failed offscreen preparation restores the canvas binding and releases every temporary resource', () => {
    // Arrange
    const fixture = sourceTarget();
    const failure = new Error('The GPU preparation failed.');
    fixture.gl.finish = () => { throw failure; };
    const expected = { drawFramebuffer: 0, readFramebuffer: 0, canvasDraws: 0, finishes: 0 };
    // Act
    assert.throws(() => fixture.api.prepareTransition(fixture.target, fixture.collection), failure, 'Preparation exposes its original GPU failure.');
    // Assert
    assert.deepEqual(fixture.observed(), expected, 'A failed preparation restores both original framebuffer bindings.');
    assert.equal(fixture.live.size, 1, 'Failure releases every temporary resource while preserving the reusable shader program.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Disposal after failed preparation releases the final program.');
});

test('ordinary map labels allocate no unused canonical glyph geometry', () => {
    // Arrange
    const fixture = sourceTarget();
    const expected = { geometry: 1, framebuffer: 2, texture: 2, stencil: 2 };
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    // Assert
    assert.deepEqual(fixture.counts, expected, 'Only the screen quad is allocated when the original point has no visible canonical marker.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Disposal releases every allocated GPU resource.');
});

test('canonical labels allocate their glyph geometry only once per scene', () => {
    // Arrange
    const fixture = sourceTarget([{ size: 12 }, { uppercase: false }]);
    const expected = { geometry: 3, framebuffer: 2, texture: 3, stencil: 2 };
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.5);
    // Assert
    assert.deepEqual(fixture.counts, expected, 'Both canonical labels retain their own glyph resource across interpolations.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Disposal releases canonical glyph resources too.');
});

test('served frame changes retain the same two scene surfaces and screen quad', () => {
    // Arrange
    const fixture = sourceTarget();
    fixture.target.paint = () => [null, null, { color: [1, 2, 3, 255], size: 4 }, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const surfaces = fixture.target.surfaces;
    const screen = fixture.target.screen;
    const next = { before: fixture.collection, after: fixture.collection };
    const expected = { geometry: 2, framebuffer: 2, texture: 2, stencil: 2 };
    // Act
    fixture.api.drawTransition(fixture.target, next, [], 0);
    // Assert
    assert.equal(fixture.target.surfaces, surfaces, 'Every served frame shares the same size-dependent surfaces.');
    assert.equal(fixture.target.screen, screen, 'Every served frame shares the same screen quad.');
    assert.deepEqual(fixture.counts, expected, 'Frame changes reuse both static geometry and viewport resources.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'The final frame releases both retained and frame-specific resources.');
});

test('viewport resizing replaces scene surfaces and releases the previous resources', () => {
    // Arrange
    const fixture = sourceTarget();
    fixture.target.paint = () => [null, null, { color: [1, 2, 3, 255], size: 4 }, null];
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    const previous = fixture.target.surfaces[0];
    const geometry = fixture.target.scenes[0][0].items[0].point;
    fixture.gl.drawingBufferWidth = 1200;
    const expected = { geometry: 3, framebuffer: 4, texture: 4, stencil: 4 };
    // Act
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.5);
    // Assert
    assert.deepEqual(fixture.counts, expected, 'Resizing allocates exactly two replacement surfaces and one replacement screen.');
    assert.equal(fixture.target.scenes[0][0].items[0].point, geometry, 'Viewport-independent static coordinates keep their existing GPU buffer.');
    assert.equal(fixture.live.has(previous.framebuffer), false, 'The previous framebuffer is released during resizing.');
    assert.equal(fixture.live.has(previous.texture), false, 'The previous texture is released during resizing.');
    assert.equal(fixture.live.has(previous.stencil), false, 'The previous stencil is released during resizing.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Disposal after resizing releases every replacement resource.');
});

test('adjacent swapped endpoints reuse both original owned surfaces', () => {
    // Arrange
    const fixture = sourceTarget(), [before, after] = endpointScenes(fixture);
    // Act
    fixture.api.drawTransition(fixture.target, { before, after }, [], 0);
    fixture.api.drawTransition(fixture.target, { before: after, after: before }, [], 0.5);
    // Assert
    assert.equal(fixture.clears.length, 2, 'Swapping the exact endpoint commands never rerasterizes either texture.');
    assert.equal(fixture.counts.framebuffer, 2, 'The transition owns exactly two physical surfaces.');
    assert.equal(new Set(fixture.target.surfaces).size, 2, 'Logical swaps never alias physical resource ownership.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Both swapped surfaces and their GPU resources are released.');
});

test('identical logical endpoints share one raster while every fraction composites', () => {
    // Arrange
    const fixture = sourceTarget(), [before] = endpointScenes(fixture);
    const fractions = [0, 0.25, 0.75];
    // Act
    for (const fraction of fractions) fixture.api.drawTransition(fixture.target, { before, after: before }, [], fraction);
    // Assert
    assert.equal(fixture.clears.length, 1, 'Identical prepared endpoint commands need one raster across original frame changes.');
    assert.deepEqual(fixture.uniforms.filter(value => value.name === 'uMix').map(value => value.value), fractions);
    assert.equal(fixture.counts.framebuffer, 2, 'Sharing pixels retains two separately owned physical surfaces.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0, 'Shared logical pixels cannot cause duplicate disposal or an orphan surface.');
});

test('a later matching endpoint is reserved before an earlier miss renders', () => {
    // Arrange
    const fixture = sourceTarget(), [before, after, replacement] = endpointScenes(fixture);
    // Act
    fixture.api.drawTransition(fixture.target, { before, after }, [], 0);
    const [reserved, replaceable] = fixture.clears;
    fixture.api.drawTransition(fixture.target, { before: replacement, after: before }, [], 0.5);
    // Assert
    assert.deepEqual(fixture.clears, [reserved, replaceable, replaceable], 'The earlier miss cannot overwrite the first physical texture needed by the later endpoint.');
    assert.equal(fixture.counts.framebuffer, 2);
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('endpoint geometry changes invalidate pixels even when served properties are identical', () => {
    // Arrange
    const fixture = sourceTarget(), [before] = endpointScenes(fixture);
    const changed = { ...before, features: before.features.map(feature => ({ ...feature, geometry: structuredClone(feature.geometry) })) };
    // Act
    fixture.api.drawTransition(fixture.target, { before, after: before }, [], 0);
    fixture.api.drawTransition(fixture.target, { before: changed, after: changed }, [], 0.5);
    // Assert
    assert.equal(fixture.clears.length, 2, 'Newly uploaded GPU geometry never borrows an earlier resource key.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('endpoint projection program changes invalidate retained pixels and release old surfaces', () => {
    // Arrange
    const fixture = sourceTarget(), [before] = endpointScenes(fixture);
    const original = fixture.target.projection();
    // Act
    fixture.api.drawTransition(fixture.target, { before, after: before }, [], 0);
    const old = [...fixture.target.surfaces];
    fixture.target.projection = () => ({ ...original, shaderData: { ...original.shaderData, variantName: original.shaderData.variantName + '-next' } });
    fixture.api.drawTransition(fixture.target, { before, after: before }, [], 0.5);
    // Assert
    assert.equal(fixture.clears.length, 2, 'A different compiled projection program renders its own endpoint pixels.');
    assert.ok(old.every(surface => !fixture.live.has(surface.framebuffer)), 'No surface from the previous program remains live.');
    assert.equal(fixture.target.surfaces.length, 2);
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

test('atlas content changes invalidate retained pixels despite stable texture identity', () => {
    // Arrange
    const fixture = sourceTarget(null, true), marker = atlasMarker();
    const before = fixture.frames[0].after;
    fixture.target.paint = () => [null, null, marker, null];
    // Act
    fixture.api.drawTransition(fixture.target, { before, after: before }, fixture.shapes, 0);
    const texture = fixture.target.glyphs.tex;
    fixture.target.glyphs.dirty = true;
    fixture.api.drawTransition(fixture.target, { before, after: before }, fixture.shapes, 0.5);
    // Assert
    assert.ok(texture);
    assert.equal(fixture.target.glyphs.tex, texture, 'The changed atlas retains its existing GPU texture.');
    assert.equal(fixture.clears.length, 2, 'Changed atlas content forces a fresh endpoint raster.');
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});

function endpointScenes(fixture) {
    const colors = [[20, 40, 60, 255], [60, 40, 20, 255], [40, 60, 20, 255]];
    const original = fixture.frames[0].before.features.find(feature => servedGeometry(feature.geometry, fixture.shapes).type === 'MultiPolygon');
    assert.ok(original, 'The original served scene provides polygon geometry.');
    const appearances = new Map();
    const scenes = colors.map(fill => {
        const properties = { ...original.properties };
        appearances.set(properties, [{ fill }, null, null, null]);
        return { ...fixture.collection, features: [{ ...original, properties, geometry: servedGeometry(original.geometry, fixture.shapes) }] };
    });
    fixture.target.paint = properties => appearances.get(properties);
    return scenes;
}

function servedGeometry(geometry, shapes) {
    return typeof geometry === 'string' ? shapes.find(shape => shape.shape === geometry).geometry : geometry;
}

test('native globe coordinates reach the GPU unchanged and open polygon rings close only when stroked', () => {
    // Arrange
    const fixture = sourceTarget(null, false, true);
    const points = [{ x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }, { x: 0, y: 0, z: 1 }];
    const geometry = { type: 'MultiPolygon', coordinates: [[points]] };
    const collection = { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry }] };
    fixture.target.paint = () => [{ fill: [1, 2, 3, 255] }, { color: [1, 2, 3, 255], width: 1, pattern: 'Solid' }, null, null];
    // Act
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, [], 0);
    const resource = fixture.target.cachedGeometry[0];
    const actual = { positions: fixture.uploads.find(upload => upload.buffer === resource.buf).values,
        boundaries: fixture.draws.filter(draw => draw.mode === 'LINE_LOOP'), geometry };
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual(actual, { positions: [1, 0, 0, 0, 1, 0, 0, 0, 1],
        boundaries: [{ mode: 'LINE_LOOP', first: 0, count: 3 }], geometry: { type: 'MultiPolygon', coordinates: [[points]] } });
    assert.equal(fixture.live.size, 0);
});

test('the empty outer ring fills the whole globe while a wholly empty shape draws nothing', () => {
    // Arrange
    const fixture = sourceTarget();
    const collection = { ...fixture.collection, features: [
        { ...fixture.collection.features[0], geometry: { type: 'MultiPolygon', coordinates: [[[]]] } },
        { ...fixture.collection.features[0], geometry: { type: 'MultiPolygon', coordinates: [] } },
    ] };
    fixture.target.paint = () => [{ fill: [1, 2, 3, 255] }, null, null, null];
    // Act
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, [], 0);
    const actual = fixture.fills.map(fill => fill.whole);
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual(actual, [true]);
    assert.equal(fixture.live.size, 0);
});


test('pixel picking returns only the accepted served identity and restores the display framebuffer', () => {
    // Arrange
    const fixture = sourceTarget();
    const points = [{ x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }, { x: 0, y: 0, z: 1 }];
    const feature = { ...fixture.collection.features[0], geometry: { type: 'MultiPolygon', coordinates: [[points]] } };
    const collection = { ...fixture.collection, features: [feature] };
    fixture.target.paint = () => [{ fill: [1, 2, 3, 255] }, null, null, null];
    fixture.gl.readPixels = (_x, _y, _width, _height, _format, _type, bytes) => bytes.set([1, 0, 0, 255]);
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, [], 0);
    // Act
    const selected = fixture.api.pickTransition(fixture.target, { x: 100, y: 100 }, new Set([feature.id]));
    const unrecorded = fixture.api.pickTransition(fixture.target, { x: 100, y: 100 }, new Set());
    const framebuffer = fixture.observed().drawFramebuffer;
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual({ selected, unrecorded, framebuffer, live: fixture.live.size }, { selected: feature.id, unrecorded: undefined, framebuffer: 0, live: 0 });
});


test('settled drawing reuses the transition geometry and survives a completed playback without a second scene model', () => {
    // Arrange
    const fixture = sourceTarget(null, false, true);
    const geometry = { type: 'MultiPolygon', coordinates: [[[{ x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }, { x: 0, y: 0, z: 1 }]]] };
    const collection = { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry }] };
    fixture.target.paint = () => [{ fill: [1, 2, 3, 255] }, null, null, null];
    // Act
    fixture.api.drawStatic(fixture.target, collection);
    const resource = fixture.target.cachedGeometry[0];
    fixture.api.drawStatic(fixture.target, collection);
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, [], 0.5);
    fixture.api.drawStatic(fixture.target, collection);
    const actual = { sameResource: fixture.target.cachedGeometry[0] === resource, resourceCount: fixture.target.cachedGeometry.length,
        original: fixture.target.staticCollection === collection, errors: fixture.uploads.some(upload => upload.values.some(Number.isNaN)) };
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual(actual, { sameResource: true, resourceCount: 1, original: true, errors: false });
    assert.equal(fixture.live.size, 0);
});


test('settled underlay stays below terrain while water covers the overlaid land fill', () => {
    // Arrange
    const fixture = sourceTarget();
    const geometry = { type: 'MultiPolygon', coordinates: [[[{ x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }, { x: 0, y: 0, z: 1 }]]] };
    const collection = { ...fixture.collection, features: [{ ...fixture.collection.features[0], geometry }], waters: [{ geometry }] };
    fixture.target.paint = () => [{ fill: [2, 2, 2, 255], underlay: [1, 1, 1, 255] }, null, null, null];
    fixture.target.water = () => [{ fill: [3, 3, 3, 255] }, null];
    // Act
    fixture.api.drawStatic(fixture.target, collection, 'underlay');
    const below = fixture.fills.splice(0).map(fill => Array.from(fill.color));
    fixture.api.drawStatic(fixture.target, collection, 'overlay');
    const above = fixture.fills.map(fill => Array.from(fill.color));
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual({ below, above }, { below: [[1, 1, 1, 255]], above: [[2, 2, 2, 255], [3, 3, 3, 255]] });
    assert.equal(fixture.live.size, 0);
});


test('translucent shoreline preserves one fill, one highlight and stroke-only shore passes', () => {
    // Arrange
    const fixture = sourceTarget();
    const geometry = { type: 'MultiPolygon', coordinates: [[[{ x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }, { x: 0, y: 0, z: 1 }]]] };
    const feature = { ...fixture.collection.features[0], geometry };
    const collection = { ...fixture.collection, features: [feature] };
    const stroke = { color: [50, 50, 50, 128], width: 2, pattern: 'Solid' };
    fixture.target.paint = () => [{ fill: [100, 100, 100, 128], underlay: [25, 25, 25, 128], shore: stroke }, stroke, null, null];
    fixture.gl.readPixels = (_x, _y, _width, _height, _format, _type, bytes) => bytes.set([1, 0, 0, 255]);
    const result = () => ({ fills: fixture.fills.splice(0).map(fill => Array.from(fill.color)),
        strokes: fixture.draws.splice(0).filter(draw => draw.mode === 'LINE_LOOP').length });
    // Act
    fixture.api.drawStatic(fixture.target, collection, 'underlay');
    const underlay = result();
    fixture.api.drawStatic(fixture.target, collection);
    const settled = result();
    fixture.api.drawStatic(fixture.target, collection, 'overlay', { ids: new Set([feature.id]), opacity: 30, stroke });
    const highlighted = result();
    const selected = fixture.api.pickTransition(fixture.target, { x: 100, y: 100 }, new Set([feature.id]));
    const picking = result();
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, [], 0.5);
    const endpoint = result();
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual({ underlay, settled, highlighted, picking, endpoint, selected, live: fixture.live.size }, {
        underlay: { fills: [[25, 25, 25, 128]], strokes: 0 },
        settled: { fills: [[100, 100, 100, 128]], strokes: 2 },
        highlighted: { fills: [[100, 100, 100, 128], [100, 100, 100, 158]], strokes: 3 },
        picking: { fills: [[1, 0, 0, 255]], strokes: 2 },
        endpoint: { fills: [[100, 100, 100, 128]], strokes: 2 },
        selected: feature.id, live: 0,
    });
});


test('course labels project visible native strokes without antimeridian jumps or polar connectors', () => {
    // Arrange
    const fixture = sourceTarget();
    const point = (longitude, latitude = 0) => ({ x: Math.cos(latitude * Math.PI / 180) * Math.cos(longitude * Math.PI / 180), y: Math.cos(latitude * Math.PI / 180) * Math.sin(longitude * Math.PI / 180), z: Math.sin(latitude * Math.PI / 180) });
    const seam = { type: 'MultiLineString', coordinates: [[point(170), point(-170)]] };
    const limb = { type: 'MultiLineString', coordinates: [[point(-120), point(0), point(120)]] };
    const pole = { type: 'MultiLineString', coordinates: [[point(0), { x: 0, y: 0, z: 1 }, point(90)]] };
    const original = JSON.stringify({ seam, limb, pole });
    const rounded = geometry => geometry.coordinates.map(line => line.map(point => point.map(value => Math.round(value))));
    // Act
    const actual = {
        seam: rounded(fixture.api.visibleLabelCourses(seam, [180, 0], true)),
        limb: rounded(fixture.api.visibleLabelCourses(limb, [0, 0], true)),
        pole: rounded(fixture.api.visibleLabelCourses(pole, [0, 0], false)),
        unchanged: JSON.stringify({ seam, limb, pole }) === original,
    };
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual(JSON.parse(JSON.stringify(actual)), { seam: [[[170, 0], [190, 0]]], limb: [[[-90, 0], [0, 0], [90, 0]]], pole: [], unchanged: true });
});

test('course labels reuse the spherical crossing helper in both directions and at horizon endpoints', () => {
    // Arrange
    const fixture = sourceTarget();
    const behind = { x: -0.6, y: 0.8, z: 0 }, front = { x: 0.6, y: 0.8, z: 0 }, horizon = { x: 0, y: 1, z: 0 };
    const nearBehind = { x: -1e-9, y: 0.6, z: 0.8 }, nearFront = { x: 1e-9, y: 0.6, z: 0.8 };
    const geometry = { type: 'MultiLineString', coordinates: [
        [behind, front], [front, behind], [nearBehind, nearFront], [nearFront, nearBehind], [behind, horizon], [horizon, behind],
    ] };
    const original = JSON.stringify(geometry), crossing = fixture.limb.frontCrossing;
    let crossings = 0;
    fixture.limb.frontCrossing = (...args) => { crossings++; return crossing(...args); };
    // Act
    const projected = fixture.api.visibleLabelCourses(geometry, [0, 0], true);
    const actual = { type: projected.type, coordinates: Array.from(projected.coordinates, line =>
        Array.from(line, point => Array.from(point, value => Number(value.toFixed(9))))), crossings, unchanged: JSON.stringify(geometry) === original };
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual(actual, { type: 'MultiLineString', coordinates: [
        [[90, 0], [53.130102354, 0]], [[53.130102354, 0], [90, 0]],
        [[90, 53.130102354], [89.999999905, 53.130102354]], [[89.999999905, 53.130102354], [90, 53.130102354]],
        [[90, 0], [90, 0]], [[90, 0], [90, 0]],
    ], crossings: 6, unchanged: true });
});

test('highlighting a served point draws its native location even when its ordinary marker belongs to the label layer', () => {
    // Arrange
    const fixture = sourceTarget(null, false, true);
    const feature = { ...fixture.collection.features[0], geometry: { type: 'Point', coordinates: { x: 1, y: 0, z: 0 } } };
    const collection = { ...fixture.collection, features: [feature] };
    const highlight = { ids: new Set([feature.id]), opacity: 89, stroke: { color: [1, 2, 3, 255], width: 4, pattern: 'Solid' }, marker: { color: [1, 2, 3, 255], size: 14 } };
    // Act
    fixture.api.drawStatic(fixture.target, collection, 'overlay', highlight);
    const actual = fixture.draws.filter(draw => draw.mode === 'POINTS');
    fixture.api.disposeTransition(fixture.target);
    // Assert
    assert.deepEqual(actual, [{ mode: 'POINTS', first: 0, count: 1 }]);
    assert.equal(fixture.live.size, 0);
});

test('static highlight picking and animation share clipping with offset-only reuse and lifetime invalidation', () => {
    const fixture = sourceTarget(null, false, true);
    const geometry = { type: 'MultiPolygon', coordinates: [[[
        { x: 0.6, y: -0.8, z: 0 }, { x: 0.6, y: 0, z: 0.8 },
        { x: -0.6, y: 0.8, z: 0 }, { x: -0.6, y: 0, z: -0.8 },
    ]]] };
    const feature = { ...fixture.collection.features[0], geometry };
    const collection = { ...fixture.collection, features: [feature] };
    const input = { shaderData: { variantName: 'test', vertexShaderPrelude: '', define: '' },
        defaultProjectionData: { clippingPlane: [0, 0, 1, 0], projectionTransition: 1 } };
    fixture.target.projection = () => input;
    fixture.target.paint = () => [{ fill: [1, 2, 3, 255] }, null, null, null];
    fixture.gl.readPixels = (_x, _y, _width, _height, _format, _type, bytes) => bytes.set([1, 0, 0, 255]);
    fixture.api.drawStatic(fixture.target, collection);
    const ring = [...fixture.target.clippedRings][0], pieces = ring.clipScratch;
    assert.ok(pieces.length > 0);
    fixture.api.drawStatic(fixture.target, collection, 'overlay', { ids: new Set([feature.id]), opacity: 30 });
    fixture.api.pickTransition(fixture.target, { x: 100, y: 100 }, new Set([feature.id]));
    assert.equal(ring.clipScratch, pieces, 'static highlights and ID picking borrow the same lobes');
    input.defaultProjectionData.clippingPlane[3] = 0.5;
    fixture.api.drawStatic(fixture.target, collection);
    assert.equal(ring.clipScratch, pieces, 'perspective horizon offset remains a shader input');
    input.defaultProjectionData.clippingPlane[0] = 1e-7;
    fixture.api.drawStatic(fixture.target, collection);
    assert.notEqual(ring.clipScratch, pieces, 'normal changes replace hemisphere clips');
    assert.ok(pieces.every(piece => piece.vao === null));
    const beforeAnimation = ring.clipScratch;
    fixture.api.drawTransition(fixture.target, { before: collection, after: collection }, [], 0.5);
    assert.ok(beforeAnimation.every(piece => piece.vao === null), 'frame invalidation releases old clipping');
    assert.ok(ring.clipScratch.length > 0, 'endpoint drawing uses the same retained ring cache');
    const beforeLayout = ring.clipScratch;
    input.shaderData = { ...input.shaderData, variantName: 'new-layout' };
    fixture.api.drawStatic(fixture.target, collection);
    const replacement = [...fixture.target.clippedRings].find(value => value !== ring);
    assert.ok(replacement?.clipScratch.length > 0, 'new attribute layouts get independent geometry and clipped buffers');
    assert.notEqual(replacement.clipScratch, beforeLayout);
    fixture.api.resetTransition(fixture.target);
    assert.equal(ring.clipScratch, null);
    assert.equal(replacement.clipScratch, null);
    assert.equal(fixture.target.clippedRings.size, 0);
    fixture.api.disposeTransition(fixture.target);
    assert.equal(fixture.live.size, 0);
});
