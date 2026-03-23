use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time;

use log::*;
use signal_hook::iterator::Signals;
use signal_hook::consts::{SIGINT, SIGTERM};
use dbus::blocking::Connection;
use dbus::{Message, arg};

#[path = "../comms.rs"]
mod comms;
mod config;
mod kbd;
mod device;
mod dbus_generated;

use kbd::{Effect, EffectManager};
use device::DeviceManager;
use dbus_generated::*;

static EFFECT_MANAGER: LazyLock<Mutex<EffectManager>> = LazyLock::new(init_effect_manager);
static DEV_MANAGER: LazyLock<Mutex<DeviceManager>> = LazyLock::new(init_dev_manager);

fn init_effect_manager() -> Mutex<EffectManager> {
    Mutex::new(EffectManager::new())
}

fn init_dev_manager() -> Mutex<DeviceManager> {
    match DeviceManager::read_laptops_file() {
        Ok(c) => Mutex::new(c),
        Err(_) => Mutex::new(device::DeviceManager::new()),
    }
}

/// Returns a locked MutexGuard with the effect manager
fn effect_manager() -> MutexGuard<'static, EffectManager> {
    EFFECT_MANAGER.lock()
        .expect("code should not panic while holding the effect manager guard")
}

/// Returns a locked MutexGuard with the device manager
fn dev_manager() -> MutexGuard<'static, DeviceManager> {
    DEV_MANAGER.lock()
        .expect("code should not panic while holding the device manager guard")
}

// Main function for daemon
fn main() {
    setup_panic_hook();
    init_logging();

    {
        let mut d = dev_manager();
        d.discover_devices();
        if let Some(laptop) = d.get_device() {
            println!("supported device: {:?}", laptop.get_name());
        } else {
            println!("no supported device found");
            std::process::exit(1);
        }
    }

    {
        let mut d = dev_manager();
        let dbus_system = Connection::new_system()
            .expect("failed to connect to D-Bus system bus");
        let proxy_ac = dbus_system.with_proxy("org.freedesktop.UPower", "/org/freedesktop/UPower/devices/line_power_AC0", time::Duration::from_millis(5000));
        use battery::OrgFreedesktopUPowerDevice;
        if let Ok(online) = proxy_ac.online() {
            info!("AC0 online: {:?}", online);
            d.set_ac_state(online);
            d.restore_standard_effect();
            if let Ok(json) = config::Configuration::read_effects_file() {
                effect_manager().load_from_save(json);
            } else {
                println!("No effects save, creating a new one");
                // No effects found, start with a green static layer, just like synapse
                effect_manager().push_effect(
                    kbd::effects::Static::new(vec![0, 255, 0]), 
                    [true; 90]
                    );
            }
        } else {
            println!("error getting current power state");
            std::process::exit(1);
        }
    }

    start_keyboard_animator_task();
    start_screensaver_monitor_task();
    start_battery_monitor_task();
    let clean_thread = start_shutdown_task();

    if let Some(listener) = comms::create() {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_data(stream),
                Err(_) => {} // Don't care about this
            }
        }
    } else {
        eprintln!("Could not create Unix socket!");
        std::process::exit(1);
    }
    clean_thread.join().unwrap();
}

/// Installs a custom panic hook to perform cleanup when the daemon crashes
fn setup_panic_hook() {
    let default_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        error!("Something went wrong! Removing the socket path");
        if std::fs::metadata(comms::SOCKET_PATH).is_ok() {
            std::fs::remove_file(comms::SOCKET_PATH).unwrap();
        }
        default_panic_hook(info);
    }));
}

fn init_logging() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.target(env_logger::Target::Stderr);
    builder.filter_level(log::LevelFilter::Info);
    builder.format_timestamp_millis();
    builder.parse_env("RAZER_LAPTOP_CONTROL_LOG");
    builder.init();
}

