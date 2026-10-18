"use strict";

// A tiny, synchronous ABI: each exported operation calls exactly one result callback.
class CvarWasm {
    constructor() {
        this.wasm = null;
        this.result = undefined;
        this.hasResult = false;
        this.encoder = new TextEncoder();
        this.decoder = new TextDecoder("utf-8", { fatal: true });
    }

    async load(url) {
        const response = await fetch(url);
        if (!response.ok) throw new Error(`Could not load ${url} (${response.status}). Run html/build.bat first.`);
        const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), {
            env: {
                resultTextLines: (ptr, len) => {
                    const text = this.read(ptr, len);
                    // Ignore the final line terminator, preserving blank lines within output.
                    this.result = text ? text.replace(/\n$/, "").split("\n") : [];
                    this.hasResult = true;
                },
                resultError: (ptr, len) => {
                    this.result = new Error(this.read(ptr, len));
                    this.hasResult = true;
                },
            },
        });
        this.wasm = instance.exports;
    }

    read(ptr, len) {
        // Reacquire the buffer each time: Wasm allocations can grow memory.
        return this.decoder.decode(new Uint8Array(this.wasm.memory.buffer, ptr, len));
    }

    takeResult() {
        if (!this.hasResult) throw new Error("Wasm did not return a result.");
        const result = this.result;
        this.result = undefined;
        this.hasResult = false;
        if (result instanceof Error) throw result;
        return result;
    }

    call(name, value) {
        this.result = undefined;
        this.hasResult = false;
        if (typeof value === "string") {
            const bytes = this.encoder.encode(value);
            const ptr = this.wasm.alloc(bytes.length);
            try {
                new Uint8Array(this.wasm.memory.buffer, ptr, bytes.length).set(bytes);
                this.wasm[name](ptr, bytes.length);
            } finally {
                this.wasm.dealloc(ptr, bytes.length);
            }
        } else {
            this.wasm[name](value);
        }
        return this.takeResult();
    }
}

const EXAMPLES = [
    {
        file: "program-state.rs",
        intro: 'Hit Enter to list the properties and actions.\n'
            + 'Read with <name>, assign with <name> <value>, invoke with poke! <text>.\n'
            + 'Quote values to preserve spaces or escapes; text "" clears the text.\n',
    },
    {
        file: "nesting-techniques.rs",
        intro: 'Hit Enter to list all the cvars and their values.\n'
            + 'Assign with <name> <value>, e.g. foo.int 42.\n'
            + 'Try foo.bool or foo.mode followed by a space for value suggestions.\n',
    },
    {
        file: "runtime-properties.rs",
        intro: 'Hit Enter to list all the cvars and their values.\n'
            + 'Assign with <name> <value>.\n'
            + 'Create with create! <type> <name> <value> (string, int, float).\n'
            + 'Remove with destroy! <name>.\n',
    },
];

// Mobile keyboards can resize/pan the visual viewport without resizing the page.
function fitConsoleViewport(view = window, root = document.documentElement) {
    const viewport = view.visualViewport;
    let frame;
    const update = () => {
        frame = undefined;
        // Leave pinch zoom to the browser rather than reflowing under the gesture.
        if (viewport && viewport.scale !== 1) return;
        root.style.setProperty("--viewport-height", `${viewport?.height ?? view.innerHeight}px`);
        root.style.setProperty("--viewport-top", `${viewport?.offsetTop ?? 0}px`);
    };
    const schedule = () => {
        if (frame === undefined) frame = view.requestAnimationFrame(update);
    };
    update();
    view.addEventListener("resize", schedule);
    viewport?.addEventListener("resize", schedule);
    viewport?.addEventListener("scroll", schedule);
    return () => {
        view.removeEventListener("resize", schedule);
        viewport?.removeEventListener("resize", schedule);
        viewport?.removeEventListener("scroll", schedule);
        if (frame !== undefined) view.cancelAnimationFrame(frame);
    };
}

