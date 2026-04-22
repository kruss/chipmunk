use anyhow::{Context, Result};
use pcap_parser::{LegacyPcapReader, PcapBlockOwned, traits::PcapReaderIterator};
use rustc_hash::FxHashMap;
use someip_messages::*;
use std::io::Read;
use std::{fs::File, path::PathBuf};

/// Collects the SOME/IP statistics from the given source files.
pub fn someip_statistics(sources: Vec<PathBuf>) -> Result<SomeipStatistics, String> {
    let mut statistics = SomeipStatistics::default();

    for source in sources {
        if let Ok(path) = source.clone().into_os_string().into_string()
            && let Ok(file) = File::open(path)
        {
            match LegacyPcapReader::new(65536, file).context("Failed to create pcap reader") {
                Ok(mut reader) => {
                    if let Err(error) = collect_statistics_from_pcap(&mut reader, &mut statistics) {
                        return Err(format!("{:?}: {}", source, error));
                    }
                }
                Err(error) => {
                    return Err(format!("{:?}: {}", source, error));
                }
            }
        } else {
            return Err(format!("invalid source: {:?}", source));
        }
    }

    Ok(statistics)
}

/// The statistics-info of SOME/IP files.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SomeipStatistics {
    pub total: MessageDistribution,
    pub messages: FxHashMap<MessageId, MessageDistribution>,
}

impl SomeipStatistics {
    pub fn count(&self) -> usize {
        self.messages.len()
    }

    pub fn message(&mut self, id: MessageId) -> &mut MessageDistribution {
        self.messages.entry(id).or_default()
    }
}

/// The Type distribution of SOME/IP messages.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MessageDistribution {
    pub sd: usize,
    pub event: usize,
    pub request: usize,
    pub response: usize,
    pub fire_forget: usize,
    pub error: usize,
}

impl MessageDistribution {
    pub fn count(&self) -> usize {
        self.sd + self.event + self.request + self.response + self.fire_forget + self.error
    }

    pub fn values(&self) -> [usize; 6] {
        [
            self.sd,
            self.event,
            self.request,
            self.response,
            self.fire_forget,
            self.error,
        ]
    }

    pub fn merge(&mut self, other: &MessageDistribution) -> &mut Self {
        self.sd += other.sd;
        self.event += other.event;
        self.request += other.request;
        self.response += other.response;
        self.fire_forget += other.fire_forget;
        self.error += other.error;
        self
    }
}

/// Collect all SOME/IP statistics from the given reader.
pub fn collect_statistics_from_pcap<S: Read>(
    reader: &mut LegacyPcapReader<S>,
    statistics: &mut SomeipStatistics,
) -> Result<(), String> {
    loop {
        match reader.next() {
            Ok((offset, block)) => {
                if let PcapBlockOwned::Legacy(b) = block {
                    if let Some(payload) = extract_udp_payload_from_pcap(&b.data) {
                        match Message::from_slice(payload) {
                            Ok(Message::Sd(header, _)) => {
                                let message_id = header.message_id().clone();
                                match header.message_type() {
                                    MessageType::Notification => {
                                        statistics.total.sd += 1;
                                        statistics.message(message_id).sd += 1;
                                    }
                                    MessageType::Error => {
                                        statistics.total.error += 1;
                                    }
                                    _ => {}
                                }
                            }
                            Ok(Message::Rpc(header, _)) => {
                                let message_id = header.message_id().clone();
                                match header.message_type() {
                                    MessageType::Notification | MessageType::TpNotification => {
                                        statistics.total.event += 1;
                                        statistics.message(message_id).event += 1;
                                    }
                                    MessageType::Request | MessageType::TpRequest => {
                                        statistics.total.request += 1;
                                        statistics.message(message_id).request += 1;
                                    }
                                    MessageType::Response | MessageType::TpResponse => {
                                        statistics.total.response += 1;
                                        statistics.message(message_id).response += 1;
                                    }
                                    MessageType::RequestNoReturn
                                    | MessageType::TpRequestNoReturn => {
                                        statistics.total.fire_forget += 1;
                                        statistics.message(message_id).fire_forget += 1;
                                    }
                                    MessageType::Error | MessageType::TpError => {
                                        statistics.total.error += 1;
                                        statistics.message(message_id).error += 1;
                                    }
                                }
                            }
                            Ok(Message::CookieClient) => {}
                            Ok(Message::CookieServer) => {}
                            Err(_) => {}
                        }
                    }
                }

                reader.consume(offset);
            }
            Err(pcap_parser::PcapError::Eof) => break,
            Err(e) => {
                eprintln!("PCAP error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

fn extract_udp_payload_from_pcap(frame: &[u8]) -> Option<&[u8]> {
    // Ethernet II header
    if frame.len() < 14 {
        return None;
    }

    let mut offset = 12usize;
    let mut ethertype = u16::from_be_bytes([frame[offset], frame[offset + 1]]);
    offset += 2;

    // Optional single or stacked VLAN tags
    while ethertype == 0x8100 || ethertype == 0x88A8 {
        if frame.len() < offset + 4 {
            return None;
        }
        // skip TCI
        offset += 2;
        ethertype = u16::from_be_bytes([frame[offset], frame[offset + 1]]);
        offset += 2;
    }

    // IPv4 only
    if ethertype != 0x0800 {
        return None;
    }

    if frame.len() < offset + 20 {
        return None;
    }

    let ip_start = offset;
    let ver_ihl = frame[ip_start];
    let version = ver_ihl >> 4;
    let ihl = (ver_ihl & 0x0f) as usize * 4;

    if version != 4 || ihl < 20 || frame.len() < ip_start + ihl {
        return None;
    }

    let protocol = frame[ip_start + 9];
    if protocol != 17 {
        // UDP only
        return None;
    }

    let udp_start = ip_start + ihl;
    if frame.len() < udp_start + 8 {
        return None;
    }

    let udp_len = u16::from_be_bytes([frame[udp_start + 4], frame[udp_start + 5]]) as usize;
    if udp_len < 8 {
        return None;
    }

    let payload_start = udp_start + 8;
    let payload_end = payload_start.checked_add(udp_len - 8)?;

    if payload_end > frame.len() {
        return None;
    }

    Some(&frame[payload_start..payload_end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_from_pcap() {
        let sources: Vec<PathBuf> =
            ["../../../../../developing/resources/someip/example.pcap".into()].to_vec();

        let statistics = someip_statistics(sources).expect("stats");

        let mut messages = FxHashMap::default();

        messages.insert(
            MessageId {
                service_id: 123,
                method_id: 32773,
            },
            MessageDistribution {
                sd: 0,
                event: 22,
                request: 0,
                response: 0,
                fire_forget: 0,
                error: 0,
            },
        );

        messages.insert(
            MessageId {
                service_id: 65535,
                method_id: 33024,
            },
            MessageDistribution {
                sd: 33,
                event: 0,
                request: 0,
                response: 0,
                fire_forget: 0,
                error: 0,
            },
        );

        assert_eq!(
            statistics,
            SomeipStatistics {
                total: MessageDistribution {
                    sd: 33,
                    event: 22,
                    request: 0,
                    response: 0,
                    fire_forget: 0,
                    error: 0,
                },
                messages,
            }
        );
    }
}
