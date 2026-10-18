// Run after build.bat: node html/tests/bridge.cjs
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const publicDir = path.join(__dirname, '..', 'public');
let initializeAlpine;
let createTerminal;
global.document = { addEventListener(name, callback) { if (name === 'alpine:init') initializeAlpine = callback; } };
const viewportStyle = new Map();
global.document.documentElement = { style: { setProperty: (name, value) => viewportStyle.set(name, value) } };
const viewport = Object.assign(new EventTarget(), { height: 844, offsetTop: 0, scale: 1 });
const frames = new Map();
let nextFrame = 0;
global.window = Object.assign(new EventTarget(), {
    visualViewport: viewport, innerHeight: 844,
    requestAnimationFrame(callback) { frames.set(++nextFrame, callback); return nextFrame; },
    cancelAnimationFrame(id) { frames.delete(id); },
});
function paintViewport() {
    const callbacks = [...frames.values()];
    frames.clear();
    callbacks.forEach(callback => callback());
}
global.Alpine = { data(name, factory) { createTerminal = factory; } };
global.fetch = async () => ({
    ok: true,
    arrayBuffer: async () => fs.readFileSync(process.env.CVAR_WASM || path.join(publicDir, 'cvar.wasm')),
});
vm.runInThisContext(fs.readFileSync(path.join(publicDir, 'index.js'), 'utf8') + '\nglobalThis.CvarWasm = CvarWasm;');

