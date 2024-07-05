use duplicate::duplicate_item;
use magnus::{Error, Module, Object, RHash};
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

fn await_without_gvl<T>(future: impl std::future::Future<Output = T>) -> T {
    RUNTIME.block_on(future)
}

// TODO: Use rb_thread_call_without_gvl
// fn await_without_gvl<F, T>(f: F) -> T
// where
//     F: std::future::Future<Output = T>,
// {
//     let return_ptr: Box<T> = unsafe {
//         let future = Box::pin(f);
//         let returned_ptr = rb_sys::rb_thread_call_without_gvl(
//             Some(await_without_gvl_inner::<T>),
//             &future as *const _ as *mut std::ffi::c_void,
//             None,
//             std::ptr::null_mut(),
//         );
//         Box::from_raw(returned_ptr as *mut T)
//     };
//
//     return *return_ptr;
//
//     extern "C" fn await_without_gvl_inner<T>(data: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
//         let future = unsafe {
//             Box::from_raw(data as *mut std::pin::Pin<Box<dyn std::future::Future<Output = T>>>)
//         };
//
//         let value = RUNTIME.block_on(future);
//
//         Box::into_raw(Box::new(value)) as *mut std::ffi::c_void
//     }
// }
fn to_ruby_error(err: utaformatix::Error) -> magnus::Error {
    let ruby = magnus::Ruby::get().expect("Failed to get Ruby pointer");
    let error = ruby
        .define_error("UtaFormatix", ruby.exception_runtime_error())
        .unwrap();
    let error = error
        .define_error("Error", ruby.exception_runtime_error())
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
        fn_name              format_enum           kind;
        [parse_standard_mid] [Format::StandardMid] ["Standard MIDI"];
        [parse_music_xml]    [Format::MusicXml]    ["MusicXML"];
        [parse_ccs]          [Format::Ccs]         ["CeVIO's project"];
        [parse_dv]           [Format::Dv]          ["DeepVocal's project"];
        [parse_ustx]         [Format::Ustx]        ["OpenUtau's project"];
        [parse_ppsf]         [Format::Ppsf]        ["Piapro Studio's project"];
        [parse_s5p]          [Format::S5p]         ["Old Synthesizer V's project"];
        [parse_svp]          [Format::Svp]         ["Synthesizer V's project"];
        [parse_tssln]        [Format::Tssln]       ["VoiSona's project"];
        [parse_uf_data]      [Format::UfData]      ["UtaFormatix data"];
        [parse_vocaloid_mid] [Format::VocaloidMid] ["VOCALOID 1's project"];
        [parse_vsq]          [Format::Vsq]         ["VOCALOID 2's project"];
        [parse_vsqx]         [Format::Vsqx]        ["VOCALOID 3/4's project"];
        [parse_vpr]          [Format::Vpr]         ["VOCALOID 5's project"];
    )]
    pub fn fn_name(
        &self,
        data: bytes::Bytes,
        pitch: Option<bool>,
        default_lyric: Option<String>,
    ) -> RubyResult<RHash> {
        let mut options = ParseOptions::default();
        if let Some(pitch) = pitch {
            options.pitch = pitch;
        }
        if let Some(default_lyric) = default_lyric {
            options.default_lyric = default_lyric;
        }
        let data = data.into_iter().collect::<Vec<_>>();
        let ufdata =
            await_without_gvl(self.inner.fn_name(&data, options)).map_err(to_ruby_error)?;

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
    core.define_method(
        "parse_standard_mid",
        magnus::method!(Core::parse_standard_mid, 3),
    )?;
    core.define_method("parse_music_xml", magnus::method!(Core::parse_music_xml, 3))?;
    core.define_method("parse_ccs", magnus::method!(Core::parse_ccs, 3))?;
    core.define_method("parse_dv", magnus::method!(Core::parse_dv, 3))?;
    core.define_method("parse_ustx", magnus::method!(Core::parse_ustx, 3))?;
    core.define_method("parse_ppsf", magnus::method!(Core::parse_ppsf, 3))?;
    core.define_method("parse_s5p", magnus::method!(Core::parse_s5p, 3))?;
    core.define_method("parse_svp", magnus::method!(Core::parse_svp, 3))?;
    core.define_method("parse_tssln", magnus::method!(Core::parse_tssln, 3))?;
    core.define_method("parse_uf_data", magnus::method!(Core::parse_uf_data, 3))?;
    core.define_method(
        "parse_vocaloid_mid",
        magnus::method!(Core::parse_vocaloid_mid, 3),
    )?;
    core.define_method("parse_vsq", magnus::method!(Core::parse_vsq, 3))?;
    core.define_method("parse_vsqx", magnus::method!(Core::parse_vsqx, 3))?;
    core.define_method("parse_vpr", magnus::method!(Core::parse_vpr, 3))?;

    Ok(())
}
