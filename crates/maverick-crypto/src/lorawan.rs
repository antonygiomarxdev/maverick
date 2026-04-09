use crate::aes_ctr::{AesCtr, Cmac};
use crate::error::{CryptoError, Result};

pub const BLOCK_SIZE: usize = 16;
pub const MIC_SIZE: usize = 4;
pub const B0_MIC_CONSTANT: u8 = 0x49;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoRawanFrameHeader {
    pub dev_addr: u32,
    pub f_ctrl: u8,
    pub fcnt: u32,
}

impl LoRawanFrameHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 9 {
            return Err(CryptoError::InvalidFrameHeader { got: bytes.len() });
        }
        let dev_addr = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let f_ctrl = bytes[4];
        let fcnt = u16::from_le_bytes([bytes[5], bytes[6]]).into();
        Ok(Self {
            dev_addr,
            f_ctrl,
            fcnt,
        })
    }

    pub fn has_opts(&self) -> bool {
        self.f_ctrl & 0x80 != 0
    }

    pub fn fcnt_msb(&self) -> Option<u8> {
        if self.f_ctrl & 0x40 != 0 {
            Some((self.fcnt >> 16) as u8)
        } else {
            None
        }
    }
}

pub struct MicCalculation;

impl MicCalculation {
    pub fn compute_nwk_s_key_mic(
        key: &[u8; 16],
        dev_addr: u32,
        fcnt: u32,
        f_port: u8,
        payload: &[u8],
    ) -> Result<[u8; 4]> {
        Self::compute_mic_internal(key, dev_addr, fcnt, f_port, payload, true)
    }

    pub fn compute_app_s_key_mic(
        key: &[u8; 16],
        dev_addr: u32,
        fcnt: u32,
        f_port: u8,
        payload: &[u8],
    ) -> Result<[u8; 4]> {
        Self::compute_mic_internal(key, dev_addr, fcnt, f_port, payload, false)
    }

    fn compute_mic_internal(
        key: &[u8; 16],
        dev_addr: u32,
        fcnt: u32,
        f_port: u8,
        payload: &[u8],
        _is_nwk_s_key: bool,
    ) -> Result<[u8; 4]> {
        let cmac = Cmac::new(key)?;

        let mut buffer = [0u8; 32];
        buffer[0] = B0_MIC_CONSTANT;
        buffer[1] = 0x00;
        buffer[2] = 0x00;
        buffer[3] = 0x00;
        buffer[4] = 0x00;
        let dev_addr_bytes = dev_addr.to_le_bytes();
        buffer[5] = dev_addr_bytes[0];
        buffer[6] = dev_addr_bytes[1];
        buffer[7] = dev_addr_bytes[2];
        buffer[8] = dev_addr_bytes[3];
        buffer[9] = 0x00;
        buffer[10] = 0x00;
        buffer[11] = f_port;
        let fcnt_bytes = fcnt.to_le_bytes();
        buffer[12] = fcnt_bytes[0];
        buffer[13] = fcnt_bytes[1];
        buffer[14] = fcnt_bytes[2];
        buffer[15] = fcnt_bytes[3];
        buffer[16] = 0x00;
        let payload_len = payload.len() as u8;
        buffer[17] = payload_len;

        let mut offset = 18;
        for chunk in payload.chunks(16) {
            buffer[offset..offset + chunk.len()].copy_from_slice(chunk);
            offset += chunk.len();
        }

        let full_buffer = if payload.len() > 14 {
            buffer[..18 + payload.len()].to_vec()
        } else {
            buffer[..18].to_vec()
        };

        let padded = pad_to_block_size(&full_buffer);
        let computed = cmac.compute(&padded)?;

        Ok([computed[0], computed[1], computed[2], computed[3]])
    }
}

fn pad_to_block_size(data: &[u8]) -> Vec<u8> {
    let remainder = data.len() % BLOCK_SIZE;
    if remainder == 0 {
        data.to_vec()
    } else {
        let mut padded = data.to_vec();
        padded.push(0x80);
        while padded.len() % BLOCK_SIZE != 0 {
            padded.push(0x00);
        }
        padded
    }
}

pub struct MicValidator;

