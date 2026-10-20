import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

function sourceTarget(label = null, realAtlas = false, realGpu = false) {
    const served = JSON.parse(fs.readFileSync(process.env.MG_TRANSITION_PAGE_FIXTURE, 'utf8')).body.transition.transition;
    const original = served.origin.features[0];
    const geometry = typeof original.geometry === 'string' ? served.shapes.find(shape => shape.shape === original.geometry).geometry : original.geometry;
    const collection = { ...served.origin, features: [{ ...original, geometry }], waters: [] };
    const live = new Set();
    const disposed = new Set();
    const counts = { geometry: 0, framebuffer: 0, texture: 0, stencil: 0 };
    const draws = [], images = [], painted = [], uploads = [];
    let boundBuffer;
    const clears = [], queries = [], uniforms = [], fills = [];
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
    const gpuContext = { module: { exports: {} } };
    if (realGpu) vm.runInNewContext(fs.readFileSync(new URL('../src/retained-gpu.js', import.meta.url), 'utf8'), gpuContext);
    const retained = realGpu ? gpuContext.module.exports : {
        uploadGeometry(_gl, _loc, verts) { counts.geometry++; return { vao: resource('geometry'), count: verts.length / 3 }; },
        uploadLobe(_gl, _loc, verts) { counts.geometry++; return { vao: resource('screen'), count: verts.length / 3 }; },
        uploadPrevious() { return gl.createBuffer(); },
        drawStencilFill() { fills.push({ framebuffer: drawFramebuffer, fraction: morphFraction }); },
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
    const context = { module: { exports: {} }, require: name => name === './retained-gpu.js' ? retained : labels };
    vm.runInNewContext(fs.readFileSync(new URL('../src/transition-target.js', import.meta.url), 'utf8'), context);
    const api = context.module.exports;
    const target = api.transitionTarget(gl, () => ({ shaderData: { variantName: 'test', vertexShaderPrelude: '', define: '' }, defaultProjectionData: {} }), () => [null, null, null, label], () => [], () => {}, reference => reference.id);
    return { api, target, gl, atlas, counts, draws, painted, images, uploads, clears, queries, uniforms, fills, live, collection, waters: served.origin.waters, frames: served.keyframes, shapes: served.shapes, frame: { before: collection, after: collection },
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
    assert.equal(fixture.clears.length, 2, 'New projection arrays with the same values reuse the original endpoint textures.');
    assert.equal(fixture.queries.length, reads * 2, 'Every fraction still saves the complete incoming GL state.');
    let expected = 2;
    for (const [index, field] of ['mainMatrix', 'tileMercatorCoords', 'clippingPlane', 'projectionTransition', 'fallbackMatrix'].entries()) {
        if (field === 'projectionTransition') data[field]++;
        else data[field][0]++;
        fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.2 + index * 0.1);
        assert.equal(fixture.clears.length, expected += 2, 'Every consumed projection uniform value invalidates the textures.');
    }
    fixture.api.resetTransition(fixture.target);
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0);
    assert.equal(fixture.clears.length, expected + 2, 'Reset never reuses textures from an earlier transition.');
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
    assert.equal(fixture.clears.length, 2, 'Stable prepared water presentation reuses the textures.');
    let expected = 2;
    for (const change of [() => paint.fill[0]++, () => stroke.color[1]++, () => stroke.width++, () => { fixture.target.water = () => [null, stroke]; }]) {
        change();
        fixture.api.drawTransition(fixture.target, frame, fixture.shapes, expected / 20);
        assert.equal(fixture.clears.length, expected += 2, 'Mutating an existing fill/stroke value changes the actual endpoint pixels.');
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
    assert.equal(fixture.clears.length, 2, 'Stable marker presentation reuses endpoint textures.');
    let expected = 2;
    for (const change of [() => marker.size++, () => marker.color[2]++, () => { fixture.gl.canvas.clientWidth /= 2; }]) {
        change();
        fixture.api.drawTransition(fixture.target, fixture.frame, [], expected / 20);
        assert.equal(fixture.clears.length, expected += 2, 'Each effective marker pixel input invalidates the textures.');
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
    assert.equal(fixture.clears.length, 2, 'Identical effective glyph content reuses the two original textures.');
    style.color[0]++;
    fixture.api.drawTransition(fixture.target, fixture.frame, [], 0.2);
    assert.equal(fixture.clears.length, 4, 'A glyph color mutation rerenders endpoint textures despite unchanged UV bounds.');
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
    assert.deepEqual(fixture.draws.filter(draw => draw.mode === 'LINE_STRIP'), [...expected, ...expected], 'Both scenes draw every original course in order without connecting adjacent courses.');
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