(async () => {
    const stopViewport = fitConsoleViewport();
    assert.equal(viewportStyle.get('--viewport-height'), '844px');
    viewport.height = 360;
    viewport.offsetTop = 120;
    viewport.dispatchEvent(new Event('resize'));
    viewport.dispatchEvent(new Event('scroll'));
    assert.equal(frames.size, 1);
    paintViewport();
    assert.equal(viewportStyle.get('--viewport-height'), '360px');
    assert.equal(viewportStyle.get('--viewport-top'), '120px');
    viewport.height = 844;
    viewport.offsetTop = 0;
    viewport.dispatchEvent(new Event('resize'));
    paintViewport();
    assert.equal(viewportStyle.get('--viewport-height'), '844px');
    assert.equal(viewportStyle.get('--viewport-top'), '0px');
    stopViewport();
    viewport.dispatchEvent(new Event('resize'));
    assert.equal(frames.size, 0);
    const stopFallback = fitConsoleViewport({ ...window, visualViewport: undefined,
        addEventListener() {}, removeEventListener() {}, innerHeight: 500 });
    assert.equal(viewportStyle.get('--viewport-height'), '500px');
    stopFallback();
    console.log('Keyboard viewport resize, pan, dismissal, fallback, and cleanup passed.');
    const bridge = new CvarWasm();
    await bridge.load('cvar.wasm');
    bridge.call('select_example', 0);
    for (let length = 0; length <= 128; length++) {
        const value = 'A'.repeat(length);
        try {
            for (const command of [`text "${value}"`, ...(length ? [`text ${value}`] : [])]) {
                assert.deepEqual(bridge.call('execute', command), [`text "${value}"`]);
            }
            assert.deepEqual(bridge.call('execute', 'text'), [`text "${value}"`]);
        } catch (error) {
            throw new Error(`String assignment/readback failed at length ${length}`, { cause: error });
        }
    }
    for (const value of ['é'.repeat(32), '😀'.repeat(32), 'A'.repeat(65536)]) {
        assert.deepEqual(bridge.call('execute', `text ${value}`), [`text "${value}"`]);
        assert.deepEqual(bridge.call('execute', 'text'), [`text "${value}"`]);
    }
    assert.deepEqual(bridge.call('complete', 'te'), ['text']);
    assert.deepEqual(bridge.call('complete', 'missing'), []);
    assert.deepEqual(bridge.call('execute', 'poke! hello'), []);
    assert.deepEqual(bridge.call('select_example', 0), ['number "42"', 'text ""', 'poke!']);
    assert.throws(() => bridge.call('execute', 'missing'), /does not exist/);
    assert.deepEqual(bridge.call('complete', ''), ['number', 'text', 'poke!']);
    console.log('Wasm string assignment/readback passed (0–128 bytes).');
    console.log('Unicode, large input, completion, and errors passed.');

    initializeAlpine();
    const terminal = createTerminal();
    const input = {
        selectionStart: 0, selectionEnd: 0, focus(options) { assert.equal(options.preventScroll, true); },
        setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; },
    };
    terminal.$refs = {
        input, scroll: { scrollTop: 0, scrollHeight: 100 },
        suggestions: { scrollTop: 0, clientHeight: 100,
            querySelector(selector) {
                const index = Number(selector.match(/\d+/)[0]);
                return { offsetTop: index * 30, offsetHeight: 30 };
            },
        },
    };
    terminal.$nextTick = callback => callback();
    await terminal.init();
    const type = (line, cursor = line.length) => {
        terminal.line = line;
        input.setSelectionRange(cursor, cursor);
        terminal.suggest();
    };
    const key = key => terminal.keydown({ key, preventDefault() {} });

    type('te');
    assert.deepEqual(terminal.suggestions, ['text']);
    assert.equal(terminal.suggestionsOpen, true);
    key('Tab');
    assert.equal(terminal.line, 'text');
    assert.equal(terminal.suggestionsOpen, false);
    type('text AAA');
    assert.equal(terminal.suggestionsOpen, false);
    terminal.submit();
    assert.equal(terminal.entries.at(-1).text, 'text "AAA"');
    assert.equal(terminal.entries.at(-2).kind, 'command');
    assert.equal(terminal.entries.at(-2).text, '>>> text AAA');
    type('draft');
    key('ArrowUp');
    assert.equal(terminal.line, 'text AAA');
    key('ArrowDown');
    assert.equal(terminal.line, 'draft');

    terminal.choose(1);
    type('fo');
    assert.deepEqual(terminal.suggestions, ['foo.bool', 'foo…']);
    key('ArrowDown');
    assert.equal(terminal.suggestionIndex, 1);
    key('Tab');
    assert.equal(terminal.line, 'foo.');
    assert.equal(input.selectionStart, 4);
    type('foo.');
    assert.deepEqual(terminal.suggestions, ['foo.bool', 'foo.float', 'foo.int', 'foo.mode', 'foo.string']);
    key('ArrowUp');
    assert.equal(terminal.suggestionIndex, terminal.suggestions.length - 1);
    assert.equal(terminal.$refs.suggestions.scrollTop, 50);
    key('ArrowDown');
    assert.equal(terminal.$refs.suggestions.scrollTop, 0);
    key('Escape');
    assert.equal(terminal.line, 'foo.');
    assert.equal(terminal.suggestionsOpen, false);
    type('  foo.i 42', 7);
    terminal.acceptSuggestion(0);
    assert.equal(terminal.line, '  foo.int 42');
    assert.equal(input.selectionStart, 9);
    terminal.line = '';
    input.setSelectionRange(0, 0);
    key('Tab');
    assert.equal(terminal.line, 'foo.bool');
    type('foo.bool ');
    assert.deepEqual(terminal.suggestions, ['false', 'true']);
    key('ArrowDown');
    key('Tab');
    assert.equal(terminal.line, 'foo.bool true');
    terminal.submit();
    assert.equal(terminal.entries.at(-1).text, 'foo.bool "true"');
    type('foo.mode ');
    assert.deepEqual(terminal.suggestions, ['Fast', 'Normal', 'Slow']);
    key('Tab');
    assert.equal(terminal.line, 'foo.mode Fast');
    terminal.submit();
    assert.equal(terminal.entries.at(-1).text, 'foo.mode "Fast"');
    type('  foo.mode   "Sl');
    assert.deepEqual(terminal.suggestions, ['"Slow"']);
    key('Tab');
    assert.equal(terminal.line, '  foo.mode   "Slow"');
    terminal.submit();
    assert.equal(terminal.entries.at(-1).text, 'foo.mode "Slow"');
    type('foo.mode Fxxx', 10);
    key('Tab');
    assert.equal(terminal.line, 'foo.mode Fast');
    type('foo.mode Missing');
    assert.equal(terminal.suggestionsOpen, false);
    assert.deepEqual(bridge.call('complete', 'poke! '), []);
    type('fo');
    terminal.choose(2);
    assert.equal(terminal.suggestionsOpen, false);
    terminal.submit('create! int score 100');
    type('sc');
    assert.deepEqual(terminal.suggestions, ['score']);
    terminal.acceptSuggestion(0);
    assert.equal(terminal.line, 'score');
    terminal.destroy();
    console.log('Live suggestions, selection, Tab, Escape, history, and command transcript passed.');
})().catch(error => { console.error(error); process.exitCode = 1; });