impl MicValidator {
    pub fn validate(
        key: &[u8; 16],
        dev_addr: u32,
        fcnt: u32,
        f_port: u8,
        payload: &[u8],
        mic: &[u8; 4],
    ) -> Result<bool> {
        let cmac = Cmac::new(key)?;

        let mut buffer = Vec::with_capacity(32 + payload.len());
        buffer.push(B0_MIC_CONSTANT);
        buffer.push(0x00);
        buffer.push(0x00);
        buffer.push(0x00);
        buffer.push(0x00);
        buffer.extend_from_slice(&dev_addr.to_le_bytes());
        buffer.push(0x00);
        buffer.push(0x00);
        buffer.push(f_port);
        buffer.extend_from_slice(&fcnt.to_le_bytes());
        buffer.push(0x00);
        buffer.push((payload.len() + 9) as u8);

        let mut block_data = [0u8; 16];
        let payload_with_header_len = 9 + payload.len();
        block_data[0] = B0_MIC_CONSTANT;
        block_data[1] = 0x00;
        block_data[2] = 0x00;
        block_data[3] = 0x00;
        block_data[4] = 0x00;
        block_data[5..9].copy_from_slice(&dev_addr.to_le_bytes());
        block_data[9] = 0x00;
        block_data[10] = 0x00;
        block_data[11] = f_port;
        block_data[12..16].copy_from_slice(&fcnt.to_le_bytes());

        let mut msg = block_data.to_vec();
        msg.push(0x00);
        msg.push(payload_with_header_len as u8);
        msg.extend_from_slice(payload);
        msg.push(0x80);

        while msg.len() % 16 != 0 {
            msg.push(0x00);
        }

        let computed = cmac.compute(&msg)?;
        let computed_mic: [u8; 4] = [computed[0], computed[1], computed[2], computed[3]];

        let mut diff = 0u8;
        for i in 0..4 {
            diff |= computed_mic[i] ^ mic[i];
        }

        Ok(diff == 0)
    }
}

pub struct PayloadDecryptor;

impl PayloadDecryptor {
    pub fn decrypt(
        key: &[u8; 16],
        dev_addr: u32,
        fcnt: u32,
        f_port: u8,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        let aes_ctr = AesCtr::new(key)?;

        let mut iv = [0u8; 16];
        iv[0] = 0x01;
        iv[1] = 0x00;
        iv[2] = 0x00;
        iv[3] = 0x00;
        let dev_addr_bytes = dev_addr.to_le_bytes();
        iv[4] = dev_addr_bytes[0];
        iv[5] = dev_addr_bytes[1];
        iv[6] = dev_addr_bytes[2];
        iv[7] = dev_addr_bytes[3];
        iv[8] = 0x00;
        iv[9] = 0x00;
        iv[10] = f_port;
        let fcnt_bytes = fcnt.to_le_bytes();
        iv[11] = fcnt_bytes[0];
        iv[12] = fcnt_bytes[1];
        iv[13] = fcnt_bytes[2];
        iv[14] = fcnt_bytes[3];
        iv[15] = 0x00;

        aes_ctr.decrypt_slice(&iv, ciphertext)
    }
}

pub struct JoinCrypto;

impl JoinCrypto {
    /// Validates the MIC of a JoinRequest per LoRaWAN 1.0.3 §6.2.3.
    /// MIC = AES128_CMAC(AppKey, MHDR|AppEUI|DevEUI|DevNonce)
    pub fn validate_join_request_mic(app_key: &[u8; 16], raw_payload: &[u8]) -> Result<bool> {
        if raw_payload.len() < MIC_SIZE {
            return Ok(false);
        }
        let message = &raw_payload[..raw_payload.len() - MIC_SIZE];
        let given_mic: [u8; 4] = raw_payload[raw_payload.len() - MIC_SIZE..]
            .try_into()
            .map_err(|_| CryptoError::InternalError("mic slice conversion failed"))?;
        let cmac = Cmac::new(app_key)?;
        let computed = cmac.compute(message)?;
        let mut diff = 0u8;
        for i in 0..MIC_SIZE {
            diff |= computed[i] ^ given_mic[i];
        }
        Ok(diff == 0)
    }

