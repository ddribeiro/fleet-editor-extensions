-- Flint LSP configuration for Neovim
--
-- Usage with nvim-lspconfig:
--   require('flint').setup()
--
-- Or manually:
--   vim.lsp.start({
--     name = 'flint',
--     cmd = { 'flint', 'lsp' },
--     root_dir = vim.fs.root(0, { 'default.yml', '.fleetlint.toml', 'fleets' }),
--     filetypes = { 'yaml' },
--   })

local M = {}

--- Check if the current project is a Fleet GitOps repo.
local function is_fleet_project(bufnr)
  local root = vim.fs.root(bufnr, { 'default.yml', 'default.yaml', '.fleetlint.toml', 'fleets' })
  return root ~= nil
end

--- Setup Flint LSP for the current buffer.
--- Call this from an autocommand or your lspconfig setup.
function M.setup(opts)
  opts = opts or {}

  vim.api.nvim_create_autocmd('FileType', {
    pattern = 'yaml',
    callback = function(args)
      if not is_fleet_project(args.buf) then
        return
      end

      local root = vim.fs.root(args.buf, { 'default.yml', 'default.yaml', '.fleetlint.toml', 'fleets' })

      vim.lsp.start({
        name = 'flint',
        cmd = opts.cmd or { 'flint', 'lsp' },
        root_dir = root,
        settings = opts.settings or {},
        init_options = opts.init_options or {},
      }, {
        bufnr = args.buf,
        reuse_client = function(client, config)
          return client.name == 'flint' and client.config.root_dir == config.root_dir
        end,
      })
    end,
  })
end

--- Manual lspconfig-compatible server definition.
--- For use with: require('lspconfig.configs').flint = require('flint').lspconfig
M.lspconfig = {
  default_config = {
    cmd = { 'flint', 'lsp' },
    filetypes = { 'yaml' },
    root_dir = function(fname)
      return vim.fs.root(fname, { 'default.yml', 'default.yaml', '.fleetlint.toml', 'fleets' })
    end,
    single_file_support = false,
    settings = {},
  },
  docs = {
    description = [[
Flint — Fleet GitOps YAML linter and language server.

Provides real-time validation, autocompletion, and diagnostics for
Fleet GitOps configuration files (default.yml, fleets/*.yml, lib/*.yml).

Install: https://github.com/fleetdm/fleet-editor-extensions/releases
]],
  },
}

return M
