# CP finder

Very basic implementation of copy paste finder and Language Server Protocol (LSP) server.

## Installation

```bash
cargo install --git https://github.com/kiddos/cpfinder
```

## Usage

```
Usage: cpfinder [OPTIONS] [ROOT] [SOURCE_TYPE] [COMMAND]

Commands:
  lsp   run as language server
  help  Print this message or the help of the given subcommand(s)

Arguments:
  [ROOT]         root folder to scan
  [SOURCE_TYPE]  source file type [possible values: java, cpp, c, rust, javascript, python]

Options:
      --min-line-count <MIN_LINE_COUNT>
          minimum number of lines to considered as copy paste [default: 6]
      --min-char-count <MIN_CHAR_COUNT>
          minimum characters to considered as copy paste [default: 80]
      --ignore-folders <IGNORE_FOLDERS>
          folders to ignore [default: thirdparty,test,node_modules]
      --list-source-folder
          list source files
      --list-top-result <LIST_TOP_RESULT>
          top number of results to list [default: 30]
      --lsp
          run as language server
  -h, --help
          Print help
  -V, --version
          Print version
```

## LSP Server Integration & Options

`cpfinder` can run as a Language Server Protocol (LSP) server communicating over standard input/output (`stdio`).

### Starting the LSP Server

You can start the LSP server mode in either of two ways:

```bash
cpfinder --lsp
# or
cpfinder lsp
```

### LSP Functionality & Configuration

When running in LSP mode, `cpfinder`:
* Automatically discovers and scans workspace files across supported source types (`.java`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.h`, `.c`, `.rs`, `.js`, `.jsx`, `.ts`, `.tsx`, `.py`).
* Indexes open documents and workspace files to detect code duplication across the project in real time.
* Publishes LSP diagnostics (with `HINT` severity) for detected copy-pasted blocks.

#### Options & Default Configuration

The LSP server uses the following default detection settings:

* **`--min-line-count`**: Minimum number of consecutive identical lines required to trigger a duplicate warning.
  * **Default**: `6`
* **`--min-char-count`**: Minimum total characters across duplicate lines required to trigger a duplicate warning.
  * **Default**: `80`
* **`--ignore-folders`**: Comma-separated folder names/patterns to exclude from scanning.
  * **CLI Default**: `thirdparty,test,node_modules`
  * **LSP Default Ignored Folders**: `thirdparty,test,node_modules,target,.git`
* **`max_file_size`**: Maximum file size in bytes to index and analyze in LSP mode. Files larger than this threshold are skipped.
  * **Default**: `1048576` (1 MB)

#### Editor Configuration Example

To configure `cpfinder` as an LSP server in editors (e.g., Neovim, VS Code, Helix):

**Neovim (`vim.lsp.start`) example:**

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = { "rust", "python", "javascript", "typescript", "c", "cpp", "java" },
  callback = function()
    vim.lsp.start({
      name = "cpfinder",
      cmd = { "cpfinder", "--lsp" },
      root_dir = vim.fs.dirname(vim.fs.find({ ".git", "Cargo.toml" }, { upward = true })[1]),
    })
  end,
})
```
