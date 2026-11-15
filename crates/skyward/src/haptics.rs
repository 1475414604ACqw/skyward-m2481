use crate::waveform::{made, share, ticks, RATE, TICKS_RATE};
use anyhow::{anyhow, ensure, Context, Result};
use evdev::raw_stream::RawDevice;
use skyward_config::Strength;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// linux/input-event-codes.h
const EV_FF: u16 = 0x15;
const FF_PERIODIC: u16 = 0x51;
const FF_CUSTOM: u16 = 0x5d;

/// The ticks play at this share of the drive the driver gives samples at in
/// full, whatever the slider says: the 17 Ultra's own system plays them at
/// 3527 mV of the 8500 mV its device tree gives.
const TICKS_STRENGTH: f32 = 0.415;
/// How long the ticks stay loaded once their samples have run out, as the
/// phone's own system leaves them: the motor comes to rest on its own
/// before taking them away brakes it.
const TICKS_REST: Duration = Duration::from_millis(40);
/// How long a click stays loaded after its samples have run out, so that a
/// busy recovery's driver still gets to play it before it is taken away.
const CLICK_REST: Duration = Duration::from_millis(120);

/// The vibration motor, played through the kernel's force-feedback
/// events. The driver fills its queue with a made click's samples as the
/// click is loaded, refilling it for no second play, and keeps the click
/// playing after its samples have run out, even when told to stop, until
/// the click is taken away. So each click is loaded as it is played, at the
/// strength the slider gives, and taken away once it has played out.
pub struct Haptics {
    motor: Motor,
    strength: Strength,
}

/// Where the motor is played: through the driver's force-feedback events,
/// or through the LED interface some drivers answer with and theirs does
/// not.
enum Motor {
    Force {
        dev: File,
        click: Click,
        ticks: Click,
        /// The click playing, by its slot and its count, until it is taken
        /// away; a click taken away early by the next is not taken away again.
        playing: Arc<Mutex<Option<(i16, u64)>>>,
        clicks: u64,
    },
    /// The folder the driver keeps `duration` and `activate` in.
    Led(PathBuf),
}

/// What a click is.
enum Click {
    /// A waveform the driver keeps, by its number. The driver says how long
    /// it plays as it loads it.
    Kept(i16),
    /// Samples made at the motor's resonant frequency, which the driver
    /// plays as they are, at `rate` a second.
    Samples { data: Vec<i8>, rate: u32 },
}

impl Haptics {
    /// The force-feedback device and the configured click. Loading a click
    /// fills the driver's queue with what it plays, so the click is loaded
    /// only as it is played.
    pub fn open(cfg: &skyward_config::Haptics) -> Result<Haptics> {
        if let Some(led) = &cfg.led {
            let dir = PathBuf::from(led);
            ensure!(dir.join("duration").exists(), "{} holds no duration", dir.display());
            eprintln!("haptics {}: clicks written to its duration and activate", dir.display());
            return Ok(Haptics { motor: Motor::Led(dir), strength: cfg.strength });
        }
        let (path, dev) = motor(cfg.device.as_deref())?;
        let hz = resonance(&cfg.resonance)?;
        let click = match cfg.click {
            Some(id) => {
                eprintln!("haptics {}: clicks with waveform {id}, ticks made for a motor swinging at {hz} Hz", path.display());
                Click::Kept(id)
            }
            None => {
                eprintln!("haptics {}: clicks and ticks made for a motor swinging at {hz} Hz", path.display());
                Click::Samples { data: made(hz), rate: RATE }
            }
        };
        let ticks = Click::Samples { data: ticks(hz), rate: TICKS_RATE };
        Ok(Haptics {
            motor: Motor::Force { dev, click, ticks, playing: Arc::new(Mutex::new(None)), clicks: 0 },
            strength: cfg.strength,
        })
    }

    /// One short click at the strength the slider's `position` gives, after
    /// taking away one still playing. Gives how long it plays.
    pub fn click(&mut self, position: f32) -> Result<Duration> {
        let share = share(&self.strength, position);
        match &mut self.motor {
            // The driver takes whole milliseconds, and a click of one is not
            // felt: the lightest click is a few tens of them.
            Motor::Led(dir) => pulse(dir, 22 + (share.clamp(0.0, 1.0) * 26.0) as u64),
            Motor::Force { .. } => self.start(false, share),
        }
    }

    /// The ticks of the screen going off, as strong as the phone's own
    /// system plays them whatever the slider says. Gives how long they play.
    pub fn ticks(&mut self) -> Result<Duration> {
        match &mut self.motor {
            Motor::Led(dir) => {
                let first = pulse(dir, 45)?;
                std::thread::sleep(Duration::from_millis(55));
                let second = pulse(dir, 30)?;
                Ok(first + Duration::from_millis(55) + second)
            }
            Motor::Force { .. } => self.start(true, TICKS_STRENGTH),
        }
    }

