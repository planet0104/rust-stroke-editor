cargo build --target asmjs-unknown-emscripten --release
copy target\asmjs-unknown-emscripten\release\rust-stroke-editor.js html\rust-stroke-editor.js