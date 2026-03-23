use hidapi::{HidApi, HidResult};

pub mod razer_hidapi;

pub const RAZER_VENDOR_ID: u16 = 0x1532;

pub struct Device {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16
}

/// Gets a list of Razer Devices. Not all of them are laptops.
pub fn razer_devices() -> HidResult<Vec<Device>> {
    let hid_api = HidApi::new()?;

    let devices = hid_api.device_list()
        .filter(|d| d.vendor_id() == RAZER_VENDOR_ID)
        .filter(|d| d.interface_number() == 0)
        .map(|d| {
            let manufacturer_string = d.manufacturer_string().unwrap_or_default();
            let product_string = d.product_string().unwrap_or_default();
            Device {
                name: format!("{} - {}", manufacturer_string, product_string),
                vendor_id: d.vendor_id(),
                product_id: d.product_id()
            }
        });

    Ok(devices.collect())
}
