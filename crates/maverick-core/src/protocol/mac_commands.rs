//! LoRaWAN 1.0.x MAC command types and parsing.
//!
//! MAC commands are carried in the FOpts field of uplink frames or in the
//! FHDR/FOpts of downlink frames. This module provides structures and parsing
//! for Class A device MAC commands.
//!
//! Reference: LoRaWAN 1.0.x Specification Section 5.

use maverick_domain::DevEui;

/// LoRaWAN MAC command-c identifiers.
/// Note: Req/Ans share the same CID byte per LoRaWAN spec.
/// This enum represents the request variants; answers use the same CID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MacCid {
    /// LinkCheckReq — device requests signal quality assessment (CID=0x02).
    LinkCheckReq = 0x02,
    /// LinkADRReq — network requests device to change data rate, TX power, or channel mask (CID=0x03).
    LinkAdrReq = 0x03,
    /// DutyCycleReq — network limits device aggregate TX duty cycle (CID=0x04).
    DutyCycleReq = 0x04,
    /// RXParamSetupReq — network sets RX1/RX2 frequency and data rate (CID=0x05).
    RxParamSetupReq = 0x05,
    /// DevStatusReq — network requests device status (battery, margin) (CID=0x06).
    DevStatusReq = 0x06,
    /// NewChannelReq — device creates or modifies a channel (CID=0x07).
    NewChannelReq = 0x07,
    /// Unknown CID.
    Unknown = 0xFF,
}

impl MacCid {
    /// Parse a CID byte to MacCid.
    pub fn from_u8(b: u8) -> Self {
        match b {
            0x02 => Self::LinkCheckReq,
            0x03 => Self::LinkAdrReq,
            0x04 => Self::DutyCycleReq,
            0x05 => Self::RxParamSetupReq,
            0x06 => Self::DevStatusReq,
            0x07 => Self::NewChannelReq,
            _ => Self::Unknown,
        }
    }

    /// Get the CID byte value for LinkCheckAns (same as LinkCheckReq per spec).
    pub const LINK_CHECK_ANS_CID: u8 = 0x02;
}

/// LinkCheckAns response — network answer to LinkCheckReq.
/// Contains the demodulation margin (link margin in dB) and the number of
/// gateways that received the last uplink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkCheckAns {
    /// Demodulation margin in dB (0-254). Higher values indicate better
    /// signal quality. Value of 255 indicates an invalid measurement.
    pub margin: u8,
    /// Number of gateways that received the last uplink successfully.
    pub gateway_count: u8,
}

impl LinkCheckAns {
    /// Encode LinkCheckAns into FOpts bytes: [CID=0x02, margin, gateway_count].
    pub fn encode_fopts(&self) -> Vec<u8> {
        vec![MacCid::LINK_CHECK_ANS_CID, self.margin, self.gateway_count]
    }
}

/// Parsed MAC commands from an uplink FOpts field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMacCommands {
    /// True if LinkCheckReq was present in FOpts.
    pub link_check_req: bool,
    /// Any additional parsed commands (for future expansion).
    pub raw_commands: Vec<u8>,
}

impl ParsedMacCommands {
    /// Parse FOpts bytes for MAC commands.
    ///
    /// Each MAC command has the format: [CID, ...payload...]
    /// Returns `ParsedMacCommands` with flags for commands of interest.
    pub fn from_fopts(f_opts: &[u8]) -> Self {
        let mut link_check_req = false;
        let mut i = 0;
        while i < f_opts.len() {
            let cid = MacCid::from_u8(f_opts[i]);
            if cid == MacCid::LinkCheckReq {
                link_check_req = true;
            }
            // Move to next command (CID + command-specific length)
            // For LinkCheckReq, it's just 1 byte (CID only)
            // For others, we'd need to know their length
            i += 1;
        }
        Self {
            link_check_req,
            raw_commands: f_opts.to_vec(),
        }
    }
}

/// Downlink decision returned by protocol handler after processing uplink.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DownlinkDecision {
    /// Set ACK flag on the next downlink if true.
    pub ack_flag: bool,
    /// MAC commands to include in the next downlink.
    pub mac_commands: Vec<u8>,
    /// DevEUI of the device (if known from session).
    pub dev_eui: Option<DevEui>,
}

impl DownlinkDecision {
    /// Add LinkCheckAns to the MAC commands list.
    pub fn with_link_check_ans(self, ans: LinkCheckAns) -> Self {
        let mut mac_commands = self.mac_commands;
        mac_commands.extend(ans.encode_fopts());
        Self {
            mac_commands,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_check_req_parsing() {
        // FOpts containing LinkCheckReq: CID=0x02
        let f_opts = vec![0x02];
        let parsed = ParsedMacCommands::from_fopts(&f_opts);
        assert!(parsed.link_check_req);
    }

    #[test]
    fn empty_fopts_no_link_check() {
        let f_opts = vec![];
        let parsed = ParsedMacCommands::from_fopts(&f_opts);
        assert!(!parsed.link_check_req);
    }

    #[test]
    fn link_check_ans_encoding() {
        let ans = LinkCheckAns {
            margin: 10,
            gateway_count: 3,
        };
        let encoded = ans.encode_fopts();
        assert_eq!(encoded, vec![0x02, 10, 3]);
    }

    #[test]
    fn mac_cid_from_u8() {
        assert_eq!(MacCid::from_u8(0x02), MacCid::LinkCheckReq);
        assert_eq!(MacCid::from_u8(0x03), MacCid::LinkAdrReq);
        assert_eq!(MacCid::from_u8(0xFF), MacCid::Unknown);
    }
}
