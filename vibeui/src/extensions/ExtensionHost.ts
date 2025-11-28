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

function loadExtension(code: string) {
    try {
        console.log('[ExtensionHost] Loading extension code...');
        const func = new Function('vscode', code);
        func(vscode);
        console.log('[ExtensionHost] Extension loaded successfully');
        postMessageToMain({ type: 'extensionLoaded' });
    } catch (e) {
        console.error('[ExtensionHost] Failed to load extension:', e);
        postMessageToMain({ type: 'extensionError', error: String(e) });
    }
}
