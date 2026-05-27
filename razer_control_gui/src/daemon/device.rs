// mod kbd;
use service::SupportedDevice;
use hidapi::HidApi;
use razer_laptop::razer_devices;
use razer_laptop::razer_hidapi::RazerHidapi;
use razer_laptop::razer_hidapi::RazerPacket;
use std::{ffi::CString, io, time};
use crate::dbus_mutter_idlemonitor;
use crate::config;
use crate::battery;
use dbus::blocking::Connection;

use log::*;
pub struct DeviceManager {
    pub device: Option<RazerLaptop>,
    cooling_pad: Option<CoolingPad>,
    supported_devices: Vec<SupportedDevice>,
    pub config: Option <config::Configuration>,
    pub idle_id: u32,
    pub active_id: u32,
    add_active: bool,
    pub change_idle: bool,
}

impl DeviceManager {
    pub fn new () -> DeviceManager {
        DeviceManager {
            device: None,
            cooling_pad: None,
            supported_devices: vec![],
            config: None,
            idle_id: 0,
            active_id: 0,
            add_active: false,
            change_idle: false,
        }
    }

    pub fn add_idle_watch(&mut self, proxy_idle: &dyn dbus_mutter_idlemonitor::OrgGnomeMutterIdleMonitor) {
        if self.change_idle {
            let mut timeout: u64 = 0;
            let mut state: usize = 0;
            if let Some(laptop) = self.get_device() {
                state = laptop.get_ac_state();
            }
            if let Some(config) = self.get_config() {
                timeout = config.power[state].idle as u64 * 60 * 1000; // idle is in minutes timeout is in miliseconds
            }
            if timeout != 0 {
                if self.idle_id != 0 {
                    self.remove_watch(proxy_idle);
                }
                if let Ok(id) = proxy_idle.add_idle_watch(timeout) {
                    println!("idle handler {:?}", id);
                    self.idle_id = id;
                }
            } else if self.idle_id != 0 {
                self.remove_watch(proxy_idle);
            }
            self.change_idle = false;
        }
    }

    pub fn set_sync(&mut self, sync: bool) -> bool {
        debug!("called: set_sync");
        let mut ac: usize = 0;
        if let Some(laptop) = self.get_device() {
            ac = laptop.ac_state as usize;
        }
        let other = (ac + 1) & 0x01;
        if let Some(config) = self.get_config() {
            config.sync = sync;
            config.power[other].brightness = config.power[ac].brightness;
            config.power[other].logo_state = config.power[ac].logo_state;
            config.power[other].screensaver = config.power[ac].screensaver;
            config.power[other].idle = config.power[ac].idle;
            if let Err(e) = config.write_to_file() {
                eprintln!("Error write config {:?}", e);
            }
        }

        true
    }

    pub fn get_sync(&mut self) -> bool {
        if let Some(config) = self.get_config() {
            return config.sync;
        }

        false
    }

    pub fn set_light_control(&mut self, enabled: bool) -> bool {
        if let Some(config) = self.get_config() {
            if config.enable_light_control != enabled {
                config.enable_light_control = enabled;
                if let Err(e) = config.write_to_file() {
                    eprintln!("Error write config {:?}", e);
                }
            }
        }

        true
    }

    pub fn get_light_control(&mut self) -> bool {
        if let Some(config) = self.get_config() {
            return config.enable_light_control;
        }

        false
    }

    fn remove_watch(&mut self, proxy_idle: &dyn dbus_mutter_idlemonitor::OrgGnomeMutterIdleMonitor) {
        if proxy_idle.remove_watch(self.idle_id).is_ok() {
            println!("remove idle handler");
        }
    }

    pub fn add_active_watch(&mut self, proxy_idle: &dyn dbus_mutter_idlemonitor::OrgGnomeMutterIdleMonitor) {
        if self.add_active {
            if let Ok(id) = proxy_idle.add_user_active_watch() {
                println!("active handler {:?}", id);
                self.active_id = id;
            }
        }
    }

    pub fn read_laptops_file() -> io::Result<DeviceManager > {
        let device_data = service::get_device_data();
        let mut res: DeviceManager = DeviceManager::new();
        res.supported_devices = serde_json::from_str(&device_data)?;
        println!("suported devices found: {:?}", res.supported_devices.len());
        match config::Configuration::read_from_config() {
            Ok(c) => res.config = Some(c),
            Err(_) => res.config = Some(config::Configuration::new()),
        }

        Ok(res)
    }

    fn get_ac_config(&mut self, ac: usize) -> Option<config::PowerConfig> {
        if let Some(c) = self.get_config() {
            return Some(c.power[ac]);
        }

        None
    }

    pub fn light_off(&mut self) {
        if self.idle_id != 0 {
            self.add_active = true;
        }
        if let Some(laptop) = self.get_device() {
            laptop.set_screensaver(true);
            laptop.set_brightness(0);
            laptop.set_logo_led_state(0);
        }
    }

