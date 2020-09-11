use alloc::collections::VecDeque;
use core::{cmp, convert::TryFrom as _};

// File generated by the build script.
mod payload_proto {
    include!(concat!(env!("OUT_DIR"), "/payload.proto.rs"));
}

pub struct Noise {
    inner: snow::TransportState,

    /// Buffer of data containing data received on the wire, before decryption.
    rx_buffer_encrypted: VecDeque<u8>,

    /// Buffer of data containing data received on the wire, after decryption.
    rx_buffer_decrypted: Vec<u8>,

    /// Buffer of data containing data received on the wire, after encryption.
    tx_buffer_encrypted: VecDeque<u8>,
}

impl Noise {
    /// Feeds data received from the wire.
    pub fn inject_inbound_data(&mut self, payload: &[u8]) {
        // TODO: possibly optimize by not always copy bytes to `rx_buffer_encrypted`
        self.rx_buffer_encrypted.extend(payload.iter().cloned());

        self.rx_buffer_decrypted.resize(payload.len(), 0);
        let _written = self
            .inner
            .read_message(payload, &mut self.rx_buffer_decrypted);
        // TODO: continue
        // TODO: check _written
    }

    ///
    /// > **Note**: You are encouraged to not call this method with small payloads, as at least
    /// >           two bytes of data are added to the stream every time this method is called.
    // TODO: docs
    pub fn inject_outbound_data(&mut self, payload: &[u8]) {
        // The maximum size of a noise message is 65535 bytes. As such, we split any payload that
        // is longer than that.
        for payload in payload.chunks(65535) {
            debug_assert!(payload.is_empty()); // guaranteed by `chunks()`

            // The complexity below stems from the fact that we write into a `VecDeque`.

            // TODO: review; might be wrong

            let out_buf_len_before = self.tx_buffer_encrypted.len();
            self.tx_buffer_encrypted
                .resize(out_buf_len_before + 2 + payload.len(), 0);

            let payload_len_bytes = u16::try_from(payload.len()).unwrap().to_be_bytes();
            self.tx_buffer_encrypted[out_buf_len_before] = payload_len_bytes[0];
            self.tx_buffer_encrypted[out_buf_len_before + 1] = payload_len_bytes[1];

            let mut out_buf_slices = self.tx_buffer_encrypted.as_mut_slices();
            out_buf_slices.1 =
                &mut out_buf_slices.1[out_buf_len_before.saturating_sub(out_buf_slices.0.len())..];
            out_buf_slices.0 = {
                let off = out_buf_slices.0.len().saturating_sub(out_buf_len_before);
                &mut out_buf_slices.0[off..]
            };

            let to_write0 = cmp::min(out_buf_slices.0.len(), payload.len());
            debug_assert!(payload.len().saturating_sub(to_write0) <= out_buf_slices.1.len());

            let _written = self
                .inner
                .write_message(&payload[..to_write0], out_buf_slices.0)
                .unwrap();
            debug_assert_eq!(_written, to_write0);

            if to_write0 != payload.len() {
                let _written = self
                    .inner
                    .write_message(&payload[to_write0..], out_buf_slices.1)
                    .unwrap();
                debug_assert_eq!(_written, out_buf_slices.1.len().saturating_sub(to_write0));
            }
        }
    }

    /// Write to the given buffer the bytes that are ready to be sent out. Returns the number of
    /// bytes written to `destination`.
    pub fn write_out(&mut self, destination: &mut [u8]) -> usize {
        let to_write = self.tx_buffer_encrypted.as_slices().0;
        let to_write_len = cmp::min(to_write.len(), destination.len());
        destination.copy_from_slice(&to_write[..to_write_len]);
        for _ in 0..to_write_len {
            let _ = self.tx_buffer_encrypted.pop_front();
        }
        to_write_len
    }
}

pub struct NoiseHandshake {}

lazy_static::lazy_static! {
    static ref NOISE_PARAMS: snow::params::NoiseParams =
        "Noise_XX_25519_ChaChaPoly_SHA256".parse().unwrap();
}