    /// Validates the MIC of a data uplink per LoRaWAN 1.0.3 §4.4.
    /// `mac_payload` = full frame bytes EXCEPT the last 4 MIC bytes.
    /// MIC = AES128_CMAC(NwkSKey, B0 | mac_payload)[0..4]
    pub fn validate_uplink_mic(
        nwk_s_key: &[u8; 16],
        dev_addr: u32,
        fcnt: u32,
        mac_payload: &[u8],
        mic: &[u8; 4],
    ) -> Result<bool> {
        let msg_len = mac_payload.len() as u8;
        let dev_addr_bytes = dev_addr.to_le_bytes();
        let fcnt_bytes = fcnt.to_le_bytes();
        // B0 block: 0x49 | Pad4 | Dir(0=uplink) | DevAddr(4 LE) | FCntUp(4 LE) | 0x00 | Len
        let b0 = [
            0x49,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00, // direction: 0 = uplink
            dev_addr_bytes[0],
            dev_addr_bytes[1],
            dev_addr_bytes[2],
            dev_addr_bytes[3],
            fcnt_bytes[0],
            fcnt_bytes[1],
            fcnt_bytes[2],
            fcnt_bytes[3],
            0x00,
            msg_len,
        ];
        let mut message = b0.to_vec();
        message.extend_from_slice(mac_payload);
        let cmac = Cmac::new(nwk_s_key)?;
        let computed = cmac.compute(&message)?;
        let mut diff = 0u8;
        for i in 0..MIC_SIZE {
            diff |= computed[i] ^ mic[i];
        }
        Ok(diff == 0)
    }

    /// Derives NwkSKey and AppSKey per LoRaWAN 1.0.3 §6.2.5.
    /// Returns (NwkSKey, AppSKey).
    pub fn derive_session_keys(
        app_key: &[u8; 16],
        app_nonce: [u8; 3],
        net_id: [u8; 3],
        dev_nonce: u16,
    ) -> Result<([u8; 16], [u8; 16])> {
        let dev_nonce_bytes = dev_nonce.to_le_bytes();
        let mut nwk_pad = [0u8; 16];
        nwk_pad[0] = 0x01;
        nwk_pad[1..4].copy_from_slice(&app_nonce);
        nwk_pad[4..7].copy_from_slice(&net_id);
        nwk_pad[7..9].copy_from_slice(&dev_nonce_bytes);
        let mut app_pad = [0u8; 16];
        app_pad[0] = 0x02;
        app_pad[1..4].copy_from_slice(&app_nonce);
        app_pad[4..7].copy_from_slice(&net_id);
        app_pad[7..9].copy_from_slice(&dev_nonce_bytes);
        let nwk_s_key = Self::aes128_encrypt_block(app_key, &nwk_pad)?;
        let app_s_key = Self::aes128_encrypt_block(app_key, &app_pad)?;
        Ok((nwk_s_key, app_s_key))
    }

    /// Derives a deterministic DevAddr from AppKey and DevEUI for private networks.
    pub fn derive_dev_addr(app_key: &[u8; 16], dev_eui: &[u8; 8]) -> Result<u32> {
        let cmac = Cmac::new(app_key)?;
        let mut input = [0u8; 16];
        input[0..8].copy_from_slice(dev_eui);
        let computed = cmac.compute(&input)?;
        Ok(u32::from_le_bytes([
            computed[0],
            computed[1],
            computed[2],
            computed[3],
        ]))
    }

    fn aes128_encrypt_block(key: &[u8; 16], plaintext: &[u8; 16]) -> Result<[u8; 16]> {
        use aes::cipher::generic_array::GenericArray;
        use aes::cipher::{BlockEncrypt, KeyInit};
        let cipher = aes::Aes128::new_from_slice(key)
            .map_err(|_| CryptoError::invalid_key_length(16, key.len()))?;
        let mut block = GenericArray::clone_from_slice(plaintext);
        cipher.encrypt_block(&mut block);
        let mut out = [0u8; 16];
        out.copy_from_slice(&block);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header_parse() {
        let bytes = [0x01, 0x02, 0x03, 0x04, 0x00, 0x2E, 0x00, 0x00, 0x00];
        let header = LoRawanFrameHeader::parse(&bytes).unwrap();
        assert_eq!(header.dev_addr, 0x04030201);
        assert_eq!(header.fcnt, 0x002E);
    }

    #[test]
    fn test_pad_to_block_size() {
        let data = vec![0x01, 0x02, 0x03];
        let padded = pad_to_block_size(&data);
        assert_eq!(padded.len(), 16);
        assert_eq!(padded[..3], [0x01, 0x02, 0x03]);
        assert_eq!(padded[3], 0x80);
    }

    #[test]
    fn test_payload_decryptor_empty() {
        let key = [0u8; 16];
        let result = PayloadDecryptor::decrypt(&key, 0x04030201, 0, 1, &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
