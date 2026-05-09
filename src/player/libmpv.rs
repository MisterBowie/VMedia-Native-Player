use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_double, c_int, c_longlong, c_uint, c_void},
    path::Path,
    ptr,
    rc::Rc,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use gtk::{glib, prelude::*};

use crate::core::models::{Track, TrackKind};

const MPV_FORMAT_STRING: c_int = 1;
const MPV_FORMAT_FLAG: c_int = 3;
const MPV_FORMAT_INT64: c_int = 4;
const MPV_FORMAT_DOUBLE: c_int = 5;

const MPV_EVENT_NONE: c_int = 0;
const MPV_EVENT_SHUTDOWN: c_int = 1;
const MPV_EVENT_END_FILE: c_int = 7;
const MPV_EVENT_FILE_LOADED: c_int = 8;
const MPV_EVENT_PLAYBACK_RESTART: c_int = 21;
const MPV_EVENT_PROPERTY_CHANGE: c_int = 22;

const OBSERVED_PATH_PROPERTY: u64 = 1;
const OBSERVED_PAUSE_PROPERTY: u64 = 2;
const OBSERVED_POSITION_PROPERTY: u64 = 3;
const OBSERVED_DURATION_PROPERTY: u64 = 4;
const OBSERVED_VOLUME_PROPERTY: u64 = 5;
const OBSERVED_AUDIO_TRACK_PROPERTY: u64 = 6;
const OBSERVED_SUBTITLE_TRACK_PROPERTY: u64 = 7;
const OBSERVED_TRACK_COUNT_PROPERTY: u64 = 8;

const OBSERVED_PLAYBACK_PROPERTIES: &[(u64, &str, c_int)] = &[
    (OBSERVED_PATH_PROPERTY, "path", MPV_FORMAT_STRING),
    (OBSERVED_PAUSE_PROPERTY, "pause", MPV_FORMAT_FLAG),
    (OBSERVED_POSITION_PROPERTY, "time-pos", MPV_FORMAT_DOUBLE),
    (OBSERVED_DURATION_PROPERTY, "duration", MPV_FORMAT_DOUBLE),
    (OBSERVED_VOLUME_PROPERTY, "volume", MPV_FORMAT_DOUBLE),
    (OBSERVED_AUDIO_TRACK_PROPERTY, "aid", MPV_FORMAT_INT64),
    (OBSERVED_SUBTITLE_TRACK_PROPERTY, "sid", MPV_FORMAT_INT64),
    (
        OBSERVED_TRACK_COUNT_PROPERTY,
        "track-list/count",
        MPV_FORMAT_INT64,
    ),
];

const MPV_RENDER_PARAM_INVALID: c_int = 0;
const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;

const MPV_RENDER_UPDATE_FRAME: u64 = 1 << 0;
const MPV_RENDER_API_TYPE_OPENGL: &[u8] = b"opengl\0";
const GL_FRAMEBUFFER_BINDING: c_uint = 0x8CA6;

#[allow(non_camel_case_types)]
type mpv_handle = c_void;
#[allow(non_camel_case_types)]
type mpv_render_context = c_void;

