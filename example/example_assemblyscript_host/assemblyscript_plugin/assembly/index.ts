import { JSON, JSONEncoder } from "assemblyscript-json";

export const MESSAGE_BUFFER = new ArrayBuffer(1024 * 10);

export function wasm_plugin_exported__hello(): i32 {
  return String.UTF8.encodeUnsafe(changetype<usize>("\"test\""), 6, changetype<usize>(MESSAGE_BUFFER)) as i32;
}

 export function wasm_plugin_exported__echo(): i32 {
    let input: string = String.UTF8.decode(MESSAGE_BUFFER);
    return String.UTF8.encodeUnsafe(changetype<usize>(input), input.length, changetype<usize>(MESSAGE_BUFFER)) as i32;
 }

 export function wasm_plugin_exported__favorite_numbers(): i32 {
     let encoder = new JSONEncoder();
     encoder.pushArray(null);
     encoder.setInteger(null, 1);
     encoder.setInteger(null, 2);
     encoder.setInteger(null, 43);
     encoder.popArray();
     let response: string = encoder.toString();
     return String.UTF8.encodeUnsafe(changetype<usize>(response), response.length, changetype<usize>(MESSAGE_BUFFER)) as i32;
 }