    pub fn restore_light(&mut self) {
        self.add_active = false;
        let mut brightness = 0;
        let mut logo_state = 0;
        let mut ac:usize = 0;
        if let Some(laptop) = self.get_device() {
            ac = laptop.get_ac_state();
        }
        if let Some(config) = self.get_ac_config(ac) {
            brightness = config.brightness;
            logo_state = config.logo_state;
        }
        if let Some(laptop) = self.get_device() {
            laptop.set_screensaver(false);
            laptop.set_brightness(brightness);
            laptop.set_logo_led_state(logo_state);
        }
    }

    pub fn restore_standard_effect(&mut self) {
        if !self.get_light_control() { return; }

        let mut effect = 0;
        let mut params: Vec<u8> = vec![];
        if let Some(config) = self.get_config() {
            effect = config.standard_effect;
            params = config.standard_effect_params.clone();
        }
        if let Some(laptop) = self.get_device() {
            laptop.set_standard_effect(effect, params);
        }
    }

    pub fn change_idle(&mut self, ac: usize, timeout: u32) -> bool {
        // let mut arm: bool = false;
        if let Some(config) = self.get_config() {
            if config.power[ac].idle != timeout {
                config.power[ac].idle = timeout;
                if config.sync {
                    let other = (ac + 1) & 0x01;
                    config.power[other].idle = timeout;
                }
                if let Err(e) = config.write_to_file() {
                    eprintln!("Error write config {:?}", e);
                }
                // arm = true;
                self.change_idle = true;
            }
        }

        true
    }

    pub fn set_power_mode(&mut self, ac: usize, pwr: u8, cpu: u8, gpu: u8) -> bool {
        let mut res: bool = false;
        if let Some(config) = self.get_config() {
            config.power[ac].power_mode = pwr;
            config.power[ac].cpu_boost = cpu;
            config.power[ac].gpu_boost = gpu;
            if let Err(e) = config.write_to_file() {
                eprintln!("Error write config {:?}", e);
            }
        }
        if let Some(laptop) = self.get_device() {
            let state = laptop.get_ac_state();
            if state != ac {
                res = true;
            } else {
                res = laptop.set_power_mode(pwr, cpu, gpu);
            }
        }

        res
    }

    pub fn set_standard_effect(&mut self, effect_id: u8, params: Vec<u8>) -> bool {
        if let Some(config) = self.get_config() {
            config.standard_effect = effect_id;
            config.standard_effect_params = params.clone();
            if let Err(e) = config.write_to_file() {
                eprintln!("Error write config {:?}", e);
            }
        }
        if let Some(laptop) = self.get_device() {
            laptop.set_standard_effect(effect_id, params);
        }

        true
    }

    pub fn set_fan_rpm(&mut self, ac:usize, rpm: i32) -> bool {
        let mut res: bool = false;
        if let Some(config) = self.get_config() {
            config.power[ac].fan_rpm = rpm;
            if let Err(e) = config.write_to_file() {
                eprintln!("Error write config {:?}", e);
            }
        }
             
        if let Some(laptop) = self.get_device() {
            let state = laptop.get_ac_state();
            if state != ac {
                res = true;
            } else {
                res = laptop.set_fan_rpm(rpm as u16);
            }
        }

        res
    }

    pub fn set_logo_led_state(&mut self, ac:usize, logo_state: u8) -> bool {
        let mut res: bool = false;
        if let Some(config) = self.get_config() {
            config.power[ac].logo_state = logo_state;
            if config.sync {
                let other = (ac + 1) & 0x01;
                config.power[other].logo_state = logo_state;
            }
            if let Err(e) = config.write_to_file() {
                eprintln!("Error write config {:?}", e);
            }
        }
             
        if let Some(laptop) = self.get_device() {
            let state = laptop.get_ac_state();
           
            if state != ac {
                res = true;
            } else {
                res = laptop.set_logo_led_state(logo_state);
            }
        }

        res
    }

    pub fn get_logo_led_state(&mut self, ac: usize) -> u8 {
        // if let Some(laptop) = self.get_device() {
            // if laptop.ac_state as usize == ac {
                // return laptop.get_logo_led_state();
            // }
        // }
    
        if let Some(config) = self.get_ac_config(ac) {
            return config.logo_state;
        }

        0
    }

    pub fn set_brightness(&mut self, ac:usize, brightness: u8) -> bool {
        let mut res: bool = false;
        let _val = brightness as u16  * 255 / 100;
        if let Some(config) = self.get_config() {
            config.power[ac].brightness = _val as u8;
            if config.sync {
                let other = (ac + 1) & 0x01;
                config.power[other].brightness = _val as u8;
            }
            if let Err(e) = config.write_to_file() {
                eprintln!("Error write config {:?}", e);
            }
        }
 
        if let Some(laptop) = self.get_device() {
            let state = laptop.get_ac_state();
            if state != ac {
                res = true;
            } else {
                res = laptop.set_brightness(_val as u8);
            }
        }

        res
    }

