use wasm_bindgen_test::*;

// https://rustwasm.github.io/wasm-bindgen/wasm-bindgen-test/browsers.html
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_1() {
	assert_eq!(0, 0);
}

// https://rustwasm.github.io/wasm-bindgen/wasm-bindgen-test/asynchronous-tests.html
#[wasm_bindgen_test(async)]
async fn test_2() {
	assert_eq!(0, 0);
}
