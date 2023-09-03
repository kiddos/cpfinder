# CP finder

very basic implementation of copy paste finder

## Installation

cargo install --git https://github.com/kiddos/cpfinder

## Usage


```
Usage: cpfinder [OPTIONS] <ROOT> <SOURCE_TYPE>

Arguments:
  <ROOT>         
  <SOURCE_TYPE>  source file type [possible values: java, cpp, c, rust, javascript, python]

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
  -h, --help
          Print help
  -V, --version
          Print version
```
