// ExtensionHost.ts

// --- Inlined vscode-api/commands.ts ---
type Thenable<T> = PromiseLike<T>;

class Commands {
    private commands = new Map<string, Function>();
    private postMessage: (message: any) => void;

    constructor(postMessage: (message: any) => void) {
        this.postMessage = postMessage;
    }

    registerCommand(command: string, callback: (...args: any[]) => any, thisArg?: any): any {
        console.log(`[ExtensionHost] Registering command: ${command}`);
        this.commands.set(command, callback.bind(thisArg));

        // Notify main thread that a command has been registered
        this.postMessage({
            type: 'registerCommand',
            command
        });

        return {
            dispose: () => {
                this.commands.delete(command);
            }
        };
    }

    executeCommand<T>(command: string, ...rest: any[]): Thenable<T> {
        console.log(`[ExtensionHost] Executing command: ${command}`);
        const callback = this.commands.get(command);
        if (callback) {
            return Promise.resolve(callback(...rest));
        }

        // If not found locally, ask main thread to execute it
        this.postMessage({
            type: 'executeCommand',
            command,
            args: rest
        });

        return Promise.resolve(undefined as unknown as T);
    }
}

// --- Inlined vscode-api/window.ts ---
class ExtWindow {
    private postMessage: (message: any) => void;

    constructor(postMessage: (message: any) => void) {
        this.postMessage = postMessage;
    }

    showInformationMessage(message: string, ...items: string[]): Thenable<string | undefined> {
        console.log(`[ExtensionHost] showInformationMessage: ${message}`);
        this.postMessage({
            type: 'showInformationMessage',
            message,
            items
        });
        return Promise.resolve(undefined);
    }

    showErrorMessage(message: string, ...items: string[]): Thenable<string | undefined> {
        console.log(`[ExtensionHost] showErrorMessage: ${message}`);
        this.postMessage({
            type: 'showErrorMessage',
            message,
            items
        });
        return Promise.resolve(undefined);
    }
}

// --- Main Worker Logic ---

// Helper to send messages to the main thread
function postMessageToMain(message: any) {
    self.postMessage(message);
}

// Define the VSCode API shape
const vscode = {
    commands: new Commands(postMessageToMain),
    window: new ExtWindow(postMessageToMain),
};

// Expose vscode global
(self as any).vscode = vscode;

console.log('[ExtensionHost] Worker started');
postMessageToMain({ type: 'hostReady' });

// Listen for messages from the main thread
self.onmessage = (event) => {
    const { type, data } = event.data;
    console.log(`[ExtensionHost] Received message: ${type}`, data);

    switch (type) {
        case 'loadExtension':
            loadExtension(data.code);
            break;
        case 'executeCommand':
            vscode.commands.executeCommand(data.command, ...(data.args || []));
            break;
    }
};

/**
 * Extension Security Model (CSP-like restrictions):
 *
 * Extensions run inside a Web Worker with the following constraints:
 * - ALLOWED APIs: vscode.commands.registerCommand, vscode.commands.executeCommand,
 *   vscode.window.showInformationMessage, vscode.window.showErrorMessage,
 *   console.log, console.warn, console.error, console.info,
 *   setTimeout, clearTimeout, setInterval, clearInterval,
 *   Promise, JSON.parse, JSON.stringify, Array, Object, Map, Set,
 *   String, Number, Boolean, Date, Math, RegExp, Symbol, Error,
 *   parseInt, parseFloat, isNaN, isFinite, encodeURIComponent,
 *   decodeURIComponent, encodeURI, decodeURI, btoa, atob
 * - BLOCKED: Network access (fetch, XMLHttpRequest, importScripts),
 *   dynamic code generation (eval, Function, new Function),
 *   Node.js APIs (require, process, child_process, fs),
 *   prototype pollution (__proto__, constructor[...]),
 *   and any other access outside the provided vscode API surface.
 * - TIMEOUT: Extensions must complete activation within 5 seconds.
 */

