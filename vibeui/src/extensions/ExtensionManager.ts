// ExtensionManager.ts

export class ExtensionManager {
    private worker: Worker | null = null;
    private registeredCommands = new Set<string>();
    private uiHandlers: {
        showInformationMessage: (message: string, items: string[]) => void;
        showErrorMessage: (message: string, items: string[]) => void;
    };

    constructor(uiHandlers: {
        showInformationMessage: (message: string, items: string[]) => void;
        showErrorMessage: (message: string, items: string[]) => void;
    }) {
        this.uiHandlers = uiHandlers;
    }

    public initialize() {
        if (this.worker) return;

        console.log('[ExtensionManager] Initializing extension host worker...');
        // Create worker from the ExtensionHost file
        // Note: In Vite, we can import workers with ?worker suffix
        // But for now, we might need a different approach if that's not configured.
        // Let's try to assume standard Vite worker import works.
        // If not, we might need to use a Blob or separate entry point.

        // For this MVP, we'll try to use the standard Worker constructor with a relative path
        // This might require the file to be built/served correctly.
        // Alternatively, we can inline the worker code for simplicity in this MVP.

        // Let's try importing it as a module first (Vite specific)
        // import ExtensionHostWorker from './ExtensionHost?worker';
        // this.worker = new ExtensionHostWorker();

        // Since we are writing this file as plain TS, we can't use the import syntax easily here without changing file extension or config.
        // So we will use a dynamic import or just assume the file is available.

        // FALLBACK: Create a Blob worker for the MVP to ensure it works without complex build config changes
        // We will read the ExtensionHost code (and dependencies) and create a blob.
        // But dependencies (vscode-api) make this hard.

        // BETTER APPROACH: Use Vite's worker import.
        // We will assume the user can import this class in App.tsx and pass the worker constructor or factory.
    }

    public isWorkerReady = false;

    public setWorker(worker: Worker) {
        this.worker = worker;
        this.worker.onmessage = this.handleMessage.bind(this);
        console.log('[ExtensionManager] Worker set');
    }

    private handleMessage(event: MessageEvent) {
        const { type, ...data } = event.data;
        console.log(`[ExtensionManager] Received message: ${type}`, data);

        switch (type) {
            case 'hostReady':
                this.isWorkerReady = true;
                console.log('[ExtensionManager] Extension host is ready');
                break;
            case 'extensionLoaded':
                console.log('[ExtensionManager] Extension loaded successfully');
                break;
            case 'extensionError':
                console.error(`[ExtensionManager] Extension error: ${data.error}`);
                break;
            case 'registerCommand':
                this.registeredCommands.add(data.command);
                console.log(`[ExtensionManager] Command registered: ${data.command}`);
                break;
            case 'showInformationMessage':
                this.uiHandlers.showInformationMessage(data.message, data.items);
                break;
            case 'showErrorMessage':
                this.uiHandlers.showErrorMessage(data.message, data.items);
                break;
            case 'executeCommand':
                // Handle command execution requests from extension (e.g. executing a built-in command)
                // For now, we just log it.
                console.log(`[ExtensionManager] Extension requested command execution: ${data.command}`);
                break;
        }
    }

    public loadExtension(code: string) {
        if (!this.worker) {
            console.error('[ExtensionManager] Worker not initialized');
            return;
        }
        this.worker.postMessage({
            type: 'loadExtension',
            data: { code }
        });
    }

    public executeCommand(command: string, ...args: any[]) {
        if (this.registeredCommands.has(command)) {
            this.worker?.postMessage({
                type: 'executeCommand',
                data: { command, args }
            });
        } else {
            console.warn(`[ExtensionManager] Command not found: ${command}`);
        }
    }

    public getRegisteredCommands() {
        return Array.from(this.registeredCommands);
    }
}
