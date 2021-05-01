import { JSON, JSONEncoder } from "assemblyscript-json";

import {wasm_plugin_imported__please_capitalize_this} from "./env"

export function wasm_plugin_exported__hello(): u64 {
    let data = String.UTF8.encode("\"Hello, Host!\"");
    let ptr = changetype<usize>(data);
    __pin(ptr);
    let len = data.byteLength as u64;

    return len << 32 | ptr as u64;
}

function please_capitalize_this(input: String): String {
    let data = String.UTF8.encode(input);
    let ptr = changetype<usize>(data);
    let len = data.byteLength;

    let fat_ptr = wasm_plugin_imported__please_capitalize_this(ptr as u32, len as u32);
    let len2 = (fat_ptr >> 32) & 0xFFFFFF;
    let ptr2 = fat_ptr & 0xFFFFFF;
    let result = String.UTF8.decodeUnsafe(ptr2 as usize, len2 as usize, false);
    return result
}

export function wasm_plugin_exported__echo(ptr: u32, len: u32): u64 {
    let input = String.UTF8.decodeUnsafe(ptr as usize, len as usize, false);
    let output = please_capitalize_this(input);

    let data = String.UTF8.encode(output);
    let ptr = changetype<usize>(data);
    __pin(ptr);
    let len = data.byteLength as u64;

    return len << 32 | ptr as u64;
}

export function wasm_plugin_exported__favorite_numbers(): u64 {
    let encoder = new JSONEncoder();
    encoder.pushArray(null);
    encoder.setInteger(null, 1);
    encoder.setInteger(null, 2);
    encoder.setInteger(null, 43);
    encoder.popArray();
    let response: string = encoder.toString();
    let encoded_response = String.UTF8.encode(response);
    let ptr = changetype<usize>(encoded_response);
    __pin(ptr);
    let len = encoded_response.byteLength as u64;

    return len << 32 | ptr as u64;
 }

export function allocate_message_buffer(len: u32): u32 {
    let buffer = new ArrayBuffer(len);
    let ptr = changetype<usize>(buffer);
    __pin(ptr);
    return ptr as u32;
}

export function free_message_buffer(ptr: u32, _len: u32): void {
    __unpin(ptr as usize);
}
