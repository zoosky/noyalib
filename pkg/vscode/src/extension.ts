// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// VS Code extension entry point.
//
// Spawns `noyalib-lsp` over stdio and wires it into VS Code's
// language-client framework. The path can be overridden via the
// `noyalib.path` workspace setting; otherwise the binary that
// shipped inside this extension's `bin/` directory is used.

import * as path from "path";
import * as vscode from "vscode";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
    const config = vscode.workspace.getConfiguration("noyalib");
    const configuredPath: string = config.get("path") ?? "";
    const bundledPath = context.asAbsolutePath(
        path.join("bin", process.platform === "win32" ? "noyalib-lsp.exe" : "noyalib-lsp"),
    );
    const serverPath = configuredPath || bundledPath;

    const serverOptions: ServerOptions = {
        run:   { command: serverPath, transport: TransportKind.stdio },
        debug: { command: serverPath, transport: TransportKind.stdio },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "yaml" }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{yaml,yml}"),
        },
    };

    client = new LanguageClient(
        "noyalib",
        "noyalib LSP",
        serverOptions,
        clientOptions,
    );
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
}