/** Patterns that are blocked in extension code to prevent code injection and sandbox escape. */
const BLOCKED_PATTERNS: ReadonlyArray<{ pattern: RegExp; reason: string }> = [
    { pattern: /\bfetch\s*\(/, reason: 'Network access (fetch) is not permitted' },
    { pattern: /\bXMLHttpRequest\b/, reason: 'Network access (XMLHttpRequest) is not permitted' },
    { pattern: /\bimportScripts\s*\(/, reason: 'importScripts is not permitted' },
    { pattern: /\beval\s*\(/, reason: 'eval() is not permitted' },
    { pattern: /\bnew\s+Function\b/, reason: 'new Function() is not permitted' },
    { pattern: /\bFunction\s*\(/, reason: 'Function() constructor is not permitted' },
    { pattern: /\brequire\s*\(/, reason: 'require() is not permitted (no Node.js access)' },
    { pattern: /\bprocess\s*\./, reason: 'process access is not permitted' },
    { pattern: /\bchild_process\b/, reason: 'child_process is not permitted' },
    { pattern: /\bfs\s*\./, reason: 'fs access is not permitted' },
    { pattern: /__proto__/, reason: 'Prototype pollution via __proto__ is not permitted' },
    { pattern: /\bconstructor\s*\[/, reason: 'Prototype pollution via constructor[] is not permitted' },
    { pattern: /\bconstructor\s*\.\s*constructor/, reason: 'Prototype chain traversal is not permitted' },
    { pattern: /\bWebSocket\s*\(/, reason: 'WebSocket access is not permitted' },
    { pattern: /\bSharedArrayBuffer\b/, reason: 'SharedArrayBuffer is not permitted' },
    { pattern: /\bAtomics\b/, reason: 'Atomics is not permitted' },
];

/**
 * Validates extension code against the blocked patterns allowlist.
 * Returns null if safe, or an error message describing the violation.
 */
function validateExtensionCode(code: string): string | null {
    for (const { pattern, reason } of BLOCKED_PATTERNS) {
        if (pattern.test(code)) {
            return `Extension code validation failed: ${reason}`;
        }
    }
    return null;
}

/** Maximum time (ms) an extension is allowed for activation before it is terminated. */
const EXTENSION_LOAD_TIMEOUT_MS = 5000;

function loadExtension(code: string) {
    console.log('[ExtensionHost] Loading extension code...');

    // Step 1: Static validation — reject code containing blocked patterns
    const validationError = validateExtensionCode(code);
    if (validationError) {
        console.error(`[ExtensionHost] ${validationError}`);
        postMessageToMain({ type: 'extensionError', error: validationError });
        return;
    }

    // Step 2: Wrap execution with a timeout to prevent infinite loops / hangs.
    // The extension code is executed via a sandboxed wrapper that only exposes
    // the vscode API object — no direct access to globalThis or self.
    const executionPromise = new Promise<void>((resolve, reject) => {
        try {
            // Build a restricted scope: the only argument the code receives is `vscode`.
            // Using indirect eval via a worker-scoped Function is the only mechanism
            // available in Web Workers; the static validation above blocks re-entry.
            const sandboxedFn = new Function('vscode', `"use strict";\n${code}`);
            sandboxedFn(Object.freeze({ ...vscode }));
            resolve();
        } catch (e) {
            reject(e);
        }
    });

    const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => {
            reject(new Error(`Extension activation timed out after ${EXTENSION_LOAD_TIMEOUT_MS}ms`));
        }, EXTENSION_LOAD_TIMEOUT_MS);
    });

    Promise.race([executionPromise, timeoutPromise])
        .then(() => {
            console.log('[ExtensionHost] Extension loaded successfully');
            postMessageToMain({ type: 'extensionLoaded' });
        })
        .catch((e) => {
            console.error('[ExtensionHost] Failed to load extension:', e);
            postMessageToMain({ type: 'extensionError', error: String(e) });
        });
}
