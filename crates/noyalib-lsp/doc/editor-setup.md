# Editor setup for noyalib-lsp

The noyalib LSP server speaks the standard
[Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
over stdio, so any editor with LSP client support can wire it up.
This page collects the working configurations for the major editors.

For the full list of methods the server implements, see
[`protocol-coverage.md`](./protocol-coverage.md).

## Install

```sh
cargo install noyalib-lsp
```

The binary lands in `~/.cargo/bin/noyalib-lsp`. The remainder of
this document assumes that path is on `PATH`; if not, substitute
the absolute path in each editor's config.

## VS Code

The recommended path is the **noyalib VS Code extension**, which
bundles the LSP server and configures it automatically. Install
from the marketplace:

```
ext install noyalib.noyalib-vscode
```

For a manual setup (e.g. you want to drive your own LSP client),
add to `settings.json`:

```jsonc
{
  "yaml.server.path": "noyalib-lsp",
  "[yaml]": {
    "editor.defaultFormatter": "noyalib.noyalib-vscode",
    "editor.formatOnSave": true
  }
}
```

## Neovim

### nvim-lspconfig

```lua
-- ~/.config/nvim/init.lua
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")

if not configs.noyalib then
  configs.noyalib = {
    default_config = {
      cmd = { "noyalib-lsp" },
      filetypes = { "yaml" },
      root_dir = lspconfig.util.find_git_ancestor,
      single_file_support = true,
    },
  }
end

lspconfig.noyalib.setup({
  on_attach = function(client, bufnr)
    -- Format on save.
    vim.api.nvim_create_autocmd("BufWritePre", {
      buffer = bufnr,
      callback = function()
        vim.lsp.buf.format({ bufnr = bufnr })
      end,
    })
  end,
})
```

### Native vim.lsp (Neovim 0.11+)

```lua
vim.lsp.config.noyalib = {
  cmd = { "noyalib-lsp" },
  filetypes = { "yaml" },
  root_markers = { ".git" },
}
vim.lsp.enable({ "noyalib" })
```

## Helix

`~/.config/helix/languages.toml`:

```toml
[[language]]
name = "yaml"
language-servers = ["noyalib-lsp"]
auto-format = true

[language-server.noyalib-lsp]
command = "noyalib-lsp"
```

## Zed

`~/.config/zed/settings.json`:

```jsonc
{
  "lsp": {
    "noyalib-lsp": {
      "binary": {
        "path": "noyalib-lsp",
        "arguments": []
      }
    }
  },
  "languages": {
    "YAML": {
      "language_servers": ["noyalib-lsp"],
      "format_on_save": "on"
    }
  }
}
```

## Emacs

### lsp-mode

```elisp
;; ~/.emacs.d/init.el
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(yaml-mode . "yaml"))
  (lsp-register-client
   (make-lsp-client :new-connection (lsp-stdio-connection "noyalib-lsp")
                    :major-modes '(yaml-mode yaml-ts-mode)
                    :server-id 'noyalib-lsp)))

(add-hook 'yaml-mode-hook #'lsp-deferred)
(add-hook 'before-save-hook #'lsp-format-buffer)
```

### eglot (Emacs 29+)

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((yaml-mode yaml-ts-mode) . ("noyalib-lsp"))))

(add-hook 'yaml-mode-hook 'eglot-ensure)
(add-hook 'yaml-ts-mode-hook 'eglot-ensure)
```

## Sublime Text

Install the LSP package, then add to
`Preferences → Package Settings → LSP → Settings`:

```jsonc
{
  "clients": {
    "noyalib-lsp": {
      "enabled": true,
      "command": ["noyalib-lsp"],
      "selector": "source.yaml"
    }
  }
}
```

## Kakoune

`~/.config/kak/kakrc`:

```kak
hook global WinSetOption filetype=yaml %{
  lsp-enable-window
  set-option global lsp_servers %{
    [noyalib-lsp]
    root_globs = [".git"]
    args = []
  }
}
```

## JetBrains IDEs (IntelliJ, RustRover, GoLand, etc.)

Install the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij)
plugin, then in **Settings → LSP**:

- Add a new server named `noyalib-lsp`
- Command: `noyalib-lsp`
- File mappings: `*.yaml`, `*.yml`

## Verifying the server is working

```sh
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' \
  | noyalib-lsp
```

Should respond with a JSON-RPC `result` containing the server's
capabilities. If it hangs or errors, verify:

1. `which noyalib-lsp` returns a path
2. The binary is executable (`chmod +x` if needed)
3. The version matches your editor's expectations
   (`noyalib-lsp --version`)

## Troubleshooting

### Diagnostics don't appear

Most editors require the LSP client to be configured for the YAML
file type. Confirm with:

```vim
:LspInfo            " Neovim with nvim-lspconfig
:lsp-info           " Helix
M-x lsp-describe-session   " Emacs / lsp-mode
```

### Format-on-save doesn't run

The server provides `textDocument/formatting`. If the editor
doesn't trigger it on save, set the editor's "format on save"
option *and* ensure the LSP client is the registered formatter
for YAML (some editors prefer the built-in tree-sitter formatter
unless told otherwise — see each editor's section above for the
explicit override).

### Hover descriptions show `null`

Hover content depends on a JSON Schema being available for the
document. Configure the schema via the editor's standard YAML
schema mapping (e.g. `yaml.schemas` in VS Code, or the
[yaml-language-server schema selector](https://github.com/redhat-developer/yaml-language-server#using-inlined-schema)
inline pragma). With no schema, hover returns the parsed value
type only.

### High CPU on large files

Large YAML files (>10 MB) trigger a full-document re-parse on
every keystroke under the default `didChange` debounce. Either:

- Increase the debounce in your editor's LSP client settings
- Use `--max-document-length` to refuse very large files
- File an issue if the parse is structurally slow — the bench
  numbers in `crates/noyalib-lsp/benches/lsp_handlers.rs` are the
  reference

## Related

- [Protocol coverage](./protocol-coverage.md) — which LSP methods
  are implemented
- [Crate README](../README.md) — full server overview
