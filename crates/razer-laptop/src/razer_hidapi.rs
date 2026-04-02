use std::thread;
use std::time::Duration;

use anyhow::Context;
use log::*;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Serialize, Deserialize, Debug)]
pub struct RazerPacket {
    pub report: u8,
    pub status: u8,
    pub id: u8,
    pub remaining_packets: u16,
    pub protocol_type: u8,
    pub data_size: u8,
    pub command_class: u8,
    pub command_id: u8,
    #[serde(with = "BigArray")]
    pub args: [u8; 80],
    pub crc: u8,
    pub reserved: u8,
}

impl RazerPacket {
// Command status
    const RAZER_CMD_NEW:u8 = 0x00;
    const RAZER_CMD_BUSY:u8 = 0x01;
    const RAZER_CMD_SUCCESSFUL:u8 = 0x02;
    // const RAZER_CMD_FAILURE:u8 = 0x03;
    // const RAZER_CMD_TIMEOUT:u8 =0x04;
    const RAZER_CMD_NOT_SUPPORTED:u8 = 0x05;

    pub fn new(command_class: u8, command_id: u8, data_size: u8) -> RazerPacket {
        RazerPacket {
            report: 0x00,
            status: RazerPacket::RAZER_CMD_NEW,
            id: 0x1F,
            remaining_packets: 0x0000,
            protocol_type: 0x00,
            data_size,
            command_class,
            command_id,
            args: [0x00; 80],
            crc: 0x00,
            reserved: 0x00,
        }
    }

    fn calc_crc(&mut self) -> Vec<u8>{
        let mut res: u8 = 0x00;
        let buf: Vec<u8> = bincode::serialize(self).unwrap();
        debug_assert!(buf.len() == 89);

        for byte in buf.iter().skip(2).copied() {
            res ^= byte;
        }

        self.crc = res;
        buf
    }
}

pub struct RazerHidapi {
    device: hidapi::HidDevice,
    reports_sent: u64,
    errors: u64,
    busy_events: u64,
}

impl RazerHidapi {

    pub fn new(device: hidapi::HidDevice) -> Self {
        RazerHidapi {
            device,
            reports_sent: 0,
            errors: 0,
            busy_events: 0
        }
    }

    pub fn send_report(&mut self, mut report: RazerPacket) -> Option<RazerPacket> {
        self.reports_sent += 1;

        let mut last_error = None;

        for _ in 0..3 {
            match self._send_report_inner(&mut report) {
                Ok(packet) => return Some(packet),
                Err(error) => last_error = Some(error)
            }
        }
    
        if let Some(error) = last_error {
            self.errors += 1;
            error!("Failed to send report: {error}");
            self.print_stats();
        }

        // The original code always sleeps before giving up
        thread::sleep(Duration::from_micros(8000));

        None
    }

    fn _send_report_inner(&mut self, report: &mut RazerPacket) -> anyhow::Result<RazerPacket>{
        let mut temp_buf: [u8; 91] = [0x00; 91];

        self.device.send_feature_report(report.calc_crc().as_slice())
            .context("failed to send feature report")?;
                
        thread::sleep(Duration::from_micros(1000));

        let size = self.device.get_feature_report(&mut temp_buf)
            .context("failed to get feature report")?;

        if size != 91 {
            anyhow::bail!("invalid report length. Expected: 91, got: {size}");
        }
        
        let response = bincode::deserialize::<RazerPacket>(&temp_buf)
            .context("failed to deserialize packet")?;

        // when request bho status the response command id is different from the request command id...
        if response.command_id == 0x92 {
            return Ok(response);
        }

        let response_matches_report = 
            response.remaining_packets == report.remaining_packets &&
            response.command_class == report.command_class &&
            response.command_id == report.command_id;

        if !response_matches_report {
            anyhow::bail!("response doesn't match request");
        }

        match response.status {
            RazerPacket::RAZER_CMD_BUSY => {
                self.busy_events += 1;
                anyhow::bail!("busy")
            },

            RazerPacket::RAZER_CMD_SUCCESSFUL => Ok(response),

            RazerPacket::RAZER_CMD_NOT_SUPPORTED => {
                anyhow::bail!("command not supported");
            },

            status => anyhow::bail!("unknown response status: {status}"),
        }
    }

    fn print_stats(&self) {
        let busy_events_percent = if self.reports_sent == 0 {
            0f32
        } else {
            self.busy_events as f32 / self.reports_sent as f32 * 100f32
        };

        info!(
            "Stats: {{sent: {}, busy: {}, errors: {}, busy_percent: {:.2}%}}",
            self.reports_sent,
            self.busy_events,
            self.errors,
            busy_events_percent,
        );
    }
}
