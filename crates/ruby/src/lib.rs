use duplicate::{duplicate, duplicate_item};
use magnus::{value::ReprValue, Error, Module, Object, RArray, RHash, Value};
use once_cell::sync::Lazy;
use utaformatix::{base::UtaFormatix, ParseOptions};

type RubyResult<T> = Result<T, magnus::Error>;

#[magnus::wrap(class = "UtaFormatix::Core")]
struct Core {
    inner: UtaFormatix,
}

static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
});

fn without_gvl<F, A, R>(arg: A, f: F) -> R
where
    F: Fn(A) -> R,
{
    return unsafe {
        let arg = Box::new((arg, f));
        let returned_ptr = rb_sys::rb_thread_call_without_gvl(
            Some(call_without_gvl::<F, A, R>),
            Box::into_raw(arg) as *mut std::ffi::c_void,
            None,
            std::ptr::null_mut(),
        );
        *Box::from_raw(returned_ptr as *mut R)
    };

    extern "C" fn call_without_gvl<F, A, R>(arg: *mut std::ffi::c_void) -> *mut std::ffi::c_void
    where
        F: Fn(A) -> R,
    {
        let (arg, f): (A, F) = unsafe { *Box::from_raw(arg as *mut (A, F)) };
        Box::into_raw(Box::new(f(arg))) as *mut std::ffi::c_void
    }
}

fn to_ruby_error(err: utaformatix::Error) -> magnus::Error {
    let ruby = magnus::Ruby::get().expect("Failed to get Ruby pointer");
    let error = ruby.define_module("UtaFormatix").unwrap();
    let error = error
        .define_error("Error", ruby.exception_standard_error())
        .unwrap();

    let exception = match err {
        utaformatix::Error::IllegalFile(ref kind) => {
            let error = error
                .define_error("IllegalFile", ruby.exception_runtime_error())
                .unwrap();

            error
                .define_error(kind.to_string(), ruby.exception_runtime_error())
                .unwrap()
        }
        ref err => error
            .define_error(err.as_ref(), ruby.exception_runtime_error())
            .unwrap(),
    };

    magnus::Error::new(exception, err.to_string())
}

impl Core {
    fn new() -> Self {
        Self {
            inner: UtaFormatix::new(),
        }
    }

    #[duplicate_item(
        fn_name;
        [parse_standard_mid];
        [parse_music_xml];
        [parse_ccs];
        [parse_dv];
        [parse_ustx];
        [parse_ppsf];
        [parse_s5p];
        [parse_svp];
        [parse_tssln];
        [parse_uf_data];
        [parse_vocaloid_mid];
        [parse_vsq];
        [parse_vsqx];
        [parse_vpr];
    )]
    pub fn fn_name(&self, args: &[Value]) -> RubyResult<RHash> {
        let args = magnus::scan_args::scan_args::<(bytes::Bytes,), (), (), (), RHash, ()>(args)?;

        let mut options = ParseOptions::default();
        let ruby = magnus::Ruby::get().expect("Failed to get Ruby pointer");
        if let Some(pitch) = args.keywords.get(ruby.to_symbol("pitch")) {
            options.pitch = pitch.to_bool();
        }
        if let Some(default_lyric) = args.keywords.get(ruby.to_symbol("default_lyric")) {
            options.default_lyric = default_lyric.to_string();
        }
        let data = args.required.0.into_iter().collect::<Vec<_>>();
        let ufdata = without_gvl((self, data, options), |(this, data, options)| {
            RUNTIME.block_on(this.inner.fn_name(&data, options))
        })
        .map_err(to_ruby_error)?;

        let value: magnus::RHash = serde_magnus::serialize(&ufdata).map_err(|e| {
            magnus::Error::new(
                magnus::Ruby::get().unwrap().exception_runtime_error(),
                e.to_string(),
            )
        })?;

        Ok(value)
    }

    #[duplicate_item(
        fn_name;
        [parse_ust];
    )]
    pub fn fn_name(&self, args: &[Value]) -> RubyResult<RHash> {
        let args = magnus::scan_args::scan_args::<(Value,), (), (), (), RHash, ()>(args)?;

        let mut options = ParseOptions::default();
        if let Some(pitch) = args.keywords.get("pitch") {
            options.pitch = pitch.to_bool();
        }
        if let Some(default_lyric) = args.keywords.get("default_lyric") {
            options.default_lyric = default_lyric.to_string();
        }
        let ruby = magnus::Ruby::get().expect("Failed to get Ruby pointer");
        let source = args.required.0;
        let source = if source.is_kind_of(ruby.class_array()) {
            RArray::from_value(source)
                .expect("Failed to convert to RArray")
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            vec![source]
        };
        let source = source
            .into_iter()
            .map(|v| v.to_r_string())
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|s| s.to_bytes().to_vec())
            .collect::<Vec<_>>();
        let ufdata = without_gvl((self, source, options), |(this, source, options)| {
            let slices = source.iter().map(|s| s.as_slice()).collect::<Vec<_>>();
            RUNTIME.block_on(this.inner.fn_name(&slices, options))
        })
        .map_err(to_ruby_error)?;

        let value: magnus::RHash = serde_magnus::serialize(&ufdata).map_err(|e| {
            magnus::Error::new(
                magnus::Ruby::get().unwrap().exception_runtime_error(),
                e.to_string(),
            )
        })?;

        Ok(value)
    }
}

#[magnus::init(name = "core")]
fn init(ruby: &magnus::Ruby) -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let utaformatix_root = ruby.define_module("UtaFormatix")?;

    let error_root = utaformatix_root.define_error("Error", ruby.exception_standard_error())?;
    error_root.define_error("EmptyProject", ruby.exception_runtime_error())?;
    error_root.define_error("IllegalNotePosition", ruby.exception_runtime_error())?;
    error_root.define_error("NotesOverlapping", ruby.exception_runtime_error())?;
    error_root.define_error("UnsupportedFileFormat", ruby.exception_runtime_error())?;
    error_root.define_error("UnsupportedLegacyPpsf", ruby.exception_runtime_error())?;
    error_root.define_error("Unexpected", ruby.exception_runtime_error())?;
    let illegal_file = error_root.define_error("IllegalFile", ruby.exception_runtime_error())?;
    illegal_file.define_error("UnknownVsqVersion", ruby.exception_runtime_error())?;
    illegal_file.define_error("XmlRootNotFound", ruby.exception_runtime_error())?;
    illegal_file.define_error("XmlElementNotFound", ruby.exception_runtime_error())?;
    illegal_file.define_error("IllegalXmlValue", ruby.exception_runtime_error())?;
    illegal_file.define_error("IllegalXmlAttribute", ruby.exception_runtime_error())?;
    illegal_file.define_error("IllegalMidiFile", ruby.exception_runtime_error())?;
    illegal_file.define_error("IllegalTsslnFile", ruby.exception_runtime_error())?;

    let core = utaformatix_root.define_class("Core", ruby.class_object())?;
    core.define_singleton_method("new", magnus::function!(Core::new, 0))?;
    duplicate! {
        [
            fn_name;
            [parse_standard_mid];
            [parse_music_xml];
            [parse_ccs];
            [parse_dv];
            [parse_ustx];
            [parse_ppsf];
            [parse_s5p];
            [parse_svp];
            [parse_tssln];
            [parse_ust];
            [parse_uf_data];
            [parse_vocaloid_mid];
            [parse_vsq];
            [parse_vsqx];
            [parse_vpr];
        ]
        core.define_method(
            stringify!(fn_name),
            magnus::method!(Core::fn_name, -1),
        )?;
    }

    Ok(())
}