/// Handles keyboard animations
pub fn start_keyboard_animator_task() -> JoinHandle<()> {
    // Start the keyboard animator thread,
    thread::spawn(|| {
        loop {
            let light_control_enabled = dev_manager().get_light_control();

            if light_control_enabled {
                if let Some(laptop) = dev_manager().get_device() {
                    effect_manager().update(laptop);
                }
            }

            thread::sleep(std::time::Duration::from_millis(kbd::ANIMATION_SLEEP_MS));
        }
    })
}

fn start_screensaver_monitor_task() -> JoinHandle<()> {
    thread::spawn(move || {
        let dbus_session = Connection::new_session()
            .expect("failed to connect to D-Bus session bus");
        let  proxy = dbus_session.with_proxy("org.gnome.Mutter.DisplayConfig", "/org/gnome/Mutter/DisplayConfig", time::Duration::from_millis(5000));
        let _id = proxy.match_signal(|h: dbus_mutter_displayconfig::OrgFreedesktopDBusPropertiesPropertiesChanged, _: &Connection, _: &Message| {
            let online: Option<&i32> = arg::prop_cast(&h.changed_properties, "PowerSaveMode");
            if let Some(online) = online {
                if *online == 3 {
                    dev_manager().light_off();
                }
                else if *online == 0 {
                    dev_manager().restore_light();
                }

            } 
            true
        });
        let  proxy_idle = dbus_session.with_proxy("org.gnome.Mutter.IdleMonitor", "/org/gnome/Mutter/IdleMonitor/Core", time::Duration::from_millis(5000));
        let _id = proxy_idle.match_signal(|h: dbus_mutter_idlemonitor::OrgGnomeMutterIdleMonitorWatchFired, _: &Connection, _: &Message| {
            {
                let mut d = dev_manager();
                if d.idle_id == h.id {
                    println!("idle trigger {:?}", h.id);
                    d.light_off();
                } else if d.active_id == h.id {
                    println!("active trigger {:?}", h.id);
                    d.restore_light();
                }
            }
            true
        });
        let proxy = dbus_session.with_proxy("org.freedesktop.ScreenSaver", "/org/freedesktop/ScreenSaver", time::Duration::from_millis(5000));
        let _id = proxy.match_signal(|h: screensaver::OrgFreedesktopScreenSaverActiveChanged, _: &Connection, _: &Message| {
            println!("ActiveChanged {:?}", h.arg0);
            {
                let mut d = dev_manager();
                if h.arg0 {
                    d.light_off();
                } else {
                    d.restore_light();
                }
            }
            true
        });

        loop { 
            if let Ok(res) = dbus_session.process(time::Duration::from_millis(1000)) {
                if res {
                    dev_manager().add_active_watch(&proxy_idle);
                }
                dev_manager().add_idle_watch(&proxy_idle);
            }
        }

    })
}

fn start_battery_monitor_task() -> JoinHandle<()> {
    thread::spawn(move || {
        let dbus_system = Connection::new_system()
            .expect("should be able to connect to D-Bus system bus");
        info!("Connected to the system D-Bus");

        let proxy_ac = dbus_system.with_proxy(
            "org.freedesktop.UPower",
            "/org/freedesktop/UPower/devices/line_power_AC0",
            time::Duration::from_millis(5000)
        );

        let proxy_battery = dbus_system.with_proxy(
            "org.freedesktop.UPower",
            "/org/freedesktop/UPower/devices/battery_BAT0",
            time::Duration::from_millis(5000)
        );

        let proxy_login = dbus_system.with_proxy(
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            time::Duration::from_millis(5000)
        );

        let _id = proxy_ac.match_signal(|h: battery::OrgFreedesktopDBusPropertiesPropertiesChanged, _: &Connection, _: &Message| {
            let online: Option<&bool> = arg::prop_cast(&h.changed_properties, "Online");
            if let Some(online) = online {
                info!("AC0 online: {:?}", online);
                dev_manager().set_ac_state(*online);
            }
            true
        });

        let _id = proxy_battery.match_signal(|h: battery::OrgFreedesktopDBusPropertiesPropertiesChanged, _: &Connection, _: &Message| {
            let perc: Option<&f64> = arg::prop_cast(&h.changed_properties, "Percentage");
            if let Some(perc) = perc {
                info!("Battery percentage: {:.1}", perc);
            }
            true
        });

        let _id = proxy_login.match_signal(|h: login1::OrgFreedesktopLogin1ManagerPrepareForSleep, _: &Connection, _: &Message| {
            info!("PrepareForSleep {:?}", h.start);
            {
                let mut d = dev_manager();
                d.set_ac_state_get();
                if h.start {
                    d.light_off();
                } else {
                    d.restore_light();
                }
            }
            true
        });

        loop { dbus_system.process(time::Duration::from_millis(1000)).unwrap(); }
    })
}