    /// Play the ticks, or else the click, at `share` of its full strength,
    /// after taking away what still plays.
    fn start(&mut self, ticks: bool, share: f32) -> Result<Duration> {
        let Motor::Force { dev, click, ticks: tick_samples, playing, clicks } = &mut self.motor else {
            anyhow::bail!("the motor is played through its LED interface");
        };
        {
            let mut held = playing.lock().expect("no thread panics while it holds the click playing");
            if let Some((slot, _)) = held.take() {
                erase(dev, slot).context("take away the click still playing")?;
            }
            let effect = if ticks { &*tick_samples } else { &*click };
            let loaded = upload(dev, effect, share).context("load the click")?;
            play(dev, loaded.slot).context("play the click")?;
            *clicks += 1;
            let this = (loaded.slot, *clicks);
            *held = Some(this);
            let length = loaded.length + if ticks { TICKS_REST } else { CLICK_REST };
            // Timed from now: a thread may wait a while before it first runs.
            let until = Instant::now() + length;
            let dev = dev.try_clone().context("share the motor")?;
            let playing = playing.clone();
            std::thread::spawn(move || {
                std::thread::sleep(until.saturating_duration_since(Instant::now()));
                let mut held = playing.lock().expect("no thread panics while it holds the click playing");
                if *held == Some(this) {
                    *held = None;
                    if let Err(e) = erase(&dev, this.0) {
                        eprintln!("take away the click: {e}");
                    }
                }
            });
            Ok(length)
        }
    }
}

/// Play the configured click once at the slider's `position`, or else the
/// ticks of the screen going off, saying how strongly and for how long, for
/// trying out a motor.
pub fn probe(cfg: &skyward_config::Haptics, position: f32, ticks: bool) -> Result<()> {
    let mut haptics = Haptics::open(cfg)?;
    let (what, strength, length) = if ticks {
        ("ticks", TICKS_STRENGTH, haptics.ticks()?)
    } else {
        ("click", share(&cfg.strength, position), haptics.click(position)?)
    };
    // Once it has played out, what played goes with the device as the
    // command ends.
    std::thread::sleep(length);
    println!("played the {what} at {strength:.3} of the full strength, {} ms long", length.as_millis());
    Ok(())
}

/// Play the motor through its LED interface for `ms` milliseconds.
fn pulse(dir: &Path, ms: u64) -> Result<Duration> {
    let duration = dir.join("duration");
    fs::write(&duration, format!("{ms}")).with_context(|| format!("write {ms} to {}", duration.display()))?;
    let activate = dir.join("activate");
    fs::write(&activate, "1").with_context(|| format!("start the motor through {}", activate.display()))?;
    Ok(Duration::from_millis(ms))
}
/// The motor's input device, opened for writing: the one the configuration
/// names, by its path or by the driver's name for it, or the first under
/// /dev/input that takes periodic force-feedback effects.
fn motor(named: Option<&str>) -> Result<(PathBuf, File)> {
    for (path, dev) in evdev::raw_stream::enumerate() {
        if !takes_periodic(&dev) {
            continue;
        }
        let asked = match named {
            Some(want) if want.starts_with('/') => path.to_string_lossy() == want,
            Some(want) => dev.name().is_some_and(|name| name == want),
            None => true,
        };
        if !asked {
            continue;
        }
        let file = OpenOptions::new().read(true).write(true).open(&path).with_context(|| format!("open {}", path.display()))?;
        return Ok((path, file));
    }
    Err(anyhow!("no input device takes force-feedback effects"))
}

fn takes_periodic(dev: &RawDevice) -> bool {
    dev.supported_ff().is_some_and(|ff| ff.contains(evdev::FFEffectCode::FF_PERIODIC))
}

/// The motor's resonant frequency in hertz, from the file the driver says
/// it in, in tenths of a hertz.
fn resonance(file: &str) -> Result<f32> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        // The bus a haptics chip sits on is numbered as the driver probes: the
        // file moves with it, so it is looked for where it landed.
        Err(e) => match find_resonance() {
            Some(text) => text,
            None => return Err(e).with_context(|| format!("read {file}")),
        },
    };
    // Some drivers write the number with its label about it, as `f0 = 1245`.
    let tenths: u32 = text
        .split_whitespace()
        .rev()
        .find_map(|n| n.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok())
        .with_context(|| format!("{file} holds {text:?}"))?;
    ensure!(tenths > 0, "{file} says the motor swings at 0 Hz");
    Ok(tenths as f32 / 10.0)
}

/// What the haptics driver says its motor swings at, from wherever the
/// driver's own device landed under /sys/devices.
fn find_resonance() -> Option<String> {
    // The file found once is read directly next time: walking /sys/devices
    // takes long enough to be felt.
    static FOUND: std::sync::OnceLock<std::sync::Mutex<Option<PathBuf>>> = std::sync::OnceLock::new();
    let found = FOUND.get_or_init(|| std::sync::Mutex::new(None));
    if let Some(path) = found.lock().ok().and_then(|g| g.clone()) {
        if let Ok(text) = fs::read_to_string(&path) {
            return Some(text);
        }
    }
    let path = walk_for_resonance()?;
    let text = fs::read_to_string(&path).ok()?;
    if let Ok(mut g) = found.lock() {
        *g = Some(path);
    }
    Some(text)
}