    pub fn get_brightness(&mut self, ac: usize) -> u8 {
        if let Some(laptop) = self.get_device() {
            if laptop.ac_state as usize == ac {
                let val = laptop.get_brightness() as u32;
                let mut perc = val * 100 * 100/ 255;
                perc += 50;
                perc /= 100;
                return perc as u8;
            }
        }

        if let Some(config) = self.get_ac_config(ac) {
            let val = config.brightness as u32;
            let mut perc = val * 100 * 100/ 255;
            perc += 50;
            perc /= 100;
            return perc as u8;
        }

        0
    }

    pub fn get_fan_rpm(&mut self, ac: usize) -> i32 {
        if let Some(laptop) = self.get_device() {
            if laptop.ac_state as usize == ac {
                return laptop.get_fan_rpm() as i32;
            }
        }

        if let Some(config) = self.get_ac_config(ac) {
            return config.fan_rpm;
        }

        0
    }

    pub fn get_power_mode(&mut self, ac:usize) -> u8 {
        if let Some(laptop) = self.get_device() {
            if laptop.ac_state as usize == ac {
                return laptop.get_power_mode(0x01);
            }
        }

        if let Some(config) = self.get_ac_config(ac) {
            return config.power_mode;
        }

        0
    }

    pub fn get_cpu_boost(&mut self, ac:usize) -> u8 {
        if let Some(laptop) = self.get_device() {
            if laptop.ac_state as usize == ac {
                return laptop.get_cpu_boost();
            }
        }

        if let Some(config) = self.get_ac_config(ac) {
            return config.cpu_boost;
        }

        0
    }

    pub fn get_gpu_boost(&mut self, ac:usize) -> u8 {
        if let Some(laptop) = self.get_device() {
            if laptop.ac_state as usize == ac {
                return laptop.get_gpu_boost();
            }
        }

        if let Some(config) = self.get_ac_config(ac) {
            return config.gpu_boost;
        }

        0
    }

    pub fn set_ac_state(&mut self, ac: bool) {
        if let Some(laptop) = self.get_device() {
            laptop.set_ac_state(ac);
        }
        self.change_idle = true;
        let config: Option<config::PowerConfig> = self.get_ac_config(ac as usize);
        if let Some(config) = config {
            if let Some(laptop) = self.get_device() {
                laptop.set_config(config);
            }
        }
    }

    pub fn set_ac_state_get(&mut self) {
        let dbus_system = Connection::new_system()
            .expect("failed to connect to D-Bus system bus");
        let proxy_ac = dbus_system.with_proxy("org.freedesktop.UPower", "/org/freedesktop/UPower/devices/line_power_AC0", time::Duration::from_millis(5000));
        use battery::OrgFreedesktopUPowerDevice;
        if let Ok(online) = proxy_ac.online() {
            if let Some(laptop) = self.get_device() {
                laptop.set_ac_state(online);
            }
            self.change_idle = true;
            let config: Option<config::PowerConfig> = self.get_ac_config(online as usize);
            if let Some(config) = config {
                if let Some(laptop) = self.get_device() {
                    laptop.set_config(config);
                }
            }
        }

    }

    pub fn get_device(&mut self) -> Option<&mut RazerLaptop> {
        self.device.as_mut()
    }

    pub fn set_bho_handler(&mut self, is_on: bool, threshold: u8) -> bool {
        self.get_device()
            .map_or(false, |laptop| laptop.set_bho(is_on, threshold))
    }

    pub fn get_bho_handler(&mut self) -> Option<(bool, u8)> {
        self.get_device()
            .and_then(|laptop| laptop.get_bho()
            .map(byte_to_bho))
    }

    pub fn refresh_cooling_pad(&mut self) {
        let hid_api = match HidApi::new() {
            Ok(hid_api) => hid_api,
            Err(error) => {
                error!("Failed to refresh cooling pad state: {error}");
                return;
            }
        };

        let candidates: Vec<(CString, i32)> = hid_api
            .device_list()
            .filter(|device| {
                device.vendor_id() == CoolingPad::VENDOR_ID
                    && device.product_id() == CoolingPad::PRODUCT_ID
            })
            .map(|device| (CString::from(device.path()), device.interface_number()))
            .collect();

        let known_path = self.cooling_pad.as_ref().map(|device| device.path.clone());
        if let Some(current) = known_path.clone() {
            if candidates
                .iter()
                .any(|(path, _)| path.as_c_str() == current.as_c_str())
            {
                return;
            }
        }

        let discovered = candidates.into_iter().find_map(|(path, interface_number)| {
            match CoolingPad::open(&hid_api, &path) {
                Ok(cooling_pad) => Some((cooling_pad, interface_number)),
                Err(error) => {
                    debug!(
                        "Cooling pad probe failed on interface {}: {}",
                        interface_number, error
                    );
                    None
                }
            }
        });

        match (known_path, discovered) {
            (_, Some((cooling_pad, interface_number))) => {
                self.cooling_pad = Some(cooling_pad);
                info!("Cooling pad connected on interface {}", interface_number);
                self.restore_cooling_pad();
            }
            (Some(_), None) => {
                self.cooling_pad = None;
                info!("Cooling pad disconnected");
            }
            (None, None) => {}
        }
    }