/// Monitors signals and stops the daemon when receiving one
pub fn start_shutdown_task() -> JoinHandle<()> {
    thread::spawn(|| {
        let mut signals = Signals::new([SIGINT, SIGTERM]).unwrap();
        let _ = signals.forever().next();
        
        // If we reach this point, we have a signal and it is time to exit
        println!("Received signal, cleaning up");
        let json = effect_manager().save();
        if let Err(error) = config::Configuration::write_effects_save(json) {
            error!("Error writing config {}", error);
        }
        if std::fs::metadata(comms::SOCKET_PATH).is_ok() {
            std::fs::remove_file(comms::SOCKET_PATH).unwrap();
        }
        std::process::exit(0);
    })
}

fn handle_data(mut stream: UnixStream) {
    let mut buffer = [0u8; 4096];
    if stream.read(&mut buffer).is_err() {
        return;
    }

    if let Some(cmd) = comms::read_from_socket_req(&buffer) {
        if let Some(s) = process_client_request(cmd) {
            if let Ok(x) = bincode::serialize(&s) {
                let result = stream.write_all(&x);

                if let Err(error) = result {
                    println!("Client disconnected with error: {error}");
                }
            }
        }
    }
}

pub fn process_client_request(cmd: comms::DaemonCommand) -> Option<comms::DaemonResponse> {
    let mut d = dev_manager();
    match cmd {
        comms::DaemonCommand::SetPowerMode { ac, pwr, cpu, gpu } => {
            Some(comms::DaemonResponse::SetPowerMode { result: d.set_power_mode(ac, pwr, cpu, gpu) })
        },
        comms::DaemonCommand::SetFanSpeed { ac, rpm } => {
            Some(comms::DaemonResponse::SetFanSpeed { result: d.set_fan_rpm(ac, rpm) })
        },
        comms::DaemonCommand::SetLogoLedState{ ac, logo_state } => {
            Some(comms::DaemonResponse::SetLogoLedState { result: d.set_logo_led_state(ac, logo_state) })
        },
        comms::DaemonCommand::SetBrightness { ac, val } => {
            Some(comms::DaemonResponse::SetBrightness {result: d.set_brightness(ac, val) })
        }
        comms::DaemonCommand::SetIdle { ac, val } => {
            Some(comms::DaemonResponse::SetIdle { result: d.change_idle(ac, val) })
        }
        comms::DaemonCommand::SetSync { sync } => {
            Some(comms::DaemonResponse::SetSync { result: d.set_sync(sync) })
        }
        comms::DaemonCommand::GetBrightness{ac} =>  {
            Some(comms::DaemonResponse::GetBrightness { result: d.get_brightness(ac)})
        },
        comms::DaemonCommand::GetLogoLedState{ac} => Some(comms::DaemonResponse::GetLogoLedState {logo_state: d.get_logo_led_state(ac) }),
        comms::DaemonCommand::GetKeyboardRGB { layer } => {
            let map = effect_manager().get_map(layer);
            Some(comms::DaemonResponse::GetKeyboardRGB {
                layer,
                rgbdata: map,
            })
        }
        comms::DaemonCommand::GetSync() => Some(comms::DaemonResponse::GetSync { sync: d.get_sync() }),
        comms::DaemonCommand::GetFanSpeed{ac} => Some(comms::DaemonResponse::GetFanSpeed { rpm: d.get_fan_rpm(ac)}),
        comms::DaemonCommand::GetPwrLevel{ac} => Some(comms::DaemonResponse::GetPwrLevel { pwr: d.get_power_mode(ac) }),
        comms::DaemonCommand::GetCPUBoost{ac} => Some(comms::DaemonResponse::GetCPUBoost { cpu: d.get_cpu_boost(ac) }),
        comms::DaemonCommand::GetGPUBoost{ac} => Some(comms::DaemonResponse::GetGPUBoost { gpu: d.get_gpu_boost(ac) }),
        comms::DaemonCommand::SetEffect{ name, params } => {
            let mut res;
            {
                let mut k = effect_manager();
                res = true;
                let effect = match name.as_str() {
                    "static" => Some(kbd::effects::Static::new(params)),
                    "static_gradient" => Some(kbd::effects::StaticGradient::new(params)),
                    "wave_gradient" => Some(kbd::effects::WaveGradient::new(params)),
                    "breathing_single" => Some(kbd::effects::BreathSingle::new(params)),
                    _ => None
                };

                if let Some(laptop) = d.get_device() {
                    if let Some(e) = effect {
                        k.pop_effect(laptop); // Remove old layer
                        k.push_effect(
                            e,
                            [true; 90]
                            );
                    } else {
                        res = false
                    }
                } else {
                    res = false;
                }
            }
            Some(comms::DaemonResponse::SetEffect{result: res})
        }

        comms::DaemonCommand::SetStandardEffect{ name, params } => {
            // TODO save standart effect may be struct ?
            let res;
            if let Some(laptop) = d.get_device() {
                {
                    let mut k = effect_manager();
                    k.pop_effect(laptop); // Remove old layer
                    let _res = match name.as_str() {
                        "off" => d.set_standard_effect(device::RazerLaptop::OFF, params),
                        "wave" => d.set_standard_effect(device::RazerLaptop::WAVE, params),
                        "reactive" => d.set_standard_effect(device::RazerLaptop::REACTIVE, params),
                        "breathing" => d.set_standard_effect(device::RazerLaptop::BREATHING, params),
                        "spectrum" => d.set_standard_effect(device::RazerLaptop::SPECTRUM, params),
                        "static" => d.set_standard_effect(device::RazerLaptop::STATIC, params),
                        "starlight" => d.set_standard_effect(device::RazerLaptop::STARLIGHT, params), 
                        _ => false,
                    };
                    res = _res;
                }
            } else {
                res = false;
            }
            Some(comms::DaemonResponse::SetStandardEffect{result: res})
        }
        comms::DaemonCommand::SetBatteryHealthOptimizer { is_on, threshold } => { 
            Some(comms::DaemonResponse::SetBatteryHealthOptimizer { result: d.set_bho_handler(is_on, threshold)})
        }
        comms::DaemonCommand::GetBatteryHealthOptimizer() => {
            d.get_bho_handler().map(|result| 
                comms::DaemonResponse::GetBatteryHealthOptimizer {
                    is_on: (result.0), 
                    threshold: (result.1) 
                }
            )
        }
        comms::DaemonCommand::GetDeviceName => {
            let name = match &d.device {
                Some(device) => device.get_name(),
                None => "Unknown Device".into()
            };
            Some(comms::DaemonResponse::GetDeviceName { name })
        }

        comms::DaemonCommand::SetEnableLightControl { enable } => {
            Some(comms::DaemonResponse::SetEnableLightControl { result: d.set_light_control(enable) })
        }
        comms::DaemonCommand::GetEnableLightControl => {
            Some(comms::DaemonResponse::GetEnableLightControl { enabled: d.get_light_control()})
        }

    }
}


