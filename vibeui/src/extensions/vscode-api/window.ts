// vscode-api/window.ts

type Thenable<T> = PromiseLike<T>;

export class Window {
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
        // For MVP, we don't wait for user selection, just resolve immediately
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
