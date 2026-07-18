// vscode-api/commands.ts
/* eslint-disable no-console, @typescript-eslint/no-explicit-any */

type Thenable<T> = PromiseLike<T>;

export class Commands {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-function-type
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
        // This requires a request/response mechanism which we'll implement simply for now
        this.postMessage({
            type: 'executeCommand',
            command,
            args: rest
        });

        return Promise.resolve(undefined as unknown as T);
    }
}
