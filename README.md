# watc
This is a WIP [WebAssembly text format](https://developer.mozilla.org/en-US/docs/WebAssembly/Understanding_the_text_format) to [WebAssembly](https://developer.mozilla.org/en-US/docs/WebAssembly) compiler.
Currently it can compile `add.wat` which contains a function that adds two numbers you pass to it and returns the result.

## How to use
Compile the `.wat` file:
```
cargo r add.wat
```
This will output a file called `add.wasm`. Run it with a WebAssembly interpreter (I am using wasmer in this example):
```
wasmer add.wasm -i add 2 3
```
The function will return the number `5`.