    fn restore_cooling_pad(&mut self) {
        let config = self
            .config
            .as_ref()
            .map(|config| config.cooling_pad.clone());

        if let (Some(config), Some(cooling_pad)) = (config, self.cooling_pad.as_mut()) {
            let _ = if config.effect == "static" {
                cooling_pad.set_color(&config.effect_params)
                    && cooling_pad.set_effect("static", &config.effect_params)
            } else {
                cooling_pad.set_effect(&config.effect, &config.effect_params)
            };
            let _ = cooling_pad.set_fan_rpm(config.fan_rpm);
        }
    }

    pub fn get_cooling_pad_state(&mut self) -> (bool, i32, String, Vec<u8>) {
        let (effect, effect_params, fallback_fan_rpm) = self
            .config
            .as_ref()
            .map(|config| {
                (
                    config.cooling_pad.effect.clone(),
                    config.cooling_pad.effect_params.clone(),
                    config.cooling_pad.fan_rpm,
                )
            })
            .unwrap_or_else(|| ("off".into(), vec![], 0));

        (
            self.cooling_pad.is_some(),
            fallback_fan_rpm,
            effect,
            effect_params,
        )
    }

    pub fn set_cooling_pad_fan_rpm(&mut self, rpm: i32) -> bool {
        let rpm = rpm.max(0);

        if let Some(config) = self.get_config() {
            config.cooling_pad.fan_rpm = rpm;
            if let Err(error) = config.write_to_file() {
                eprintln!("Error write config {:?}", error);
            }
        }

        self.cooling_pad
            .as_mut()
            .is_some_and(|cooling_pad| cooling_pad.set_fan_rpm(rpm))
    }

    pub fn set_cooling_pad_effect(&mut self, effect: String, params: Vec<u8>) -> bool {
        if let Some(config) = self.get_config() {
            config.cooling_pad.effect = effect.clone();
            config.cooling_pad.effect_params = params.clone();
            if let Err(error) = config.write_to_file() {
                eprintln!("Error write config {:?}", error);
            }
        }

        self.cooling_pad.as_mut().is_some_and(|cooling_pad| {
            if effect == "static" {
                cooling_pad.set_color(&params) && cooling_pad.set_effect("static", &params)
            } else {
                cooling_pad.set_effect(&effect, &params)
            }
        })
    }

    fn get_config(&mut  self) -> Option<&mut config::Configuration> {
        self.config.as_mut()
    }

    // pub fn set_device(&mut self, device: RazerLaptop) {
        // self.device = Some(device);
    // }

    pub fn find_supported_device(&mut self, vid: u16, pid: u16) -> Option<&SupportedDevice> {
        for device in &self.supported_devices {
            // Unwrap: we control the strings and know they are are valid
            let svid = u16::from_str_radix(&device.vid, 16).unwrap();
            let spid = u16::from_str_radix(&device.pid, 16).unwrap();

            if svid == vid && spid == pid {
                return Some(device);
            }
        }

        None
    }

