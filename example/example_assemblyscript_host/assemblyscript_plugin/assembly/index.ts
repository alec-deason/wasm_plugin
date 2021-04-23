import { JSON, JSONEncoder } from "assemblyscript-json";

import {wasm_plugin_imported__please_capitalize_this} from "./env"

export const MESSAGE_BUFFER = new ArrayBuffer(1024 * 10);

export function wasm_plugin_exported__hello(): i32 {
  return String.UTF8.encodeUnsafe(changetype<usize>("\"Hello, Host!\""), 14, changetype<usize>(MESSAGE_BUFFER)) as i32;
}


function please_capitalize_this(input: String): String {
    let len = String.UTF8.encodeUnsafe(changetype<usize>(input), input.length, changetype<usize>(MESSAGE_BUFFER)) as i32;
    let response_len = wasm_plugin_imported__please_capitalize_this(len);
    return String.UTF8.decode(MESSAGE_BUFFER.slice(0, response_len));
}

export function wasm_plugin_exported__echo(len: i32): i32 {
   let input: string = String.UTF8.decode(MESSAGE_BUFFER.slice(0, len));
   let output = please_capitalize_this(input);
   return String.UTF8.encodeUnsafe(changetype<usize>(output), output.length, changetype<usize>(MESSAGE_BUFFER)) as i32;
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
