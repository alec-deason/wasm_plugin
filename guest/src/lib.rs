#[macro_export]
macro_rules! initialize_message_buffer {
    () => {
        #[no_mangle]
        static mut MESSAGE_BUFFER: [u8; 1024 * 10] = [0; 1024 * 10];
    };
}

#[macro_export]
macro_rules! shim_getrandom {
    () => {
        use getrandom::register_custom_getrandom;

        use getrandom::Error;

        extern "C" {
            fn __getrandom(ptr: i32, len: i32);
        }

        fn external_getrandom(buf: &mut [u8]) -> Result<(), Error> {
            let len = buf.len();
            let ptr = buf.as_ptr();
            unsafe {
                __getrandom(ptr as i32, len as i32);
            }
            Ok(())
        }
        register_custom_getrandom!(external_getrandom);
    };
}

#[macro_export]
macro_rules! export_plugin_function_with_no_input {
    ($name:ident, $function:path) => {
        const _: () = {
            #[no_mangle]
            pub extern "C" fn $name() -> i32 {
                let buf = unsafe { &mut crate::MESSAGE_BUFFER };
                let message: Vec<u8> = bincode::serialize(&$function()).unwrap();
                let len = message.len() as i32;
                buf.iter_mut().zip(message).for_each(|(dst, src)| {
                    *dst = src;
                });
                len
            }
        };
    };
}

#[macro_export]
macro_rules! export_plugin_function_with_input_message {
    ($name:ident, $function:path) => {
        const _: () = {
            #[no_mangle]
            pub extern "C" fn $name() -> i32 {
                let buf = unsafe { &mut crate::MESSAGE_BUFFER };
                let message = bincode::deserialize(buf).unwrap();

                let message: Vec<u8> = bincode::serialize(&$function(message)).unwrap();
                let len = message.len() as i32;
                buf.iter_mut().zip(message).for_each(|(dst, src)| {
                    *dst = src;
                });
                len
            }
        };
    };
}