    pub fn discover_devices(&mut self)  {
        match razer_devices() {
            Ok(devices) => {
                for device in devices {

                    let result = self.find_supported_device(device.vendor_id, device.product_id);
                    if let Some(supported_device) = result {

                        match device.open() {
                            Ok(device) => {
                                self.device = Some(RazerLaptop::new(
                                    supported_device.name.clone(),
                                    supported_device.features.clone(),
                                    supported_device.fan.clone(),
                                    device
                                ));
                                break;
                            }
                            Err(e) => {
                                eprintln!("Error: {}", e);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
            },
        }
    }
}

struct CoolingPad {
    path: CString,
    device: hidapi::HidDevice,
}

impl CoolingPad {
    const VENDOR_ID: u16 = 0x1532;
    const PRODUCT_ID: u16 = 0x0F43;
    const REPORT_LEN: usize = 91;
    const REPORT_ID: u8 = 0x00;
    const PACKET_LEN: usize = 90;
    const TRANSACTION_ID: u8 = 0x1F;
    const STATUS_NEW: u8 = 0x00;
    const STATUS_BUSY: u8 = 0x01;
    const STATUS_OK: u8 = 0x02;
    const MIN_FAN_RPM: i32 = 500;
    const MAX_FAN_RPM: i32 = 3200;
    const FAN_RANGE: i32 = Self::MAX_FAN_RPM - Self::MIN_FAN_RPM;
    const CMD_CLASS_FAN: u8 = 0x0D;
    const CMD_CLASS_LED: u8 = 0x0F;
    const CMD_ID_SET_FAN_SPEED: u8 = 0x01;
    const CMD_ID_GET_FAN_SPEED: u8 = 0x81;
    const CMD_ID_SET_EFFECT: u8 = 0x02;
    const CMD_ID_SET_COLOR: u8 = 0x03;
    const CMD_ID_SET_BRIGHTNESS: u8 = 0x04;
    const LED_STORAGE_VARIABLE: u8 = 0x01;
    const LED_ID_ALL: u8 = 0x00;
    const LED_ID_BACKLIGHT: u8 = 0x05;
    const EFFECT_OFF: u8 = 0x00;
    const EFFECT_STATIC: u8 = 0x01;
    const EFFECT_BREATHING: u8 = 0x02;
    const EFFECT_WAVE: u8 = 0x03;
    const EFFECT_SPECTRUM: u8 = 0x04;
    const FAN_CLASS_ID: u8 = 0x01;
    const FAN_THERMAL_ID: u8 = 0x05;

    fn open(hid_api: &HidApi, path: &CString) -> anyhow::Result<CoolingPad> {
        let device = hid_api.open_path(path.as_c_str())?;
        let mut probe = [0u8; Self::REPORT_LEN];
        probe[0] = Self::REPORT_ID;
        let size = device.get_feature_report(&mut probe)?;
        if size != Self::REPORT_LEN {
            anyhow::bail!("unexpected feature report length {size}");
        }

        Ok(CoolingPad {
            path: path.clone(),
            device,
        })
    }

    fn calc_crc(packet: &[u8; Self::PACKET_LEN]) -> u8 {
        let mut crc = 0u8;
        for byte in packet.iter().take(88).skip(2) {
            crc ^= *byte;
        }
        crc
    }

    fn send_command(
        &mut self,
        command_class: u8,
        command_id: u8,
        args: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        let mut packet = [0u8; Self::PACKET_LEN];
        let copy_len = args.len().min(80);
        packet[0] = Self::STATUS_NEW;
        packet[1] = Self::TRANSACTION_ID;
        packet[5] = copy_len as u8;
        packet[6] = command_class;
        packet[7] = command_id;
        packet[8..8 + copy_len].copy_from_slice(&args[..copy_len]);
        packet[88] = Self::calc_crc(&packet);

        let mut report = [0u8; Self::REPORT_LEN];
        report[0] = Self::REPORT_ID;
        report[1..].copy_from_slice(&packet);

        for _ in 0..3 {
            self.device.send_feature_report(&report)?;
            std::thread::sleep(std::time::Duration::from_millis(10));

            let mut response = [0u8; Self::REPORT_LEN];
            response[0] = Self::REPORT_ID;
            let size = self.device.get_feature_report(&mut response)?;
            if size != Self::REPORT_LEN {
                anyhow::bail!("unexpected response length {size}");
            }

            let status = response[1];
            let response_class = response[7];
            let response_id = response[8];

            if response_class != command_class || response_id != command_id {
                anyhow::bail!(
                    "response mismatch: expected {:02x}:{:02x}, got {:02x}:{:02x}",
                    command_class,
                    command_id,
                    response_class,
                    response_id
                );
            }

            if status == Self::STATUS_OK {
                return Ok(response[1..].to_vec());
            }

            if status != Self::STATUS_BUSY {
                anyhow::bail!(
                    "device returned status {:02x} for {:02x}:{:02x}",
                    status,
                    command_class,
                    command_id
                );
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        anyhow::bail!(
            "device remained busy for {:02x}:{:02x}",
            command_class,
            command_id
        )
    }

    fn rpm_to_percent(rpm: i32) -> u8 {
        let percent = ((rpm - Self::MIN_FAN_RPM) as f64 / Self::FAN_RANGE as f64) * 100f64;
        percent.round().clamp(0f64, 100f64) as u8
    }

    fn percent_to_rpm(percent: u8) -> i32 {
        Self::MIN_FAN_RPM + (i32::from(percent) * Self::FAN_RANGE / 100)
    }

    fn set_fan_rpm(&mut self, rpm: i32) -> bool {
        if rpm == 0 {
            return self.send_fan_auto();
        }

        let rpm = rpm.clamp(Self::MIN_FAN_RPM, Self::MAX_FAN_RPM);
        let percent = Self::rpm_to_percent(rpm);
        self.send_command(
            Self::CMD_CLASS_FAN,
            Self::CMD_ID_SET_FAN_SPEED,
            &[Self::FAN_CLASS_ID, Self::FAN_THERMAL_ID, percent],
        )
        .is_ok()
    }

    fn get_fan_rpm(&mut self) -> Option<i32> {
        let response = self
            .send_command(
                Self::CMD_CLASS_FAN,
                Self::CMD_ID_GET_FAN_SPEED,
                &[Self::FAN_CLASS_ID, Self::FAN_THERMAL_ID, 0x00],
            )
            .ok()?;

        const PACKET_HEADER_LEN: usize = 8;
        const FAN_PERCENT_PAYLOAD_OFFSET: usize = 2;
        if response.len() <= 5 {
            return None;
        }

        let data_size = usize::from(response[5]);
        if data_size < FAN_PERCENT_PAYLOAD_OFFSET + 1 {
            return None;
        }

        let percent_index = PACKET_HEADER_LEN + FAN_PERCENT_PAYLOAD_OFFSET;
        if response.len() <= percent_index {
            return None;
        }

        Some(Self::percent_to_rpm(response[percent_index]))
    }

    fn send_fan_auto(&mut self) -> bool {
        let mut buf = [0u8; Self::REPORT_LEN];
        buf[0] = 0x00;
        buf[1..].copy_from_slice(&[
            0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x0D, 0x10, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x18, 0x00,
        ]);
        self.device.send_feature_report(&buf).is_ok()
    }

    fn set_brightness(&mut self, brightness: u8) -> bool {
        self.send_command(
            Self::CMD_CLASS_LED,
            Self::CMD_ID_SET_BRIGHTNESS,
            &[
                Self::LED_STORAGE_VARIABLE,
                Self::LED_ID_BACKLIGHT,
                brightness,
            ],
        )
        .is_ok()
    }

    fn set_effect(&mut self, name: &str, params: &[u8]) -> bool {
        let args: Vec<u8> = match name {
            "off" => vec![
                Self::LED_STORAGE_VARIABLE,
                Self::LED_ID_ALL,
                Self::EFFECT_OFF,
                0x00,
                0x00,
                0x00,
            ],
            "static" => {
                let [r, g, b] = Self::extract_color(params);
                vec![
                    Self::LED_STORAGE_VARIABLE,
                    Self::LED_ID_ALL,
                    Self::EFFECT_STATIC,
                    0x00,
                    0x00,
                    0x01,
                    r,
                    g,
                    b,
                ]
            }
            "breathing" => {
                let [r, g, b] = Self::extract_color(params);
                vec![
                    Self::LED_STORAGE_VARIABLE,
                    Self::LED_ID_ALL,
                    Self::EFFECT_BREATHING,
                    0x00,
                    0x00,
                    0x01,
                    r,
                    g,
                    b,
                ]
            }
            "wave" => vec![
                Self::LED_STORAGE_VARIABLE,
                Self::LED_ID_ALL,
                Self::EFFECT_WAVE,
                0x01,
                0x01,
                0x00,
            ],
            "spectrum" => vec![
                Self::LED_STORAGE_VARIABLE,
                Self::LED_ID_ALL,
                Self::EFFECT_SPECTRUM,
                0x00,
                0x00,
                0x00,
            ],
            _ => return false,
        };

        if name != "off" && !self.set_brightness(0xFF) {
            return false;
        }

        self.send_command(Self::CMD_CLASS_LED, Self::CMD_ID_SET_EFFECT, &args)
            .is_ok()
    }

    fn set_color(&mut self, params: &[u8]) -> bool {
        let [r, g, b] = Self::extract_color(params);
        self.send_command(
            Self::CMD_CLASS_LED,
            Self::CMD_ID_SET_COLOR,
            &[Self::LED_STORAGE_VARIABLE, Self::LED_ID_BACKLIGHT, r, g, b],
        )
        .is_ok()
    }

    fn extract_color(params: &[u8]) -> [u8; 3] {
        [
            *params.first().unwrap_or(&0xFF),
            *params.get(1).unwrap_or(&0x00),
            *params.get(2).unwrap_or(&0x00),
        ]
    }
}

pub struct RazerLaptop {
    name: String,
    features: Vec<String>,
    fan: Vec<u16>,
    device: RazerHidapi,
    power: u8, // need for fan
    fan_rpm: u8, // need for power
    ac_state: u8, // index config array
    screensaver: bool,
}
//
impl RazerLaptop {
// LED STORAGE Options
    const NOSTORE:u8 = 0x00;
    const VARSTORE:u8 = 0x01;
// LED definitions
    const LOGO_LED:u8 = 0x04;
    const BACKLIGHT_LED:u8 = 0x05;
// effects
    pub const OFF:u8 = 0x00;
    pub const WAVE:u8 = 0x01;
    pub const REACTIVE:u8 = 0x02; // Afterglo
    #[allow(dead_code)]
    pub const BREATHING:u8 = 0x03;
    pub const SPECTRUM:u8 = 0x04;
    pub const CUSTOMFRAME:u8 = 0x05;
    pub const STATIC:u8 = 0x06;
    #[allow(dead_code)]
    pub const STARLIGHT:u8 = 0x19;

    pub fn new(name: String, features: Vec<String>, fan: Vec<u16>, device: RazerHidapi) -> RazerLaptop {
        RazerLaptop {
            name,
            features,
            fan,
            device,
            power: 0,
            fan_rpm: 0,
            ac_state: 0,
            screensaver: false
        }
    }

    pub fn set_screensaver(&mut self, active: bool) {
        self.screensaver = active;
    }

    pub fn set_config(&mut self, config: config::PowerConfig) -> bool {
        let mut ret: bool = false;

        if !self.screensaver {
            ret |= self.set_brightness(config.brightness);
            ret |= self.set_logo_led_state(config.logo_state);
        } else {
            ret |= self.set_brightness(0);
            ret |= self.set_logo_led_state(0);
        }
        ret |= self.set_power_mode(config.power_mode, config.cpu_boost, config.gpu_boost);
        ret |= self.set_fan_rpm(config.fan_rpm as u16);

        ret
    }

    pub fn set_ac_state(&mut self, online: bool) -> usize {
        if online {
            self.ac_state = 1;
        } else {
            self.ac_state = 0;
        }

        self.ac_state as usize
    }

    pub fn get_ac_state(&mut self) -> usize {
        self.ac_state as usize
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn have_feature(&mut self, fch: String) -> bool {
        self.features.contains(&fch)
    }

    fn clamp_fan(&mut self, rpm: u16) -> u8 {
        if rpm > self.fan[1] {
            return (self.fan[1] / 100) as u8;
        }
        if rpm < self.fan[0] {
            return (self.fan[0] / 100) as u8;
        }

        (rpm / 100) as u8
    }

    fn clamp_u8(&mut self, value: u8, min: u8, max: u8) ->u8 {
        if value > max {
            return max;
        }
        if value < min {
            return min;
        }

        value
    }

    pub fn set_standard_effect(&mut self, effect_id: u8, params: Vec<u8>) -> bool {
        let mut report: RazerPacket = RazerPacket::new(0x03, 0x0a, 80);
        report.args[0] = effect_id; // effect id
        if !params.is_empty() {
            for idx in 0..params.len() {
                report.args[idx+1] = params[idx];
            }
        }
        if self.device.send_report(report).is_some() {
            return true;
        }

        false
    }

    pub fn set_custom_frame_data(&mut self, row: u8, data: Vec<u8>) {
        // if data.len() == kbd::board::KEYS_PER_ROW {
        if data.len() == 45 {
            let mut report: RazerPacket = RazerPacket::new(0x03, 0x0b, 0x34);
            report.args[0] = 0xff;
            report.args[1] = row;
            report.args[2] = 0x00; // start col
            report.args[3] = 0x0f; // end col
            for idx in 0..data.len() {
                report.args[idx + 7] = data[idx];
            }
            self.device.send_report(report);
        }
    }

    pub fn set_custom_frame(&mut self) -> bool {
        let mut report: RazerPacket = RazerPacket::new(0x03, 0x0a, 0x02);
        report.args[0] = RazerLaptop::CUSTOMFRAME; // effect id
        report.args[1] = RazerLaptop::NOSTORE;
        if self.device.send_report(report).is_some() {
            return true;
        }

        false
    }

    pub fn get_power_mode(&mut self, zone: u8) -> u8 {
        let mut report: RazerPacket = RazerPacket::new(0x0d, 0x82, 0x04);
        report.args[0] = 0x00;
        report.args[1] = zone;
        report.args[2] = 0x00;
        report.args[3] = 0x00;
        if let Some(response) = self.device.send_report(report) {
            return response.args[2];
        }
        0
    }

    fn set_power(&mut self, zone: u8) -> bool {
        let mut report: RazerPacket = RazerPacket::new(0x0d, 0x02, 0x04);
        report.args[0] = 0x00;
        report.args[1] = zone;
        report.args[2] = self.power;
        match self.fan_rpm {
            0 => report.args[3] = 0x00,
            _ => report.args[3] = 0x01
        }
        if self.device.send_report(report).is_some() {
            return  true;
        }

        false
    }

    pub fn get_cpu_boost(&mut self) -> u8 {
        let mut report: RazerPacket = RazerPacket::new(0x0d, 0x87, 0x03);
        report.args[0] = 0x00;
        report.args[1] = 0x01;
        report.args[2] = 0x00;
        if let Some(response) = self.device.send_report(report) {
            return response.args[2];
        }
        0
    }

    fn set_cpu_boost(&mut self, mut boost: u8) -> bool {
        let mut report: RazerPacket = RazerPacket::new(0x0d, 0x07, 0x03);
        if boost == 3 && !self.have_feature("boost".to_string()) {
            boost = 2;
        }
        report.args[0] = 0x00;
        report.args[1] = 0x01;
        report.args[2] = boost;
        if self.device.send_report(report).is_some() {
            return true;
        }

        false
    }

    fn get_gpu_boost(&mut self) -> u8 {
        let mut report: RazerPacket = RazerPacket::new(0x0d, 0x87, 0x03);
        report.args[0] = 0x00;
        report.args[1] = 0x02;
        report.args[2] = 0x00;
        if let Some(response) = self.device.send_report(report){
            return response.args[2];
        }
        0
    }

    fn set_gpu_boost(&mut self, boost: u8) -> bool {
        let mut report: RazerPacket = RazerPacket::new(0x0d, 0x07, 0x03);
        report.args[0] = 0x00;
        report.args[1] = 0x02;
        report.args[2] = boost;
        if self.device.send_report(report).is_some() {
            return true;
        }
        false
    }

    pub fn set_power_mode(&mut self, mode: u8, cpu_boost: u8, gpu_boost: u8) -> bool {
        if mode <= 3 {
            self.power = mode;
            self.set_power(0x01);
            self.set_power(0x02);
        } else if mode == 4 {
            self.power =  mode;
            self.fan_rpm = 0;
            self.get_power_mode(0x01);
            self.set_power(0x01);
            self.get_cpu_boost();
            self.set_cpu_boost(cpu_boost);
            self.get_gpu_boost();
            self.set_gpu_boost(gpu_boost);
            self.get_power_mode(0x02);
            self.set_power(0x02);
        }

        true
    }

    fn set_rpm(&mut self, zone: u8) -> bool {
        let mut report:RazerPacket = RazerPacket::new(0x0d, 0x01, 0x03);
        // Set fan RPM
        report.args[0] = 0x00;
        report.args[1] = zone;
        report.args[2] = self.fan_rpm;
        if self.device.send_report(report).is_some() {
            return true;
        }

        false
    }

    pub fn set_fan_rpm(&mut self, value: u16) -> bool {
        if self.power != 4 {
            match value == 0 {
                true => self.fan_rpm = value as u8,
                false => self.fan_rpm = self.clamp_fan(value),
            }
            self.get_power_mode(0x01);
            self.set_power(0x01);
            if value != 0 {
                self.set_rpm(0x01);
            }
            self.get_power_mode(0x02);
            self.set_power(0x02);
            if value != 0 {
                self.set_rpm(0x02);
            }
        }

        true
    }

    pub fn get_fan_rpm(&mut self) -> u16 {
        let res: u16 = self.fan_rpm as u16;
        res * 100
    }

    pub fn set_logo_led_state(&mut self, mode: u8) -> bool {
        if mode > 0 {
            let mut report: RazerPacket = RazerPacket::new(0x03, 0x02, 0x03);
            report.args[0] = RazerLaptop::VARSTORE;
            report.args[1] = RazerLaptop::LOGO_LED;
            if mode == 1 {
                report.args[2] = 0x00;
            } else if mode == 2 {
                report.args[2] = 0x02;
            }
            self.device.send_report(report);
        }

        let mut report: RazerPacket = RazerPacket::new(0x03, 0x00, 0x03);
        report.args[0] = RazerLaptop::VARSTORE;
        report.args[1] = RazerLaptop::LOGO_LED;
        report.args[2] = self.clamp_u8(mode, 0x00, 0x01);
        if self.device.send_report(report).is_some() {
            return true;
        }

        false
    }

    #[allow(dead_code)]
    pub fn get_logo_led_state(&mut self) -> u8 {
        let mut report: RazerPacket = RazerPacket::new(0x03, 0x82, 0x03);
        report.args[0] = RazerLaptop::VARSTORE;
        report.args[1] = RazerLaptop::LOGO_LED;
        if let Some(response) = self.device.send_report(report){
            return response.args[2];
        }
        0
    }

    pub fn set_brightness(&mut self, brightness: u8) -> bool {
        let mut report: RazerPacket = RazerPacket::new(0x03, 0x03, 0x03);
        report.args[0] = RazerLaptop::VARSTORE;
        report.args[1] = RazerLaptop::BACKLIGHT_LED;
        report.args[2] = brightness;
        if self.device.send_report(report).is_some() {
            return true;
        }

        false
    }

    pub fn get_brightness(&mut self) -> u8 {
        let mut report: RazerPacket = RazerPacket::new(0x03, 0x83, 0x03);
        report.args[0] = RazerLaptop::VARSTORE;
        report.args[1] = RazerLaptop::BACKLIGHT_LED;
        report.args[2] = 0x00;
        if let Some(response) = self.device.send_report(report){
            return response.args[2];
        }
        0
    }

    pub fn get_bho(&mut self) -> Option<u8> {
        if !self.have_feature("bho".to_string()) {
            return None;
        }

        let mut report: RazerPacket = RazerPacket::new(0x07, 0x92, 0x01);
        report.args[0] = 0x00;

        self.device.send_report(report).map(|resp| resp.args[0])
    }

    pub fn set_bho(&mut self, is_on: bool, threshold: u8) -> bool {
        if !self.have_feature("bho".to_string()) {
            return false;
        }

        let mut report = RazerPacket::new(0x07, 0x12, 0x01);
        report.args[0] = bho_to_byte(is_on, threshold);

        self.device.send_report(report)
            .map_or(false, |r| { 
                println!("Response Packet:\n{:#?}", r); 
                true
            } 
        )
    }

}

// top bit flags whether battery health optimization is on or off
// bottom bits are the actual threshold that it is set to
fn byte_to_bho(u: u8) -> (bool, u8) {
    (u & (1 << 7) != 0, (u & 0b0111_1111))
}

fn bho_to_byte(is_on: bool, threshold: u8) -> u8 {
    if is_on {
        return threshold | 0b1000_0000;
    }
    threshold
}
