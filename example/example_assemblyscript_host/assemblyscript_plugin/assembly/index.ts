export const MESSAGE_BUFFER = new ArrayBuffer(1024 * 10);

export function wasm_plugin_exported__hello(): i32 {
  return String.UTF8.encodeUnsafe(changetype<usize>("\"test\""), 6, changetype<usize>(MESSAGE_BUFFER));
}