/// Where the haptics driver's own device landed: the file it says its
/// motor's resonant frequency in.
fn walk_for_resonance() -> Option<PathBuf> {
    fn walk(dir: &Path, depth: u32) -> Option<PathBuf> {
        if depth == 0 {
            return None;
        }
        for entry in fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if entry.file_name().to_string_lossy().starts_with("vibrator") {
                for file in ["lra_f0", "f0_cali"] {
                    let at = path.join(file);
                    if at.exists() {
                        return Some(at);
                    }
                }
            }
            if let Some(found) = walk(&path, depth - 1) {
                return Some(found);
            }
        }
        None
    }
    walk(Path::new("/sys/devices"), 9)
}

/// A click as the driver loaded it: the slot the kernel keeps it in, and
/// how long it plays.
struct Loaded {
    slot: i16,
    length: Duration,
}

/// Samples handed to the driver to play as they are, as its
/// `custom_fifo_data` lays them out.
#[repr(C)]
struct Samples {
    /// Unused by the driver.
    index: u32,
    length: u32,
    rate: u32,
    data: *const i8,
}

/// Load `click` at `share` of its full strength as a custom periodic
/// effect in a slot of its own. The driver drives the motor at the click's
/// highest voltage times the magnitude.
fn upload(dev: &File, click: &Click, share: f32) -> Result<Loaded> {
    // The driver tells the two kinds apart by the length in bytes of what
    // the effect points to, and loads nothing for any other length, though
    // the upload still succeeds. For a waveform it keeps it reads the
    // waveform's number from the first of three values and writes how long
    // it plays into the other two, in whole seconds and then milliseconds.
    let mut kept: [i16; 3] = [0; 3];
    let mut samples = Samples { index: 0, length: 0, rate: RATE, data: std::ptr::null() };
    let (len, data) = match click {
        Click::Kept(id) => {
            kept[0] = *id;
            (std::mem::size_of_val(&kept), kept.as_mut_ptr())
        }
        Click::Samples { data, rate } => {
            samples.length = u32::try_from(data.len())?;
            samples.rate = *rate;
            samples.data = data.as_ptr();
            (std::mem::size_of::<Samples>(), (&mut samples as *mut Samples).cast::<i16>())
        }
    };
    let periodic = libc::ff_periodic_effect {
        waveform: FF_CUSTOM,
        period: 0,
        magnitude: (share.clamp(0.0, 1.0) * f32::from(i16::MAX)).round() as i16,
        offset: 0,
        phase: 0,
        envelope: libc::ff_envelope { attack_length: 0, attack_level: 0, fade_length: 0, fade_level: 0 },
        custom_len: len as u32,
        custom_data: data,
    };
    let mut effect = libc::ff_effect {
        type_: FF_PERIODIC,
        id: -1,
        direction: 0,
        trigger: libc::ff_trigger { button: 0, interval: 0 },
        replay: libc::ff_replay { length: 0, delay: 0 },
        u: [0; 4],
    };
    // The union holds the periodic effect; it is exactly as large.
    const _: () = assert!(std::mem::size_of::<libc::ff_periodic_effect>() == std::mem::size_of::<[u64; 4]>());
    effect.u = unsafe { std::mem::transmute::<libc::ff_periodic_effect, [u64; 4]>(periodic) };
    // EVIOCSFF: _IOW('E', 0x80, struct ff_effect)
    let size = std::mem::size_of::<libc::ff_effect>() as u32;
    let request = (1u32 << 30) | (size << 16) | (u32::from(b'E') << 8) | 0x80;
    if unsafe { libc::ioctl(dev.as_raw_fd(), request as _, &mut effect) } < 0 {
        return Err(io::Error::last_os_error().into());
    }
    let length = match click {
        Click::Kept(id) => {
            let [_, seconds, millis] = kept;
            let length = Duration::from_secs(u64::try_from(seconds)?) + Duration::from_millis(u64::try_from(millis)?);
            ensure!(!length.is_zero(), "the driver loaded waveform {id} without saying how long it plays");
            length
        }
        // The queue is full before the samples start playing.
        Click::Samples { data, rate } => Duration::from_secs_f64(data.len() as f64 / f64::from(*rate)),
    };
    Ok(Loaded { slot: effect.id, length })
}

/// Start the loaded effect in slot `id`.
fn play(dev: &mut File, id: i16) -> io::Result<()> {
    let event = libc::input_event { time: libc::timeval { tv_sec: 0, tv_usec: 0 }, type_: EV_FF, code: id as u16, value: 1 };
    let bytes = unsafe { std::slice::from_raw_parts((&event as *const libc::input_event).cast::<u8>(), std::mem::size_of::<libc::input_event>()) };
    dev.write_all(bytes)
}

/// Take the effect in slot `id` away, which stops it for good.
fn erase(dev: &File, id: i16) -> io::Result<()> {
    // EVIOCRMFF: _IOW('E', 0x81, int)
    let request = (1u32 << 30) | ((std::mem::size_of::<libc::c_int>() as u32) << 16) | (u32::from(b'E') << 8) | 0x81;
    if unsafe { libc::ioctl(dev.as_raw_fd(), request as _, libc::c_int::from(id)) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