#[repr(C)]
struct mpv_render_param {
    type_: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct mpv_opengl_init_params {
    get_proc_address: Option<unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void>,
    get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
struct mpv_opengl_fbo {
    fbo: c_int,
    w: c_int,
    h: c_int,
    internal_format: c_int,
}

#[repr(C)]
struct mpv_event {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

struct RenderUpdateContext {
    main_context: glib::MainContext,
    gl_area: Mutex<Option<glib::SendWeakRef<gtk::GLArea>>>,
    wakeup_pending: AtomicBool,
    frame_available: AtomicBool,
}

struct LibMpvInner {
    handle: *mut mpv_handle,
    render_context: Mutex<*mut mpv_render_context>,
    render_update_context: &'static RenderUpdateContext,
}

#[derive(Clone)]
pub struct LibMpv {
    inner: Rc<LibMpvInner>,
}

#[link(name = "GL")]
unsafe extern "C" {
    fn glGetIntegerv(pname: c_uint, data: *mut c_int);
}

#[link(name = "mpv")]
unsafe extern "C" {
    fn mpv_create() -> *mut mpv_handle;
    fn mpv_initialize(ctx: *mut mpv_handle) -> c_int;
    fn mpv_terminate_destroy(ctx: *mut mpv_handle);
    fn mpv_set_option_string(
        ctx: *mut mpv_handle,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn mpv_command(ctx: *mut mpv_handle, args: *const *const c_char) -> c_int;
    fn mpv_get_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    fn mpv_error_string(error: c_int) -> *const c_char;
    fn mpv_free(data: *mut c_void);
    fn mpv_wait_event(ctx: *mut mpv_handle, timeout: c_double) -> *mut mpv_event;
    fn mpv_observe_property(
        ctx: *mut mpv_handle,
        reply_userdata: u64,
        name: *const c_char,
        format: c_int,
    ) -> c_int;
    fn mpv_set_wakeup_callback(
        ctx: *mut mpv_handle,
        callback: Option<unsafe extern "C" fn(*mut c_void)>,
        data: *mut c_void,
    );

    fn mpv_render_context_create(
        res: *mut *mut mpv_render_context,
        mpv: *mut mpv_handle,
        params: *mut mpv_render_param,
    ) -> c_int;
    fn mpv_render_context_set_update_callback(
        ctx: *mut mpv_render_context,
        callback: Option<unsafe extern "C" fn(*mut c_void)>,
        callback_ctx: *mut c_void,
    );
    fn mpv_render_context_update(ctx: *mut mpv_render_context) -> u64;
    fn mpv_render_context_render(
        ctx: *mut mpv_render_context,
        params: *mut mpv_render_param,
    ) -> c_int;
    fn mpv_render_context_report_swap(ctx: *mut mpv_render_context);
    fn mpv_render_context_free(ctx: *mut mpv_render_context);
}

impl RenderUpdateContext {
    fn set_gl_area(&self, area: &gtk::GLArea) {
        *self.gl_area.lock().expect("render GLArea mutex poisoned") =
            Some(glib::SendWeakRef::from(area.downgrade()));
    }

    fn clear_gl_area(&self) {
        self.gl_area
            .lock()
            .expect("render GLArea mutex poisoned")
            .take();
    }

    fn gl_area(&self) -> Option<glib::SendWeakRef<gtk::GLArea>> {
        self.gl_area
            .lock()
            .expect("render GLArea mutex poisoned")
            .clone()
    }

    fn mark_wakeup_pending(&self) {
        self.wakeup_pending.store(true, Ordering::Release);
    }

    fn take_wakeup_pending(&self) -> bool {
        self.wakeup_pending.swap(false, Ordering::AcqRel)
    }

    fn mark_frame_available(&self) {
        self.frame_available.store(true, Ordering::Release);
    }

    fn reset_frame_available(&self) {
        self.frame_available.store(false, Ordering::Release);
    }

    fn has_frame_available(&self) -> bool {
        self.frame_available.load(Ordering::Acquire)
    }
}

impl LibMpv {
    pub fn new() -> Result<Self, String> {
        // SAFETY: `mpv_create` has no preconditions and returns either a valid handle or null.
        let handle = unsafe { mpv_create() };
        if handle.is_null() {
            return Err("Failed to create libmpv handle.".to_string());
        }

        let render_update_context = Box::leak(Box::new(RenderUpdateContext {
            main_context: glib::MainContext::default(),
            gl_area: Mutex::new(None),
            wakeup_pending: AtomicBool::new(false),
            frame_available: AtomicBool::new(false),
        }));
        let instance = Self {
            inner: Rc::new(LibMpvInner {
                handle,
                render_context: Mutex::new(ptr::null_mut()),
                render_update_context,
            }),
        };
        instance.set_option_string("terminal", "no")?;
        instance.set_option_string("keep-open", "yes")?;
        instance.set_option_string("input-default-bindings", "yes")?;
        instance.set_option_string("osc", "yes")?;
        instance.set_option_string("vo", "libmpv")?;

        // SAFETY: `instance.inner.handle` was created by `mpv_create`, is non-null here, and remains owned by `instance`.
        let status = unsafe { mpv_initialize(instance.inner.handle) };
        if status < 0 {
            return Err(instance.error_message(status));
        }

        instance.observe_playback_properties()?;
        let callback_ptr = instance.inner.render_update_context as *const RenderUpdateContext;
        // SAFETY: `instance.inner.handle` is a live mpv handle and `callback_ptr` points to
        // backend-owned callback state that remains valid for the backend lifetime.
        unsafe {
            mpv_set_wakeup_callback(
                instance.inner.handle,
                Some(queue_backend_update),
                callback_ptr.cast_mut().cast(),
            );
        }

        Ok(instance)
    }

    pub fn take_wakeup_pending(&self) -> bool {
        self.inner.render_update_context.take_wakeup_pending()
    }

    pub fn reset_frame_state(&self) {
        self.inner.render_update_context.reset_frame_available();
    }

    pub fn drain_pending_events(&self) -> Result<bool, String> {
        let mut needs_refresh = false;

        loop {
            // SAFETY: `self.inner.handle` is a live mpv handle and a zero timeout performs
            // a non-blocking poll of the client event queue.
            let event = unsafe { mpv_wait_event(self.inner.handle, 0.0) };
            if event.is_null() {
                return Err("libmpv returned a null event pointer.".to_string());
            }

            // SAFETY: libmpv guarantees the returned pointer stays valid until the next
            // `mpv_wait_event` call on this handle.
            let event = unsafe { &*event };
            if event.event_id == MPV_EVENT_NONE {
                break;
            }

            if event_requires_refresh(event.event_id, event.reply_userdata) {
                needs_refresh = true;
            }
        }

        Ok(needs_refresh)
    }

    pub fn initialize_render_context(&self, area: &gtk::GLArea) -> Result<(), String> {
        self.destroy_render_context();
        self.reset_frame_state();
        area.make_current();

        if let Some(error) = area.error() {
            return Err(format!("GTK GL context error: {error}"));
        }

        let mut render_context = ptr::null_mut();
        let init_params = mpv_opengl_init_params {
            get_proc_address: Some(resolve_gl_proc_address),
            get_proc_address_ctx: ptr::null_mut(),
        };
        let mut params = [
            mpv_render_param {
                type_: MPV_RENDER_PARAM_API_TYPE,
                data: MPV_RENDER_API_TYPE_OPENGL.as_ptr().cast_mut().cast(),
            },
            mpv_render_param {
                type_: MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: (&init_params as *const mpv_opengl_init_params)
                    .cast_mut()
                    .cast(),
            },
            mpv_render_param {
                type_: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];

        // SAFETY: `self.inner.handle` is a live mpv handle, `params` is a valid null-terminated parameter array,
        // and a current OpenGL context exists on this thread via `GLArea::make_current`.
        let status = unsafe {
            mpv_render_context_create(&mut render_context, self.inner.handle, params.as_mut_ptr())
        };
        if status < 0 {
            return Err(self.error_message(status));
        }

        self.inner.render_update_context.set_gl_area(area);
        let callback_ptr = self.inner.render_update_context as *const RenderUpdateContext;

        // SAFETY: `render_context` is a live render context, the callback does not call libmpv directly,
        // and `callback_ptr` points to a process-long callback context that remains valid for the backend lifetime.
        unsafe {
            mpv_render_context_set_update_callback(
                render_context,
                Some(queue_gl_area_render),
                callback_ptr.cast_mut().cast(),
            );
        }

        *self
            .inner
            .render_context
            .lock()
            .expect("render context mutex poisoned") = render_context;

        Ok(())
    }

    pub fn destroy_render_context(&self) {
        self.inner.render_update_context.clear_gl_area();
        self.reset_frame_state();

        let mut render_context = self
            .inner
            .render_context
            .lock()
            .expect("render context mutex poisoned");
        if render_context.is_null() {
            return;
        }

        // SAFETY: `*render_context` is owned by this wrapper; the callback is cleared before the context is freed.
        unsafe {
            mpv_render_context_set_update_callback(*render_context, None, ptr::null_mut());
            mpv_render_context_free(*render_context);
        }
        *render_context = ptr::null_mut();
    }

    pub fn render_to_gl_area(&self, area: &gtk::GLArea) -> Result<bool, String> {
        let Some((width, height)) =
            scaled_render_size(area.width(), area.height(), area.scale_factor())
        else {
            return Ok(false);
        };

        area.make_current();
        if let Some(error) = area.error() {
            return Err(format!("GTK GL context error: {error}"));
        }
        area.attach_buffers();

        let render_context = *self
            .inner
            .render_context
            .lock()
            .expect("render context mutex poisoned");
        if render_context.is_null() {
            return Ok(false);
        }

        // SAFETY: a current OpenGL context exists on this thread and `render_context` is valid here.
        let update_flags = unsafe { mpv_render_context_update(render_context) };
        if render_update_has_frame(update_flags) {
            self.inner.render_update_context.mark_frame_available();
        }

        let mut target = mpv_opengl_fbo {
            fbo: current_framebuffer_binding(),
            w: width,
            h: height,
            internal_format: 0,
        };
        let mut flip_y = 1;
        let mut params = [
            mpv_render_param {
                type_: MPV_RENDER_PARAM_OPENGL_FBO,
                data: (&mut target as *mut mpv_opengl_fbo).cast(),
            },
            mpv_render_param {
                type_: MPV_RENDER_PARAM_FLIP_Y,
                data: (&mut flip_y as *mut c_int).cast(),
            },
            mpv_render_param {
                type_: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];

        // SAFETY: `render_context` is live and `params` points to a valid, null-terminated render parameter array.
        let status = unsafe { mpv_render_context_render(render_context, params.as_mut_ptr()) };
        if status < 0 {
            return Err(self.error_message(status));
        }

        // SAFETY: `render_context` is valid and swap reporting is optional but safe after a successful render.
        unsafe { mpv_render_context_report_swap(render_context) };
        Ok(self.inner.render_update_context.has_frame_available())
    }

    pub fn load_file(&self, path: &Path) -> Result<(), String> {
        self.reset_frame_state();
        let path = path.to_string_lossy().into_owned();
        self.command(&["loadfile", &path, "replace"])
    }

    pub fn toggle_pause(&self) -> Result<(), String> {
        self.command(&["cycle", "pause"])
    }

    pub fn seek_relative(&self, seconds: f64) -> Result<(), String> {
        self.command(&["seek", &seconds.to_string(), "relative"])
    }

    pub fn set_volume(&self, volume: f64) -> Result<(), String> {
        self.set_property_double("volume", volume)
    }

    pub fn set_audio_track(&self, track_id: i64) -> Result<(), String> {
        self.set_property_i64("aid", track_id)
    }

    pub fn set_subtitle_track(&self, track_id: i64) -> Result<(), String> {
        self.set_property_i64("sid", track_id)
    }

    pub fn path(&self) -> Result<Option<String>, String> {
        match self.get_property_string("path") {
            Ok(path) if path.is_empty() => Ok(None),
            Ok(path) => Ok(Some(path)),
            Err(error) if error.contains("property unavailable") => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn paused(&self) -> Result<bool, String> {
        self.get_property_flag("pause")
    }

    pub fn position_seconds(&self) -> Result<f64, String> {
        self.get_property_double("time-pos")
    }

    pub fn duration_seconds(&self) -> Result<f64, String> {
        self.get_property_double("duration")
    }

    pub fn volume(&self) -> Result<f64, String> {
        self.get_property_double("volume")
    }

    pub fn current_audio_track(&self) -> Result<Option<i64>, String> {
        self.optional_track_id("aid")
    }

    pub fn current_subtitle_track(&self) -> Result<Option<i64>, String> {
        self.optional_track_id("sid")
    }

    pub fn tracks(&self) -> Result<(Vec<Track>, Vec<Track>), String> {
        let count = self.get_property_i64("track-list/count")?;
        let mut audio = Vec::new();
        let mut subtitles = Vec::new();

        for index in 0..count {
            let kind = match self
                .get_property_string(&format!("track-list/{index}/type"))
                .ok()
                .as_deref()
            {
                Some("audio") => TrackKind::Audio,
                Some("sub") => TrackKind::Subtitle,
                _ => continue,
            };

            let id = match self.get_property_i64(&format!("track-list/{index}/id")) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let title = self
                .get_property_string(&format!("track-list/{index}/title"))
                .ok()
                .filter(|value| !value.is_empty());
            let lang = self
                .get_property_string(&format!("track-list/{index}/lang"))
                .ok()
                .filter(|value| !value.is_empty());
            let selected = self
                .get_property_flag(&format!("track-list/{index}/selected"))
                .unwrap_or(false);

            let label = title.or(lang).unwrap_or_else(|| match kind {
                TrackKind::Audio => format!("音轨 {id}"),
                TrackKind::Subtitle => format!("字幕 {id}"),
            });

            let track = Track {
                id,
                label,
                kind,
                selected,
            };

            match kind {
                TrackKind::Audio => audio.push(track),
                TrackKind::Subtitle => subtitles.push(track),
            }
        }

        Ok((audio, subtitles))
    }

    pub fn seek_absolute(&self, seconds: f64) -> Result<(), String> {
        self.command(&["seek", &seconds.to_string(), "absolute"])
    }

    pub fn set_speed(&self, speed: f64) -> Result<(), String> {
        self.set_property_double("speed", speed)
    }

    pub fn speed(&self) -> Result<f64, String> {
        self.get_property_double("speed")
    }

    pub fn set_mute(&self, mute: bool) -> Result<(), String> {
        let value = if mute { "yes" } else { "no" };
        self.command(&["set", "mute", value])
    }

    pub fn muted(&self) -> Result<bool, String> {
        self.get_property_flag("mute")
    }

    pub fn set_subtitle_delay(&self, seconds: f64) -> Result<(), String> {
        self.set_property_double("sub-delay", seconds)
    }

    pub fn subtitle_delay(&self) -> Result<f64, String> {
        self.get_property_double("sub-delay")
    }

    pub fn set_audio_delay(&self, seconds: f64) -> Result<(), String> {
        self.set_property_double("audio-delay", seconds)
    }

    pub fn audio_delay(&self) -> Result<f64, String> {
        self.get_property_double("audio-delay")
    }

    pub fn screenshot(&self) -> Result<(), String> {
        self.command(&["screenshot", "video"])
    }

    pub fn stop(&self) -> Result<(), String> {
        self.command(&["stop"])
    }

    pub fn load_subtitle(&self, path: &Path) -> Result<(), String> {
        let path = path.to_string_lossy().into_owned();
        self.command(&["sub-add", &path])
    }

    pub fn media_title(&self) -> Result<Option<String>, String> {
        match self.get_property_string("media-title") {
            Ok(title) if title.is_empty() => Ok(None),
            Ok(title) => Ok(Some(title)),
            Err(error) if error.contains("property unavailable") => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn video_codec(&self) -> Result<Option<String>, String> {
        match self.get_property_string("video-codec") {
            Ok(codec) if codec.is_empty() => Ok(None),
            Ok(codec) => Ok(Some(codec)),
            Err(_) => Ok(None),
        }
    }

    pub fn audio_codec(&self) -> Result<Option<String>, String> {
        match self.get_property_string("audio-codec-name") {
            Ok(codec) if codec.is_empty() => Ok(None),
            Ok(codec) => Ok(Some(codec)),
            Err(_) => Ok(None),
        }
    }

    pub fn video_width(&self) -> Result<Option<i64>, String> {
        match self.get_property_i64("width") {
            Ok(w) if w > 0 => Ok(Some(w)),
            Ok(_) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    pub fn video_height(&self) -> Result<Option<i64>, String> {
        match self.get_property_i64("height") {
            Ok(h) if h > 0 => Ok(Some(h)),
            Ok(_) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    pub fn set_ab_loop_a(&self, pos: Option<f64>) -> Result<(), String> {
        match pos {
            Some(p) => self.set_property_double("ab-loop-a", p),
            None => self.command(&["set", "ab-loop-a", "no"]),
        }
    }

    pub fn set_ab_loop_b(&self, pos: Option<f64>) -> Result<(), String> {
        match pos {
            Some(p) => self.set_property_double("ab-loop-b", p),
            None => self.command(&["set", "ab-loop-b", "no"]),
        }
    }

    fn observe_playback_properties(&self) -> Result<(), String> {
        for &(reply_userdata, name, format) in OBSERVED_PLAYBACK_PROPERTIES {
            let name = CString::new(name)
                .map_err(|_| format!("Invalid observed property name: {name}"))?;
            // SAFETY: `self.inner.handle` is valid and `name` lives for the duration of the
            // observation registration call.
            let status = unsafe {
                mpv_observe_property(self.inner.handle, reply_userdata, name.as_ptr(), format)
            };
            if status < 0 {
                return Err(self.error_message(status));
            }
        }

        Ok(())
    }

    fn command(&self, parts: &[&str]) -> Result<(), String> {
        let cstrings = parts
            .iter()
            .map(|part| {
                CString::new(*part).map_err(|_| format!("Invalid libmpv command part: {part}"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut raw_parts = cstrings
            .iter()
            .map(|value| value.as_ptr())
            .collect::<Vec<_>>();
        raw_parts.push(ptr::null());

        // SAFETY: `self.inner.handle` is a live libmpv handle and `raw_parts` is null-terminated with pointers valid for this call.
        let status = unsafe { mpv_command(self.inner.handle, raw_parts.as_ptr()) };
        if status < 0 {
            return Err(self.error_message(status));
        }

        Ok(())
    }

    fn set_option_string(&self, name: &str, value: &str) -> Result<(), String> {
        let name = CString::new(name).map_err(|_| format!("Invalid option name: {name}"))?;
        let value = CString::new(value).map_err(|_| format!("Invalid option value: {value}"))?;

        // SAFETY: `self.inner.handle` is valid and both C strings are owned locally and live through the FFI call.
        let status =
            unsafe { mpv_set_option_string(self.inner.handle, name.as_ptr(), value.as_ptr()) };
        if status < 0 {
            return Err(self.error_message(status));
        }

        Ok(())
    }

    fn get_property_string(&self, name: &str) -> Result<String, String> {
        let name = CString::new(name).map_err(|_| format!("Invalid property name: {name}"))?;
        let mut value_ptr: *mut c_char = ptr::null_mut();

        // SAFETY: `self.inner.handle` is valid, `name` is a valid C string, and libmpv writes a heap-allocated string pointer into `value_ptr`.
        let status = unsafe {
            mpv_get_property(
                self.inner.handle,
                name.as_ptr(),
                MPV_FORMAT_STRING,
                (&mut value_ptr as *mut *mut c_char).cast(),
            )
        };
        if status < 0 {
            return Err(self.error_message(status));
        }
        if value_ptr.is_null() {
            return Ok(String::new());
        }

        // SAFETY: `value_ptr` came from libmpv as a valid null-terminated string and remains valid until freed below.
        let value = unsafe { CStr::from_ptr(value_ptr) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: `value_ptr` was allocated by libmpv for this property read and must be released with `mpv_free` exactly once.
        unsafe { mpv_free(value_ptr.cast()) };
        Ok(value)
    }

    fn get_property_flag(&self, name: &str) -> Result<bool, String> {
        let mut value: c_int = 0;
        self.get_property_scalar(name, MPV_FORMAT_FLAG, (&mut value as *mut c_int).cast())?;
        Ok(value != 0)
    }

    fn get_property_i64(&self, name: &str) -> Result<i64, String> {
        let mut value: c_longlong = 0;
        self.get_property_scalar(
            name,
            MPV_FORMAT_INT64,
            (&mut value as *mut c_longlong).cast(),
        )?;
        Ok(value)
    }

    fn get_property_double(&self, name: &str) -> Result<f64, String> {
        let mut value: c_double = 0.0;
        self.get_property_scalar(
            name,
            MPV_FORMAT_DOUBLE,
            (&mut value as *mut c_double).cast(),
        )?;
        Ok(value)
    }

    fn set_property_i64(&self, name: &str, value: i64) -> Result<(), String> {
        let mut value: c_longlong = value;
        self.set_property_scalar(
            name,
            MPV_FORMAT_INT64,
            (&mut value as *mut c_longlong).cast(),
        )
    }

    fn set_property_double(&self, name: &str, value: f64) -> Result<(), String> {
        let mut value: c_double = value;
        self.set_property_scalar(
            name,
            MPV_FORMAT_DOUBLE,
            (&mut value as *mut c_double).cast(),
        )
    }

    fn get_property_scalar(
        &self,
        name: &str,
        format: c_int,
        data: *mut c_void,
    ) -> Result<(), String> {
        let name = CString::new(name).map_err(|_| format!("Invalid property name: {name}"))?;
        // SAFETY: `self.inner.handle` is valid, `name` is a valid C string, and `data` points to writable storage matching `format`.
        let status = unsafe { mpv_get_property(self.inner.handle, name.as_ptr(), format, data) };
        if status < 0 {
            return Err(self.error_message(status));
        }
        Ok(())
    }

    fn set_property_scalar(
        &self,
        name: &str,
        format: c_int,
        data: *mut c_void,
    ) -> Result<(), String> {
        let name = CString::new(name).map_err(|_| format!("Invalid property name: {name}"))?;
        // SAFETY: `self.inner.handle` is valid, `name` is a valid C string, and `data` points to initialized storage matching `format`.
        let status = unsafe { mpv_set_property(self.inner.handle, name.as_ptr(), format, data) };
        if status < 0 {
            return Err(self.error_message(status));
        }
        Ok(())
    }

    fn optional_track_id(&self, property: &str) -> Result<Option<i64>, String> {
        match self.get_property_i64(property) {
            Ok(value) if value >= 0 => Ok(Some(value)),
            Ok(_) => Ok(None),
            Err(error) if error.contains("property unavailable") => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn error_message(&self, status: c_int) -> String {
        // SAFETY: `mpv_error_string` returns either null or a pointer to a static null-terminated error string for `status`.
        let message = unsafe { mpv_error_string(status) };
        if message.is_null() {
            return format!("libmpv error code {status}");
        }

        // SAFETY: `message` is non-null here and points to a valid static null-terminated string from libmpv.
        let text = unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned();
        format!("libmpv error ({status}): {text}")
    }
}

impl Drop for LibMpvInner {
    fn drop(&mut self) {
        self.render_update_context.clear_gl_area();

        let render_context = self
            .render_context
            .get_mut()
            .expect("render context mutex poisoned during drop");
        if !render_context.is_null() {
            // SAFETY: the callback is being cleared before the owned render context is freed during final drop.
            unsafe {
                mpv_render_context_set_update_callback(*render_context, None, ptr::null_mut());
                mpv_render_context_free(*render_context);
            }
            *render_context = ptr::null_mut();
        }

        if !self.handle.is_null() {
            // SAFETY: `self.handle` is owned by this wrapper, still valid here, and its
            // wakeup callback must be cleared before final destruction.
            unsafe {
                mpv_set_wakeup_callback(self.handle, None, ptr::null_mut());
                mpv_terminate_destroy(self.handle);
            };
            self.handle = ptr::null_mut();
        }
    }
}

fn observed_property_reply_ids() -> [u64; OBSERVED_PLAYBACK_PROPERTIES.len()] {
    [
        OBSERVED_PATH_PROPERTY,
        OBSERVED_PAUSE_PROPERTY,
        OBSERVED_POSITION_PROPERTY,
        OBSERVED_DURATION_PROPERTY,
        OBSERVED_VOLUME_PROPERTY,
        OBSERVED_AUDIO_TRACK_PROPERTY,
        OBSERVED_SUBTITLE_TRACK_PROPERTY,
        OBSERVED_TRACK_COUNT_PROPERTY,
    ]
}

fn is_observed_property_reply(reply_userdata: u64) -> bool {
    observed_property_reply_ids().contains(&reply_userdata)
}

fn event_requires_refresh(event_id: c_int, reply_userdata: u64) -> bool {
    match event_id {
        MPV_EVENT_SHUTDOWN
        | MPV_EVENT_END_FILE
        | MPV_EVENT_FILE_LOADED
        | MPV_EVENT_PLAYBACK_RESTART => true,
        MPV_EVENT_PROPERTY_CHANGE => is_observed_property_reply(reply_userdata),
        _ => false,
    }
}

unsafe extern "C" fn queue_gl_area_render(callback_context: *mut c_void) {
    if callback_context.is_null() {
        return;
    }

    // SAFETY: `callback_context` points to the backend-owned render update context,
    // which remains allocated for the entire backend lifetime.
    let callback_context = unsafe { &*(callback_context as *const RenderUpdateContext) };
    let main_context = callback_context.main_context.clone();
    let gl_area = callback_context.gl_area();

    main_context.invoke(move || {
        if let Some(gl_area) = gl_area.and_then(|gl_area| gl_area.upgrade()) {
            gl_area.queue_render();
        }
    });
}

unsafe extern "C" fn queue_backend_update(callback_context: *mut c_void) {
    if callback_context.is_null() {
        return;
    }

    // SAFETY: `callback_context` points to the backend-owned render update context,
    // which remains allocated for the entire backend lifetime.
    let callback_context = unsafe { &*(callback_context as *const RenderUpdateContext) };
    callback_context.mark_wakeup_pending();
}

unsafe extern "C" fn resolve_gl_proc_address(_: *mut c_void, name: *const c_char) -> *mut c_void {
    if name.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: `RTLD_DEFAULT` searches already-loaded GL libraries in the current process.
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, name) }
}

fn render_update_has_frame(update_flags: u64) -> bool {
    update_flags & MPV_RENDER_UPDATE_FRAME != 0
}

fn current_framebuffer_binding() -> c_int {
    let mut framebuffer = 0;
    // SAFETY: a current desktop OpenGL context exists on this thread before calling this helper,
    // and `framebuffer` points to writable storage for the queried integer value.
    unsafe { glGetIntegerv(GL_FRAMEBUFFER_BINDING, &mut framebuffer) };
    framebuffer
}

fn scaled_render_size(width: i32, height: i32, scale_factor: i32) -> Option<(i32, i32)> {
    if width <= 0 || height <= 0 || scale_factor <= 0 {
        return None;
    }

    Some((
        width.saturating_mul(scale_factor),
        height.saturating_mul(scale_factor),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        MPV_EVENT_END_FILE, MPV_EVENT_FILE_LOADED, MPV_EVENT_NONE, MPV_EVENT_PLAYBACK_RESTART,
        MPV_EVENT_PROPERTY_CHANGE, MPV_RENDER_UPDATE_FRAME, OBSERVED_AUDIO_TRACK_PROPERTY,
        OBSERVED_DURATION_PROPERTY, OBSERVED_PATH_PROPERTY, OBSERVED_PAUSE_PROPERTY,
        OBSERVED_POSITION_PROPERTY, OBSERVED_SUBTITLE_TRACK_PROPERTY,
        OBSERVED_TRACK_COUNT_PROPERTY, OBSERVED_VOLUME_PROPERTY, event_requires_refresh,
        observed_property_reply_ids, render_update_has_frame, scaled_render_size,
    };

    #[test]
    fn scaled_render_size_returns_none_for_non_positive_values() {
        assert_eq!(scaled_render_size(0, 720, 1), None);
        assert_eq!(scaled_render_size(1280, -1, 1), None);
        assert_eq!(scaled_render_size(1280, 720, 0), None);
    }

    #[test]
    fn scaled_render_size_applies_scale_factor() {
        assert_eq!(scaled_render_size(640, 360, 2), Some((1280, 720)));
    }

    #[test]
    fn observed_property_reply_ids_cover_playback_state_sync() {
        assert_eq!(
            observed_property_reply_ids(),
            [
                OBSERVED_PATH_PROPERTY,
                OBSERVED_PAUSE_PROPERTY,
                OBSERVED_POSITION_PROPERTY,
                OBSERVED_DURATION_PROPERTY,
                OBSERVED_VOLUME_PROPERTY,
                OBSERVED_AUDIO_TRACK_PROPERTY,
                OBSERVED_SUBTITLE_TRACK_PROPERTY,
                OBSERVED_TRACK_COUNT_PROPERTY,
            ]
        );
    }

    #[test]
    fn event_requires_refresh_for_state_driving_events() {
        assert!(event_requires_refresh(MPV_EVENT_FILE_LOADED, 0));
        assert!(event_requires_refresh(MPV_EVENT_END_FILE, 0));
        assert!(event_requires_refresh(MPV_EVENT_PLAYBACK_RESTART, 0));
        assert!(event_requires_refresh(
            MPV_EVENT_PROPERTY_CHANGE,
            OBSERVED_PAUSE_PROPERTY,
        ));
        assert!(!event_requires_refresh(MPV_EVENT_PROPERTY_CHANGE, 999));
        assert!(!event_requires_refresh(MPV_EVENT_NONE, 0));
    }

    #[test]
    fn render_update_has_frame_only_for_frame_bit() {
        assert!(render_update_has_frame(MPV_RENDER_UPDATE_FRAME));
        assert!(render_update_has_frame(MPV_RENDER_UPDATE_FRAME | 2));
        assert!(!render_update_has_frame(0));
        assert!(!render_update_has_frame(2));
    }
}
