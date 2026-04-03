import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs';
import {
    workspace,
    ExtensionContext,
    window,
    commands,
    OutputChannel,
    Uri,
} from 'vscode';
import {
    Executable,
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let outputChannel: OutputChannel;

export async function activate(context: ExtensionContext): Promise<void> {
    outputChannel = window.createOutputChannel('Flint');
    outputChannel.appendLine('Flint extension activating...');

    // Check if extension is enabled
    const config = workspace.getConfiguration('flint');
    if (!config.get<boolean>('enable', true)) {
        outputChannel.appendLine('Flint is disabled in settings');
        return;
    }

    // Get server path
    const serverPath = getServerPath(context, config);
    if (!serverPath) {
        window.showErrorMessage(
            'Flint: Could not find flint binary. Please set flint.serverPath in settings or install flint.'
        );
        return;
    }

    outputChannel.appendLine(`Using server binary: ${serverPath}`);

    // Verify binary exists
    if (!fs.existsSync(serverPath)) {
        window.showErrorMessage(
            `Flint: Server binary not found at ${serverPath}`
        );
        return;
    }

    // Create server options (using Executable type like typos-lsp)
    const run: Executable = {
        command: serverPath,
        args: ['lsp'],
    };

    const debug: Executable = {
        command: serverPath,
        args: ['lsp', '--debug'],
    };

    const serverOptions: ServerOptions = {
        run,
        debug,
    };

    // Create client options
    const clientOptions: LanguageClientOptions = {
        // Register for YAML files matching Flint patterns
        documentSelector: [
            { scheme: 'file', language: 'yaml', pattern: '**/default.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/default.yaml' },
            { scheme: 'file', language: 'yaml', pattern: '**/fleets/*.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/fleets/*.yaml' },
            { scheme: 'file', language: 'yaml', pattern: '**/fleets/**/*.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/fleets/**/*.yaml' },
            { scheme: 'file', language: 'yaml', pattern: '**/teams/*.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/teams/*.yaml' },
            { scheme: 'file', language: 'yaml', pattern: '**/teams/**/*.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/teams/**/*.yaml' },
            { scheme: 'file', language: 'yaml', pattern: '**/lib/**/*.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/lib/**/*.yaml' },
            { scheme: 'file', language: 'yaml', pattern: '**/platforms/**/*.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/platforms/**/*.yaml' },
            { scheme: 'file', language: 'yaml', pattern: '**/labels/**/*.yml' },
            { scheme: 'file', language: 'yaml', pattern: '**/labels/**/*.yaml' },
        ],
        initializationOptions: {
            fleetVersion: workspace.getConfiguration('flint').get<string>('fleetVersion', 'latest'),
        },
        synchronize: {
            // Watch for changes to Fleet config files
            fileEvents: workspace.createFileSystemWatcher('**/*.{yml,yaml}'),
        },
        outputChannel,
        traceOutputChannel: outputChannel,
    };

    // Create the language client
    client = new LanguageClient(
        'flint',
        'Flint',
        serverOptions,
        clientOptions
    );

    // Register commands
    context.subscriptions.push(
        commands.registerCommand('flint.restartServer', async () => {
            outputChannel.appendLine('Restarting Fleet LSP server...');
            if (client) {
                await client.restart();
                outputChannel.appendLine('Fleet LSP server restarted');
            }
        })
    );

    context.subscriptions.push(
        commands.registerCommand('flint.showOutput', () => {
            outputChannel.show();
        })
    );

    // Scaffold if needed, then open default.yml — shared by both commands
    async function ensureScaffoldAndOpen(): Promise<void> {
        const workspaceFolder = workspace.workspaceFolders?.[0];
        if (!workspaceFolder) {
            window.showErrorMessage('Flint: Open a folder first.');
            return;
        }

        const defaultYml = Uri.joinPath(workspaceFolder.uri, 'default.yml');

        // Scaffold if default.yml doesn't exist yet
        if (!fs.existsSync(defaultYml.fsPath)) {
            if (client) {
                try {
                    await client.sendRequest('workspace/executeCommand', {
                        command: 'fleet.scaffold',
                        arguments: [],
                    });
                } catch (err) {
                    window.showErrorMessage(`Flint: Failed to scaffold — ${err}`);
                    return;
                }
            } else {
                window.showErrorMessage('Flint: Language server not running.');
                return;
            }
        }

        // Open default.yml
        if (fs.existsSync(defaultYml.fsPath)) {
            const doc = await workspace.openTextDocument(defaultYml);
            await window.showTextDocument(doc);
        }
    }

    context.subscriptions.push(
        commands.registerCommand('flint.getStarted', ensureScaffoldAndOpen)
    );

    context.subscriptions.push(
        commands.registerCommand('flint.openDefaultYml', ensureScaffoldAndOpen)
    );

    // Start the client
    try {
        await client.start();
        outputChannel.appendLine('Flint LSP server started successfully');
    } catch (error) {
        outputChannel.appendLine(`Failed to start LSP server: ${error}`);
        window.showErrorMessage(
            `Flint: Failed to start language server. Check the output channel for details.`
        );
    }

    context.subscriptions.push(client);
}

export async function deactivate(): Promise<void> {
    if (client) {
        await client.stop();
    }
}

/**
 * Get the path to the flint binary.
 * Priority:
 * 1. User-configured path (flint.serverPath)
 * 2. Bundled binary in extension's bin/ directory
 */
function getServerPath(
    context: ExtensionContext,
    config: ReturnType<typeof workspace.getConfiguration>
): string | undefined {
    // Check user-configured path first
    const configuredPath = config.get<string>('serverPath');
    if (configuredPath && configuredPath.trim() !== '') {
        // Expand ~ to home directory
        const expandedPath = configuredPath.replace(/^~/, os.homedir());
        if (fs.existsSync(expandedPath)) {
            return expandedPath;
        }
        outputChannel.appendLine(
            `Configured server path not found: ${expandedPath}`
        );
    }

    // Try bundled binary
    const bundledPath = getBundledBinaryPath(context);
    if (bundledPath && fs.existsSync(bundledPath)) {
        return bundledPath;
    }

    // Try to find in PATH (for development)
    const pathBinary = findInPath('flint');
    if (pathBinary) {
        return pathBinary;
    }

    return undefined;
}

/**
 * Get the path to the bundled binary for the current platform.
 */
function getBundledBinaryPath(context: ExtensionContext): string | undefined {
    const platform = os.platform();
    const arch = os.arch();

    let binaryName: string;
    switch (platform) {
        case 'darwin':
            if (arch !== 'arm64') {
                outputChannel.appendLine('macOS Intel (x86_64) is not supported. Use Apple Silicon.');
                return undefined;
            }
            binaryName = 'flint-darwin-arm64';
            break;
        case 'linux':
            binaryName = arch === 'arm64'
                ? 'flint-linux-arm64'
                : 'flint-linux-x64';
            break;
        case 'win32':
            binaryName = 'flint-win32-x64.exe';
            break;
        default:
            outputChannel.appendLine(`Unsupported platform: ${platform}`);
            return undefined;
    }

    return path.join(context.extensionPath, 'bin', binaryName);
}

/**
 * Try to find an executable in the system PATH.
 */
function findInPath(name: string): string | undefined {
    const pathEnv = process.env.PATH || '';
    const pathSeparator = os.platform() === 'win32' ? ';' : ':';
    const extensions = os.platform() === 'win32' ? ['.exe', '.cmd', '.bat', ''] : [''];

    for (const dir of pathEnv.split(pathSeparator)) {
        for (const ext of extensions) {
            const fullPath = path.join(dir, name + ext);
            if (fs.existsSync(fullPath)) {
                return fullPath;
            }
        }
    }

    return undefined;
}