document.addEventListener("alpine:init", () => {
    // Keep the Wasm instance outside Alpine's reactive proxies.
    const bridge = new CvarWasm();
    let stopViewport;
    Alpine.data("terminalDemo", () => ({
        examples: EXAMPLES, selected: 0, ready: false, loading: true,
        fatal: "", line: "", entries: [], history: [], historyIndex: 0, draft: "", nextId: 0,
        suggestions: [], suggestionsOpen: false, suggestionIndex: 0, suggestionStart: 0, suggestionEnd: 0,
        completingValue: false,

        async init() {
            stopViewport = fitConsoleViewport();
            try {
                await bridge.load("cvar.wasm");
                this.ready = true;
                this.choose(0);
            } catch (error) {
                this.fatal = `${error.message} Serve the public folder over HTTP; file:// cannot load Wasm.`;
            } finally { this.loading = false; }
        },

        destroy() { stopViewport?.(); },

        append(kind, text) {
            if (Array.isArray(text)) text = text.join("\n");
            if (text) this.entries.push({ id: this.nextId++, kind, text: text.replace(/\n$/, "") });
            if (this.entries.length > 400) this.entries.splice(0, this.entries.length - 400);
            this.$nextTick(() => { this.$refs.scroll.scrollTop = this.$refs.scroll.scrollHeight; });
        },

        focus() { this.$nextTick(() => this.$refs.input.focus({ preventScroll: true })); },

        choose(index) {
            if (!this.ready) return;
            this.selected = index;
            this.entries = [];
            this.line = "";
            this.history = [];
            this.historyIndex = 0;
            this.draft = "";
            this.hideSuggestions();
            this.append("system", this.examples[index].intro + '\n');
            try { this.append("output", bridge.call("select_example", index)); }
            catch (error) { this.append("error", error.message); }
            this.focus();
        },

        submit(command = this.line) {
            if (!this.ready) return;
            this.hideSuggestions();
            this.append("command", `>>> ${command}`);
            if (command && this.history.at(-1) !== command) this.history.push(command);
            this.historyIndex = this.history.length;
            this.draft = "";
            this.line = "";
            try { this.append("output", bridge.call("execute", command)); }
            catch (error) { this.append("error", error.message); }
            this.focus();
        },

        recall(direction) {
            this.hideSuggestions();
            if (this.historyIndex === this.history.length) this.draft = this.line;
            this.historyIndex = Math.max(0, Math.min(this.history.length, this.historyIndex + direction));
            this.line = this.history[this.historyIndex] ?? this.draft;
            this.$nextTick(() => this.$refs.input.setSelectionRange(this.line.length, this.line.length));
        },

        hideSuggestions() {
            this.suggestionsOpen = false;
            this.suggestions = [];
            this.suggestionIndex = 0;
        },

        suggest(includeEmpty = false) {
            this.hideSuggestions();
            if (!this.ready) return;
            const input = this.$refs.input;
            const cursor = input.selectionStart;
            if (cursor !== input.selectionEnd) return;
            const before = this.line.slice(0, cursor);
            const partial = before.replace(/^[\t\n\v\f\r ]+/, "");
            if (!partial && !includeEmpty) return;
            try {
                this.suggestions = bridge.call("complete", partial).sort();
                this.suggestionStart = before.length - partial.length;
                const separator = partial.search(/[\t\n\v\f\r ]/);
                this.completingValue = separator >= 0;
                if (this.completingValue) {
                    const args = partial.slice(separator);
                    this.suggestionStart += separator + args.length - args.replace(/^[\t\n\v\f\r ]+/, "").length;
                    // Value candidates replace the whole argument, including quoted spaces.
                    this.suggestionEnd = this.line.length;
                } else {
                    const tail = this.line.slice(cursor);
                    const tokenEnd = tail.search(/[\t\n\v\f\r ]/);
                    this.suggestionEnd = tokenEnd < 0 ? this.line.length : cursor + tokenEnd;
                }
                this.suggestionsOpen = this.suggestions.length > 0;
                this.scrollSuggestion();
            } catch (error) { this.append("error", error.message); }
        },

        scrollSuggestion() {
            this.$nextTick(() => {
                const list = this.$refs.suggestions;
                const item = list.querySelector(`[data-index="${this.suggestionIndex}"]`);
                if (!item) return;
                // Scroll only the overlay; scrollIntoView can also pan the mobile viewport.
                if (item.offsetTop < list.scrollTop) list.scrollTop = item.offsetTop;
                else if (item.offsetTop + item.offsetHeight > list.scrollTop + list.clientHeight) {
                    list.scrollTop = item.offsetTop + item.offsetHeight - list.clientHeight;
                }
            });
        },

        moveSuggestion(direction) {
            this.suggestionIndex = (this.suggestionIndex + direction + this.suggestions.length) % this.suggestions.length;
            this.scrollSuggestion();
        },

        acceptSuggestion(index = this.suggestionIndex) {
            const path = this.suggestions[index];
            if (path === undefined) return;
            const replacement = !this.completingValue && path.endsWith("…") ? path.slice(0, -1) + "." : path;
            this.line = this.line.slice(0, this.suggestionStart) + replacement + this.line.slice(this.suggestionEnd);
            const position = this.suggestionStart + replacement.length;
            this.hideSuggestions();
            this.$nextTick(() => {
                this.$refs.input.focus({ preventScroll: true });
                this.$refs.input.setSelectionRange(position, position);
            });
        },

        cursorMoved(event) {
            if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) this.suggest();
        },

        keydown(event) {
            if (event.isComposing) return;
            if (event.key === "Tab" && !event.shiftKey) {
                event.preventDefault();
                if (!this.suggestionsOpen) this.suggest(true);
                this.acceptSuggestion();
            }
            else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
                event.preventDefault();
                const direction = event.key === "ArrowUp" ? -1 : 1;
                if (this.suggestionsOpen) this.moveSuggestion(direction);
                else this.recall(direction);
            }
            else if (event.key === "Escape") { event.preventDefault(); this.hideSuggestions(); }
            else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "l") {
                event.preventDefault(); this.entries = [];
            }
        },
    }));
});
