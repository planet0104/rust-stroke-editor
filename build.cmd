::
wasm-pack build --target web
copy pkg\rust_stroke_editor_bg.wasm html\rust_stroke_editor_bg.wasm
copy pkg\rust_stroke_editor.js html\rust_stroke_editor.js

::npm install http-server -g
::cd html
::http-server -p 3000 --cors
