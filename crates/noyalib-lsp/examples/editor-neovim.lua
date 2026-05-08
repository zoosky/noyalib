-- SPDX-License-Identifier: Apache-2.0 OR MIT
--
-- noyalib-lsp configuration for Neovim (≥ 0.10) via nvim-lspconfig.
--
-- Drop this snippet into your Lua config — typically
-- `~/.config/nvim/lua/plugins/yaml.lua` if you use lazy.nvim,
-- or inline it in `init.lua`. Adjust `cmd` if `noyalib-lsp` is
-- not on $PATH (e.g., a project-local install under `.bin/`).

return {
  -- 1. Register noyalib-lsp as a custom server with lspconfig.
  {
    "neovim/nvim-lspconfig",
    opts = function(_, opts)
      local lspconfig = require("lspconfig")
      local configs = require("lspconfig.configs")

      if not configs.noyalib then
        configs.noyalib = {
          default_config = {
            cmd = { "noyalib-lsp" },
            filetypes = { "yaml" },
            root_dir = function(fname)
              return lspconfig.util.find_git_ancestor(fname)
                or vim.fn.getcwd()
            end,
            single_file_support = true,
            settings = {},
          },
        }
      end

      lspconfig.noyalib.setup({
        on_attach = function(_client, bufnr)
          -- Format-on-save (uses textDocument/formatting → noyafmt).
          vim.api.nvim_create_autocmd("BufWritePre", {
            buffer = bufnr,
            callback = function()
              vim.lsp.buf.format({ async = false, timeout_ms = 2000 })
            end,
          })

          -- Convenient keymaps; tune to your scheme.
          local map = function(lhs, rhs)
            vim.keymap.set("n", lhs, rhs, { buffer = bufnr, silent = true })
          end
          map("K", vim.lsp.buf.hover)              -- value + span at cursor
          map("[d", vim.diagnostic.goto_prev)
          map("]d", vim.diagnostic.goto_next)
          map("<leader>f", function()
            vim.lsp.buf.format({ async = true })
          end)
        end,
      })
    end,
  },
}
